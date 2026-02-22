//! MCP Hub Bridge — stdio-to-HTTP proxy for Claude Desktop
//!
//! Reads JSON-RPC messages from stdin, forwards them to the MCP Hub HTTP proxy,
//! and writes responses to stdout. This allows Claude Desktop (which only supports
//! stdio MCP servers) to talk to any MCP server managed by MCP Hub.
//!
//! Usage:
//!   mcp-hub-bridge --mcp-id <SERVER_ID> [--port <PORT>]

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

struct Args {
    port: u16,
    mcp_id: String,
}

fn parse_args() -> Result<Args, String> {
    let mut args = std::env::args().skip(1);
    let mut port: u16 = 3001;
    let mut mcp_id: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--port" => {
                let val = args.next().ok_or("--port requires a value")?;
                port = val.parse().map_err(|_| format!("invalid port: {}", val))?;
            }
            "--mcp-id" => {
                mcp_id = Some(args.next().ok_or("--mcp-id requires a value")?);
            }
            other => return Err(format!("unknown argument: {}", other)),
        }
    }

    Ok(Args {
        port,
        mcp_id: mcp_id.ok_or("--mcp-id is required")?,
    })
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("mcp-hub-bridge: {}", e);
            eprintln!("Usage: mcp-hub-bridge --mcp-id <ID> [--port <PORT>]");
            return std::process::ExitCode::from(1);
        }
    };

    let url = format!("http://127.0.0.1:{}/mcp/{}", args.port, args.mcp_id);
    let client = reqwest::Client::new();

    eprintln!("mcp-hub-bridge: proxying stdio <-> {}", url);

    let stdin = BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();
    let mut lines = stdin.lines();

    loop {
        tokio::select! {
            line = lines.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        if let Err(e) = handle_line(&client, &url, &line, &mut stdout).await {
                            eprintln!("mcp-hub-bridge: error: {}", e);
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        eprintln!("mcp-hub-bridge: stdin error: {}", e);
                        break;
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                eprintln!("mcp-hub-bridge: interrupted");
                break;
            }
        }
    }

    eprintln!("mcp-hub-bridge: shutting down, sending DELETE for session cleanup");
    let _ = client.delete(&url).send().await;

    std::process::ExitCode::SUCCESS
}

async fn handle_line(
    client: &reqwest::Client,
    url: &str,
    line: &str,
    stdout: &mut tokio::io::Stdout,
) -> Result<(), Box<dyn std::error::Error>> {
    let value: serde_json::Value = serde_json::from_str(line)?;

    let response = match client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&value)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            // Proxy unreachable — return JSON-RPC error if request had an id
            if let Some(id) = value.get("id") {
                let err = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32000,
                        "message": format!("proxy unreachable: {}", e)
                    }
                });
                let mut out = serde_json::to_vec(&err)?;
                out.push(b'\n');
                stdout.write_all(&out).await?;
                stdout.flush().await?;
            }
            return Ok(());
        }
    };

    let status = response.status();

    // 202 = notification acknowledged, no response expected
    if status.as_u16() == 202 {
        return Ok(());
    }

    if !status.is_success() {
        if let Some(id) = value.get("id") {
            let body = response.text().await.unwrap_or_default();
            let err = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32000,
                    "message": format!("HTTP {}: {}", status.as_u16(), body)
                }
            });
            let mut out = serde_json::to_vec(&err)?;
            out.push(b'\n');
            stdout.write_all(&out).await?;
            stdout.flush().await?;
        }
        return Ok(());
    }

    let body = response.bytes().await?;
    stdout.write_all(&body).await?;
    stdout.write_all(b"\n").await?;
    stdout.flush().await?;

    Ok(())
}
