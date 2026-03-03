//! Callback infrastructure for hooks and permissions.
//!
//! Provides user-friendly traits and types for implementing custom callbacks
//! that execute during the agent loop.

use crate::error::Result;
use crate::types::{
    AsyncHookOutput, HookContext, HookInput, HookOutput, PermissionResult, SyncHookOutput,
    ToolPermissionContext,
};

use async_trait::async_trait;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

/// Trait for hook callbacks.
///
/// Implement this trait to create custom hooks that execute at specific points
/// in the agent loop (e.g., before/after tool use, on prompt submit, etc.).
///
/// # Example
///
/// ```no_run
/// use claude_agent_sdk_rust::callbacks::HookCallback;
/// use claude_agent_sdk_rust::types::{HookInput, HookOutput, HookContext, SyncHookOutput};
/// use claude_agent_sdk_rust::Result;
/// use async_trait::async_trait;
///
/// struct LoggingHook;
///
/// #[async_trait]
/// impl HookCallback for LoggingHook {
///     async fn call(
///         &self,
///         input: HookInput,
///         tool_use_id: Option<String>,
///         _context: HookContext,
///     ) -> Result<HookOutput> {
///         println!("Hook called: {:?}", input);
///
///         // Allow execution to continue
///         Ok(HookOutput::Sync(Box::new(SyncHookOutput {
///             continue_: Some(true),
///             suppress_output: None,
///             stop_reason: None,
///             decision: None,
///             system_message: None,
///             reason: None,
///             hook_specific_output: None,
///         })))
///     }
/// }
/// ```
#[async_trait]
pub trait HookCallback: Send + Sync {
    /// Execute the hook callback.
    ///
    /// # Arguments
    ///
    /// * `input` - Hook input containing event-specific data
    /// * `tool_use_id` - ID of the tool use (if applicable)
    /// * `context` - Additional context (signal, etc.)
    ///
    /// # Returns
    ///
    /// Hook output indicating whether to continue, any modifications, etc.
    async fn call(
        &self,
        input: HookInput,
        tool_use_id: Option<String>,
        context: HookContext,
    ) -> Result<HookOutput>;
}

/// Trait for permission callbacks.
///
/// Implement this trait to create custom permission handlers that control
/// which tools Claude can use and how.
///
/// # Example
///
/// ```no_run
/// use claude_agent_sdk_rust::callbacks::PermissionCallback;
/// use claude_agent_sdk_rust::types::{PermissionResult, ToolPermissionContext, PermissionResultAllow};
/// use claude_agent_sdk_rust::Result;
/// use async_trait::async_trait;
/// use serde_json::Value;
///
/// struct SafetyChecker;
///
/// #[async_trait]
/// impl PermissionCallback for SafetyChecker {
///     async fn call(
///         &self,
///         tool_name: String,
///         input: Value,
///         _context: ToolPermissionContext,
///     ) -> Result<PermissionResult> {
///         // Block dangerous bash commands
///         if tool_name == "Bash" {
///             if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
///                 if cmd.contains("rm -rf") {
///                     return Ok(PermissionResult::Deny(
///                         claude_agent_sdk_rust::types::PermissionResultDeny {
///                             behavior: "deny".to_string(),
///                             message: "Dangerous command blocked".to_string(),
///                             interrupt: false,
///                         }
///                     ));
///                 }
///             }
///         }
///
///         // Allow by default
///         Ok(PermissionResult::Allow(PermissionResultAllow {
///             behavior: "allow".to_string(),
///             updated_input: None,
///             updated_permissions: None,
///         }))
///     }
/// }
/// ```
#[async_trait]
pub trait PermissionCallback: Send + Sync {
    /// Execute the permission callback.
    ///
    /// # Arguments
    ///
    /// * `tool_name` - Name of the tool being checked
    /// * `input` - Tool input parameters
    /// * `context` - Additional context (suggestions, etc.)
    ///
    /// # Returns
    ///
    /// Permission result indicating allow or deny with optional modifications.
    async fn call(
        &self,
        tool_name: String,
        input: Value,
        context: ToolPermissionContext,
    ) -> Result<PermissionResult>;
}

/// Helper to convert a closure into a HookCallback.
///
/// This allows using simple closures as hook callbacks without implementing
/// the full trait.
pub struct ClosureHook<F>
where
    F: Fn(
            HookInput,
            Option<String>,
            HookContext,
        ) -> Pin<Box<dyn Future<Output = Result<HookOutput>> + Send>>
        + Send
        + Sync,
{
    func: F,
}

