use crate::types::{AppConfig, TransportType};
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Manages loading and saving the JSON config file
pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    /// Create a new ConfigManager with the given path
    pub fn new(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    /// Initialize ConfigManager using the Tauri app data directory
    pub fn from_app_handle(app_handle: &tauri::AppHandle) -> Result<Self> {
        use tauri::Manager;
        let app_dir = app_handle
            .path()
            .app_data_dir()
            .context("Failed to resolve app data directory")?;

        let config_path = app_dir.join("config.json");
        Ok(Self::new(config_path))
    }

    /// Load config from disk, returning default if file doesn't exist
    pub fn load(&self) -> Result<AppConfig> {
        if !self.config_path.exists() {
            tracing::info!("Config file not found, using defaults");
            return Ok(AppConfig::default());
        }

        let data = std::fs::read_to_string(&self.config_path)
            .context("Failed to read config file")?;

        let config: AppConfig =
            serde_json::from_str(&data).context("Failed to parse config file")?;

        tracing::info!(
            "Loaded config with {} MCPs from {:?}",
            config.mcps.len(),
            self.config_path
        );

        Ok(config)
    }

    /// Save config to disk with atomic write
    pub fn save(&self, config: &AppConfig) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }

        let data = serde_json::to_string_pretty(config)
            .context("Failed to serialize config")?;

        std::fs::write(&self.config_path, data)
            .context("Failed to write config file")?;

        tracing::info!("Saved config to {:?}", self.config_path);
        Ok(())
    }

    /// Validate a config structure
    pub fn validate(config: &AppConfig) -> Result<(), String> {
        if config.proxy_port < 1024 {
            return Err("Proxy port must be >= 1024".to_string());
        }

        if config.health_check_interval_secs < 5 {
            return Err("Health check interval must be >= 5 seconds".to_string());
        }

        for mcp in &config.mcps {
            if mcp.id.is_empty() {
                return Err("MCP ID cannot be empty".to_string());
            }
            if mcp.name.is_empty() {
                return Err("MCP name cannot be empty".to_string());
            }

            match mcp.transport_type {
                TransportType::Stdio => {
                    if mcp.command.as_ref().map_or(true, |c| c.is_empty()) {
                        return Err(format!(
                            "MCP '{}': Stdio transport requires a command",
                            mcp.name
                        ));
                    }
                }
                TransportType::Sse | TransportType::StreamableHttp => {
                    if mcp.url.as_ref().map_or(true, |u| u.is_empty()) {
                        return Err(format!(
                            "MCP '{}': HTTP/SSE transport requires a URL",
                            mcp.name
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}
