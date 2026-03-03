use claude_agent_sdk_rust::{ClaudeAgentOptions, ClaudeSDKClient};

/// Example demonstrating usage tracking for Claude Code.
///
/// This example shows how to:
/// - Check current usage across different time windows
/// - Monitor usage limits
/// - Handle usage thresholds
///
/// Requirements:
/// - Claude Code Max Plan subscription
/// - Valid OAuth token in ~/.claude/.credentials.json
///
/// Run with:
/// ```bash
/// cargo run --example usage_tracking
/// ```
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client (connection not required for usage checks)
    let client = ClaudeSDKClient::new(ClaudeAgentOptions::default());

    println!("📊 Fetching Claude Code usage data...\n");

    // Get current usage
    match client.get_usage().await {
        Ok(usage) => {
            println!("=== Current Usage ===\n");

            // Display 5-hour window
            println!("🕐 5-Hour Rolling Window:");
            println!("   Usage: {:.1}%", usage.five_hour.utilization);
            if let Some(resets_at) = &usage.five_hour.resets_at {
                println!("   Resets at: {}", resets_at);
            }
            println!();

            // Display 7-day overall
            println!("📅 7-Day (Weekly) - All Models:");
            println!("   Usage: {:.1}%", usage.seven_day.utilization);
            if let Some(resets_at) = &usage.seven_day.resets_at {
                println!("   Resets at: {}", resets_at);
            }
            println!();

            // Display 7-day Opus
            println!("💎 7-Day (Weekly) - Opus Only:");
            println!("   Usage: {:.1}%", usage.seven_day_opus.utilization);
            if let Some(resets_at) = &usage.seven_day_opus.resets_at {
                println!("   Resets at: {}", resets_at);
            } else {
                println!("   Not applicable (Opus not used or not available)");
            }
            println!();

            // Display OAuth apps usage
            println!("🔑 7-Day OAuth Apps:");
            println!("   Usage: {:.1}%", usage.seven_day_oauth_apps.utilization);
            if usage.seven_day_oauth_apps.utilization > 0.0 {
                if let Some(resets_at) = &usage.seven_day_oauth_apps.resets_at {
                    println!("   Resets at: {}", resets_at);
                }
            } else {
                println!("   (Typically 0 for CLI usage)");
            }
            println!();

            // Display warnings based on usage levels
            println!("=== Usage Analysis ===\n");

            let max_util = usage.max_utilization();
            println!("📈 Highest utilization: {:.1}%", max_util);

            if usage.is_at_limit() {
                println!("⚠️  WARNING: At or very close to usage limit (>= 95%)!");
                println!("   Consider waiting for limits to reset before heavy usage.");
            } else if usage.is_approaching_limit() {
                println!("⚠️  Approaching usage limit (>= 80%).");
                println!("   Use conservatively to avoid hitting limits.");
            } else if usage.is_above_threshold(50.0) {
                println!("ℹ️  Moderate usage detected.");
                println!("   You have plenty of quota remaining.");
            } else {
                println!("✅ Low usage - plenty of quota available!");
            }

            // Display visual progress bars
            println!("\n=== Visual Progress ===\n");
            print_progress_bar("5-Hour", usage.five_hour.utilization);
            print_progress_bar("Weekly", usage.seven_day.utilization);
            print_progress_bar("Opus  ", usage.seven_day_opus.utilization);
        }
        Err(e) => {
            eprintln!("❌ Failed to fetch usage data: {}", e);
            eprintln!("\nPossible causes:");
            eprintln!("  - No OAuth credentials found (Max Plan required)");
            eprintln!("  - Access token expired (run `claude` to refresh)");
            eprintln!("  - Network error");
            eprintln!("  - API endpoint unavailable");
            return Err(e.into());
        }
    }

    Ok(())
}

/// Print a simple progress bar for usage visualization
fn print_progress_bar(label: &str, utilization: f64) {
    let width = 50;
    let filled = ((utilization / 100.0) * width as f64) as usize;
    let empty = width - filled;

    let color = if utilization >= 95.0 {
        "🔴" // Red for critical
    } else if utilization >= 80.0 {
        "🟡" // Yellow for warning
    } else {
        "🟢" // Green for safe
    };

    print!("{} {} [", color, label);
    for _ in 0..filled {
        print!("█");
    }
    for _ in 0..empty {
        print!("░");
    }
    println!("] {:.1}%", utilization);
}
