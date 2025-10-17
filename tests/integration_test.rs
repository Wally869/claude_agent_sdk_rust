//! Integration tests for Claude Agent SDK.
//!
//! These tests require:
//! - Claude Code CLI installed
//! - ANTHROPIC_API_KEY environment variable set
//!
//! Run with: cargo test --test integration_test -- --ignored

use claude_agent_sdk::{
    callbacks::{hooks, permissions, HookCallback, PermissionCallback},
    query, ClaudeAgentOptions, ClaudeSDKClient, Message, PermissionMode,
    types::{HookContext, HookEvent, HookInput, HookOutput, PermissionResult, ToolPermissionContext},
};
use async_trait::async_trait;
use futures::StreamExt;
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// Helper to check if CLI is available
fn check_cli_available() -> bool {
    //std::env::var("ANTHROPIC_API_KEY").is_ok()
    return true;
}

#[tokio::test]
#[ignore] // Requires CLI and API key
async fn test_simple_query() {
    if !check_cli_available() {
        eprintln!("Skipping: ANTHROPIC_API_KEY not set");
        return;
    }

    let messages = query("What is 2 + 2? Answer with just the number.", None)
        .await
        .expect("Query should succeed");

    let mut messages = Box::pin(messages);
    let mut got_assistant = false;
    let mut got_result = false;

    while let Some(result) = messages.next().await {
        match result.expect("Message should parse") {
            Message::Assistant(_) => got_assistant = true,
            Message::Result(r) => {
                got_result = true;
                assert!(r.duration_ms > 0, "Should have duration");
                assert_eq!(r.num_turns, 1, "Should have 1 turn");
            }
            _ => {}
        }
    }

    assert!(got_assistant, "Should receive assistant message");
    assert!(got_result, "Should receive result message");
}

#[tokio::test]
#[ignore]
async fn test_query_with_options() {
    if !check_cli_available() {
        return;
    }

    let options = ClaudeAgentOptions::builder()
        .max_turns(2)
        .permission_mode(PermissionMode::BypassPermissions)
        .build();

    let messages = query("Say hello", Some(options))
        .await
        .expect("Query should succeed");

    let mut messages = Box::pin(messages);
    let mut got_response = false;

    while let Some(result) = messages.next().await {
        if let Message::Assistant(_) = result.expect("Message should parse") {
            got_response = true;
        }
    }

    assert!(got_response, "Should get assistant response");
}

#[tokio::test]
#[ignore]
async fn test_client_connect_disconnect() {
    if !check_cli_available() {
        return;
    }

    let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());

    // Connect
    client
        .connect(None)
        .await
        .expect("Connection should succeed");

    assert!(client.is_connected(), "Client should be connected");

    // Disconnect
    client
        .disconnect()
        .await
        .expect("Disconnect should succeed");
}

#[tokio::test]
#[ignore]
async fn test_client_query_and_response() {
    if !check_cli_available() {
        return;
    }

    let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());
    client.connect(None).await.expect("Connection failed");

    // Send query
    client
        .query("What is 5 + 3? Answer with just the number.")
        .await
        .expect("Query should succeed");

    // Receive response
    let messages = client
        .receive_response()
        .expect("Should get message stream");
    let mut messages = Box::pin(messages);

    let mut got_assistant = false;
    let mut got_result = false;

    while let Some(result) = messages.next().await {
        match result.expect("Message should parse") {
            Message::Assistant(_) => got_assistant = true,
            Message::Result(_) => {
                got_result = true;
                break;
            }
            _ => {}
        }
    }

    assert!(got_assistant, "Should receive assistant message");
    assert!(got_result, "Should receive result message");

    drop(messages);
    client.disconnect().await.expect("Disconnect failed");
}

