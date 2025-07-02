use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::Result;
use serde::Deserialize;
use waybar_cffi::{
    gtk::{prelude::*, Label, Box as GtkBox, Orientation},
    waybar_module, InitInfo, Module,
};

mod config;
mod hyprland;
mod vdesk;

use config::ModuleConfig;
use hyprland::HyprlandIPC;
use vdesk::VirtualDesktopsManager;

/// Configuration for the virtual desktops module
#[derive(Deserialize)]
pub struct Config {
    /// Format string for virtual desktop display
    #[serde(default = "default_format")]
    pub format: String,

    /// Whether to show empty virtual desktops
    #[serde(default = "default_show_empty")]
    pub show_empty: bool,

    /// Separator between virtual desktops
    #[serde(default = "default_separator")]
    pub separator: String,

    /// Icon mapping for virtual desktop IDs
    #[serde(default)]
    pub format_icons: std::collections::HashMap<String, String>,

    /// Show window count in tooltip
    #[serde(default = "default_show_window_count")]
    pub show_window_count: bool,

    /// Sort method: "number", "name", "focused-first"
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
}

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

/// The main virtual desktops module
pub struct VirtualDesktopsModule {
    container: GtkBox,
    labels: Vec<Label>,
    manager: Arc<Mutex<VirtualDesktopsManager>>,
    config: ModuleConfig,
}

impl Module for VirtualDesktopsModule {
    type Config = Config;

    fn init(info: &InitInfo, config: Self::Config) -> Self {
        // Convert waybar config to internal config
        let module_config = ModuleConfig {
            format: config.format,
            show_empty: config.show_empty,
            separator: config.separator,
            format_icons: config.format_icons,
            show_window_count: config.show_window_count,
            sort_by: config.sort_by,
        };

        // Create the container widget
        let container = info.get_root_widget();
        let hbox = GtkBox::new(Orientation::Horizontal, 0);
        container.add(&hbox);

        // Initialize the virtual desktops manager
        let manager = Arc::new(Mutex::new(VirtualDesktopsManager::new()));

        // Start background thread for IPC monitoring
        let manager_clone = Arc::clone(&manager);
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Err(e) = monitor_virtual_desktops(manager_clone).await {
                    log::error!("Virtual desktop monitoring failed: {}", e);
                }
            });
        });

        let mut module = Self {
            container: hbox,
            labels: Vec::new(),
            manager,
            config: module_config,
        };

        // Initial update
        module.update();

        module
    }

    fn update(&mut self) {
        self.update_display();
    }

    fn refresh(&mut self, _signal: i32) {
        self.update();
    }

    fn do_action(&mut self, action: &str) {
        if let Ok(vdesk_id) = action.parse::<u32>() {
            if let Err(e) = self.switch_to_virtual_desktop(vdesk_id) {
                log::error!("Failed to switch to virtual desktop {}: {}", vdesk_id, e);
            }
        }
    }
}

impl VirtualDesktopsModule {
    fn update_display(&mut self) {
        // Clear existing labels
        for label in &self.labels {
            self.container.remove(label);
        }
        self.labels.clear();
        
        // Get virtual desktops from manager
        let manager = self.manager.lock().unwrap();
        let virtual_desktops = manager.get_virtual_desktops();
        
        // Create new labels for each virtual desktop
        for vdesk in virtual_desktops {
            if !vdesk.populated && !vdesk.focused && !self.config.show_empty {
                continue;
            }
            
            // Format the virtual desktop display text using config
            let display_text = self.config.format_virtual_desktop(
                &vdesk.name, 
                vdesk.id, 
                vdesk.window_count
            );
            
            let label = Label::new(Some(&display_text));
            
            // Set tooltip with detailed information
            let tooltip_text = self.config.format_tooltip(
                &vdesk.name,
                vdesk.id,
                vdesk.window_count,
                vdesk.focused
            );
            label.set_tooltip_text(Some(&tooltip_text));
            
            // Apply CSS classes based on state
            let style_context = label.style_context();
            if vdesk.focused {
                style_context.add_class("vdesk-focused");
            } else {
                style_context.add_class("vdesk-unfocused");
            }
            
            if !vdesk.populated && !self.config.show_empty {
                style_context.add_class("hidden");
            }
            
            self.container.add(&label);
            self.labels.push(label);
        }

        self.container.show_all();
    }
    
    fn switch_to_virtual_desktop(&self, vdesk_id: u32) -> Result<()> {
        // Use hyprctl to switch to the virtual desktop
        std::process::Command::new("hyprctl")
            .args(&["dispatch", "vdesk", &vdesk_id.to_string()])
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to switch virtual desktop: {}", e))?;
        
        Ok(())
    }
}

/// Background task to monitor virtual desktop changes
async fn monitor_virtual_desktops(manager: Arc<Mutex<VirtualDesktopsManager>>) -> Result<()> {
    // Initial state update
    {
        let mut mgr = manager.lock().unwrap();
        if let Err(e) = mgr.initialize().await {
            log::error!("Failed to initialize virtual desktop manager: {}", e);
        }
    }

    // Create a separate IPC connection for event monitoring
    let mut ipc = HyprlandIPC::new().await?;

    // Listen for events in a loop
    loop {
        match ipc.listen_for_events().await {
            Ok(event) => {
                if event.starts_with("vdesk>>") {
                    let mut mgr = manager.lock().unwrap();
                    if let Err(e) = mgr.update_state().await {
                        log::error!("Failed to update virtual desktop state: {}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("Error listening for events: {}", e);
                // Wait a bit before retrying
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }
}

// Export the module
waybar_module!(VirtualDesktopsModule);
