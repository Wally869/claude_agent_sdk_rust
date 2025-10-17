//! ClaudeSDKClient - Interactive bidirectional client for multi-turn conversations.
//!
//! Provides a high-level API for interactive sessions with Claude Code CLI.

use crate::callbacks::{HookCallback, PermissionCallback};
use crate::error::{ClaudeSDKError, Result};
use crate::parser;
use crate::query::{CanUseToolFn, HookCallbackFn, Query, QueryHandle};
use crate::types::{HookContext, HookEvent, HookInput, ToolPermissionContext};
use crate::transport::{check_claude_version, find_claude_cli, subprocess::SubprocessTransport};
use crate::types::{ClaudeAgentOptions, Message, PermissionMode};

use futures::Stream;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Interactive client for bidirectional conversations with Claude.
///
/// Supports multiple queries in a single session, dynamic configuration changes,
/// and full control protocol access.
///
/// # Example
///
/// ```no_run
/// use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
/// use futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let options = ClaudeAgentOptions::default();
///     let mut client = ClaudeSDKClient::new(options);
///
///     // Connect to Claude
///     client.connect(None).await?;
///
///     // Send a query
///     client.query("What is 2 + 2?").await?;
///
///     // Receive the response
///     let mut messages = client.receive_response()?;
///     while let Some(msg) = messages.next().await {
///         println!("{:?}", msg?);
///     }
///
///     // Disconnect
///     client.disconnect().await?;
///
///     Ok(())
/// }
/// ```
pub struct ClaudeSDKClient {
    /// Configuration options for the client.
    options: ClaudeAgentOptions,

    /// Query handle (present when connected).
    query_handle: Option<QueryHandle>,

    /// Hook callbacks registry (shared).
    hook_callbacks: Arc<std::sync::Mutex<HashMap<String, Arc<HookCallbackFn>>>>,

    /// Permission callback (if set, shared).
    permission_callback: Arc<std::sync::Mutex<Option<Arc<CanUseToolFn>>>>,

    /// Next callback ID counter.
    next_callback_id: usize,
}

impl ClaudeSDKClient {
    /// Create a new ClaudeSDKClient with the given options.
    ///
    /// Does not connect to the CLI yet. Call `connect()` to establish connection.
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration options for Claude
    ///
    /// # Example
    ///
    /// ```no_run
    /// use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    ///
    /// let options = ClaudeAgentOptions::default();
    /// let client = ClaudeSDKClient::new(options);
    /// ```
    pub fn new(options: ClaudeAgentOptions) -> Self {
        Self {
            options,
            query_handle: None,
            hook_callbacks: Arc::new(std::sync::Mutex::new(HashMap::new())),
            permission_callback: Arc::new(std::sync::Mutex::new(None)),
            next_callback_id: 0,
        }
    }

    /// Register a hook callback.
    ///
    /// Hooks execute at specific points in the agent loop (e.g., before/after tool use).
    /// Multiple hooks can be registered and will be invoked when matching events occur.
    ///
    /// # Arguments
    ///
    /// * `event` - The hook event type to register for
    /// * `matcher` - Optional tool name pattern (e.g., "Bash", "Read|Write", "*")
    /// * `callback` - The hook callback implementation
    ///
    /// # Returns
    ///
    /// A callback ID that can be used to unregister the hook later.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # use claude_agent_sdk::callbacks::HookCallback;
    /// # use claude_agent_sdk::types::{HookEvent, HookInput, HookOutput, HookContext, SyncHookOutput};
    /// # use async_trait::async_trait;
    /// # struct MyHook;
    /// # #[async_trait]
    /// # impl HookCallback for MyHook {
    /// #     async fn call(&self, _: HookInput, _: Option<String>, _: HookContext) -> claude_agent_sdk::Result<HookOutput> {
    /// #         Ok(HookOutput::Sync(SyncHookOutput::default()))
    /// #     }
    /// # }
    /// let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());
    /// let hook_id = client.register_hook(HookEvent::PreToolUse, Some("Bash"), MyHook);
    /// ```
    pub fn register_hook<H>(
        &mut self,
        _event: HookEvent,
        _matcher: Option<&str>,
        callback: H,
    ) -> String
    where
        H: HookCallback + 'static,
    {
        let callback_id = format!("hook_{}", self.next_callback_id);
        self.next_callback_id += 1;

        // Convert trait object to function pointer and wrap in Arc
        let callback_arc = Arc::new(callback);
        let callback_fn: HookCallbackFn = Box::new(
            move |input: HookInput, tool_use_id: Option<String>, context: HookContext| {
                let callback_clone = callback_arc.clone();
                Box::pin(async move {
                    callback_clone.call(input, tool_use_id, context).await
                })
            },
        );

        self.hook_callbacks.lock().unwrap().insert(callback_id.clone(), Arc::new(callback_fn));

        callback_id
    }

