# Claude Agent SDK for Rust

Rust SDK for building production-ready AI agents with Claude. Replicate the full feature set of the Python Claude Agent SDK with idiomatic Rust patterns.

## Overview

The Claude Agent SDK enables you to build powerful AI agents using Claude Code's agent harness. This SDK wraps the Claude Code CLI, providing type-safe access to:

- Automatic context management and compaction
- 20+ built-in tools (file operations, code execution, web search)
- Custom MCP (Model Context Protocol) tools
- Fine-grained permission controls
- Hook system for deterministic behavior
- Interactive bidirectional conversations
- Usage tracking and quota monitoring (Max Plan)

## Important Notes

> **Structured output requires more than 1 turn.** When using `output_format` with a JSON schema, the CLI may need additional turns internally to produce structured JSON. Set `max_turns` to at least 2-3 (or omit it entirely) — using `max_turns(1)` will likely result in `error_max_turns` with no structured output.

> **`max_budget_usd` is a soft cap.** The budget limit is checked between turns, not mid-generation. The current turn will always complete before the budget is evaluated, so actual spend may slightly exceed the configured limit.

## Prerequisites

- **Rust**: 1.70 or higher
- **Claude Code CLI**: 2.0.0 or higher
- **Node.js**: Required to install Claude Code
- **Authentication**: Claude subscription (Pro, Team, or Enterprise) or Anthropic API key

## Installation

### 1. Install Claude Code CLI

```bash
npm install -g @anthropic-ai/claude-code
```

Verify installation:
```bash
claude -v
# Should output: 2.0.0 or higher
```

### 2. Add SDK to Your Project

```toml
[dependencies]
claude-agent-sdk = "0.1"
tokio = { version = "1", features = ["full"] }
futures = "0.3"
```

## Authentication Setup

Claude Code supports multiple authentication methods:

### Option 1: Claude Subscription (Recommended)

If you have a Claude Pro, Team, or Enterprise subscription:

```bash
claude setup-token
```

This will authenticate using your Claude subscription. No additional configuration needed!

### Option 2: API Key

If you're using an Anthropic API key:

1. Get your API key from https://console.anthropic.com/account/keys
2. Set the environment variable:

**Linux/macOS:**
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

**Windows (PowerShell):**
```powershell
$env:ANTHROPIC_API_KEY="sk-ant-..."
```

**Windows (Command Prompt):**
```cmd
set ANTHROPIC_API_KEY=sk-ant-...
```

Verify authentication works:
```bash
claude --print "Hello, Claude!"
```

## Quick Start

### Simple Query

```rust
use claude_agent_sdk::{query, Message};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut messages = query("What is 2 + 2?", None).await?;

    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant(assistant) => {
                for block in &assistant.message.content {
                    if let Some(text) = block.as_text() {
                        println!("Claude: {}", text.text);
                    }
                }
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

### With Configuration

```rust
use claude_agent_sdk::{query, ClaudeAgentOptions, PermissionMode, SystemPrompt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeAgentOptions::builder()
        .allowed_tools(vec!["Read".into(), "Write".into()])
        .permission_mode(PermissionMode::AcceptEdits)
        .system_prompt(SystemPrompt::Text(
            "You are a helpful file assistant".to_string()
        ))
        .build();

    let mut messages = query("Create a hello.txt file", Some(options)).await?;

    // Process messages...

    Ok(())
}
```

### Interactive Conversation

```rust
use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions, Message};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeAgentOptions::builder()
        .allowed_tools(vec!["Read".into(), "Bash".into()])
        .build();

    let mut client = ClaudeSDKClient::new(options);
    client.connect(None).await?;

    // First query
    client.query("List files in current directory").await?;
    let mut response = client.receive_response()?;
    while let Some(msg) = response.next().await {
        if let Ok(Message::Result(_)) = msg {
            break;
        }
    }
    drop(response);

    // Follow-up query
    client.query("Read the first file").await?;
    let mut response = client.receive_response()?;
    while let Some(msg) = response.next().await {
        println!("{:?}", msg?);
    }
    drop(response);

    client.disconnect().await?;
    Ok(())
}
```

## Key Features

### Tool Control

```rust
let options = ClaudeAgentOptions::builder()
    .allowed_tools(vec!["Read", "Write", "Bash"])
    .disallowed_tools(vec!["WebSearch"])  // Block specific tools
    .build();
```

### Permission Modes

- **Default**: Prompt for dangerous operations
- **AcceptEdits**: Auto-accept file edits
- **Plan**: Plan mode (no execution)
- **BypassPermissions**: Allow all tools (use with caution!)

```rust
.permission_mode(PermissionMode::AcceptEdits)
```

### System Prompts

```rust
// Text prompt
.system_prompt(SystemPrompt::Text(
    "You are an expert Rust developer".to_string()
))

// Or use Claude Code preset
.system_prompt(SystemPrompt::Preset(SystemPromptPreset {
    preset_type: "preset".to_string(),
    preset: "claude_code".to_string(),
    append: Some("Focus on Rust best practices".to_string())
}))
```

### Working Directory

```rust
.cwd("/path/to/your/project")
```

### Model Selection

```rust
.model(Some("claude-opus-4-20250514".to_string()))
```

### Thinking and Effort

Control Claude's reasoning depth:

```rust
use claude_agent_sdk::{query, ClaudeAgentOptions, ThinkingConfig, Effort};

