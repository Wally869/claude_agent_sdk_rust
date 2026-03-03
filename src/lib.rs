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
//!     let stream = query("What is 2 + 2?", None).await?;
//!     let mut messages = Box::pin(stream);
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
    AgentDefinition, AssistantMessage, ClaudeAgentOptions, ContentBlock, Effort, HookContext,
    HookEvent, HookInput, HookOutput, Message, MessageContent, PermissionMode, PermissionResult,
    ResultMessage, SandboxSettings, SdkPluginConfig, SettingSource, SystemPrompt,
    SystemPromptPreset, TextBlock, ThinkingConfig, ToolPermissionContext, ToolUseBlock,
    ToolsOption, UserMessage,
};

use futures::Stream;
use query::Query;
use transport::{check_claude_version, find_claude_cli, subprocess::SubprocessTransport};

/// Query Claude with a simple prompt.
///
/// This is the simplest way to interact with Claude. It creates a one-shot
/// query and returns a stream of messages.
///
/// Internally uses streaming mode (matching Python/TypeScript SDK behavior)
/// to send the prompt via stdin after initialization. This allows agents
/// and large configs to be sent via the initialize request.
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
///     let stream = query("Hello, Claude!", None).await?;
///     let mut messages = Box::pin(stream);
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

    // Always use streaming mode internally (matching Python/TypeScript SDK)
    let mut transport = SubprocessTransport::new_streaming(cli_path, &options);

    // Spawn process (empty prompt for streaming mode)
    transport.spawn(&options, "").await?;

    // Create Query, start it, initialize
    let query_obj = Query::new(transport);
    let query_handle = query_obj.start();

    // Initialize streaming mode
    query_handle.initialize(None).await?;

    // Send user message via stdin after initialize, then close stdin
    // to signal no more input (matching Python SDK's end_input() call)
    query_handle.send_user_message(&prompt_str).await?;
    query_handle.close_stdin().await;

    // Return stream of parsed messages
    let message_stream = async_stream::stream! {
        use futures::stream::StreamExt as _;

        let mut stream = Box::pin(query_handle.read_messages());

        while let Some(result) = stream.next().await {
            match result {
                Ok(value) => {
                    match parser::parse_message(value) {
                        Ok(message) => yield Ok(message),
                        Err(ClaudeSDKError::UnknownMessageType(_)) => {
                            // Skip unknown message types (forward-compatible)
                            continue;
                        }
                        Err(e) => yield Err(e),
                    }
                }
                Err(e) => yield Err(e),
            }
        }
    };

    Ok(message_stream)
}
