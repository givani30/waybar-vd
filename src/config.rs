//! # Configuration Module
//!
//! Provides configuration structures and utilities for the Waybar Virtual Desktops
//! CFFI module. Supports comprehensive customization of display format, behavior,
//! and performance parameters.
//!
//! # Configuration Options
//!
//! The module supports extensive configuration through the `ModuleConfig` struct:
//!
//! - **Display Format**: Customizable format strings with placeholders
//! - **Icon Mapping**: Per-desktop icon configuration
//! - **Visibility Control**: Show/hide empty virtual desktops
//! - **Performance Tuning**: Retry logic and backoff parameters
//! - **Sorting Options**: Multiple sorting strategies
//!
//! # Format Placeholders
//!
//! The following placeholders are supported in format strings:
//!
//! - `{name}`: Virtual desktop name
//! - `{icon}`: Mapped icon for the desktop
//! - `{id}`: Numeric desktop identifier
//! - `{window_count}`: Number of windows on the desktop
//!
//! # Example Configuration
//!
//! ```json
//! {
//!   "format": "{icon} {name}",
//!   "format_icons": {
//!     "1": "󰋇",
//!     "2": "󰍉"
//!   },
//!   "show_empty": false,
//!   "show_window_count": true,
//!   "separator": " ",
//!   "sort_by": "number",
//!   "retry_max": 10,
//!   "retry_base_delay_ms": 500
//! }
//! ```

// src/config.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration structure for the Virtual Desktops module
///
/// This struct defines all configurable aspects of the module behavior,
/// from display formatting to performance parameters. All fields have
/// sensible defaults and support serde deserialization.
///
/// # Field Descriptions
///
/// - `format`: Template string for desktop display (supports placeholders)
/// - `show_empty`: Whether to display virtual desktops with no windows
/// - `separator`: String used to separate multiple desktop elements
/// - `format_icons`: Mapping of desktop IDs/names to display icons
/// - `show_window_count`: Include window count in tooltip information
/// - `sort_by`: Sorting strategy ("number", "name", "focused-first")
/// - `retry_max`: Maximum IPC retry attempts before failure
/// - `retry_base_delay_ms`: Base delay for exponential backoff (milliseconds)
///
/// # Performance Tuning
///
/// The retry parameters allow fine-tuning of the IPC resilience:
/// - Higher `retry_max`: More persistent but slower failure detection
/// - Lower `retry_base_delay_ms`: Faster retries but higher CPU usage
///
/// # Examples
///
/// Basic configuration:
/// ```rust
/// use waybar_virtual_desktops_cffi::config::ModuleConfig;
///
/// let config = ModuleConfig {
///     format: "{icon} {name}".to_string(),
///     show_empty: false,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfig {
    /// Display format for virtual desktop names
    #[serde(default = "default_format")]
    pub format: String,

    /// Whether to show empty virtual desktops
    #[serde(default = "default_show_empty")]
    pub show_empty: bool,

    /// Separator between virtual desktop elements
    #[serde(default = "default_separator")]
    pub separator: String,

    /// Icon mapping for virtual desktop IDs
    #[serde(default)]
    pub format_icons: HashMap<String, String>,

    /// Show window count in tooltip
    #[serde(default = "default_show_window_count")]
    pub show_window_count: bool,

    /// Sort method: "number", "name", "focused-first"
    #[serde(default = "default_sort_by")]
    pub sort_by: String,

    /// Maximum number of retry attempts for IPC operations
    #[serde(default = "default_retry_max")]
    pub retry_max: u32,

    /// Base delay in milliseconds for exponential backoff
    #[serde(default = "default_retry_base_delay_ms")]
    pub retry_base_delay_ms: u64,
}

// Default functions for serde
fn default_format() -> String {
    "{name}".to_string()
}

fn default_show_empty() -> bool {
    false
}

fn default_separator() -> String {
    " ".to_string()
}

fn default_show_window_count() -> bool {
    false
}

fn default_sort_by() -> String {
    "number".to_string()
}

fn default_retry_max() -> u32 {
    10
}

fn default_retry_base_delay_ms() -> u64 {
    500
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self {
            format: default_format(),
            show_empty: default_show_empty(),
            separator: default_separator(),
            format_icons: HashMap::new(),
            show_window_count: default_show_window_count(),
            sort_by: default_sort_by(),
            retry_max: default_retry_max(),
            retry_base_delay_ms: default_retry_base_delay_ms(),
        }
    }
}