impl<F> ClosureHook<F>
where
    F: Fn(
            HookInput,
            Option<String>,
            HookContext,
        ) -> Pin<Box<dyn Future<Output = Result<HookOutput>> + Send>>
        + Send
        + Sync,
{
    /// Create a new ClosureHook.
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

#[async_trait]
impl<F> HookCallback for ClosureHook<F>
where
    F: Fn(
            HookInput,
            Option<String>,
            HookContext,
        ) -> Pin<Box<dyn Future<Output = Result<HookOutput>> + Send>>
        + Send
        + Sync,
{
    async fn call(
        &self,
        input: HookInput,
        tool_use_id: Option<String>,
        context: HookContext,
    ) -> Result<HookOutput> {
        (self.func)(input, tool_use_id, context).await
    }
}

/// Helper to convert a closure into a PermissionCallback.
pub struct ClosurePermission<F>
where
    F: Fn(
            String,
            Value,
            ToolPermissionContext,
        ) -> Pin<Box<dyn Future<Output = Result<PermissionResult>> + Send>>
        + Send
        + Sync,
{
    func: F,
}

impl<F> ClosurePermission<F>
where
    F: Fn(
            String,
            Value,
            ToolPermissionContext,
        ) -> Pin<Box<dyn Future<Output = Result<PermissionResult>> + Send>>
        + Send
        + Sync,
{
    /// Create a new ClosurePermission.
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

#[async_trait]
impl<F> PermissionCallback for ClosurePermission<F>
where
    F: Fn(
            String,
            Value,
            ToolPermissionContext,
        ) -> Pin<Box<dyn Future<Output = Result<PermissionResult>> + Send>>
        + Send
        + Sync,
{
    async fn call(
        &self,
        tool_name: String,
        input: Value,
        context: ToolPermissionContext,
    ) -> Result<PermissionResult> {
        (self.func)(tool_name, input, context).await
    }
}

/// Helper functions to create common hook responses.
pub mod hooks {
    use super::*;

    /// Create a hook output that allows execution to continue.
    pub fn allow() -> HookOutput {
        HookOutput::Sync(Box::new(SyncHookOutput {
            continue_: Some(true),
            suppress_output: None,
            stop_reason: None,
            decision: None,
            system_message: None,
            reason: None,
            hook_specific_output: None,
        }))
    }

    /// Create a hook output that blocks execution.
    pub fn block(reason: impl Into<String>) -> HookOutput {
        HookOutput::Sync(Box::new(SyncHookOutput {
            continue_: Some(false),
            suppress_output: None,
            stop_reason: Some(reason.into()),
            decision: Some("block".to_string()),
            system_message: None,
            reason: None,
            hook_specific_output: None,
        }))
    }

    /// Create a hook output that allows with a message to the user.
    pub fn allow_with_message(message: impl Into<String>) -> HookOutput {
        HookOutput::Sync(Box::new(SyncHookOutput {
            continue_: Some(true),
            suppress_output: None,
            stop_reason: None,
            decision: None,
            system_message: Some(message.into()),
            reason: None,
            hook_specific_output: None,
        }))
    }

    /// Create a hook output that defers execution (async).
    pub fn defer(timeout_ms: Option<u32>) -> HookOutput {
        HookOutput::Async(AsyncHookOutput {
            async_: true,
            async_timeout: timeout_ms,
        })
    }
}

/// Helper functions to create common permission responses.
pub mod permissions {
    use super::*;
    use crate::types::{PermissionResultAllow, PermissionResultDeny};

    /// Create a permission result that allows the tool use.
    pub fn allow() -> PermissionResult {
        PermissionResult::Allow(PermissionResultAllow {
            behavior: "allow".to_string(),
            updated_input: None,
            updated_permissions: None,
        })
    }

    /// Create a permission result that allows with modified input.
    pub fn allow_with_input(updated_input: Value) -> PermissionResult {
        PermissionResult::Allow(PermissionResultAllow {
            behavior: "allow".to_string(),
            updated_input: Some(updated_input),
            updated_permissions: None,
        })
    }

    /// Create a permission result that denies the tool use.
    pub fn deny(message: impl Into<String>) -> PermissionResult {
        PermissionResult::Deny(PermissionResultDeny {
            behavior: "deny".to_string(),
            message: message.into(),
            interrupt: false,
        })
    }

    /// Create a permission result that denies and stops the session.
    pub fn deny_and_interrupt(message: impl Into<String>) -> PermissionResult {
        PermissionResult::Deny(PermissionResultDeny {
            behavior: "deny".to_string(),
            message: message.into(),
            interrupt: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_helpers() {
        let allow = hooks::allow();
        assert!(matches!(allow, HookOutput::Sync(_)));

        let block = hooks::block("Dangerous operation");
        assert!(matches!(block, HookOutput::Sync(_)));

        let defer = hooks::defer(Some(5000));
        assert!(matches!(defer, HookOutput::Async(_)));
    }

    #[test]
    fn test_permission_helpers() {
        let allow = permissions::allow();
        assert!(matches!(allow, PermissionResult::Allow(_)));

        let deny = permissions::deny("Not allowed");
        assert!(matches!(deny, PermissionResult::Deny(_)));
    }
}
