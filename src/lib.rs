//! waybar-vd: Waybar virtual desktops module for Hyprland integration

// src/lib.rs
use std::sync::Arc;
use std::thread;

use anyhow::Result;
use serde::Deserialize;
use tokio::sync::mpsc;
use waybar_cffi::{
    gtk::{prelude::*, Box as GtkBox, Orientation, glib},
    waybar_module, InitInfo, Module,
};

pub mod config;
pub mod errors;
pub mod hyprland;
pub mod metrics;
pub mod monitor;
pub mod ui;
pub mod vdesk;

use config::ModuleConfig;
use hyprland::HyprlandIPC;
use metrics::PerformanceMetrics;
use ui::WidgetManager;
use vdesk::VirtualDesktopsManager;

type VdeskUpdateMessage = Vec<vdesk::VirtualDesktop>;

/// Configuration wrapper supporting both nested and direct formats
#[derive(Deserialize)]
#[serde(untagged)]
pub enum ConfigWrapper {
    Nested { config: ModuleConfig },
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

/// Waybar module configuration interface
#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_show_empty")]
    pub show_empty: bool,
    #[serde(default = "default_separator")]
    pub separator: String,
    #[serde(default)]
    pub format_icons: std::collections::HashMap<String, String>,
    #[serde(default = "default_show_window_count")]
    pub show_window_count: bool,
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
    #[serde(default = "default_retry_max")]
    pub retry_max: u32,
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

/// Main Waybar module for Hyprland virtual desktop display
pub struct VirtualDesktopsModule {
    widget_manager: Arc<std::sync::Mutex<WidgetManager>>,
    _runtime: Arc<tokio::runtime::Runtime>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    monitor_handle: Option<tokio::task::JoinHandle<()>>,
    metrics: Arc<PerformanceMetrics>,
    _update_handle: glib::JoinHandle<()>,
}

impl Module for VirtualDesktopsModule {
    type Config = Config;

    fn init(info: &InitInfo, config: Self::Config) -> Self {
        let _ = env_logger::try_init();
        log::info!("waybar-vd module initializing...");

        let init_start = std::time::Instant::now();
        let metrics = Arc::new(PerformanceMetrics::new());

        // Convert string sort_by to enum
        let sort_by = config.sort_by.parse().unwrap_or_else(|e| {
            log::warn!(
                "Invalid sort_by value '{}': {}. Using default.",
                config.sort_by,
                e
            );
            crate::config::SortStrategy::default()
        });

        let module_config = ModuleConfig {
            format: config.format,
            show_empty: config.show_empty,
            separator: config.separator,
            format_icons: config.format_icons,
            show_window_count: config.show_window_count,
            sort_by,
            retry_max: config.retry_max,
            retry_base_delay_ms: config.retry_base_delay_ms,
        };

        if let Err(e) = module_config.validate() {
            log::error!("Configuration validation failed: {}", e);
            panic!("Invalid configuration: {}", e);
        }

        log::debug!(
            "Module config: format={}, show_empty={}",
            module_config.format,
            module_config.show_empty
        );
        let container = info.get_root_widget();
        let hbox = GtkBox::new(Orientation::Horizontal, 0);

        // Set CSS name for styling
        hbox.set_widget_name("waybar-vd");

        container.add(&hbox);
        log::debug!("Created GTK container widget with CSS name 'waybar-vd'");

        let rt = Arc::new(tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"));
        let runtime_handle = rt.handle().clone();
        let manager = Arc::new(tokio::sync::Mutex::new(VirtualDesktopsManager::new()));

        let (tx, mut rx) = mpsc::channel::<VdeskUpdateMessage>(32);

        {
            let manager_for_init = Arc::clone(&manager);
            let tx_for_init = tx.clone();
            thread::spawn(move || {
                runtime_handle.block_on(async {
                    let mut mgr = manager_for_init.lock().await;
                    if let Err(e) = mgr.initialize().await {
                        log::error!("Failed to initialize virtual desktop manager: {}", e);
                        return;
                    }
                    // Send the initial state immediately
                    let initial_state = mgr.get_virtual_desktops();
                    if let Err(e) = tx_for_init.send(initial_state).await {
                        log::error!("Failed to send initial state: {}", e);
                    }
                });
            })
            .join()
            .unwrap_or_else(|_| {
                log::error!("Failed to join initialization thread");
            });
        }

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let manager_clone = Arc::clone(&manager);
        let config_clone = module_config.clone();
        let monitor_handle = rt.handle().spawn(async move {
            // Pass the transmitter `tx` to the monitor loop
            if let Err(e) =
                crate::monitor::resilient_monitor_loop(manager_clone, config_clone, shutdown_rx, tx)
                    .await
            {
                log::error!("Resilient monitor loop failed: {}", e);
            }
        });

        let widget_manager =
            WidgetManager::new(hbox, module_config.clone(), Arc::clone(&metrics));
        let metrics_clone = Arc::clone(&metrics);

        // Create a shared reference to the widget manager
        let widget_manager_shared = Arc::new(std::sync::Mutex::new(widget_manager));
        let widget_manager_clone = Arc::clone(&widget_manager_shared);

        let update_handle = glib::MainContext::default().spawn_local(async move {
            while let Some(vdesks) = rx.recv().await {
                let _timer = metrics_clone.start_widget_update_timer(Arc::clone(&metrics_clone));

                // This code now runs on the main UI thread without blocking
                if let Ok(mut wm) = widget_manager_clone.lock() {
                    // Pass the full, unfiltered list - WidgetManager handles visibility internally
                    if let Err(e) = wm.update_widgets(&vdesks) {
                        log::error!("Failed to update widgets: {}", e);
                        metrics_clone.record_ipc_error();
                    }
                    wm.refresh_display();
                }
            }
        });

        Self {
            widget_manager: widget_manager_shared,
            _runtime: rt,
            shutdown_tx: Some(shutdown_tx),
            monitor_handle: Some(monitor_handle),
            metrics: Arc::clone(&metrics),
            _update_handle: update_handle, // Store the handle
        }
    }

    fn update(&mut self) {
        // Updates are now event-driven through channels
        log::debug!("Manual update triggered, but updates are now event-driven.");
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

        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        if let Some(monitor_handle) = self.monitor_handle.take() {
            let rt_handle = self._runtime.handle().clone();
            std::thread::spawn(move || {
                rt_handle.block_on(async {
                    match tokio::time::timeout(tokio::time::Duration::from_secs(5), monitor_handle)
                        .await
                    {
                        Ok(Ok(())) => log::debug!("Monitor task completed gracefully"),
                        Ok(Err(e)) => log::warn!("Monitor task completed with error: {}", e),
                        Err(_) => log::warn!("Monitor task shutdown timed out"),
                    }
                });
            })
            .join()
            .unwrap_or_else(|_| {
                log::error!("Failed to join shutdown thread");
            });
        }

        // Log final metrics summary before shutdown
        log::info!("Final performance metrics:");
        self.log_metrics_summary();

        log::info!("VirtualDesktopsModule shutdown complete");
    }
}

impl VirtualDesktopsModule {
    // update_display method removed - updates now come through async channels

