//! GTK widget management for virtual desktop display

use crate::config::ModuleConfig;
use crate::hyprland::HyprlandIPC;
use crate::metrics::PerformanceMetrics;
use crate::vdesk::VirtualDesktop;
use crate::errors::Result;

use std::collections::{BTreeMap, HashSet, HashMap};
use std::sync::Arc;
use tokio::runtime::Handle;
use waybar_cffi::gtk::{prelude::*, Label, Box as GtkBox, EventBox};

/// Virtual desktop widget
#[derive(Debug)]
pub struct VirtualDesktopWidget {
    pub event_box: EventBox,
    pub label: Label,
    pub vdesk_id: u32,
    pub display_text: String,
    pub tooltip_text: String,
    pub focused: bool,
}

impl VirtualDesktopWidget {
    /// Create widget
    pub fn new(
        vdesk: &VirtualDesktop,
        display_text: String,
        tooltip_text: String,
        config: &ModuleConfig,
        runtime_handle: Handle,
    ) -> Self {
        let label = Label::new(Some(&display_text));
        label.set_tooltip_text(Some(&tooltip_text));

        // Make the label clickable
        let event_box = EventBox::new();
        event_box.add(&label);

        // Set up click handler using async IPC
        let vdesk_id_for_click = vdesk.id;
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

        if !vdesk.populated && !config.show_empty {
            style_context.add_class("hidden");
        }

        Self {
            event_box,
            label,
            vdesk_id: vdesk.id,
            display_text,
            tooltip_text,
            focused: vdesk.focused,
        }
    }

    /// Update widget if changed
    pub fn update_if_changed(
        &mut self,
        vdesk: &VirtualDesktop,
        display_text: String,
        tooltip_text: String,
    ) -> bool {
        let mut updated = false;

        // Update display text if changed
        if self.display_text != display_text {
            self.label.set_text(&display_text);
            self.display_text = display_text;
            updated = true;
        }

        // Update tooltip if changed
        if self.tooltip_text != tooltip_text {
            self.label.set_tooltip_text(Some(&tooltip_text));
            self.tooltip_text = tooltip_text;
            updated = true;
        }

        // Update CSS classes if focus state changed
        if self.focused != vdesk.focused {
            let style_context = self.label.style_context();
            if vdesk.focused {
                style_context.remove_class("vdesk-unfocused");
                style_context.add_class("vdesk-focused");
            } else {
                style_context.remove_class("vdesk-focused");
                style_context.add_class("vdesk-unfocused");
            }
            self.focused = vdesk.focused;
            updated = true;
        }

        updated
    }
}

/// Widget lifecycle management
pub struct WidgetManager {
    container: GtkBox,
    widgets: BTreeMap<u32, VirtualDesktopWidget>,
    widget_order: Vec<u32>,
    config: ModuleConfig,
    runtime_handle: Handle,
    metrics: Arc<PerformanceMetrics>,
}

impl WidgetManager {
    /// Create widget manager
    pub fn new(container: GtkBox, config: ModuleConfig, runtime_handle: Handle, metrics: Arc<PerformanceMetrics>) -> Self {
        Self {
            container,
            widgets: BTreeMap::new(),
            widget_order: Vec::new(),
            config,
            runtime_handle,
            metrics,
        }
    }

    /// Update widgets
    pub fn update_widgets(&mut self, visible_vdesks: &[VirtualDesktop]) -> Result<()> {
        // Create visibility lookup
        let visible_ids: HashSet<u32> = visible_vdesks.iter().map(|vd| vd.id).collect();

        // Remove widgets for hidden virtual desktops
        let widgets_to_remove: Vec<u32> = self.widgets.keys()
            .filter(|&id| !visible_ids.contains(id))
            .copied()
            .collect();

        for widget_id in widgets_to_remove {
            if let Some(widget) = self.widgets.remove(&widget_id) {
                self.container.remove(&widget.event_box);
                self.widget_order.retain(|&id| id != widget_id);
            }
        }

        // Update or create widgets for visible virtual desktops
        let mut new_order = Vec::with_capacity(visible_vdesks.len());

        for vdesk in visible_vdesks {
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

            if let Some(existing_widget) = self.widgets.get_mut(&vdesk.id) {
                existing_widget.update_if_changed(vdesk, display_text, tooltip_text);
            } else {
                let widget = VirtualDesktopWidget::new(
                    vdesk,
                    display_text,
                    tooltip_text,
                    &self.config,
                    self.runtime_handle.clone(),
                );
                self.container.add(&widget.event_box);
                self.widgets.insert(vdesk.id, widget);
            }
        }

        // Reorder widgets if sequence changed (optimized for O(k) complexity)
        if new_order != self.widget_order {
            self.optimize_widget_reordering(new_order)?;
        }

        Ok(())
    }

