// src/lib.rs
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::Result;
use serde::Deserialize;
use waybar_cffi::{
    gtk::{self, prelude::*, Label, Box as GtkBox, Orientation, EventBox},
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
        // Initialize logging (only if not already initialized)
        let _ = env_logger::try_init();

        log::info!("Virtual Desktops CFFI module initializing...");

        // Convert waybar config to internal config
        let module_config = ModuleConfig {
            format: config.format,
            show_empty: config.show_empty,
            separator: config.separator,
            format_icons: config.format_icons,
            show_window_count: config.show_window_count,
            sort_by: config.sort_by,
        };

        log::debug!("Module config: format={}, show_empty={}", module_config.format, module_config.show_empty);

        // Create the container widget
        let container = info.get_root_widget();
        let hbox = GtkBox::new(Orientation::Horizontal, 0);
        container.add(&hbox);
        log::debug!("Created GTK container widget");

        // Initialize the virtual desktops manager
        let manager = Arc::new(Mutex::new(VirtualDesktopsManager::new()));

        // Initialize the manager synchronously to populate initial state
        {
            let manager_for_init = Arc::clone(&manager);
            thread::spawn(move || {
                match tokio::runtime::Runtime::new() {
                    Ok(rt) => {
                        rt.block_on(async {
                            let mut mgr = manager_for_init.lock().unwrap();
                            if let Err(e) = mgr.initialize().await {
                                log::error!("Failed to initialize virtual desktop manager: {}", e);
                            } else {
                                log::debug!("Virtual desktop manager initialized successfully");
                            }
                        });
                    }
                    Err(e) => {
                        log::error!("Failed to create tokio runtime for initialization: {}", e);
                    }
                }
            }).join().unwrap_or_else(|_| {
                log::error!("Failed to join initialization thread");
            });
        }

        // Start background thread for IPC monitoring
        let manager_clone = Arc::clone(&manager);
        thread::spawn(move || {
            match tokio::runtime::Runtime::new() {
                Ok(rt) => {
                    rt.block_on(async {
                        if let Err(e) = monitor_virtual_desktops(manager_clone).await {
                            log::error!("Virtual desktop monitoring failed: {}", e);
                        }
                    });
                }
                Err(e) => {
                    log::error!("Failed to create tokio runtime: {}", e);
                }
            }
        });

        let mut module = Self {
            container: hbox,
            labels: Vec::new(),
            manager,
            config: module_config,
        };

        // Initial update - now the manager should have data
        log::debug!("Performing initial update");
        module.update();

        log::info!("Virtual Desktops CFFI module initialized successfully");
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
        // Clear existing labels and their containers
        let children: Vec<gtk::Widget> = self.container.children();
        for child in children {
            self.container.remove(&child);
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

            // Make the label clickable
            let event_box = EventBox::new();
            event_box.add(&label);

            // Set up click handler
            let vdesk_id_for_click = vdesk.id;
            event_box.connect_button_press_event(move |_, event| {
                if event.button() == 1 { // Left click
                    let _ = std::process::Command::new("hyprctl")
                        .args(&["dispatch", "vdesk", &vdesk_id_for_click.to_string()])
                        .output();
                }
                false.into()
            });

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

            self.container.add(&event_box);
            self.labels.push(label);
        }

        self.container.show_all();
    }
    
    fn switch_to_virtual_desktop(&self, vdesk_id: u32) -> Result<()> {
        // Use direct socket communication to switch to the virtual desktop
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create tokio runtime: {}", e))?;

        rt.block_on(async {
            let ipc = HyprlandIPC::new().await
                .map_err(|e| anyhow::anyhow!("Failed to create Hyprland IPC: {}", e))?;

            ipc.switch_to_virtual_desktop(vdesk_id).await
                .map_err(|e| anyhow::anyhow!("Failed to switch virtual desktop: {}", e))
        })
    }
}

/// Background task to monitor virtual desktop changes
async fn monitor_virtual_desktops(manager: Arc<Mutex<VirtualDesktopsManager>>) -> Result<()> {
    log::info!("Starting virtual desktop monitoring...");

    // Create IPC connection for event monitoring
    let mut ipc = match HyprlandIPC::new().await {
        Ok(ipc) => {
            log::info!("Successfully connected to Hyprland IPC for monitoring");
            ipc
        }
        Err(e) => {
            log::error!("Failed to connect to Hyprland IPC: {}", e);
            return Err(e);
        }
    };

    // Listen for events in a loop
    loop {
        match ipc.listen_for_events().await {
            Ok(event) => {
                if event.starts_with("vdesk>>") {
                    log::debug!("Received vdesk event: {}", event);
                    let mut mgr = manager.lock().unwrap();
                    if let Err(e) = mgr.update_state().await {
                        log::error!("Failed to update virtual desktop state: {}", e);
                    } else {
                        log::debug!("Virtual desktop state updated successfully");
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
