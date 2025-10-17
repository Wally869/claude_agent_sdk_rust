//! Permission system types for tool control.

use serde::{Deserialize, Serialize};

/// Result of a permission check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PermissionResult {
    /// Permission granted.
    Allow(PermissionResultAllow),
    /// Permission denied.
    Deny(PermissionResultDeny),
}

/// Permission granted result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PermissionResultAllow {
    /// Behavior (always "allow").
    pub behavior: String, // "allow"
    /// Modified tool input (if changed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<serde_json::Value>,
    /// Permission updates to apply.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_permissions: Option<Vec<PermissionUpdate>>,
}

impl Default for PermissionResultAllow {
    fn default() -> Self {
        Self {
            behavior: "allow".to_string(),
            updated_input: None,
            updated_permissions: None,
        }
    }
}

/// Permission denied result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PermissionResultDeny {
    /// Behavior (always "deny").
    pub behavior: String, // "deny"
    /// Explanation message.
    pub message: String,
    /// Whether to interrupt the entire session.
    #[serde(default)]
    pub interrupt: bool,
}

impl Default for PermissionResultDeny {
    fn default() -> Self {
        Self {
            behavior: "deny".to_string(),
            message: String::new(),
            interrupt: false,
        }
    }
}

/// Context provided to permission callbacks.
#[derive(Debug, Clone, Default)]
pub struct ToolPermissionContext {
    /// Abort signal (future feature).
    pub signal: Option<()>,
    /// Permission suggestions from CLI.
    pub suggestions: Vec<PermissionUpdate>,
}

/// Permission update to apply.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionUpdate {
    /// Type of update.
    #[serde(rename = "type")]
    pub update_type: PermissionUpdateType,
    /// Rules to add/replace/remove (for rule operations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<PermissionRuleValue>>,
    /// Behavior for rules (allow/deny/ask).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior: Option<PermissionBehavior>,
    /// Mode to set (for setMode operation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// Directories to add/remove (for directory operations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directories: Option<Vec<String>>,
    /// Where to save the update.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<PermissionUpdateDestination>,
}

/// Type of permission update.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PermissionUpdateType {
    /// Add new rules.
    AddRules,
    /// Replace existing rules.
    ReplaceRules,
    /// Remove rules.
    RemoveRules,
    /// Set permission mode.
    SetMode,
    /// Add allowed directories.
    AddDirectories,
    /// Remove allowed directories.
    RemoveDirectories,
}

/// Permission rule value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRuleValue {
    /// Tool name the rule applies to.
    pub tool_name: String,
    /// Rule content/pattern (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_content: Option<String>,
}

/// Permission behavior for rules.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PermissionBehavior {
    /// Allow matching tools.
    Allow,
    /// Deny matching tools.
    Deny,
    /// Ask user/callback for permission.
    Ask,
}

/// Where to save permission updates.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PermissionUpdateDestination {
    /// User settings.
    UserSettings,
    /// Project settings.
    ProjectSettings,
    /// Local settings.
    LocalSettings,
    /// Session only (temporary).
    Session,
}