#[tokio::test]
#[ignore]
async fn test_multiple_queries() {
    if !check_cli_available() {
        return;
    }

    let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());
    client.connect(None).await.expect("Connection failed");

    // First query
    client.query("What is 1 + 1?").await.expect("Query 1 failed");
    let messages = client.receive_response().expect("Stream 1 failed");
    let mut messages = Box::pin(messages);

    while let Some(result) = messages.next().await {
        if let Message::Result(_) = result.expect("Parse failed") {
            break;
        }
    }
    drop(messages);

    // Second query
    client.query("What is 2 + 2?").await.expect("Query 2 failed");
    let messages = client.receive_response().expect("Stream 2 failed");
    let mut messages = Box::pin(messages);

    while let Some(result) = messages.next().await {
        if let Message::Result(_) = result.expect("Parse failed") {
            break;
        }
    }
    drop(messages);

    client.disconnect().await.expect("Disconnect failed");
}

// Mock hook for testing
struct TestHook {
    call_count: Arc<Mutex<usize>>,
}

#[async_trait]
impl HookCallback for TestHook {
    async fn call(
        &self,
        _input: HookInput,
        _tool_use_id: Option<String>,
        _context: HookContext,
    ) -> claude_agent_sdk::Result<HookOutput> {
        let mut count = self.call_count.lock().unwrap();
        *count += 1;
        Ok(hooks::allow())
    }
}

#[tokio::test]
#[ignore]
async fn test_hook_callback() {
    if !check_cli_available() {
        return;
    }

    let call_count = Arc::new(Mutex::new(0));
    let hook = TestHook {
        call_count: call_count.clone(),
    };

    let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());

    // Register hook
    let _hook_id = client.register_hook(HookEvent::PreToolUse, None, hook);

    client.connect(None).await.expect("Connection failed");

    // Query that might use tools
    client
        .query("What is the current working directory? Use appropriate tools.")
        .await
        .expect("Query failed");

    let messages = client.receive_response().expect("Stream failed");
    let mut messages = Box::pin(messages);

    while let Some(result) = messages.next().await {
        if let Message::Result(_) = result.expect("Parse failed") {
            break;
        }
    }

    drop(messages);
    client.disconnect().await.expect("Disconnect failed");

    // Note: Hook may or may not be called depending on Claude's behavior
    // We're just testing that registering hooks doesn't break anything
}

// Mock permission callback for testing
struct BlockingPermissionCallback;

#[async_trait]
impl PermissionCallback for BlockingPermissionCallback {
    async fn call(
        &self,
        tool_name: String,
        _input: Value,
        _context: ToolPermissionContext,
    ) -> claude_agent_sdk::Result<PermissionResult> {
        // Block all Bash commands
        if tool_name == "Bash" {
            return Ok(permissions::deny("Bash is blocked in tests"));
        }

        Ok(permissions::allow())
    }
}

#[tokio::test]
#[ignore]
async fn test_permission_callback() {
    if !check_cli_available() {
        return;
    }

    let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());

    // Register permission callback
    client.set_permission_callback(BlockingPermissionCallback);

    client.connect(None).await.expect("Connection failed");

    // Try to use bash (should be blocked by callback)
    client
        .query("Run 'echo hello' using bash")
        .await
        .expect("Query failed");

    let messages = client.receive_response().expect("Stream failed");
    let mut messages = Box::pin(messages);

    while let Some(result) = messages.next().await {
        if let Message::Result(_) = result.expect("Parse failed") {
            break;
        }
    }

    drop(messages);
    client.disconnect().await.expect("Disconnect failed");

    // Note: We can't easily verify the callback was invoked without more complex
    // infrastructure, but we can verify it doesn't break the system
}

#[tokio::test]
#[ignore]
async fn test_permission_mode_bypass() {
    if !check_cli_available() {
        return;
    }

    let options = ClaudeAgentOptions::builder()
        .permission_mode(PermissionMode::BypassPermissions)
        .build();

    let mut client = ClaudeSDKClient::new(options);
    client.connect(None).await.expect("Connection failed");

    client
        .query("What is 2 + 2?")
        .await
        .expect("Query failed");

    let messages = client.receive_response().expect("Stream failed");
    let mut messages = Box::pin(messages);

    let mut got_result = false;
    while let Some(result) = messages.next().await {
        if let Message::Result(_) = result.expect("Parse failed") {
            got_result = true;
            break;
        }
    }

    assert!(got_result, "Should receive result");

    drop(messages);
    client.disconnect().await.expect("Disconnect failed");
}

