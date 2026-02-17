use crate::types::*;
use anyhow::{anyhow, Context, Result};
use rmcp::model::CallToolRequestParams;
use rmcp::service::RunningService;
use rmcp::transport::TokioChildProcess;
use rmcp::RoleClient;
use rmcp::ServiceExt;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::process::Command;
use tokio::sync::Mutex;

/// Represents a single MCP server connection
pub struct McpConnection {
    pub config: McpServerConfig,
    state: Arc<Mutex<ConnectionState>>,
    service: Arc<Mutex<Option<RunningService<RoleClient, ()>>>>,
    tools: Arc<Mutex<Vec<Tool>>>,
    resources: Arc<Mutex<Vec<Resource>>>,
    connected_at: Arc<Mutex<Option<SystemTime>>>,
    last_ping: Arc<Mutex<Option<SystemTime>>>,
    error_message: Arc<Mutex<Option<String>>>,
    reconnect_attempts: Arc<Mutex<u32>>,
}

impl McpConnection {
    /// Create a new connection (not yet connected)
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            service: Arc::new(Mutex::new(None)),
            tools: Arc::new(Mutex::new(Vec::new())),
            resources: Arc::new(Mutex::new(Vec::new())),
            connected_at: Arc::new(Mutex::new(None)),
            last_ping: Arc::new(Mutex::new(None)),
            error_message: Arc::new(Mutex::new(None)),
            reconnect_attempts: Arc::new(Mutex::new(0)),
        }
    }

    /// Get current connection state
    pub async fn get_state(&self) -> ConnectionState {
        *self.state.lock().await
    }

    /// Set connection state and update related fields
    async fn set_state(&self, new_state: ConnectionState) {
        let mut state = self.state.lock().await;
        tracing::info!(
            "MCP '{}': {:?} -> {:?}",
            self.config.name,
            *state,
            new_state
        );
        *state = new_state;

        match new_state {
            ConnectionState::Connected => {
                *self.connected_at.lock().await = Some(SystemTime::now());
                *self.error_message.lock().await = None;
                *self.reconnect_attempts.lock().await = 0;
            }
            ConnectionState::Disconnected => {
                *self.connected_at.lock().await = None;
            }
            _ => {}
        }
    }

    /// Set an error message
    async fn set_error(&self, msg: String) {
        *self.error_message.lock().await = Some(msg);
    }

    /// Get current reconnect attempts count
    pub async fn get_reconnect_attempts(&self) -> u32 {
        *self.reconnect_attempts.lock().await
    }

    /// Increment reconnect attempts
    pub async fn increment_reconnect_attempts(&self) {
        let mut attempts = self.reconnect_attempts.lock().await;
        *attempts += 1;
    }

    /// Attempt to connect to the MCP server
    pub async fn connect(&self) -> Result<()> {
        self.set_state(ConnectionState::Connecting).await;

        let result = match self.config.transport_type {
            TransportType::Stdio => self.connect_stdio().await,
            TransportType::Sse => self.connect_sse().await,
            TransportType::StreamableHttp => self.connect_http().await,
        };

        match result {
            Ok(()) => {
                // Fetch capabilities after connecting
                if let Err(e) = self.fetch_capabilities().await {
                    tracing::warn!(
                        "MCP '{}': Connected but failed to fetch capabilities: {}",
                        self.config.name,
                        e
                    );
                }
                self.set_state(ConnectionState::Connected).await;
                Ok(())
            }
            Err(e) => {
                let detailed = format!("{:#}", e);
                tracing::error!(
                    "MCP '{}': connect failed: {}",
                    self.config.name,
                    detailed
                );
                self.set_error(detailed).await;
                self.set_state(ConnectionState::Error).await;
                Err(e)
            }
        }
    }

    /// Connect via stdio (child process)
    async fn connect_stdio(&self) -> Result<()> {
        let command_str = self
            .config
            .command
            .as_ref()
            .ok_or_else(|| anyhow!("No command specified for stdio transport"))?
            .trim();

        if command_str.is_empty() {
            return Err(anyhow!("No command specified for stdio transport"));
        }

        // Split command: if user pasted "npx -y @foo/bar", use "npx" as executable and ["-y", "@foo/bar"] as args
        let (executable, extra_args) = if let Some(space) = command_str.find(' ') {
            let (exe, rest) = command_str.split_at(space);
            let rest_args: Vec<String> = rest
                .trim()
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            (exe.to_string(), rest_args)
        } else {
            (command_str.to_string(), Vec::new())
        };

        let mut args = self.config.args.clone().unwrap_or_default();
        args.splice(0..0, extra_args); // prepend extra_args to existing args

        // Build the command
        let mut cmd = Command::new(&executable);
        cmd.args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set environment variables if provided
        if let Some(env) = &self.config.env {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        let full_cmd = format!("{} {}", executable, args.join(" "))
            .trim_end()
            .to_string();
        let transport = TokioChildProcess::new(cmd)
            .map_err(|e| {
                anyhow!(
                    "Failed to spawn MCP server process (command: {}): {}",
                    full_cmd,
                    e
                )
            })?;

        let service = ().serve(transport)
            .await
            .context("Failed to initialize MCP client service")?;

        *self.service.lock().await = Some(service);
        Ok(())
    }

    /// Connect via legacy SSE transport (GET /sse + POST /messages)
    async fn connect_sse(&self) -> Result<()> {
        let url = self
            .config
            .url
            .as_ref()
            .ok_or_else(|| anyhow!("No URL specified for SSE transport"))?;

        use crate::mcp::legacy_sse::LegacySseWorker;
        use rmcp::transport::worker::WorkerTransport;

        let mut worker = LegacySseWorker::from_url(url.as_str())
            .map_err(|e| anyhow!("Invalid SSE URL: {}", e))?;

        // Pass custom headers from config (e.g. Authorization)
        if let Some(headers) = &self.config.headers {
            let header_vec: Vec<(String, String)> = headers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            worker = worker.with_headers(header_vec);
        }

        let transport = WorkerTransport::spawn(worker);

        let service = ().serve(transport)
            .await
            .context("Failed to initialize legacy SSE MCP client")?;

        *self.service.lock().await = Some(service);
        Ok(())
    }

    /// Connect via Streamable HTTP
    async fn connect_http(&self) -> Result<()> {
        let url = self
            .config
            .url
            .as_ref()
            .ok_or_else(|| anyhow!("No URL specified for HTTP transport"))?;

        use rmcp::transport::StreamableHttpClientTransport;
        use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;

        // Build a custom reqwest client with headers and no overall timeout
        // (the SSE stream is long-lived so we can't set a global timeout).
        // No read_timeout or timeout — SSE streams are long-lived and must not be killed.
        // reqwest has no read timeout by default, which is what we want.
        let mut client_builder = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90));

        // Apply custom headers from config (e.g. Authorization, cookies, etc.)
        if let Some(headers) = &self.config.headers {
            let mut header_map = reqwest::header::HeaderMap::new();
            for (key, value) in headers {
                if let (Ok(name), Ok(val)) = (
                    reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                    reqwest::header::HeaderValue::from_str(value),
                ) {
                    header_map.insert(name, val);
                } else {
                    tracing::warn!("MCP '{}': skipping invalid header: {}", self.config.name, key);
                }
            }
            client_builder = client_builder.default_headers(header_map);
        }

        let client = client_builder
            .build()
            .context("Failed to build HTTP client")?;

        let config = StreamableHttpClientTransportConfig::with_uri(url.as_str());
        let transport = StreamableHttpClientTransport::with_client(client, config);

        let service = ().serve(transport)
            .await
            .context("Failed to initialize HTTP MCP client")?;

        *self.service.lock().await = Some(service);
        Ok(())
    }

    /// Fetch tools and resources from the connected server
    async fn fetch_capabilities(&self) -> Result<()> {
        let service_lock = self.service.lock().await;
        let service = service_lock
            .as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?;

        // List tools
        match service.list_tools(Default::default()).await {
            Ok(result) => {
                let tools: Vec<Tool> = result
                    .tools
                    .into_iter()
                    .map(|t| Tool {
                        name: t.name.to_string(),
                        description: t.description.map(|d| d.to_string()),
                        input_schema: serde_json::to_value(&t.input_schema)
                            .unwrap_or(serde_json::Value::Object(Default::default())),
                    })
                    .collect();

                tracing::info!(
                    "MCP '{}': found {} tools",
                    self.config.name,
                    tools.len()
                );
                *self.tools.lock().await = tools;
            }
            Err(e) => {
                tracing::warn!(
                    "MCP '{}': failed to list tools: {}",
                    self.config.name,
                    e
                );
            }
        }

        // List resources
        match service.list_resources(Default::default()).await {
            Ok(result) => {
                let resources: Vec<Resource> = result
                    .resources
                    .into_iter()
                    .map(|r| Resource {
                        uri: r.uri.to_string(),
                        name: Some(r.name.to_string()),
                        description: r.description.clone().map(|d| d.to_string()),
                        mime_type: r.mime_type.clone().map(|m| m.to_string()),
                    })
                    .collect();

                tracing::info!(
                    "MCP '{}': found {} resources",
                    self.config.name,
                    resources.len()
                );
                *self.resources.lock().await = resources;
            }
            Err(e) => {
                tracing::warn!(
                    "MCP '{}': failed to list resources: {}",
                    self.config.name,
                    e
                );
            }
        }

        Ok(())
    }

    /// Ping the server for health check
    pub async fn ping(&self) -> Result<()> {
        let service_lock = self.service.lock().await;
        let service = service_lock
            .as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?;

        // Use list_tools as a lightweight health check (no dedicated ping in rmcp)
        let _ = service
            .list_tools(Default::default())
            .await
            .context("Health check failed")?;

        *self.last_ping.lock().await = Some(SystemTime::now());
        Ok(())
    }

    /// Disconnect from the server
    pub async fn disconnect(&self) {
        if let Some(service) = self.service.lock().await.take() {
            let _ = service.cancel().await;
        }
        *self.tools.lock().await = Vec::new();
        *self.resources.lock().await = Vec::new();
        self.set_state(ConnectionState::Disconnected).await;
    }

    /// Get current status snapshot
    pub async fn status(&self, proxy_port: u16) -> McpStatus {
        let state = *self.state.lock().await;
        let tools_count = self.tools.lock().await.len();
        let resources_count = self.resources.lock().await.len();
        let connected_at = *self.connected_at.lock().await;
        let last_ping = *self.last_ping.lock().await;
        let error_message = self.error_message.lock().await.clone();

        let uptime_seconds = connected_at.and_then(|t| {
            SystemTime::now()
                .duration_since(t)
                .ok()
                .map(|d| d.as_secs())
        });

        let proxy_url = if state == ConnectionState::Connected {
            Some(format!(
                "http://127.0.0.1:{}/mcp/{}",
                proxy_port, self.config.id
            ))
        } else {
            None
        };

        McpStatus {
            id: self.config.id.clone(),
            name: self.config.name.clone(),
            state,
            transport_type: self.config.transport_type.clone(),
            connected_at: connected_at.map(format_system_time),
            last_ping: last_ping.map(format_system_time),
            error_message,
            tools_count,
            resources_count,
            uptime_seconds,
            proxy_url,
        }
    }

    /// Get cached tools
    pub async fn get_tools(&self) -> Vec<Tool> {
        self.tools.lock().await.clone()
    }

    /// Get cached resources
    pub async fn get_resources(&self) -> Vec<Resource> {
        self.resources.lock().await.clone()
    }

    /// Execute a JSON-RPC method against the underlying MCP server.
    /// Returns the `result` value on success (not the full JSON-RPC envelope).
    pub async fn execute_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let service_lock = self.service.lock().await;
        let service = service_lock
            .as_ref()
            .ok_or_else(|| anyhow!("Not connected"))?;

        let result = match method {
            "ping" => {
                // rmcp doesn't expose a dedicated ping; use list_tools as a lightweight check
                let _ = service.list_tools(Default::default()).await.context("ping failed")?;
                serde_json::json!({})
            }
            "tools/list" => {
                let result = service
                    .list_tools(Default::default())
                    .await
                    .context("tools/list failed")?;
                serde_json::to_value(&result)?
            }
            "tools/call" => {
                let tool_params: CallToolRequestParams = serde_json::from_value(params)
                    .context("Invalid tools/call params")?;
                let result = service
                    .call_tool(tool_params)
                    .await
                    .context("tools/call failed")?;
                serde_json::to_value(&result)?
            }
            "resources/list" => {
                let result = service
                    .list_resources(Default::default())
                    .await
                    .context("resources/list failed")?;
                serde_json::to_value(&result)?
            }
            "resources/read" => {
                let read_params = serde_json::from_value(params)
                    .context("Invalid resources/read params")?;
                let result = service
                    .read_resource(read_params)
                    .await
                    .context("resources/read failed")?;
                serde_json::to_value(&result)?
            }
            "resources/templates/list" => {
                let result = service
                    .list_resource_templates(Default::default())
                    .await
                    .context("resources/templates/list failed")?;
                serde_json::to_value(&result)?
            }
            "prompts/list" => {
                let result = service
                    .list_prompts(Default::default())
                    .await
                    .context("prompts/list failed")?;
                serde_json::to_value(&result)?
            }
            "prompts/get" => {
                let prompt_params = serde_json::from_value(params)
                    .context("Invalid prompts/get params")?;
                let result = service
                    .get_prompt(prompt_params)
                    .await
                    .context("prompts/get failed")?;
                serde_json::to_value(&result)?
            }
            "completion/complete" => {
                let complete_params = serde_json::from_value(params)
                    .context("Invalid completion/complete params")?;
                let result = service
                    .complete(complete_params)
                    .await
                    .context("completion/complete failed")?;
                serde_json::to_value(&result)?
            }
            "logging/setLevel" => {
                let level_params = serde_json::from_value(params)
                    .context("Invalid logging/setLevel params")?;
                service
                    .set_level(level_params)
                    .await
                    .context("logging/setLevel failed")?;
                serde_json::json!({})
            }
            other => {
                return Err(anyhow!("Method not found: {}", other));
            }
        };

        Ok(result)
    }
}

fn format_system_time(time: SystemTime) -> String {
    let datetime: chrono::DateTime<chrono::Utc> = time.into();
    datetime.to_rfc3339()
}
