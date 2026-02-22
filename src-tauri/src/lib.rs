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
use std::collections::VecDeque;
use std::sync::Mutex as StdMutex;
use tracing::Subscriber;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;
use tracing_subscriber::EnvFilter;
use crate::types::LogEntry;

const LOG_BUFFER_CAPACITY: usize = 500;

struct LogLayer {
    store: Arc<StdMutex<VecDeque<LogEntry>>>,
    emitter: Arc<StdMutex<Option<tauri::AppHandle>>>,
}

impl LogLayer {
    fn push_entry(&self, entry: LogEntry) {
        if let Ok(mut logs) = self.store.lock() {
            if logs.len() >= LOG_BUFFER_CAPACITY {
                logs.pop_front();
            }
            logs.push_back(entry.clone());
        }

        if let Ok(handle_guard) = self.emitter.lock() {
            if let Some(handle) = handle_guard.as_ref() {
                let _ = handle.emit("log-entry", &entry);
            }
        }
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: Option<String>,
    fields: Vec<(String, String)>,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value).trim_matches('"').to_string());
        } else {
            self.fields
                .push((field.name().to_string(), format!("{:?}", value)));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields
                .push((field.name().to_string(), value.to_string()));
        }
    }
}

impl<S> Layer<S> for LogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let mut message = visitor.message.unwrap_or_else(|| "log event".to_string());
        if !visitor.fields.is_empty() {
            let extras = visitor
                .fields
                .into_iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(" ");
            message = format!("{} | {}", message, extras);
        }

        let entry = LogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            level: event.metadata().level().to_string(),
            target: event.metadata().target().to_string(),
            message,
        };

        self.push_entry(entry);
    }
}

/// Main Tauri application setup
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_store = Arc::new(StdMutex::new(VecDeque::with_capacity(LOG_BUFFER_CAPACITY)));
    let log_emitter = Arc::new(StdMutex::new(None));

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer();
    let log_layer = LogLayer {
        store: Arc::clone(&log_store),
        emitter: Arc::clone(&log_emitter),
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(log_layer)
        .init();

    tracing::info!("Starting Local MCP Proxy");

    let log_store = Arc::clone(&log_store);
    let log_emitter = Arc::clone(&log_emitter);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
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

            if let Ok(mut handle_guard) = log_emitter.lock() {
                *handle_guard = Some(app_handle.clone());
            }

            // Store app state
            app.manage(AppState {
                manager: Arc::clone(&manager),
                config_manager: Arc::clone(&config_mgr),
                log_store: Arc::clone(&log_store),
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

            tracing::info!("Local MCP Proxy setup complete");
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
            commands::set_disabled_items,
            commands::get_proxy_url,
            commands::get_app_config,
            commands::update_app_config,
            commands::get_logs,
            commands::check_claude_desktop,
            commands::add_to_claude_desktop,
            commands::update_in_claude_desktop,
            commands::remove_from_claude_desktop,
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
        .expect("error while running Local MCP Proxy");
}