let options = ClaudeAgentOptions::builder()
    .thinking(Some(ThinkingConfig::Adaptive))  // Let Claude decide when to think
    .effort(Some(Effort::High))                // More thorough responses
    .build();

let messages = query("Solve this complex problem...", Some(options)).await?;
```

**ThinkingConfig variants:**
- `Adaptive` — Claude decides when extended thinking is useful
- `Enabled { budget_tokens }` — Always think, with a token budget
- `Disabled` — No extended thinking

**Effort levels:** `Low`, `Medium`, `High`, `Max`

### Structured Output

Get responses as validated JSON matching a schema:

```rust
use claude_agent_sdk::{query, ClaudeAgentOptions, Message};

let schema = serde_json::json!({
    "type": "json_schema",
    "schema": {
        "type": "object",
        "properties": {
            "capital": { "type": "string" },
            "population": { "type": "string" }
        },
        "required": ["capital", "population"]
    }
});

let options = ClaudeAgentOptions::builder()
    .output_format(Some(schema))
    .max_turns(3u32)  // Structured output needs >1 turn
    .build();

let mut messages = query("What is the capital and population of Japan?", Some(options)).await?;

while let Some(msg) = messages.next().await {
    if let Ok(Message::Result(result)) = msg {
        if let Some(output) = &result.structured_output {
            println!("{}", output);
            // {"capital": "Tokyo", "population": "approximately 125 million"}
        }
    }
}
```

### Budget Limits

Set a soft spending cap per query:

```rust
let options = ClaudeAgentOptions::builder()
    .max_budget_usd(Some(0.05))  // Soft cap — see Important Notes above
    .build();
```

### Fallback Model

Specify a fallback model if the primary is unavailable:

```rust
let options = ClaudeAgentOptions::builder()
    .model(Some("claude-opus-4-20250514".into()))
    .fallback_model(Some("claude-sonnet-4-5-20250929".into()))
    .build();
```

## Advanced Features

### Permission Callbacks

Programmatically control tool usage with the `PermissionCallback` trait:

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
        if tool_name == "Bash" {
            if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
                if cmd.contains("rm -rf") {
                    return Ok(permissions::deny("Dangerous command blocked"));
                }
            }
        }
        Ok(permissions::allow())
    }
}

// Use with client
let mut client = ClaudeSDKClient::new(options);
client.set_permission_callback(SafetyChecker);
client.connect(None).await?;
```

### Hook System

Execute custom code at specific points in the agent loop:

```rust
use claude_agent_sdk::callbacks::{HookCallback, hooks};
use claude_agent_sdk::types::{HookInput, HookOutput, HookContext, HookEvent};
use async_trait::async_trait;

struct ValidationHook;

#[async_trait]
impl HookCallback for ValidationHook {
    async fn call(
        &self,
        input: HookInput,
        _tool_use_id: Option<String>,
        _context: HookContext,
    ) -> claude_agent_sdk::Result<HookOutput> {
        if let HookInput::PreToolUse(pre) = input {
            if pre.tool_name == "Bash" {
                if let Some(cmd) = pre.tool_input.get("command")
                    .and_then(|v| v.as_str()) {
                    if cmd.contains("dangerous") {
                        return Ok(hooks::block("Blocked dangerous command"));
                    }
                }
            }
        }
        Ok(hooks::allow())
    }
}

// Register hook
let mut client = ClaudeSDKClient::new(options);
client.register_hook(HookEvent::PreToolUse, None, ValidationHook);
client.connect(None).await?;
```

### MCP Server Configuration

Use external MCP servers for custom tools:

```rust
use std::collections::HashMap;
use claude_agent_sdk::types::{McpServerConfig, McpStdioConfig};

let mut mcp_servers = HashMap::new();
mcp_servers.insert("calculator".into(), McpServerConfig::Stdio(
    McpStdioConfig {
        command: "python".into(),
        args: Some(vec!["-m".into(), "calculator_server".into()]),
        env: None
    }
));

let options = ClaudeAgentOptions::builder()
    .mcp_servers(mcp_servers)
    .allowed_tools(vec!["mcp__calculator__add", "mcp__calculator__multiply"])
    .build();
```

## Available Tools

Claude Code includes 20+ built-in tools:

