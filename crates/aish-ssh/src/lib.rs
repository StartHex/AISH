//! AISH SSH — SSH connectivity via system ssh command.
//!
//! Wraps the system `ssh` binary through StdioTransport, which means
//! all SSH config, keys, agents, known_hosts work automatically.
//! The NDJSON protocol flows over stdin/stdout through the SSH pipe.

use std::path::PathBuf;
use tracing::info;

/// SSH connection parameters.
#[derive(Debug, Clone)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    /// Optional path to identity file (-i)
    pub identity_file: Option<PathBuf>,
    /// SSH config file path (-F)
    pub config_file: Option<PathBuf>,
    /// Extra ssh args
    pub extra_args: Vec<String>,
}

impl Default for SshConfig {
    fn default() -> Self {
        SshConfig {
            host: "localhost".into(),
            port: 22,
            user: "root".into(),
            identity_file: None,
            config_file: None,
            extra_args: vec![],
        }
    }
}

impl SshConfig {
    /// Build the ssh command and its arguments for executing a remote command.
    /// The remote command's stdin/stdout will be connected through the SSH pipe.
    pub fn to_command(&self, remote_command: &str) -> (String, Vec<String>) {
        let mut args: Vec<String> = vec![];

        // Batch mode: don't prompt for password
        args.push("-o".into());
        args.push("BatchMode=yes".into());

        // Connection timeout
        args.push("-o".into());
        args.push("ConnectTimeout=10".into());

        // Server alive interval to detect dead connections
        args.push("-o".into());
        args.push("ServerAliveInterval=30".into());

        // Port
        if self.port != 22 {
            args.push("-p".into());
            args.push(self.port.to_string());
        }

        // Identity file
        if let Some(ref key) = self.identity_file {
            args.push("-i".into());
            args.push(key.to_string_lossy().to_string());
        }

        // Config file
        if let Some(ref cfg) = self.config_file {
            args.push("-F".into());
            args.push(cfg.to_string_lossy().to_string());
        }

        // Extra args
        args.extend(self.extra_args.clone());

        // Target host
        args.push(format!("{}@{}", self.user, self.host));

        // Remote command
        args.push(remote_command.to_string());

        info!(
            host = %self.host,
            port = self.port,
            user = %self.user,
            remote = %remote_command,
            "Built SSH command"
        );

        ("ssh".into(), args)
    }

    /// Build a standard `aish-adapter-<type>` invocation over SSH.
    pub fn adapter_command(&self, adapter_type: &str) -> (String, Vec<String>) {
        let remote_cmd = format!("aish-adapter-{}", adapter_type);
        self.to_command(&remote_cmd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_config_default() {
        let cfg = SshConfig::default();
        assert_eq!(cfg.host, "localhost");
        assert_eq!(cfg.port, 22);
    }

    #[test]
    fn test_to_command_basic() {
        let cfg = SshConfig {
            host: "10.0.0.5".into(),
            port: 22,
            user: "deploy".into(),
            ..Default::default()
        };
        let (cmd, args) = cfg.to_command("aish-adapter-claude");
        assert_eq!(cmd, "ssh");
        assert!(args.contains(&"deploy@10.0.0.5".to_string()));
        assert!(args.contains(&"aish-adapter-claude".to_string()));
        assert!(args.contains(&"BatchMode=yes".to_string()));
    }

    #[test]
    fn test_to_command_with_key_and_port() {
        let cfg = SshConfig {
            host: "prod.example.com".into(),
            port: 2222,
            user: "admin".into(),
            identity_file: Some(PathBuf::from("~/.ssh/prod_key")),
            ..Default::default()
        };
        let (_, args) = cfg.to_command("aish-adapter-hermes");
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"2222".to_string()));
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"~/.ssh/prod_key".to_string()));
    }
}
