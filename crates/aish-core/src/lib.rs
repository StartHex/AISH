//! AISH core — shared types, event bus, registry, scheduler, band isolation.

pub mod band;
pub mod config;
pub mod event;
pub mod registry;
pub mod scheduler;
pub mod types;

// Re-exports
pub use band::{Band, BandStatus};
pub use config::{AdaptersConfig, DaemonConfig, McpServerConfig, Settings};
pub use event::{BusEvent, EventBus};
pub use registry::{AgentHandle, AgentRegistry};
pub use scheduler::TaskScheduler;
pub use types::*;
