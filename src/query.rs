//! Query layer - Control protocol, message routing, and callback management.
//!
//! The Query layer sits between the Transport layer and the public API,
//! handling bidirectional communication with the CLI through the control protocol.

use crate::error::{ClaudeSDKError, Result};
use crate::transport::subprocess::SubprocessTransport;
use crate::types::{HookContext, HookInput, HookOutput, PermissionResult, ToolPermissionContext};

use futures::Stream;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;
use tokio::sync::{Mutex, mpsc, oneshot};

/// Type alias for the pending control-response map.
pub type PendingResponses = Arc<Mutex<HashMap<String, oneshot::Sender<Result<Value>>>>>;

/// Type for async hook callback functions.
pub type HookCallbackFn = Box<
    dyn Fn(
            HookInput,
            Option<String>,
            HookContext,
        ) -> Pin<Box<dyn Future<Output = Result<HookOutput>> + Send>>
        + Send
        + Sync,
>;

/// Type for permission callback functions.
pub type CanUseToolFn = Box<
    dyn Fn(
            String,
            Value,
            ToolPermissionContext,
        ) -> Pin<Box<dyn Future<Output = Result<PermissionResult>> + Send>>
        + Send
        + Sync,
>;

/// Query manages the control protocol and routes messages between CLI and SDK.
pub struct Query {
    /// Transport for communicating with CLI subprocess.
    transport: SubprocessTransport,

    /// Pending control responses awaiting completion.
    pending_responses: Arc<Mutex<HashMap<String, oneshot::Sender<Result<Value>>>>>,

    /// Counter for generating unique request IDs.
    request_counter: Arc<Mutex<u64>>,

    /// Hook callbacks indexed by callback ID.
    hook_callbacks: Arc<HashMap<String, Arc<HookCallbackFn>>>,

    /// Permission callback (if set).
    can_use_tool_callback: Arc<Option<Arc<CanUseToolFn>>>,

    /// Channel for SDK messages (non-control messages).
    sdk_messages_tx: mpsc::Sender<Value>,
    sdk_messages_rx: Arc<Mutex<mpsc::Receiver<Value>>>,

    /// Server info from initialization (if streaming mode).
    server_info: Arc<Mutex<Option<Value>>>,
}

