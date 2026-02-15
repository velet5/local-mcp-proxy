use crate::config::ConfigManager;
use crate::mcp::manager::McpManager;
use crate::types::*;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

/// Shared application state accessible to all commands
pub struct AppState {
    pub manager: Arc<Mutex<McpManager>>,
    pub config_manager: Arc<Mutex<ConfigManager>>,
}

/// Helper to persist config after any modification
async fn persist_config(state: &AppState) -> Result<(), String> {
    let mgr = state.manager.lock().await;
    let config = mgr.get_config().clone();
    let config_mgr = state.config_manager.lock().await;
    config_mgr.save(&config).map_err(|e| e.to_string())
}

/// List all MCPs with their current statuses
#[tauri::command]
pub async fn list_mcps(state: State<'_, AppState>) -> Result<Vec<McpStatus>, String> {
    let mgr = state.manager.lock().await;
    Ok(mgr.list_statuses().await)
}

/// Get full details (config, status, tools, resources) for a specific MCP
#[tauri::command]
pub async fn get_mcp_detail(id: String, state: State<'_, AppState>) -> Result<McpDetail, String> {
    let mgr = state.manager.lock().await;
    mgr.get_detail(&id).await.map_err(|e| e.to_string())
}

/// Add a new MCP server
#[tauri::command]
pub async fn add_mcp(
    config: McpServerConfig,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Validate
    if config.name.is_empty() {
        return Err("Name is required".to_string());
    }

    let id = {
        let mut mgr = state.manager.lock().await;
        mgr.add_mcp(config).await.map_err(|e| e.to_string())?
    };

    persist_config(&state).await?;
    Ok(id)
}

/// Update an existing MCP configuration
#[tauri::command]
pub async fn update_mcp(
    config: McpServerConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut mgr = state.manager.lock().await;
        mgr.update_mcp(config).await.map_err(|e| e.to_string())?;
    }

    persist_config(&state).await?;
    Ok(())
}

/// Remove an MCP server
#[tauri::command]
pub async fn remove_mcp(id: String, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut mgr = state.manager.lock().await;
        mgr.remove_mcp(&id).await.map_err(|e| e.to_string())?;
    }

    persist_config(&state).await?;
    Ok(())
}

/// Manually connect a specific MCP
#[tauri::command]
pub async fn connect_mcp(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mgr = state.manager.lock().await;
    mgr.connect_mcp(&id).await.map_err(|e| e.to_string())
}

/// Manually disconnect a specific MCP
#[tauri::command]
pub async fn disconnect_mcp(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mgr = state.manager.lock().await;
    mgr.disconnect_mcp(&id).await.map_err(|e| e.to_string())
}

/// Get the proxy URL for a specific MCP
#[tauri::command]
pub async fn get_proxy_url(id: String, state: State<'_, AppState>) -> Result<String, String> {
    let mgr = state.manager.lock().await;
    Ok(mgr.get_proxy_url(&id))
}

/// Get the global app configuration
#[tauri::command]
pub async fn get_app_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let mgr = state.manager.lock().await;
    Ok(mgr.get_config().clone())
}

/// Update the global app configuration
#[tauri::command]
pub async fn update_app_config(
    config: AppConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    ConfigManager::validate(&config)?;

    {
        let mut mgr = state.manager.lock().await;
        mgr.update_config(config.clone());
    }

    // Persist the full config (including mcps)
    let config_mgr = state.config_manager.lock().await;
    let mgr = state.manager.lock().await;
    let full_config = mgr.get_config().clone();
    config_mgr.save(&full_config).map_err(|e| e.to_string())?;

    Ok(())
}
