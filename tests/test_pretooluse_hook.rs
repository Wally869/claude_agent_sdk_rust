//! Test to verify if PreToolUse hooks can prevent tool execution.

use async_trait::async_trait;
use claude_agent_sdk::{
    ClaudeAgentOptions, ClaudeSDKClient, Message,
    callbacks::{HookCallback, hooks},
    types::{HookContext, HookEvent, HookInput, HookOutput},
};
use futures::StreamExt;
use std::sync::{Arc, Mutex};

/// Hook that denies all Bash commands
struct BlockBashHook {
    bash_attempts: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl HookCallback for BlockBashHook {
    async fn call(
        &self,
        input: HookInput,
        _tool_use_id: Option<String>,
        _context: HookContext,
    ) -> claude_agent_sdk::Result<HookOutput> {
        match input {
            HookInput::PreToolUse(pre_tool) => {
                eprintln!("\n[HOOK CALLED] PreToolUse for: {}", pre_tool.tool_name);
                eprintln!("[HOOK] Input: {:?}", pre_tool.tool_input);

                if pre_tool.tool_name == "Bash" {
                    let command = pre_tool
                        .tool_input
                        .get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    let mut attempts = self.bash_attempts.lock().unwrap();
                    attempts.push(command.to_string());

                    eprintln!("[HOOK] BLOCKING Bash command: {}", command);
                    return Ok(hooks::block("Bash commands are blocked by hook"));
                }

                eprintln!("[HOOK] Allowing tool: {}", pre_tool.tool_name);
                Ok(hooks::allow())
            }
            _ => Ok(hooks::allow()),
        }
    }
}

#[tokio::test]
#[ignore] // Requires CLI and API key
async fn test_pretooluse_hook_blocks_execution() {
    eprintln!("\n========================================");
    eprintln!("Testing PreToolUse Hook Blocking");
    eprintln!("========================================\n");

    let bash_attempts = Arc::new(Mutex::new(Vec::new()));
    let hook = BlockBashHook {
        bash_attempts: bash_attempts.clone(),
    };

    let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());

    // Register PreToolUse hook
    let _hook_id = client.register_hook(HookEvent::PreToolUse, None, hook);

    client.connect(None).await.expect("Connection failed");

    // Try to run a bash command (should be blocked)
    client
        .query("Run 'echo HOOK_TEST' using bash")
        .await
        .expect("Query failed");

    let messages = client.receive_response().expect("Stream failed");
    let mut messages = Box::pin(messages);

    let mut got_bash_tool_use = false;
    let mut got_bash_result = false;
    let mut bash_result_content = String::new();

    while let Some(result) = messages.next().await {
        match result.expect("Parse failed") {
            Message::Assistant(msg) => {
                eprintln!("[MESSAGE] Assistant message received");
                for block in &msg.message.content {
                    if let claude_agent_sdk::ContentBlock::ToolUse(tool_use) = block {
                        eprintln!("[MESSAGE] Tool use: {}", tool_use.name);
                        if tool_use.name == "Bash" {
                            got_bash_tool_use = true;
                        }
                    }
                }
            }
            Message::User(msg) => {
                eprintln!("[MESSAGE] User message (tool result) received");
                if let claude_agent_sdk::MessageContent::Blocks(blocks) = &msg.message.content {
                    for block in blocks {
                        if let claude_agent_sdk::ContentBlock::ToolResult(result) = block {
                            let is_err = result.is_error.unwrap_or(false);
                            if !is_err {
                                got_bash_result = true;
                                if let Some(content) = &result.content {
                                    bash_result_content = content.to_string();
                                    eprintln!("[MESSAGE] Tool result: {}", bash_result_content);
                                }
                            } else {
                                if let Some(content) = &result.content {
                                    eprintln!("[MESSAGE] Tool error: {}", content);
                                }
                            }
                        }
                    }
                }
            }
            Message::Result(_) => {
                break;
            }
            _ => {}
        }
    }

    drop(messages);
    client.disconnect().await.expect("Disconnect failed");

    // Check results
    let attempts = bash_attempts.lock().unwrap();
    eprintln!("\n========================================");
    eprintln!("Test Results:");
    eprintln!("========================================");
    eprintln!("Hook was called {} times", attempts.len());
    eprintln!("Bash tool_use in messages: {}", got_bash_tool_use);
    eprintln!("Bash tool result received: {}", got_bash_result);
    eprintln!("Result content: {}", bash_result_content);
    eprintln!("========================================\n");

    if !attempts.is_empty() {
        eprintln!("✓ Hook WAS called for Bash");
    } else {
        eprintln!("✗ Hook was NOT called");
    }

    if got_bash_result && bash_result_content.contains("HOOK_TEST") {
        eprintln!("✗ Bash command EXECUTED despite hook denial!");
    } else if !got_bash_result {
        eprintln!("✓ Bash command appears to have been BLOCKED!");
    }
}
