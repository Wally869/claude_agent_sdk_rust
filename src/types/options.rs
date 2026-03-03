//! Configuration options for Claude Agent SDK.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use typed_builder::TypedBuilder;

use super::agents::{AgentDefinition, SettingSource};
use super::mcp::McpServerConfig;
use super::plugins::SdkPluginConfig;
use super::sandbox::SandboxSettings;

/// Base set of tools configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ToolsOption {
    /// Explicit list of tool names.
    List(Vec<String>),
    /// Preset tool configuration.
    Preset(ToolsPreset),
}

/// Tools preset configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolsPreset {
    /// Type (always "preset").
    #[serde(rename = "type")]
    pub preset_type: String,
    /// Preset name (e.g., "claude_code").
    pub preset: String,
}

/// Thinking configuration for extended thinking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ThinkingConfig {
    /// Adaptive thinking (model decides depth).
    Adaptive,
    /// Enabled with specific budget.
    Enabled {
        /// Maximum tokens for thinking.
        budget_tokens: u32,
    },
    /// Thinking disabled.
    Disabled,
}

/// Effort level for thinking depth.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    Low,
    Medium,
    High,
    Max,
}

impl std::fmt::Display for Effort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Effort::Low => write!(f, "low"),
            Effort::Medium => write!(f, "medium"),
            Effort::High => write!(f, "high"),
            Effort::Max => write!(f, "max"),
        }
    }
}

/// Configuration options for Claude Agent queries and clients.
#[derive(Debug, Clone, TypedBuilder, Default)]
#[builder(field_defaults(default, setter(into)))]
pub struct ClaudeAgentOptions {
    /// Base set of tools (list of names or preset).
    pub tools: Option<ToolsOption>,

    /// Tools that Claude is allowed to use.
    pub allowed_tools: Vec<String>,

    /// Tools that Claude is NOT allowed to use.
    pub disallowed_tools: Vec<String>,

    /// System prompt (text or preset).
    pub system_prompt: Option<SystemPrompt>,

    /// MCP server configurations.
    #[builder(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,

    /// Permission mode for tool execution.
    pub permission_mode: Option<PermissionMode>,

    /// Maximum number of conversation turns.
    pub max_turns: Option<u32>,

    /// Maximum budget in USD.
    pub max_budget_usd: Option<f64>,

    /// Model to use (overrides default).
    pub model: Option<String>,

    /// Fallback model if primary is unavailable.
    pub fallback_model: Option<String>,

    /// Beta features to enable.
    #[builder(default)]
    pub betas: Vec<String>,

    /// Tool to use for permission prompts (internal).
    pub permission_prompt_tool_name: Option<String>,

    /// Working directory for the CLI.
    pub cwd: Option<PathBuf>,

    /// Custom path to Claude Code CLI binary.
    pub cli_path: Option<PathBuf>,

    /// Path to settings file or JSON string.
    pub settings: Option<String>,

    /// Additional directories to add for context.
    #[builder(default)]
    pub add_dirs: Vec<PathBuf>,

    /// Environment variables to pass to CLI.
    #[builder(default)]
    pub env: HashMap<String, String>,

    /// Arbitrary additional CLI flags.
    #[builder(default)]
    pub extra_args: HashMap<String, Option<String>>,

    /// Maximum buffer size for JSON parsing (default 1MB).
    pub max_buffer_size: Option<usize>,

    /// Continue previous conversation.
    #[builder(default)]
    pub continue_conversation: bool,

    /// Session ID to resume.
    pub resume: Option<String>,

    /// Force a specific session ID for a new conversation (must be valid UUID).
    pub session_id: Option<String>,

    /// Include partial streaming messages.
    #[builder(default)]
    pub include_partial_messages: bool,

    /// Fork session on resume instead of continuing.
    #[builder(default)]
    pub fork_session: bool,

    /// Setting sources to load (user, project, local).
    pub setting_sources: Option<Vec<SettingSource>>,

    /// Custom agent definitions.
    pub agents: Option<HashMap<String, AgentDefinition>>,

    /// Plugin configurations.
    #[builder(default)]
    pub plugins: Vec<SdkPluginConfig>,

    /// Sandbox configuration for bash command isolation.
    pub sandbox: Option<SandboxSettings>,

    /// Max tokens for thinking blocks.
    /// @deprecated Use `thinking` instead.
    pub max_thinking_tokens: Option<u32>,

    /// Controls extended thinking behavior. Takes precedence over max_thinking_tokens.
    pub thinking: Option<ThinkingConfig>,

    /// Effort level for thinking depth.
    pub effort: Option<Effort>,

    /// Output format for structured outputs (matches Messages API structure).
    /// Example: `{"type": "json_schema", "schema": {"type": "object", "properties": {...}}}`
    pub output_format: Option<serde_json::Value>,

    /// Enable file checkpointing to track file changes during the session.
    #[builder(default)]
    pub enable_file_checkpointing: bool,

    /// User parameter passed to the CLI process.
    pub user: Option<String>,
}

/// System prompt configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SystemPrompt {
    /// Plain text system prompt.
    Text(String),
    /// Preset system prompt.
    Preset(SystemPromptPreset),
}

/// Preset system prompt configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemPromptPreset {
    /// Type (always "preset").
    #[serde(rename = "type")]
    pub preset_type: String,
    /// Preset name (e.g., "claude_code").
    pub preset: String,
    /// Text to append to preset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append: Option<String>,
}

/// Permission mode for tool execution.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// Default mode (prompt for dangerous tools).
    Default,
    /// Auto-accept file edits.
    AcceptEdits,
    /// Plan mode (no execution).
    Plan,
    /// Bypass all permissions (dangerous!).
    BypassPermissions,
}

impl std::fmt::Display for PermissionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionMode::Default => write!(f, "default"),
            PermissionMode::AcceptEdits => write!(f, "acceptEdits"),
            PermissionMode::Plan => write!(f, "plan"),
            PermissionMode::BypassPermissions => write!(f, "bypassPermissions"),
        }
    }
}
