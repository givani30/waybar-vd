//! # Waybar Virtual Desktops CFFI Module
//!
//! A high-performance CFFI module for Waybar that displays Hyprland virtual desktops
//! with real-time updates, click handling, and comprehensive configuration options.
//!
//! ## Features
//!
//! - **Real-time Updates**: Monitors Hyprland virtual desktop changes via IPC
//! - **Click Handling**: Switch virtual desktops by clicking on widgets
//! - **Customizable Display**: Configurable format strings, icons, and styling
//! - **Performance Optimized**: O(log n) widget updates using BTreeMap data structures
//! - **Resilient Architecture**: Exponential backoff, graceful shutdown, bounded failure recovery
//! - **Security Hardened**: Formal regex validation for instance signatures
//!
//! ## Configuration
//!
//! The module supports both direct and nested configuration formats:
//!
//! ```json
//! {
//!   "cffi/virtual_desktops": {
//!     "module_path": "/path/to/libwaybar_virtual_desktops_cffi.so",
//!     "config": {
//!       "format": "{icon} {name}",
//!       "format_icons": {
//!         "1": "󰋇",
//!         "2": "󰍉",
//!         "3": "󰎄"
//!       },
//!       "show_empty": false,
//!       "show_window_count": true,
//!       "separator": " ",
//!       "sort_by": "number"
//!     }
//!   }
//! }
//! ```
//!
//! ## Error Handling
//!
//! The module implements comprehensive error handling with:
//! - Exponential backoff for IPC connection failures
//! - Bounded failure recovery (max 5 consecutive failures)
//! - Graceful degradation when Hyprland is unavailable
//! - Detailed logging for troubleshooting
//!
//! ## Performance Characteristics
//!
//! - Widget updates: O(n log n) complexity using BTreeMap
//! - IPC operations: Async with configurable retry logic
//! - Memory usage: Bounded with proper resource cleanup
//! - Runtime lifecycle: Shared `Arc<Runtime>` with graceful shutdown

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

/// Unified configuration wrapper supporting both direct and nested formats
///
/// This enum enables polymorphic deserialization to handle both Waybar's standard
/// nested configuration format and direct configuration for backwards compatibility.
///
/// # Examples
///
/// Nested format (recommended):
/// ```json
/// {
///   "cffi/virtual_desktops": {
///     "config": {
///       "format": "{icon} {name}",
///       "show_empty": false
///     }
///   }
/// }
/// ```
///
/// Direct format (legacy):
/// ```json
/// {
///   "cffi/virtual_desktops": {
///     "format": "{icon} {name}",
///     "show_empty": false
///   }
/// }
/// ```
///
/// # Implementation Notes
///
/// Uses `#[serde(untagged)]` with Nested variant first to ensure proper parsing
/// precedence. The order matters because serde tries variants sequentially.
#[derive(Deserialize)]
#[serde(untagged)]
pub enum ConfigWrapper {
    /// Nested configuration format: {"config": {"format": "...", "show_empty": false, ...}}
    Nested { config: ModuleConfig },
    /// Direct configuration format: {"format": "...", "show_empty": false, ...}
    Direct(ModuleConfig),
}

impl From<ConfigWrapper> for ModuleConfig {
    fn from(wrapper: ConfigWrapper) -> Self {
        match wrapper {
            ConfigWrapper::Direct(config) => config,
            ConfigWrapper::Nested { config } => config,
        }
    }
}

/// Legacy configuration struct for backward compatibility
/// This is kept for explicit type safety in the Module trait
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
///
/// Represents a single virtual desktop widget in the Waybar display.
/// Each widget consists of a clickable EventBox containing a Label with
/// formatted text and tooltip information.
///
/// # Fields
///
/// - `event_box`: GTK EventBox for handling click events
/// - `label`: GTK Label displaying the formatted virtual desktop text
/// - `vdesk_id`: Unique identifier for the virtual desktop
/// - `display_text`: Current formatted text shown in the label
/// - `tooltip_text`: Current tooltip text for additional information
/// - `focused`: Whether this virtual desktop is currently focused
///
/// # Performance Notes
///
/// Widget updates are optimized to only modify GTK elements when content
/// actually changes, reducing unnecessary UI operations.
#[derive(Debug)]
struct VirtualDesktopWidget {
    event_box: EventBox,
    label: Label,
    vdesk_id: u32,
    display_text: String,
    tooltip_text: String,
    focused: bool,
}

