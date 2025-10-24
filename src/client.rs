//! ClaudeSDKClient - Interactive bidirectional client for multi-turn conversations.
//!
//! Provides a high-level API for interactive sessions with Claude Code CLI.

use crate::callbacks::{HookCallback, PermissionCallback};
use crate::error::{ClaudeSDKError, Result};
use crate::parser;
use crate::query::{CanUseToolFn, HookCallbackFn, Query, QueryHandle};
use crate::types::{HookContext, HookEvent, HookInput, HookMatcherConfig, ToolPermissionContext};
use crate::transport::{check_claude_version, find_claude_cli, subprocess::SubprocessTransport};
use crate::types::{ClaudeAgentOptions, Message, PermissionMode};

use futures::Stream;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Hook registration metadata.
#[derive(Debug, Clone)]
struct HookRegistration {
    event: HookEvent,
    matcher: Option<String>,
    callback_id: String,
}

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

    /// Hook registrations metadata (to build hooks config for initialize).
    hook_registrations: Vec<HookRegistration>,

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
            hook_registrations: Vec::new(),
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
        event: HookEvent,
        matcher: Option<&str>,
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

        // Store registration metadata
        self.hook_registrations.push(HookRegistration {
            event,
            matcher: matcher.map(|s| s.to_string()),
            callback_id: callback_id.clone(),
        });

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

    /// Build hooks configuration for initialize request.
    ///
    /// Groups hook registrations by event and creates HookMatcherConfig entries.
    fn build_hooks_config(&self) -> Option<HashMap<String, Vec<HookMatcherConfig>>> {
        if self.hook_registrations.is_empty() {
            return None;
        }

        let mut hooks_by_event: HashMap<HookEvent, HashMap<Option<String>, Vec<String>>> = HashMap::new();

        // Group callbacks by event and matcher
        for reg in &self.hook_registrations {
            let matchers = hooks_by_event.entry(reg.event).or_insert_with(HashMap::new);
            let callback_ids = matchers.entry(reg.matcher.clone()).or_insert_with(Vec::new);
            callback_ids.push(reg.callback_id.clone());
        }

        // Convert to HookMatcherConfig format
        let mut hooks_config: HashMap<String, Vec<HookMatcherConfig>> = HashMap::new();

        for (event, matchers) in hooks_by_event {
            let event_key = format!("{:?}", event); // "PreToolUse", "PostToolUse", etc.

            let matcher_configs: Vec<HookMatcherConfig> = matchers
                .into_iter()
                .map(|(matcher, callback_ids)| HookMatcherConfig {
                    matcher,
                    hook_callback_ids: callback_ids,
                })
                .collect();

            hooks_config.insert(event_key, matcher_configs);
        }

        Some(hooks_config)
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

        // Auto-set permission_prompt_tool_name if permission callback is registered
        let mut options_for_transport = self.options.clone();
        {
            let callback = self.permission_callback.lock().unwrap();
            if callback.is_some() && options_for_transport.permission_prompt_tool_name.is_none() {
                options_for_transport.permission_prompt_tool_name = Some("stdio".to_string());
            }
        }

        // Create transport in streaming mode
        let mut transport = SubprocessTransport::new_streaming(cli_path, &options_for_transport);

        // Spawn process (empty prompt for streaming mode)
        transport.spawn(&options_for_transport, "").await?;

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

        // Build hooks configuration
        let hooks_config = self.build_hooks_config();
        let hooks_value = hooks_config.map(|config| serde_json::to_value(config).unwrap());

        // Initialize streaming mode with hooks config
        query_handle.initialize(hooks_value).await?;

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

    /// Get the current session ID.
    ///
    /// Returns the session ID once it has been captured from messages.
    /// The session ID is extracted automatically from the first message
    /// that contains it (typically ResultMessage or StreamEvent).
    ///
    /// Returns None if:
    /// - Not connected yet
    /// - No messages have been received
    /// - Session hasn't been established
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # use futures::StreamExt;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = ClaudeSDKClient::new(ClaudeAgentOptions::default());
    /// client.connect(Some("Hello".to_string())).await?;
    ///
    /// // Process messages
    /// let mut messages = client.receive_messages()?;
    /// while let Some(msg) = messages.next().await {
    ///     let _ = msg?;
    ///     // Session ID gets captured automatically
    ///     if let Some(session_id) = client.get_session_id() {
    ///         println!("Session: {}", session_id);
    ///         break;
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_session_id(&self) -> Option<String> {
        self.query_handle
            .as_ref()
            .and_then(|handle| handle.get_session_id())
    }

    /// Get current usage data for Claude Code (OAuth/Max Plan users only).
    ///
    /// This method retrieves usage statistics including:
    /// - 5-hour rolling window usage
    /// - 7-day (weekly) usage across all models
    /// - 7-day OAuth apps usage
    /// - 7-day Opus-specific usage
    ///
    /// Each usage limit includes:
    /// - `utilization`: Percentage used (0-100)
    /// - `resets_at`: ISO 8601 timestamp when limit resets
    ///
    /// # Requirements
    ///
    /// - Must have OAuth credentials in `~/.claude/.credentials.json`
    /// - Only works with Max Plan subscriptions (not API keys)
    /// - Requires valid, non-expired access token
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Credentials file not found or invalid
    /// - Access token expired (use Claude CLI to refresh)
    /// - Network request fails
    /// - API returns error (401 for invalid token)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ClaudeSDKClient::new(ClaudeAgentOptions::default());
    ///
    /// let usage = client.get_usage().await?;
    ///
    /// println!("5-hour usage: {}%", usage.five_hour.utilization);
    /// println!("Weekly usage: {}%", usage.seven_day.utilization);
    ///
    /// if usage.is_approaching_limit() {
    ///     println!("Warning: Approaching usage limit!");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_usage(&self) -> Result<crate::types::UsageData> {
        use crate::types::UsageData;

        // Read OAuth credentials
        let credentials = Self::read_oauth_credentials()?;

        // Extract access token
        let access_token = credentials
            .get("claudeAiOauth")
            .and_then(|oauth| oauth.get("accessToken"))
            .and_then(|token| token.as_str())
            .ok_or_else(|| ClaudeSDKError::AuthenticationError(
                "No access token found in credentials file".to_string()
            ))?;

        // Make request to usage endpoint
        let client = reqwest::Client::new();
        let response = client
            .get("https://api.anthropic.com/api/oauth/usage")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("anthropic-beta", "oauth-2025-04-20")
            .send()
            .await
            .map_err(|e| ClaudeSDKError::NetworkError(e.to_string()))?;

        // Check for errors
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ClaudeSDKError::NetworkError(
                format!("Usage API request failed ({}): {}", status, error_text)
            ));
        }

        // Parse response
        let usage: UsageData = response
            .json()
            .await
            .map_err(|e| ClaudeSDKError::ParseError(format!("Failed to parse usage response: {}", e)))?;

        Ok(usage)
    }

    /// Read OAuth credentials from ~/.claude/.credentials.json
    fn read_oauth_credentials() -> Result<serde_json::Value> {
        let home_dir = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| ClaudeSDKError::AuthenticationError(
                "Cannot determine home directory".to_string()
            ))?;

        let credentials_path = std::path::PathBuf::from(home_dir)
            .join(".claude")
            .join(".credentials.json");

        let contents = std::fs::read_to_string(&credentials_path)
            .map_err(|e| ClaudeSDKError::AuthenticationError(
                format!("Failed to read credentials file at {:?}: {}", credentials_path, e)
            ))?;

        serde_json::from_str(&contents)
            .map_err(|e| ClaudeSDKError::ParseError(
                format!("Invalid credentials file format: {}", e)
            ))
    }
}
