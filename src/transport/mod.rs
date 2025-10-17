//! Transport layer for CLI communication.

use crate::error::{ClaudeSDKError, Result};
use std::path::PathBuf;

pub mod subprocess;

/// Minimum required Claude Code CLI version.
pub const MINIMUM_CLAUDE_VERSION: &str = "2.0.0";

/// Find the Claude Code CLI binary.
pub fn find_claude_cli(custom_path: Option<&PathBuf>) -> Result<PathBuf> {
    // If custom path provided, use it
    if let Some(path) = custom_path {
        if path.exists() {
            return Ok(path.clone());
        }
        return Err(ClaudeSDKError::CLINotFound);
    }

    // Try which command
    if let Ok(path) = which::which("claude") {
        return Ok(path);
    }

    // Try common locations
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());

    let common_paths = vec![
        format!("{}/.npm-global/bin/claude", home),
        "/usr/local/bin/claude".to_string(),
        format!("{}/.local/bin/claude", home),
        format!("{}/node_modules/.bin/claude", home),
        format!("{}/.yarn/bin/claude", home),
    ];

    for path_str in common_paths {
        let path = PathBuf::from(path_str);
        if path.exists() {
            return Ok(path);
        }
    }

    Err(ClaudeSDKError::CLINotFound)
}

/// Check Claude Code CLI version.
pub async fn check_claude_version(cli_path: &PathBuf) -> Result<String> {
    use tokio::process::Command;

    let output = Command::new(cli_path)
        .arg("-v")
        .output()
        .await
        .map_err(|e| ClaudeSDKError::connection(format!("Failed to check CLI version: {}", e)))?;

    let version_str = String::from_utf8_lossy(&output.stdout);
    let version = version_str.trim();

    // Simple version check - just log warning if old
    if !version.is_empty() {
        // Parse version numbers for comparison
        let parse_version = |v: &str| -> Option<Vec<u32>> {
            v.split('.')
                .take(3)
                .map(|s| s.parse::<u32>().ok())
                .collect::<Option<Vec<u32>>>()
        };

        if let (Some(current), Some(minimum)) =
            (parse_version(version), parse_version(MINIMUM_CLAUDE_VERSION))
        {
            if current < minimum {
                eprintln!(
                    "Warning: Claude Code version {} is below minimum {}",
                    version, MINIMUM_CLAUDE_VERSION
                );
            }
        }
    }

    Ok(version.to_string())
}
