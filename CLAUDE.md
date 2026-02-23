# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Local MCP Proxy is a Tauri 2 desktop app for managing multiple MCP (Model Context Protocol) servers. It provides a UI for connecting to, monitoring, and proxying requests to MCP servers via HTTP endpoints.

**Frontend**: Vue 3 + TypeScript + Pinia + Tailwind CSS, built with Vite
**Backend**: Rust (Tauri 2) with rmcp SDK, Axum HTTP server, Tokio async runtime

## Build & Development Commands

```bash
npm run tauri dev        # Run full app in development (frontend + backend)
npm run dev              # Frontend-only dev server (Vite, port 1420)
npm run build            # Build frontend (type-check + vite build)
npm run tauri build      # Production build of the full Tauri app
```

Rust backend is at `src-tauri/`. Use `cargo check --manifest-path src-tauri/Cargo.toml` for quick Rust compilation checks. No test suite exists yet.

## Architecture

### Frontend → Backend Communication

The frontend calls Rust functions via Tauri's `invoke()` IPC. Commands are defined in `src-tauri/src/commands.rs` and registered in `src-tauri/src/lib.rs`. The backend emits events (`mcp-statuses-changed`, `log-entry`) that the frontend listens to for real-time updates.

### Backend State

`AppState` (in `commands.rs`) holds three `Arc<Mutex<>>` components:
- **McpManager** (`mcp/manager.rs`) — orchestrates all MCP connections, handles health checks and auto-reconnect
- **ConfigManager** (`config.rs`) — persists settings to Tauri's `app_data_dir()/config.json` (on macOS: `~/Library/Application Support/com.github.velet5.localmcpproxy/config.json`)
- **LogStore** — circular buffer (500 entries) of tracing events, forwarded to frontend

### MCP Connections

`McpConnection` (`mcp/connection.rs`) wraps individual server connections with three transport types: Stdio, SSE, and StreamableHttp. Each connection is a state machine: Disconnected → Connecting → Connected/Error → Reconnecting.

### HTTP Proxy

An Axum server (`proxy/server.rs`, default port 3001) exposes each MCP server as HTTP endpoints. Key routes: `GET /mcps`, `GET /mcp/:id/tools`, `GET /mcp/:id/resources`, `POST /mcp/:id/message`, `DELETE /mcp/:id`.

### Type System

TypeScript types in `src/types/index.ts` mirror Rust types in `src-tauri/src/types.rs`. Both sides must stay in sync — changes to one require corresponding changes to the other.

### Frontend Routing

- `/` — Dashboard (all MCP cards)
- `/mcp/:id` — Server detail (tools, resources, config)
- `/add` — Add MCP form (`?edit=<id>` for editing)
- `/settings` — Global config
- `/logs` — Warning/error viewer

### Sidecar Binary

`src-tauri/src/bin/local-mcp-proxy-bridge.rs` is a sidecar binary. The build script (`src-tauri/build.rs`) handles copying it to platform-specific paths. On first builds, it creates a placeholder if the binary doesn't exist yet.