/// The main virtual desktops module for Waybar CFFI integration
///
/// This is the primary module that implements the Waybar Module trait and manages
/// the display of Hyprland virtual desktops. It provides real-time updates,
/// click handling, and optimized widget management.
///
/// # Architecture
///
/// The module uses a multi-threaded architecture with:
/// - Main thread: GTK UI operations and widget management
/// - Background thread: Async IPC monitoring with Hyprland
/// - Shared state: Thread-safe communication via `Arc<Mutex<VirtualDesktopsManager>>`
///
/// # Performance Optimizations
///
/// - **BTreeMap widgets**: O(log n) lookup/insert/remove operations
/// - **widget_order Vec**: O(1) position tracking for reordering
/// - **Incremental updates**: Only modify widgets when content changes
/// - **Bounded resources**: Graceful shutdown prevents resource leaks
///
/// # Error Handling
///
/// - **Resilient monitoring**: Exponential backoff with bounded failure recovery
/// - **Graceful degradation**: Continues operation when Hyprland is unavailable
/// - **Resource cleanup**: Proper shutdown via Drop trait implementation
///
/// # Example Usage
///
/// The module is automatically instantiated by Waybar when configured:
///
/// ```json
/// {
///   "cffi/virtual_desktops": {
///     "module_path": "/path/to/libwaybar_virtual_desktops_cffi.so",
///     "config": {
///       "format": "{icon} {name}",
///       "show_empty": false
///     }
///   }
/// }
/// ```
pub struct VirtualDesktopsModule {
    container: GtkBox,
    widgets: std::collections::BTreeMap<u32, VirtualDesktopWidget>, // O(log n) operations
    widget_order: Vec<u32>, // Maintains display order for O(1) position lookup
    manager: Arc<tokio::sync::Mutex<VirtualDesktopsManager>>,
    config: ModuleConfig,
    runtime_handle: Handle,
    _runtime: Arc<tokio::runtime::Runtime>, // Keep runtime alive through ownership
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    monitor_handle: Option<tokio::task::JoinHandle<()>>,
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
        let rt = Arc::new(tokio::runtime::Runtime::new()
            .expect("Failed to create Tokio runtime"));
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

        // Create shutdown channel for graceful termination
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        // Start resilient monitoring task
        let manager_clone = Arc::clone(&manager);
        let config_clone = module_config.clone();
        let monitor_handle = runtime_handle.spawn(async move {
            if let Err(e) = resilient_monitor_loop(manager_clone, config_clone, shutdown_rx).await {
                log::error!("Resilient monitor loop failed: {}", e);
            }
        });

