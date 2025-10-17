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

## Examples

See the `examples/` directory for complete working examples:

- **`basic.rs`** - Simple one-shot query
- **`with_options.rs`** - Configuration examples
- **`interactive.rs`** - Bidirectional conversation
- **`with_callbacks.rs`** - Hooks and permission callbacks

Run examples:
```bash
cargo run --example basic
cargo run --example interactive
cargo run --example with_callbacks
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
