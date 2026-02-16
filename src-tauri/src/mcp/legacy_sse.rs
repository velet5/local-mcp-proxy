//! Legacy SSE transport for old-style MCP servers.
//!
//! Old-style SSE MCP servers work like this:
//!   1. Client opens `GET <base_url>/sse` → server sends SSE events
//!   2. Server sends an `endpoint` event with a relative URL like `/messages?sessionId=xxx`
//!   3. Client sends JSON-RPC requests via `POST <base_url><endpoint>`
//!   4. Server sends JSON-RPC responses/notifications via the SSE stream

use std::borrow::Cow;

use futures::StreamExt;
use reqwest::Client;
use rmcp::{
    RoleClient,
    model::ServerJsonRpcMessage,
    transport::worker::{Worker, WorkerConfig, WorkerContext, WorkerQuitReason, WorkerSendRequest},
};
use thiserror::Error;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Error)]
pub enum LegacySseError {
    #[error("HTTP request failed: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("SSE stream ended before receiving endpoint")]
    NoEndpoint,
    #[error("SSE stream ended unexpectedly")]
    StreamEnded,
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Transport channel closed")]
    ChannelClosed,
    #[error("Tokio join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

pub struct LegacySseWorker {
    /// The base URL of the SSE server (e.g. "http://host:port")
    base_url: String,
    /// The SSE endpoint path (e.g. "/sse")
    sse_path: String,
    /// Optional extra headers
    headers: Vec<(String, String)>,
}

impl LegacySseWorker {
    /// Create from a full SSE URL like "http://host:port/sse"
    pub fn from_url(url: &str) -> Result<Self, LegacySseError> {
        let parsed = reqwest::Url::parse(url)
            .map_err(|e| LegacySseError::InvalidUrl(format!("{}: {}", url, e)))?;

        let base_url = format!(
            "{}://{}{}",
            parsed.scheme(),
            parsed.host_str().unwrap_or("localhost"),
            parsed
                .port()
                .map(|p| format!(":{}", p))
                .unwrap_or_default()
        );
        let sse_path = parsed.path().to_string();

        Ok(Self {
            base_url,
            sse_path,
            headers: Vec::new(),
        })
    }

    #[allow(dead_code)]
    pub fn with_headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.headers = headers;
        self
    }

    fn full_url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            format!("{}{}", self.base_url, path)
        }
    }
}

impl Worker for LegacySseWorker {
    type Role = RoleClient;
    type Error = LegacySseError;

    fn err_closed() -> Self::Error {
        LegacySseError::ChannelClosed
    }

    fn err_join(e: tokio::task::JoinError) -> Self::Error {
        LegacySseError::JoinError(e)
    }

    fn config(&self) -> WorkerConfig {
        WorkerConfig {
            name: Some("LegacySseWorker".to_string()),
            channel_buffer_capacity: 16,
        }
    }

