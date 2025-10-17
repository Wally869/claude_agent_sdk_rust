//! Type definitions for Claude Agent SDK.

pub mod agents;
pub mod content;
pub mod control;
pub mod hooks;
pub mod mcp;
pub mod messages;
pub mod options;
pub mod permissions;

// Re-export commonly used types
pub use agents::{AgentDefinition, SettingSource};
pub use content::{ContentBlock, TextBlock, ThinkingBlock, ToolResultBlock, ToolUseBlock};
pub use control::{ControlRequest, SDKControlRequest, SDKControlResponse};
pub use hooks::{
    AsyncHookOutput, HookContext, HookEvent, HookInput, HookMatcher, HookOutput,
    PreToolUseHookInput, PostToolUseHookInput, SyncHookOutput,
};
pub use mcp::{McpHttpConfig, McpServerConfig, McpSdkServerConfig, McpSseConfig, McpStdioConfig};
pub use messages::{
    AssistantMessage, Message, MessageContent, ResultMessage, StreamEvent, SystemMessage,
    UserMessage,
};
pub use options::{ClaudeAgentOptions, PermissionMode, SystemPrompt, SystemPromptPreset};
pub use permissions::{
    PermissionBehavior, PermissionResult, PermissionResultAllow, PermissionResultDeny,
    PermissionRuleValue, PermissionUpdate, PermissionUpdateDestination, PermissionUpdateType,
    ToolPermissionContext,
};
