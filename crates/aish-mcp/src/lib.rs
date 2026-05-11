//! AISH MCP — Model Context Protocol implementation.
//!
//! Implements JSON-RPC 2.0 transport layer + MCP client for
//! communicating with AI agent adapters.

pub mod client;
pub mod server;
pub mod transport;
pub mod types;

pub use client::McpClient;
pub use server::McpServer;
pub use transport::{ReceivedMessage, StdioTransport, Transport};
pub use types::*;
