use crate::mcp::manager::McpManager;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Json,
    },
    routing::{get, post},
    Router,
};
use futures::stream::{self, Stream};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tower_http::cors::{Any, CorsLayer};

/// Shared state for the proxy server
#[derive(Clone)]
pub struct ProxyState {
    pub manager: Arc<Mutex<McpManager>>,
}

/// Create the Axum router for the proxy server
pub fn create_router(manager: Arc<Mutex<McpManager>>) -> Router {
    let state = ProxyState { manager };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_check))
        .route("/mcps", get(list_mcps))
        .route("/mcp/:id/sse", get(sse_endpoint))
        .route("/mcp/:id/message", post(forward_message))
        .route("/mcp/:id/tools", get(list_tools))
        .route("/mcp/:id/resources", get(list_resources))
        .layer(cors)
        .with_state(state)
}

/// Start the proxy server on the given port with HTTP
pub async fn start_proxy_server(
    port: u16,
    manager: Arc<Mutex<McpManager>>,
) -> anyhow::Result<()> {
    let app = create_router(manager);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Starting HTTP proxy server on http://127.0.0.1:{}", port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// GET /health — Overall health check
async fn health_check(
    State(state): State<ProxyState>,
) -> impl IntoResponse {
    let mgr = state.manager.lock().await;
    let statuses = mgr.list_statuses().await;
    let connected = statuses.iter().filter(|s| s.state == crate::types::ConnectionState::Connected).count();

    Json(serde_json::json!({
        "status": "ok",
        "total_mcps": statuses.len(),
        "connected_mcps": connected,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// GET /mcps — List all MCPs and their statuses
async fn list_mcps(
    State(state): State<ProxyState>,
) -> impl IntoResponse {
    let mgr = state.manager.lock().await;
    let statuses = mgr.list_statuses().await;
    Json(statuses)
}

/// GET /mcp/:id/sse — SSE endpoint that proxies MCP events
///
/// This implements the MCP SSE transport server-side:
/// - On connect, sends an `endpoint` event with the POST URL for messages
/// - Keeps the connection alive with periodic pings
/// - Relays tool call results and notifications
async fn sse_endpoint(
    Path(id): Path<String>,
    State(state): State<ProxyState>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let mgr = state.manager.lock().await;

    // Verify the MCP exists and is connected
    let conn = mgr
        .get_connection(&id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let mcp_state = conn.get_state().await;
    if mcp_state != crate::types::ConnectionState::Connected {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let config = mgr.get_config();
    let port = config.proxy_port;
    let mcp_name = conn.config.name.clone();

    // Build the SSE stream
    // First event: the endpoint URL for POSTing messages
    let endpoint_url = format!("http://127.0.0.1:{}/mcp/{}/message", port, id);

    let tools = conn.get_tools().await;
    let resources = conn.get_resources().await;

    let initial_events = vec![
        Ok(Event::default()
            .event("endpoint")
            .data(endpoint_url)),
        Ok(Event::default()
            .event("meta")
            .data(
                serde_json::json!({
                    "name": mcp_name,
                    "tools_count": tools.len(),
                    "resources_count": resources.len(),
                    "status": "connected"
                })
                .to_string(),
            )),
    ];

    // Create a stream of initial events, then keep alive with periodic updates
    let event_stream = stream::iter(initial_events);

    // Periodic keep-alive pings every 15 seconds
    let ping_stream = tokio_stream::wrappers::IntervalStream::new(
        tokio::time::interval(std::time::Duration::from_secs(15)),
    )
    .map(|_| {
        Ok(Event::default()
            .event("ping")
            .data(chrono::Utc::now().to_rfc3339()))
    });

    let combined = event_stream.chain(ping_stream);

    Ok(Sse::new(combined).keep_alive(KeepAlive::default()))
}

/// POST /mcp/:id/message — Forward a JSON-RPC message to the MCP
async fn forward_message(
    Path(id): Path<String>,
    State(state): State<ProxyState>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, StatusCode> {
    let mgr = state.manager.lock().await;

    let conn = mgr
        .get_connection(&id)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Extract method and params from JSON-RPC message
    let method = body
        .get("method")
        .and_then(|m| m.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let params = body.get("params").cloned().unwrap_or(serde_json::Value::Null);
    let request_id = body.get("id").cloned();

    let result = match method {
        "tools/call" => {
            let tool_name = params
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or(StatusCode::BAD_REQUEST)?;

            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or(serde_json::Value::Object(Default::default()));

            match conn.call_tool(tool_name, arguments).await {
                Ok(result) => serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": result
                }),
                Err(e) => serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "error": {
                        "code": -32000,
                        "message": format!("{}", e)
                    }
                }),
            }
        }
        "ping" => {
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": {}
            })
        }
        _ => {
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "error": {
                    "code": -32601,
                    "message": format!("Method '{}' not supported via proxy", method)
                }
            })
        }
    };

    Ok(Json(result))
}

/// GET /mcp/:id/tools — List tools for a specific MCP
async fn list_tools(
    Path(id): Path<String>,
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, StatusCode> {
    let mgr = state.manager.lock().await;

    let conn = mgr
        .get_connection(&id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let tools = conn.get_tools().await;
    Ok(Json(tools))
}

/// GET /mcp/:id/resources — List resources for a specific MCP
async fn list_resources(
    Path(id): Path<String>,
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, StatusCode> {
    let mgr = state.manager.lock().await;

    let conn = mgr
        .get_connection(&id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let resources = conn.get_resources().await;
    Ok(Json(resources))
}
