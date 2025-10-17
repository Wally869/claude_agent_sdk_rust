//! Configuration options for Claude Agent SDK.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use typed_builder::TypedBuilder;

use super::{agents::{AgentDefinition, SettingSource}, mcp::McpServerConfig};

/// Configuration options for Claude Agent queries and clients.
#[derive(Debug, Clone, TypedBuilder, Default)]
#[builder(field_defaults(default, setter(into)))]
pub struct ClaudeAgentOptions {
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

    /// Model to use (overrides default).
    pub model: Option<String>,

    /// Tool to use for permission prompts (internal).
    pub permission_prompt_tool_name: Option<String>,

    /// Working directory for the CLI.
    pub cwd: Option<PathBuf>,

    /// Custom path to Claude Code CLI binary.
    pub cli_path: Option<PathBuf>,

    /// Path to settings file.
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
