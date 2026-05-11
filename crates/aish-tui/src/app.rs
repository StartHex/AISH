//! Application state and types for the AISH TUI.

#![allow(dead_code)]

use aish_core::types::{AgentId, AgentStatus, Permit, TaskId, TaskStatus};
use chrono::{DateTime, Utc};

/// Top-level tabs in the right panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Tasks,
    Activity,
    Models,
    Permissions,
    Skills,
    Mcp,
    Tokens,
    FanOut,
    Band,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[
            Tab::Tasks,
            Tab::Activity,
            Tab::Models,
            Tab::Permissions,
            Tab::Skills,
            Tab::Mcp,
            Tab::Tokens,
            Tab::FanOut,
            Tab::Band,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Tab::Tasks => "Tasks",
            Tab::Activity => "Activity",
            Tab::Models => "Models",
            Tab::Permissions => "Perms",
            Tab::Skills => "Skills",
            Tab::Mcp => "MCP",
            Tab::Tokens => "Tokens",
            Tab::FanOut => "Fan-out",
            Tab::Band => "Band",
        }
    }
}

/// Agent entry displayed in the left panel.
#[derive(Debug, Clone)]
pub struct AgentEntry {
    pub id: AgentId,
    pub alias: String,
    pub status: AgentStatus,
    pub model: String,
    pub current_task: Option<String>,
    pub progress: Option<f32>,
    pub tokens_input: u64,
    pub tokens_output: u64,
    pub uptime: String,
}

/// Task entry displayed in the Tasks tab.
#[derive(Debug, Clone)]
pub struct TaskEntry {
    pub id: TaskId,
    pub agent: AgentId,
    pub prompt: String,
    pub status: TaskStatus,
    pub progress: Option<f32>,
    pub duration: Option<String>,
}

/// Activity log entry (tool call).
#[derive(Debug, Clone)]
pub struct ActivityEntry {
    pub timestamp: DateTime<Utc>,
    pub agent: AgentId,
    pub task_id: Option<TaskId>,
    pub tool: String,
    pub args: String,
    pub result: String,
    pub duration_ms: u64,
    pub success: bool,
}

/// Model entry for the Models tab.
#[derive(Debug, Clone)]
pub struct ModelEntry {
    pub id: String,
    pub provider: String,
    pub context_window: Option<usize>,
    pub is_current: bool,
}

/// Permission entry for the Permissions tab.
#[derive(Debug, Clone)]
pub struct PermissionEntry {
    pub tool: String,
    pub permit: Permit,
    pub description: String,
    pub last_changed: Option<String>,
}

/// Skill entry for the Skills tab.
#[derive(Debug, Clone)]
pub struct SkillEntry {
    pub name: String,
    pub description: String,
    pub loaded: bool,
    pub call_count: u64,
}

/// MCP server entry.
#[derive(Debug, Clone)]
pub struct McpServerEntry {
    pub name: String,
    pub status: String,
    pub tools: usize,
}

/// Token usage summary.
#[derive(Debug, Clone)]
pub struct TokenSummary {
    pub total_input: u64,
    pub total_output: u64,
    pub by_model: Vec<ModelTokenEntry>,
    pub window: String,
}

#[derive(Debug, Clone)]
pub struct ModelTokenEntry {
    pub model: String,
    pub input: u64,
    pub output: u64,
}

/// Band entry for the Band tab.
#[derive(Debug, Clone)]
pub struct BandEntry {
    pub name: String,
    pub isolation: String,
    pub root: String,
    pub status: String,
}

/// Fan-out group entry.
#[derive(Debug, Clone)]
pub struct FanOutGroup {
    pub prompt: String,
    pub targets: Vec<FanOutTarget>,
    pub strategy: FanOutStrategy,
}

#[derive(Debug, Clone)]
pub struct FanOutTarget {
    pub agent: AgentId,
    pub online: bool,
    pub selected: bool,
    pub result: Option<FanOutResult>,
}