#[tokio::test]
#[ignore]
async fn test_model_selection() {
    if !check_cli_available() {
        return;
    }

    let options = ClaudeAgentOptions::builder()
        .model(Some("claude-sonnet-4-5-20250929".to_string()))
        .build();

    let messages = query("Hello", Some(options))
        .await
        .expect("Query should succeed");

    let mut messages = Box::pin(messages);
    let mut got_response = false;

    while let Some(result) = messages.next().await {
        if let Message::Assistant(_) = result.expect("Parse failed") {
            got_response = true;
        }
    }

    assert!(got_response, "Should get response");
}

#[tokio::test]
#[ignore]
async fn test_max_turns_limit() {
    if !check_cli_available() {
        return;
    }

    let options = ClaudeAgentOptions::builder()
        .max_turns(1)
        .build();

    let messages = query("What is Rust?", Some(options))
        .await
        .expect("Query should succeed");

    let mut messages = Box::pin(messages);
    let mut num_turns = 0;

    while let Some(result) = messages.next().await {
        if let Message::Result(r) = result.expect("Parse failed") {
            num_turns = r.num_turns;
        }
    }

    assert!(num_turns <= 1, "Should respect max_turns limit");
}

#[tokio::test]
#[ignore]
async fn test_tool_filtering() {
    if !check_cli_available() {
        return;
    }

    let options = ClaudeAgentOptions::builder()
        .allowed_tools(vec!["Read".to_string()])
        .build();

    let messages = query("Hello", Some(options))
        .await
        .expect("Query should succeed");

    let mut messages = Box::pin(messages);

    while let Some(result) = messages.next().await {
        if let Message::Assistant(_) = result.expect("Parse failed") {
            // Successfully processed with limited tools
            break;
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_system_prompt() {
    if !check_cli_available() {
        return;
    }

    use claude_agent_sdk::types::SystemPrompt;

    let options = ClaudeAgentOptions::builder()
        .system_prompt(SystemPrompt::Text(
            "You are a helpful assistant that always responds in exactly 3 words.".to_string()
        ))
        .build();

    let messages = query("What is Rust?", Some(options))
        .await
        .expect("Query should succeed");

    let mut messages = Box::pin(messages);
    let mut got_response = false;

    while let Some(result) = messages.next().await {
        if let Message::Assistant(_) = result.expect("Parse failed") {
            got_response = true;
        }
    }

    assert!(got_response, "Should get response with custom system prompt");
}

#[tokio::test]
async fn test_error_cli_not_found() {
    use std::path::PathBuf;

    // Set an invalid CLI path
    let options = ClaudeAgentOptions::builder()
        .cli_path(Some(PathBuf::from("/nonexistent/path/to/claude")))
        .build();

    let result = query("test", Some(options)).await;

    assert!(result.is_err(), "Should error with invalid CLI path");

    if let Err(e) = result {
        let error_str = e.to_string();
        assert!(
            error_str.contains("not found") || error_str.contains("No such file"),
            "Error should indicate CLI not found: {}",
            error_str
        );
    }
}

#[tokio::test]
#[ignore]
async fn test_interrupt() {
    if !check_cli_available() {
        return;
    }

    let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());
    client.connect(None).await.expect("Connection failed");

    // Start a query
    client
        .query("Count to 100")
        .await
        .expect("Query failed");

    // Immediately interrupt
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let interrupt_result = client.interrupt().await;

    // Interrupt may or may not succeed depending on timing
    // We're just verifying it doesn't crash
    let _ = interrupt_result;

    client.disconnect().await.expect("Disconnect failed");
}
