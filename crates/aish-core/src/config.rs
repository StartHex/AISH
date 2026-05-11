//! Configuration types: adapters, daemon, settings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Top-level adapters config (~/.config/aish/adapters.ron)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptersConfig {
    pub adapters: Vec<AdapterDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterDef {
    pub id: String,
    pub alias: Option<String>,
    pub transport: TransportConfig,
    pub default_model: Option<String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    300_000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TransportConfig {
    #[serde(rename = "stdio")]
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
    },
    #[serde(rename = "ssh")]
    Ssh {
        host: String,
        #[serde(default = "default_ssh_port")]
        port: u16,
        user: String,
        key_path: Option<PathBuf>,
        remote_command: String,
    },
    #[serde(rename = "unix")]
    Unix {
        path: PathBuf,
    },
    #[serde(rename = "tcp")]
    Tcp {
        host: String,
        port: u16,
        #[serde(default)]
        tls: bool,
    },
}

fn default_ssh_port() -> u16 {
    22
}

/// Daemon config (~/.config/aish/daemon.ron)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub mcp_server: McpServerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub unix_socket: UnixSocketConfig,
    pub tcp: TcpConfig,
    pub auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnixSocketConfig {
    pub enabled: bool,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpConfig {
    pub enabled: bool,
    pub bind: String,
    pub port: u16,
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

/// User settings (~/.config/aish/settings.ron)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub hot_reload: HotReloadConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotReloadConfig {
    pub auto_watch: bool,
    pub manual_reload: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            hot_reload: HotReloadConfig {
                auto_watch: true,
                manual_reload: true,
            },
        }
    }
}

impl Default for DaemonConfig {
    fn default() -> Self {
        DaemonConfig {
            mcp_server: McpServerConfig {
                unix_socket: UnixSocketConfig {
                    enabled: true,
                    path: Some(PathBuf::from("~/.aish/daemon.sock")),
                },
                tcp: TcpConfig {
                    enabled: false,
                    bind: "127.0.0.1".into(),
                    port: 9876,
                    tls: None,
                },
                auth_token: None,
            },
        }
    }
}

/// Resolve config path. If AISH_BAND is set, config lives under the band root.
pub fn config_dir() -> PathBuf {
    if let Ok(band_root) = std::env::var("AISH_BAND_ROOT") {
        PathBuf::from(band_root).join("config")
    } else {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("aish")
    }
}
