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
