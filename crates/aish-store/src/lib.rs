//! AISH Store — SQLite persistence layer.

mod migrate;

use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use tracing::info;

pub use migrate::run_migrations;

/// Open (or create) the AISH database at `path`, applying migrations.
pub fn open(path: &Path) -> Result<Connection> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(path)?;

    // Performance pragmas
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    run_migrations(&conn)?;
    info!(path = %path.display(), "Database opened");
    Ok(conn)
}

/// Resolve database path from AISH_DB_PATH env var or default location.
pub fn default_db_path() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("AISH_DB_PATH") {
        std::path::PathBuf::from(path)
    } else if let Ok(band_root) = std::env::var("AISH_BAND_ROOT") {
        std::path::PathBuf::from(band_root)
            .join("data")
            .join("aish.db")
    } else {
        dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("aish")
            .join("aish.db")
    }
}
