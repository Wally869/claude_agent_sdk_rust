//! Type definitions for Claude Agent SDK.

pub mod agents;
pub mod content;
pub mod control;
pub mod hooks;
pub mod mcp;
pub mod messages;
pub mod options;
pub mod permissions;
pub mod plugins;
pub mod sandbox;
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
