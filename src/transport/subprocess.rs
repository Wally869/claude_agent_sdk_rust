//! Subprocess transport for Claude Code CLI.

use crate::error::{ClaudeSDKError, Result};
use crate::types::ClaudeAgentOptions;
use async_stream::stream;
use futures::Stream;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};

const DEFAULT_MAX_BUFFER_SIZE: usize = 1024 * 1024; // 1MB

/// Subprocess-based transport for Claude CLI.
pub struct SubprocessTransport {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    cli_path: std::path::PathBuf,
    max_buffer_size: usize,
    streaming_mode: bool,
}

impl SubprocessTransport {
    /// Create a new subprocess transport for one-shot query mode.
    pub fn new(cli_path: std::path::PathBuf, options: &ClaudeAgentOptions) -> Self {
        let max_buffer_size = options.max_buffer_size.unwrap_or(DEFAULT_MAX_BUFFER_SIZE);
        Self {
            child: None,
            stdin: None,
            cli_path,
            max_buffer_size,
            streaming_mode: false,
        }
    }

    /// Create a new subprocess transport for streaming/interactive mode.
    pub fn new_streaming(cli_path: std::path::PathBuf, options: &ClaudeAgentOptions) -> Self {
        let max_buffer_size = options.max_buffer_size.unwrap_or(DEFAULT_MAX_BUFFER_SIZE);
        Self {
            child: None,
            stdin: None,
            cli_path,
            max_buffer_size,
            streaming_mode: true,
        }
    }

    /// Build CLI command with arguments (one-shot mode).
    fn build_command(&self, options: &ClaudeAgentOptions, prompt: &str) -> Vec<String> {
        let mut args = vec![
            "--print".to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
        ];

        // Add common options
        self.add_common_options(&mut args, options);

        // Add the prompt as positional argument
        args.push(prompt.to_string());

        args
    }