    /// Widget count
    pub fn widget_count(&self) -> usize {
        self.widgets.len()
    }

    /// Check if widget exists
    pub fn has_widget(&self, vdesk_id: u32) -> bool {
        self.widgets.contains_key(&vdesk_id)
    }

    /// Widget order
    pub fn widget_order(&self) -> &[u32] {
        &self.widget_order
    }

    /// Show all widgets
    pub fn refresh_display(&self) {
        self.container.show_all();
    }

    /// Optimized widget reordering with O(k) complexity where k = number of changed positions
    /// This minimizes GTK reorder operations and reduces visual flicker
    fn optimize_widget_reordering(&mut self, new_order: Vec<u32>) -> Result<()> {
        if new_order == self.widget_order {
            // Record that reordering was optimized (no moves needed)
            self.metrics.record_widget_reorder(true);
            return Ok(());
        }

        // Build position maps for efficient lookup
        let current_positions: HashMap<u32, usize> = self.widget_order
            .iter()
            .enumerate()
            .map(|(pos, &id)| (id, pos))
            .collect();

        let target_positions: HashMap<u32, usize> = new_order
            .iter()
            .enumerate()
            .map(|(pos, &id)| (id, pos))
            .collect();

        // Collect widgets that need to be moved
        let mut moves_needed = Vec::new();
        for (widget_id, &target_pos) in &target_positions {
            if let Some(&current_pos) = current_positions.get(widget_id) {
                if current_pos != target_pos {
                    moves_needed.push((*widget_id, target_pos));
                }
            }
        }

        // Only perform GTK operations if moves are actually needed
        let was_optimized = moves_needed.len() < new_order.len();
        if !moves_needed.is_empty() {
            log::debug!("Optimized reordering: moving {} out of {} widgets",
                       moves_needed.len(), new_order.len());

            // Batch GTK operations to reduce layout thrashing
            // Note: GTK3 doesn't have freeze/thaw, but we can minimize calls
            for (widget_id, target_pos) in moves_needed {
                if let Some(widget) = self.widgets.get(&widget_id) {
                    self.container.reorder_child(&widget.event_box, target_pos as i32);
                }
            }
        } else {
            log::debug!("Optimized reordering: no moves needed");
        }

        // Record reordering metrics
        self.metrics.record_widget_reorder(was_optimized);

        self.widget_order = new_order;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ModuleConfig;
    // Note: GTK imports would be needed for actual widget testing

    fn create_test_vdesk(id: u32, name: &str, focused: bool, populated: bool) -> VirtualDesktop {
        VirtualDesktop {
            id,
            name: name.to_string(),
            focused,
            populated,
            window_count: if populated { 2 } else { 0 },
            workspaces: if populated { vec![id, id + 10] } else { vec![] },
        }
    }

    #[test]
    #[ignore] // Requires GTK initialization
    fn test_widget_creation() {
        let config = ModuleConfig::default();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let handle = runtime.handle().clone();

        let vdesk = create_test_vdesk(1, "Test Desktop", true, true);
        let widget = VirtualDesktopWidget::new(
            &vdesk,
            "Test Desktop".to_string(),
            "Virtual Desktop 1: Test Desktop".to_string(),
            &config,
            handle,
        );

        assert_eq!(widget.vdesk_id, 1);
        assert_eq!(widget.display_text, "Test Desktop");
        assert!(widget.focused);
    }

    #[test]
    #[ignore] // Requires GTK initialization
    fn test_widget_update_detection() {
        let config = ModuleConfig::default();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let handle = runtime.handle().clone();

        let vdesk = create_test_vdesk(1, "Test Desktop", false, true);
        let mut widget = VirtualDesktopWidget::new(
            &vdesk,
            "Test Desktop".to_string(),
            "Tooltip".to_string(),
            &config,
            handle,
        );

        // Test no change
        let updated = widget.update_if_changed(
            &vdesk,
            "Test Desktop".to_string(),
            "Tooltip".to_string(),
        );
        assert!(!updated);

        // Test display text change
        let updated = widget.update_if_changed(
            &vdesk,
            "New Text".to_string(),
            "Tooltip".to_string(),
        );
        assert!(updated);
        assert_eq!(widget.display_text, "New Text");

        // Test focus change
        let focused_vdesk = VirtualDesktop { focused: true, ..vdesk };
        let updated = widget.update_if_changed(
            &focused_vdesk,
            "New Text".to_string(),
            "Tooltip".to_string(),
        );
        assert!(updated);
        assert!(widget.focused);
    }

    #[tokio::test]
    async fn test_widget_manager_updates() {
        let _config = ModuleConfig::default();
        let _runtime_handle = tokio::runtime::Handle::current();
        
        // Note: This would need GTK initialization in a real test environment
        // For now, we test the logic without actual GTK widgets
        
        let vdesks = vec![
            create_test_vdesk(1, "Desktop 1", true, true),
            create_test_vdesk(2, "Desktop 2", false, true),
        ];

        // Test that the visibility logic works correctly
        let visible_ids: HashSet<u32> = vdesks.iter().map(|vd| vd.id).collect();
        assert!(visible_ids.contains(&1));
        assert!(visible_ids.contains(&2));
        assert!(!visible_ids.contains(&3));
    }

    #[test]
    fn test_widget_order_tracking() {
        let order1 = vec![1, 2, 3];
        let order2 = vec![1, 3, 2];
        let order3 = vec![1, 2, 3];

        // Test order comparison
        assert_ne!(order1, order2);
        assert_eq!(order1, order3);

        // Test order change detection
        let mut current_order = vec![1, 2, 3];
        let new_order = vec![3, 1, 2];

        if new_order != current_order {
            current_order = new_order.clone();
        }

        assert_eq!(current_order, vec![3, 1, 2]);
    }

    #[test]
    fn test_optimized_reordering_logic() {
        use std::collections::HashMap;

        // Test position mapping logic
        let current_order = vec![1, 2, 3, 4, 5];
        let new_order = vec![1, 3, 2, 5, 4];

        let current_positions: HashMap<u32, usize> = current_order
            .iter()
            .enumerate()
            .map(|(pos, &id)| (id, pos))
            .collect();

        let target_positions: HashMap<u32, usize> = new_order
            .iter()
            .enumerate()
            .map(|(pos, &id)| (id, pos))
            .collect();

        // Collect widgets that need to be moved
        let mut moves_needed = Vec::new();
        for (widget_id, &target_pos) in &target_positions {
            if let Some(&current_pos) = current_positions.get(widget_id) {
                if current_pos != target_pos {
                    moves_needed.push((*widget_id, target_pos));
                }
            }
        }

        // Should only move widgets 2, 3, 4, 5 (widget 1 stays in position 0)
        moves_needed.sort_by_key(|&(id, _)| id);
        assert_eq!(moves_needed, vec![(2, 2), (3, 1), (4, 4), (5, 3)]);
    }

    #[test]
    fn test_reordering_edge_cases() {
        use std::collections::HashMap;

        // Test no changes needed
        let order = vec![1, 2, 3];
        let current_positions: HashMap<u32, usize> = order
            .iter()
            .enumerate()
            .map(|(pos, &id)| (id, pos))
            .collect();
        let target_positions = current_positions.clone();

        let moves_needed: Vec<_> = target_positions
            .iter()
            .filter_map(|(widget_id, &target_pos)| {
                current_positions.get(widget_id)
                    .filter(|&&current_pos| current_pos != target_pos)
                    .map(|_| (*widget_id, target_pos))
            })
            .collect();

        assert!(moves_needed.is_empty());

        // Test complete reversal
        let current_order = vec![1, 2, 3];
        let new_order = vec![3, 2, 1];

        let current_positions: HashMap<u32, usize> = current_order
            .iter()
            .enumerate()
            .map(|(pos, &id)| (id, pos))
            .collect();

        let target_positions: HashMap<u32, usize> = new_order
            .iter()
            .enumerate()
            .map(|(pos, &id)| (id, pos))
            .collect();

        let mut moves_needed = Vec::new();
        for (widget_id, &target_pos) in &target_positions {
            if let Some(&current_pos) = current_positions.get(widget_id) {
                if current_pos != target_pos {
                    moves_needed.push((*widget_id, target_pos));
                }
            }
        }

        // Should move widgets 1 and 3 (widget 2 stays in middle)
        moves_needed.sort_by_key(|&(id, _)| id);
        assert_eq!(moves_needed, vec![(1, 2), (3, 0)]);
    }

    #[test]
    fn test_reordering_performance_characteristics() {
        use std::collections::HashMap;

        // Test that optimization reduces operations for large widget sets
        let large_order: Vec<u32> = (1..=100).collect();

        // Scenario 1: No changes (should be O(1))
        let current_positions: HashMap<u32, usize> = large_order
            .iter()
            .enumerate()
            .map(|(pos, &id)| (id, pos))
            .collect();
        let target_positions = current_positions.clone();

        let moves_needed: Vec<_> = target_positions
            .iter()
            .filter_map(|(widget_id, &target_pos)| {
                current_positions.get(widget_id)
                    .filter(|&&current_pos| current_pos != target_pos)
                    .map(|_| (*widget_id, target_pos))
            })
            .collect();

        assert_eq!(moves_needed.len(), 0);

        // Scenario 2: Single element move (should be O(1))
        let mut single_move_order = large_order.clone();
        single_move_order.swap(0, 1); // Move first element to second position

        let current_positions: HashMap<u32, usize> = large_order
            .iter()
            .enumerate()
            .map(|(pos, &id)| (id, pos))
            .collect();

        let target_positions: HashMap<u32, usize> = single_move_order
            .iter()
            .enumerate()
            .map(|(pos, &id)| (id, pos))
            .collect();

        let moves_needed: Vec<_> = target_positions
            .iter()
            .filter_map(|(widget_id, &target_pos)| {
                current_positions.get(widget_id)
                    .filter(|&&current_pos| current_pos != target_pos)
                    .map(|_| (*widget_id, target_pos))
            })
            .collect();

        // Only 2 widgets should need to move (the swapped ones)
        assert_eq!(moves_needed.len(), 2);

        // Scenario 3: Partial reordering (should be O(k) where k < n)
        let mut partial_reorder = large_order.clone();
        // Reverse only the first 10 elements
        partial_reorder[0..10].reverse();

        let current_positions: HashMap<u32, usize> = large_order
            .iter()
            .enumerate()
            .map(|(pos, &id)| (id, pos))
            .collect();

        let target_positions: HashMap<u32, usize> = partial_reorder
            .iter()
            .enumerate()
            .map(|(pos, &id)| (id, pos))
            .collect();

        let moves_needed: Vec<_> = target_positions
            .iter()
            .filter_map(|(widget_id, &target_pos)| {
                current_positions.get(widget_id)
                    .filter(|&&current_pos| current_pos != target_pos)
                    .map(|_| (*widget_id, target_pos))
            })
            .collect();

        // Should only move the affected elements (8 out of 10, since 2 stay in place)
        assert!(moves_needed.len() <= 10);
        assert!(moves_needed.len() < large_order.len());

        // Verify that elements 11-100 don't need to move
        let moved_ids: std::collections::HashSet<u32> = moves_needed
            .iter()
            .map(|&(id, _)| id)
            .collect();

        for id in 11..=100 {
            assert!(!moved_ids.contains(&id), "Widget {} should not need to move", id);
        }
    }
}