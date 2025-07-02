// src/config.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfig {
    /// Display format for virtual desktop names
    pub format: String,
    
    /// Whether to show empty virtual desktops
    pub show_empty: bool,
    
    /// Separator between virtual desktop elements
    pub separator: String,
    
    /// Icon mapping for virtual desktop IDs
    pub format_icons: HashMap<String, String>,
    
    /// Show window count in tooltip
    pub show_window_count: bool,
    
    /// Sort method: "number", "name", "focused-first"
    pub sort_by: String,

    /// Maximum number of retry attempts for IPC operations
    pub retry_max: u32,

    /// Base delay in milliseconds for exponential backoff
    pub retry_base_delay_ms: u64,
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self {
            format: "{name}".to_string(),
            show_empty: false,
            separator: " ".to_string(),
            format_icons: HashMap::new(),
            show_window_count: false,
            sort_by: "number".to_string(),
            retry_max: 10,
            retry_base_delay_ms: 500,
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
}