#[derive(Debug, Clone)]
pub struct FanOutResult {
    pub status: String,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub enum FanOutStrategy {
    Parallel,
    Sequential,
    Race,
    Vote,
}

/// Central application state.
pub struct App {
    pub agents: Vec<AgentEntry>,
    pub tasks: Vec<TaskEntry>,
    pub activity_log: Vec<ActivityEntry>,
    pub models: Vec<ModelEntry>,
    pub permissions: Vec<PermissionEntry>,
    pub skills: Vec<SkillEntry>,
    pub mcp_servers: Vec<McpServerEntry>,
    pub token_summary: Option<TokenSummary>,
    pub bands: Vec<BandEntry>,
    pub fan_out: Option<FanOutGroup>,
    pub selected_tab: Tab,
    pub selected_model_idx: usize,
    pub selected_perm_idx: usize,
    pub token_window: TokenWindow,
    pub agent_selected: usize,
    pub task_selected: usize,
    pub activity_selected: usize,
    pub command_input: String,
    pub status_message: String,
    pub should_quit: bool,
    pub task_scroll: usize,
    pub activity_scroll: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenWindow {
    Today,
    Month,
    Total,
}

impl TokenWindow {
    pub fn label(&self) -> &'static str {
        match self {
            TokenWindow::Today => "Today",
            TokenWindow::Month => "This Month",
            TokenWindow::Total => "Total",
        }
    }
}

fn task1_id() -> TaskId {
    TaskId(uuid::Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap())
}
fn task2_id() -> TaskId {
    TaskId(uuid::Uuid::parse_str("b2c3d4e5-f6a7-8901-bcde-f12345678901").unwrap())
}
fn task3_id() -> TaskId {
    TaskId(uuid::Uuid::parse_str("c3d4e5f6-a7b8-9012-cdef-123456789012").unwrap())
}

impl App {
    pub fn new() -> Self {
        let mut app = App {
            agents: Vec::new(),
            tasks: Vec::new(),
            activity_log: Vec::new(),
            models: Vec::new(),
            permissions: Vec::new(),
            skills: Vec::new(),
            mcp_servers: Vec::new(),
            token_summary: None,
            bands: Vec::new(),
            fan_out: None,
            selected_tab: Tab::Tasks,
            selected_model_idx: 0,
            selected_perm_idx: 0,
            token_window: TokenWindow::Today,
            agent_selected: 0,
            task_selected: 0,
            activity_selected: 0,
            command_input: String::new(),
            status_message: String::new(),
            should_quit: false,
            task_scroll: 0,
            activity_scroll: 0,
        };
        app.seed_mock_data();
        app
    }

