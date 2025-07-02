use crate::hyprland::HyprlandIPC;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct VirtualDesktop {
    pub id: u32,
    pub name: String,
    pub focused: bool,
    pub populated: bool,
    pub window_count: u32,
}

impl VirtualDesktop {
    pub fn new(id: u32, name: String) -> Self {
        Self {
            id,
            name,
            focused: false,
            populated: false,
            window_count: 0,
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

        // Update individual virtual desktop info for names
        for vdesk_id in 1..=5 {
            let vdesk_info = {
                let ipc = self.ipc.as_mut().unwrap();
                ipc.get_virtual_desktop_info(vdesk_id).await
            };

            if let Ok(info) = vdesk_info {
                self.update_virtual_desktop_name(vdesk_id, &info)?;
            }
        }

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
        
        // Parse the printstate output
        // Format: "Virtual desk 1:    Focus\n    Focused: true\n    Populated: true\n    Windows: 2\n"
        let lines: Vec<&str> = state.lines().collect();
        let mut current_vdesk: Option<VirtualDesktop> = None;
        
        for line in lines {
            let line = line.trim();
            
            if line.starts_with("Virtual desk ") {
                // Save previous virtual desktop if exists
                if let Some(vdesk) = current_vdesk.take() {
                    self.virtual_desktops.insert(vdesk.id, vdesk);
                }
                
                // Parse new virtual desktop
                if let Some(colon_pos) = line.find(':') {
                    let desk_part = &line[..colon_pos];
                    let name_part = line[colon_pos + 1..].trim();
                    
                    if let Some(id_str) = desk_part.strip_prefix("Virtual desk ") {
                        if let Ok(id) = id_str.parse::<u32>() {
                            current_vdesk = Some(VirtualDesktop::new(id, name_part.to_string()));
                        }
                    }
                }
            } else if let Some(ref mut vdesk) = current_vdesk {
                if line.starts_with("Focused: ") {
                    vdesk.focused = line.ends_with("true");
                } else if line.starts_with("Populated: ") {
                    vdesk.populated = line.ends_with("true");
                } else if line.starts_with("Windows: ") {
                    if let Some(count_str) = line.strip_prefix("Windows: ") {
                        vdesk.window_count = count_str.parse().unwrap_or(0);
                    }
                }
            }
        }
        
        // Save the last virtual desktop
        if let Some(vdesk) = current_vdesk {
            self.virtual_desktops.insert(vdesk.id, vdesk);
        }
        
        Ok(())
    }

    fn update_virtual_desktop_name(&mut self, vdesk_id: u32, vdesk_info: &str) -> Result<()> {
        // Parse printdesk output: "Virtual desk 1:    Focus"
        if let Some(colon_pos) = vdesk_info.find(':') {
            let name_part = vdesk_info[colon_pos + 1..].trim();

            // Update or create virtual desktop with the name
            if let Some(vdesk) = self.virtual_desktops.get_mut(&vdesk_id) {
                vdesk.name = name_part.to_string();
            } else {
                // Create new virtual desktop if it doesn't exist
                let vdesk = VirtualDesktop::new(vdesk_id, name_part.to_string());
                self.virtual_desktops.insert(vdesk_id, vdesk);
            }
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
        
        let state = r#"Virtual desks
Virtual desk 1:    Focus
    Focused: true
    Populated: true
    Windows: 2

Virtual desk 2:    Research
    Focused: false
    Populated: true
    Windows: 1

Virtual desk 3:    Comms
    Focused: false
    Populated: false
    Windows: 0
"#;
        
        manager.parse_virtual_desktop_state(state).unwrap();
        
        let vdesks = manager.get_virtual_desktops();
        assert_eq!(vdesks.len(), 3);
        
        let focus_vdesk = &vdesks[0];
        assert_eq!(focus_vdesk.id, 1);
        assert_eq!(focus_vdesk.name, "Focus");
        assert!(focus_vdesk.focused);
        assert!(focus_vdesk.populated);
        assert_eq!(focus_vdesk.window_count, 2);
        
        let research_vdesk = &vdesks[1];
        assert_eq!(research_vdesk.id, 2);
        assert_eq!(research_vdesk.name, "Research");
        assert!(!research_vdesk.focused);
        assert!(research_vdesk.populated);
        assert_eq!(research_vdesk.window_count, 1);
        
        let comms_vdesk = &vdesks[2];
        assert_eq!(comms_vdesk.id, 3);
        assert_eq!(comms_vdesk.name, "Comms");
        assert!(!comms_vdesk.focused);
        assert!(!comms_vdesk.populated);
        assert_eq!(comms_vdesk.window_count, 0);
    }
}
