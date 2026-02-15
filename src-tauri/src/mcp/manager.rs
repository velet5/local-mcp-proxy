use tauri::Emitter;
use crate::mcp::connection::McpConnection;
use crate::types::*;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time;

/// Central manager for all MCP connections
pub struct McpManager {
    connections: HashMap<String, Arc<McpConnection>>,
    config: AppConfig,
}

impl McpManager {
    /// Create a new manager with the given config
    pub fn new(config: AppConfig) -> Self {
        Self {
            connections: HashMap::new(),
            config,
        }
    }

    /// Initialize: connect all enabled MCPs from config
    pub async fn initialize(&mut self) {
        let configs: Vec<McpServerConfig> = self.config.mcps.clone();

        for mcp_config in configs {
            if !mcp_config.enabled {
                tracing::info!("MCP '{}' is disabled, skipping", mcp_config.name);
                continue;
            }

            let id = mcp_config.id.clone();
            let conn = Arc::new(McpConnection::new(mcp_config));

            match conn.connect().await {
                Ok(()) => {
                    tracing::info!("MCP '{}' connected successfully", conn.config.name);
                }
                Err(e) => {
                    tracing::warn!("MCP '{}' failed to connect: {}", conn.config.name, e);
                }
            }

            self.connections.insert(id, conn);
        }
    }

    /// Add a new MCP server
    pub async fn add_mcp(&mut self, config: McpServerConfig) -> Result<String> {
        let id = config.id.clone();

        // Check for duplicate
        if self.connections.contains_key(&id) {
            return Err(anyhow!("MCP with ID '{}' already exists", id));
        }

        let conn = Arc::new(McpConnection::new(config.clone()));

        // Attempt connection
        if config.enabled {
            if let Err(e) = conn.connect().await {
                tracing::warn!("New MCP '{}' failed initial connect: {}", config.name, e);
                // Still add it — user can retry
            }
        }

        self.connections.insert(id.clone(), conn);
        self.config.mcps.push(config);

        Ok(id)
    }

    /// Update an existing MCP's configuration
    pub async fn update_mcp(&mut self, config: McpServerConfig) -> Result<()> {
        let id = config.id.clone();

        // Disconnect old connection
        if let Some(old_conn) = self.connections.remove(&id) {
            old_conn.disconnect().await;
        }

        // Create new connection
        let conn = Arc::new(McpConnection::new(config.clone()));

        if config.enabled {
            if let Err(e) = conn.connect().await {
                tracing::warn!("Updated MCP '{}' failed to connect: {}", config.name, e);
            }
        }

        self.connections.insert(id.clone(), conn);

        // Update in config
        if let Some(pos) = self.config.mcps.iter().position(|m| m.id == id) {
            self.config.mcps[pos] = config;
        } else {
            self.config.mcps.push(config);
        }

        Ok(())
    }

    /// Remove an MCP server
    pub async fn remove_mcp(&mut self, id: &str) -> Result<()> {
        if let Some(conn) = self.connections.remove(id) {
            conn.disconnect().await;
        }
        self.config.mcps.retain(|m| m.id != id);
        Ok(())
    }

    /// Manually connect a specific MCP
    pub async fn connect_mcp(&self, id: &str) -> Result<()> {
        let conn = self
            .connections
            .get(id)
            .ok_or_else(|| anyhow!("MCP '{}' not found", id))?;

        conn.connect().await
    }

    /// Manually disconnect a specific MCP
    pub async fn disconnect_mcp(&self, id: &str) -> Result<()> {
        let conn = self
            .connections
            .get(id)
            .ok_or_else(|| anyhow!("MCP '{}' not found", id))?;

        conn.disconnect().await;
        Ok(())
    }

    /// Get status list of all MCPs
    pub async fn list_statuses(&self) -> Vec<McpStatus> {
        let mut statuses = Vec::new();
        for conn in self.connections.values() {
            statuses.push(conn.status(self.config.proxy_port).await);
        }
        // Sort by name for consistent ordering
        statuses.sort_by(|a, b| a.name.cmp(&b.name));
        statuses
    }

    /// Get full detail for a specific MCP
    pub async fn get_detail(&self, id: &str) -> Result<McpDetail> {
        let conn = self
            .connections
            .get(id)
            .ok_or_else(|| anyhow!("MCP '{}' not found", id))?;

        let status = conn.status(self.config.proxy_port).await;
        let tools = conn.get_tools().await;
        let resources = conn.get_resources().await;

        Ok(McpDetail {
            config: conn.config.clone(),
            status,
            tools,
            resources,
        })
    }

    /// Get a connection reference (for proxy use)
    pub fn get_connection(&self, id: &str) -> Option<Arc<McpConnection>> {
        self.connections.get(id).cloned()
    }

    /// Get current app config
    pub fn get_config(&self) -> &AppConfig {
        &self.config
    }

    /// Update app config (does not reconnect MCPs)
    pub fn update_config(&mut self, config: AppConfig) {
        self.config.proxy_port = config.proxy_port;
        self.config.health_check_interval_secs = config.health_check_interval_secs;
        self.config.auto_reconnect = config.auto_reconnect;
        self.config.max_reconnect_attempts = config.max_reconnect_attempts;
        // Don't overwrite mcps list — it's managed by add/update/remove
    }

    /// Get proxy URL for a specific MCP
    pub fn get_proxy_url(&self, id: &str) -> String {
        format!(
            "http://127.0.0.1:{}/mcp/{}/sse",
            self.config.proxy_port, id
        )
    }

    /// Run one health check cycle on all connections
    pub async fn health_check_cycle(&self) {
        for (id, conn) in &self.connections {
            let state = conn.get_state().await;

            match state {
                ConnectionState::Connected => {
                    // Ping to verify health
                    if let Err(e) = conn.ping().await {
                        tracing::warn!("MCP '{}' ping failed: {}", id, e);
                        // Will be picked up next cycle for reconnect
                    }
                }
                ConnectionState::Error | ConnectionState::Disconnected => {
                    // Try to reconnect if enabled and under max attempts
                    if self.config.auto_reconnect && conn.config.enabled {
                        let attempts = conn.get_reconnect_attempts().await;
                        if attempts < self.config.max_reconnect_attempts {
                            tracing::info!(
                                "MCP '{}': reconnect attempt {} of {}",
                                id,
                                attempts + 1,
                                self.config.max_reconnect_attempts
                            );
                            conn.increment_reconnect_attempts().await;

                            // Exponential backoff is handled by the health check interval
                            if let Err(e) = conn.connect().await {
                                tracing::warn!("MCP '{}' reconnect failed: {}", id, e);
                            }
                        }
                    }
                }
                _ => {
                    // Connecting/Reconnecting — skip
                }
            }
        }
    }

    /// Disconnect all MCPs (e.g. on app exit)
    pub async fn shutdown(&self) {
        for conn in self.connections.values() {
            conn.disconnect().await;
        }
        tracing::info!("All MCP connections shut down");
    }
}

/// Start the background health check loop
pub fn start_health_loop(
    manager: Arc<Mutex<McpManager>>,
    app_handle: tauri::AppHandle,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            let interval_secs = {
                let mgr = manager.lock().await;
                mgr.get_config().health_check_interval_secs
            };

            time::sleep(time::Duration::from_secs(interval_secs)).await;

            let mgr = manager.lock().await;
            mgr.health_check_cycle().await;

            // Emit updated statuses to the frontend
            let statuses = mgr.list_statuses().await;
            let _ = app_handle.emit("mcp-statuses-changed", &statuses);
        }
    });
}