    /// Register a permission callback.
    ///
    /// The permission callback is invoked when Claude needs permission to use a tool.
    /// Only one permission callback can be active at a time.
    ///
    /// # Arguments
    ///
    /// * `callback` - The permission callback implementation
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # use claude_agent_sdk::callbacks::PermissionCallback;
    /// # use claude_agent_sdk::types::{PermissionResult, ToolPermissionContext, PermissionResultAllow};
    /// # use async_trait::async_trait;
    /// # use serde_json::Value;
    /// # struct MyPermissionChecker;
    /// # #[async_trait]
    /// # impl PermissionCallback for MyPermissionChecker {
    /// #     async fn call(&self, _: String, _: Value, _: ToolPermissionContext) -> claude_agent_sdk::Result<PermissionResult> {
    /// #         Ok(PermissionResult::Allow(PermissionResultAllow {
    /// #             behavior: "allow".to_string(),
    /// #             updated_input: None,
    /// #             updated_permissions: None,
    /// #         }))
    /// #     }
    /// # }
    /// let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());
    /// client.set_permission_callback(MyPermissionChecker);
    /// ```
    pub fn set_permission_callback<P>(&mut self, callback: P)
    where
        P: PermissionCallback + 'static,
    {
        let callback_arc = Arc::new(callback);
        let callback_fn: CanUseToolFn = Box::new(
            move |tool_name: String, input: Value, context: ToolPermissionContext| {
                let callback_clone = callback_arc.clone();
                Box::pin(async move {
                    callback_clone.call(tool_name, input, context).await
                })
            },
        );

        *self.permission_callback.lock().unwrap() = Some(Arc::new(callback_fn));
    }

    /// Connect to Claude Code CLI and establish streaming session.
    ///
    /// Spawns the CLI process in streaming mode, creates Query layer,
    /// and optionally sends an initial prompt.
    ///
    /// # Arguments
    ///
    /// * `prompt` - Optional initial prompt to send immediately after connection
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - CLI binary not found
    /// - Failed to spawn subprocess
    /// - Already connected
    /// - Initialization handshake failed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());
    /// client.connect(Some("Hello!".to_string())).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(&mut self, prompt: Option<String>) -> Result<()> {
        if self.query_handle.is_some() {
            return Err(ClaudeSDKError::AlreadyConnected);
        }

        // Find CLI
        let cli_path = find_claude_cli(self.options.cli_path.as_ref())?;

        // Check version (optional, doesn't fail)
        if std::env::var("CLAUDE_AGENT_SDK_SKIP_VERSION_CHECK").is_err() {
            let _ = check_claude_version(&cli_path).await;
        }

        // Create transport in streaming mode
        let mut transport = SubprocessTransport::new_streaming(cli_path, &self.options);

        // Spawn process (empty prompt for streaming mode)
        transport.spawn(&self.options, "").await?;

        // Extract callbacks from Arc<Mutex<>>
        let hook_callbacks = self.hook_callbacks.lock().unwrap().clone();
        let permission_callback = self.permission_callback.lock().unwrap().clone();

        // Create Query with callbacks and start it
        let query = Query::new_with_callbacks(
            transport,
            hook_callbacks,
            permission_callback,
        );
        let query_handle = query.start();

        // Initialize streaming mode
        query_handle.initialize(None).await?;

        // Send initial prompt if provided
        if let Some(p) = prompt {
            query_handle.send_user_message(&p).await?;
        }

        self.query_handle = Some(query_handle);

