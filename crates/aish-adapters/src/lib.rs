//! AISH Adapters — built-in MCP Server implementations for AI agents.
//! Each adapter is a separate binary (not yet implemented — M1 milestone).
//!
//! Planned:
//! - aish-adapter-claude: wraps Claude Code
//! - aish-adapter-hermes: wraps Hermes
//! - aish-adapter-openclaw: wraps OpenClaw
//! - aish-adapter-generic: generic SSE/MCP pass-through

pub mod claude;
pub mod generic;
pub mod hermes;
pub mod mock;
pub mod openclaw;
