//! Claude Code adapter binary — wraps `claude` CLI as an MCP server.
//!
//! Usage: aish-adapter-claude [--project <dir>] [--model <model>]
//! Reads JSON-RPC from stdin, writes responses + notifications to stdout.
//! On task.submit, spawns `claude -p <prompt>` and streams results.

use aish_mcp::server::McpServer;
use aish_mcp::types::{CallToolResult, ContentItem};
use chrono::Utc;
use clap::Parser;
use serde_json::{json, Value};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(name = "aish-adapter-claude", version)]
struct Args {
    /// Project directory for Claude Code
    #[arg(long)]
    project: Option<String>,

    /// Default model
    #[arg(long, default_value = "claude-sonnet-4-6")]
    model: String,

    /// Path to claude binary
    #[arg(long, default_value = "claude")]
    claude_bin: String,
}

struct ClaudeState {
    model: Mutex<String>,
    token_input: AtomicU64,
    token_output: AtomicU64,
    active_tasks: Mutex<Vec<String>>,
    project_dir: Option<String>,
    claude_bin: String,
}

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

fn build_claude_server(args: Args) -> McpServer {
    let mut server = McpServer::new("claude-code-adapter", env!("CARGO_PKG_VERSION"));
    let state = Arc::new(ClaudeState {
        model: Mutex::new(args.model),
        token_input: AtomicU64::new(0),
        token_output: AtomicU64::new(0),
        active_tasks: Mutex::new(vec![]),
        project_dir: args.project,
        claude_bin: args.claude_bin,
    });

    // --- agent.status ---
    {
        let s = state.clone();
        server.register_tool("agent.status", "Get agent status", json!({}), move |_| {
            let s = s.clone();
            async move {
                let model = s.model.lock().await.clone();
                let tasks = s.active_tasks.lock().await.len();
                Ok(json_content(json!({
                    "status": "online",
                    "model": model,
                    "tasks_active": tasks,
                    "adapter": "claude-code",
                })))
            }
        });
    }

    // --- agent.list_models ---
    server.register_tool(
        "agent.list_models",
        "List available Claude models",
        json!({}),
        |_| async move {
            Ok(json_content(json!([
                {"id": "claude-sonnet-4-6", "provider": "anthropic", "context_window": 200000},
                {"id": "claude-opus-4-7", "provider": "anthropic", "context_window": 200000},
                {"id": "claude-haiku-4-5", "provider": "anthropic", "context_window": 200000},
                {"id": "claude-sonnet-4", "provider": "anthropic", "context_window": 200000},
            ])))
        },
    );

    // --- agent.set_model ---
    {
        let s = state.clone();
        server.register_tool(
            "agent.set_model",
            "Switch Claude model",
            json!({"model":"string"}),
            move |args| {
                let s = s.clone();
                async move {
                    let new_model = args["model"]
                        .as_str()
                        .unwrap_or("claude-sonnet-4-6")
                        .to_string();
                    let old = s.model.lock().await.clone();
                    *s.model.lock().await = new_model.clone();
                    Ok(text_content(&format!("Model: {} → {}", old, new_model)))
                }
            },
        );
    }

    // --- agent.permissions ---
    server.register_tool(
        "agent.permissions", "Get Claude Code permission matrix",
        json!({}),
        |_| async move {
            Ok(json_content(json!({
                "default_permit": "Ask",
                "entries": [
                    {"tool_name": "Bash", "permit": "Allow", "description": "Execute shell commands"},
                    {"tool_name": "Read", "permit": "Allow", "description": "Read files"},
                    {"tool_name": "Write", "permit": "Ask", "description": "Write new files"},
                    {"tool_name": "Edit", "permit": "Ask", "description": "Edit existing files"},
                    {"tool_name": "Glob", "permit": "Allow", "description": "Search file names"},
                    {"tool_name": "Grep", "permit": "Allow", "description": "Search file contents"},
                    {"tool_name": "Agent", "permit": "Allow", "description": "Spawn sub-agents"},
                    {"tool_name": "WebFetch", "permit": "Deny", "description": "Fetch URLs"},
                    {"tool_name": "WebSearch", "permit": "Deny", "description": "Search the web"},
                ],
            })))
        },
    );

    // --- agent.skills ---
    server.register_tool("agent.skills", "List Claude Code skills", json!({}), |_| async move {
        Ok(json_content(json!([
            {"name": "code-review", "description": "Review pull requests", "loaded": true, "call_count": 0},
            {"name": "security-review", "description": "Security audit", "loaded": true, "call_count": 0},
            {"name": "simplify", "description": "Refactor code", "loaded": true, "call_count": 0},
        ])))
    });

    // --- agent.mcp_servers ---
    server.register_tool(
        "agent.mcp_servers",
        "List MCP servers",
        json!({}),
        |_| async move { Ok(json_content(json!([]))) },
    );

    // --- agent.token_usage ---
    {
        let s = state.clone();
        server.register_tool(
            "agent.token_usage",
            "Get token usage",
            json!({"window":"string"}),
            move |_| {
                let s = s.clone();
                async move {
                    let input = s.token_input.load(std::sync::atomic::Ordering::Relaxed);
                    let output = s.token_output.load(std::sync::atomic::Ordering::Relaxed);
                    Ok(json_content(json!({
                        "total_input": input,
                        "total_output": output,
                        "by_model": {},
                    })))
                }
            },
        );
    }

    // --- task.submit (the key method: spawns claude) ---
    {
        let s = state.clone();
        server.register_tool(
            "task.submit",
            "Submit a prompt to Claude Code",
            json!({"prompt": "string", "model": "string?"}),
            move |tool_args| {
                let s = s.clone();
                // Extract values before moving into async block
                let prompt = tool_args["prompt"].as_str().unwrap_or("").to_string();
                let model = tool_args
                    .get("model")
                    .and_then(|m| m.as_str())
                    .unwrap_or("claude-sonnet-4-6")
                    .to_string();
                async move {
                    let task_id = format!("claude-{}", Utc::now().timestamp_millis());
                    s.active_tasks.lock().await.push(task_id.clone());

                    // Rough token estimate before moving prompt
                    let input_tokens_est = (prompt.len() as u64) / 4;

                    // Spawn claude process
                    let claude_bin = s.claude_bin.clone();
                    let project_dir = s.project_dir.clone();

                    let result = tokio::task::spawn_blocking(move || {
                        std::process::Command::new(&claude_bin)
                            .arg("-p")
                            .arg(&prompt)
                            .arg("--model")
                            .arg(&model)
                            .arg("--output-format")
                            .arg("text")
                            .current_dir(project_dir.unwrap_or_else(|| ".".into()))
                            .output()
                    })
                    .await;

                    match result {
                        Ok(Ok(output)) => {
                            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                            let output_tokens = (stdout.len() as u64) / 4;
                            s.token_input
                                .fetch_add(input_tokens_est, std::sync::atomic::Ordering::Relaxed);
                            s.token_output
                                .fetch_add(output_tokens, std::sync::atomic::Ordering::Relaxed);

                            // Remove from active
                            s.active_tasks.lock().await.retain(|t| t != &task_id);

                            if output.status.success() {
                                info!(task = %task_id, "Claude task completed");
                                Ok(text_content(&format!(
                                    "Task {} completed.\n\n{}\n\n---\nTokens: ~{} in / ~{} out",
                                    task_id, stdout, input_tokens_est, output_tokens
                                )))
                            } else {
                                warn!(task = %task_id, "Claude task failed");
                                Ok(text_content(&format!(
                                    "Task {} failed.\n\nstderr: {}\n\nstdout: {}",
                                    task_id, stderr, stdout
                                )))
                            }
                        }
                        Ok(Err(e)) => {
                            s.active_tasks.lock().await.retain(|t| t != &task_id);
                            error!("Failed to spawn claude: {}", e);
                            Ok(text_content(&format!("Error spawning claude: {}", e)))
                        }
                        Err(e) => {
                            s.active_tasks.lock().await.retain(|t| t != &task_id);
                            error!("Join error: {}", e);
                            Ok(text_content(&format!("Internal error: {}", e)))
                        }
                    }
                }
            },
        );
    }

    // --- task.list ---
    {
        let s = state.clone();
        server.register_tool("task.list", "List active tasks", json!({}), move |_| {
            let s = s.clone();
            async move {
                let tasks = s.active_tasks.lock().await.clone();
                Ok(json_content(json!(tasks)))
            }
        });
    }

    // --- task.cancel ---
    {
        let s = state.clone();
        server.register_tool(
            "task.cancel",
            "Cancel a task",
            json!({"task_id":"string"}),
            move |args| {
                let tid = args["task_id"].as_str().unwrap_or("").to_string();
                let s = s.clone();
                async move {
                    let mut tasks = s.active_tasks.lock().await;
                    if tasks.contains(&tid) {
                        tasks.retain(|t| t != &tid);
                        Ok(text_content(&format!("Task {} cancelled", tid)))
                    } else {
                        Ok(text_content(&format!("Task {} not found", tid)))
                    }
                }
            },
        );
    }

    // --- adapter metadata ---
    server.register_tool(
        "adapter.version",
        "Get adapter version",
        json!({}),
        |_| async move {
            Ok(text_content(concat!(
                "claude-code-adapter v",
                env!("CARGO_PKG_VERSION")
            )))
        },
    );

    server.register_tool(
        "adapter.capabilities",
        "Get adapter capabilities",
        json!({}),
        |_| async move {
            Ok(json_content(json!([
                "agent.status",
                "agent.list_models",
                "agent.set_model",
                "agent.permissions",
                "agent.skills",
                "agent.mcp_servers",
                "agent.token_usage",
                "task.submit",
                "task.list",
                "task.cancel",
                "adapter.version",
                "adapter.capabilities",
            ])))
        },
    );

    server
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    info!(
        model = %args.model,
        project = ?args.project,
        "Claude Code adapter starting"
    );

    let server = build_claude_server(args);
    if let Err(e) = server.run_stdio().await {
        eprintln!("Adapter error: {}", e);
        std::process::exit(1);
    }
}
