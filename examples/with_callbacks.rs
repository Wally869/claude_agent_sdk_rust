//! Example demonstrating hook and permission callbacks.
//!
//! This example shows how to:
//! - Implement custom HookCallback and PermissionCallback traits
//! - Register callbacks with ClaudeSDKClient
//! - Control tool usage with permission callbacks
//! - Monitor agent behavior with hook callbacks

use claude_agent_sdk::{
    callbacks::{hooks, permissions, HookCallback, PermissionCallback},
    ClaudeAgentOptions, ClaudeSDKClient, Message,
    types::{HookContext, HookEvent, HookInput, HookOutput, PermissionResult, ToolPermissionContext},
    Result,
};
use async_trait::async_trait;
use futures::StreamExt;
use serde_json::Value;

/// A logging hook that prints when tools are used.
struct ToolLogger;

#[async_trait]
impl HookCallback for ToolLogger {
    async fn call(
        &self,
        input: HookInput,
        tool_use_id: Option<String>,
        _context: HookContext,
    ) -> Result<HookOutput> {
        // Match on hook input variant
        match input {
            HookInput::PreToolUse(pre_tool) => {
                println!("\n[Hook:PreToolUse] Tool: {}", pre_tool.tool_name);
                if let Some(id) = tool_use_id {
                    println!("[Hook] Tool use ID: {}", id);
                }
                println!("[Hook] Input: {:?}", pre_tool.tool_input);
            }
            HookInput::PostToolUse(post_tool) => {
                println!("\n[Hook:PostToolUse] Tool: {}", post_tool.tool_name);
                println!("[Hook] Result received");
            }
            HookInput::UserPromptSubmit(prompt_submit) => {
                println!("\n[Hook:UserPromptSubmit] Prompt: {}", prompt_submit.prompt);
            }
            _ => {
                println!("\n[Hook] Other event");
            }
        }

        // Allow execution to continue
        Ok(hooks::allow())
    }
}

/// A permission callback that blocks dangerous bash commands.
struct SafetyChecker;

#[async_trait]
impl PermissionCallback for SafetyChecker {
    async fn call(
        &self,
        tool_name: String,
        input: Value,
        _context: ToolPermissionContext,
    ) -> Result<PermissionResult> {
        println!("\n[Permission] Checking tool: {}", tool_name);

        // Block dangerous bash commands
        if tool_name == "Bash" {
            if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
                println!("[Permission] Command: {}", cmd);

                // List of dangerous patterns
                let dangerous_patterns = vec![
                    "rm -rf /",
                    "rm -rf *",
                    ":(){ :|:& };:",  // fork bomb
                    "mkfs",
                    "dd if=/dev/zero",
                ];

                for pattern in dangerous_patterns {
                    if cmd.contains(pattern) {
                        println!("[Permission] BLOCKED - Dangerous command detected!");
                        return Ok(permissions::deny(
                            format!("Blocked dangerous command: {}", pattern)
                        ));
                    }
                }
            }
        }

        // Allow by default
        println!("[Permission] ALLOWED");
        Ok(permissions::allow())
    }
}

/// A hook that blocks writes to sensitive files.
struct FileProtector;

#[async_trait]
impl HookCallback for FileProtector {
    async fn call(
        &self,
        input: HookInput,
        _tool_use_id: Option<String>,
        _context: HookContext,
    ) -> Result<HookOutput> {
        // Check if this is a PreToolUse event for Write or Edit
        if let HookInput::PreToolUse(pre_tool) = &input {
            if pre_tool.tool_name == "Write" || pre_tool.tool_name == "Edit" {
                // Check if file path contains sensitive patterns
                if let Some(file_path) = pre_tool.tool_input.get("file_path").and_then(|v| v.as_str()) {
                    let sensitive_paths = vec![
                        "Cargo.toml",
                        "package.json",
                        ".env",
                        "credentials",
                    ];

                    for pattern in sensitive_paths {
                        if file_path.contains(pattern) {
                            println!("\n[FileProtector] Blocking write to: {}", file_path);
                            return Ok(hooks::block(
                                format!("Protected file: {}", file_path)
                            ));
                        }
                    }
                }
            }
        }

        Ok(hooks::allow())
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== Claude Agent SDK - Callbacks Example ===\n");

    // Create options
    let options = ClaudeAgentOptions::builder()
        .max_turns(3)
        .build();

    // Create client
    let mut client = ClaudeSDKClient::new(options);

    // Register hook callbacks
    println!("Registering callbacks...");

    let logger_id = client.register_hook(
        HookEvent::PreToolUse,
        None,  // Match all tools
        ToolLogger,
    );
    println!("  - Tool logger registered: {}", logger_id);

    let protector_id = client.register_hook(
        HookEvent::PreToolUse,
        Some("Write|Edit"),  // Only match Write and Edit tools
        FileProtector,
    );
    println!("  - File protector registered: {}", protector_id);

    // Register permission callback
    client.set_permission_callback(SafetyChecker);
    println!("  - Safety checker registered\n");

    // Connect to Claude
    println!("Connecting to Claude...");
    client.connect(None).await?;
    println!("Connected!\n");

    // Send a query that will trigger tools
    println!("--- Sending Query ---");
    let query = "List the files in the current directory using ls command.";
    println!("Query: {}\n", query);

    client.query(query).await?;

    // Receive response
    println!("--- Response ---\n");
    let messages = client.receive_response()?;
    let mut messages = Box::pin(messages);

    while let Some(result) = messages.next().await {
        match result? {
            Message::Assistant(assistant) => {
                println!("Claude:");
                for block in &assistant.message.content {
                    if let Some(text) = block.as_text() {
                        println!("{}", text.text);
                    } else if let Some(tool_use) = block.as_tool_use() {
                        println!("  [Using tool: {}]", tool_use.name);
                    } else if block.is_tool_result() {
                        println!("  [Tool completed]");
                    }
                }
                println!();
            }
            Message::Result(result) => {
                println!("\n--- Result ---");
                println!("Duration: {}ms", result.duration_ms);
                println!("Turns: {}", result.num_turns);
                println!("Subtype: {}", result.subtype);
                if let Some(cost) = result.total_cost_usd {
                    println!("Cost: ${:.4}", cost);
                }
            }
            Message::System(system) => {
                println!("[System: {}]", system.subtype);
            }
            _ => {}
        }
    }

    // Drop the first message stream before continuing
    drop(messages);

    // Demonstrate blocking behavior
    println!("\n\n=== Testing Safety Features ===\n");

    println!("Attempting dangerous command (should be blocked)...");
    client.query("Run 'rm -rf /' command").await?;

    let messages = client.receive_response()?;
    let mut messages = Box::pin(messages);
    while let Some(result) = messages.next().await {
        match result? {
            Message::Assistant(assistant) => {
                for block in &assistant.message.content {
                    if let Some(text) = block.as_text() {
                        println!("Claude: {}", text.text);
                    }
                }
            }
            Message::Result(_) => {
                println!("\nQuery completed (command was blocked by safety checker)\n");
            }
            _ => {}
        }
    }

    // Drop the stream before disconnecting
    drop(messages);

    // Disconnect
    println!("Disconnecting...");
    client.disconnect().await?;
    println!("Done!");

    Ok(())
}
