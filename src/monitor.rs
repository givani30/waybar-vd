//! Background monitoring for virtual desktop state changes

use crate::config::ModuleConfig;
use crate::errors::Result;
use crate::hyprland::HyprlandIPC;
use crate::vdesk::VirtualDesktopsManager;

use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

type VdeskUpdateMessage = Vec<crate::vdesk::VirtualDesktop>;

/// Resilient monitoring loop
pub async fn resilient_monitor_loop(
    manager: Arc<Mutex<VirtualDesktopsManager>>,
    config: ModuleConfig,
    mut shutdown_rx: tokio::sync::oneshot::Receiver<()>,
    tx: mpsc::Sender<VdeskUpdateMessage>,
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
            result = monitor_virtual_desktops_once(&manager, &config, tx.clone()) => {
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
                            return Err(crate::errors::VirtualDesktopError::RetryExhausted {
                                attempts: MAX_CONSECUTIVE_FAILURES,
                                last_error: e.to_string(),
                            });
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

/// Single monitoring cycle
async fn monitor_virtual_desktops_once(
    manager: &Arc<Mutex<VirtualDesktopsManager>>,
    config: &ModuleConfig,
    tx: mpsc::Sender<VdeskUpdateMessage>,
) -> Result<()> {
    log::debug!("Starting monitor cycle...");

    // Create IPC connection
    let mut ipc = HyprlandIPC::with_config(config.retry_max, config.retry_base_delay_ms).await
        .map_err(|e| crate::errors::VirtualDesktopError::IpcConnection {
            source: std::io::Error::new(std::io::ErrorKind::ConnectionRefused, e.to_string())
        })?;

    log::debug!("Successfully connected to Hyprland IPC for monitoring");

    // Listen for events
    loop {
        match ipc.listen_for_events().await {
            Ok(event) => {
                if event.starts_with("vdesk>>") {
                    log::debug!("Received vdesk event: {}", event);
                    let mut mgr = manager.lock().await;
                    if let Err(e) = mgr.update_state().await {
                        log::error!("Failed to update virtual desktop state: {}", e);
                    } else {
                        log::debug!("Virtual desktop state updated, sending to UI thread.");
                        // Get the new state and send it through the channel
                        let vdesks = mgr.get_virtual_desktops();
                        if let Err(e) = tx.send(vdesks).await {
                            log::error!("Failed to send update to UI thread: {}. Channel closed.", e);
                            // Channel is closed, so we should exit the loop.
                            return Err(crate::errors::VirtualDesktopError::Internal {
                                message: "UI channel closed".to_string(),
                            });
                        }
                    }
                }
            }
            Err(e) => {
                return Err(crate::errors::VirtualDesktopError::IpcConnection { 
                    source: std::io::Error::new(std::io::ErrorKind::BrokenPipe, e.to_string()) 
                });
            }
        }
    }
}