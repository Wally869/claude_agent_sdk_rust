//! Message parser for Claude CLI output.

use crate::error::{ClaudeSDKError, Result};
use crate::types::Message;

/// Known message types that we can parse.
const KNOWN_TYPES: &[&str] = &["user", "assistant", "system", "result", "stream_event"];

/// Parse a JSON value into a typed Message.
///
/// Returns `Ok(message)` for known types, or `Err` with a skippable parse error
/// for unknown types. This is forward-compatible: newer CLI versions may send
/// message types that older SDK versions don't know about.
pub fn parse_message(value: serde_json::Value) -> Result<Message> {
    // Check if this is a known message type before attempting deserialization
    if let Some(msg_type) = value.get("type").and_then(|v| v.as_str())
        && !KNOWN_TYPES.contains(&msg_type)
    {
        // Skip unknown message types (forward-compatible)
        return Err(ClaudeSDKError::UnknownMessageType(msg_type.to_string()));
    }

    serde_json::from_value(value).map_err(|e| ClaudeSDKError::message_parse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_assistant_message() {
        let json = json!({
            "type": "assistant",
            "message": {
                "content": [
                    {"type": "text", "text": "Hello!"}
                ],
                "model": "claude-sonnet-4-5"
            }
        });

        let msg = parse_message(json).unwrap();
        assert!(msg.is_assistant());
    }

    #[test]
    fn test_parse_result_message() {
        let json = json!({
            "type": "result",
            "subtype": "success",
            "duration_ms": 1000,
            "duration_api_ms": 800,
            "is_error": false,
            "num_turns": 1,
            "session_id": "test",
            "total_cost_usd": 0.01
        });

        let msg = parse_message(json).unwrap();
        assert!(msg.is_result());
    }

    #[test]
    fn test_skip_unknown_message_type() {
        let json = json!({
            "type": "rate_limit_event",
            "data": {}
        });

        let result = parse_message(json);
        assert!(matches!(result, Err(ClaudeSDKError::UnknownMessageType(_))));
    }
}
