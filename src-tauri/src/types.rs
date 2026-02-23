use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Transport type for connecting to an MCP server
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransportType {
    Stdio,
    Sse,
    StreamableHttp,
}

/// Connection state machine
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error,
    Reconnecting,
}

/// Configuration for a single MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    pub transport_type: TransportType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub disabled_tools: Vec<String>,
    #[serde(default)]
    pub disabled_resources: Vec<String>,
}

fn default_true() -> bool {
    true
}

/// Status snapshot for a single MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpStatus {
    pub id: String,
    pub name: String,
    pub state: ConnectionState,
    pub transport_type: TransportType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_ping: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub tools_count: usize,
    pub resources_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
}

/// Tool metadata from an MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

/// Resource metadata from an MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Full details for a single MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpDetail {
    pub config: McpServerConfig,
    pub status: McpStatus,
    pub tools: Vec<Tool>,
    pub resources: Vec<Resource>,
}

/// Application-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_proxy_port")]
    pub proxy_port: u16,
    #[serde(default = "default_health_interval")]
    pub health_check_interval_secs: u64,
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,
    #[serde(default = "default_max_reconnect")]
    pub max_reconnect_attempts: u32,
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_secs: u64,
    #[serde(default)]
    pub mcps: Vec<McpServerConfig>,
}

/// Log entry captured from tracing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

fn default_proxy_port() -> u16 {
    3001
}

fn default_health_interval() -> u64 {
    30
}

fn default_max_reconnect() -> u32 {
    5
}

fn default_connection_timeout() -> u64 {
    30
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            proxy_port: default_proxy_port(),
            health_check_interval_secs: default_health_interval(),
            auto_reconnect: true,
            max_reconnect_attempts: default_max_reconnect(),
            connection_timeout_secs: default_connection_timeout(),
            mcps: Vec::new(),
        }
    }
}
