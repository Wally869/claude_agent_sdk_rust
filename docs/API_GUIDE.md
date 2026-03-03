# Claude Agent SDK for Rust - API Guide

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
- [Simple Query API](#simple-query-api)
- [Interactive Client API](#interactive-client-api)
- [Hook System](#hook-system)
- [Permission Callbacks](#permission-callbacks)
- [Thinking and Effort](#thinking-and-effort)
- [Structured Output](#structured-output)
- [Budget and Cost Control](#budget-and-cost-control)
- [Configuration Options](#configuration-options)
- [Message Types](#message-types)
- [Error Handling](#error-handling)
- [Examples](#examples)

## Overview

The Claude Agent SDK for Rust provides a production-ready interface to Claude Code CLI, enabling you to build AI agents with full control over permissions, hooks, and agent behavior.

### Key Features

- **Simple Query API** - One-shot queries with streaming responses
- **Interactive Client** - Multi-turn conversations with bidirectional communication
- **Hook System** - Intercept and control agent behavior at key points
- **Permission Callbacks** - Fine-grained control over tool usage
- **Type-Safe** - Fully typed API with compile-time safety
- **Async/Await** - Built on Tokio for efficient async operations

## Quick Start

```rust
use claude_agent_sdk::{query, Message};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simple one-shot query
    let mut messages = query("What is 2 + 2?", None).await?;

    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant(assistant) => {
                println!("Claude: {:?}", assistant);
            }
            Message::Result(result) => {
                println!("Cost: ${:.4}", result.total_cost_usd.unwrap_or(0.0));
            }
            _ => {}
        }
    }

    Ok(())
}
```

## Core Concepts

### 1. Query vs Client

**`query()`** - For simple, one-shot queries:
- Spawns CLI, sends prompt, returns stream of messages
- Automatically cleans up after result
- Best for single questions or tasks

**`ClaudeSDKClient`** - For interactive sessions:
- Maintains persistent connection
- Supports multiple queries in sequence
- Dynamic configuration changes
- Full control protocol access

### 2. Message Stream

All interactions return a stream of `Message` enum variants:

```rust
pub enum Message {
    User(UserMessage),        // User input
    Assistant(AssistantMessage), // Claude's response
    System(SystemMessage),    // System metadata
    Result(ResultMessage),    // Final metrics
    StreamEvent(StreamEvent), // Partial messages
}
```

### 3. Content Blocks

Assistant messages contain content blocks:

```rust
pub enum ContentBlock {
    Text(TextBlock),           // Text output
    Thinking(ThinkingBlock),   // Extended thinking
    ToolUse(ToolUseBlock),     // Tool invocation
    ToolResult(ToolResultBlock), // Tool output
}
```

## Simple Query API

### Basic Usage

```rust
use claude_agent_sdk::query;
use futures::StreamExt;

let messages = query("Explain async/await in Rust", None).await?;
let mut messages = Box::pin(messages);

while let Some(msg) = messages.next().await {
    // Handle message
}
```

### With Options

```rust
use claude_agent_sdk::{query, ClaudeAgentOptions, PermissionMode};

let options = ClaudeAgentOptions::builder()
    .permission_mode(PermissionMode::AcceptEdits)
    .max_turns(5)
    .allowed_tools(vec!["Read".to_string(), "Write".to_string()])
    .build();

let messages = query("Help me refactor this code", Some(options)).await?;
```

## Interactive Client API

### Connection Lifecycle

```rust
use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};

// Create client
let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());

// Connect (starts CLI process)
client.connect(Some("Hello!".to_string())).await?;

// Use client for multiple queries...

// Disconnect (cleans up)
client.disconnect().await?;
```

### Sending Queries

```rust
// Send a query
client.query("What files are in this directory?").await?;

// Receive response
let messages = client.receive_response()?;
let mut messages = Box::pin(messages);

while let Some(msg) = messages.next().await {
    match msg? {
        Message::Assistant(a) => { /* handle */ }
        Message::Result(r) => break, // Auto-terminates
        _ => {}
    }
}
```

### Dynamic Configuration

```rust
// Change permission mode mid-session
client.set_permission_mode(PermissionMode::AcceptAll).await?;

// Change AI model
client.set_model(Some("claude-opus-4-20250514".to_string())).await?;

// Interrupt current processing
client.interrupt().await?;
```

## Hook System

Hooks allow you to intercept and control agent behavior at specific points.

### Hook Events

- `PreToolUse` - Before tool execution
- `PostToolUse` - After tool execution
- `PostToolUseFailure` - After a tool execution fails
- `UserPromptSubmit` - When user submits a prompt
- `Notification` - When the agent sends a notification
- `Stop` - When agent loop stops
- `SubagentStart` - When a subagent starts
- `SubagentStop` - When a subagent stops
- `PreCompact` - Before context compaction
- `PermissionRequest` - When a permission decision is needed

### Implementing a Hook

```rust
use claude_agent_sdk::callbacks::{HookCallback, hooks};
use claude_agent_sdk::types::{HookInput, HookOutput, HookContext};
use async_trait::async_trait;

struct LoggingHook;

#[async_trait]
impl HookCallback for LoggingHook {
    async fn call(
        &self,
        input: HookInput,
        tool_use_id: Option<String>,
        _context: HookContext,
    ) -> claude_agent_sdk::Result<HookOutput> {
        // Match on hook input type
        if let HookInput::PreToolUse(pre_tool) = input {
            println!("Tool: {}", pre_tool.tool_name);
            println!("Input: {:?}", pre_tool.tool_input);
        }

        // Allow execution to continue
        Ok(hooks::allow())
    }
}
```

### Registering Hooks

```rust
use claude_agent_sdk::types::HookEvent;

let mut client = ClaudeSDKClient::new(options);

// Register hook for all tools
let hook_id = client.register_hook(
    HookEvent::PreToolUse,
    None,  // Match all tools
    LoggingHook,
);

// Register hook for specific tools
let hook_id = client.register_hook(
    HookEvent::PreToolUse,
    Some("Bash|Edit"),  // Match Bash or Edit
    SecurityHook,
);
```

### Hook Responses

```rust
use claude_agent_sdk::callbacks::hooks;

// Allow execution
Ok(hooks::allow())

// Block execution
Ok(hooks::block("Reason for blocking"))

// Allow with message to user
Ok(hooks::allow_with_message("Custom message"))

// Defer (async execution)
Ok(hooks::defer(Some(5000))) // 5 second timeout
```

## Permission Callbacks

Permission callbacks control which tools Claude can use and how.

### Implementing a Permission Callback

```rust
use claude_agent_sdk::callbacks::{PermissionCallback, permissions};
use claude_agent_sdk::types::{PermissionResult, ToolPermissionContext};
use async_trait::async_trait;
use serde_json::Value;

struct SafetyChecker;

#[async_trait]
impl PermissionCallback for SafetyChecker {
    async fn call(
        &self,
        tool_name: String,
        input: Value,
        _context: ToolPermissionContext,
    ) -> claude_agent_sdk::Result<PermissionResult> {
        // Block dangerous commands
        if tool_name == "Bash" {
            if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
                if cmd.contains("rm -rf") {
                    return Ok(permissions::deny("Dangerous command blocked"));
                }
            }
        }

        // Allow by default
        Ok(permissions::allow())
    }
}
```

### Registering Permission Callback

```rust
let mut client = ClaudeSDKClient::new(options);

// Set permission callback (only one active at a time)
client.set_permission_callback(SafetyChecker);
```

### Permission Responses

```rust
use claude_agent_sdk::callbacks::permissions;

// Allow tool use
Ok(permissions::allow())

// Allow with modified input
Ok(permissions::allow_with_input(modified_input))

// Deny tool use
Ok(permissions::deny("Not allowed"))

// Deny and stop session
Ok(permissions::deny_and_interrupt("Critical violation"))
```

## Thinking and Effort

Control Claude's reasoning depth with thinking configuration and effort levels.

### ThinkingConfig

```rust
use claude_agent_sdk::{ClaudeAgentOptions, ThinkingConfig};

// Adaptive: Claude decides when to use extended thinking
let options = ClaudeAgentOptions::builder()
    .thinking(Some(ThinkingConfig::Adaptive))
    .build();

// Enabled with explicit token budget
let options = ClaudeAgentOptions::builder()
    .thinking(Some(ThinkingConfig::Enabled { budget_tokens: 10000 }))
    .build();

// Disabled
let options = ClaudeAgentOptions::builder()
    .thinking(Some(ThinkingConfig::Disabled))
    .build();
```

Maps to the `--max-thinking-tokens` CLI flag. `Adaptive` passes `0`, `Enabled` passes the budget, `Disabled` omits the flag.

### Effort

```rust
use claude_agent_sdk::{ClaudeAgentOptions, Effort};

let options = ClaudeAgentOptions::builder()
    .effort(Some(Effort::High))
    .build();
```

**Levels:** `Low`, `Medium`, `High`, `Max`. Maps to the `--effort` CLI flag.

## Structured Output

Get Claude's response as validated JSON conforming to a schema. The `output_format` option accepts a JSON value with `type: "json_schema"` and a `schema` field containing a standard JSON Schema.

> **Important:** Structured output typically requires more than 1 turn. Set `max_turns` to at least 2-3 or omit it entirely. Using `max_turns(1)` will likely produce `error_max_turns` with no structured output.

```rust
use claude_agent_sdk::{query, ClaudeAgentOptions, Message};
use futures::StreamExt;

let schema = serde_json::json!({
    "type": "json_schema",
    "schema": {
        "type": "object",
        "properties": {
            "summary": { "type": "string" },
            "language": { "type": "string" },
            "line_count": { "type": "number" }
        },
        "required": ["summary", "language", "line_count"]
    }
});

let options = ClaudeAgentOptions::builder()
    .output_format(Some(schema))
    .build();

let mut messages = query("Analyze src/main.rs", Some(options)).await?;
let mut messages = Box::pin(messages);

while let Some(msg) = messages.next().await {
    if let Ok(Message::Result(result)) = msg {
        if let Some(output) = &result.structured_output {
            // output is a serde_json::Value matching your schema
            println!("{}", serde_json::to_string_pretty(output)?);
        }
    }
}
```

The structured output is available on `ResultMessage.structured_output` when the query completes with `subtype: "success"`.

## Budget and Cost Control

### max_budget_usd

Set a per-query spending limit:

```rust
let options = ClaudeAgentOptions::builder()
    .max_budget_usd(Some(0.10))
    .build();
```

> **Important:** This is a **soft cap**. The budget is checked between turns — the current turn always completes before the limit is evaluated. Actual spend may slightly exceed the configured value.

### Fallback Model

Specify a model to fall back to if the primary model is unavailable:

```rust
let options = ClaudeAgentOptions::builder()
    .model(Some("claude-opus-4-20250514".into()))
    .fallback_model(Some("claude-sonnet-4-5-20250929".into()))
    .build();
```

## Configuration Options

### ClaudeAgentOptions

```rust
use claude_agent_sdk::{
    ClaudeAgentOptions, PermissionMode, SystemPrompt, SettingSource,
    ThinkingConfig, Effort,
};

let options = ClaudeAgentOptions::builder()
    // Permission mode
    .permission_mode(PermissionMode::AcceptEdits)

    // Allowed tools (whitelist)
    .allowed_tools(vec!["Read".into(), "Write".into()])

    // Blocked tools (blacklist)
    .blocked_tools(vec!["Bash".into()])

    // System prompt
    .system_prompt(SystemPrompt::Text(
        "You are a Rust expert assistant".into()
    ))

    // Model selection
    .model(Some("claude-sonnet-4-5-20250929".into()))
    .fallback_model(Some("claude-haiku-4-5-20251001".into()))

    // Conversation limits
    .max_turns(10u32)
    .max_budget_usd(Some(0.50))

    // Thinking and effort
    .thinking(Some(ThinkingConfig::Adaptive))
    .effort(Some(Effort::High))

    // Structured output
    .output_format(Some(serde_json::json!({
        "type": "json_schema",
        "schema": { "type": "object", "properties": {} }
    })))

    // CLI path (auto-detected if not set)
    .cli_path(Some("/path/to/claude".into()))

    // Settings sources
    .setting_sources(Some(vec![
        SettingSource::User,
        SettingSource::Project,
    ]))

    .build();
```

### Permission Modes

```rust
pub enum PermissionMode {
    AcceptAll,    // Auto-approve all tools
    AcceptEdits,  // Auto-approve file edits only
    Ask,          // Prompt for each tool (default)
    Callback,     // Use permission callback
}
```

## Message Types

### AssistantMessage

```rust
pub struct AssistantMessage {
    pub message: AssistantMessageInner,
    pub parent_tool_use_id: Option<String>,
}

pub struct AssistantMessageInner {
    pub role: String,  // "assistant"
    pub content: Vec<ContentBlock>,
}
```

### ResultMessage

```rust
pub struct ResultMessage {
    pub subtype: String,                        // "success" | "error_max_turns" | ...
    pub duration_ms: u64,
    pub duration_api_ms: u64,
    pub is_error: bool,
    pub num_turns: u32,
    pub session_id: String,
    pub total_cost_usd: Option<f64>,
    pub usage: Option<Value>,
    pub result: Option<String>,                 // Result text (if any)
    pub structured_output: Option<Value>,       // JSON output when output_format is set
}
```

### SystemMessage

```rust
pub struct SystemMessage {
    pub subtype: String,
    pub content: Option<Value>,
}
```

## Error Handling

### Error Types

```rust
pub enum ClaudeSDKError {
    CLINotFound(String),
    TransportError(String),
    MessageParse(String),
    NotConnected,
    AlreadyConnected,
    HookNotFound(String),
    PermissionCallbackNotSet,
    ControlTimeout { timeout_seconds: u64, request: String },
    Other(String),
}
```

### Handling Errors

```rust
use claude_agent_sdk::ClaudeSDKError;

match query("prompt", None).await {
    Ok(stream) => { /* handle stream */ }
    Err(ClaudeSDKError::CLINotFound(msg)) => {
        eprintln!("Claude CLI not found: {}", msg);
        eprintln!("Install from: https://claude.com/claude-code");
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Examples

### Example 1: Simple Code Review

```rust
use claude_agent_sdk::{query, Message, ClaudeAgentOptions};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeAgentOptions::builder()
        .allowed_tools(vec!["Read".into()])
        .system_prompt(SystemPrompt::Text(
            "You are a code reviewer. Focus on best practices.".into()
        ))
        .build();

    let messages = query(
        "Review the code in src/main.rs",
        Some(options)
    ).await?;

    let mut messages = Box::pin(messages);

    while let Some(msg) = messages.next().await {
        if let Message::Assistant(a) = msg? {
            for block in &a.message.content {
                if let Some(text) = block.as_text() {
                    println!("{}", text.text);
                }
            }
        }
    }

    Ok(())
}
```

### Example 2: Interactive Refactoring

```rust
use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions, Message};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeAgentOptions::builder()
        .permission_mode(PermissionMode::AcceptEdits)
        .build();

    let mut client = ClaudeSDKClient::new(options);
    client.connect(None).await?;

    // First query
    client.query("Refactor src/utils.rs to use modern Rust patterns").await?;
    let messages = client.receive_response()?;
    let mut messages = Box::pin(messages);

    while let Some(msg) = messages.next().await {
        if let Message::Result(_) = msg? { break; }
    }
    drop(messages);

    // Follow-up query
    client.query("Add tests for the refactored code").await?;
    let messages = client.receive_response()?;
    let mut messages = Box::pin(messages);

    while let Some(msg) = messages.next().await {
        if let Message::Result(_) = msg? { break; }
    }
    drop(messages);

    client.disconnect().await?;
    Ok(())
}
```

### Example 3: Safety-Enhanced Agent

```rust
use claude_agent_sdk::{
    ClaudeSDKClient, ClaudeAgentOptions, Message,
    callbacks::{PermissionCallback, permissions, HookCallback, hooks},
    types::{HookEvent, HookInput, HookOutput, HookContext,
            PermissionResult, ToolPermissionContext},
};
use async_trait::async_trait;
use serde_json::Value;

struct CommandValidator;

#[async_trait]
impl PermissionCallback for CommandValidator {
    async fn call(
        &self,
        tool_name: String,
        input: Value,
        _ctx: ToolPermissionContext,
    ) -> claude_agent_sdk::Result<PermissionResult> {
        if tool_name == "Bash" {
            if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
                // Block network access
                if cmd.contains("curl") || cmd.contains("wget") {
                    return Ok(permissions::deny("Network access blocked"));
                }
            }
        }
        Ok(permissions::allow())
    }
}

struct AuditLogger;

#[async_trait]
impl HookCallback for AuditLogger {
    async fn call(
        &self,
        input: HookInput,
        _tool_use_id: Option<String>,
        _ctx: HookContext,
    ) -> claude_agent_sdk::Result<HookOutput> {
        if let HookInput::PreToolUse(pre_tool) = input {
            // Log to audit file
            println!("[AUDIT] {} - {:?}", pre_tool.tool_name, pre_tool.tool_input);
        }
        Ok(hooks::allow())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());

    // Register security controls
    client.set_permission_callback(CommandValidator);
    client.register_hook(HookEvent::PreToolUse, None, AuditLogger);

    client.connect(None).await?;
    client.query("Analyze system logs").await?;

    // Process messages...

    client.disconnect().await?;
    Ok(())
}
```

## Best Practices

### 1. Stream Handling

Always pin streams before iterating:

```rust
let messages = client.receive_response()?;
let mut messages = Box::pin(messages);  // Pin the stream

while let Some(msg) = messages.next().await {
    // Handle messages
}
```

### 2. Resource Cleanup

Drop streams before reusing client:

```rust
let messages = client.receive_response()?;
let mut messages = Box::pin(messages);

// Process messages...

drop(messages);  // Drop stream before next query

client.query("Next question").await?;
```

### 3. Error Handling

Handle specific error types:

```rust
match client.connect(None).await {
    Ok(_) => { /* connected */ }
    Err(ClaudeSDKError::CLINotFound(_)) => {
        // Provide installation instructions
    }
    Err(ClaudeSDKError::AlreadyConnected) => {
        // Client is already connected
    }
    Err(e) => eprintln!("Connection error: {}", e),
}
```

### 4. Permission Callbacks

Always return a result - never panic:

```rust
#[async_trait]
impl PermissionCallback for MyChecker {
    async fn call(...) -> Result<PermissionResult> {
        // Validate inputs
        // Return allow/deny
        // Never panic!
        Ok(permissions::allow())
    }
}
```

### 5. Hook Callbacks

Keep hooks fast - avoid blocking operations:

```rust
#[async_trait]
impl HookCallback for FastHook {
    async fn call(...) -> Result<HookOutput> {
        // Quick validation/logging only
        // Defer heavy operations to async hooks
        Ok(hooks::allow())
    }
}
```

## Troubleshooting

### CLI Not Found

```bash
# Install Claude Code CLI
# Visit: https://claude.com/claude-code

# Or set custom path
export CLAUDE_CLI_PATH=/path/to/claude
```

### Authentication Required

```bash
# Authenticate with Claude
claude auth login
```

### Version Mismatch

```bash
# Check CLI version (requires 0.2.0+)
claude --version

# Skip version check
export CLAUDE_AGENT_SDK_SKIP_VERSION_CHECK=1
```

## Further Reading

- [README.md](../README.md) - Installation and setup
- [examples/](../examples/) - Complete working examples
- [Rust API Documentation](https://docs.rs/claude-agent-sdk) - Full API reference
- [Claude Code Documentation](https://docs.claude.com/claude-code) - CLI documentation

## Support

- GitHub Issues: https://github.com/anthropics/claude-agent-sdk-rust/issues
- Claude Code: https://claude.com/claude-code
