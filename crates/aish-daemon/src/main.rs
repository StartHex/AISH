//! AISH Daemon — persistent MCP server over Unix socket + TCP.
//!
//! The daemon exposes agent management as MCP tools, allowing external
//! clients (TUI, GUI, CLI, or third-party scripts) to interact with
//! the AISH agent registry via JSON-RPC 2.0 over the configured channels.

use aish_core::config::DaemonConfig;
use aish_core::types::{AgentId, AgentStatus, TaskFilter};
use aish_core::{AgentRegistry, EventBus, TaskScheduler};
use aish_mcp::server::McpServer;
use aish_mcp::types::{CallToolResult, ContentItem};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::net::TcpListener;
#[cfg(unix)]
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tracing::{error, info};

fn text_content(text: &str) -> CallToolResult {
    CallToolResult {
        content: vec![ContentItem {
            content_type: "text".into(),
            text: Some(text.to_string()),
            data: None,
            mime_type: None,
        }],
        is_error: false,
    }
}

fn json_content(value: Value) -> CallToolResult {
    CallToolResult {
        content: vec![ContentItem {
            content_type: "text".into(),
            text: Some(value.to_string()),
            data: None,
            mime_type: None,
        }],
        is_error: false,
    }
}

fn build_daemon_server(
    registry: Arc<AgentRegistry>,
    scheduler: Arc<TaskScheduler>,
    _event_bus: Arc<EventBus>,
) -> McpServer {
    let mut server = McpServer::new("aish-daemon", env!("CARGO_PKG_VERSION"));

    // --- agent.list ---
    {
        let r = registry.clone();
        server.register_tool(
            "agent.list",
            "List all registered agents",
            json!({}),
            move |_| {
                let r = r.clone();
                async move {
                    let agents: Vec<Value> = r
                        .list()
                        .iter()
                        .map(|h| {
                            json!({
                                "id": h.id.to_string(),
                                "alias": h.alias,
                                "status": format!("{:?}", h.status()),
                                "default_model": h.default_model,
                            })
                        })
                        .collect();
                    Ok(json_content(json!(agents)))
                }
            },
        );
    }

    // --- agent.status ---
    {
        let r = registry.clone();
        server.register_tool(
            "agent.status",
            "Get status of a specific agent",
            json!({"id": "string"}),
            move |args| {
                let r = r.clone();
                let agent_id = args["id"].as_str().unwrap_or("").to_string();
                async move {
                    match r.get(&AgentId::from(agent_id.as_str())) {
                        Some(handle) => Ok(json_content(json!({
                            "id": handle.id.to_string(),
                            "status": format!("{:?}", handle.status()),
                            "model": handle.default_model,
                        }))),
                        None => Ok(text_content(&format!("Agent '{}' not found", agent_id))),
                    }
                }
            },
        );
    }

    // --- agent.count ---
    {
        let r = registry.clone();
        server.register_tool(
            "agent.count",
            "Count agents by status",
            json!({}),
            move |_| {
                let r = r.clone();
                async move {
                    let all = r.list();
                    let online = all
                        .iter()
                        .filter(|h| {
                            matches!(
                                h.status(),
                                AgentStatus::Online { .. } | AgentStatus::Busy { .. }
                            )
                        })
                        .count();
                    let total = all.len();
                    Ok(json_content(json!({
                        "total": total,
                        "online": online,
                        "offline": total - online,
                    })))
                }
            },
        );
    }

    // --- task.list ---
    {
        let s = scheduler.clone();
        server.register_tool("task.list", "List all tasks", json!({}), move |_| {
            let s = s.clone();
            async move {
                let filter = TaskFilter {
                    agent_id: None,
                    status: None,
                    limit: None,
                    offset: None,
                };
                let tasks: Vec<Value> = s
                    .list(&filter)
                    .iter()
                    .map(|t| {
                        json!({
                            "id": t.id.to_string(),
                            "agent": t.agent_id.to_string(),
                            "prompt": t.prompt_preview,
                            "status": format!("{:?}", t.status),
                            "progress": t.progress,
                        })
                    })
                    .collect();
                Ok(json_content(json!(tasks)))
            }
        });
    }

    // --- task.count ---
    {
        let s = scheduler.clone();
        server.register_tool(
            "task.count",
            "Count tasks by status",
            json!({}),
            move |_| {
                let s = s.clone();
                async move {
                    let filter = TaskFilter {
                        agent_id: None,
                        status: None,
                        limit: None,
                        offset: None,
                    };
                    let total = s.list(&filter).len();
                    Ok(json_content(json!({ "total": total })))
                }
            },
        );
    }

    // --- daemon.ping ---
    server.register_tool("daemon.ping", "Health check", json!({}), |_| async move {
        Ok(text_content("pong"))
    });

    // --- daemon.version ---
    server.register_tool(
        "daemon.version",
        "Get daemon version",
        json!({}),
        |_| async move {
            Ok(text_content(concat!(
                "aish-daemon v",
                env!("CARGO_PKG_VERSION")
            )))
        },
    );

    server
}

