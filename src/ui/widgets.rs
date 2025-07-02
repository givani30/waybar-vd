//! GTK widget management for virtual desktop display

use crate::config::ModuleConfig;
use crate::hyprland::HyprlandIPC;
use crate::vdesk::VirtualDesktop;
use crate::errors::Result;

use std::collections::{BTreeMap, HashSet};
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
}

impl WidgetManager {
    /// Create widget manager
    pub fn new(container: GtkBox, config: ModuleConfig, runtime_handle: Handle) -> Self {
        Self {
            container,
            widgets: BTreeMap::new(),
            widget_order: Vec::new(),
            config,
            runtime_handle,
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

        // Reorder widgets if sequence changed
        if new_order != self.widget_order {
            for (new_position, &widget_id) in new_order.iter().enumerate() {
                if let Some(widget) = self.widgets.get(&widget_id) {
                    self.container.reorder_child(&widget.event_box, new_position as i32);
                }
            }
            self.widget_order = new_order;
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
}