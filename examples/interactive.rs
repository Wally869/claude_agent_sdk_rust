//! Interactive example demonstrating ClaudeSDKClient for multi-turn conversations.

use claude_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient, Message, Result};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Claude Agent SDK - Interactive Client Example ===\n");

    // Create client with default options
    let options = ClaudeAgentOptions::default();
    let mut client = ClaudeSDKClient::new(options);

    println!("Connecting to Claude...");
    client.connect(None).await?;
    println!("Connected!\n");

    // Get server info
    if let Some(info) = client.get_server_info().await? {
        if let Some(style) = info.get("output_style").and_then(|v| v.as_str()) {
            println!("Output style: {}", style);
        }
        println!("Server initialized.\n");
    }

    // First query
    {
        println!("Sending first query: 'What is 2 + 2?'");
        client
            .query("What is 2 + 2? Please explain briefly.")
            .await?;

        println!("\n--- Response 1 ---");
        let response1 = client.receive_response()?;
        let mut response1 = Box::pin(response1);
        while let Some(result) = response1.next().await {
            match result? {
                Message::Assistant(assistant) => {
                    println!("Claude:");
                    for block in &assistant.message.content {
                        if let Some(text) = block.as_text() {
                            println!("{}", text.text);
                        }
                    }
                    println!();
                }
                Message::Result(result) => {
                    println!("--- Result ---");
                    println!("Duration: {}ms", result.duration_ms);
                    println!("Turns: {}", result.num_turns);
                    if let Some(cost) = result.total_cost_usd {
                        println!("Cost: ${:.4}", cost);
                    }
                    println!("Session: {}", result.session_id);
                }
                Message::System(system) => {
                    println!("[System: {}]", system.subtype);
                }
                _ => {}
            }
        }
    } // response1 dropped here

    // Second query in the same session
    {
        println!("\n\nSending second query: 'Now multiply that by 3'");
        client.query("Now multiply that by 3.").await?;

        println!("\n--- Response 2 ---");
        let response2 = client.receive_response()?;
        let mut response2 = Box::pin(response2);
        while let Some(result) = response2.next().await {
            match result? {
                Message::Assistant(assistant) => {
                    println!("Claude:");
                    for block in &assistant.message.content {
                        if let Some(text) = block.as_text() {
                            println!("{}", text.text);
                        }
                    }
                    println!();
                }
                Message::Result(result) => {
                    println!("--- Result ---");
                    println!("Duration: {}ms", result.duration_ms);
                    println!("Turns: {}", result.num_turns);
                    if let Some(cost) = result.total_cost_usd {
                        println!("Cost: ${:.4}", cost);
                    }
                    println!("Session: {}", result.session_id);
                }
                Message::System(system) => {
                    println!("[System: {}]", system.subtype);
                }
                _ => {}
            }
        }
    } // response2 dropped here

    // Disconnect
    println!("\n\nDisconnecting...");
    client.disconnect().await?;
    println!("Disconnected!");

    Ok(())
}
