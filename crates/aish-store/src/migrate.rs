//! Schema migrations.

use anyhow::Result;
use rusqlite::Connection;
use tracing::info;

const MIGRATIONS: &[&str] = &[
    // 001: Initial schema
    r#"
    CREATE TABLE IF NOT EXISTS adapters (
        id TEXT PRIMARY KEY,
        alias TEXT,
        transport_type TEXT NOT NULL,
        transport_config TEXT NOT NULL,
        default_model TEXT,
        timeout_ms INTEGER DEFAULT 300000,
        created_at TEXT DEFAULT (datetime('now')),
        last_seen TEXT
    );

    CREATE TABLE IF NOT EXISTS tasks (
        id TEXT PRIMARY KEY,
        agent_id TEXT NOT NULL REFERENCES adapters(id),
        fan_out_group_id TEXT,
        prompt_preview TEXT NOT NULL,
        prompt_full TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'queued',
        model TEXT,
        priority INTEGER DEFAULT 0,
        progress REAL DEFAULT 0.0,
        result_json TEXT,
        error TEXT,
        created_at TEXT DEFAULT (datetime('now')),
        started_at TEXT,
        completed_at TEXT
    );

    CREATE INDEX IF NOT EXISTS idx_tasks_agent ON tasks(agent_id);
    CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
    CREATE INDEX IF NOT EXISTS idx_tasks_fanout ON tasks(fan_out_group_id);

    CREATE TABLE IF NOT EXISTS fan_out_groups (
        id TEXT PRIMARY KEY,
        prompt TEXT NOT NULL,
        strategy TEXT NOT NULL,
        merged_result_json TEXT,
        created_at TEXT DEFAULT (datetime('now')),
        completed_at TEXT
    );

    CREATE TABLE IF NOT EXISTS tool_calls (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        task_id TEXT REFERENCES tasks(id),
        agent_id TEXT NOT NULL,
        tool_name TEXT NOT NULL,
        arguments_json TEXT,
        result_summary TEXT,
        status TEXT,
        started_at TEXT DEFAULT (datetime('now')),
        duration_ms INTEGER
    );

    CREATE INDEX IF NOT EXISTS idx_tool_calls_task ON tool_calls(task_id);
    CREATE INDEX IF NOT EXISTS idx_tool_calls_agent ON tool_calls(agent_id);

    CREATE TABLE IF NOT EXISTS token_usage (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        agent_id TEXT NOT NULL,
        task_id TEXT,
        model TEXT NOT NULL,
        input_tokens INTEGER NOT NULL,
        output_tokens INTEGER NOT NULL,
        cache_write_tokens INTEGER DEFAULT 0,
        cache_read_tokens INTEGER DEFAULT 0,
        recorded_at TEXT DEFAULT (datetime('now'))
    );

    CREATE INDEX IF NOT EXISTS idx_token_usage_agent ON token_usage(agent_id);
    CREATE INDEX IF NOT EXISTS idx_token_usage_time ON token_usage(recorded_at);

    CREATE TABLE IF NOT EXISTS permission_audit (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        agent_id TEXT NOT NULL,
        tool_name TEXT NOT NULL,
        old_permit TEXT,
        new_permit TEXT NOT NULL,
        reason TEXT,
        changed_at TEXT DEFAULT (datetime('now'))
    );

    CREATE TABLE IF NOT EXISTS mcp_connection_log (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        agent_id TEXT NOT NULL,
        server_name TEXT NOT NULL,
        event TEXT NOT NULL,
        error TEXT,
        recorded_at TEXT DEFAULT (datetime('now'))
    );

    CREATE TABLE IF NOT EXISTS bands (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL UNIQUE,
        isolation_level TEXT NOT NULL,
        config_json TEXT NOT NULL,
        status TEXT DEFAULT 'active',
        created_at TEXT DEFAULT (datetime('now')),
        destroyed_at TEXT
    );

    CREATE TABLE IF NOT EXISTS schema_version (
        version INTEGER PRIMARY KEY,
        applied_at TEXT DEFAULT (datetime('now'))
    );
    "#,
];

/// Apply all pending migrations.
pub fn run_migrations(conn: &Connection) -> Result<()> {
    // Ensure schema_version table exists
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT DEFAULT (datetime('now'))
        )",
    )?;

    let current_version: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    for (i, migration_sql) in MIGRATIONS.iter().enumerate() {
        let version = (i + 1) as i32;
        if version > current_version {
            info!(version, "Applying migration");
            conn.execute_batch(migration_sql)?;
            conn.execute(
                "INSERT INTO schema_version (version) VALUES (?1)",
                [version],
            )?;
        }
    }

    info!(
        current = current_version,
        latest = MIGRATIONS.len(),
        "Migrations up to date"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        // Run twice — should be idempotent
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"adapters".to_string()));
        assert!(tables.contains(&"tasks".to_string()));
        assert!(tables.contains(&"tool_calls".to_string()));
        assert!(tables.contains(&"token_usage".to_string()));
        assert!(tables.contains(&"permission_audit".to_string()));
        assert!(tables.contains(&"mcp_connection_log".to_string()));
        assert!(tables.contains(&"bands".to_string()));
        assert!(tables.contains(&"fan_out_groups".to_string()));
    }
}