impl Query {
    /// Create a new Query with a transport.
    pub fn new(transport: SubprocessTransport) -> Self {
        let (tx, rx) = mpsc::channel(100);

        Self {
            transport,
            pending_responses: Arc::new(Mutex::new(HashMap::new())),
            request_counter: Arc::new(Mutex::new(0)),
            hook_callbacks: Arc::new(HashMap::new()),
            can_use_tool_callback: Arc::new(None),
            sdk_messages_tx: tx,
            sdk_messages_rx: Arc::new(Mutex::new(rx)),
            server_info: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a new Query with callbacks.
    pub fn new_with_callbacks(
        transport: SubprocessTransport,
        hook_callbacks: HashMap<String, Arc<HookCallbackFn>>,
        can_use_tool_callback: Option<Arc<CanUseToolFn>>,
    ) -> Self {
        let (tx, rx) = mpsc::channel(100);

        Self {
            transport,
            pending_responses: Arc::new(Mutex::new(HashMap::new())),
            request_counter: Arc::new(Mutex::new(0)),
            hook_callbacks: Arc::new(hook_callbacks),
            can_use_tool_callback: Arc::new(can_use_tool_callback),
            sdk_messages_tx: tx,
            sdk_messages_rx: Arc::new(Mutex::new(rx)),
            server_info: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the message router background task.
    ///
    /// This consumes self and spawns an async task that reads from transport
    /// and routes messages to either control protocol handlers or SDK message stream.
    pub fn start(mut self) -> QueryHandle {
        // Extract stdin for writing control responses
        let stdin = Arc::new(Mutex::new(self.transport.take_stdin()));

        let pending_responses = self.pending_responses;
        let hook_callbacks = self.hook_callbacks;
        let can_use_tool_callback = self.can_use_tool_callback;
        let sdk_messages_tx = self.sdk_messages_tx.clone();
        let sdk_messages_rx = self.sdk_messages_rx;
        let server_info = self.server_info;
        let request_counter = self.request_counter;
        let current_session_id = Arc::new(std::sync::OnceLock::new());

        // Clone Arc values for use in the async block
        let pending_responses_task = pending_responses.clone();
        let hook_callbacks_task = hook_callbacks.clone();
        let can_use_tool_callback_task = can_use_tool_callback.clone();
        let stdin_task = stdin.clone();
        let current_session_id_task = current_session_id.clone();

        // Spawn background task
        tokio::spawn(async move {
            let mut stream = Box::pin(self.transport.read_messages());

            use futures::StreamExt;

            while let Some(result) = stream.next().await {
                match result {
                    Ok(msg) => {
                        // Route based on message type
                        match msg.get("type").and_then(|v| v.as_str()) {
                            Some("control_response") => {
                                // Handle control response
                                if let Err(e) = Self::handle_control_response_static(
                                    msg,
                                    &pending_responses_task,
                                )
                                .await
                                {
                                    eprintln!("Error handling control response: {}", e);
                                }
                            }
                            Some("control_request") => {
                                // Handle control request (from CLI to SDK)
                                if let Err(e) = Self::handle_control_request_static(
                                    msg,
                                    &hook_callbacks_task,
                                    &can_use_tool_callback_task,
                                    &stdin_task,
                                )
                                .await
                                {
                                    eprintln!("Error handling control request: {}", e);
                                }
                            }
                            _ => {
                                // Extract session ID if present (from result, stream_event, or assistant messages)
                                if let Some(session_id) =
                                    msg.get("session_id").and_then(|v| v.as_str())
                                {
                                    // Try to set session ID (only succeeds once, subsequent calls ignored)
                                    let _ = current_session_id_task.set(session_id.to_string());
                                }

                                // Regular SDK message - send to stream
                                if sdk_messages_tx.send(msg).await.is_err() {
                                    // Receiver dropped, exit loop
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading message: {}", e);
                        break;
                    }
                }
            }

            // Signal end of stream by closing channel
            drop(sdk_messages_tx);
        });

        QueryHandle {
            pending_responses,
            request_counter,
            sdk_messages_rx,
            server_info,
            stdin,
            current_session_id,
        }
    }

    /// Handle control response (match to pending request).
    async fn handle_control_response_static(
        msg: Value,
        pending_responses: &PendingResponses,
    ) -> Result<()> {
        let request_id = msg["response"]["request_id"].as_str().ok_or_else(|| {
            ClaudeSDKError::message_parse("Missing request_id in control_response")
        })?;

        let mut pending = pending_responses.lock().await;
        if let Some(tx) = pending.remove(request_id) {
            let subtype = msg["response"]["subtype"].as_str();

            if subtype == Some("error") {
                let error_msg = msg["response"]["error"].as_str().unwrap_or("Unknown error");
                let _ = tx.send(Err(ClaudeSDKError::other(error_msg)));
            } else {
                let response = msg["response"]["response"].clone();
                let _ = tx.send(Ok(response));
            }
        }

        Ok(())
    }

    /// Handle control request (from CLI to SDK).
    async fn handle_control_request_static(
        msg: Value,
        hook_callbacks: &Arc<HashMap<String, Arc<HookCallbackFn>>>,
        can_use_tool_callback: &Arc<Option<Arc<CanUseToolFn>>>,
        stdin: &Arc<Mutex<Option<ChildStdin>>>,
    ) -> Result<()> {
        let request_id = msg["request_id"]
            .as_str()
            .ok_or_else(|| ClaudeSDKError::message_parse("Missing request_id in control_request"))?
            .to_string();

        let request = &msg["request"];
        let subtype = request["subtype"]
            .as_str()
            .ok_or_else(|| ClaudeSDKError::message_parse("Missing subtype in control_request"))?;

        // Execute appropriate handler
        let response_result: Result<Value> = match subtype {
            "hook_callback" => Self::handle_hook_callback(request.clone(), hook_callbacks).await,
            "can_use_tool" => {
                Self::handle_can_use_tool(request.clone(), can_use_tool_callback).await
            }
            "mcp_message" => {
                // MCP bridging not yet implemented
                Err(ClaudeSDKError::other("MCP bridging not yet implemented"))
            }
            _ => Err(ClaudeSDKError::message_parse(format!(
                "Unknown control request subtype: {}",
                subtype
            ))),
        };

        // Send control response back
        let response_msg = match response_result {
            Ok(response) => {
                json!({
                    "type": "control_response",
                    "response": {
                        "subtype": "success",
                        "request_id": request_id,
                        "response": response
                    }
                })
            }
            Err(e) => {
                json!({
                    "type": "control_response",
                    "response": {
                        "subtype": "error",
                        "request_id": request_id,
                        "error": e.to_string()
                    }
                })
            }
        };

        // Write response to stdin
        let response_str = serde_json::to_string(&response_msg)?;
        Self::write_to_stdin(stdin, &response_str).await?;

        Ok(())
    }

    /// Write data to stdin.
    async fn write_to_stdin(stdin: &Arc<Mutex<Option<ChildStdin>>>, data: &str) -> Result<()> {
        let mut stdin_guard = stdin.lock().await;
        let stdin_ref = stdin_guard
            .as_mut()
            .ok_or(ClaudeSDKError::TransportNotReady)?;

        stdin_ref.write_all(data.as_bytes()).await?;
        stdin_ref.write_all(b"\n").await?;
        stdin_ref.flush().await?;

        Ok(())
    }

    /// Handle hook callback request.
    async fn handle_hook_callback(
        request: Value,
        hook_callbacks: &Arc<HashMap<String, Arc<HookCallbackFn>>>,
    ) -> Result<Value> {
        let callback_id = request["callback_id"]
            .as_str()
            .ok_or_else(|| ClaudeSDKError::message_parse("Missing callback_id"))?;

        let callback = hook_callbacks
            .get(callback_id)
            .ok_or_else(|| ClaudeSDKError::HookNotFound(callback_id.to_string()))?;

        // Parse hook input
        let input: HookInput = serde_json::from_value(request["input"].clone())?;
        let tool_use_id = request["tool_use_id"].as_str().map(|s| s.to_string());
        let context = HookContext { signal: None };

        // Execute callback
        let output = callback.as_ref()(input, tool_use_id, context).await?;

        // Convert field names for CLI (async_ → async, continue_ → continue)
        let json_output = Self::convert_hook_output_for_cli(output)?;

        Ok(json_output)
    }

    /// Handle can_use_tool permission callback request.
    async fn handle_can_use_tool(
        request: Value,
        can_use_tool_callback: &Arc<Option<Arc<CanUseToolFn>>>,
    ) -> Result<Value> {
        let callback = can_use_tool_callback
            .as_ref()
            .as_ref()
            .ok_or(ClaudeSDKError::PermissionCallbackNotSet)?;

        let tool_name = request["tool_name"]
            .as_str()
            .ok_or_else(|| ClaudeSDKError::message_parse("Missing tool_name"))?
            .to_string();

        let input = request["input"].clone();
        let suggestions = request["permission_suggestions"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();

        let context = ToolPermissionContext {
            signal: None,
            suggestions,
        };

        // Execute callback
        let result = callback.as_ref()(tool_name.clone(), input.clone(), context).await?;

        // Convert to JSON response
        let json_result = match result {
            PermissionResult::Allow(allow) => {
                let mut obj = json!({
                    "behavior": "allow",
                    // CLI requires updatedInput to always be present, even if unchanged
                    "updatedInput": allow.updated_input.unwrap_or(input)
                });

                if let Some(updated_permissions) = allow.updated_permissions {
                    obj["updatedPermissions"] = serde_json::to_value(updated_permissions)?;
                }

                obj
            }
            PermissionResult::Deny(deny) => {
                json!({
                    "behavior": "deny",
                    "message": deny.message,
                    "interrupt": deny.interrupt
                })
            }
        };

        Ok(json_result)
    }

    /// Convert HookOutput to JSON with proper field names for CLI.
    ///
    /// Rust uses `async_` and `continue_` to avoid keywords, but CLI expects
    /// `async` and `continue` in JSON.
    fn convert_hook_output_for_cli(output: HookOutput) -> Result<Value> {
        let mut json = serde_json::to_value(&output)?;

        // Handle different output types
        match output {
            HookOutput::Sync(_) => {
                // Convert continue_ to continue
                if let Some(obj) = json.as_object_mut()
                    && let Some(continue_val) = obj.remove("continue_")
                {
                    obj.insert("continue".to_string(), continue_val);
                }
            }
            HookOutput::Async(_) => {
                // Convert async_ to async
                if let Some(obj) = json.as_object_mut()
                    && let Some(async_val) = obj.remove("async_")
                {
                    obj.insert("async".to_string(), async_val);
                }
            }
        }

        Ok(json)
    }
}

/// Handle to a running Query.
pub struct QueryHandle {
    /// Pending control responses awaiting completion.
    pending_responses: Arc<Mutex<HashMap<String, oneshot::Sender<Result<Value>>>>>,

    /// Counter for generating unique request IDs.
    request_counter: Arc<Mutex<u64>>,

    /// Channel for SDK messages (non-control messages).
    sdk_messages_rx: Arc<Mutex<mpsc::Receiver<Value>>>,

    /// Server info from initialization (if streaming mode).
    server_info: Arc<Mutex<Option<Value>>>,

    /// Stdin for writing control requests.
    stdin: Arc<Mutex<Option<ChildStdin>>>,

    /// Current session ID (captured from messages).
    current_session_id: Arc<std::sync::OnceLock<String>>,
}

impl QueryHandle {
    /// Get stream of SDK messages (non-control messages).
    pub fn read_messages(&self) -> impl Stream<Item = Result<Value>> + '_ {
        let rx = self.sdk_messages_rx.clone();

        async_stream::stream! {
            let mut rx_guard = rx.lock().await;

            while let Some(msg) = rx_guard.recv().await {
                yield Ok(msg);
            }
        }
    }

    /// Get server info from initialization (if available).
    pub async fn get_server_info(&self) -> Option<Value> {
        let server_info = self.server_info.lock().await;
        server_info.clone()
    }

    /// Get the current session ID.
    ///
    /// Returns the session ID once it has been captured from messages.
    /// Returns None if no session has been established yet.
    pub fn get_session_id(&self) -> Option<String> {
        self.current_session_id.get().cloned()
    }

    /// Send a control request and wait for response.
    async fn send_control_request(&self, request: Value) -> Result<Value> {
        // Generate unique request ID
        let mut counter = self.request_counter.lock().await;
        *counter += 1;
        let request_id = format!("req_{}_{:08x}", *counter, rand::random::<u32>());
        drop(counter);

        // Create oneshot channel for response
        let (tx, rx) = oneshot::channel();

        // Store in pending responses
        {
            let mut pending = self.pending_responses.lock().await;
            pending.insert(request_id.clone(), tx);
        }

        // Send control request
        let control_msg = json!({
            "type": "control_request",
            "request_id": request_id,
            "request": request
        });

        let control_str = serde_json::to_string(&control_msg)?;
        Query::write_to_stdin(&self.stdin, &control_str).await?;

        // Wait for response with timeout
        tokio::time::timeout(std::time::Duration::from_secs(60), rx)
            .await
            .map_err(|_| {
                ClaudeSDKError::control_timeout(
                    60,
                    request["subtype"].as_str().unwrap_or("unknown").to_string(),
                )
            })?
            .map_err(|_| ClaudeSDKError::other("Response channel closed"))?
    }

    /// Initialize streaming mode connection.
    pub async fn initialize(&self, hooks: Option<Value>) -> Result<Value> {
        let mut request = json!({
            "subtype": "initialize"
        });

        if let Some(hooks_val) = hooks {
            request["hooks"] = hooks_val;
        }

        let response = self.send_control_request(request).await?;

        // Store server info
        {
            let mut server_info = self.server_info.lock().await;
            *server_info = Some(response.clone());
        }

        Ok(response)
    }

    /// Send interrupt control request to stop current processing.
    pub async fn interrupt(&self) -> Result<()> {
        let request = json!({
            "subtype": "interrupt"
        });

        self.send_control_request(request).await?;
        Ok(())
    }

    /// Change permission mode dynamically.
    pub async fn set_permission_mode(&self, mode: &str) -> Result<()> {
        let request = json!({
            "subtype": "set_permission_mode",
            "mode": mode
        });

        self.send_control_request(request).await?;
        Ok(())
    }

    /// Change AI model dynamically.
    pub async fn set_model(&self, model: Option<String>) -> Result<()> {
        let request = json!({
            "subtype": "set_model",
            "model": model
        });

        self.send_control_request(request).await?;
        Ok(())
    }

    /// Rewind tracked files to their state at a specific user message.
    pub async fn rewind_files(&self, user_message_id: &str) -> Result<()> {
        let request = json!({
            "subtype": "rewind_files",
            "user_message_id": user_message_id
        });

        self.send_control_request(request).await?;
        Ok(())
    }

    /// Get current MCP server connection status.
    pub async fn get_mcp_status(&self) -> Result<Value> {
        let request = json!({
            "subtype": "mcp_status"
        });

        self.send_control_request(request).await
    }

    /// Send a user message to CLI.
    pub async fn send_user_message(&self, prompt: &str) -> Result<()> {
        let message = json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": prompt
            }
        });

        let message_str = serde_json::to_string(&message)?;
        Query::write_to_stdin(&self.stdin, &message_str).await?;

        Ok(())
    }

    /// Close stdin to signal no more input.
    ///
    /// Used in one-shot query mode after sending the prompt to tell the CLI
    /// that no more messages are coming.
    pub async fn close_stdin(&self) {
        let mut stdin_guard = self.stdin.lock().await;
        // Drop the stdin handle to close the pipe
        *stdin_guard = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AsyncHookOutput, SyncHookOutput};

    #[test]
    fn test_hook_output_field_conversion() {
        // Test Sync output with continue_
        let sync_output = HookOutput::Sync(Box::new(SyncHookOutput {
            continue_: Some(true),
            suppress_output: None,
            stop_reason: None,
            decision: None,
            system_message: None,
            reason: None,
            hook_specific_output: None,
        }));

        let json = Query::convert_hook_output_for_cli(sync_output).unwrap();
        assert!(json.get("continue").is_some());
        assert!(json.get("continue_").is_none());

        // Test Async output with async_
        let async_output = HookOutput::Async(AsyncHookOutput {
            async_: true,
            async_timeout: Some(5000),
        });

        let json = Query::convert_hook_output_for_cli(async_output).unwrap();
        assert!(json.get("async").is_some());
        assert!(json.get("async_").is_none());
    }
}