impl ModuleConfig {
    /// Format a virtual desktop name according to the configured format
    pub fn format_virtual_desktop(&self, name: &str, id: u32, window_count: u32) -> String {
        let icon = self.format_icons
            .get(&id.to_string())
            .or_else(|| self.format_icons.get(name))
            .map(|s| s.as_str())
            .unwrap_or("");
        
        self.format
            .replace("{name}", name)
            .replace("{icon}", icon)
            .replace("{id}", &id.to_string())
            .replace("{window_count}", &window_count.to_string())
    }
    
    /// Generate tooltip text for a virtual desktop
    pub fn format_tooltip(&self, name: &str, id: u32, window_count: u32, focused: bool) -> String {
        let mut tooltip = format!("Virtual Desktop {}: {}", id, name);
        
        if self.show_window_count {
            tooltip.push_str(&format!(" ({} windows)", window_count));
        }
        
        if focused {
            tooltip.push_str(" - focused");
        }
        
        tooltip
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_formatting() {
        let mut format_icons = HashMap::new();
        format_icons.insert("1".to_string(), "🏠".to_string());
        format_icons.insert("Work".to_string(), "💼".to_string());

        let config = ModuleConfig {
            format: "{icon} {name} ({window_count})".to_string(),
            show_empty: true,
            separator: " | ".to_string(),
            format_icons,
            show_window_count: true,
            sort_by: "number".to_string(),
            retry_max: 10,
            retry_base_delay_ms: 500,
        };

        // Test formatting with icon by ID
        let result = config.format_virtual_desktop("Home", 1, 3);
        assert_eq!(result, "🏠 Home (3)");

        // Test formatting with icon by name
        let result = config.format_virtual_desktop("Work", 2, 5);
        assert_eq!(result, "💼 Work (5)");

        // Test formatting without icon
        let result = config.format_virtual_desktop("Other", 3, 0);
        assert_eq!(result, " Other (0)");

        // Test tooltip formatting
        let tooltip = config.format_tooltip("Home", 1, 3, true);
        assert_eq!(tooltip, "Virtual Desktop 1: Home (3 windows) - focused");

        let tooltip = config.format_tooltip("Work", 2, 5, false);
        assert_eq!(tooltip, "Virtual Desktop 2: Work (5 windows)");
    }

    #[test]
    fn test_default_config() {
        let config = ModuleConfig::default();

        assert_eq!(config.format, "{name}");
        assert!(!config.show_empty);
        assert_eq!(config.separator, " ");
        assert!(config.format_icons.is_empty());
        assert!(!config.show_window_count);
        assert_eq!(config.sort_by, "number");
        assert_eq!(config.retry_max, 10);
        assert_eq!(config.retry_base_delay_ms, 500);
    }

    #[test]
    fn test_custom_retry_config() {
        let json = r#"{
            "format": "{name}",
            "show_empty": false,
            "separator": " ",
            "format_icons": {},
            "show_window_count": false,
            "sort_by": "number",
            "retry_max": 5,
            "retry_base_delay_ms": 1000
        }"#;

        let config: ModuleConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.retry_max, 5);
        assert_eq!(config.retry_base_delay_ms, 1000);
        // Other fields should be as specified
        assert_eq!(config.show_empty, false);
        assert_eq!(config.separator, " ");
    }

    #[test]
    fn test_direct_config_deserialization() {
        // Test direct format
        let direct_json = r#"{
            "format": "{icon} {name}",
            "show_empty": true,
            "separator": " | ",
            "format_icons": {"1": "🏠"},
            "show_window_count": true,
            "sort_by": "focused-first",
            "retry_max": 15,
            "retry_base_delay_ms": 750
        }"#;

        let config: ModuleConfig = serde_json::from_str(direct_json).unwrap();
        assert_eq!(config.format, "{icon} {name}");
        assert_eq!(config.show_empty, true);
        assert_eq!(config.separator, " | ");
        assert_eq!(config.format_icons.get("1"), Some(&"🏠".to_string()));
        assert_eq!(config.show_window_count, true);
        assert_eq!(config.sort_by, "focused-first");
        assert_eq!(config.retry_max, 15);
        assert_eq!(config.retry_base_delay_ms, 750);
    }

    #[test]
    fn test_config_with_defaults() {
        // Test minimal config with defaults
        let minimal_json = r#"{
            "format": "{name}"
        }"#;

        let config: ModuleConfig = serde_json::from_str(minimal_json).unwrap();
        assert_eq!(config.format, "{name}");
        assert_eq!(config.show_empty, false); // default
        assert_eq!(config.separator, " "); // default
        assert!(config.format_icons.is_empty()); // default
        assert_eq!(config.show_window_count, false); // default
        assert_eq!(config.sort_by, "number"); // default
        assert_eq!(config.retry_max, 10); // default
        assert_eq!(config.retry_base_delay_ms, 500); // default
    }
}
