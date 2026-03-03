//! Type definitions for Claude Agent SDK.

/// Agent definitions and setting source configuration.
pub mod agents;
/// Content blocks within assistant messages (text, thinking, tool use/result).
pub mod content;
/// Control protocol requests and responses between SDK and CLI.
pub mod control;
/// Hook events, inputs, and outputs for intercepting agent behavior.
pub mod hooks;
/// MCP (Model Context Protocol) server configurations.
pub mod mcp;
/// Message types streamed from the CLI (user, assistant, system, result).
pub mod messages;
/// Agent options, permission modes, and configuration builders.
pub mod options;
/// Permission results and tool permission context types.
pub mod permissions;
/// SDK plugin configuration.
pub mod plugins;
/// Sandbox settings for restricting bash command execution.
pub mod sandbox;
/// Usage tracking and quota monitoring (Max Plan).
pub mod usage;

// Re-export commonly used types
pub use agents::{AgentDefinition, SettingSource};
pub use content::{ContentBlock, TextBlock, ThinkingBlock, ToolResultBlock, ToolUseBlock};
pub use control::{ControlRequest, HookMatcherConfig, SDKControlRequest, SDKControlResponse};
pub use hooks::{
    AsyncHookOutput, HookContext, HookEvent, HookInput, HookMatcher, HookOutput,
    NotificationHookInput, PostToolUseFailureHookInput, PostToolUseHookInput,
    PreToolUseHookInput, SubagentStartHookInput, SubagentStopHookInput, SyncHookOutput,
};
pub use mcp::{McpHttpConfig, McpServerConfig, McpSdkServerConfig, McpSseConfig, McpStdioConfig};
pub use messages::{
    AssistantMessage, Message, MessageContent, ResultMessage, StreamEvent, SystemMessage,
    UserMessage,
};
pub use options::{
    ClaudeAgentOptions, Effort, PermissionMode, SystemPrompt, SystemPromptPreset, ThinkingConfig,
    ToolsOption, ToolsPreset,
};
pub use permissions::{
    PermissionBehavior, PermissionResult, PermissionResultAllow, PermissionResultDeny,
    PermissionRuleValue, PermissionUpdate, PermissionUpdateDestination, PermissionUpdateType,
    ToolPermissionContext,
};
pub use plugins::SdkPluginConfig;
pub use sandbox::{SandboxIgnoreViolations, SandboxNetworkConfig, SandboxSettings};
pub use usage::{UsageData, UsageLimit};