    async fn run(
        self,
        mut context: WorkerContext<Self>,
    ) -> Result<(), WorkerQuitReason<Self::Error>> {
        let client = Client::new();
        let ct = context.cancellation_token.clone();

        // Step 1: Open the SSE stream
        tracing::info!("Legacy SSE: connecting to {}{}", self.base_url, self.sse_path);

        let sse_url = self.full_url(&self.sse_path);
        let mut request = client.get(&sse_url);
        for (key, value) in &self.headers {
            request = request.header(key.as_str(), value.as_str());
        }

        let response = request
            .send()
            .await
            .map_err(|e| WorkerQuitReason::fatal(LegacySseError::Reqwest(e), "open SSE stream"))?;

        if !response.status().is_success() {
            return Err(WorkerQuitReason::fatal(
                LegacySseError::InvalidUrl(format!(
                    "SSE endpoint returned status {}",
                    response.status()
                )),
                "open SSE stream",
            ));
        }

        // Step 2: Read SSE events to find the endpoint
        let mut sse_stream = sse_stream::SseStream::from_byte_stream(response.bytes_stream());

        let messages_endpoint: Option<String>;

        tracing::info!("Legacy SSE: waiting for endpoint event...");
        loop {
            tokio::select! {
                _ = ct.cancelled() => {
                    return Err(WorkerQuitReason::Cancelled);
                }
                event = sse_stream.next() => {
                    match event {
                        Some(Ok(sse_event)) => {
                            let event_type = sse_event.event.as_deref().unwrap_or("message");
                            tracing::debug!("Legacy SSE: got event type='{}', data={:?}", event_type, sse_event.data);

                            if event_type == "endpoint" {
                                if let Some(data) = sse_event.data {
                                    let data: String = data;
                                    let endpoint = data.trim().to_string();
                                    tracing::info!("Legacy SSE: received endpoint: {}", endpoint);
                                    messages_endpoint = Some(endpoint);
                                    break;
                                }
                            }
                        }
                        Some(Err(e)) => {
                            tracing::error!("Legacy SSE: error reading SSE stream: {}", e);
                            return Err(WorkerQuitReason::fatal(
                                LegacySseError::StreamEnded,
                                format!("SSE stream error waiting for endpoint: {}", e),
                            ));
                        }
                        None => {
                            return Err(WorkerQuitReason::fatal(
                                LegacySseError::NoEndpoint,
                                "SSE stream ended before endpoint event",
                            ));
                        }
                    }
                }
            }
        }

        let messages_url = self.full_url(
            messages_endpoint
                .as_deref()
                .ok_or_else(|| WorkerQuitReason::fatal(LegacySseError::NoEndpoint, "no endpoint"))?,
        );
        tracing::info!("Legacy SSE: POST endpoint is {}", messages_url);

        // Step 3: Forward the initialize request from rmcp
        let WorkerSendRequest {
            message: init_request,
            responder: init_responder,
        } = context.recv_from_handler().await?;

        let init_body = serde_json::to_string(&init_request).map_err(|e| {
            WorkerQuitReason::fatal(LegacySseError::Json(e), "serialize initialize request")
        })?;

        tracing::debug!("Legacy SSE: sending initialize: {}", init_body);

        match client
            .post(&messages_url)
            .header("Content-Type", "application/json")
            .body(init_body)
            .send()
            .await
        {
            Ok(_resp) => {
                let _ = init_responder.send(Ok(()));
            }
            Err(e) => {
                let msg = format!("initialize POST failed: {}", e);
                let _ = init_responder.send(Err(LegacySseError::Reqwest(e)));
                return Err(WorkerQuitReason::fatal(
                    LegacySseError::ChannelClosed,
                    msg,
                ));
            }
        }

        // Read the initialize response from the SSE stream
        let init_response = Self::read_next_jsonrpc(&mut sse_stream, &ct).await?;
        context.send_to_handler(init_response).await?;

        // Step 4: Forward the initialized notification
        let WorkerSendRequest {
            message: initialized_notification,
            responder: initialized_responder,
        } = context.recv_from_handler().await?;

        let notif_body = serde_json::to_string(&initialized_notification).map_err(|e| {
            WorkerQuitReason::fatal(LegacySseError::Json(e), "serialize initialized notification")
        })?;

        tracing::debug!("Legacy SSE: sending initialized notification: {}", notif_body);

        let _ = client
            .post(&messages_url)
            .header("Content-Type", "application/json")
            .body(notif_body)
            .send()
            .await
            .map_err(|e| {
                WorkerQuitReason::fatal(LegacySseError::Reqwest(e), "send initialized notification")
            })?;
        let _ = initialized_responder.send(Ok(()));

        // Step 5: Main event loop
        let (sse_tx, mut sse_rx) = tokio::sync::mpsc::channel::<ServerJsonRpcMessage>(16);

        // Spawn SSE reader task
        let sse_ct = ct.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = sse_ct.cancelled() => break,
                    event = sse_stream.next() => {
                        match event {
                            Some(Ok(sse_event)) => {
                                let event_type = sse_event.event.as_deref().unwrap_or("message");
                                if event_type == "message" {
                                    if let Some(data) = sse_event.data {
                                        let data: String = data;
                                        let trimmed = data.trim();
                                        if trimmed.is_empty() {
                                            continue;
                                        }
                                        match serde_json::from_str::<ServerJsonRpcMessage>(trimmed) {
                                            Ok(msg) => {
                                                if sse_tx.send(msg).await.is_err() {
                                                    tracing::debug!("Legacy SSE: handler dropped, stopping SSE reader");
                                                    break;
                                                }
                                            }
                                            Err(e) => {
                                                tracing::warn!("Legacy SSE: failed to parse SSE message: {} — data: {}", e, trimmed);
                                            }
                                        }
                                    }
                                } else if event_type == "endpoint" {
                                    // Ignore duplicate endpoint events
                                } else {
                                    tracing::debug!("Legacy SSE: ignoring event type '{}'", event_type);
                                }
                            }
                            Some(Err(e)) => {
                                tracing::error!("Legacy SSE: SSE stream error: {}", e);
                                break;
                            }
                            None => {
                                tracing::info!("Legacy SSE: SSE stream ended");
                                break;
                            }
                        }
                    }
                }
            }
        });

        // Main loop: forward messages between rmcp handler and SSE
        loop {
            tokio::select! {
                _ = ct.cancelled() => {
                    return Err(WorkerQuitReason::Cancelled);
                }

                handler_msg = context.recv_from_handler() => {
                    let WorkerSendRequest { message, responder } = handler_msg?;

                    let body = match serde_json::to_string(&message) {
                        Ok(b) => b,
                        Err(e) => {
                            let _ = responder.send(Err(LegacySseError::Json(e)));
                            continue;
                        }
                    };

                    tracing::debug!("Legacy SSE: POST {}", body);

                    match client
                        .post(&messages_url)
                        .header("Content-Type", "application/json")
                        .body(body)
                        .send()
                        .await
                    {
                        Ok(resp) => {
                            if resp.status().is_success() {
                                let _ = responder.send(Ok(()));
                            } else {
                                let status = resp.status();
                                let body_text = resp.text().await.unwrap_or_default();
                                tracing::warn!("Legacy SSE: POST returned {} — {}", status, body_text);
                                let _ = responder.send(Err(LegacySseError::InvalidUrl(
                                    format!("POST returned status {}", status),
                                )));
                            }
                        }
                        Err(e) => {
                            let _ = responder.send(Err(LegacySseError::Reqwest(e)));
                        }
                    }
                }

                server_msg = sse_rx.recv() => {
                    match server_msg {
                        Some(msg) => {
                            context.send_to_handler(msg).await?;
                        }
                        None => {
                            tracing::info!("Legacy SSE: SSE reader task ended");
                            return Err(WorkerQuitReason::fatal(
                                LegacySseError::StreamEnded,
                                "SSE stream closed",
                            ));
                        }
                    }
                }
            }
        }
    }
}

