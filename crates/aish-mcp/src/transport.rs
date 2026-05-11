//! Transport abstraction for MCP protocol communication.
//!
//! Each transport manages: (a) sending JSON-RPC requests and matching them to responses,
//! (b) forwarding incoming notifications to subscribers.

use crate::types::JsonRpcRequest;
use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::{trace, warn};

// ---- Message types ----

#[derive(Debug, Clone)]
pub enum ReceivedMessage {
    Response { id: u64, result: serde_json::Value },
    Notification { method: String, params: serde_json::Value },
}

// ---- Transport trait ----

#[async_trait]
pub trait Transport: Send + Sync {
    /// Send a JSON-RPC request and wait for the matching response.
    async fn send(&self, request: &JsonRpcRequest) -> Result<serde_json::Value>;

    /// Subscribe to notifications (server→client pushes without an id).
    fn subscribe_notifications(&self) -> broadcast::Receiver<ReceivedMessage>;

    /// Check liveness.
    async fn is_alive(&self) -> bool;

    /// Graceful shutdown.
    async fn shutdown(&self) -> Result<()>;
}

// ---- StdioTransport ----

pub struct StdioTransport {
    pending: Arc<DashMap<u64, oneshot::Sender<serde_json::Value>>>,
    notification_tx: broadcast::Sender<ReceivedMessage>,
    stdin_tx: mpsc::Sender<String>,
    child: Arc<parking_lot::Mutex<Option<Child>>>,
}

impl StdioTransport {
    /// Spawn a subprocess and wire up stdin/stdout NDJSON.
    pub async fn spawn(command: &str, args: &[&str], env: &[(&str, &str)]) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args);
        for (k, v) in env {
            cmd.env(k, v);
        }
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.kill_on_drop(true);

        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take().expect("stdout not piped");
        let stdin = child.stdin.take().expect("stdin not piped");

        let pending: Arc<DashMap<u64, oneshot::Sender<serde_json::Value>>> =
            Arc::new(DashMap::new());
        let (notification_tx, _) = broadcast::channel(256);
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(64);

        // Reader task: parse NDJSON lines from stdout
        let pending_r = pending.clone();
        let notif_tx = notification_tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let value: serde_json::Value = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!(%line, error = %e, "Failed to parse NDJSON line");
                        continue;
                    }
                };

                if let Some(id) = value.get("id").and_then(|v| v.as_u64()) {
                    if value.get("method").is_some() {
                        // It's a request (server→client) — treat as notification
                        let method = value["method"].as_str().unwrap_or("").to_string();
                        let params = value.get("params").cloned().unwrap_or(serde_json::Value::Null);
                        let _ = notif_tx.send(ReceivedMessage::Notification { method, params });
                    } else {
                        // It's a response
                        let result = value.get("result").cloned().unwrap_or(serde_json::Value::Null);
                        if let Some((_, tx)) = pending_r.remove(&id) {
                            let _ = tx.send(result);
                        } else {
                            trace!(id, "No pending request for response id");
                        }
                    }
                } else if value.get("method").is_some() {
                    // Notification (no id)
                    let method = value["method"].as_str().unwrap_or("").to_string();
                    let params = value.get("params").cloned().unwrap_or(serde_json::Value::Null);
                    let _ = notif_tx.send(ReceivedMessage::Notification { method, params });
                }
            }
        });

        // Writer task: write NDJSON lines to stdin
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(line) = stdin_rx.recv().await {
                if let Err(e) = stdin.write_all(line.as_bytes()).await {
                    warn!(error = %e, "Failed to write to stdin");
                    break;
                }
                if let Err(e) = stdin.write_all(b"\n").await {
                    warn!(error = %e, "Failed to write newline");
                    break;
                }
                if let Err(e) = stdin.flush().await {
                    warn!(error = %e, "Failed to flush stdin");
                    break;
                }
            }
        });

        Ok(StdioTransport {
            pending,
            notification_tx,
            stdin_tx,
            child: Arc::new(parking_lot::Mutex::new(Some(child))),
        })
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn send(&self, request: &JsonRpcRequest) -> Result<serde_json::Value> {
        let id = request.id.unwrap_or(0);
        let (tx, rx) = oneshot::channel();
        self.pending.insert(id, tx);

        let line = serde_json::to_string(request)?;
        self.stdin_tx.send(line).await?;

        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(_)) => {
                self.pending.remove(&id);
                anyhow::bail!("Response channel closed for request {}", id)
            }
            Err(_) => {
                self.pending.remove(&id);
                anyhow::bail!("Request {} timed out", id)
            }
        }
    }

    fn subscribe_notifications(&self) -> broadcast::Receiver<ReceivedMessage> {
        self.notification_tx.subscribe()
    }

    async fn is_alive(&self) -> bool {
        let mut guard = self.child.lock();
        if let Some(ref mut child) = *guard {
            matches!(child.try_wait(), Ok(None))
        } else {
            false
        }
    }

    async fn shutdown(&self) -> Result<()> {
        let child_opt = {
            let mut guard = self.child.lock();
            guard.take()
        };
        if let Some(mut child) = child_opt {
            let _ = child.kill().await;
        }
        Ok(())
    }
}

