//! Task scheduler — submit, cancel, retry tasks with optional fan-out.

use crate::event::{BusEvent, EventBus};
use crate::types::*;
use anyhow::Result;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{info, warn};

/// Manages task lifecycle across all agents.
pub struct TaskScheduler {
    tasks: DashMap<TaskId, TaskInfo>,
    event_bus: Arc<EventBus>,
}

impl TaskScheduler {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        TaskScheduler {
            tasks: DashMap::new(),
            event_bus,
        }
    }

    /// Submit a task to a single agent.
    pub fn submit(&self, agent_id: &AgentId, req: TaskRequest) -> TaskId {
        let task_id = TaskId::new();
        let now = Utc::now();

        let task_info = TaskInfo {
            id: task_id,
            agent_id: agent_id.clone(),
            prompt_preview: req.prompt.chars().take(200).collect(),
            status: TaskStatus::Queued,
            model: req.model.unwrap_or_else(|| "default".into()),
            progress: 0.0,
            priority: req.priority,
            created_at: now,
            completed_at: None,
        };

        self.tasks.insert(task_id, task_info.clone());

        info!(%task_id, %agent_id, "Task submitted");
        self.event_bus.publish(BusEvent::TaskSubmitted {
            agent: agent_id.clone(),
            task: task_id,
            prompt_preview: task_info.prompt_preview.clone(),
        });

        task_id
    }

    /// Fan-out: submit the same task to multiple agents.
    pub fn submit_all(&self, agents: &[AgentId], req: TaskRequest) -> Vec<TaskId> {
        agents
            .iter()
            .map(|agent_id| self.submit(agent_id, req.clone()))
            .collect()
    }

    /// Mark a task as running.
    pub fn set_running(&self, task_id: &TaskId) {
        if let Some(mut entry) = self.tasks.get_mut(task_id) {
            entry.status = TaskStatus::Running { progress: 0.0 };
            self.event_bus.publish(BusEvent::TaskStarted {
                agent: entry.agent_id.clone(),
                task: *task_id,
            });
        }
    }

    /// Update task progress.
    pub fn set_progress(&self, task_id: &TaskId, progress: f32, message: &str) {
        if let Some(mut entry) = self.tasks.get_mut(task_id) {
            entry.progress = progress;
            entry.status = TaskStatus::Running { progress };
            self.event_bus.publish(BusEvent::TaskProgress {
                agent: entry.agent_id.clone(),
                task: *task_id,
                progress,
                message: message.to_string(),
            });
        }
    }

    /// Mark a task as completed.
    pub fn set_done(&self, task_id: &TaskId, result: TaskResult) {
        if let Some(mut entry) = self.tasks.get_mut(task_id) {
            entry.status = TaskStatus::Done {
                result: result.clone(),
            };
            entry.progress = 1.0;
            entry.completed_at = Some(Utc::now());
            self.event_bus.publish(BusEvent::TaskCompleted {
                agent: entry.agent_id.clone(),
                task: *task_id,
                result,
            });
        }
    }

    /// Mark a task as failed.
    pub fn set_failed(&self, task_id: &TaskId, error: &str) {
        if let Some(mut entry) = self.tasks.get_mut(task_id) {
            entry.status = TaskStatus::Failed {
                error: error.to_string(),
            };
            entry.completed_at = Some(Utc::now());
            self.event_bus.publish(BusEvent::TaskFailed {
                agent: entry.agent_id.clone(),
                task: *task_id,
                error: error.to_string(),
            });
        }
    }

    /// Cancel a task.
    pub fn cancel(&self, task_id: &TaskId) -> Result<()> {
        if let Some(mut entry) = self.tasks.get_mut(task_id) {
            entry.status = TaskStatus::Cancelled;
            entry.completed_at = Some(Utc::now());
            self.event_bus.publish(BusEvent::TaskCancelled {
                agent: entry.agent_id.clone(),
                task: *task_id,
            });
            info!(%task_id, "Task cancelled");
            Ok(())
        } else {
            warn!(%task_id, "Task not found for cancellation");
            Ok(())
        }
    }

    /// Get a task by ID.
    pub fn get(&self, task_id: &TaskId) -> Option<TaskInfo> {
        self.tasks.get(task_id).map(|e| e.clone())
    }

    /// List tasks with optional filter.
    pub fn list(&self, filter: &TaskFilter) -> Vec<TaskInfo> {
        let mut tasks: Vec<TaskInfo> = self
            .tasks
            .iter()
            .filter(|entry| {
                if let Some(ref agent_id) = filter.agent_id {
                    if &entry.agent_id != agent_id {
                        return false;
                    }
                }
                true
            })
            .map(|entry| entry.value().clone())
            .collect();

        tasks.sort_by_key(|b| std::cmp::Reverse(b.created_at));

        let offset = filter.offset.unwrap_or(0);
        let limit = filter.limit.unwrap_or(100);
        tasks.into_iter().skip(offset).take(limit).collect()
    }

    /// Count tasks by status.
    pub fn count_by_status(&self, status: TaskStatus) -> usize {
        self.tasks
            .iter()
            .filter(|entry| {
                std::mem::discriminant(&entry.status) == std::mem::discriminant(&status)
            })
            .count()
    }
}
