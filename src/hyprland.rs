//! Hyprland IPC communication for virtual desktop management

// src/hyprland.rs
use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::env;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;

/// Regex pattern for validating Hyprland instance signatures
static INSTANCE_SIGNATURE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9_-]{1,64}$")
        .expect("Invalid regex pattern for instance signature validation")
});

/// Validates Hyprland instance signature
fn validate_instance_signature(signature: &str) -> Result<()> {
    if signature.is_empty() {
        return Err(anyhow!("Invalid HYPRLAND_INSTANCE_SIGNATURE: empty signature"));
    }

    if !INSTANCE_SIGNATURE_PATTERN.is_match(signature) {
        return Err(anyhow!(
            "Invalid HYPRLAND_INSTANCE_SIGNATURE: '{}' contains unsafe characters or invalid format. \
             Must be alphanumeric with optional underscores/hyphens, 1-64 characters long.",
            signature
        ));
    }

    log::debug!("Instance signature '{}' validated successfully", signature);
    Ok(())
}

/// Hyprland IPC client
#[derive(Debug)]
pub struct HyprlandIPC {
    socket_path: PathBuf,
    event_socket_path: PathBuf,
    retry_max: u32,
    retry_base_delay_ms: u64,
}

impl HyprlandIPC {
    pub async fn new() -> Result<Self> {
        Self::with_config(10, 500).await
    }

    pub async fn with_config(retry_max: u32, retry_base_delay_ms: u64) -> Result<Self> {
        let instance_signature = env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .map_err(|_| anyhow!("HYPRLAND_INSTANCE_SIGNATURE not set"))?;

        // Validate instance signature
        validate_instance_signature(&instance_signature)?;

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
        
        if !socket_path.exists() {
            return Err(anyhow!("Hyprland command socket not found: {:?}", socket_path));
        }
        
        if !event_socket_path.exists() {
            return Err(anyhow!("Hyprland event socket not found: {:?}", event_socket_path));
        }
        
        Ok(Self {
            socket_path,
            event_socket_path,
            retry_max,
            retry_base_delay_ms,
        })
    }
    
    pub async fn listen_for_events(&mut self) -> Result<String> {
        let mut retry_count = 0;
        let max_retries = self.retry_max;
        let base_delay_ms = self.retry_base_delay_ms;
        const MAX_DELAY_MS: u64 = 30000; // 30 seconds max

        loop {
            match self.try_listen_for_events().await {
                Ok(event) => {
                    return Ok(event);
                }
                Err(e) => {
                    retry_count += 1;

                    if retry_count > max_retries {
                        log::error!("Max retries ({}) exceeded for event listening. Giving up.", max_retries);
                        return Err(anyhow::anyhow!("Event listening failed after {} retries: {}", max_retries, e));
                    }

                    // Exponential backoff with jitter
                    let base_delay = std::cmp::min(
                        base_delay_ms * 2_u64.pow(retry_count - 1),
                        MAX_DELAY_MS
                    );

                    // Add ±25% jitter
                    let jitter_range = base_delay / 4; // 25% of base delay
                    let jitter = fastrand::u64(0..=jitter_range * 2); // 0 to 50% of base
                    let delay_ms = base_delay.saturating_sub(jitter_range).saturating_add(jitter);

                    log::warn!("Event listening failed (attempt {}/{}), retrying in {}ms (base: {}ms): {}",
                              retry_count, max_retries, delay_ms, base_delay, e);

                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
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
            if line.starts_with("vdesk>>") {
                return Ok(line);
            }
        }

        Err(anyhow!("Event stream ended"))
    }
    
    pub async fn get_virtual_desktop_state(&self) -> Result<String> {
        self.send_command("j/printstate").await
    }
    
    pub async fn get_virtual_desktop_info(&self, vdesk_id: u32) -> Result<String> {
        let command = format!("printdesk {}", vdesk_id);
        self.send_command(&command).await
    }
    
    pub async fn switch_to_virtual_desktop(&self, vdesk_id: u32) -> Result<()> {
        let command = format!("dispatch vdesk {}", vdesk_id);
        self.send_command(&command).await?;
        Ok(())
    }
    
    /// Send a raw command to Hyprland via the command socket
    pub async fn send_command(&self, command: &str) -> Result<String> {
        use tokio::io::{AsyncWriteExt, AsyncReadExt};

        let mut stream = UnixStream::connect(&self.socket_path).await?;

        stream.write_all(command.as_bytes()).await?;

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
        if env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
            let result = HyprlandIPC::new().await;
            match result {
                Ok(_) => println!("HyprlandIPC created successfully"),
                Err(e) => println!("Expected error when Hyprland not running: {}", e),
            }
        }
    }

    #[tokio::test]
    async fn test_environment_variable_validation() {
        std::env::set_var("XDG_RUNTIME_DIR", "/tmp");

        std::env::set_var("HYPRLAND_INSTANCE_SIGNATURE", "../malicious");
        let result = HyprlandIPC::new().await;
        assert!(result.is_err(), "Should reject path traversal attempts");
        if let Err(e) = result {
            assert!(e.to_string().contains("unsafe characters"));
        }

        std::env::set_var("HYPRLAND_INSTANCE_SIGNATURE", "");
        let result = HyprlandIPC::new().await;
        assert!(result.is_err(), "Should reject empty signature");
        if let Err(e) = result {
            assert!(e.to_string().contains("empty signature"));
        }

        std::env::set_var("HYPRLAND_INSTANCE_SIGNATURE", "test/malicious");
        let result = HyprlandIPC::new().await;
        assert!(result.is_err(), "Should reject signature with slash");
        if let Err(e) = result {
            assert!(e.to_string().contains("unsafe characters"));
        }

        std::env::set_var("HYPRLAND_INSTANCE_SIGNATURE", "test$malicious");
        let result = HyprlandIPC::new().await;
        assert!(result.is_err(), "Should reject signature with special characters");

        std::env::set_var("HYPRLAND_INSTANCE_SIGNATURE", &"a".repeat(65));
        let result = HyprlandIPC::new().await;
        assert!(result.is_err(), "Should reject signature that's too long");

        for valid_sig in &["test123", "hypr_instance", "session-1", "a", "A1_-test"] {
            std::env::set_var("HYPRLAND_INSTANCE_SIGNATURE", valid_sig);
            let result = HyprlandIPC::new().await;
            if let Err(e) = result {
                assert!(e.to_string().contains("socket not found") || e.to_string().contains("No such file"));
            }
        }
    }

    #[test]
    fn test_instance_signature_validation() {
        assert!(validate_instance_signature("test123").is_ok());
        assert!(validate_instance_signature("hypr_instance").is_ok());
        assert!(validate_instance_signature("session-1").is_ok());
        assert!(validate_instance_signature("a").is_ok());
        assert!(validate_instance_signature("A1_-test").is_ok());
        assert!(validate_instance_signature("").is_err());
        assert!(validate_instance_signature("../malicious").is_err());
        assert!(validate_instance_signature("test/malicious").is_err());
        assert!(validate_instance_signature("test$malicious").is_err());
        assert!(validate_instance_signature("test malicious").is_err());
        assert!(validate_instance_signature("test\nmalicious").is_err());
        assert!(validate_instance_signature(&"a".repeat(65)).is_err());
    }
}