impl LegacySseWorker {
    async fn read_next_jsonrpc(
        sse_stream: &mut (impl futures::Stream<Item = Result<sse_stream::Sse, sse_stream::Error>>
                  + Unpin),
        ct: &CancellationToken,
    ) -> Result<ServerJsonRpcMessage, WorkerQuitReason<LegacySseError>> {
        loop {
            tokio::select! {
                _ = ct.cancelled() => {
                    return Err(WorkerQuitReason::Cancelled);
                }
                event = sse_stream.next() => {
                    match event {
                        Some(Ok(sse_event)) => {
                            let event_type = sse_event.event.as_deref().unwrap_or("message");
                            if event_type == "message" {
                                if let Some(data) = sse_event.data {
                                    let data: String = data;
                                    let trimmed = data.trim();
                                    if trimmed.is_empty() {
                                        continue;
                                    }
                                    let msg: ServerJsonRpcMessage = serde_json::from_str(trimmed)
                                        .map_err(|e| {
                                            tracing::error!("Legacy SSE: failed to parse: {} — data: {}", e, trimmed);
                                            WorkerQuitReason::fatal(
                                                LegacySseError::Json(e),
                                                Cow::Owned(format!("parse SSE message: {}", trimmed)),
                                            )
                                        })?;
                                    return Ok(msg);
                                }
                            }
                        }
                        Some(Err(e)) => {
                            return Err(WorkerQuitReason::fatal(
                                LegacySseError::StreamEnded,
                                format!("SSE stream error: {}", e),
                            ));
                        }
                        None => {
                            return Err(WorkerQuitReason::fatal(
                                LegacySseError::StreamEnded,
                                "SSE stream ended while waiting for response",
                            ));
                        }
                    }
                }
            }
        }
    }
}
