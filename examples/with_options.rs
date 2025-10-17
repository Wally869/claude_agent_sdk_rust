//! Example demonstrating various ClaudeAgentOptions.

use claude_agent_sdk::{
    ClaudeAgentOptions, Message, PermissionMode, SettingSource, SystemPrompt, query
};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Claude Agent SDK - Options Example ===\n");

    // Build options with custom configuration
    let options = ClaudeAgentOptions::builder()
        // Allow only specific tools
        .allowed_tools(vec![
            "Read".to_string(),
            "Write".to_string(),
        ])
        // Set permission mode to auto-accept file edits
        .permission_mode(PermissionMode::AcceptEdits)
        // Custom system prompt
        .system_prompt(SystemPrompt::Text(
            "You are a helpful Rust programming assistant. \
             Be concise and focus on best practices.".to_string()
        ))
        // Limit conversation turns
        .max_turns(5)
        // Use specific model
        .model(Some("claude-sonnet-4-5-20250929".to_string()))
        // Load specific settings
        .setting_sources(Some(vec![
            SettingSource::User,
            SettingSource::Project,
        ]))
        .build();

    println!("Configuration:");
    println!("  - Tools: Read, Write only");
    println!("  - Permission mode: AcceptEdits");
    println!("  - Max turns: 5");
    println!("  - Model: claude-sonnet-4-5");
    println!();

    // Use the query API with options
    let messages = query(
        "Explain the difference between String and &str in Rust in 2-3 sentences.",
        Some(options)
    ).await?;

    let mut messages = Box::pin(messages);

    println!("--- Response ---\n");

    while let Some(result) = messages.next().await {
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
                println!("\n--- Result ---");
                println!("Duration: {}ms", result.duration_ms);
                println!("Turns: {}", result.num_turns);
                if let Some(cost) = result.total_cost_usd {
                    println!("Cost: ${:.4}", cost);
                }
                if let Some(usage) = &result.usage {
                    if let (Some(input), Some(output)) = (
                        usage.get("input_tokens").and_then(|v| v.as_u64()),
                        usage.get("output_tokens").and_then(|v| v.as_u64())
                    ) {
                        println!("Tokens: {} in, {} out", input, output);
                    }
                }
            }
            Message::System(system) => {
                println!("[System: {}]", system.subtype);
            }
            _ => {}
        }
    }

    Ok(())
}
