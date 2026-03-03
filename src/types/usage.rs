//! Usage tracking and quota monitoring for Claude Code Max Plan users.

use serde::{Deserialize, Serialize};

/// Represents a usage limit period with utilization percentage and reset time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageLimit {
    /// Percentage of limit used (0-100)
    pub utilization: f64,

    /// ISO 8601 timestamp when the limit resets, or None if not applicable
    pub resets_at: Option<String>,
}

/// Usage data for Claude Code (Max Plan OAuth users)
///
/// This represents the current usage across different time windows and contexts.
/// All utilization values are percentages from 0-100.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    /// Current 5-hour rolling window usage
    pub five_hour: UsageLimit,

    /// Weekly usage across all models
    pub seven_day: UsageLimit,

    /// OAuth app-specific weekly usage (typically 0 for CLI)
    pub seven_day_oauth_apps: UsageLimit,

    /// Weekly Opus-specific usage
    pub seven_day_opus: UsageLimit,
}

impl UsageData {
    /// Returns the highest utilization percentage across all limits
    pub fn max_utilization(&self) -> f64 {
        [
            self.five_hour.utilization,
            self.seven_day.utilization,
            self.seven_day_oauth_apps.utilization,
            self.seven_day_opus.utilization,
        ]
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max)
    }

    /// Returns true if any usage limit is above the given threshold (0-100)
    pub fn is_above_threshold(&self, threshold: f64) -> bool {
        self.max_utilization() > threshold
    }

    /// Returns true if approaching any usage limit (>= 80%)
    pub fn is_approaching_limit(&self) -> bool {
        self.is_above_threshold(80.0)
    }

    /// Returns true if at or very close to any usage limit (>= 95%)
    pub fn is_at_limit(&self) -> bool {
        self.is_above_threshold(95.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_usage(five_hour: f64, seven_day: f64, opus: f64) -> UsageData {
        UsageData {
            five_hour: UsageLimit {
                utilization: five_hour,
                resets_at: Some("2025-10-20T02:59:59Z".to_string()),
            },
            seven_day: UsageLimit {
                utilization: seven_day,
                resets_at: Some("2025-10-23T21:59:59Z".to_string()),
            },
            seven_day_oauth_apps: UsageLimit {
                utilization: 0.0,
                resets_at: None,
            },
            seven_day_opus: UsageLimit {
                utilization: opus,
                resets_at: Some("2025-10-23T21:59:59Z".to_string()),
            },
        }
    }

    #[test]
    fn test_max_utilization() {
        let usage = create_test_usage(25.0, 50.0, 75.0);
        assert_eq!(usage.max_utilization(), 75.0);
    }

    #[test]
    fn test_is_above_threshold() {
        let usage = create_test_usage(25.0, 50.0, 75.0);
        assert!(usage.is_above_threshold(70.0));
        assert!(!usage.is_above_threshold(80.0));
    }

    #[test]
    fn test_is_approaching_limit() {
        let usage_safe = create_test_usage(25.0, 50.0, 75.0);
        assert!(!usage_safe.is_approaching_limit());

        let usage_approaching = create_test_usage(25.0, 85.0, 75.0);
        assert!(usage_approaching.is_approaching_limit());
    }

    #[test]
    fn test_is_at_limit() {
        let usage_safe = create_test_usage(25.0, 85.0, 75.0);
        assert!(!usage_safe.is_at_limit());

        let usage_at_limit = create_test_usage(25.0, 97.0, 75.0);
        assert!(usage_at_limit.is_at_limit());
    }
}
