//! Content block types for Claude messages.

use serde::{Deserialize, Serialize};

/// A block of content within a message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain text content.
    Text(TextBlock),
    /// Thinking/reasoning content (extended thinking).
    Thinking(ThinkingBlock),
    /// Tool use request from Claude.
    ToolUse(ToolUseBlock),
    /// Tool execution result.
    ToolResult(ToolResultBlock),
}

/// Plain text content block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextBlock {
    /// The text content.
    pub text: String,
}

/// Thinking content block (Claude's reasoning).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThinkingBlock {
    /// The thinking/reasoning content.
    pub thinking: String,
    /// Cryptographic signature for verification.
    pub signature: String,
}

/// Tool use block (request to execute a tool).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolUseBlock {
    /// Unique identifier for this tool use.
    pub id: String,
    /// Name of the tool to execute.
    pub name: String,
    /// Tool-specific input parameters.
    pub input: serde_json::Value,
}

/// Tool result block (result of tool execution).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResultBlock {
    /// ID of the corresponding tool use.
    pub tool_use_id: String,
    /// Result content (can be string or structured data).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    /// Whether the tool execution resulted in an error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl ContentBlock {
    /// Returns true if this is a text block.
    pub fn is_text(&self) -> bool {
        matches!(self, ContentBlock::Text(_))
    }

    /// Returns true if this is a thinking block.
    pub fn is_thinking(&self) -> bool {
        matches!(self, ContentBlock::Thinking(_))
    }

    /// Returns true if this is a tool use block.
    pub fn is_tool_use(&self) -> bool {
        matches!(self, ContentBlock::ToolUse(_))
    }

    /// Returns true if this is a tool result block.
    pub fn is_tool_result(&self) -> bool {
        matches!(self, ContentBlock::ToolResult(_))
    }

    /// Attempts to extract as TextBlock.
    pub fn as_text(&self) -> Option<&TextBlock> {
        if let ContentBlock::Text(t) = self {
            Some(t)
        } else {
            None
        }
    }

    /// Attempts to extract as ToolUseBlock.
    pub fn as_tool_use(&self) -> Option<&ToolUseBlock> {
        if let ContentBlock::ToolUse(t) = self {
            Some(t)
        } else {
            None
        }
    }
}
