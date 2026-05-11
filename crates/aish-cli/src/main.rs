//! AISH CLI — command-line interface for AI agent management.
//!
//! Subcommands:
//! - `aish exec`       — submit a task to agent(s)
//! - `aish agent`      — manage agent adapters
//! - `aish band`       — manage isolation environments
//! - `aish tui`        — launch TUI (deferred to M2)
//! - `aish daemon`     — start background daemon (deferred to M2)

use aish_core::band::Band;
use aish_core::config::AdaptersConfig;
use aish_core::event::EventBus;
use aish_core::registry::AgentRegistry;
use aish_core::scheduler::TaskScheduler;
use aish_core::types::*;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "aish", version, about = "AI Agent Shell — manage AI coding agents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Override config directory
    #[arg(long, env = "AISH_CONFIG_DIR")]
    config_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Submit a task to an agent (or fan-out to multiple agents)
    Exec {
        /// The prompt to send
        prompt: String,

        /// Target agent (use --all for fan-out)
        #[arg(short, long)]
        agent: Option<String>,

        /// Send to all online agents
        #[arg(long, conflicts_with = "agent")]
        all: bool,

        /// Override model
        #[arg(short, long)]
        model: Option<String>,

        /// Task priority
        #[arg(long, default_value = "normal")]
        priority: String,
    },

    /// Manage agent adapters
    Agent {
        #[command(subcommand)]
        sub: AgentCommand,
    },

    /// Manage band isolation environments
    Band {
        #[command(subcommand)]
        sub: BandCommand,
    },

    /// Launch TUI (not yet implemented)
    Tui,

    /// Start daemon (not yet implemented)
    Daemon,
}

#[derive(Subcommand)]
enum AgentCommand {
    /// List registered agents
    List,
    /// Show agent details
    Show { id: String },
    /// Add an agent from config
    Add {
        /// Agent type (claude-code, hermes, openclaw)
        #[arg(long)]
        r#type: String,
        /// Local or remote
        #[arg(long, default_value = "local")]
        mode: String,
        /// Display alias
        #[arg(long)]
        name: Option<String>,
        /// SSH host (for remote)
        #[arg(long)]
        host: Option<String>,
        /// SSH port
        #[arg(long, default_value = "22")]
        port: u16,
        /// SSH user
        #[arg(long)]
        user: Option<String>,
        /// Default model
        #[arg(long)]
        model: Option<String>,
    },
}

