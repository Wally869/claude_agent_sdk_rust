//! Basic example of using the Claude Agent SDK.

use claude_agent_sdk::{Message, Result, query};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Claude Agent SDK - Basic Example ===\n");

    // Simple query
    let messages = query("What is 2 + 2? Please explain briefly.", None).await?;
    let mut messages = Box::pin(messages);

    println!("Querying Claude...\n");

    while let Some(result) = messages.next().await {
        match result? {
            Message::Assistant(assistant) => {
                println!("Claude responded:");
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

    Ok(())
}
