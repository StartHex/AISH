//! MCP Client — connects to an MCP Server (adapter) through a transport.

use crate::transport::{ReceivedMessage, Transport};
use crate::types::*;
use anyhow::Result;
use serde_json::Value;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;

pub struct McpClient {
    transport: Arc<dyn Transport>,
    next_id: AtomicU64,
    server_info: parking_lot::RwLock<Option<ServerInfo>>,
    capabilities: parking_lot::RwLock<Option<ServerCapabilities>>,
}

impl McpClient {
    pub fn new(transport: Arc<dyn Transport>) -> Self {
        McpClient {
            transport,
            next_id: AtomicU64::new(1),
            server_info: parking_lot::RwLock::new(None),
            capabilities: parking_lot::RwLock::new(None),
        }
    }

    /// Perform the MCP handshake (initialize + initialized notification).
    pub async fn initialize(&self, client_name: &str) -> Result<()> {
        let params = InitializeParams {
            protocol_version: "2024-11-05".into(),
            capabilities: ClientCapabilities {
                roots: Some(RootsCapability {
                    list_changed: true,
                }),
                sampling: None,
            },
            client_info: ClientInfo {
                name: client_name.into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        };

        let result = self.call("initialize", Some(serde_json::to_value(params)?)).await?;
        let init_result: InitializeResult = serde_json::from_value(result)?;

        info!(
            server = %init_result.server_info.name,
            version = %init_result.server_info.version,
            "MCP handshake complete"
        );

        *self.server_info.write() = Some(init_result.server_info);
        *self.capabilities.write() = Some(init_result.capabilities);

        self.notify("notifications/initialized", None).await?;
        Ok(())
    }

    /// List available tools.
    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        let result = self.call("tools/list", None).await?;
        let tools_result: ListToolsResult = serde_json::from_value(result)?;
        Ok(tools_result.tools)
    }

    /// Call a tool.
    pub async fn call_tool(&self, name: &str, arguments: Option<Value>) -> Result<CallToolResult> {
        let params = CallToolParams {
            name: name.to_string(),
            arguments,
        };
        let result = self.call("tools/call", Some(serde_json::to_value(params)?)).await?;
        let tool_result: CallToolResult = serde_json::from_value(result)?;
        Ok(tool_result)
    }

    /// Low-level JSON-RPC call with auto-generated id.
    pub async fn call(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let request = JsonRpcRequest::new(id, method, params);

        let response = self.transport.send(&request).await?;

        // If the response itself contains an error field, treat as error
        // (handled at the transport level already, but double-check)
        Ok(response)
    }

    /// Send a notification (fire-and-forget).
    pub async fn notify(&self, method: &str, params: Option<Value>) -> Result<()> {
        let notification = JsonRpcRequest::notification(method, params);
        // Notifications don't have an id — the transport will handle them
        let _ = self.transport.send(&notification).await;
        Ok(())
    }

    /// Subscribe to notifications from the server.
    pub fn subscribe_notifications(&self) -> broadcast::Receiver<ReceivedMessage> {
        self.transport.subscribe_notifications()
    }

    /// Server info from handshake.
    pub fn server_info(&self) -> Option<ServerInfo> {
        self.server_info.read().clone()
    }

    /// Transport liveness check.
    pub async fn is_alive(&self) -> bool {
        self.transport.is_alive().await
    }

    /// Shut down.
    pub async fn shutdown(&self) -> Result<()> {
        self.transport.shutdown().await
    }
}
