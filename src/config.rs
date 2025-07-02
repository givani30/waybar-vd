//! Configuration for virtual desktop display and behavior

// src/config.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Virtual desktop sorting strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SortStrategy {
    Number,
    Name,
    #[serde(rename = "focused-first")]
    FocusedFirst,
}

impl Default for SortStrategy {
    fn default() -> Self {
        Self::Number
    }
}

impl std::fmt::Display for SortStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number => write!(f, "number"),
            Self::Name => write!(f, "name"),
            Self::FocusedFirst => write!(f, "focused-first"),
        }
    }
}

impl std::str::FromStr for SortStrategy {
    type Err = crate::errors::VirtualDesktopError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "number" => Ok(Self::Number),
            "name" => Ok(Self::Name),
            "focused-first" => Ok(Self::FocusedFirst),
            _ => Err(crate::errors::VirtualDesktopError::invalid_config(
                "sort_by",
                s,
                "must be 'number', 'name', or 'focused-first'"
            )),
        }
    }
}

/// Virtual desktop module configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfig {
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_show_empty")]
    pub show_empty: bool,
    #[serde(default = "default_separator")]
    pub separator: String,
    #[serde(default)]
    pub format_icons: HashMap<String, String>,
    #[serde(default = "default_show_window_count")]
    pub show_window_count: bool,
    #[serde(default)]
    pub sort_by: SortStrategy,
    #[serde(default = "default_retry_max")]
    pub retry_max: u32,
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
            sort_by: SortStrategy::default(),
            retry_max: default_retry_max(),
            retry_base_delay_ms: default_retry_base_delay_ms(),
        }
    }
}

impl ModuleConfig {
    /// Validate configuration parameters
    pub fn validate(&self) -> Result<(), crate::errors::VirtualDesktopError> {
        if !self.format.contains('{') {
            return Err(crate::errors::VirtualDesktopError::invalid_config(
                "format",
                &self.format,
                "must contain at least one placeholder like {name}, {icon}, {id}, or {window_count}"
            ));
        }

        if self.retry_max == 0 {
            return Err(crate::errors::VirtualDesktopError::invalid_config(
                "retry_max",
                &self.retry_max.to_string(),
                "must be greater than 0"
            ));
        }

        if self.retry_max > 50 {
            return Err(crate::errors::VirtualDesktopError::invalid_config(
                "retry_max",
                &self.retry_max.to_string(),
                "must be 50 or less to prevent excessive delays"
            ));
        }

        if self.retry_base_delay_ms > 10000 {
            return Err(crate::errors::VirtualDesktopError::invalid_config(
                "retry_base_delay_ms",
                &self.retry_base_delay_ms.to_string(),
                "must be 10000ms or less to prevent excessive delays"
            ));
        }

        Ok(())
    }

    /// Format virtual desktop display text
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
    
    /// Generate tooltip text
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
            sort_by: SortStrategy::Number,
            retry_max: 10,
            retry_base_delay_ms: 500,
        };

        let result = config.format_virtual_desktop("Home", 1, 3);
        assert_eq!(result, "🏠 Home (3)");

        let result = config.format_virtual_desktop("Work", 2, 5);
        assert_eq!(result, "💼 Work (5)");

        let result = config.format_virtual_desktop("Other", 3, 0);
        assert_eq!(result, " Other (0)");

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
        assert_eq!(config.sort_by, SortStrategy::Number);
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
        assert_eq!(config.show_empty, false);
        assert_eq!(config.separator, " ");
    }

    #[test]
    fn test_direct_config_deserialization() {
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
        assert_eq!(config.sort_by, SortStrategy::FocusedFirst);
        assert_eq!(config.retry_max, 15);
        assert_eq!(config.retry_base_delay_ms, 750);
    }

    #[test]
    fn test_config_with_defaults() {
        let minimal_json = r#"{
            "format": "{name}"
        }"#;

        let config: ModuleConfig = serde_json::from_str(minimal_json).unwrap();
        assert_eq!(config.format, "{name}");
        assert_eq!(config.show_empty, false);
        assert_eq!(config.separator, " ");
        assert!(config.format_icons.is_empty());
        assert_eq!(config.show_window_count, false);
        assert_eq!(config.sort_by, SortStrategy::Number);
        assert_eq!(config.retry_max, 10);
        assert_eq!(config.retry_base_delay_ms, 500);
    }

    #[test]
    fn test_config_validation() {
        let valid_config = ModuleConfig::default();
        assert!(valid_config.validate().is_ok());

        let invalid_format = ModuleConfig {
            format: "no placeholders".to_string(),
            ..Default::default()
        };
        assert!(invalid_format.validate().is_err());

        let invalid_retry = ModuleConfig {
            retry_max: 0,
            ..Default::default()
        };
        assert!(invalid_retry.validate().is_err());

        let invalid_retry_high = ModuleConfig {
            retry_max: 100,
            ..Default::default()
        };
        assert!(invalid_retry_high.validate().is_err());

        let invalid_delay = ModuleConfig {
            retry_base_delay_ms: 20000,
            ..Default::default()
        };
        assert!(invalid_delay.validate().is_err());
    }

    #[test]
    fn test_sort_strategy_parsing() {
        assert_eq!("number".parse::<SortStrategy>().unwrap(), SortStrategy::Number);
        assert_eq!("name".parse::<SortStrategy>().unwrap(), SortStrategy::Name);
        assert_eq!("focused-first".parse::<SortStrategy>().unwrap(), SortStrategy::FocusedFirst);

        assert!("invalid".parse::<SortStrategy>().is_err());

        assert_eq!(SortStrategy::Number.to_string(), "number");
        assert_eq!(SortStrategy::Name.to_string(), "name");
        assert_eq!(SortStrategy::FocusedFirst.to_string(), "focused-first");
    }
}
