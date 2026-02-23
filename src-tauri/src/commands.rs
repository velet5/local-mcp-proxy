use crate::config::ConfigManager;
use crate::mcp::manager::McpManager;
use crate::types::*;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::collections::VecDeque;
use tauri::State;
use tokio::sync::Mutex;

/// Shared application state accessible to all commands
pub struct AppState {
    pub manager: Arc<Mutex<McpManager>>,
    pub config_manager: Arc<Mutex<ConfigManager>>,
    pub log_store: Arc<StdMutex<VecDeque<LogEntry>>>,
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
    // Grab the connection Arc, then drop the manager lock before the potentially
    // long-running connect() call.  This prevents blocking all other commands
    // (list_mcps, get_mcp_detail, etc.) while a connection handshake is in progress.
    let conn = {
        let mgr = state.manager.lock().await;
        mgr.get_connection(&id)
            .ok_or_else(|| format!("MCP '{}' not found", id))?
    };
    conn.connect().await.map_err(|e| e.to_string())
}

/// Manually disconnect a specific MCP
#[tauri::command]
pub async fn disconnect_mcp(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let conn = {
        let mgr = state.manager.lock().await;
        mgr.get_connection(&id)
            .ok_or_else(|| format!("MCP '{}' not found", id))?
    };
    conn.disconnect().await;
    Ok(())
}

/// Update disabled tools/resources for a specific MCP
#[tauri::command]
pub async fn set_disabled_items(
    id: String,
    disabled_tools: Vec<String>,
    disabled_resources: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut mgr = state.manager.lock().await;
        mgr.set_disabled_items(&id, disabled_tools, disabled_resources)
            .map_err(|e| e.to_string())?;
    }
    persist_config(&state).await?;
    Ok(())
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
        mgr.update_config(config.clone()).await;
    }

    // Persist the full config (including mcps)
    let config_mgr = state.config_manager.lock().await;
    let mgr = state.manager.lock().await;
    let full_config = mgr.get_config().clone();
    config_mgr.save(&full_config).map_err(|e| e.to_string())?;

    Ok(())
}

/// Get recent log entries
#[tauri::command]
pub async fn get_logs(state: State<'_, AppState>) -> Result<Vec<LogEntry>, String> {
    let logs = state
        .log_store
        .lock()
        .map_err(|_| "Log buffer unavailable".to_string())?;
    Ok(logs.iter().cloned().collect())
}

/// Check if an MCP is already configured in Claude Desktop
#[tauri::command]
pub async fn check_claude_desktop(
    mcp_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let name = {
        let mgr = state.manager.lock().await;
        let config = mgr.get_config();
        config
            .mcps
            .iter()
            .find(|m| m.id == mcp_id)
            .ok_or("MCP not found")?
            .name
            .clone()
    };

    let config_path = claude_desktop_config_path()?;
    if !config_path.exists() {
        return Ok(false);
    }

    let content = std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
    let config: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| e.to_string())?;

    Ok(config
        .get("mcpServers")
        .and_then(|s| s.get(&name))
        .is_some())
}

/// Add an MCP to Claude Desktop's config via the bridge sidecar
#[tauri::command]
pub async fn add_to_claude_desktop(
    mcp_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (name, port) = get_mcp_name_and_port(&mcp_id, &state).await?;
    let bridge_path = find_bridge_binary()?;
    let config_path = claude_desktop_config_path()?;

    let mut config = read_claude_desktop_config(&config_path)?;

    // Ensure mcpServers object exists
    if config.get("mcpServers").is_none() {
        config["mcpServers"] = serde_json::json!({});
    }

    if config["mcpServers"].get(&name).is_some() {
        return Err("Already added to Claude Desktop".to_string());
    }

    config["mcpServers"][&name] = serde_json::json!({
        "command": bridge_path,
        "args": ["--mcp-id", &mcp_id, "--port", &port.to_string()]
    });

    write_claude_desktop_config(&config_path, &config)?;
    Ok(())
}

/// Update an MCP entry in Claude Desktop's config
#[tauri::command]
pub async fn update_in_claude_desktop(
    mcp_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (name, port) = get_mcp_name_and_port(&mcp_id, &state).await?;
    let bridge_path = find_bridge_binary()?;
    let config_path = claude_desktop_config_path()?;

    let mut config = read_claude_desktop_config(&config_path)?;

    if config.get("mcpServers").is_none() {
        config["mcpServers"] = serde_json::json!({});
    }

    config["mcpServers"][&name] = serde_json::json!({
        "command": bridge_path,
        "args": ["--mcp-id", &mcp_id, "--port", &port.to_string()]
    });

    write_claude_desktop_config(&config_path, &config)?;
    Ok(())
}

/// Remove an MCP from Claude Desktop's config
#[tauri::command]
pub async fn remove_from_claude_desktop(
    mcp_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let name = {
        let mgr = state.manager.lock().await;
        let config = mgr.get_config();
        config
            .mcps
            .iter()
            .find(|m| m.id == mcp_id)
            .ok_or("MCP not found")?
            .name
            .clone()
    };

    let config_path = claude_desktop_config_path()?;
    if !config_path.exists() {
        return Err("Claude Desktop config not found".to_string());
    }

    let mut config = read_claude_desktop_config(&config_path)?;

    let removed = config
        .get_mut("mcpServers")
        .and_then(|s| s.as_object_mut())
        .map(|servers| servers.remove(&name).is_some())
        .unwrap_or(false);

    if !removed {
        return Err("MCP not found in Claude Desktop config".to_string());
    }

    write_claude_desktop_config(&config_path, &config)?;
    Ok(())
}

async fn get_mcp_name_and_port(
    mcp_id: &str,
    state: &State<'_, AppState>,
) -> Result<(String, u16), String> {
    let mgr = state.manager.lock().await;
    let config = mgr.get_config();
    let mcp = config
        .mcps
        .iter()
        .find(|m| m.id == mcp_id)
        .ok_or("MCP not found")?;
    Ok((mcp.name.clone(), config.proxy_port))
}

fn read_claude_desktop_config(
    config_path: &std::path::Path,
) -> Result<serde_json::Value, String> {
    if config_path.exists() {
        let content = std::fs::read_to_string(config_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    } else {
        Ok(serde_json::json!({}))
    }
}

fn write_claude_desktop_config(
    config_path: &std::path::Path,
    config: &serde_json::Value,
) -> Result<(), String> {
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(config_path, content).map_err(|e| e.to_string())?;
    Ok(())
}

fn claude_desktop_config_path() -> Result<std::path::PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    Ok(std::path::PathBuf::from(home)
        .join("Library/Application Support/Claude/claude_desktop_config.json"))
}

fn find_bridge_binary() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().ok_or("cannot resolve binary directory")?;

    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with("local-mcp-proxy-bridge") && !name_str.contains('.') {
            return Ok(entry.path().to_string_lossy().to_string());
        }
    }

    Err("local-mcp-proxy-bridge binary not found next to the running executable".to_string())
}
