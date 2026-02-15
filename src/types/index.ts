export enum TransportType {
  Stdio = "stdio",
  Sse = "sse",
  StreamableHttp = "streamable_http",
}

export enum ConnectionState {
  Disconnected = "disconnected",
  Connecting = "connecting",
  Connected = "connected",
  Error = "error",
  Reconnecting = "reconnecting",
}

export interface McpServerConfig {
  id: string;
  name: string;
  transport_type: TransportType;
  command?: string;
  args?: string[];
  url?: string;
  env?: Record<string, string>;
  headers?: Record<string, string>;
  enabled: boolean;
}

export interface McpStatus {
  id: string;
  name: string;
  state: ConnectionState;
  transport_type: TransportType;
  connected_at?: string;
  last_ping?: string;
  error_message?: string;
  tools_count: number;
  resources_count: number;
  uptime_seconds?: number;
  proxy_url?: string;
}

export interface Tool {
  name: string;
  description?: string;
  input_schema: Record<string, unknown>;
}

export interface Resource {
  uri: string;
  name?: string;
  description?: string;
  mime_type?: string;
}

export interface McpDetail {
  config: McpServerConfig;
  status: McpStatus;
  tools: Tool[];
  resources: Resource[];
}

export interface AppConfig {
  proxy_port: number;
  health_check_interval_secs: number;
  auto_reconnect: boolean;
  max_reconnect_attempts: number;
  mcps: McpServerConfig[];
}

export const CONNECTION_STATE_COLORS: Record<ConnectionState, string> = {
  [ConnectionState.Connected]: "emerald",
  [ConnectionState.Connecting]: "blue",
  [ConnectionState.Reconnecting]: "amber",
  [ConnectionState.Error]: "red",
  [ConnectionState.Disconnected]: "slate",
};

export const TRANSPORT_LABELS: Record<TransportType, string> = {
  [TransportType.Stdio]: "Stdio (Local Process)",
  [TransportType.Sse]: "Server-Sent Events",
  [TransportType.StreamableHttp]: "Streamable HTTP",
};
