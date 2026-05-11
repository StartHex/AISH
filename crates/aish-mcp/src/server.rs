//! MCP Server framework — for building adapter processes.
//!
//! An MCP server reads JSON-RPC from stdin, dispatches to registered handlers,
//! and writes responses + notifications to stdout. Each AI agent adapter
//! (Claude Code, Hermes, OpenClaw) is an MCP server built on this framework.

use crate::types::*;
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, error, info, warn};

/// Signature of a tool handler function.
pub type ToolHandler = Arc<
    dyn Fn(
            Value,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<CallToolResult>> + Send>>
        + Send
        + Sync,
>;

/// MCP Server that reads from stdin and writes to stdout.
pub struct McpServer {
    tools: HashMap<String, ToolDef>,
    notifications: broadcast::Sender<Value>,
    server_info: ServerInfo,
    initialized: Mutex<bool>,
}

struct ToolDef {
    tool: Tool,
    handler: ToolHandler,
}

impl McpServer {
    pub fn new(name: &str, version: &str) -> Self {
        let (tx, _) = broadcast::channel(256);
        McpServer {
            tools: HashMap::new(),
            notifications: tx,
            server_info: ServerInfo {
                name: name.to_string(),
                version: version.to_string(),
            },
            initialized: Mutex::new(false),
        }
    }

    /// Register a tool with its handler.
    pub fn register_tool<F, Fut>(
        &mut self,
        name: &str,
        description: &str,
        input_schema: Value,
        handler: F,
    ) where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<CallToolResult>> + Send + 'static,
    {
        let tool = Tool {
            name: name.to_string(),
            description: Some(description.to_string()),
            input_schema,
        };
        let handler: ToolHandler = Arc::new(move |args| {
            let fut = handler(args);
            Box::pin(fut)
        });
        self.tools
            .insert(name.to_string(), ToolDef { tool, handler });
    }

    /// Send a notification to all connected clients.
    pub fn send_notification(&self, method: &str, params: Value) {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        let _ = self.notifications.send(notification);
    }

    /// Subscribe to outgoing notifications (for testing).
    pub fn subscribe_notifications(&self) -> broadcast::Receiver<Value> {
        self.notifications.subscribe()
    }

    /// Run the server: read NDJSON from reader, write to writer.
    /// Blocks until stdin closes.
    pub async fn run<R, W>(&self, reader: R, writer: Arc<Mutex<W>>) -> Result<()>
    where
        R: tokio::io::AsyncRead + Unpin,
        W: tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let buf_reader = BufReader::new(reader);
        let mut lines = buf_reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let request: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(e) => {
                    warn!(%line, error = %e, "Failed to parse JSON");
                    let err = JsonRpcResponse::error(
                        None,
                        error_codes::PARSE_ERROR,
                        &format!("Parse error: {}", e),
                    );
                    let resp = serde_json::to_string(&err)? + "\n";
                    writer.lock().await.write_all(resp.as_bytes()).await?;
                    continue;
                }
            };

            let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let id = request.get("id").and_then(|i| i.as_u64());
            let params = request.get("params").cloned().unwrap_or(Value::Null);

            let response = self.handle_request(method, id, params).await;

            if let Some(resp) = response {
                let json = serde_json::to_string(&resp)? + "\n";
                if let Err(e) = writer.lock().await.write_all(json.as_bytes()).await {
                    error!(error = %e, "Failed to write response");
                    break;
                }
            }
        }

        info!("Server stdin closed, shutting down");
        Ok(())
    }

    /// Run the server on stdin/stdout (most common mode for adapters).
    pub async fn run_stdio(&self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let stdout = Arc::new(Mutex::new(tokio::io::stdout()));
        self.run(stdin, stdout).await
    }

    /// Handle a single JSON-RPC request. Returns None for notifications.
    pub async fn handle_request(
        &self,
        method: &str,
        id: Option<u64>,
        params: Value,
    ) -> Option<Value> {
        match method {
            "initialize" => {
                let result = self.handle_initialize(params);
                let resp = JsonRpcResponse::success(id.unwrap_or(0), result);
                Some(serde_json::to_value(resp).unwrap())
            }
            "ping" => {
                let resp = JsonRpcResponse::success(id.unwrap_or(0), serde_json::json!({}));
                Some(serde_json::to_value(resp).unwrap())
            }
            "tools/list" => {
                let tools: Vec<&Tool> = self.tools.values().map(|d| &d.tool).collect();
                let result = serde_json::json!({ "tools": tools });
                let resp = JsonRpcResponse::success(id.unwrap_or(0), result);
                Some(serde_json::to_value(resp).unwrap())
            }
            "tools/call" => {
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(Value::Null);

                match self.tools.get(tool_name) {
                    Some(def) => match (def.handler)(args).await {
                        Ok(result) => {
                            let content = result
                                .content
                                .into_iter()
                                .map(|c| serde_json::to_value(c).unwrap())
                                .collect::<Vec<_>>();
                            let resp = JsonRpcResponse::success(
                                id.unwrap_or(0),
                                serde_json::json!({ "content": content, "isError": result.is_error }),
                            );
                            Some(serde_json::to_value(resp).unwrap())
                        }
                        Err(e) => {
                            let resp = JsonRpcResponse::error(
                                Some(id.unwrap_or(0)),
                                error_codes::INTERNAL_ERROR,
                                &format!("Tool error: {}", e),
                            );
                            Some(serde_json::to_value(resp).unwrap())
                        }
                    },
                    None => {
                        let resp = JsonRpcResponse::error(
                            Some(id.unwrap_or(0)),
                            error_codes::METHOD_NOT_FOUND,
                            &format!("Tool not found: {}", tool_name),
                        );
                        Some(serde_json::to_value(resp).unwrap())
                    }
                }
            }
            "notifications/initialized" => {
                *self.initialized.lock().await = true;
                info!("Client initialized");
                None // No response for notification
            }
            _ => {
                if id.is_none() {
                    // Unknown notification — silently ignore
                    debug!(%method, "Unknown notification, ignoring");
                    None
                } else {
                    let resp = JsonRpcResponse::error(
                        id,
                        error_codes::METHOD_NOT_FOUND,
                        &format!("Method not found: {}", method),
                    );
                    Some(serde_json::to_value(resp).unwrap())
                }
            }
        }
    }

    fn handle_initialize(&self, _params: Value) -> Value {
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": { "listChanged": true }
            },
            "serverInfo": self.server_info,
        })
    }
}
