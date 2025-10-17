//! Agent definition types.

use serde::{Deserialize, Serialize};

/// Custom agent definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentDefinition {
    /// Description of what the agent does.
    pub description: String,
    /// System prompt for the agent.
    pub prompt: String,
    /// Tools the agent can use (allowlist).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    /// Model override for this agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Settings source for loading configuration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum SettingSource {
    /// User-level settings.
    User,
    /// Project-level settings.
    Project,
    /// Local directory settings.
    Local,
}

impl std::fmt::Display for SettingSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingSource::User => write!(f, "user"),
            SettingSource::Project => write!(f, "project"),
            SettingSource::Local => write!(f, "local"),
        }
    }
}
