//! Virtual desktop management and state tracking

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

        let state = {
            let ipc = self.ipc.as_mut().unwrap();
            ipc.get_virtual_desktop_state().await?
        };



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
        let incoming_vdesks: Vec<VirtualDesktop> = serde_json::from_str(state)
            .map_err(|e| anyhow::anyhow!("Failed to parse virtual desktop JSON: {}", e))?;

        let incoming_ids: std::collections::HashSet<u32> = incoming_vdesks.iter().map(|v| v.id).collect();

        // Update existing or add new desktops
        for vdesk in incoming_vdesks {
            self.virtual_desktops.insert(vdesk.id, vdesk);
        }

        // Remove desktops that are no longer present
        self.virtual_desktops.retain(|&id, _| incoming_ids.contains(&id));

        Ok(())
    }


}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_virtual_desktop_state() {
        let mut manager = VirtualDesktopsManager::new();

        // Initial state
        let initial_state = r#"[{
            "id": 1,
            "name": "  Focus",
            "focused": true,
            "populated": true,
            "workspaces": [1, 2],
            "windows": 2
        }]"#;
        manager.parse_virtual_desktop_state(initial_state).unwrap();
        assert_eq!(manager.get_virtual_desktops().len(), 1);

        // Update: one desktop modified, one added, one removed
        let updated_state = r#"[{
            "id": 1,
            "name": "  Focus",
            "focused": false,
            "populated": true,
            "workspaces": [1, 2],
            "windows": 3
        },{
            "id": 2,
            "name": "󰍉 Research",
            "focused": true,
            "populated": true,
            "workspaces": [3, 4],
            "windows": 1
        }]"#;
        manager.parse_virtual_desktop_state(updated_state).unwrap();
        
        let vdesks = manager.get_virtual_desktops();
        assert_eq!(vdesks.len(), 2);
        
        let focus_vdesk = manager.virtual_desktops.get(&1).unwrap();
        assert!(!focus_vdesk.focused);
        assert_eq!(focus_vdesk.window_count, 3);

        let research_vdesk = manager.virtual_desktops.get(&2).unwrap();
        assert!(research_vdesk.focused);
    }

    #[test]
    fn test_parse_invalid_json() {
        let mut manager = VirtualDesktopsManager::new();
        manager.virtual_desktops.insert(1, VirtualDesktop::new(1, "Test".to_string()));

        let invalid_json = "{ invalid json }";
        let result = manager.parse_virtual_desktop_state(invalid_json);
        assert!(result.is_err());

        // Ensure state is unchanged on error
        assert_eq!(manager.get_virtual_desktops().len(), 1);
    }
}
