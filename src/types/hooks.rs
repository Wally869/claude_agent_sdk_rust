//! Hook system types for event interception.

use serde::{Deserialize, Serialize};

/// Hook event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEvent {
    /// Before tool execution.
    PreToolUse,
    /// After tool execution.
    PostToolUse,
    /// After tool execution failure.
    PostToolUseFailure,
    /// When user submits a prompt.
    UserPromptSubmit,
    /// When agent loop stops.
    Stop,
    /// When subagent stops.
    SubagentStop,
    /// Before context compaction.
    PreCompact,
    /// When a notification is sent.
    Notification,
    /// When a subagent starts.
    SubagentStart,
    /// When a permission request is made.
    PermissionRequest,
}

/// Hook matcher configuration.
#[non_exhaustive]
pub struct HookMatcher {
    /// Tool name pattern (e.g., "Bash", "Read|Write", "*").
    pub matcher: Option<String>,
}

/// Base fields present in all hook inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseHookInput {
    /// Session identifier.
    pub session_id: String,
    /// Path to transcript file.
    pub transcript_path: String,
    /// Current working directory.
    pub cwd: String,
    /// Current permission mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
}

/// Input for PreToolUse hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreToolUseHookInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name.
    pub hook_event_name: String, // "PreToolUse"
    /// Tool being invoked.
    pub tool_name: String,
    /// Tool input parameters.
    pub tool_input: serde_json::Value,
    /// Tool use identifier.
    pub tool_use_id: String,
}

/// Input for PostToolUse hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUseHookInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name.
    pub hook_event_name: String, // "PostToolUse"
    /// Tool that was invoked.
    pub tool_name: String,
    /// Tool input parameters.
    pub tool_input: serde_json::Value,
    /// Tool execution result.
    pub tool_response: serde_json::Value,
    /// Tool use identifier.
    pub tool_use_id: String,
}

/// Input for PostToolUseFailure hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUseFailureHookInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name.
    pub hook_event_name: String, // "PostToolUseFailure"
    /// Tool that failed.
    pub tool_name: String,
    /// Tool input parameters.
    pub tool_input: serde_json::Value,
    /// Tool use identifier.
    pub tool_use_id: String,
    /// Error message.
    pub error: String,
    /// Whether this was an interrupt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_interrupt: Option<bool>,
}

/// Input for UserPromptSubmit hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPromptSubmitHookInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name.
    pub hook_event_name: String, // "UserPromptSubmit"
    /// User's prompt.
    pub prompt: String,
}

/// Input for Stop hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopHookInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name.
    pub hook_event_name: String, // "Stop"
    /// Whether stop hook is active.
    pub stop_hook_active: bool,
}

/// Input for SubagentStop hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentStopHookInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name.
    pub hook_event_name: String, // "SubagentStop"
    /// Whether stop hook is active.
    pub stop_hook_active: bool,
    /// Agent identifier.
    pub agent_id: String,
    /// Path to agent's transcript.
    pub agent_transcript_path: String,
    /// Type of the subagent.
    pub agent_type: String,
}

/// Input for PreCompact hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreCompactHookInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name.
    pub hook_event_name: String, // "PreCompact"
    /// What triggered compaction.
    pub trigger: String, // "manual" | "auto"
    /// Custom compaction instructions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_instructions: Option<String>,
}

/// Input for Notification hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationHookInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name.
    pub hook_event_name: String, // "Notification"
    /// Notification message.
    pub message: String,
    /// Optional title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Notification type.
    pub notification_type: String,
}

/// Input for SubagentStart hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentStartHookInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name.
    pub hook_event_name: String, // "SubagentStart"
    /// Agent identifier.
    pub agent_id: String,
    /// Type of the subagent.
    pub agent_type: String,
}

/// Input for PermissionRequest hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequestHookInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name.
    pub hook_event_name: String, // "PermissionRequest"
    /// Tool requesting permission.
    pub tool_name: String,
    /// Tool input parameters.
    pub tool_input: serde_json::Value,
    /// Permission suggestions from CLI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_suggestions: Option<Vec<serde_json::Value>>,
}

/// Hook input (discriminated union).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookInput {
    PreToolUse(PreToolUseHookInput),
    PostToolUse(PostToolUseHookInput),
    PostToolUseFailure(PostToolUseFailureHookInput),
    UserPromptSubmit(UserPromptSubmitHookInput),
    Stop(StopHookInput),
    SubagentStop(SubagentStopHookInput),
    PreCompact(PreCompactHookInput),
    Notification(NotificationHookInput),
    SubagentStart(SubagentStartHookInput),
    PermissionRequest(PermissionRequestHookInput),
}

/// Hook context provided to callbacks.
#[derive(Debug, Clone, Default)]
pub struct HookContext {
    /// Abort signal (future feature).
    pub signal: Option<()>,
}

/// Hook output (for synchronous hooks).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncHookOutput {
    /// Whether to continue execution (use continue_ to avoid keyword).
    #[serde(rename = "continue", skip_serializing_if = "Option::is_none")]
    pub continue_: Option<bool>,
    /// Hide output from transcript.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,
    /// Reason if stopping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// Decision (e.g., "block").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    /// System message to user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_message: Option<String>,
    /// Feedback for Claude.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Hook-specific output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<HookSpecificOutput>,
}

/// Hook-specific output fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookSpecificOutput {
    PreToolUse(PreToolUseHookSpecificOutput),
    PostToolUse(PostToolUseHookSpecificOutput),
    PostToolUseFailure(PostToolUseFailureHookSpecificOutput),
    UserPromptSubmit(UserPromptSubmitHookSpecificOutput),
    Notification(NotificationHookSpecificOutput),
    SubagentStart(SubagentStartHookSpecificOutput),
    PermissionRequest(PermissionRequestHookSpecificOutput),
}

/// PreToolUse-specific output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreToolUseHookSpecificOutput {
    pub hook_event_name: String, // "PreToolUse"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_decision: Option<String>, // "allow" | "deny" | "ask"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_decision_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// PostToolUse-specific output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostToolUseHookSpecificOutput {
    pub hook_event_name: String, // "PostToolUse"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_mcp_tool_output: Option<serde_json::Value>,
}

/// PostToolUseFailure-specific output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostToolUseFailureHookSpecificOutput {
    pub hook_event_name: String, // "PostToolUseFailure"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// UserPromptSubmit-specific output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPromptSubmitHookSpecificOutput {
    pub hook_event_name: String, // "UserPromptSubmit"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// Notification-specific output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationHookSpecificOutput {
    pub hook_event_name: String, // "Notification"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// SubagentStart-specific output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentStartHookSpecificOutput {
    pub hook_event_name: String, // "SubagentStart"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// PermissionRequest-specific output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequestHookSpecificOutput {
    pub hook_event_name: String, // "PermissionRequest"
    /// Decision object for permission handling.
    pub decision: serde_json::Value,
}

/// Async hook output (for deferred execution).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsyncHookOutput {
    /// Set to true for async execution (use async_ to avoid keyword).
    #[serde(rename = "async")]
    pub async_: bool,
    /// Timeout in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub async_timeout: Option<u32>,
}

/// Hook output (sync or async).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookOutput {
    Sync(Box<SyncHookOutput>),
    Async(AsyncHookOutput),
}

impl Default for HookOutput {
    fn default() -> Self {
        HookOutput::Sync(Box::default())
    }
}
