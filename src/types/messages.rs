//! Message types for Claude Agent SDK.

use serde::{Deserialize, Serialize};
use super::content::ContentBlock;

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// Message from the user.
    User(UserMessage),
    /// Response from Claude.
    Assistant(AssistantMessage),
    /// System metadata message.
    System(SystemMessage),
    /// Final result with metrics.
    Result(ResultMessage),
    /// Streaming event (when include_partial_messages is true).
    StreamEvent(StreamEvent),
}

/// User message content (can be string or blocks).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content.
    Text(String),
    /// Structured content blocks.
    Blocks(Vec<ContentBlock>),
}

/// Message from the user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserMessage {
    /// The message content.
    pub message: UserMessageInner,
    /// Unique message identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    /// Parent tool use ID (for subagents).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_use_id: Option<String>,
    /// Tool use result data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_result: Option<serde_json::Value>,
}

/// Inner structure of user message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserMessageInner {
    /// Role (always "user").
    pub role: String,
    /// Message content.
    pub content: MessageContent,
}

/// Response from Claude.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantMessage {
    /// The message content.
    pub message: AssistantMessageInner,
    /// Parent tool use ID (for subagents).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_use_id: Option<String>,
    /// Session ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Error type (e.g., "rate_limit", "server_error").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Inner structure of assistant message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantMessageInner {
    /// Content blocks.
    pub content: Vec<ContentBlock>,
    /// Message ID from API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Model that generated this response.
    pub model: String,
    /// Role (always "assistant").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Stop reason (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// Stop sequence (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
    /// Message type from API.
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub message_type: Option<String>,
    /// Token usage information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<serde_json::Value>,
}

/// System metadata message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemMessage {
    /// Subtype of system message.
    pub subtype: String,
    /// Arbitrary data payload.
    #[serde(flatten)]
    pub data: serde_json::Value,
}

/// Final result message with metrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResultMessage {
    /// Result subtype (success/error).
    pub subtype: String,
    /// Total wall-clock time in milliseconds.
    pub duration_ms: u64,
    /// Time spent in API calls in milliseconds.
    pub duration_api_ms: u64,
    /// Whether this is an error result.
    pub is_error: bool,
    /// Number of conversation turns.
    pub num_turns: u32,
    /// Session identifier.
    pub session_id: String,
    /// Total estimated cost in USD.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,
    /// Token usage information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<serde_json::Value>,
    /// Result text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    /// Structured output (when output_format is configured).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_output: Option<serde_json::Value>,
}

/// Streaming event for partial message updates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StreamEvent {
    /// Unique event identifier.
    pub uuid: String,
    /// Session identifier.
    pub session_id: String,
    /// Raw Anthropic API stream event.
    pub event: serde_json::Value,
    /// Parent tool use ID (for subagents).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_use_id: Option<String>,
}

impl Message {
    /// Returns true if this is a user message.
    pub fn is_user(&self) -> bool {
        matches!(self, Message::User(_))
    }

    /// Returns true if this is an assistant message.
    pub fn is_assistant(&self) -> bool {
        matches!(self, Message::Assistant(_))
    }

    /// Returns true if this is a system message.
    pub fn is_system(&self) -> bool {
        matches!(self, Message::System(_))
    }

    /// Returns true if this is a result message.
    pub fn is_result(&self) -> bool {
        matches!(self, Message::Result(_))
    }

    /// Returns true if this is a stream event.
    pub fn is_stream_event(&self) -> bool {
        matches!(self, Message::StreamEvent(_))
    }
}