    /// Get current performance metrics snapshot
    pub fn get_metrics(&self) -> crate::metrics::MetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Log performance metrics summary
    pub fn log_metrics_summary(&self) {
        self.metrics.log_summary();
    }

    /// Force a metrics log (for testing/debugging)
    pub fn force_metrics_log(&self) {
        log::info!("=== PERFORMANCE METRICS REPORT ===");
        self.metrics.log_summary();
        log::info!("=== END METRICS REPORT ===");
    }

    fn switch_to_virtual_desktop(&self, vdesk_id: u32) -> Result<()> {
        // Spawn a new thread to handle the async task
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            
            rt.block_on(async move {
                match HyprlandIPC::new().await {
                    Ok(ipc) => {
                        if let Err(e) = ipc.switch_to_virtual_desktop(vdesk_id).await {
                            log::error!("Failed to switch to virtual desktop {}: {}", vdesk_id, e);
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to create Hyprland IPC for action: {}", e);
                    }
                }
            });
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{config::ModuleConfig, ConfigWrapper};
    use std::collections::HashSet;

    #[test]
    fn test_widget_update_algorithm_complexity() {
        use std::collections::BTreeMap;
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

        let visible_ids: HashSet<u32> = visible_vdesks.iter().map(|vd| vd.id).collect();
        assert!(visible_ids.contains(&1));
        assert!(!visible_ids.contains(&2));
        assert!(visible_ids.contains(&3));

        let mut widgets: BTreeMap<u32, String> = BTreeMap::new();
        widgets.insert(1, "Widget 1".to_string());
        widgets.insert(2, "Widget 2".to_string());
        widgets.insert(3, "Widget 3".to_string());

        assert!(widgets.contains_key(&1));
        assert!(widgets.contains_key(&3));
        assert!(!widgets.contains_key(&4));

        assert!(widgets.remove(&2).is_some());
        assert_eq!(widgets.len(), 2);

        let keys: Vec<u32> = widgets.keys().copied().collect();
        assert_eq!(keys, vec![1, 3]);
    }

    #[test]
    fn test_jitter_calculation() {
        let base_delay = 1000u64;
        let jitter_range = base_delay / 4;

        for _ in 0..100 {
            let jitter = fastrand::u64(0..=jitter_range * 2);
            let delay_ms = base_delay
                .saturating_sub(jitter_range)
                .saturating_add(jitter);
            assert!(delay_ms >= 750);
            assert!(delay_ms <= 1250);
        }
    }

    #[test]
    fn test_polymorphic_config_deserialization() {
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

        let wrapper: ConfigWrapper = serde_json::from_str(nested_json).unwrap();
        let config: ModuleConfig = wrapper.into();
        assert_eq!(config.format, "{icon} {name}");
        assert_eq!(config.show_empty, true);
        assert_eq!(config.separator, " | ");
        assert_eq!(config.format_icons.get("1"), Some(&"🏠".to_string()));
        assert_eq!(config.show_window_count, true);
        assert_eq!(config.sort_by, crate::config::SortStrategy::FocusedFirst);
        assert_eq!(config.retry_max, 15);
        assert_eq!(config.retry_base_delay_ms, 750);
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
        assert_eq!(config.sort_by, crate::config::SortStrategy::FocusedFirst);
        assert_eq!(config.retry_max, 15);
        assert_eq!(config.retry_base_delay_ms, 750);
    }
}

// Export the module
waybar_module!(VirtualDesktopsModule);
