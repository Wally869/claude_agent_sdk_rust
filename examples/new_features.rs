//! Example demonstrating new features ported from the Python SDK.
//!
//! Tests: thinking config, effort, max_budget_usd, structured output,
//! tools option, and the interactive client's get_mcp_status().

use claude_agent_sdk::{
    ClaudeAgentOptions, ClaudeSDKClient, Effort, Message, Result, ThinkingConfig,
    query,
};
use futures::StreamExt;

/// Helper: print assistant text from a message stream.
async fn print_response(
    messages: &mut std::pin::Pin<Box<impl futures::Stream<Item = Result<Message>>>>,
) {
    while let Some(result) = messages.next().await {
        match result {
            Ok(Message::Assistant(assistant)) => {
                for block in &assistant.message.content {
                    if let Some(text) = block.as_text() {
                        print!("{}", text.text);
                    }
                }
                println!();
            }
            Ok(Message::Result(result)) => {
                println!("  Duration: {}ms | Turns: {}", result.duration_ms, result.num_turns);
                if let Some(cost) = result.total_cost_usd {
                    println!("  Cost: ${:.4}", cost);
                }
                if let Some(output) = &result.structured_output {
                    println!("  Structured output: {}", output);
                }
                break;
            }
            Ok(Message::System(sys)) => {
                println!("  [System: {}]", sys.subtype);
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("  Error: {}", e);
                break;
            }
        }
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== Claude Agent SDK - New Features Example ===\n");

    // ─── 1. Thinking config + effort ───
    println!("--- 1. Thinking (adaptive) + effort (high) ---");
    {
        let options = ClaudeAgentOptions::builder()
            .thinking(Some(ThinkingConfig::Adaptive))
            .effort(Some(Effort::High))
            .max_turns(1u32)
            .build();

        let stream = query("What is 37 * 43? Think step by step.", Some(options)).await?;
        let mut stream = Box::pin(stream);
        print_response(&mut stream).await;
    }
    println!();

    // ─── 2. Max budget constraint ───
    println!("--- 2. Max budget ($0.01) ---");
    {
        let options = ClaudeAgentOptions::builder()
            .max_budget_usd(Some(0.01))
            .max_turns(1u32)
            .build();

        let stream = query("Say hello in 3 languages.", Some(options)).await?;
        let mut stream = Box::pin(stream);
        print_response(&mut stream).await;
    }
    println!();

    // ─── 3. Structured output (json_schema) ───
    println!("--- 3. Structured output (JSON schema) ---");
    {
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
            .max_turns(3u32)
            .build();

        let stream = query(
            "What is the capital and population of Japan?",
            Some(options),
        ).await?;
        let mut stream = Box::pin(stream);
        print_response(&mut stream).await;
    }
    println!();

    // ─── 4. Interactive client: get_mcp_status ───
    println!("--- 4. Interactive client: MCP status ---");
    {
        let options = ClaudeAgentOptions::builder()
            .max_turns(1u32)
            .build();
        let mut client = ClaudeSDKClient::new(options);
        client.connect(None).await?;

        match client.get_mcp_status().await {
            Ok(status) => {
                if let Some(servers) = status.get("mcpServers").and_then(|v| v.as_array()) {
                    println!("  MCP servers: {}", servers.len());
                    for server in servers {
                        let name = server.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                        let st = server.get("status").and_then(|v| v.as_str()).unwrap_or("?");
                        println!("    - {}: {}", name, st);
                    }
                } else {
                    println!("  No MCP servers configured.");
                }
            }
            Err(e) => println!("  MCP status error: {}", e),
        }

        client.disconnect().await?;
    }

    println!("\nDone!");
    Ok(())
}