    /// Spawn the CLI process.
    pub async fn spawn(&mut self, options: &ClaudeAgentOptions, prompt: &str) -> Result<()> {
        let args = if self.streaming_mode {
            self.build_command_streaming(options)
        } else {
            self.build_command(options, prompt)
        };

        let mut cmd = Command::new(&self.cli_path);
        cmd.args(&args)
            .stdin(if self.streaming_mode { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set environment
        cmd.env("CLAUDE_CODE_ENTRYPOINT", "sdk-rust");
        cmd.env("CLAUDE_AGENT_SDK_VERSION", env!("CARGO_PKG_VERSION"));

        // Enable file checkpointing if requested
        if options.enable_file_checkpointing {
            cmd.env("CLAUDE_CODE_ENABLE_SDK_FILE_CHECKPOINTING", "true");
        }

        // Merge user env vars
        for (key, value) in &options.env {
            cmd.env(key, value);
        }

        // Set working directory
        if let Some(cwd) = &options.cwd {
            cmd.current_dir(cwd);
            cmd.env("PWD", cwd);
        }

        // Set user if provided
        // Note: user parameter is passed via CLI, not env var on Unix

        let mut child = cmd.spawn()
            .map_err(|e| ClaudeSDKError::connection(format!("Failed to spawn CLI: {}", e)))?;

        // In streaming mode, capture stdin
        if self.streaming_mode {
            self.stdin = child.stdin.take();
        }

        self.child = Some(child);

        Ok(())
    }

    /// Build CLI command for streaming/interactive mode.
    ///
    /// Note: No `--print` flag in streaming mode — the CLI stays alive
    /// and communicates bidirectionally via stdin/stdout.
    fn build_command_streaming(&self, options: &ClaudeAgentOptions) -> Vec<String> {
        let mut args = vec![
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--input-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
        ];

        // Same options as non-streaming, but without prompt
        self.add_common_options(&mut args, options);

        args
    }

    /// Build settings value, merging sandbox settings if provided.
    fn build_settings_value(options: &ClaudeAgentOptions) -> Option<String> {
        let has_settings = options.settings.is_some();
        let has_sandbox = options.sandbox.is_some();

        if !has_settings && !has_sandbox {
            return None;
        }

        // If only settings and no sandbox, pass through as-is
        if has_settings && !has_sandbox {
            return options.settings.clone();
        }

        // If we have sandbox settings, we need to merge into a JSON object
        let mut settings_obj: serde_json::Value = if let Some(settings_str) = &options.settings {
            let trimmed = settings_str.trim();
            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                serde_json::from_str(trimmed).unwrap_or_else(|_| serde_json::json!({}))
            } else {
                // It's a file path - try to read and parse
                std::fs::read_to_string(trimmed)
                    .ok()
                    .and_then(|contents| serde_json::from_str(&contents).ok())
                    .unwrap_or_else(|| serde_json::json!({}))
            }
        } else {
            serde_json::json!({})
        };

        // Merge sandbox settings
        if let Some(sandbox) = &options.sandbox {
            if let Ok(sandbox_value) = serde_json::to_value(sandbox) {
                settings_obj["sandbox"] = sandbox_value;
            }
        }

        Some(serde_json::to_string(&settings_obj).unwrap_or_default())
    }

    /// Add common CLI options (used by both modes).
    fn add_common_options(&self, args: &mut Vec<String>, options: &ClaudeAgentOptions) {
        // System prompt
        match &options.system_prompt {
            None => {
                args.push("--system-prompt".to_string());
                args.push(String::new());
            }
            Some(crate::types::SystemPrompt::Text(text)) => {
                args.push("--system-prompt".to_string());
                args.push(text.clone());
            }
            Some(crate::types::SystemPrompt::Preset(preset)) => {
                if let Some(append) = &preset.append {
                    args.push("--append-system-prompt".to_string());
                    args.push(append.clone());
                }
            }
        }

        // Base tools
        if let Some(tools) = &options.tools {
            match tools {
                crate::types::ToolsOption::List(list) => {
                    if list.is_empty() {
                        args.push("--tools".to_string());
                        args.push(String::new());
                    } else {
                        args.push("--tools".to_string());
                        args.push(list.join(","));
                    }
                }
                crate::types::ToolsOption::Preset(_) => {
                    args.push("--tools".to_string());
                    args.push("default".to_string());
                }
            }
        }

        // Allowed/disallowed tools
        if !options.allowed_tools.is_empty() {
            args.push("--allowedTools".to_string());
            args.push(options.allowed_tools.join(","));
        }

        if !options.disallowed_tools.is_empty() {
            args.push("--disallowedTools".to_string());
            args.push(options.disallowed_tools.join(","));
        }

        // Max turns
        if let Some(max_turns) = options.max_turns {
            args.push("--max-turns".to_string());
            args.push(max_turns.to_string());
        }

        // Max budget
        if let Some(max_budget_usd) = options.max_budget_usd {
            args.push("--max-budget-usd".to_string());
            args.push(max_budget_usd.to_string());
        }

        // Permission mode
        if let Some(mode) = &options.permission_mode {
            args.push("--permission-mode".to_string());
            args.push(mode.to_string());
        }

        // Model
        if let Some(model) = &options.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }

        // Fallback model
        if let Some(fallback_model) = &options.fallback_model {
            args.push("--fallback-model".to_string());
            args.push(fallback_model.clone());
        }

        // Betas
        if !options.betas.is_empty() {
            args.push("--betas".to_string());
            args.push(options.betas.join(","));
        }

        // Permission prompt tool (for SDK control protocol)
        if let Some(tool) = &options.permission_prompt_tool_name {
            args.push("--permission-prompt-tool".to_string());
            args.push(tool.clone());
        }

        // Session management
        if options.continue_conversation {
            args.push("--continue".to_string());
        }

        if let Some(resume_id) = &options.resume {
            args.push("--resume".to_string());
            args.push(resume_id.clone());
        }

        if options.fork_session {
            args.push("--fork-session".to_string());
        }

        if let Some(session_id) = &options.session_id {
            args.push("--session-id".to_string());
            args.push(session_id.clone());
        }

        // Settings and sandbox (merged)
        if let Some(settings_value) = Self::build_settings_value(options) {
            args.push("--settings".to_string());
            args.push(settings_value);
        }

        // Additional directories
        for dir in &options.add_dirs {
            args.push("--add-dir".to_string());
            args.push(dir.to_string_lossy().to_string());
        }

        // MCP servers
        if !options.mcp_servers.is_empty() {
            let mut servers_for_cli: serde_json::Map<String, serde_json::Value> =
                serde_json::Map::new();

            for (name, config) in &options.mcp_servers {
                match config {
                    crate::types::McpServerConfig::Sdk(sdk_config) => {
                        // For SDK servers, pass everything except the instance field
                        servers_for_cli.insert(
                            name.clone(),
                            serde_json::json!({"type": "sdk", "name": sdk_config.name}),
                        );
                    }
                    _ => {
                        // For external servers, pass as-is
                        if let Ok(value) = serde_json::to_value(config) {
                            servers_for_cli.insert(name.clone(), value);
                        }
                    }
                }
            }

            if !servers_for_cli.is_empty() {
                let mcp_config = serde_json::json!({"mcpServers": servers_for_cli});
                args.push("--mcp-config".to_string());
                args.push(serde_json::to_string(&mcp_config).unwrap_or_default());
            }
        }

        // Include partial messages
        if options.include_partial_messages {
            args.push("--include-partial-messages".to_string());
        }

        // Setting sources
        if let Some(sources) = &options.setting_sources {
            let sources_str = sources
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(",");
            args.push("--setting-sources".to_string());
            args.push(sources_str);
        } else {
            // Always pass setting sources (matching Python SDK behavior)
            args.push("--setting-sources".to_string());
            args.push(String::new());
        }

        // Plugins
        for plugin in &options.plugins {
            if plugin.plugin_type == "local" {
                args.push("--plugin-dir".to_string());
                args.push(plugin.path.clone());
            }
        }

        // Thinking config → --max-thinking-tokens
        // `thinking` takes precedence over deprecated `max_thinking_tokens`
        let mut resolved_max_thinking_tokens = options.max_thinking_tokens;
        if let Some(thinking) = &options.thinking {
            match thinking {
                crate::types::ThinkingConfig::Adaptive => {
                    if resolved_max_thinking_tokens.is_none() {
                        resolved_max_thinking_tokens = Some(32_000);
                    }
                }
                crate::types::ThinkingConfig::Enabled { budget_tokens } => {
                    resolved_max_thinking_tokens = Some(*budget_tokens);
                }
                crate::types::ThinkingConfig::Disabled => {
                    resolved_max_thinking_tokens = Some(0);
                }
            }
        }
        if let Some(tokens) = resolved_max_thinking_tokens {
            args.push("--max-thinking-tokens".to_string());
            args.push(tokens.to_string());
        }

        // Effort
        if let Some(effort) = &options.effort {
            args.push("--effort".to_string());
            args.push(effort.to_string());
        }

        // Output format → --json-schema
        if let Some(output_format) = &options.output_format {
            if output_format.get("type").and_then(|v| v.as_str()) == Some("json_schema") {
                if let Some(schema) = output_format.get("schema") {
                    args.push("--json-schema".to_string());
                    args.push(serde_json::to_string(schema).unwrap_or_default());
                }
            }
        }

        // Extra args passthrough
        for (flag, value) in &options.extra_args {
            match value {
                None => {
                    // Boolean flag without value
                    args.push(format!("--{}", flag));
                }
                Some(val) => {
                    args.push(format!("--{}", flag));
                    args.push(val.to_string());
                }
            }
        }
    }