#[cfg(unix)]
fn expand_tilde(path: &str) -> String {
    if path.starts_with('~') {
        if let Ok(home) = std::env::var("HOME") {
            return path.replace('~', &home);
        }
    }
    path.to_string()
}

#[cfg(unix)]
async fn run_unix_socket(server: Arc<McpServer>, path: &str) -> anyhow::Result<()> {
    let expanded = expand_tilde(path);

    // Remove stale socket file
    if tokio::fs::metadata(&expanded).await.is_ok() {
        tokio::fs::remove_file(&expanded).await?;
    }

    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&expanded).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let listener = UnixListener::bind(&expanded)?;
    info!(path = %expanded, "Unix socket listening");

    loop {
        let (stream, addr) = listener.accept().await?;
        let peer = addr
            .as_pathname()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".into());
        info!(peer = %peer, "Unix socket connection accepted");

        let server = server.clone();
        tokio::spawn(async move {
            let (reader, writer) = stream.into_split();
            let writer = Arc::new(Mutex::new(writer));
            if let Err(e) = server.run(reader, writer).await {
                error!(peer = %peer, error = %e, "Unix socket connection error");
            }
            info!(peer = %peer, "Unix socket connection closed");
        });
    }
}

async fn run_tcp(server: Arc<McpServer>, bind: &str, port: u16) -> anyhow::Result<()> {
    let addr = format!("{}:{}", bind, port);
    let listener = TcpListener::bind(&addr).await?;
    info!(addr = %addr, "TCP listening");

    loop {
        let (stream, peer) = listener.accept().await?;
        info!(peer = %peer, "TCP connection accepted");

        let server = server.clone();
        tokio::spawn(async move {
            let (reader, writer) = stream.into_split();
            let writer = Arc::new(Mutex::new(writer));
            if let Err(e) = server.run(reader, writer).await {
                error!(peer = %peer, error = %e, "TCP connection error");
            }
            info!(peer = %peer, "TCP connection closed");
        });
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = DaemonConfig::default();
    info!("AISH Daemon starting");

    let registry = Arc::new(AgentRegistry::new(Arc::new(EventBus::default())));
    let scheduler = Arc::new(TaskScheduler::new(Arc::new(EventBus::default())));
    let event_bus = Arc::new(EventBus::default());

    let server = Arc::new(build_daemon_server(registry, scheduler, event_bus));

    let mcp_cfg = &config.mcp_server;
    let mut tasks = tokio::task::JoinSet::new();

    #[cfg(unix)]
    if mcp_cfg.unix_socket.enabled {
        let s = server.clone();
        let socket_path = mcp_cfg
            .unix_socket
            .path
            .clone()
            .unwrap_or_else(|| std::path::PathBuf::from("~/.aish/daemon.sock"))
            .to_string_lossy()
            .to_string();
        tasks.spawn(async move {
            if let Err(e) = run_unix_socket(s, &socket_path).await {
                error!("Unix socket listener failed: {}", e);
            }
        });
    }
    #[cfg(not(unix))]
    if mcp_cfg.unix_socket.enabled {
        info!("Unix socket not supported on this platform, use TCP instead");
    }

    if mcp_cfg.tcp.enabled {
        let s = server.clone();
        let bind = mcp_cfg.tcp.bind.clone();
        let port = mcp_cfg.tcp.port;
        tasks.spawn(async move {
            if let Err(e) = run_tcp(s, &bind, port).await {
                error!("TCP listener failed: {}", e);
            }
        });
    }

    info!(
        unix = mcp_cfg.unix_socket.enabled,
        tcp = mcp_cfg.tcp.enabled,
        "Daemon ready"
    );

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(()) => info!("Listener task exited"),
            Err(e) => error!(error = %e, "Listener task panicked"),
        }
    }

    Ok(())
}
