//! Agent registry — manages adapter lifecycle and discovery.

use crate::config::AdaptersConfig;
use crate::event::EventBus;
use crate::types::*;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

/// Handle to a connected agent adapter.
#[derive(Clone)]
pub struct AgentHandle {
    pub id: AgentId,
    pub alias: Option<String>,
    pub connection_type: ConnectionType,
    pub default_model: Option<String>,
    pub timeout_ms: u64,
    status: Arc<parking_lot::RwLock<AgentStatus>>,
    event_bus: Arc<EventBus>,
    #[allow(dead_code)]
    created_at: chrono::DateTime<Utc>,
}

impl AgentHandle {
    pub fn status(&self) -> AgentStatus {
        self.status.read().clone()
    }

    pub fn set_status(&self, new_status: AgentStatus) {
        let mut s = self.status.write();
        *s = new_status.clone();

        match &new_status {
            AgentStatus::Online { model, .. } => {
                self.event_bus.publish(crate::event::BusEvent::AgentOnline {
                    agent: self.id.clone(),
                    model: model.clone(),
                });
            }
            AgentStatus::Offline { since } => {
                self.event_bus.publish(crate::event::BusEvent::AgentOffline {
                    agent: self.id.clone(),
                    reason: "connection lost".into(),
                    since: *since,
                });
            }
            AgentStatus::Degraded { reason, .. } => {
                self.event_bus.publish(crate::event::BusEvent::AgentDegraded {
                    agent: self.id.clone(),
                    reason: reason.clone(),
                });
            }
            _ => {}
        }
    }
}

pub struct AgentRegistry {
    agents: DashMap<AgentId, AgentHandle>,
    event_bus: Arc<EventBus>,
}

impl AgentRegistry {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        AgentRegistry {
            agents: DashMap::new(),
            event_bus,
        }
    }

    /// Register an adapter from its definition (does not connect yet).
    pub fn register(&self, def: &crate::config::AdapterDef) -> AgentHandle {
        let connection_type = match &def.transport {
            crate::config::TransportConfig::Stdio { command, args, env } => {
                ConnectionType::Stdio {
                    command: command.clone(),
                    args: args.clone(),
                    env: env.clone(),
                }
            }
            crate::config::TransportConfig::Ssh { host, port, user, key_path, remote_command: _ } => {
                ConnectionType::Ssh {
                    host: host.clone(),
                    port: *port,
                    user: user.clone(),
                    key_path: key_path.clone(),
                }
            }
            crate::config::TransportConfig::Unix { path } => ConnectionType::UnixSocket {
                path: path.clone(),
            },
            crate::config::TransportConfig::Tcp { host, port, tls } => ConnectionType::Tcp {
                host: host.clone(),
                port: *port,
                tls: *tls,
            },
        };

        let handle = AgentHandle {
            id: AgentId(def.id.clone()),
            alias: def.alias.clone(),
            connection_type,
            default_model: def.default_model.clone(),
            timeout_ms: def.timeout_ms,
            status: Arc::new(parking_lot::RwLock::new(AgentStatus::Connecting)),
            event_bus: self.event_bus.clone(),
            created_at: Utc::now(),
        };

        self.agents.insert(handle.id.clone(), handle.clone());
        handle
    }

    /// Load and register all adapters from config file.
    pub fn load_from_config(&self, config: &AdaptersConfig) -> Vec<AgentHandle> {
        config.adapters.iter().map(|def| self.register(def)).collect()
    }

    /// Get an agent by ID.
    pub fn get(&self, id: &AgentId) -> Option<AgentHandle> {
        self.agents.get(id).map(|h| h.clone())
    }

    /// List all registered agents.
    pub fn list(&self) -> Vec<AgentHandle> {
        self.agents.iter().map(|r| r.value().clone()).collect()
    }

    /// List online agents.
    pub fn online(&self) -> Vec<AgentHandle> {
        self.agents
            .iter()
            .filter(|r| matches!(*r.value().status.read(), AgentStatus::Online { .. } | AgentStatus::Busy { .. }))
            .map(|r| r.value().clone())
            .collect()
    }

    /// Remove an agent from the registry.
    pub fn remove(&self, id: &AgentId) -> Option<AgentHandle> {
        self.agents.remove(id).map(|(_, h)| h)
    }

    /// Count of registered agents.
    pub fn count(&self) -> usize {
        self.agents.len()
    }
}
