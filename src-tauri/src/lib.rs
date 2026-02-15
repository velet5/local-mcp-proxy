mod commands;
mod config;
mod mcp;
mod proxy;
mod types;

use commands::AppState;
use tauri::Emitter;
use config::ConfigManager;
use mcp::manager::{McpManager, start_health_loop};
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

/// Main Tauri application setup
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("Starting MCP Hub");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Initialize config manager
            let config_manager = ConfigManager::from_app_handle(&app_handle)
                .expect("Failed to initialize config manager");

            // Load config
            let app_config = config_manager
                .load()
                .expect("Failed to load config");

            tracing::info!(
                "Loaded config: {} MCPs, proxy port {}",
                app_config.mcps.len(),
                app_config.proxy_port
            );

            let proxy_port = app_config.proxy_port;

            // Create MCP manager
            let manager = Arc::new(Mutex::new(McpManager::new(app_config)));
            let config_mgr = Arc::new(Mutex::new(config_manager));

            // Store app state
            app.manage(AppState {
                manager: Arc::clone(&manager),
                config_manager: Arc::clone(&config_mgr),
            });

            // Spawn initialization in background
            let mgr_init = Arc::clone(&manager);
            let handle_init = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                // Initialize all MCP connections
                {
                    let mut mgr = mgr_init.lock().await;
                    mgr.initialize().await;
                }

                // Emit initial statuses
                {
                    let mgr = mgr_init.lock().await;
                    let statuses = mgr.list_statuses().await;
                    let _ = handle_init.emit("mcp-statuses-changed", &statuses);
                }

                tracing::info!("MCP initialization complete");
            });

            // Start health check loop
            let mgr_health = Arc::clone(&manager);
            start_health_loop(mgr_health, app_handle.clone());

            // Start proxy server (HTTP)
            let mgr_proxy = Arc::clone(&manager);
            tauri::async_runtime::spawn(async move {
                if let Err(e) = proxy::server::start_proxy_server(proxy_port, mgr_proxy).await {
                    tracing::error!("Proxy server error: {}", e);
                }
            });

            tracing::info!("MCP Hub setup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_mcps,
            commands::get_mcp_detail,
            commands::add_mcp,
            commands::update_mcp,
            commands::remove_mcp,
            commands::connect_mcp,
            commands::disconnect_mcp,
            commands::get_proxy_url,
            commands::get_app_config,
            commands::update_app_config,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let manager = window.app_handle().state::<AppState>().manager.clone();
                tauri::async_runtime::spawn(async move {
                    let mgr = manager.lock().await;
                    mgr.shutdown().await;
                });
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running MCP Hub");
}