#[derive(Subcommand)]
enum BandCommand {
    /// List all bands
    List,
    /// Show detailed status of a band
    Status { name: String },
    /// Create a new band
    Create {
        name: String,
        /// Isolation level: lightweight or standard
        #[arg(long, default_value = "lightweight")]
        isolation: String,
    },
    /// Destroy a band
    Destroy { name: String },
    /// Execute a command inside a band
    Exec {
        name: String,
        /// The command to run
        command: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let config_dir = cli
        .config_dir
        .unwrap_or_else(aish_core::config::config_dir);
    std::fs::create_dir_all(&config_dir)?;

    let event_bus = Arc::new(EventBus::default());
    let registry = AgentRegistry::new(event_bus.clone());
    let scheduler = TaskScheduler::new(event_bus);

    // Load adapters config if it exists
    let adapters_path = config_dir.join("adapters.ron");
    if adapters_path.exists() {
        let raw = std::fs::read_to_string(&adapters_path)?;
        let config: AdaptersConfig = ron::from_str(&raw)?;
        registry.load_from_config(&config);
    }

    match cli.command {
        Commands::Exec {
            prompt,
            agent,
            all,
            model,
            priority,
        } => {
            let priority = match priority.as_str() {
                "low" => Priority::Low,
                "high" => Priority::High,
                "critical" => Priority::Critical,
                _ => Priority::Normal,
            };

            let req = TaskRequest {
                prompt: prompt.clone(),
                context: None,
                model,
                timeout: None,
                priority,
            };

            if all {
                let online = registry.online();
                if online.is_empty() {
                    anyhow::bail!("No online agents available");
                }
                let agent_ids: Vec<AgentId> = online.iter().map(|h| h.id.clone()).collect();
                let task_ids = scheduler.submit_all(&agent_ids, req);
                println!("Fan-out submitted to {} agents:", agent_ids.len());
                for (agent, task_id) in agent_ids.iter().zip(task_ids.iter()) {
                    println!("  {} → {}", agent, task_id);
                }
            } else if let Some(agent_id) = agent {
                let task_id = scheduler.submit(&AgentId(agent_id.clone()), req);
                println!("Task submitted: {} → {}", agent_id, task_id);
            } else {
                // Default: use first online agent
                let online = registry.online();
                if online.is_empty() {
                    anyhow::bail!("No online agents. Register an agent first with: aish agent add");
                }
                let task_id = scheduler.submit(&online[0].id, req);
                println!("Task submitted: {} → {}", online[0].id, task_id);
            }
        }

        Commands::Agent { sub } => match sub {
            AgentCommand::List => {
                let agents = registry.list();
                if agents.is_empty() {
                    println!("No agents registered. Add one with: aish agent add --type claude-code");
                } else {
                    println!("{:<30} {:<15} {:?}", "ID", "STATUS", "MODEL");
                    println!("{}", "-".repeat(60));
                    for agent in &agents {
                        let status = agent.status();
                        let status_str = match status {
                            AgentStatus::Online { .. } => "online".to_string(),
                            AgentStatus::Busy { .. } => "busy".to_string(),
                            AgentStatus::Degraded { .. } => "degraded".to_string(),
                            AgentStatus::Offline { .. } => "offline".to_string(),
                            AgentStatus::Connecting => "connecting".to_string(),
                        };
                        let model_str = match status {
                            AgentStatus::Online { ref model, .. } => model.clone(),
                            AgentStatus::Busy { ref model, .. } => model.clone(),
                            AgentStatus::Degraded { ref model, .. } => model.clone(),
                            _ => "-".to_string(),
                        };
                        println!("{:<30} {:<15} {}", agent.id, status_str, model_str);
                    }
                }
            }
            AgentCommand::Show { id } => {
                if let Some(agent) = registry.get(&AgentId(id.clone())) {
                    println!("ID:       {}", agent.id);
                    println!("Alias:    {:?}", agent.alias);
                    println!("Status:   {:?}", agent.status());
                    println!("Model:    {:?}", agent.default_model);
                } else {
                    anyhow::bail!("Agent '{}' not found", id);
                }
            }
            AgentCommand::Add {
                r#type,
                mode,
                name,
                host,
                port,
                user,
                model,
            } => {
                let id = format!(
                    "{}/{}",
                    mode,
                    name.as_deref().unwrap_or(&r#type)
                );

                let transport = if mode == "remote" {
                    aish_core::config::TransportConfig::Ssh {
                        host: host.unwrap_or_else(|| "localhost".into()),
                        port,
                        user: user.unwrap_or_else(|| "root".into()),
                        key_path: None,
                        remote_command: format!("aish-adapter-{}", r#type),
                    }
                } else {
                    aish_core::config::TransportConfig::Stdio {
                        command: format!("aish-adapter-{}", r#type),
                        args: vec![],
                        env: Default::default(),
                    }
                };

                let def = aish_core::config::AdapterDef {
                    id: id.clone(),
                    alias: name,
                    transport,
                    default_model: model,
                    timeout_ms: 300_000,
                };

                // Load existing config, append, save
                let mut config: AdaptersConfig = if adapters_path.exists() {
                    let raw = std::fs::read_to_string(&adapters_path)?;
                    ron::from_str(&raw).unwrap_or(AdaptersConfig { adapters: vec![] })
                } else {
                    AdaptersConfig { adapters: vec![] }
                };
                config.adapters.push(def.clone());
                let ron_str = ron::ser::to_string_pretty(&config, ron::ser::PrettyConfig::default())?;
                std::fs::write(&adapters_path, ron_str)?;

                registry.register(&def);
                println!("Agent registered: {}", id);
            }
        },

        Commands::Band { sub } => {
            let bands_root = Band::default_bands_root();
            match sub {
                BandCommand::List => {
                    let bands = Band::list(&bands_root)?;
                    if bands.is_empty() {
                        println!("No bands. Create one with: aish band create <name>");
                    } else {
                        println!("{:<20} {:<15} {}", "NAME", "ISOLATION", "CREATED");
                        println!("{}", "-".repeat(60));
                        for band in &bands {
                            println!(
                                "{:<20} {:?}  {}",
                                band.name, band.isolation, band.created_at
                            );
                        }
                    }
                }
                BandCommand::Status { name } => {
                    let bands = Band::list(&bands_root)?;
                    let band = bands
                        .iter()
                        .find(|b| b.name == name)
                        .ok_or_else(|| anyhow::anyhow!("Band '{}' not found", name))?;
                    let status = band.status()?;
                    println!("Band:       {}", status.name);
                    println!("Isolation:  {}", status.isolation);
                    println!("Root:       {}", status.root);
                    println!("Created:    {}", status.created_at);
                    println!("Exists:     {}", if status.exists { "yes" } else { "no" });
                    println!("DB:         {} ({} bytes)",
                        if status.db_exists { "yes" } else { "no" },
                        status.db_size_bytes,
                    );
                    println!("Adapters:   {}", if status.adapters_configured { "configured" } else { "not configured" });
                }
                BandCommand::Create { name, isolation } => {
                    let level = match isolation.as_str() {
                        "standard" => aish_core::band::BandIsolationLevel::Standard,
                        _ => aish_core::band::BandIsolationLevel::Lightweight,
                    };
                    let band = Band::create(&name, level, &bands_root)?;
                    println!("Band '{}' created at {}", band.name, band.root.display());
                    println!("Environment variables set:");
                    for (k, v) in band.env_vars() {
                        println!("  {}={}", k, v);
                    }
                }
                BandCommand::Destroy { name } => {
                    Band::destroy(&name, &bands_root)?;
                    println!("Band '{}' destroyed", name);
                }
                BandCommand::Exec { name, command } => {
                    let bands = Band::list(&bands_root)?;
                    let band = bands
                        .iter()
                        .find(|b| b.name == name)
                        .ok_or_else(|| anyhow::anyhow!("Band '{}' not found", name))?;

                    let parts: Vec<&str> = command.split_whitespace().collect();
                    let (cmd, args) = parts.split_first().unwrap();
                    let output = band.exec(cmd, args)?;

                    println!("{}", String::from_utf8_lossy(&output.stdout));
                    if !output.stderr.is_empty() {
                        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                    }
                }
            }
        }

        Commands::Tui => {
            // Spawn the TUI binary
            let status = std::process::Command::new("aish-tui").status();
            if let Err(e) = status {
                eprintln!("Failed to start TUI: {}", e);
                eprintln!("Make sure 'aish-tui' is installed (cargo install --path crates/aish-tui)");
            }
        }

        Commands::Daemon => {
            println!("Daemon mode is not yet implemented. Coming in M2.");
        }
    }

    Ok(())
}
