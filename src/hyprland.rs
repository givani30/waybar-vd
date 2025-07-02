// src/hyprland.rs
use anyhow::{anyhow, Result};
use std::env;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;

pub struct HyprlandIPC {
    socket_path: PathBuf,
    event_socket_path: PathBuf,
}

impl HyprlandIPC {
    pub async fn new() -> Result<Self> {
        let instance_signature = env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .map_err(|_| anyhow!("HYPRLAND_INSTANCE_SIGNATURE not set"))?;
        
        let runtime_dir = env::var("XDG_RUNTIME_DIR")
            .map_err(|_| anyhow!("XDG_RUNTIME_DIR not set"))?;
        
        let socket_path = PathBuf::from(runtime_dir.clone())
            .join("hypr")
            .join(&instance_signature)
            .join(".socket.sock");
        
        let event_socket_path = PathBuf::from(runtime_dir)
            .join("hypr")
            .join(&instance_signature)
            .join(".socket2.sock");
        
        // Verify sockets exist
        if !socket_path.exists() {
            return Err(anyhow!("Hyprland command socket not found: {:?}", socket_path));
        }
        
        if !event_socket_path.exists() {
            return Err(anyhow!("Hyprland event socket not found: {:?}", event_socket_path));
        }
        
        Ok(Self {
            socket_path,
            event_socket_path,
        })
    }
    
    pub async fn listen_for_events(&mut self) -> Result<String> {
        loop {
            match self.try_listen_for_events().await {
                Ok(event) => return Ok(event),
                Err(e) => {
                    log::warn!("Event listening failed, retrying in 1s: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    async fn try_listen_for_events(&mut self) -> Result<String> {
        let stream = UnixStream::connect(&self.event_socket_path).await?;
        let reader = BufReader::new(stream);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            log::debug!("Received event: {}", line);
            // Filter for virtual desktop events
            if line.starts_with("vdesk>>") {
                return Ok(line);
            }
        }

        Err(anyhow!("Event stream ended"))
    }
    
    pub async fn get_virtual_desktop_state(&self) -> Result<String> {
        // Send command directly to Hyprland socket for JSON format
        self.send_command("j/printstate").await
    }
    
    pub async fn get_virtual_desktop_info(&self, vdesk_id: u32) -> Result<String> {
        // Send command directly to Hyprland socket
        let command = format!("printdesk {}", vdesk_id);
        self.send_command(&command).await
    }
    
    pub async fn switch_to_virtual_desktop(&self, vdesk_id: u32) -> Result<()> {
        // Send dispatch command directly to Hyprland socket
        let command = format!("dispatch vdesk {}", vdesk_id);
        self.send_command(&command).await?;
        Ok(())
    }
    
    /// Send a raw command to Hyprland via the command socket
    pub async fn send_command(&self, command: &str) -> Result<String> {
        use tokio::io::{AsyncWriteExt, AsyncReadExt};

        let mut stream = UnixStream::connect(&self.socket_path).await?;

        // Write the command to the socket
        stream.write_all(command.as_bytes()).await?;

        // Read the response
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await?;

        Ok(String::from_utf8_lossy(&response).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_hyprland_ipc_creation() {
        // This test will only work if Hyprland is running
        // and environment variables are set
        if env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
            let result = HyprlandIPC::new().await;
            match result {
                Ok(_) => println!("HyprlandIPC created successfully"),
                Err(e) => println!("Expected error when Hyprland not running: {}", e),
            }
        }
    }
}
