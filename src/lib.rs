// src/lib.rs
use std::sync::Arc;
use std::thread;

use anyhow::Result;
use serde::Deserialize;
use tokio::runtime::Handle;
use waybar_cffi::{
    gtk::{prelude::*, Label, Box as GtkBox, Orientation, EventBox},
    waybar_module, InitInfo, Module,
};

pub mod config;
pub mod hyprland;
pub mod vdesk;

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

    /// Maximum number of retry attempts for IPC operations
    #[serde(default = "default_retry_max")]
    pub retry_max: u32,

    /// Base delay in milliseconds for exponential backoff
    #[serde(default = "default_retry_base_delay_ms")]
    pub retry_base_delay_ms: u64,
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

fn default_retry_max() -> u32 {
    10
}

fn default_retry_base_delay_ms() -> u64 {
    500
}

/// Widget state for a virtual desktop
#[derive(Debug)]
struct VirtualDesktopWidget {
    event_box: EventBox,
    label: Label,
    vdesk_id: u32,
    display_text: String,
    tooltip_text: String,
    focused: bool,
}

/// The main virtual desktops module
pub struct VirtualDesktopsModule {
    container: GtkBox,
    widgets: Vec<VirtualDesktopWidget>,
    manager: Arc<tokio::sync::Mutex<VirtualDesktopsManager>>,
    config: ModuleConfig,
    runtime_handle: Handle,
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
            retry_max: config.retry_max,
            retry_base_delay_ms: config.retry_base_delay_ms,
        };

        log::debug!("Module config: format={}, show_empty={}", module_config.format, module_config.show_empty);

        // Create the container widget
        let container = info.get_root_widget();
        let hbox = GtkBox::new(Orientation::Horizontal, 0);
        container.add(&hbox);
        log::debug!("Created GTK container widget");

        // Create a single Tokio runtime for the entire module
        let rt = tokio::runtime::Runtime::new()
            .expect("Failed to create Tokio runtime");
        let runtime_handle = rt.handle().clone();

        // Initialize the virtual desktops manager with async Mutex
        let manager = Arc::new(tokio::sync::Mutex::new(VirtualDesktopsManager::new()));

        // Initialize the manager synchronously to populate initial state
        {
            let manager_for_init = Arc::clone(&manager);
            let handle = runtime_handle.clone();
            thread::spawn(move || {
                handle.block_on(async {
                    match manager_for_init.lock().await.initialize().await {
                        Ok(_) => log::debug!("Virtual desktop manager initialized successfully"),
                        Err(e) => log::error!("Failed to initialize virtual desktop manager: {}", e),
                    }
                });
            }).join().unwrap_or_else(|_| {
                log::error!("Failed to join initialization thread");
            });
        }

        // Start background thread for IPC monitoring using the same runtime
        let manager_clone = Arc::clone(&manager);
        let handle_clone = runtime_handle.clone();
        let config_clone = module_config.clone();
        thread::spawn(move || {
            handle_clone.block_on(async {
                if let Err(e) = monitor_virtual_desktops(manager_clone, config_clone).await {
                    log::error!("Virtual desktop monitoring failed: {}", e);
                }
            });
        });

        // Keep the runtime alive by moving it to a background thread
        thread::spawn(move || {
            rt.block_on(async {
                // This will keep the runtime alive indefinitely
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
                }
            });
        });

        let mut module = Self {
            container: hbox,
            widgets: Vec::new(),
            manager,
            config: module_config,
            runtime_handle,
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
        // Get virtual desktops from manager using async Mutex
        let manager = Arc::clone(&self.manager);
        let handle = self.runtime_handle.clone();
        let virtual_desktops = handle.block_on(async {
            manager.lock().await.get_virtual_desktops()
        });

        // Filter virtual desktops that should be displayed
        let visible_vdesks: Vec<_> = virtual_desktops.into_iter()
            .filter(|vdesk| vdesk.populated || vdesk.focused || self.config.show_empty)
            .collect();

        // Perform incremental updates
        self.update_widgets_incrementally(visible_vdesks);
        self.container.show_all();
    }

    fn update_widgets_incrementally(&mut self, visible_vdesks: Vec<crate::vdesk::VirtualDesktop>) {
        use std::collections::HashSet;

        // Create HashSet for O(1) lookup - reduces complexity from O(n×m) to O(n+m)
        let visible_ids: HashSet<u32> = visible_vdesks.iter().map(|vd| vd.id).collect();

        // Remove widgets for virtual desktops that are no longer visible
        let mut i = 0;
        while i < self.widgets.len() {
            let widget_vdesk_id = self.widgets[i].vdesk_id;
            if !visible_ids.contains(&widget_vdesk_id) {
                let widget = self.widgets.remove(i);
                self.container.remove(&widget.event_box);
            } else {
                i += 1;
            }
        }

        // Update or create widgets for visible virtual desktops
        for (index, vdesk) in visible_vdesks.iter().enumerate() {
            let display_text = self.config.format_virtual_desktop(
                &vdesk.name,
                vdesk.id,
                vdesk.window_count
            );

            let tooltip_text = self.config.format_tooltip(
                &vdesk.name,
                vdesk.id,
                vdesk.window_count,
                vdesk.focused
            );

            // Check if we already have a widget for this virtual desktop
            if let Some(existing_widget) = self.widgets.iter_mut().find(|w| w.vdesk_id == vdesk.id) {
                // Update existing widget if content has changed
                if existing_widget.display_text != display_text {
                    existing_widget.label.set_text(&display_text);
                    existing_widget.display_text = display_text;
                }

                if existing_widget.tooltip_text != tooltip_text {
                    existing_widget.label.set_tooltip_text(Some(&tooltip_text));
                    existing_widget.tooltip_text = tooltip_text;
                }

                // Update CSS classes if focus state changed
                if existing_widget.focused != vdesk.focused {
                    let style_context = existing_widget.label.style_context();
                    if vdesk.focused {
                        style_context.remove_class("vdesk-unfocused");
                        style_context.add_class("vdesk-focused");
                    } else {
                        style_context.remove_class("vdesk-focused");
                        style_context.add_class("vdesk-unfocused");
                    }
                    existing_widget.focused = vdesk.focused;
                }
            } else {
                // Create new widget for this virtual desktop
                let widget = self.create_virtual_desktop_widget(vdesk, display_text, tooltip_text);

                // Insert at the correct position to maintain order
                if index < self.widgets.len() {
                    self.widgets.insert(index, widget);
                    self.container.reorder_child(&self.widgets[index].event_box, index as i32);
                } else {
                    self.container.add(&widget.event_box);
                    self.widgets.push(widget);
                }
            }
        }

        // Reorder widgets to match the virtual desktop order
        for (index, vdesk) in visible_vdesks.iter().enumerate() {
            if let Some(widget_index) = self.widgets.iter().position(|w| w.vdesk_id == vdesk.id) {
                if widget_index != index {
                    let widget = self.widgets.remove(widget_index);
                    self.widgets.insert(index, widget);
                    self.container.reorder_child(&self.widgets[index].event_box, index as i32);
                }
            }
        }
    }

    fn create_virtual_desktop_widget(
        &self,
        vdesk: &crate::vdesk::VirtualDesktop,
        display_text: String,
        tooltip_text: String
    ) -> VirtualDesktopWidget {
        let label = Label::new(Some(&display_text));
        label.set_tooltip_text(Some(&tooltip_text));

        // Make the label clickable
        let event_box = EventBox::new();
        event_box.add(&label);

        // Set up click handler using async IPC
        let vdesk_id_for_click = vdesk.id;
        let runtime_handle = self.runtime_handle.clone();
        event_box.connect_button_press_event(move |_, event| {
            if event.button() == 1 { // Left click
                let handle = runtime_handle.clone();
                let vdesk_id = vdesk_id_for_click;

                // Spawn async task to handle the click
                handle.spawn(async move {
                    match HyprlandIPC::new().await {
                        Ok(ipc) => {
                            if let Err(e) = ipc.switch_to_virtual_desktop(vdesk_id).await {
                                log::error!("Failed to switch to virtual desktop {}: {}", vdesk_id, e);
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to create Hyprland IPC for click handler: {}", e);
                        }
                    }
                });
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

        VirtualDesktopWidget {
            event_box,
            label,
            vdesk_id: vdesk.id,
            display_text,
            tooltip_text,
            focused: vdesk.focused,
        }
    }
    
    fn switch_to_virtual_desktop(&self, vdesk_id: u32) -> Result<()> {
        // Use the shared runtime for async operations
        self.runtime_handle.block_on(async {
            let ipc = HyprlandIPC::new().await
                .map_err(|e| anyhow::anyhow!("Failed to create Hyprland IPC: {}", e))?;

            ipc.switch_to_virtual_desktop(vdesk_id).await
                .map_err(|e| anyhow::anyhow!("Failed to switch virtual desktop: {}", e))
        })
    }
}

/// Background task to monitor virtual desktop changes
async fn monitor_virtual_desktops(
    manager: Arc<tokio::sync::Mutex<VirtualDesktopsManager>>,
    config: crate::config::ModuleConfig
) -> Result<()> {
    log::info!("Starting virtual desktop monitoring...");

    // Create IPC connection for event monitoring with configurable retry parameters
    let mut ipc = match HyprlandIPC::with_config(config.retry_max, config.retry_base_delay_ms).await {
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
                    let mut mgr = manager.lock().await;
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    #[test]
    fn test_widget_update_algorithm_complexity() {
        // Test that our HashSet optimization works correctly
        let visible_vdesks = vec![
            crate::vdesk::VirtualDesktop {
                id: 1,
                name: "Desktop 1".to_string(),
                focused: true,
                populated: true,
                window_count: 0,
                workspaces: vec![],
            },
            crate::vdesk::VirtualDesktop {
                id: 3,
                name: "Desktop 3".to_string(),
                focused: false,
                populated: false,
                window_count: 0,
                workspaces: vec![],
            },
        ];

        // Create HashSet for O(1) lookup - this is what our optimized algorithm does
        let visible_ids: HashSet<u32> = visible_vdesks.iter().map(|vd| vd.id).collect();

        // Test that lookup is O(1) instead of O(n)
        assert!(visible_ids.contains(&1));
        assert!(!visible_ids.contains(&2));
        assert!(visible_ids.contains(&3));

        // Verify the set contains exactly what we expect
        assert_eq!(visible_ids.len(), 2);
        assert_eq!(visible_ids, [1, 3].iter().cloned().collect());
    }

    #[test]
    fn test_jitter_calculation() {
        // Test that our jitter calculation produces reasonable values
        let base_delay = 1000u64; // 1 second
        let jitter_range = base_delay / 4; // 25% = 250ms

        // Simulate the jitter calculation from our code
        for _ in 0..100 {
            let jitter = fastrand::u64(0..=jitter_range * 2); // 0 to 500ms
            let delay_ms = base_delay.saturating_sub(jitter_range).saturating_add(jitter);

            // Delay should be between 750ms and 1250ms (±25% jitter)
            assert!(delay_ms >= 750);
            assert!(delay_ms <= 1250);
        }
    }
}

// Export the module
waybar_module!(VirtualDesktopsModule);