    fn seed_mock_data(&mut self) {
        // Mock agents
        self.agents = vec![
            AgentEntry {
                id: AgentId::from("local/claude-code"),
                alias: "local/claude-code".into(),
                status: AgentStatus::Busy {
                    current_task: task1_id(),
                    progress: 0.72,
                    model: "claude-sonnet-4-6".into(),
                },
                model: "claude-sonnet-4-6".into(),
                current_task: Some("fix auth bug".into()),
                progress: Some(0.72),
                tokens_input: 420_000,
                tokens_output: 85_000,
                uptime: "3h 12m".into(),
            },
            AgentEntry {
                id: AgentId::from("ssh/prod-hermes"),
                alias: "ssh/prod-hermes".into(),
                status: AgentStatus::Online {
                    uptime: std::time::Duration::from_secs(12 * 3600 + 300),
                    model: "deepseek-v4-pro".into(),
                },
                model: "deepseek-v4-pro".into(),
                current_task: None,
                progress: None,
                tokens_input: 800_000,
                tokens_output: 210_000,
                uptime: "12h 5m".into(),
            },
            AgentEntry {
                id: AgentId::from("ssh/staging-openclaw"),
                alias: "ssh/staging-openclaw".into(),
                status: AgentStatus::Offline {
                    since: Utc::now() - chrono::Duration::hours(2),
                },
                model: "claude-opus-4-7".into(),
                current_task: None,
                progress: None,
                tokens_input: 50_000,
                tokens_output: 10_000,
                uptime: "--".into(),
            },
        ];

        // Mock tasks
        let t1 = task1_id();
        let t2 = task2_id();
        let t3 = task3_id();
        self.tasks = vec![
            TaskEntry {
                id: t1,
                agent: AgentId::from("local/claude-code"),
                prompt: "fix the auth bug in src/auth.rs".into(),
                status: TaskStatus::Running { progress: 0.72 },
                progress: Some(0.72),
                duration: Some("45s".into()),
            },
            TaskEntry {
                id: t2,
                agent: AgentId::from("ssh/prod-hermes"),
                prompt: "optimize the slow SQL query in reports".into(),
                status: TaskStatus::Done {
                    result: aish_core::types::TaskResult {
                        content: "Query optimized".into(),
                        tokens_used: None,
                        tool_calls: vec![],
                        duration: std::time::Duration::from_secs_f64(3.2),
                    },
                },
                progress: None,
                duration: Some("3.2s".into()),
            },
            TaskEntry {
                id: t3,
                agent: AgentId::from("local/claude-code"),
                prompt: "review PR #42 for security issues".into(),
                status: TaskStatus::Queued,
                progress: None,
                duration: None,
            },
        ];

        // Mock activity log
        let now = Utc::now();
        self.activity_log = vec![
            ActivityEntry {
                timestamp: now - chrono::Duration::seconds(5),
                agent: AgentId::from("local/claude-code"),
                task_id: Some(t1),
                tool: "Read".into(),
                args: "src/auth.rs:42".into(),
                result: "OK".into(),
                duration_ms: 45,
                success: true,
            },
            ActivityEntry {
                timestamp: now - chrono::Duration::seconds(10),
                agent: AgentId::from("ssh/prod-hermes"),
                task_id: Some(t2),
                tool: "Grep".into(),
                args: "\"validate_token\"".into(),
                result: "3 matches".into(),
                duration_ms: 12,
                success: true,
            },
            ActivityEntry {
                timestamp: now - chrono::Duration::seconds(15),
                agent: AgentId::from("local/claude-code"),
                task_id: Some(t1),
                tool: "Edit".into(),
                args: "src/auth.rs:42".into(),
                result: "applied".into(),
                duration_ms: 230,
                success: true,
            },
            ActivityEntry {
                timestamp: now - chrono::Duration::seconds(20),
                agent: AgentId::from("ssh/prod-hermes"),
                task_id: Some(t2),
                tool: "Bash".into(),
                args: "cargo test".into(),
                result: "FAILED: 1 test".into(),
                duration_ms: 2100,
                success: false,
            },
            ActivityEntry {
                timestamp: now - chrono::Duration::seconds(25),
                agent: AgentId::from("local/claude-code"),
                task_id: Some(t1),
                tool: "Grep".into(),
                args: "\"fn login\"".into(),
                result: "2 matches".into(),
                duration_ms: 8,
                success: true,
            },
            ActivityEntry {
                timestamp: now - chrono::Duration::seconds(30),
                agent: AgentId::from("local/claude-code"),
                task_id: Some(t1),
                tool: "Bash".into(),
                args: "cargo build".into(),
                result: "OK (12s)".into(),
                duration_ms: 12000,
                success: true,
            },
        ];

        // Mock models — per-agent available models
        self.models = vec![
            ModelEntry {
                id: "claude-sonnet-4-6".into(),
                provider: "anthropic".into(),
                context_window: Some(200_000),
                is_current: true,
            },
            ModelEntry {
                id: "claude-opus-4-7".into(),
                provider: "anthropic".into(),
                context_window: Some(200_000),
                is_current: false,
            },
            ModelEntry {
                id: "claude-haiku-4-5".into(),
                provider: "anthropic".into(),
                context_window: Some(200_000),
                is_current: false,
            },
            ModelEntry {
                id: "deepseek-v4-pro".into(),
                provider: "deepseek".into(),
                context_window: Some(128_000),
                is_current: true,
            },
            ModelEntry {
                id: "deepseek-v3".into(),
                provider: "deepseek".into(),
                context_window: Some(64_000),
                is_current: false,
            },
            ModelEntry {
                id: "qwen-3-max".into(),
                provider: "alibaba".into(),
                context_window: Some(128_000),
                is_current: false,
            },
        ];

        // Mock permissions
        self.permissions = vec![
            PermissionEntry {
                tool: "Bash".into(),
                permit: Permit::Allow,
                description: "Execute shell commands".into(),
                last_changed: Some("2026-05-09".into()),
            },
            PermissionEntry {
                tool: "Read".into(),
                permit: Permit::Allow,
                description: "Read files".into(),
                last_changed: Some("2026-05-08".into()),
            },
            PermissionEntry {
                tool: "Write".into(),
                permit: Permit::Ask,
                description: "Write new files".into(),
                last_changed: Some("2026-05-08".into()),
            },
            PermissionEntry {
                tool: "Edit".into(),
                permit: Permit::Ask,
                description: "Edit existing files".into(),
                last_changed: Some("2026-05-08".into()),
            },
            PermissionEntry {
                tool: "Glob".into(),
                permit: Permit::Allow,
                description: "Search file names".into(),
                last_changed: None,
            },
            PermissionEntry {
                tool: "Grep".into(),
                permit: Permit::Allow,
                description: "Search file contents".into(),
                last_changed: None,
            },
            PermissionEntry {
                tool: "Agent".into(),
                permit: Permit::Allow,
                description: "Spawn sub-agents".into(),
                last_changed: None,
            },
            PermissionEntry {
                tool: "WebFetch".into(),
                permit: Permit::Deny,
                description: "Fetch URLs".into(),
                last_changed: Some("2026-05-10".into()),
            },
            PermissionEntry {
                tool: "WebSearch".into(),
                permit: Permit::Deny,
                description: "Search the web".into(),
                last_changed: Some("2026-05-10".into()),
            },
        ];

        // Mock skills
        self.skills = vec![
            SkillEntry {
                name: "code-review".into(),
                description: "Review pull requests for bugs and style".into(),
                loaded: true,
                call_count: 42,
            },
            SkillEntry {
                name: "security-review".into(),
                description: "Security audit for vulnerabilities".into(),
                loaded: true,
                call_count: 15,
            },
            SkillEntry {
                name: "simplify".into(),
                description: "Refactor code for readability".into(),
                loaded: true,
                call_count: 8,
            },
            SkillEntry {
                name: "test-gen".into(),
                description: "Generate unit tests".into(),
                loaded: false,
                call_count: 0,
            },
        ];

        // Mock MCP servers
        self.mcp_servers = vec![
            McpServerEntry {
                name: "filesystem".into(),
                status: "connected".into(),
                tools: 4,
            },
            McpServerEntry {
                name: "github".into(),
                status: "connected".into(),
                tools: 12,
            },
            McpServerEntry {
                name: "postgres".into(),
                status: "disconnected".into(),
                tools: 3,
            },
        ];

        // Mock token summary
        self.token_summary = Some(TokenSummary {
            total_input: 1_270_000,
            total_output: 305_000,
            by_model: vec![
                ModelTokenEntry {
                    model: "claude-sonnet-4-6".into(),
                    input: 420_000,
                    output: 85_000,
                },
                ModelTokenEntry {
                    model: "deepseek-v4-pro".into(),
                    input: 800_000,
                    output: 210_000,
                },
                ModelTokenEntry {
                    model: "claude-opus-4-7".into(),
                    input: 50_000,
                    output: 10_000,
                },
            ],
            window: "Today".into(),
        });

        // Mock bands
        self.bands = vec![
            BandEntry {
                name: "default".into(),
                isolation: "Lightweight".into(),
                root: "~/.local/share/aish/bands/default".into(),
                status: "active".into(),
            },
            BandEntry {
                name: "sandbox".into(),
                isolation: "Standard".into(),
                root: "/tmp/aish-band-sandbox".into(),
                status: "stopped".into(),
            },
        ];

        // Mock fan-out group
        self.fan_out = Some(FanOutGroup {
            prompt: "review the auth module for security issues".into(),
            targets: vec![
                FanOutTarget {
                    agent: AgentId::from("local/claude-code"),
                    online: true,
                    selected: true,
                    result: Some(FanOutResult {
                        status: "Done".into(),
                        summary: "Found 2 issues: SQL injection in login(), missing rate limiting"
                            .into(),
                    }),
                },
                FanOutTarget {
                    agent: AgentId::from("ssh/prod-hermes"),
                    online: true,
                    selected: true,
                    result: Some(FanOutResult {
                        status: "Running 72%".into(),
                        summary: "Analyzing auth.rs... found potential issue in login()".into(),
                    }),
                },
                FanOutTarget {
                    agent: AgentId::from("ssh/staging-openclaw"),
                    online: false,
                    selected: false,
                    result: Some(FanOutResult {
                        status: "Error".into(),
                        summary: "Connection timeout".into(),
                    }),
                },
            ],
            strategy: FanOutStrategy::Parallel,
        });
    }

    pub fn select_next_agent(&mut self) {
        if !self.agents.is_empty() {
            self.agent_selected = (self.agent_selected + 1) % self.agents.len();
        }
    }

    pub fn select_prev_agent(&mut self) {
        if !self.agents.is_empty() {
            self.agent_selected = self
                .agent_selected
                .checked_sub(1)
                .unwrap_or(self.agents.len() - 1);
        }
    }

    pub fn next_tab(&mut self) {
        let tabs = Tab::all();
        let idx = tabs
            .iter()
            .position(|t| *t == self.selected_tab)
            .unwrap_or(0);
        self.selected_tab = tabs[(idx + 1) % tabs.len()];
    }

    pub fn prev_tab(&mut self) {
        let tabs = Tab::all();
        let idx = tabs
            .iter()
            .position(|t| *t == self.selected_tab)
            .unwrap_or(0);
        self.selected_tab = tabs[idx.checked_sub(1).unwrap_or(tabs.len() - 1)];
    }
}