    /// Write a message to stdin (for streaming mode).
    pub async fn write(&mut self, data: &str) -> Result<()> {
        let stdin = self.stdin.as_mut()
            .ok_or(ClaudeSDKError::TransportNotReady)?;

        stdin.write_all(data.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        Ok(())
    }

    /// Take ownership of stdin (for passing to background tasks).
    pub fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.stdin.take()
    }

    /// Read messages from stdout as a stream.
    pub fn read_messages(&mut self) -> impl Stream<Item = Result<serde_json::Value>> + '_ {
        stream! {
            if let Some(child) = &mut self.child {
                // Spawn stderr reader task
                if let Some(stderr) = child.stderr.take() {
                    tokio::spawn(async move {
                        let reader = BufReader::new(stderr);
                        let mut lines = reader.lines();
                        while let Ok(Some(_line)) = lines.next_line().await {
                            // Ignore stderr output
                        }
                    });
                }

                if let Some(stdout) = child.stdout.take() {
                    let reader = BufReader::new(stdout);
                    let mut lines = reader.lines();
                    let mut json_buffer = String::new();

                    while let Ok(Some(line)) = lines.next_line().await {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        json_buffer.push_str(trimmed);

                        if json_buffer.len() > self.max_buffer_size {
                            yield Err(ClaudeSDKError::buffer_overflow(self.max_buffer_size));
                            json_buffer.clear();
                            continue;
                        }

                        // Try to parse JSON
                        match serde_json::from_str::<serde_json::Value>(&json_buffer) {
                            Ok(value) => {
                                json_buffer.clear();
                                yield Ok(value);
                            }
                            Err(_) => {
                                // Not complete JSON yet, keep buffering
                                continue;
                            }
                        }
                    }
                }

                // Check exit code
                if let Ok(status) = child.wait().await {
                    if !status.success() {
                        if let Some(code) = status.code() {
                            yield Err(ClaudeSDKError::process(
                                code,
                                "CLI process exited with error".to_string(),
                                None,
                            ));
                        }
                    }
                }
            }
        }
    }
}
