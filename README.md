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
- **Anthropic API Key**: From Console or Claude Max subscription

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

### Get Your API Key

**Option 1: Claude Console**
1. Visit https://console.anthropic.com/account/keys
2. Create a new API key

**Option 2: From Claude Code** (if already configured)
```bash
claude config get apiKey
```

### Set Environment Variable

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

### Claude Max Integration

If you have a Claude Max subscription, **it works automatically**:
- Use the same API key from your Claude Max account
- No special configuration needed
- Authentication handled by the CLI

## Quick Start

### Simple Query

```rust
use claude_agent_sdk::{query, Message, AssistantMessage, TextBlock};
use futures::stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut messages = query("What is 2 + 2?", None).await?;

    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant(AssistantMessage { content, .. }) => {
                for block in content {
                    if let TextBlock { text } = block {
                        println!("Claude: {}", text);
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
use claude_agent_sdk::{query, ClaudeAgentOptions, PermissionMode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeAgentOptions::builder()
        .allowed_tools(vec!["Read".into(), "Write".into()])
        .permission_mode(PermissionMode::AcceptEdits)
        .system_prompt("You are a helpful file assistant".into())
        .build();

    let mut messages = query("Create a hello.txt file", Some(options)).await?;

    // Process messages...

    Ok(())
}
```

### Interactive Conversation

```rust
use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
use futures::stream::StreamExt;

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
        println!("{:?}", msg?);
    }

    // Follow-up query
    client.query("Read the first file").await?;
    let mut response = client.receive_response()?;
    while let Some(msg) = response.next().await {
        println!("{:?}", msg?);
    }

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
.system_prompt("You are an expert Rust developer")

// Or use Claude Code preset
.system_prompt(SystemPrompt::Preset(SystemPromptPreset {
    preset_type: "preset",
    preset: "claude_code",
    append: Some("Focus on Rust best practices")
}))
```

### Working Directory

```rust
.cwd("/path/to/your/project")
```

### Model Selection

```rust
.model("claude-opus-4-20250514")
```

## Advanced Features

### Permission Callbacks

Programmatically control tool usage:

```rust
use claude_agent_sdk::{CanUseTool, PermissionResult};

async fn my_permission_callback(
    tool_name: String,
    input: Value,
    context: ToolPermissionContext
) -> Result<PermissionResult> {
    if tool_name == "Bash" {
        let command = input["command"].as_str().unwrap();
        if command.contains("rm -rf") {
            return Ok(PermissionResult::Deny(PermissionResultDeny {
                message: "Dangerous command blocked".into(),
                interrupt: false
            }));
        }
    }
    Ok(PermissionResult::Allow(PermissionResultAllow::default()))
}

// Use with client (not query function)
let mut client = ClaudeSDKClient::new(options);
// Set callback during initialization
```

### Hook System

Execute custom code at specific points:

```rust
// PreToolUse hook example
async fn validate_bash(
    input: HookInput,
    tool_use_id: Option<String>,
    context: HookContext
) -> Result<HookOutput> {
    if let HookInput::PreToolUse(pre) = input {
        if pre.tool_name == "Bash" {
            let command = pre.tool_input["command"].as_str().unwrap();
            if command.contains("dangerous") {
                return Ok(HookOutput::Sync(SyncHookOutput {
                    continue_: false,
                    stop_reason: Some("Blocked dangerous command".into()),
                    ..Default::default()
                }));
            }
        }
    }
    Ok(HookOutput::default())
}
```

### MCP Server Configuration

Use external MCP servers:

```rust
use std::collections::HashMap;

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
- **Search**: Grep, WebSearch
- **Communication**: Task (subagents)
- **And more**: See [Claude Code documentation](https://docs.anthropic.com/en/docs/claude-code/settings#tools-available-to-claude)

## Error Handling

```rust
use claude_agent_sdk::{
    ClaudeSDKError,
    CLINotFoundError,
    CLIConnectionError,
    ProcessError
};

match query("test", None).await {
    Ok(messages) => { /* process */ }
    Err(ClaudeSDKError::CLINotFound) => {
        eprintln!("Please install Claude Code: npm install -g @anthropic-ai/claude-code");
    }
    Err(ClaudeSDKError::Process { exit_code, message, stderr }) => {
        eprintln!("Process failed ({}): {}", exit_code, message);
        if let Some(err) = stderr {
            eprintln!("Stderr: {}", err);
        }
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## Examples

See the `examples/` directory for complete examples:

- `basic.rs` - Simple one-shot query
- `with_options.rs` - Configuration examples
- `interactive.rs` - Bidirectional conversation
- `hooks.rs` - Hook system usage
- `permissions.rs` - Permission callbacks
- `mcp_tools.rs` - MCP integration
- `quick_start.rs` - Multiple patterns

Run examples:
```bash
cargo run --example basic
cargo run --example interactive
```

## Documentation

### API Guide

See [docs/API_GUIDE.md](docs/API_GUIDE.md) for comprehensive documentation including:
- Complete API reference
- Usage patterns and best practices
- Hook and permission callback examples
- Troubleshooting guides

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
which claude  # Should show path
```

Or set custom path:
```rust
.cli_path("/path/to/claude")
```

### Authentication Failed

```
Error: Failed to authenticate with Anthropic API
```

**Solution**: Set API key environment variable:
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

Verify it's set:
```bash
echo $ANTHROPIC_API_KEY
```

### Version Mismatch

```
Warning: Claude Code version 1.x.x < minimum 2.0.0
```

**Solution**: Update Claude Code:
```bash
npm update -g @anthropic-ai/claude-code
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

# Integration tests (requires ANTHROPIC_API_KEY)
cargo test --test '*' -- --ignored
```

Format and lint:
```bash
cargo fmt
cargo clippy
```

## Migration from Python SDK

If you're coming from the Python SDK, see [MIGRATION_FROM_PYTHON.md](MIGRATION_FROM_PYTHON.md) for:
- Type system differences
- Async pattern changes
- Builder pattern usage
- Error handling differences

## Resources

- [API Guide](docs/API_GUIDE.md) - Comprehensive API documentation and examples
- [Implementation Plan](IMPLEMENTATION_PLAN.md) - Detailed architecture and design
- [Claude Code Docs](https://docs.claude.com/en/docs/claude-code/sdk/sdk-python)
- [Anthropic Console](https://console.anthropic.com)

## License

MIT

## Contributing

Contributions welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) first.

## Support

- Issues: https://github.com/yourusername/claude-agent-sdk-rust/issues
- Documentation: https://docs.rs/claude-agent-sdk
- Claude Code Help: https://docs.claude.com/en/docs/claude-code

---

**Note**: This SDK wraps the Claude Code CLI. Make sure it's installed and your API key is configured before use.
