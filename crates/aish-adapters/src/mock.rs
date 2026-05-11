//! Mock adapter — a fully functional MCP server simulating an AI agent.
//! Used for testing the AISH → MCP → adapter pipeline without real AI backends.

use aish_mcp::server::McpServer;
use aish_mcp::types::{CallToolResult, ContentItem};
use chrono::Utc;
use serde_json::{json, Value};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Simulated agent state.
struct MockAgentState {
    tasks: Mutex<Vec<MockTask>>,
    model: Mutex<String>,
    permissions: Mutex<Vec<Value>>,
    skills: Mutex<Vec<Value>>,
    mcp_servers: Mutex<Vec<Value>>,
    token_input: AtomicU64,
    token_output: AtomicU64,
}

#[derive(Debug, Clone)]
struct MockTask {
    id: String,
    prompt: String,
    status: String,
    progress: f32,
}

impl MockAgentState {
    fn new() -> Self {
        MockAgentState {
            tasks: Mutex::new(vec![]),
            model: Mutex::new("mock-model-v1".to_string()),
            permissions: Mutex::new(vec![
                json!({"tool_name": "Bash", "permit": "Allow", "description": "Execute shell commands"}),
                json!({"tool_name": "Read", "permit": "Allow", "description": "Read files"}),
                json!({"tool_name": "Write", "permit": "Ask", "description": "Write new files"}),
                json!({"tool_name": "Edit", "permit": "Ask", "description": "Edit existing files"}),
                json!({"tool_name": "WebFetch", "permit": "Deny", "description": "Fetch URLs"}),
            ]),
            skills: Mutex::new(vec![
                json!({"name": "code-review", "description": "Review code changes", "loaded": true, "call_count": 42}),
                json!({"name": "security-audit", "description": "Security vulnerability scan", "loaded": true, "call_count": 15}),
            ]),
            mcp_servers: Mutex::new(vec![
                json!({"name": "filesystem", "status": "Connected", "tools_count": 5, "resources_count": 0}),
                json!({"name": "github", "status": "Connected", "tools_count": 12, "resources_count": 3}),
            ]),
            token_input: AtomicU64::new(0),
            token_output: AtomicU64::new(0),
        }
    }
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

/// Build and return a fully registered mock MCP server.
pub fn build_mock_server() -> McpServer {
    let mut server = McpServer::new("mock-adapter", "0.1.0");
    let state = Arc::new(MockAgentState::new());

    // ---- Agent Management tools ----

    let s = state.clone();
    server.register_tool(
        "agent.status",
        "Get agent current status",
        json!({}),
        move |_args| {
            let s = s.clone();
            async move {
                let model = s.model.lock().await.clone();
                Ok(json_content(json!({
                    "status": "online",
                    "uptime_secs": 3600,
                    "model": model,
                    "tasks_active": s.tasks.lock().await.len(),
                })))
            }
        },
    );

    server.register_tool(
        "agent.list_models",
        "List available models",
        json!({}),
        move |_args| async move {
            Ok(json_content(json!([
                {"id": "mock-model-v1", "provider": "mock", "context_window": 200000},
                {"id": "mock-model-v2", "provider": "mock", "context_window": 500000},
                {"id": "mock-model-fast", "provider": "mock", "context_window": 100000},
            ])))
        },
    );

    let s = state.clone();
    server.register_tool(
        "agent.set_model",
        "Switch model",
        json!({"model": "string"}),
        move |args| {
            let s = s.clone();
            async move {
                let model = args["model"].as_str().unwrap_or("unknown").to_string();
                let old = s.model.lock().await.clone();
                *s.model.lock().await = model.clone();
                Ok(text_content(&format!(
                    "Model switched: {} → {}",
                    old, model
                )))
            }
        },
    );

    let s = state.clone();
    server.register_tool(
        "agent.permissions",
        "Get permission matrix",
        json!({}),
        move |_args| {
            let s = s.clone();
            async move {
                Ok(json_content(json!({
                    "entries": *s.permissions.lock().await,
                    "default_permit": "Ask",
                })))
            }
        },
    );

    let s = state.clone();
    server.register_tool(
        "agent.set_permission",
        "Set tool permission",
        json!({"tool":"string","permit":"string"}),
        move |args| {
            let s = s.clone();
            async move {
                let tool = args["tool"].as_str().unwrap_or("");
                let permit = args["permit"].as_str().unwrap_or("Ask");
                let mut perms = s.permissions.lock().await;
                for p in perms.iter_mut() {
                    if p["tool_name"].as_str() == Some(tool) {
                        let old = p["permit"].as_str().unwrap_or("?").to_string();
                        p["permit"] = json!(permit);
                        return Ok(text_content(&format!(
                            "Permission changed: {} {} → {}",
                            tool, old, permit
                        )));
                    }
                }
                Ok(text_content(&format!(
                    "Tool '{}' not found in permission list",
                    tool
                )))
            }
        },
    );

    let s = state.clone();
    server.register_tool(
        "agent.skills",
        "List registered skills",
        json!({}),
        move |_args| {
            let s = s.clone();
            async move { Ok(json_content(json!(*s.skills.lock().await))) }
        },
    );

    let s = state.clone();
    server.register_tool(
        "agent.mcp_servers",
        "List connected MCP servers",
        json!({}),
        move |_args| {
            let s = s.clone();
            async move { Ok(json_content(json!(*s.mcp_servers.lock().await))) }
        },
    );

    let s = state.clone();
    server.register_tool(
        "agent.token_usage",
        "Get token statistics",
        json!({"window": "string"}),
        move |_args| {
            let s = s.clone();
            async move {
                Ok(json_content(json!({
                    "total_input": s.token_input.load(std::sync::atomic::Ordering::Relaxed),
                    "total_output": s.token_output.load(std::sync::atomic::Ordering::Relaxed),
                    "by_model": {
                        "mock-model-v1": {
                            "input": s.token_input.load(std::sync::atomic::Ordering::Relaxed),
                            "output": s.token_output.load(std::sync::atomic::Ordering::Relaxed),
                            "requests": 15,
                        }
                    },
                })))
            }
        },
    );

    // ---- Task Management tools ----

    let s = state.clone();
    server.register_tool(
        "task.submit",
        "Submit a new task",
        json!({"prompt":"string"}),
        move |args| {
            let s = s.clone();
            async move {
                let prompt = args["prompt"].as_str().unwrap_or("");
                let task_id = format!("mock-task-{}", Utc::now().timestamp_millis());
                let task = MockTask {
                    id: task_id.clone(),
                    prompt: prompt.to_string(),
                    status: "queued".to_string(),
                    progress: 0.0,
                };
                s.tasks.lock().await.push(task);

                // Simulate some token usage
                s.token_input.fetch_add(
                    prompt.len() as u64 / 4,
                    std::sync::atomic::Ordering::Relaxed,
                );
                s.token_output
                    .fetch_add(200, std::sync::atomic::Ordering::Relaxed);

                Ok(text_content(&format!("Task submitted: {}", task_id)))
            }
        },
    );

    let s = state.clone();
    server.register_tool(
        "task.list",
        "List tasks",
        json!({"filter":"object"}),
        move |_args| {
            let s = s.clone();
            async move {
                let tasks: Vec<Value> = s
                    .tasks
                    .lock()
                    .await
                    .iter()
                    .map(|t| {
                        json!({
                            "id": t.id,
                            "prompt_preview": &t.prompt[..t.prompt.len().min(100)],
                            "status": t.status,
                            "progress": t.progress,
                        })
                    })
                    .collect();
                Ok(json_content(json!(tasks)))
            }
        },
    );

    let s = state.clone();
    server.register_tool(
        "task.cancel",
        "Cancel a task",
        json!({"task_id":"string"}),
        move |args| {
            let tid = args["task_id"].as_str().unwrap_or("").to_string();
            let s = s.clone();
            async move {
                let mut tasks = s.tasks.lock().await;
                if let Some(t) = tasks.iter_mut().find(|t| t.id == tid) {
                    t.status = "cancelled".to_string();
                    Ok(text_content(&format!("Task {} cancelled", tid)))
                } else {
                    Ok(text_content(&format!("Task {} not found", tid)))
                }
            }
        },
    );

    // ---- Adapter metadata ----

    server.register_tool(
        "adapter.version",
        "Get adapter version",
        json!({}),
        |_args| async move { Ok(text_content("mock-adapter v0.1.0")) },
    );

    server.register_tool(
        "adapter.capabilities",
        "Get adapter capabilities",
        json!({}),
        |_args| async move {
            Ok(json_content(json!([
                "agent.status",
                "agent.list_models",
                "agent.set_model",
                "agent.permissions",
                "agent.set_permission",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_server_initialize_and_list_tools() {
        let server = build_mock_server();

        // Initialize
        let init_resp = server
            .handle_request(
                "initialize",
                Some(1),
                json!({
                    "protocolVersion": "2024-11-05",
                    "clientInfo": {"name": "test", "version": "1.0"},
                }),
            )
            .await;
        assert!(init_resp.is_some());

        // List tools
        let tools_resp = server
            .handle_request("tools/list", Some(2), Value::Null)
            .await;
        assert!(tools_resp.is_some());

        let tools_val = tools_resp.unwrap();
        let tools: Vec<String> = tools_val["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();

        assert!(tools.contains(&"agent.status".to_string()));
        assert!(tools.contains(&"task.submit".to_string()));
        assert!(tools.contains(&"agent.permissions".to_string()));
        assert!(tools.contains(&"adapter.version".to_string()));
    }

    #[tokio::test]
    async fn test_mock_server_agent_status() {
        let server = build_mock_server();
        server
            .handle_request(
                "initialize",
                Some(1),
                json!({
                    "protocolVersion": "2024-11-05",
                    "clientInfo": {"name": "test", "version": "1.0"},
                }),
            )
            .await;

        let resp = server
            .handle_request(
                "tools/call",
                Some(3),
                json!({
                    "name": "agent.status",
                    "arguments": {}
                }),
            )
            .await;
        assert!(resp.is_some());

        let val = resp.unwrap();
        let content = &val["result"]["content"][0]["text"];
        let status: Value = serde_json::from_str(content.as_str().unwrap()).unwrap();
        assert_eq!(status["status"], "online");
        assert_eq!(status["model"], "mock-model-v1");
    }

    #[tokio::test]
    async fn test_mock_server_task_submit_and_list() {
        let server = build_mock_server();
        server
            .handle_request(
                "initialize",
                Some(1),
                json!({
                    "protocolVersion": "2024-11-05",
                    "clientInfo": {"name": "test", "version": "1.0"},
                }),
            )
            .await;

        // Submit a task
        let resp = server
            .handle_request(
                "tools/call",
                Some(3),
                json!({
                    "name": "task.submit",
                    "arguments": {"prompt": "explain main.rs"}
                }),
            )
            .await;
        assert!(resp.is_some());
        let val = resp.unwrap();
        let text = val["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Task submitted"));

        // List tasks
        let resp = server
            .handle_request(
                "tools/call",
                Some(4),
                json!({
                    "name": "task.list",
                    "arguments": {}
                }),
            )
            .await;
        assert!(resp.is_some());
    }

    #[tokio::test]
    async fn test_mock_server_set_model() {
        let server = build_mock_server();
        server
            .handle_request(
                "initialize",
                Some(1),
                json!({
                    "protocolVersion": "2024-11-05",
                    "clientInfo": {"name": "test", "version": "1.0"},
                }),
            )
            .await;

        let resp = server
            .handle_request(
                "tools/call",
                Some(3),
                json!({
                    "name": "agent.set_model",
                    "arguments": {"model": "mock-model-v2"}
                }),
            )
            .await;
        let val = resp.unwrap();
        let text = val["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("mock-model-v1"));
        assert!(text.contains("mock-model-v2"));
    }

    #[tokio::test]
    async fn test_mock_server_permissions() {
        let server = build_mock_server();
        server
            .handle_request(
                "initialize",
                Some(1),
                json!({
                    "protocolVersion": "2024-11-05",
                    "clientInfo": {"name": "test", "version": "1.0"},
                }),
            )
            .await;

        let resp = server
            .handle_request(
                "tools/call",
                Some(3),
                json!({
                    "name": "agent.permissions",
                    "arguments": {}
                }),
            )
            .await;
        let val = resp.unwrap();
        let text = val["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Bash"));
        assert!(text.contains("Allow"));
    }

    #[tokio::test]
    async fn test_mock_server_token_usage() {
        let server = build_mock_server();
        server
            .handle_request(
                "initialize",
                Some(1),
                json!({
                    "protocolVersion": "2024-11-05",
                    "clientInfo": {"name": "test", "version": "1.0"},
                }),
            )
            .await;

        let resp = server
            .handle_request(
                "tools/call",
                Some(3),
                json!({
                    "name": "agent.token_usage",
                    "arguments": {"window": "today"}
                }),
            )
            .await;
        let val = resp.unwrap();
        let text = val["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("total_input"));
        assert!(text.contains("total_output"));
        // No cost fields
        assert!(!text.contains("cost"));
    }
}
