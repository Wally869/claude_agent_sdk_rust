//! MCP (Model Context Protocol) server configuration types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpServerConfig {
    /// Stdio-based MCP server (subprocess).
    Stdio(McpStdioConfig),
    /// SSE (Server-Sent Events) based MCP server.
    Sse(McpSseConfig),
    /// HTTP-based MCP server.
    Http(McpHttpConfig),
    /// SDK-based in-process MCP server (future feature).
    Sdk(McpSdkServerConfig),
}

/// Configuration for stdio MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpStdioConfig {
    /// Command to execute.
    pub command: String,
    /// Command arguments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Environment variables.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
}

/// Configuration for SSE MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpSseConfig {
    /// Server URL.
    pub url: String,
    /// HTTP headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

/// Configuration for HTTP MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpHttpConfig {
    /// Server URL.
    pub url: String,
    /// HTTP headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

/// Configuration for SDK MCP server (in-process).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpSdkServerConfig {
    /// Server name.
    pub name: String,
    /// Note: The actual server instance cannot be serialized.
    /// It's stored separately and not passed to CLI.
    #[serde(skip)]
    pub instance: Option<()>, // Placeholder for future MCP server instance
}
