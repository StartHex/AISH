//! Core type definitions shared across all crates.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

// ---- Identifiers ----

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for AgentId {
    fn from(s: &str) -> Self {
        AgentId(s.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub Uuid);

impl TaskId {
    pub fn new() -> Self {
        TaskId(Uuid::new_v4())
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0.to_string()[..8])
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AdapterId(pub String);

impl fmt::Display for AdapterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---- Connection ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    Stdio {
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    },
    Ssh {
        host: String,
        port: u16,
        user: String,
        key_path: Option<PathBuf>,
    },
    UnixSocket {
        path: PathBuf,
    },
    Tcp {
        host: String,
        port: u16,
        tls: bool,
    },
}

// ---- Agent Status ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Online {
        uptime: Duration,
        model: String,
    },
    Busy {
        current_task: TaskId,
        progress: f32,
        model: String,
    },
    Degraded {
        model: String,
        reason: String,
    },
    Offline {
        since: DateTime<Utc>,
    },
    Connecting,
}

// ---- Model ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub context_window: Option<usize>,
    pub max_output_tokens: Option<usize>,
}

// ---- Task ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub prompt: String,
    pub context: Option<Vec<Message>>,
    pub model: Option<String>,
    pub timeout: Option<Duration>,
    pub priority: Priority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: TaskId,
    pub agent_id: AgentId,
    pub prompt_preview: String,
    pub status: TaskStatus,
    pub model: String,
    pub progress: f32,
    pub priority: Priority,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Queued,
    Running { progress: f32 },
    Done { result: TaskResult },
    Failed { error: String },
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub content: String,
    pub tokens_used: Option<TokenDelta>,
    pub tool_calls: Vec<ToolCallRecord>,
    pub duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDelta {
    pub input: u64,
    pub output: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub result_summary: String,
    pub status: ToolCallStatus,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolCallStatus {
    Success,
    Error(String),
    Timeout,
}

// ---- Permissions ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Permit {
    Allow,
    Deny,
    Ask,
}

impl fmt::Display for Permit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Permit::Allow => write!(f, "Allow"),
            Permit::Deny => write!(f, "Deny"),
            Permit::Ask => write!(f, "Ask"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionEntry {
    pub tool_name: String,
    pub permit: Permit,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionMatrix {
    pub entries: Vec<PermissionEntry>,
    pub default_permit: Permit,
}

// ---- Skill ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub loaded: bool,
    pub call_count: u64,
    pub source_file: Option<PathBuf>,
}

// ---- MCP ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub status: McpConnectionStatus,
    pub tools_count: usize,
    pub resources_count: usize,
    pub last_connected: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpConnectionStatus {
    Connected,
    Disconnected,
    Error(String),
}

// ---- Token ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStats {
    pub total_input: u64,
    pub total_output: u64,
    pub by_model: HashMap<String, ModelTokenStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTokenStats {
    pub input: u64,
    pub output: u64,
    pub requests: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeWindowPreset {
    Today,
    ThisMonth,
    Total,
}

// ---- Message ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Message {
            role: "user".into(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Message {
            role: "assistant".into(),
            content: content.into(),
        }
    }
}

// ---- Task Filter ----

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskFilter {
    pub agent_id: Option<AgentId>,
    pub status: Option<TaskStatus>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

// ---- Activity ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub task_id: TaskId,
    pub agent_id: AgentId,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub result_summary: String,
    pub status: String,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivityFilter {
    pub agent_id: Option<AgentId>,
    pub tool_name: Option<String>,
    pub limit: Option<usize>,
}
