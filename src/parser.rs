//! Message parser for Claude CLI output.

use crate::error::{ClaudeSDKError, Result};
use crate::types::Message;

/// Parse a JSON value into a typed Message.
pub fn parse_message(value: serde_json::Value) -> Result<Message> {
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
}