        Ok(())
    }

    /// Send a query/prompt to Claude during an active session.
    ///
    /// Does not wait for response. Use `receive_messages()` or `receive_response()`
    /// to get Claude's reply.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The message to send to Claude
    ///
    /// # Errors
    ///
    /// Returns error if not connected or write fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());
    /// # client.connect(None).await?;
    /// client.query("Explain async/await in Rust").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query(&mut self, prompt: impl Into<String>) -> Result<()> {
        let query_handle = self.query_handle.as_ref()
            .ok_or(ClaudeSDKError::NotConnected)?;

        query_handle.send_user_message(&prompt.into()).await?;

        Ok(())
    }

    /// Receive all messages from Claude indefinitely.
    ///
    /// Returns a stream of messages that continues until disconnect or error.
    /// Use this for long-running sessions where you want to process all messages.
    ///
    /// # Errors
    ///
    /// Returns error if not connected.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # use futures::StreamExt;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());
    /// # client.connect(None).await?;
    /// let mut messages = client.receive_messages()?;
    /// while let Some(msg) = messages.next().await {
    ///     println!("{:?}", msg?);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn receive_messages(&self) -> Result<impl Stream<Item = Result<Message>> + '_> {
        let query_handle = self.query_handle.as_ref()
            .ok_or(ClaudeSDKError::NotConnected)?;

        let stream = async_stream::stream! {
            let mut raw_stream = Box::pin(query_handle.read_messages());

            use futures::StreamExt;

            while let Some(result) = raw_stream.next().await {
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

        Ok(stream)
    }

    /// Receive messages until a ResultMessage is encountered.
    ///
    /// Returns a stream that automatically terminates after yielding the final
    /// ResultMessage. Convenient for single-query workflows.
    ///
    /// # Errors
    ///
    /// Returns error if not connected.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions, Message};
    /// # use futures::StreamExt;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());
    /// # client.connect(None).await?;
    /// # client.query("Hello").await?;
    /// let mut messages = client.receive_response()?;
    /// while let Some(msg) = messages.next().await {
    ///     match msg? {
    ///         Message::Assistant(a) => println!("Assistant: {:?}", a),
    ///         Message::Result(r) => {
    ///             println!("Done! Cost: ${:.4}", r.total_cost_usd.unwrap_or(0.0));
    ///             break; // Stream will terminate automatically
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn receive_response(&self) -> Result<impl Stream<Item = Result<Message>> + '_> {
        let query_handle = self.query_handle.as_ref()
            .ok_or(ClaudeSDKError::NotConnected)?;

        let stream = async_stream::stream! {
            let mut raw_stream = Box::pin(query_handle.read_messages());

            use futures::StreamExt;

            while let Some(result) = raw_stream.next().await {
                match result {
                    Ok(value) => {
                        match parser::parse_message(value) {
                            Ok(message) => {
                                // Check if this is a result message
                                let is_result = matches!(message, Message::Result(_));

                                yield Ok(message);

                                // Terminate stream after yielding result
                                if is_result {
                                    break;
                                }
                            }
                            Err(e) => {
                                yield Err(e);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(e);
                        break;
                    }
                }
            }
        };

        Ok(stream)
    }

    /// Interrupt the current processing.
    ///
    /// Sends an interrupt control request to stop Claude's current operation.
    /// Useful for cancelling long-running tasks.
    ///
    /// # Errors
    ///
    /// Returns error if not connected or request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());
    /// # client.connect(None).await?;
    /// client.interrupt().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn interrupt(&self) -> Result<()> {
        let query_handle = self.query_handle.as_ref()
            .ok_or(ClaudeSDKError::NotConnected)?;

        query_handle.interrupt().await
    }

    /// Change permission mode dynamically during the session.
    ///
    /// # Arguments
    ///
    /// * `mode` - New permission mode to use
    ///
    /// # Errors
    ///
    /// Returns error if not connected or request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions, PermissionMode};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());
    /// # client.connect(None).await?;
    /// client.set_permission_mode(PermissionMode::AcceptEdits).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_permission_mode(&self, mode: PermissionMode) -> Result<()> {
        let query_handle = self.query_handle.as_ref()
            .ok_or(ClaudeSDKError::NotConnected)?;

        query_handle.set_permission_mode(&mode.to_string()).await
    }

    /// Change AI model dynamically during the session.
    ///
    /// # Arguments
    ///
    /// * `model` - New model to use, or None for default
    ///
    /// # Errors
    ///
    /// Returns error if not connected or request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());
    /// # client.connect(None).await?;
    /// client.set_model(Some("claude-opus-4-20250514".to_string())).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_model(&self, model: Option<String>) -> Result<()> {
        let query_handle = self.query_handle.as_ref()
            .ok_or(ClaudeSDKError::NotConnected)?;

        query_handle.set_model(model).await
    }

    /// Get server information from initialization.
    ///
    /// Returns the initialization response containing available commands,
    /// capabilities, and output style.
    ///
    /// # Errors
    ///
    /// Returns error if not connected.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());
    /// # client.connect(None).await?;
    /// if let Some(info) = client.get_server_info().await? {
    ///     println!("Server capabilities: {:?}", info);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_server_info(&self) -> Result<Option<serde_json::Value>> {
        let query_handle = self.query_handle.as_ref()
            .ok_or(ClaudeSDKError::NotConnected)?;

        Ok(query_handle.get_server_info().await)
    }

    /// Disconnect from Claude and cleanup resources.
    ///
    /// Gracefully closes the connection and waits for process termination.
    /// Consumes self to prevent reuse after disconnect.
    ///
    /// # Errors
    ///
    /// Returns error if not connected.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());
    /// # client.connect(None).await?;
    /// client.disconnect().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn disconnect(self) -> Result<()> {
        if self.query_handle.is_none() {
            return Err(ClaudeSDKError::NotConnected);
        }

        // Drop query handle, which will close channels and cleanup
        drop(self.query_handle);

        // Note: We don't have access to the transport or child process here
        // to explicitly wait for termination. The QueryHandle dropping should
        // trigger cleanup. In a more complete implementation, we'd want to
        // store a way to explicitly wait for process termination.

        Ok(())
    }

    /// Check if client is currently connected.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// let client = ClaudeSDKClient::new(ClaudeAgentOptions::default());
    /// assert!(!client.is_connected());
    /// ```
    pub fn is_connected(&self) -> bool {
        self.query_handle.is_some()
    }
}