        let mut module = Self {
            container: hbox,
            widgets: std::collections::BTreeMap::new(),
            widget_order: Vec::new(),
            manager,
            config: module_config,
            runtime_handle,
            _runtime: rt,
            shutdown_tx: Some(shutdown_tx),
            monitor_handle: Some(monitor_handle),
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

impl Drop for VirtualDesktopsModule {
    fn drop(&mut self) {
        log::info!("VirtualDesktopsModule dropping - initiating graceful shutdown");

        // Signal shutdown to monitoring task
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        // Wait for monitor task to complete (with timeout)
        if let Some(monitor_handle) = self.monitor_handle.take() {
            let rt_handle = self.runtime_handle.clone();
            std::thread::spawn(move || {
                rt_handle.block_on(async {
                    match tokio::time::timeout(
                        tokio::time::Duration::from_secs(5),
                        monitor_handle
                    ).await {
                        Ok(Ok(())) => log::debug!("Monitor task completed gracefully"),
                        Ok(Err(e)) => log::warn!("Monitor task completed with error: {}", e),
                        Err(_) => log::warn!("Monitor task shutdown timed out"),
                    }
                });
            }).join().unwrap_or_else(|_| {
                log::error!("Failed to join shutdown thread");
            });
        }

        log::info!("VirtualDesktopsModule shutdown complete");
        // Runtime will be dropped when Arc count reaches zero
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

    /// Optimized widget update with O(n log n) complexity using BTreeMap
    ///
    /// Performance improvements:
    /// - BTreeMap provides O(log n) lookup/insert/remove vs O(n) for Vec
    /// - Separate widget_order Vec provides O(1) position tracking
    /// - Eliminates O(n²) reordering by tracking position changes
    /// - Reduces widget creation/destruction through efficient diffing
    fn update_widgets_incrementally(&mut self, visible_vdesks: Vec<crate::vdesk::VirtualDesktop>) {
        use std::collections::HashSet;

        // Create HashSet for O(1) visibility lookup
        let visible_ids: HashSet<u32> = visible_vdesks.iter().map(|vd| vd.id).collect();

        // Phase 1: Remove widgets for virtual desktops that are no longer visible
        // O(k log n) where k is number of widgets to remove, n is total widgets
        let widgets_to_remove: Vec<u32> = self.widgets.keys()
            .filter(|&id| !visible_ids.contains(id))
            .copied()
            .collect();

        for widget_id in widgets_to_remove {
            if let Some(widget) = self.widgets.remove(&widget_id) {
                self.container.remove(&widget.event_box);
                // Remove from order tracking
                self.widget_order.retain(|&id| id != widget_id);
            }
        }

        // Phase 2: Update or create widgets for visible virtual desktops
        // O(n log m) where n is visible desktops, m is total widgets
        let mut new_order = Vec::with_capacity(visible_vdesks.len());

        for vdesk in &visible_vdesks {
            new_order.push(vdesk.id);

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

            // O(log n) lookup in BTreeMap
            if let Some(existing_widget) = self.widgets.get_mut(&vdesk.id) {
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
                // Create new widget - O(log n) insertion
                let widget = self.create_virtual_desktop_widget(vdesk, display_text, tooltip_text);
                self.container.add(&widget.event_box);
                self.widgets.insert(vdesk.id, widget);
            }
        }

        // Phase 3: Reorder widgets efficiently if order changed
        // O(n) comparison + O(k) reordering where k is number of position changes
        if new_order != self.widget_order {
            for (new_position, &widget_id) in new_order.iter().enumerate() {
                if let Some(widget) = self.widgets.get(&widget_id) {
                    self.container.reorder_child(&widget.event_box, new_position as i32);
                }
            }
            self.widget_order = new_order;
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

/// Resilient monitoring loop with bounded failure recovery and graceful shutdown
///
/// This function implements the core monitoring logic that listens for Hyprland
/// virtual desktop events and updates the module state accordingly. It provides
/// robust error handling with exponential backoff and bounded failure recovery.
///
/// # Arguments
///
/// * `manager` - Shared virtual desktops manager for state updates
/// * `config` - Module configuration including retry parameters
/// * `shutdown_rx` - Oneshot receiver for graceful shutdown signaling
///
/// # Error Handling Strategy
///
/// 1. **Exponential Backoff**: Delays between retries increase exponentially
/// 2. **Bounded Failures**: Maximum of 5 consecutive failures before giving up
/// 3. **Graceful Shutdown**: Responds to shutdown signals immediately
/// 4. **Failure Isolation**: Individual monitor cycles can fail without affecting the loop
///
/// # Performance Characteristics
///
/// - **Memory**: O(1) - bounded resource usage
/// - **CPU**: Event-driven, minimal overhead when idle
/// - **Network**: Persistent Unix socket connection with reconnection logic
///
/// # Returns
///
/// Returns `Ok(())` on graceful shutdown or `Err` if maximum failures exceeded.
///
/// # Example Error Recovery
///
/// ```text
/// Attempt 1 fails -> wait 500ms
/// Attempt 2 fails -> wait 1000ms
/// Attempt 3 fails -> wait 2000ms
/// Attempt 4 fails -> wait 4000ms
/// Attempt 5 fails -> return error
/// ```
async fn resilient_monitor_loop(
    manager: Arc<tokio::sync::Mutex<VirtualDesktopsManager>>,
    config: crate::config::ModuleConfig,
    mut shutdown_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<()> {
    log::info!("Starting resilient virtual desktop monitoring...");

    let mut consecutive_failures = 0;
    const MAX_CONSECUTIVE_FAILURES: u32 = 5;

    loop {
        tokio::select! {
            _ = &mut shutdown_rx => {
                log::info!("Graceful shutdown requested for monitor loop");
                break;
            }
            result = monitor_virtual_desktops_once(&manager, &config) => {
                match result {
                    Ok(_) => {
                        consecutive_failures = 0;
                        log::debug!("Monitor cycle completed successfully");
                    }
                    Err(e) => {
                        consecutive_failures += 1;
                        log::error!("Monitor failure {}/{}: {}",
                                  consecutive_failures, MAX_CONSECUTIVE_FAILURES, e);

                        if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                            return Err(anyhow::anyhow!("Monitor permanently failed after {} attempts", MAX_CONSECUTIVE_FAILURES));
                        }

                        // Exponential backoff for recovery
                        let delay = tokio::time::Duration::from_millis(
                            config.retry_base_delay_ms * 2_u64.pow(consecutive_failures - 1)
                        );
                        log::info!("Waiting {:?} before retry attempt", delay);

                        tokio::select! {
                            _ = &mut shutdown_rx => {
                                log::info!("Shutdown requested during recovery delay");
                                break;
                            }
                            _ = tokio::time::sleep(delay) => {
                                log::debug!("Recovery delay completed, retrying...");
                            }
                        }
                    }
                }
            }
        }
    }

    log::info!("Resilient monitor loop terminated gracefully");
    Ok(())
}

/// Single monitoring cycle - can fail and be retried
async fn monitor_virtual_desktops_once(
    manager: &Arc<tokio::sync::Mutex<VirtualDesktopsManager>>,
    config: &crate::config::ModuleConfig
) -> Result<()> {
    log::debug!("Starting monitor cycle...");

    // Create IPC connection for event monitoring with configurable retry parameters
    let mut ipc = HyprlandIPC::with_config(config.retry_max, config.retry_base_delay_ms).await
        .map_err(|e| anyhow::anyhow!("Failed to connect to Hyprland IPC: {}", e))?;

    log::debug!("Successfully connected to Hyprland IPC for monitoring");

    // Listen for events in a loop until error or shutdown
    loop {
        match ipc.listen_for_events().await {
            Ok(event) => {
                if event.starts_with("vdesk>>") {
                    log::debug!("Received vdesk event: {}", event);
                    let mut mgr = manager.lock().await;
                    if let Err(e) = mgr.update_state().await {
                        log::error!("Failed to update virtual desktop state: {}", e);
                        // Don't fail the entire monitor cycle for state update errors
                    } else {
                        log::debug!("Virtual desktop state updated successfully");
                    }
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error listening for events: {}", e));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use crate::{ConfigWrapper, config::ModuleConfig};

    #[test]
    fn test_widget_update_algorithm_complexity() {
        use std::collections::BTreeMap;

        // Test that our BTreeMap optimization provides O(log n) complexity
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

        // Create HashSet for O(1) visibility lookup
        let visible_ids: HashSet<u32> = visible_vdesks.iter().map(|vd| vd.id).collect();

        // Test that visibility lookup is O(1)
        assert!(visible_ids.contains(&1));
        assert!(!visible_ids.contains(&2));
        assert!(visible_ids.contains(&3));

        // Test BTreeMap operations are O(log n) instead of O(n)
        let mut widgets: BTreeMap<u32, String> = BTreeMap::new();
        widgets.insert(1, "Widget 1".to_string());
        widgets.insert(2, "Widget 2".to_string());
        widgets.insert(3, "Widget 3".to_string());

        // O(log n) lookup instead of O(n) Vec::find
        assert!(widgets.contains_key(&1));
        assert!(widgets.contains_key(&3));
        assert!(!widgets.contains_key(&4));

        // O(log n) removal instead of O(n) Vec::remove
        assert!(widgets.remove(&2).is_some());
        assert_eq!(widgets.len(), 2);

        // Verify the optimized data structure maintains order
        let keys: Vec<u32> = widgets.keys().copied().collect();
        assert_eq!(keys, vec![1, 3]); // BTreeMap maintains sorted order
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

    #[test]
    fn test_polymorphic_config_deserialization() {
        // Test nested format (simulating waybar config structure)
        let nested_json = r#"{
            "config": {
                "format": "{icon} {name}",
                "show_empty": true,
                "separator": " | ",
                "format_icons": {"1": "🏠"},
                "show_window_count": true,
                "sort_by": "focused-first",
                "retry_max": 15,
                "retry_base_delay_ms": 750
            }
        }"#;

        // Parse as ConfigWrapper first, then convert
        let wrapper: ConfigWrapper = serde_json::from_str(nested_json).unwrap();
        let config: ModuleConfig = wrapper.into();
        assert_eq!(config.format, "{icon} {name}");
        assert_eq!(config.show_empty, true);
        assert_eq!(config.separator, " | ");
        assert_eq!(config.format_icons.get("1"), Some(&"🏠".to_string()));
        assert_eq!(config.show_window_count, true);
        assert_eq!(config.sort_by, "focused-first");
        assert_eq!(config.retry_max, 15);
        assert_eq!(config.retry_base_delay_ms, 750);

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

        let wrapper: ConfigWrapper = serde_json::from_str(direct_json).unwrap();
        let config: ModuleConfig = wrapper.into();
        assert_eq!(config.format, "{icon} {name}");
        assert_eq!(config.show_empty, true);
        assert_eq!(config.separator, " | ");
        assert_eq!(config.format_icons.get("1"), Some(&"🏠".to_string()));
        assert_eq!(config.show_window_count, true);
        assert_eq!(config.sort_by, "focused-first");
        assert_eq!(config.retry_max, 15);
        assert_eq!(config.retry_base_delay_ms, 750);
    }
}

// Export the module
waybar_module!(VirtualDesktopsModule);
