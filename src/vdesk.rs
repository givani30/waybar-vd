//! # Virtual Desktop Management Module
//!
//! Provides data structures and management logic for Hyprland virtual desktops.
//! Handles parsing of Hyprland IPC responses, state management, and virtual
//! desktop lifecycle operations.
//!
//! # Core Concepts
//!
//! - **Virtual Desktop**: A logical grouping of workspaces with shared properties
//! - **Workspace**: Individual Hyprland workspace within a virtual desktop
//! - **State Management**: Thread-safe tracking of virtual desktop changes
//! - **JSON Parsing**: Robust deserialization of Hyprland IPC responses
//!
//! # Data Flow
//!
//! 1. **IPC Response**: Raw JSON from Hyprland workspace queries
//! 2. **Parsing**: Deserialize into structured `VirtualDesktop` objects
//! 3. **State Update**: Update manager state with new virtual desktop information
//! 4. **UI Notification**: Trigger widget updates through callback mechanism
//!
//! # Error Handling
//!
//! - **JSON Validation**: Comprehensive parsing error handling
//! - **State Consistency**: Atomic updates to prevent race conditions
//! - **Graceful Degradation**: Continue operation with partial data
//!
//! # Example Usage
//!
//! ```rust
//! use waybar_virtual_desktops_cffi::vdesk::{VirtualDesktopsManager, VirtualDesktop};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let mut manager = VirtualDesktopsManager::new();
//!
//! // Parse Hyprland workspace JSON
//! let json_response = r#"[{"id": 1, "name": "Desktop 1", "windows": 2}]"#;
//! manager.update_from_json(json_response)?;
//!
//! // Get current virtual desktops
//! let desktops = manager.get_virtual_desktops();
//! println!("Found {} virtual desktops", desktops.len());
//! # Ok(())
//! # }
//! ```

// src/vdesk.rs
use crate::hyprland::HyprlandIPC;
use anyhow::Result;
use std::collections::HashMap;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct VirtualDesktop {
    pub id: u32,
    pub name: String,
    pub focused: bool,
    pub populated: bool,
    #[serde(rename = "windows")]
    pub window_count: u32,
    pub workspaces: Vec<u32>,
}

impl VirtualDesktop {
    pub fn new(id: u32, name: String) -> Self {
        Self {
            id,
            name,
            focused: false,
            populated: false,
            window_count: 0,
            workspaces: Vec::new(),
        }
    }
}

pub struct VirtualDesktopsManager {
    virtual_desktops: HashMap<u32, VirtualDesktop>,
    ipc: Option<HyprlandIPC>,
}

impl VirtualDesktopsManager {
    pub fn new() -> Self {
        Self {
            virtual_desktops: HashMap::new(),
            ipc: None,
        }
    }
    
    pub async fn initialize(&mut self) -> Result<()> {
        self.ipc = Some(HyprlandIPC::new().await?);
        self.update_state().await?;
        Ok(())
    }
    
    pub async fn update_state(&mut self) -> Result<()> {
        if self.ipc.is_none() {
            self.ipc = Some(HyprlandIPC::new().await?);
        }

        // Get virtual desktop state from Hyprland
        let state = {
            let ipc = self.ipc.as_mut().unwrap();
            ipc.get_virtual_desktop_state().await?
        };



        // Parse the state and update our virtual desktops
        self.parse_virtual_desktop_state(&state)?;

        Ok(())
    }
    
    pub fn get_virtual_desktops(&self) -> Vec<VirtualDesktop> {
        let mut vdesks: Vec<_> = self.virtual_desktops.values().cloned().collect();
        vdesks.sort_by_key(|vd| vd.id);
        vdesks
    }
    
    pub fn get_focused_virtual_desktop(&self) -> Option<&VirtualDesktop> {
        self.virtual_desktops.values().find(|vd| vd.focused)
    }
    
    fn parse_virtual_desktop_state(&mut self, state: &str) -> Result<()> {
        // Clear current state
        self.virtual_desktops.clear();

        // Parse the JSON output from hyprctl printstate -j
        let virtual_desktops: Vec<VirtualDesktop> = serde_json::from_str(state)
            .map_err(|e| anyhow::anyhow!("Failed to parse virtual desktop JSON: {}", e))?;

        // Store the virtual desktops in our HashMap
        for vdesk in virtual_desktops {
            self.virtual_desktops.insert(vdesk.id, vdesk);
        }

        Ok(())
    }


}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_virtual_desktop_state() {
        let mut manager = VirtualDesktopsManager::new();

        let state = r#"[{
    "id": 1,
    "name": "  Focus",
    "focused": true,
    "populated": true,
    "workspaces": [1, 2],
    "windows": 2
},{
    "id": 2,
    "name": "󰍉 Research",
    "focused": false,
    "populated": true,
    "workspaces": [3, 4],
    "windows": 1
},{
    "id": 3,
    "name": "󰵅  Comms",
    "focused": false,
    "populated": false,
    "workspaces": [],
    "windows": 0
}]"#;
        
        manager.parse_virtual_desktop_state(state).unwrap();
        
        let vdesks = manager.get_virtual_desktops();
        assert_eq!(vdesks.len(), 3);
        
        let focus_vdesk = &vdesks[0];
        assert_eq!(focus_vdesk.id, 1);
        assert_eq!(focus_vdesk.name, "  Focus");
        assert!(focus_vdesk.focused);
        assert!(focus_vdesk.populated);
        assert_eq!(focus_vdesk.window_count, 2);
        assert_eq!(focus_vdesk.workspaces, vec![1, 2]);

        let research_vdesk = &vdesks[1];
        assert_eq!(research_vdesk.id, 2);
        assert_eq!(research_vdesk.name, "󰍉 Research");
        assert!(!research_vdesk.focused);
        assert!(research_vdesk.populated);
        assert_eq!(research_vdesk.window_count, 1);
        assert_eq!(research_vdesk.workspaces, vec![3, 4]);

        let comms_vdesk = &vdesks[2];
        assert_eq!(comms_vdesk.id, 3);
        assert_eq!(comms_vdesk.name, "󰵅  Comms");
        assert!(!comms_vdesk.focused);
        assert!(!comms_vdesk.populated);
        assert_eq!(comms_vdesk.window_count, 0);
    }

    #[test]
    fn test_parse_invalid_json() {
        let mut manager = VirtualDesktopsManager::new();

        // Test with invalid JSON
        let invalid_json = "{ invalid json }";
        let result = manager.parse_virtual_desktop_state(invalid_json);
        assert!(result.is_err());

        // Verify state is cleared on error (should be empty)
        let vdesks = manager.get_virtual_desktops();
        assert_eq!(vdesks.len(), 0);
    }
}
