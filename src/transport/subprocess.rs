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

        // Merge user env vars
        for (key, value) in &options.env {
            cmd.env(key, value);
        }

        // Set working directory
        if let Some(cwd) = &options.cwd {
            cmd.current_dir(cwd);
        }

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
    fn build_command_streaming(&self, options: &ClaudeAgentOptions) -> Vec<String> {
        let mut args = vec![
            "--print".to_string(),
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

    /// Add common CLI options (used by both modes).
    fn add_common_options(&self, args: &mut Vec<String>, options: &ClaudeAgentOptions) {
        // System prompt
        if let Some(system_prompt) = &options.system_prompt {
            match system_prompt {
                crate::types::SystemPrompt::Text(text) => {
                    args.push("--system-prompt".to_string());
                    args.push(text.clone());
                }
                crate::types::SystemPrompt::Preset(preset) => {
                    if let Some(append) = &preset.append {
                        args.push("--append-system-prompt".to_string());
                        args.push(append.clone());
                    }
                }
            }
        }

        // Tools
        if !options.allowed_tools.is_empty() {
            args.push(format!("--allowedTools={}", options.allowed_tools.join(",")));
        }

        if !options.disallowed_tools.is_empty() {
            args.push(format!("--disallowedTools={}", options.disallowed_tools.join(",")));
        }

        // Permission mode
        if let Some(mode) = &options.permission_mode {
            args.push("--permission-mode".to_string());
            args.push(mode.to_string());
        }

        // Permission prompt tool (for SDK control protocol)
        if let Some(tool) = &options.permission_prompt_tool_name {
            args.push("--permission-prompt-tool".to_string());
            args.push(tool.clone());
        }

        // Model
        if let Some(model) = &options.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }

        // Max turns
        if let Some(max_turns) = options.max_turns {
            args.push("--max-turns".to_string());
            args.push(max_turns.to_string());
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