// ---- SSH Transport ----

/// SSH transport: uses the system `ssh` command to connect to a remote host
/// and pipe NDJSON through stdin/stdout. Built on top of StdioTransport.
pub struct SshTransport {
    inner: StdioTransport,
}

impl SshTransport {
    /// Connect via SSH and start the remote adapter command.
    pub async fn connect(config: &aish_ssh::SshConfig, remote_command: &str) -> Result<Self> {
        let (cmd, args) = config.to_command(remote_command);
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let inner = StdioTransport::spawn(&cmd, &args_refs, &[]).await?;
        Ok(SshTransport { inner })
    }

    /// Convenience: connect and run a standard aish-adapter-<type>.
    pub async fn connect_adapter(config: &aish_ssh::SshConfig, adapter_type: &str) -> Result<Self> {
        let (cmd, args) = config.adapter_command(adapter_type);
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let inner = StdioTransport::spawn(&cmd, &args_refs, &[]).await?;
        Ok(SshTransport { inner })
    }
}

#[async_trait]
impl Transport for SshTransport {
    async fn send(&self, request: &JsonRpcRequest) -> Result<serde_json::Value> {
        self.inner.send(request).await
    }
    fn subscribe_notifications(&self) -> broadcast::Receiver<ReceivedMessage> {
        self.inner.subscribe_notifications()
    }
    async fn is_alive(&self) -> bool {
        self.inner.is_alive().await
    }
    async fn shutdown(&self) -> Result<()> {
        self.inner.shutdown().await
    }
}

pub struct UnixSocketTransport;

#[async_trait]
impl Transport for UnixSocketTransport {
    async fn send(&self, _request: &JsonRpcRequest) -> Result<serde_json::Value> {
        anyhow::bail!("Unix socket transport not yet implemented")
    }
    fn subscribe_notifications(&self) -> broadcast::Receiver<ReceivedMessage> {
        let (_, rx) = broadcast::channel(1);
        rx
    }
    async fn is_alive(&self) -> bool { false }
    async fn shutdown(&self) -> Result<()> { Ok(()) }
}

pub struct TcpTransport;

#[async_trait]
impl Transport for TcpTransport {
    async fn send(&self, _request: &JsonRpcRequest) -> Result<serde_json::Value> {
        anyhow::bail!("TCP transport not yet implemented")
    }
    fn subscribe_notifications(&self) -> broadcast::Receiver<ReceivedMessage> {
        let (_, rx) = broadcast::channel(1);
        rx
    }
    async fn is_alive(&self) -> bool { false }
    async fn shutdown(&self) -> Result<()> { Ok(()) }
}
