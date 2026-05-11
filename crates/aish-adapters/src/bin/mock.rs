//! Mock adapter binary — standalone MCP server for testing.
//!
//! Usage: aish-adapter-mock
//! Reads JSON-RPC from stdin, writes responses to stdout.

use aish_adapters::mock::build_mock_server;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Mock adapter starting");

    let server = build_mock_server();
    if let Err(e) = server.run_stdio().await {
        eprintln!("Mock adapter error: {}", e);
        std::process::exit(1);
    }
}
