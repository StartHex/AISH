//! Smoke test that runs entirely within a band virtual environment.
//!
//! Usage: AISH_BAND_ROOT=/tmp/aish-bands/test-smoke cargo test --test band_smoke_test
//!
//! This test creates a temporary band, runs the DB init, and verifies
//! that no state leaks to the user's real ~/.aish or ~/.config/aish.

use std::path::PathBuf;

fn band_root() -> PathBuf {
    std::env::var("AISH_BAND_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let dir = std::env::temp_dir().join("aish-bands").join("test-smoke");
            std::fs::create_dir_all(&dir).unwrap();
            dir
        })
}

#[test]
fn test_band_directory_structure() {
    let root = band_root();
    let home = root.join("home");
    let config = root.join("config").join("aish");
    let data = root.join("data");

    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&config).unwrap();
    std::fs::create_dir_all(&data).unwrap();

    assert!(home.exists());
    assert!(config.exists());
    assert!(data.exists());
}

#[test]
fn test_band_env_isolation() {
    // When AISH_BAND is set, config_dir should point inside the band
    let root = band_root();
    std::env::set_var("AISH_BAND_ROOT", root.to_string_lossy().to_string());
    std::env::set_var("AISH_BAND", "test-smoke");

    let config_dir = aish_core::config::config_dir();
    assert!(
        config_dir.starts_with(&root),
        "Config dir {:?} should be inside band root {:?}",
        config_dir,
        root
    );

    let db_path = aish_store::default_db_path();
    assert!(
        db_path.starts_with(&root),
        "DB path {:?} should be inside band root {:?}",
        db_path,
        root
    );

    // Clean up env so other tests aren't affected
    std::env::remove_var("AISH_BAND_ROOT");
    std::env::remove_var("AISH_BAND");
}

#[test]
fn test_db_init_in_band() {
    let root = band_root();
    let data_dir = root.join("data");
    std::fs::create_dir_all(&data_dir).unwrap();
    let db_path = data_dir.join("test.db");

    // Open DB — should create all tables
    let conn = aish_store::open(&db_path).unwrap();

    // Verify a key table exists
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='tasks'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "Tasks table should exist");

    // Verify WAL mode is active
    let journal_mode: String = conn
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .unwrap();
    assert_eq!(journal_mode, "wal");

    // Clean up
    drop(conn);
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_band_create_and_list() {
    let tmp = std::env::temp_dir().join("aish-band-test");
    std::fs::create_dir_all(&tmp).unwrap();

    let band = aish_core::Band::create(
        "smoke-test",
        aish_core::band::BandIsolationLevel::Lightweight,
        &tmp,
    )
    .unwrap();

    assert_eq!(band.name, "smoke-test");
    assert!(band.root.join("home").exists());
    assert!(band.root.join("band.toml").exists());

    // List
    let bands = aish_core::Band::list(&tmp).unwrap();
    assert!(!bands.is_empty());

    // Destroy
    aish_core::Band::destroy("smoke-test", &tmp).unwrap();
    let bands = aish_core::Band::list(&tmp).unwrap();
    assert!(bands.is_empty());

    // Clean up
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_types_serde_roundtrip() {
    use aish_core::types::*;

    // TaskRequest serialization
    let req = TaskRequest {
        prompt: "test prompt".into(),
        context: None,
        model: Some("claude-sonnet-4".into()),
        timeout: None,
        priority: Priority::Normal,
    };
    let json = serde_json::to_string(&req).unwrap();
    let back: TaskRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(back.prompt, "test prompt");
    assert_eq!(back.model.unwrap(), "claude-sonnet-4");

    // Event serialization
    let event = aish_core::event::BusEvent::TaskSubmitted {
        agent: AgentId("test/agent".into()),
        task: TaskId::new(),
        prompt_preview: "hello".into(),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("TaskSubmitted"));
    assert!(json.contains("test/agent"));
}

#[test]
fn test_mcp_jsonrpc_request() {
    use aish_mcp::types::JsonRpcRequest;

    let req = JsonRpcRequest::new(1, "tools/list", None);
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"jsonrpc\":\"2.0\""));
    assert!(json.contains("\"id\":1"));
    assert!(json.contains("\"method\":\"tools/list\""));

    let notif = JsonRpcRequest::notification("notifications/task/progress", None);
    let json = serde_json::to_string(&notif).unwrap();
    assert!(!json.contains("\"id\""));
    assert!(json.contains("\"method\":\"notifications/task/progress\""));
}