- **File Operations**: Read, Write, Edit, Glob
- **Code Execution**: Bash, NotebookEdit
- **Search**: Grep, WebSearch, WebFetch
- **Communication**: Task (subagents), SlashCommand
- **And more**: See [Claude Code documentation](https://docs.anthropic.com/en/docs/claude-code/settings#tools-available-to-claude)

## Error Handling

```rust
use claude_agent_sdk::ClaudeSDKError;

match query("test", None).await {
    Ok(messages) => { /* process */ }
    Err(ClaudeSDKError::CLINotFound { path }) => {
        eprintln!("Claude CLI not found at: {:?}", path);
        eprintln!("Install with: npm install -g @anthropic-ai/claude-code");
    }
    Err(ClaudeSDKError::Process { exit_code, message, stderr }) => {
        eprintln!("Process failed (exit {}): {}", exit_code, message);
        if let Some(err) = stderr {
            eprintln!("Details: {}", err);
        }
    }
    Err(ClaudeSDKError::ControlTimeout { timeout_secs, request_type }) => {
        eprintln!("Timeout after {}s waiting for: {}", timeout_secs, request_type);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## Session Management

The SDK provides full support for managing conversation sessions, allowing you to:
- Capture session IDs from conversations
- Resume previous conversations with full context
- Continue from the most recent conversation

### Capturing Session IDs

Session IDs are automatically captured from messages:

```rust
use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions, Message};
use futures::StreamExt;

let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());
client.connect(Some("Hello!".to_string())).await?;

// Process messages
let mut messages = client.receive_messages()?;
while let Some(msg) = messages.next().await {
    match msg? {
        Message::Result(result) => {
            // Session ID is available from result message
            let session_id = result.session_id;
            println!("Session: {}", session_id);
        }
        _ => {}
    }
}

// Or get it directly from the client
if let Some(session_id) = client.get_session_id() {
    println!("Current session: {}", session_id);
}
```

### Resuming Sessions

Resume a specific conversation by session ID:

```rust
let options = ClaudeAgentOptions::builder()
    .resume("session-id-here".to_string())
    .build();

let mut client = ClaudeSDKClient::new(options);
client.connect(Some("Continue our conversation...".to_string())).await?;
```

### Continuing Most Recent

Continue the most recent conversation:

```rust
let options = ClaudeAgentOptions::builder()
    .continue_conversation(true)
    .build();

let mut client = ClaudeSDKClient::new(options);
client.connect(Some("As we were discussing...".to_string())).await?;
```

### Forking Sessions

Create a new session ID when resuming (for experimentation):

```rust
let options = ClaudeAgentOptions::builder()
    .resume("original-session-id".to_string())
    .fork_session(true)  // Creates new ID instead of reusing
    .build();
```

Sessions are stored in `~/.claude/projects/<project>/<session-id>.jsonl` and preserve full conversation context including:
- All messages
- Tool usage history
- Context and state

See `examples/session_resume.rs` for a complete working example.

## Examples

See the `examples/` directory for complete working examples:

- **`basic.rs`** - Simple one-shot query
- **`with_options.rs`** - Configuration examples
- **`interactive.rs`** - Bidirectional conversation
- **`with_callbacks.rs`** - Hooks and permission callbacks
- **`session_resume.rs`** - Session management and resuming conversations
- **`usage_tracking.rs`** - Monitor Claude Code usage and quotas (Max Plan)
- **`new_features.rs`** - Thinking, effort, budget limits, structured output, MCP status

Run examples:
```bash
cargo run --example basic
cargo run --example interactive
cargo run --example with_callbacks
cargo run --example session_resume
cargo run --example usage_tracking
cargo run --example new_features
```

## Documentation

### API Guide

See [docs/API_GUIDE.md](docs/API_GUIDE.md) for comprehensive documentation.

### Rust API Documentation

Generate and view full API documentation:

```bash
cargo doc --open
```

## Troubleshooting

### CLI Not Found

```
Error: Claude Code CLI not found
```

**Solution**: Install CLI and ensure it's in PATH:
```bash
npm install -g @anthropic-ai/claude-code
which claude  # Unix/macOS
where claude  # Windows
```

Or set custom path:
```rust
.cli_path(Some("/path/to/claude".into()))
```

### Authentication Failed

```
Error: Authentication failed
```

**Solutions**:
1. If using Claude subscription: Run `claude setup-token`
2. If using API key: Set `ANTHROPIC_API_KEY` environment variable
3. Verify: Run `claude --print "test"` to check authentication

### Version Mismatch

```
Warning: Claude Code version 1.x.x < minimum 2.0.0
```

**Solution**: Update Claude Code:
```bash
npm update -g @anthropic-ai/claude-code
```

### Process Timeouts

If you're getting timeout errors for initialization or control requests:

```rust
// The SDK uses 60s timeouts by default for control protocol messages
// If your queries need more time, consider using streaming mode with
// the ClaudeSDKClient instead of the one-shot query() function
```

## Development

Build the project:
```bash
cargo build
```

Run tests:
```bash
# Unit tests
cargo test --lib

# Integration tests (requires authentication)
cargo test --test integration_test -- --ignored --test-threads=1
```

Format and lint:
```bash
cargo fmt
cargo clippy
```

## Resources

- [API Guide](docs/API_GUIDE.md) - Comprehensive API documentation
- [Claude Code Documentation](https://docs.claude.com/en/docs/claude-code)
- [Anthropic Console](https://console.anthropic.com)

## License

MIT

## Support

- GitHub Issues: For bug reports and feature requests
- Documentation: `cargo doc --open` for API reference
- Claude Code Help: https://docs.claude.com/en/docs/claude-code

---

**Note**: This SDK wraps the Claude Code CLI. Make sure it's installed and authenticated before use.
