//! End-to-end test: AISH CLI → MCP Client → Adapter Process → Result.
//! Spawns the mock adapter as a subprocess and exercises the full protocol.

use aish_mcp::client::McpClient;
use aish_mcp::transport::StdioTransport;
use serde_json::json;
use std::sync::Arc;

/// Spawn a mock adapter process and return a connected MCP client.
async fn connect_to_mock_adapter() -> McpClient {
    let transport = StdioTransport::spawn(
        "cargo",
        &[
            "run",
            "--bin",
            "aish-adapter-mock",
            "--manifest-path",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../aish-adapters/Cargo.toml"),
        ],
        &[],
    )
    .await
    .expect("Failed to spawn mock adapter");

    let client = McpClient::new(Arc::new(transport));
    client
        .initialize("e2e-test")
        .await
        .expect("MCP handshake failed");
    client
}

#[tokio::test]
async fn test_e2e_initialize_and_list_tools() {
    let client = connect_to_mock_adapter().await;

    let tools = client.list_tools().await.expect("Failed to list tools");
    assert!(!tools.is_empty(), "Should have registered tools");

    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(tool_names.contains(&"agent.status"));
    assert!(tool_names.contains(&"task.submit"));
    assert!(tool_names.contains(&"agent.permissions"));

    let _ = client.shutdown().await;
}

#[tokio::test]
async fn test_e2e_submit_task_and_get_result() {
    let client = connect_to_mock_adapter().await;

    // Submit a task
    let result = client
        .call_tool("task.submit", Some(json!({"prompt": "explain main.rs"})))
        .await
        .expect("Failed to submit task");

    let text = &result.content[0].text.as_ref().unwrap();
    assert!(text.contains("Task submitted"), "Got: {}", text);

    // List tasks
    let result = client
        .call_tool("task.list", Some(json!({})))
        .await
        .expect("Failed to list tasks");

    let task_text = &result.content[0].text.as_ref().unwrap();
    assert!(task_text.contains("mock-task"), "Got: {}", task_text);

    let _ = client.shutdown().await;
}

#[tokio::test]
async fn test_e2e_agent_status() {
    let client = connect_to_mock_adapter().await;

    let result = client
        .call_tool("agent.status", None)
        .await
        .expect("Failed to get status");

    let text = &result.content[0].text.as_ref().unwrap();
    let status: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(status["status"], "online");
    assert_eq!(status["model"], "mock-model-v1");

    let _ = client.shutdown().await;
}

#[tokio::test]
async fn test_e2e_model_switch() {
    let client = connect_to_mock_adapter().await;

    let result = client
        .call_tool("agent.set_model", Some(json!({"model": "mock-model-v2"})))
        .await
        .expect("Failed to switch model");

    let text = &result.content[0].text.as_ref().unwrap();
    assert!(text.contains("mock-model-v1"));
    assert!(text.contains("mock-model-v2"));

    // Verify status reflects the switch
    let result = client.call_tool("agent.status", None).await.unwrap();
    let text = &result.content[0].text.as_ref().unwrap();
    let status: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(status["model"], "mock-model-v2");

    let _ = client.shutdown().await;
}

#[tokio::test]
async fn test_e2e_permissions_flow() {
    let client = connect_to_mock_adapter().await;

    // Get permissions
    let result = client
        .call_tool("agent.permissions", None)
        .await
        .expect("Failed to get permissions");

    let text = &result.content[0].text.as_ref().unwrap();
    assert!(text.contains("Bash"));
    assert!(text.contains("Allow"));

    // Change a permission
    let result = client
        .call_tool(
            "agent.set_permission",
            Some(json!({"tool": "Bash", "permit": "Deny"})),
        )
        .await
        .expect("Failed to set permission");

    let text = &result.content[0].text.as_ref().unwrap();
    assert!(text.contains("Bash"));
    assert!(text.contains("Deny"));

    let _ = client.shutdown().await;
}

#[tokio::test]
async fn test_e2e_token_usage_no_cost() {
    let client = connect_to_mock_adapter().await;

    let result = client
        .call_tool("agent.token_usage", Some(json!({"window": "today"})))
        .await
        .expect("Failed to get token usage");

    let text = &result.content[0].text.as_ref().unwrap();
    let stats: serde_json::Value = serde_json::from_str(text).unwrap();

    // Should have token counts but NO cost
    assert!(stats.get("total_input").is_some());
    assert!(stats.get("total_output").is_some());
    assert!(stats.get("total_cost").is_none());

    let _ = client.shutdown().await;
}
