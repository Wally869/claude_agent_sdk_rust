//! Claude Agent SDK for Rust
//!
//! Build production-ready AI agents with Claude Code.
//!
//! # Quick Start
//!
//! ```no_run
//! use claude_agent_sdk::{query, ClaudeAgentOptions, Message};
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut messages = query("What is 2 + 2?", None).await?;
//!
//!     while let Some(msg) = messages.next().await {
//!         match msg? {
//!             Message::Assistant(assistant) => {
//!                 println!("Claude: {:?}", assistant);
//!             }
//!             Message::Result(result) => {
//!                 println!("Cost: ${:.4}", result.total_cost_usd.unwrap_or(0.0));
//!             }
//!             _ => {}
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod callbacks;
pub mod client;
pub mod error;
pub mod parser;
pub mod query;
pub mod transport;
pub mod types;

// Re-export commonly used types
pub use callbacks::{HookCallback, PermissionCallback};
pub use client::ClaudeSDKClient;
pub use error::{ClaudeSDKError, Result};
pub use types::{
    AgentDefinition, AssistantMessage, ClaudeAgentOptions, ContentBlock, HookContext, HookEvent,
    HookInput, HookOutput, Message, MessageContent, PermissionMode, PermissionResult,
    ResultMessage, SettingSource, SystemPrompt, SystemPromptPreset, TextBlock, ToolPermissionContext,
    ToolUseBlock, UserMessage,
};

use futures::Stream;
use transport::{find_claude_cli, check_claude_version, subprocess::SubprocessTransport};

/// Query Claude with a simple prompt.
///
/// This is the simplest way to interact with Claude. It creates a one-shot
/// query and returns a stream of messages.
///
/// # Arguments
///
/// * `prompt` - The prompt to send to Claude
/// * `options` - Optional configuration (uses defaults if None)
///
/// # Returns
///
/// A stream of `Message` objects representing the conversation.
///
/// # Example
///
/// ```no_run
/// use claude_agent_sdk::{query, Message};
/// use futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut messages = query("Hello, Claude!", None).await?;
///
///     while let Some(msg) = messages.next().await {
///         println!("{:?}", msg?);
///     }
///
///     Ok(())
/// }
/// ```
pub async fn query(
    prompt: impl Into<String>,
    options: Option<ClaudeAgentOptions>,
) -> Result<impl Stream<Item = Result<Message>>> {
    let prompt_str = prompt.into();
    let options = options.unwrap_or_default();

    // Find CLI
    let cli_path = find_claude_cli(options.cli_path.as_ref())?;

    // Check version (optional, doesn't fail)
    if std::env::var("CLAUDE_AGENT_SDK_SKIP_VERSION_CHECK").is_err() {
        let _ = check_claude_version(&cli_path).await;
    }

    // Create transport
    let mut transport = SubprocessTransport::new(cli_path, &options);

    // Spawn process
    transport.spawn(&options, &prompt_str).await?;

    // Return stream of parsed messages
    let message_stream = async_stream::stream! {
        use futures::stream::StreamExt as _;

        let mut stream = Box::pin(transport.read_messages());

        while let Some(result) = stream.next().await {
            match result {
                Ok(value) => {
                    match parser::parse_message(value) {
                        Ok(message) => yield Ok(message),
                        Err(e) => yield Err(e),
                    }
                }
                Err(e) => yield Err(e),
            }
        }
    };

    Ok(message_stream)
}
