//! Simple performance metrics for the virtual desktops module

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use serde::Serialize;

/// Simple performance metrics focused on the O(n²) optimization
#[derive(Debug)]
pub struct PerformanceMetrics {
    // Widget reordering metrics - the core focus
    widget_reorder_count: AtomicU64,
    widget_reorder_optimized_count: AtomicU64,
    widget_update_duration_micros: AtomicU64,

    // Basic error tracking
    ipc_error_count: AtomicU64,

    // Startup tracking
    uptime_start: Instant,
}

/// Simple timer for widget updates
pub struct WidgetUpdateTimer {
    start: Instant,
    metrics: std::sync::Arc<PerformanceMetrics>,
}

/// Simple metrics snapshot focused on reordering performance
#[derive(Debug, Serialize)]
pub struct MetricsSnapshot {
    pub uptime_seconds: u64,
    pub widget_reorders_total: u64,
    pub widget_reorders_optimized: u64,
    pub reorder_optimization_rate: f64,
    pub avg_widget_update_micros: f64,
    pub ipc_errors_total: u64,
}

impl PerformanceMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            widget_reorder_count: AtomicU64::new(0),
            widget_reorder_optimized_count: AtomicU64::new(0),
            widget_update_duration_micros: AtomicU64::new(0),
            ipc_error_count: AtomicU64::new(0),
            uptime_start: Instant::now(),
        }
    }

    /// Start timing a widget update
    pub fn start_widget_update_timer(&self, metrics: std::sync::Arc<PerformanceMetrics>) -> WidgetUpdateTimer {
        WidgetUpdateTimer {
            start: Instant::now(),
            metrics,
        }
    }

    /// Record a widget reorder operation
    pub fn record_widget_reorder(&self, was_optimized: bool) {
        self.widget_reorder_count.fetch_add(1, Ordering::Relaxed);
        if was_optimized {
            self.widget_reorder_optimized_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record an IPC error
    pub fn record_ipc_error(&self) {
        self.ipc_error_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        let uptime = self.uptime_start.elapsed();
        let reorders_total = self.widget_reorder_count.load(Ordering::Relaxed);
        let reorders_optimized = self.widget_reorder_optimized_count.load(Ordering::Relaxed);
        let total_update_micros = self.widget_update_duration_micros.load(Ordering::Relaxed);

        MetricsSnapshot {
            uptime_seconds: uptime.as_secs(),
            widget_reorders_total: reorders_total,
            widget_reorders_optimized: reorders_optimized,
            reorder_optimization_rate: if reorders_total > 0 {
                reorders_optimized as f64 / reorders_total as f64
            } else { 0.0 },
            avg_widget_update_micros: if reorders_total > 0 {
                total_update_micros as f64 / reorders_total as f64
            } else { 0.0 },
            ipc_errors_total: self.ipc_error_count.load(Ordering::Relaxed),
        }
    }

    /// Log metrics summary focused on reordering performance
    pub fn log_summary(&self) {
        let snapshot = self.snapshot();
        log::info!("Widget Reordering Performance:");
        log::info!("  Uptime: {}s", snapshot.uptime_seconds);
        log::info!("  Reorders: {} total, {} optimized ({:.1}% optimization rate)",
                  snapshot.widget_reorders_total, snapshot.widget_reorders_optimized,
                  snapshot.reorder_optimization_rate * 100.0);
        log::info!("  Avg update time: {:.2}ms", snapshot.avg_widget_update_micros / 1000.0);
        if snapshot.ipc_errors_total > 0 {
            log::warn!("  IPC errors: {}", snapshot.ipc_errors_total);
        }
    }
}

impl WidgetUpdateTimer {
    /// Complete the timer and record the measurement
    pub fn finish(self) {
        let duration_micros = self.start.elapsed().as_micros() as u64;
        self.metrics.widget_update_duration_micros.fetch_add(duration_micros, Ordering::Relaxed);
    }
}

impl Drop for WidgetUpdateTimer {
    fn drop(&mut self) {
        // Auto-record if timer is dropped without explicit finish()
        let duration_micros = self.start.elapsed().as_micros() as u64;
        self.metrics.widget_update_duration_micros.fetch_add(duration_micros, Ordering::Relaxed);
    }
}
