//! Control protocol types for SDK/CLI communication.

use serde::{Deserialize, Serialize};

/// Control request from SDK to CLI or CLI to SDK.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKControlRequest {
    /// Message type (always "control_request").
    #[serde(rename = "type")]
    pub msg_type: String,
    /// Unique request identifier.
    pub request_id: String,
    /// The actual request.
    pub request: ControlRequest,
}

/// Control response from SDK or CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKControlResponse {
    /// Message type (always "control_response").
    #[serde(rename = "type")]
    pub msg_type: String,
    /// The response.
    pub response: ControlResponseData,
}

/// Control response data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ControlResponseData {
    Success(ControlResponseSuccess),
    Error(ControlResponseError),
}

/// Successful control response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlResponseSuccess {
    /// Subtype (always "success").
    pub subtype: String,
    /// Request ID this responds to.
    pub request_id: String,
    /// Response data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<serde_json::Value>,
}

/// Error control response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlResponseError {
    /// Subtype (always "error").
    pub subtype: String,
    /// Request ID this responds to.
    pub request_id: String,
    /// Error message.
    pub error: String,
}

/// Hook matcher configuration for initialize request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookMatcherConfig {
    /// Tool name matcher pattern (e.g., "Bash", "Read|Write", "*").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
    /// Hook callback IDs to invoke for this matcher.
    pub hook_callback_ids: Vec<String>,
}

/// Control request types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum ControlRequest {
    /// Initialize streaming mode.
    Initialize {
        #[serde(skip_serializing_if = "Option::is_none")]
        hooks: Option<serde_json::Value>,
    },
    /// Interrupt current processing.
    Interrupt,
    /// Change permission mode.
    SetPermissionMode { mode: String },
    /// Change AI model.
    SetModel { model: Option<String> },
    /// Request permission for tool use (CLI → SDK).
    CanUseTool {
        tool_name: String,
        input: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        permission_suggestions: Option<Vec<serde_json::Value>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        blocked_path: Option<String>,
    },
    /// Execute hook callback (CLI → SDK).
    HookCallback {
        callback_id: String,
        input: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_use_id: Option<String>,
    },
    /// Route MCP message to SDK server (CLI → SDK).
    McpMessage {
        server_name: String,
        message: serde_json::Value,
    },
}
