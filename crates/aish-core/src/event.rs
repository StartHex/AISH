//! Global event bus for real-time state propagation to TUI/GUI/CLI.

use crate::types::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

const DEFAULT_CAPACITY: usize = 1024;

pub struct EventBus {
    tx: broadcast::Sender<BusEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        EventBus { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BusEvent> {
        self.tx.subscribe()
    }

    pub fn publish(&self, event: BusEvent) {
        let _ = self.tx.send(event);
    }
}

impl Default for EventBus {
    fn default() -> Self {
        EventBus::new(DEFAULT_CAPACITY)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BusEvent {
    // ---- Agent lifecycle ----
    AgentOnline {
        agent: AgentId,
        model: String,
    },
    AgentOffline {
        agent: AgentId,
        reason: String,
        since: DateTime<Utc>,
    },
    AgentDegraded {
        agent: AgentId,
        reason: String,
    },
    AgentRecovered {
        agent: AgentId,
    },

    // ---- Task lifecycle ----
    TaskSubmitted {
        agent: AgentId,
        task: TaskId,
        prompt_preview: String,
    },
    TaskStarted {
        agent: AgentId,
        task: TaskId,
    },
    TaskProgress {
        agent: AgentId,
        task: TaskId,
        progress: f32,
        message: String,
    },
    TaskCompleted {
        agent: AgentId,
        task: TaskId,
        result: TaskResult,
    },
    TaskFailed {
        agent: AgentId,
        task: TaskId,
        error: String,
    },
    TaskCancelled {
        agent: AgentId,
        task: TaskId,
    },

    // ---- Tool calls ----
    ToolCallStart {
        agent: AgentId,
        task: TaskId,
        tool: String,
        args: serde_json::Value,
    },
    ToolCallEnd {
        agent: AgentId,
        task: TaskId,
        tool: String,
        result_summary: String,
        status: String,
        duration_ms: u64,
    },

    // ---- Model ----
    ModelSwitched {
        agent: AgentId,
        from: String,
        to: String,
    },
    ModelError {
        agent: AgentId,
        model: String,
        error: String,
    },

    // ---- Token ----
    TokenConsumed {
        agent: AgentId,
        model: String,
        input: u64,
        output: u64,
    },

    // ---- Permissions ----
    PermissionChanged {
        agent: AgentId,
        tool: String,
        old: Permit,
        new: Permit,
    },

    // ---- MCP ----
    McpServerUp {
        agent: AgentId,
        server: String,
    },
    McpServerDown {
        agent: AgentId,
        server: String,
        error: String,
    },
}
