//! Plugin configuration types.

use serde::{Deserialize, Serialize};

/// SDK plugin configuration.
///
/// Currently only local plugins are supported via the "local" type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SdkPluginConfig {
    /// Plugin type (currently only "local").
    #[serde(rename = "type")]
    pub plugin_type: String,
    /// Path to the plugin directory.
    pub path: String,
}
