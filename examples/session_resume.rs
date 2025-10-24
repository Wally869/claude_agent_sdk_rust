use claude_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient, ContentBlock, Message};
use futures::StreamExt;

/// Example demonstrating session management and resuming conversations.
///
/// This example shows:
/// - Starting a conversation and capturing the session ID
/// - Disconnecting from the session
/// - Resuming a previous conversation by session ID
/// - Using --continue to resume the most recent conversation
///
/// Run with:
/// ```bash
/// cargo run --example session_resume
/// ```
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Session Management Example ===\n");

    // Part 1: Start a new conversation and capture session ID
    println!("📝 Starting new conversation...");
    let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());

    client.connect(None).await?;
    client.query("Remember this: my favorite color is blue.").await?;

    // Process response and capture session ID
    let messages = client.receive_response()?;
    let mut messages = Box::pin(messages);
    let mut session_id = None;

    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant(assistant) => {
                // Extract text from content blocks
                let text = assistant.message.content.iter()
                    .filter_map(|b| {
                        if let ContentBlock::Text(t) = b {
                            Some(t.text.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("🤖 Claude: {}", text);

                // Session ID should be captured automatically
                session_id = client.get_session_id();
                if let Some(id) = &session_id {
                    println!("\n✅ Session ID captured: {}", id);
                }
            }
            Message::Result(result) => {
                println!("\n📊 Conversation complete:");
                println!("   Turns: {}", result.num_turns);
                println!("   Duration: {}ms", result.duration_ms);

                // Get final session ID
                session_id = Some(result.session_id.clone());
                break;
            }
            _ => {}
        }
    }

    let captured_session_id = session_id.expect("No session ID captured");
    println!("\n💾 Saved session ID: {}\n", captured_session_id);

    // Drop messages stream before disconnecting
    drop(messages);

    // Disconnect
    client.disconnect().await?;
    println!("🔌 Disconnected from session\n");

    // Part 2: Resume the conversation by session ID
    println!("=== Resuming Previous Session ===\n");
    println!("🔄 Resuming session: {}", captured_session_id);

    let options = ClaudeAgentOptions::builder()
        .resume(captured_session_id.clone())
        .build();

    let mut client = ClaudeSDKClient::new(options);
    client.connect(None).await?;
    client.query("What was my favorite color?").await?;

    // Process resumed conversation
    let messages = client.receive_response()?;
    let mut messages = Box::pin(messages);

    println!("\n📬 Resumed conversation:");
    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant(assistant) => {
                let text = assistant.message.content.iter()
                    .filter_map(|b| {
                        if let ContentBlock::Text(t) = b {
                            Some(t.text.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("🤖 Claude: {}", text);
                println!("\n✅ Claude remembered! The conversation context was preserved.");
            }
            Message::Result(result) => {
                println!("\n📊 Resume complete:");
                println!("   Session ID: {}", result.session_id);
                println!("   Total turns: {}", result.num_turns);
                break;
            }
            _ => {}
        }
    }

    drop(messages);
    client.disconnect().await?;

    // Part 3: Demonstrate continue (most recent)
    println!("\n=== Using --continue for Most Recent ===\n");

    let options = ClaudeAgentOptions::builder()
        .continue_conversation(true)
        .build();

    let mut client = ClaudeSDKClient::new(options);
    client.connect(None).await?;
    client.query("And what about my favorite color again?").await?;

    let messages = client.receive_response()?;
    let mut messages = Box::pin(messages);

    println!("📬 Continuing most recent conversation:");
    while let Some(msg) = messages.next().await {
        match msg? {
            Message::Assistant(assistant) => {
                let text = assistant.message.content.iter()
                    .filter_map(|b| {
                        if let ContentBlock::Text(t) = b {
                            Some(t.text.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("🤖 Claude: {}", text);
            }
            Message::Result(_) => {
                println!("\n✅ Continue completed successfully!");
                break;
            }
            _ => {}
        }
    }

    drop(messages);
    client.disconnect().await?;

    println!("\n=== Session Management Demo Complete ===");
    println!("\nKey takeaways:");
    println!("  • Session IDs are automatically captured from messages");
    println!("  • Use client.get_session_id() to retrieve the current session");
    println!("  • Resume specific sessions with .resume(session_id)");
    println!("  • Continue most recent with .continue_conversation(true)");
    println!("  • Sessions preserve full conversation context");

    Ok(())
}
