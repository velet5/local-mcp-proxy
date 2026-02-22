use crate::mcp::connection::McpConnection;
use crate::mcp::manager::McpManager;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
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
        .route(
            "/mcp/:id",
            get(streamable_http_get)
                .post(streamable_http_post)
                .delete(streamable_http_delete),
        )
        .route("/mcp/:id/tools", get(list_tools))
        .route("/mcp/:id/resources", get(list_resources))
        .layer(cors)
        .with_state(state)
}

/// Start the proxy server on the given port
pub async fn start_proxy_server(
    port: u16,
    manager: Arc<Mutex<McpManager>>,
) -> anyhow::Result<()> {
    let app = create_router(manager);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Starting MCP Streamable HTTP proxy on http://127.0.0.1:{}", port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Health & discovery endpoints
// ---------------------------------------------------------------------------

/// GET /health
async fn health_check(State(state): State<ProxyState>) -> impl IntoResponse {
    let mgr = state.manager.lock().await;
    let statuses = mgr.list_statuses().await;
    let connected = statuses
        .iter()
        .filter(|s| s.state == crate::types::ConnectionState::Connected)
        .count();

    Json(serde_json::json!({
        "status": "ok",
        "total_mcps": statuses.len(),
        "connected_mcps": connected,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// GET /mcps
async fn list_mcps(State(state): State<ProxyState>) -> impl IntoResponse {
    let mgr = state.manager.lock().await;
    let statuses = mgr.list_statuses().await;
    Json(statuses)
}

// ---------------------------------------------------------------------------
// MCP Streamable HTTP transport  (spec 2025-03-26)
// ---------------------------------------------------------------------------

/// GET /mcp/:id — Open SSE stream for server-initiated notifications.
/// Per the Streamable HTTP spec this is optional; we return 405 for now
/// since we don't relay server notifications yet.
async fn streamable_http_get(
    Path(id): Path<String>,
    State(state): State<ProxyState>,
) -> StatusCode {
    let mgr = state.manager.lock().await;
    let Some(conn) = mgr.get_connection(&id) else {
        return StatusCode::NOT_FOUND;
    };

    let mcp_state = conn.get_state().await;
    if mcp_state != crate::types::ConnectionState::Connected {
        return StatusCode::SERVICE_UNAVAILABLE;
    }

    // The Streamable HTTP spec says GET is for server-initiated messages.
    // We don't proxy those yet, so return 405 Method Not Allowed.
    StatusCode::METHOD_NOT_ALLOWED
}

/// POST /mcp/:id — Main JSON-RPC endpoint.
/// Accepts a single JSON-RPC request object or a batch (JSON array).
/// Returns `application/json` with the JSON-RPC response(s), or 202 for
/// pure notification messages (no `id` field).
async fn streamable_http_post(
    Path(id): Path<String>,
    State(state): State<ProxyState>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, StatusCode> {
    let mgr = state.manager.lock().await;
    let conn = mgr.get_connection(&id).ok_or(StatusCode::NOT_FOUND)?;
    let disabled = mgr.get_disabled_items(&id);

    // Batch request
    if let Some(requests) = body.as_array() {
        let mut responses = Vec::new();
        for req in requests {
            if let Some(resp) = handle_single_request(req, &conn, &disabled).await {
                responses.push(resp);
            }
        }
        if responses.is_empty() {
            return Ok(StatusCode::ACCEPTED.into_response());
        }
        return Ok(Json(serde_json::Value::Array(responses)).into_response());
    }

    // Single request
    match handle_single_request(&body, &conn, &disabled).await {
        Some(resp) => Ok(Json(resp).into_response()),
        None => Ok(StatusCode::ACCEPTED.into_response()),
    }
}

/// DELETE /mcp/:id — Session termination (acknowledge and no-op).
async fn streamable_http_delete(
    Path(id): Path<String>,
    State(state): State<ProxyState>,
) -> StatusCode {
    let mgr = state.manager.lock().await;
    if mgr.get_connection(&id).is_some() {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

/// Dispatch a single JSON-RPC request object.
/// Returns `None` for notifications (requests without an `id`).
async fn handle_single_request(
    request: &serde_json::Value,
    conn: &McpConnection,
    disabled: &(Vec<String>, Vec<String>),
) -> Option<serde_json::Value> {
    let method = request.get("method")?.as_str()?;
    let params = request
        .get("params")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let id = request.get("id").cloned();

    // JSON-RPC notifications have no `id` — no response expected
    if id.is_none() {
        return None;
    }

    // `initialize` is handled by the proxy itself (we are the MCP server here)
    if method == "initialize" {
        return Some(serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2025-03-26",
                "capabilities": {
                    "tools": { "listChanged": false },
                    "resources": { "subscribe": false, "listChanged": false },
                    "prompts": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "Local MCP Proxy",
                    "version": "0.1.0"
                }
            }
        }));
    }

    // Forward everything else to the underlying MCP server
    match conn.execute_request(method, params).await {
        Ok(mut result) => {
            // Filter disabled tools from tools/list responses
            if method == "tools/list" {
                if let Some(tools) = result.get_mut("tools").and_then(|t| t.as_array_mut()) {
                    tools.retain(|t| {
                        t.get("name")
                            .and_then(|n| n.as_str())
                            .map(|name| !disabled.0.contains(&name.to_string()))
                            .unwrap_or(true)
                    });
                }
            }
            // Filter disabled resources from resources/list responses
            if method == "resources/list" {
                if let Some(resources) = result.get_mut("resources").and_then(|r| r.as_array_mut()) {
                    resources.retain(|r| {
                        r.get("uri")
                            .and_then(|u| u.as_str())
                            .map(|uri| !disabled.1.contains(&uri.to_string()))
                            .unwrap_or(true)
                    });
                }
            }
            Some(serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result
            }))
        }
        Err(e) => {
            let code = if e.to_string().contains("Method not found") {
                -32601 // Method not found
            } else {
                -32000 // Server error
            };
            Some(serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": code,
                    "message": format!("{}", e)
                }
            }))
        }
    }
}

// ---------------------------------------------------------------------------
// Convenience endpoints (non-MCP-transport)
// ---------------------------------------------------------------------------

/// GET /mcp/:id/tools
async fn list_tools(
    Path(id): Path<String>,
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, StatusCode> {
    let mgr = state.manager.lock().await;
    let conn = mgr.get_connection(&id).ok_or(StatusCode::NOT_FOUND)?;
    let (disabled_tools, _) = mgr.get_disabled_items(&id);
    let tools: Vec<_> = conn
        .get_tools()
        .await
        .into_iter()
        .filter(|t| !disabled_tools.contains(&t.name))
        .collect();
    Ok(Json(tools))
}

/// GET /mcp/:id/resources
async fn list_resources(
    Path(id): Path<String>,
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, StatusCode> {
    let mgr = state.manager.lock().await;
    let conn = mgr.get_connection(&id).ok_or(StatusCode::NOT_FOUND)?;
    let (_, disabled_resources) = mgr.get_disabled_items(&id);
    let resources: Vec<_> = conn
        .get_resources()
        .await
        .into_iter()
        .filter(|r| !disabled_resources.contains(&r.uri))
        .collect();
    Ok(Json(resources))
}
