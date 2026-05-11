# AISH 设计方案 v0.3

> 状态: 已定稿 · 全部 11 项决策已确认

---

## 目录

1. [架构总览](#1-架构总览)
2. [协议设计：AISH/MCP](#2-协议设计aishmcp)
3. [适配器规范](#3-适配器规范)
4. [核心抽象与类型](#4-核心抽象与类型)
5. [事件总线](#5-事件总线)
6. [TUI 布局设计](#6-tui-布局设计)
7. [GUI 布局设计](#7-gui-布局设计)
8. [多 Agent 协同](#8-多-agent-协同)
9. [Band 隔离环境](#9-band-隔离环境)
10. [数据模型](#10-数据模型)
11. [Crate 详解](#11-crate-详解)
12. [里程碑规划](#12-里程碑规划)
13. [风险与缓解](#13-风险与缓解)
14. [待讨论](#14-待讨论)

---

## 1. 架构总览

```
                          ┌─────────────────────────┐
                          │   External MCP Clients   │
                          │  (IDE plugins, scripts)  │
                          └────────────┬────────────┘
                                       │ MCP (JSON-RPC)
                                       ▼
┌──────────────────────────────────────────────────────────────────┐
│                         AISH Daemon                               │
│                                                                   │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────────┐   │
│  │ aish-tui │  │ aish-gui │  │ aish-cli │  │ MCP Server     │   │
│  │(ratatui) │  │(cxx-qt)  │  │(clap)    │  │ (for external) │   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └───────┬────────┘   │
│       └──────────────┼─────────────┘               │             │
│                      ▼                             │             │
│            ┌──────────────────┐                    │             │
│            │   App Core       │◄───────────────────┘             │
│            │  (Event Loop)    │                                   │
│            └────────┬─────────┘                                   │
│                     ▼                                             │
│  ┌────────────────────────────────────────────┐                  │
│  │          Agent Registry + Scheduler         │                  │
│  │  ┌──────────────────────────────────────┐  │                  │
│  │  │  Fan-out Router (multi-agent协同)    │  │                  │
│  │  └──────────────────────────────────────┘  │                  │
│  └──────────────────┬─────────────────────────┘                  │
│                     │                                             │
│         ┌───────────┼───────────┬──────────┐                     │
│         ▼           ▼           ▼          ▼                     │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐            │
│  │ MCP      │ │ MCP      │ │ MCP      │ │ MCP      │   ...      │
│  │ Transport│ │ Transport│ │ Transport│ │ Transport│            │
│  │ (stdio)  │ │ (SSH)    │ │ (unix)   │ │ (TCP)    │            │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘            │
│       │            │            │            │                   │
│  ┌────┴────────────┴────────────┴────────────┴─────┐             │
│  │              MCP Protocol Bus                    │             │
│  │         (JSON-RPC 2.0 notifications)             │             │
│  └─────────────────────────────────────────────────┘             │
│                                                                   │
│  ┌──────────┐  ┌──────────┐  ┌────────────────────┐             │
│  │  SQLite   │  │  Config  │  │  Band Manager      │             │
│  │  Store    │  │  Loader  │  │  (隔离测试环境)     │             │
│  └──────────┘  └──────────┘  └────────────────────┘             │
└──────────────────────────────────────────────────────────────────┘
                                       ▲
                                       │ MCP (JSON-RPC over stdio/SSH)
                          ┌────────────┴─────────────────────┐
                          │       Adapter Processes           │
                          │                                   │
                          │  ┌────────┐ ┌────────┐ ┌───────┐ │
                          │  │ Claude │ │ Hermes │ │OpenClaw│ │
                          │  │Adapter │ │Adapter │ │Adapter │ │
                          │  └────────┘ └────────┘ └───────┘ │
                          │                                   │
                          │  Each adapter is an MCP Server    │
                          │  speaking JSON-RPC 2.0            │
                          └──────────────────────────────────┘
```

### 层级职责

| 层 | crate(s) | 职责 |
|---|---|---|
| 表现层 | `aish-tui`, `aish-gui` | ratatui TUI + Qt GUI，共享同一 `App Core` |
| CLI 层 | `aish-cli` | `aish exec/agent/band` 一次性命令 |
| Daemon 层 | `aish-daemon` | 后台进程——GUI 和外部 MCP Client 通过它连接 |
| 核心层 | `aish-core` | 所有类型定义、事件总线、注册中心、调度器 |
| 协议层 | `aish-mcp` | MCP 协议的 Rust 实现（Client + Transport 抽象） |
| 适配层 | `aish-adapters` | 各 AI Agent 的 MCP Server 适配器（独立进程） |
| 连接层 | `aish-ssh` | SSH 连接管理（russh 封装） |
| 持久层 | `aish-store` | SQLite CRUD + 迁移 |

### 关键设计原则

1. **一切皆 MCP**：适配器是 MCP Server，AISH Core 是 MCP Client。第三方适配器可跨语言。
2. **进程隔离**：每个适配器跑在独立进程中，崩溃不影响其他 agent。
3. **共享核心**：TUI、GUI、CLI 共享同一个 `AppCore`，只是表现层不同。
4. **事件驱动**：所有状态变更通过 `tokio::broadcast` 事件总线传播。

---

## 2. 协议设计：AISH/MCP

### 2.1 基础：JSON-RPC 2.0

所有通信走 JSON-RPC 2.0，每行一个 JSON 对象（NDJSON），与 MCP 完全兼容。

```
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{...}}
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05",...}}
{"jsonrpc":"2.0","method":"notifications/task/progress","params":{...}}
```

### 2.2 Transport 层

| Transport | 场景 | 实现 |
|---|---|---|
| `stdio` | 本地适配器进程 | stdin/stdout NDJSON |
| `ssh` | 远程适配器 | SSH exec + stdin/stdout NDJSON |
| `unix` | 本地 daemon ↔ GUI | Unix domain socket |
| `tcp` | 远程 AISH daemon / 跨机器 MCP Client | TLS + TCP |

### 2.2.1 Daemon MCP Server 配置

AISH Daemon 同时监听 Unix socket 和 TCP，两个通道均可通过配置和 TUI/GUI 界面开关：

```ron
// ~/.config/aish/daemon.ron
DaemonConfig(
    mcp_server: McpServerConfig(
        unix_socket: UnixSocketConfig(
            enabled: true,
            path: Some("~/.aish/daemon.sock"),
        ),
        tcp: TcpConfig(
            enabled: false,             // 默认关闭，需手动开启
            bind: "127.0.0.1",
            port: 9876,
            tls: TlsConfig(
                enabled: true,
                cert_path: "~/.aish/certs/server.crt",
                key_path: "~/.aish/certs/server.key",
            ),
        ),
        auth_token: Some("${AISH_DAEMON_TOKEN}"),  // 客户端需携此 token
    ),
)
```

TUI/GUI 中可通过 Settings 面板实时开关 Unix socket / TCP 监听，无需重启 daemon。

### 2.3 MCP 工具定义（每个适配器必须实现）

协议上走 `tools/list` + `tools/call`，语义上分为 4 组：

#### Agent 管理（Agent Management）

| Tool Name | Parameters | Returns | Description |
|---|---|---|---|
| `agent.status` | — | `AgentStatus` | 获取 agent 当前状态 |
| `agent.list_models` | — | `Vec<ModelInfo>` | 列出可用模型 |
| `agent.set_model` | `model: String` | — | 切换模型 |
| `agent.permissions` | — | `PermissionMatrix` | 获取权限矩阵 |
| `agent.set_permission` | `tool, permit` | — | 修改单个工具权限 |
| `agent.skills` | — | `Vec<SkillInfo>` | 列出已注册 skills |
| `agent.reload_skill` | `name: String` | — | 热加载指定 skill |
| `agent.mcp_servers` | — | `Vec<McpServerInfo>` | 列出 agent 连接的 MCP servers |
| `agent.token_usage` | `window: TimeWindow` | `TokenStats` | 获取 token 统计（仅计数，不含费用） |
| `agent.config` | — | `AgentConfig` | 获取 agent 完整配置 |

#### 任务管理（Task Management）

| Tool Name | Parameters | Returns | Description |
|---|---|---|---|
| `task.submit` | `TaskRequest` | `TaskId` | 提交新任务 |
| `task.cancel` | `task_id: String` | — | 取消任务 |
| `task.list` | `filter: TaskFilter` | `Vec<TaskInfo>` | 列出任务 |
| `task.retry` | `task_id: String` | `TaskId` | 重试失败任务 |

#### 数据查询（Data Query）

| Tool Name | Parameters | Returns | Description |
|---|---|---|---|
| `data.activity_log` | `filter: ActivityFilter` | `Vec<ActivityEntry>` | 查询调用活动 |
| `data.token_history` | `window, granularity` | `Vec<TokenDataPoint>` | 查询历史 token 消耗 |

#### 适配器元信息（Adapter Metadata）

| Tool Name | Parameters | Returns | Description |
|---|---|---|---|
| `adapter.version` | — | `String` | 适配器版本 |
| `adapter.capabilities` | — | `Vec<String>` | 声明支持的能力集 |

### 2.4 MCP 通知（Server → Client 推送）

| Notification | Payload | 触发时机 |
|---|---|---|
| `notifications/task/progress` | `{task_id, progress, message}` | 任务进度更新 |
| `notifications/task/completed` | `{task_id, result}` | 任务完成 |
| `notifications/task/failed` | `{task_id, error}` | 任务失败 |
| `notifications/tool_call` | `{task_id, tool, args, result, duration_ms}` | 工具调用完成 |
| `notifications/token_consumed` | `{model, input, output}` | Token 消耗（仅计数） |
| `notifications/agent/status_changed` | `{old, new}` | Agent 状态变更 |
| `notifications/model/switched` | `{from, to}` | 模型切换 |
| `notifications/permission/changed` | `{tool, old_permit, new_permit}` | 权限变更 |
| `notifications/mcp/server_connected` | `{server_name}` | MCP Server 上线 |
| `notifications/mcp/server_disconnected` | `{server_name, error}` | MCP Server 掉线 |

### 2.5 第三方适配器示例（Python）

外部进程适配器只需实现 MCP Server + 上述 tools + 发送通知：

```python
# aish-adapter-myagent — 一个最小的第三方适配器
import sys, json

def handle_request(req):
    method = req["method"]
    if method == "tools/list":
        return {"tools": [...]}  # 声明你的 tools
    elif method == "tools/call":
        name = req["params"]["name"]
        if name == "agent.status":
            return {"content": [{"type": "text", "text": json.dumps(my_status())}]}
        elif name == "task.submit":
            task_id = create_task(req["params"]["arguments"])
            return {"content": [{"type": "text", "text": task_id}]}
    # ...

def send_notification(method, params):
    msg = json.dumps({"jsonrpc": "2.0", "method": method, "params": params})
    sys.stdout.write(msg + "\n")
    sys.stdout.flush()

# Main loop: read JSON-RPC from stdin, write to stdout
for line in sys.stdin:
    req = json.loads(line)
    resp = handle_request(req)
    sys.stdout.write(json.dumps(resp) + "\n")
    sys.stdout.flush()
```

---

## 3. 适配器规范

### 3.1 适配器发现

AISH 从配置文件发现适配器：

```ron
// ~/.config/aish/adapters.ron
AdaptersConfig(
    adapters: [
        AdapterDef(
            id: "local/claude-code",
            alias: Some("dev-claude"),
            transport: Stdio(
                command: "aish-adapter-claude",
                args: ["--project", "/Users/me/work"],
                env: {},
            ),
            default_model: Some("claude-sonnet-4-6"),
            timeout_ms: 300000,
        ),
        AdapterDef(
            id: "ssh/prod-hermes",
            alias: Some("prod-hermes"),
            transport: Ssh(
                host: "10.0.0.5",
                port: 22,
                user: "root",
                key_path: Some("~/.ssh/id_ed25519"),
                remote_command: "aish-adapter-hermes",
            ),
            default_model: Some("deepseek-v4-pro"),
            timeout_ms: 600000,
        ),
        AdapterDef(
            id: "local/my-custom",
            alias: Some("my-agent"),
            transport: Stdio(
                command: "python3",
                args: ["~/adapters/myagent.py"],
                env: {"MY_KEY": "xxx"},
            ),
            default_model: None,
            timeout_ms: 120000,
        ),
    ],
)
```

### 3.2 适配器生命周期

```
AISH Core                        Adapter Process
    │                                   │
    │── spawn / SSH exec ──────────────▶│  进程启动
    │                                   │
    │── initialize ────────────────────▶│  握手 + 能力交换
    │◀── capabilities ─────────────────│
    │                                   │
    │── tools/list ────────────────────▶│  获取完整工具列表
    │◀── tool definitions ─────────────│
    │                                   │
    │  ┌──── 运行期 ────────────────┐   │
    │  │ tools/call + notifications │   │
    │  └────────────────────────────┘   │
    │                                   │
    │── shutdown ──────────────────────▶│  优雅关闭
    │◀── ack ──────────────────────────│
    │                                   X  进程退出
```

### 3.3 适配器健康检查

AISH 每 30s 发送 `ping` 请求，超时 5s 无响应则标记为 `Degraded`，连续 3 次超时标记为 `Offline` 并尝试自动重连。

### 3.4 配置热重载

`adapters.ron` 变更时，两种模式同时支持，用户可在配置中选择：

```ron
// ~/.config/aish/settings.ron
Settings(
    hot_reload: HotReload(
        auto_watch: true,         // 自动 watch 文件变更（inotify/kqueue）
        manual_reload: true,      // 允许 :reload 命令手动触发
    ),
)
```

| 模式 | 机制 | 行为 |
|---|---|---|
| **Auto Watch** | `notify` crate 监听 `adapters.ron` inode 变更 | 检测到变更后 500ms debounce，自动 diff 新旧配置：新增适配器→启动；移除→graceful shutdown；变更→重启适配器 |
| **Manual `:reload`** | TUI/GUI 命令 | 立即重新读取配置文件，同上 diff + apply |
| **冲突处理** | — | 两种模式同时开启时，auto watch 触发后重置 debounce timer，`:reload` 立即生效 |

热重载不影响正在运行的任务——已提交的任务继续执行直到完成，仅新提交走新配置。

---

## 4. 核心抽象与类型

### 4.1 中央类型（`aish-core::types`）

```rust
// ---- 标识符 ----
pub struct AgentId(String);          // "local/claude-code"
pub struct TaskId(Uuid);             // 全局唯一
pub struct AdapterId(String);        // "claude-code" / "hermes" / "my-adapter"

// ---- Agent 状态 ----
pub enum AgentStatus {
    Online { uptime: Duration, model: String },
    Busy { current_task: TaskId, progress: f32 },
    Degraded { reason: String },
    Offline { since: DateTime<Utc> },
    Connecting,
}

pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub context_window: Option<usize>,
    pub max_output_tokens: Option<usize>,
}

// ---- 任务 ----
pub struct TaskRequest {
    pub prompt: String,
    pub context: Option<Vec<Message>>,
    pub model: Option<String>,
    pub timeout: Option<Duration>,
    pub priority: Priority,
}

pub struct TaskInfo {
    pub id: TaskId,
    pub agent_id: AgentId,
    pub prompt: String,
    pub status: TaskStatus,
    pub model: String,
    pub progress: f32,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

pub enum TaskStatus {
    Queued,
    Running { progress: f32 },
    Done { result: TaskResult },
    Failed { error: String },
    Cancelled,
}

// ---- 权限 ----
pub enum Permit { Allow, Deny, Ask }

pub struct PermissionEntry {
    pub tool_name: String,
    pub permit: Permit,
    pub description: String,
}

pub struct PermissionMatrix {
    pub entries: Vec<PermissionEntry>,
    pub default_permit: Permit,
}

// ---- Skill ----
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub loaded: bool,
    pub call_count: u64,
    pub source_file: Option<PathBuf>,
}

// ---- MCP ----
pub struct McpServerInfo {
    pub name: String,
    pub status: McpConnectionStatus,
    pub tools_count: usize,
    pub resources_count: usize,
    pub last_connected: Option<DateTime<Utc>>,
}

pub enum McpConnectionStatus { Connected, Disconnected, Error(String) }

// ---- Token ----
pub struct TokenStats {
    pub total_input: u64,
    pub total_output: u64,
    pub by_model: HashMap<String, ModelTokenStats>,
}

pub struct ModelTokenStats {
    pub input: u64,
    pub output: u64,
    pub requests: u64,
}

pub struct TimeWindow {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,      // "user" | "assistant" | "system"
    pub content: String,
}
```

### 4.2 Agent 注册中心（`AgentRegistry`）

```rust
pub struct AgentRegistry {
    agents: DashMap<AgentId, AgentHandle>,
    config: RwLock<AdaptersConfig>,
}

impl AgentRegistry {
    /// 从配置文件加载并启动所有适配器
    pub async fn load_from_config(&self) -> Result<Vec<AgentId>>;
    /// 动态添加适配器（热插拔）
    pub async fn add(&self, def: AdapterDef) -> Result<AgentHandle>;
    /// 移除适配器
    pub async fn remove(&self, id: &AgentId) -> Result<()>;
    /// 获取所有 agent
    pub fn list(&self) -> Vec<AgentHandle>;
    /// 按 ID 查找
    pub fn get(&self, id: &AgentId) -> Option<AgentHandle>;
    /// 获取所有在线 agent
    pub fn online(&self) -> Vec<AgentHandle>;
}
```

### 4.3 任务调度器（`TaskScheduler`）

```rust
pub struct TaskScheduler {
    registry: Arc<AgentRegistry>,
    event_bus: Arc<EventBus>,
    store: Arc<Store>,
}

impl TaskScheduler {
    /// 单 agent 提交
    pub async fn submit(&self, agent: &AgentId, req: TaskRequest) -> Result<TaskId>;
    /// Fan-out：同时发给多个 agent
    pub async fn submit_all(&self, agents: &[AgentId], req: TaskRequest) -> Result<Vec<TaskId>>;
    /// 取消
    pub async fn cancel(&self, task_id: &TaskId) -> Result<()>;
    /// 重试
    pub async fn retry(&self, task_id: &TaskId) -> Result<TaskId>;
    /// 列出活跃任务
    pub async fn list(&self, filter: TaskFilter) -> Vec<TaskInfo>;
}
```

---

## 5. 事件总线

```rust
pub struct EventBus {
    tx: tokio::sync::broadcast::Sender<BusEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self;
    pub fn publish(&self, event: BusEvent);
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<BusEvent>;
}

pub enum BusEvent {
    // ---- Agent 生命周期 ----
    AgentOnline       { agent: AgentId },
    AgentOffline      { agent: AgentId, reason: String },
    AgentDegraded     { agent: AgentId, reason: String },
    AgentRecovered    { agent: AgentId },

    // ---- 任务生命周期 ----
    TaskSubmitted     { agent: AgentId, task: TaskId, prompt: String },
    TaskStarted       { agent: AgentId, task: TaskId },
    TaskProgress      { agent: AgentId, task: TaskId, progress: f32, message: String },
    TaskCompleted     { agent: AgentId, task: TaskId, result: TaskResult },
    TaskFailed        { agent: AgentId, task: Id, error: String },
    TaskCancelled     { agent: AgentId, task: TaskId },

    // ---- 工具调用 ----
    ToolCallStart     { agent: AgentId, task: TaskId, tool: String, args: Value },
    ToolCallEnd       { agent: AgentId, task: TaskId, tool: String, result: ToolResult, duration_ms: u64 },

    // ---- 模型 ----
    ModelSwitched     { agent: AgentId, from: String, to: String },
    ModelError        { agent: AgentId, model: String, error: String },

    // ---- Token ----
    TokenConsumed     { agent: AgentId, model: String, input: u64, output: u64 },

    // ---- 权限 ----
    PermissionChanged { agent: AgentId, tool: String, old: Permit, new: Permit },

    // ---- MCP ----
    McpServerUp       { agent: AgentId, server: String },
    McpServerDown     { agent: AgentId, server: String, error: String },
}
```

TUI/GUI 各自 `subscribe()` 这个事件总线，所有面板通过事件驱动更新。

---

## 6. TUI 布局设计

### 6.1 全局布局

```
┌──────────────────────────────────────────────────────────────────────┐
│ AISH v0.1 · 3 online · 2 tasks · 1.2M tokens · F1:Help              │
├─────────────────────────────┬────────────────────────────────────────┤
│  Agents                     │  [Tasks│Activity│Models│Perms│Skills│   │
│                             │   MCP│Tokens│Fan-out│Band]              │
│  ● local/claude-code        │                                        │
│    Busy: fix auth bug       │  ┌──────────────────────────────────┐  │
│    Progress: ████████░░ 72% │  │ Tab content...                   │  │
│    Model: claude-sonnet-4   │  │                                   │  │
│    Tokens: 420k               │  │                                   │  │
│                             │  │                                   │  │
│  ● ssh/prod-hermes          │  └──────────────────────────────────┘  │
│    Idle · uptime 12h        │                                        │
│    Model: deepseek-v4       │                                        │
│    Tokens: 800k               │                                        │
│                             │                                        │
│  ○ ssh/staging-openclaw     │                                        │
│    Offline · since 2h ago   │                                        │
│                             │                                        │
│  [+ Add Agent]              │                                        │
├─────────────────────────────┴────────────────────────────────────────┤
│ :fan-out "review the auth module" --all --model claude-sonnet-4       │
└──────────────────────────────────────────────────────────────────────┘
```

### 6.2 各 Tab 详情

#### Tasks Tab

```
┌─ Tasks ──────────────────────────────────────────────────────────────┐
│ Filter: [all▼] Sort: [recent▼] Search: [/________________]            │
│                                                                       │
│ ▶ local/claude-code ──────────────────────────────────────────────── │
│   ● fix auth bug            Running  ████████░░░░  72%  t_001        │
│   ○ review PR #42           Queued                   t_003        │
│                                                                       │
│ ▶ ssh/prod-hermes ────────────────────────────────────────────────── │
│   ● optimize sql query      Done ✓   3.2s             t_002        │
│                                                                       │
│ [Enter: detail] [Esc: cancel] [r: retry] [f: fan-out selected]       │
└───────────────────────────────────────────────────────────────────────┘
```

#### Activity Tab

```
┌─ Activity ───────────────────────────────────────────────────────────┐
│ Filter: [tool:Read▼] Agent: [all▼] Search: [/auth________]            │
│                                                                       │
│ 12:34:56  claude-code  t_001  Read     src/auth.rs:42         ✓ 45ms │
│ 12:34:55  hermes       t_002  Grep     "validate_token"       ✓ 12ms │
│ 12:34:54  claude-code  t_001  Edit     42: "fn auth"          ...    │
│ 12:34:52  hermes       t_002  Bash     cargo test             ✗ 2.1s │
│ 12:34:50  claude-code  t_001  Grep     "fn login"             ✓ 8ms  │
│                                                                       │
│ [/ search] [c: clear filter] [→ details]                             │
└───────────────────────────────────────────────────────────────────────┘
```

#### Models Tab

```
┌─ Models ──────────────────────────────────────────────────────────────┐
│ Global Default: claude-sonnet-4-6    [s: set global]                  │
│                                                                       │
│ Agent                 Current Model           Status    Tokens Today │
│ ──────────────────────────────────────────────────────────────────── │
│ local/claude-code  ● claude-sonnet-4-6       active    420k         │
│                       ├─ claude-opus-4-7     available               │
│                       ├─ claude-haiku-4-5    available               │
│                       └─ claude-sonnet-4     available               │
│                                                                       │
│ ssh/prod-hermes    ● deepseek-v4-pro         active    800k         │
│                       ├─ deepseek-v3         available               │
│                       └─ qwen-3-max          available               │
│                                                                       │
│ [Enter: switch model] [s: set as default for agent]                  │
│ [G: set global default]                                              │
└───────────────────────────────────────────────────────────────────────┘
```

#### Permissions Tab

```
┌─ Permissions ─────────────────────────────────────────────────────────┐
│ Agent: [local/claude-code▼]                    [Audit Log: 23 changes]│
│                                                                       │
│ Tool            Permit    Description             Last Changed        │
│ ──────────────────────────────────────────────────────────────────── │
│ Bash            [Allow▼]  Execute shell commands   2026-05-09         │
│ Read            [Allow▼]  Read files              2026-05-08         │
│ Write           [Ask  ▼]  Write new files         2026-05-08         │
│ Edit            [Ask  ▼]  Edit existing files     2026-05-08         │
│ Glob            [Allow▼]  Search file names       2026-05-07         │
│ Grep            [Allow▼]  Search file contents    2026-05-07         │
│ Agent           [Allow▼]  Spawn sub-agents        -- (default)       │
│ WebFetch        [Deny ▼]  Fetch URLs              2026-05-10         │
│                                                                       │
│ [Space: cycle permit] [a: allow all] [d: deny all] [→ audit log]     │
└───────────────────────────────────────────────────────────────────────┘
```

#### Fan-out Tab（多 Agent 协同）

Fan-out Tab 有三层视图，用户可在层间切换：

**Layer 1: 执行控制台（Execute）**

```
┌─ Fan-out ── [1.Execute│2.Compare│3.Split│4.Extract] ─────────────────┐
│ Mode: [Parallel▼]  Targets: [Selected▼]  Merge: [None▼]              │
│                                                                       │
│ Prompt: "review the auth module for security issues"                  │
│ Model Override: [claude-sonnet-4-6▼]                                 │
│                                                                       │
│ ┌─ Targets (3 agents) ──────────────────────────────────────────────┐│
│ │ ☑ local/claude-code   (online)                                    ││
│ │ ☑ ssh/prod-hermes     (online)                                    ││
│ │ ☐ ssh/staging-oc      (offline — skipped)                         ││
│ └────────────────────────────────────────────────────────────────────┘│
│                                                                       │
│ ┌─ Results ─────────────────────────────────────────────────────────┐│
│ │ claude-code  ✓  Done   "Found 2 issues: SQL injection in login.." ││
│ │ hermes       ⏳ 72%    "Analyzing auth.rs... found potential..."   ││
│ │ openclaw     ✗  Error  "Connection timeout"                       ││
│ └────────────────────────────────────────────────────────────────────┘│
│                                                                       │
│ [Enter: execute] [Tab: next view]                                     │
└───────────────────────────────────────────────────────────────────────┘
```

**Layer 2: Diff 对照视图（Compare）—— 代码类任务**

```
┌─ Fan-out ── [1.Execute│2.Compare│3.Split│4.Extract] ─────────────────┐
│ Base: [original file▼]  Side A: [claude-code▼]  Side B: [hermes▼]    │
│                                                                       │
│ ┌─ src/auth.rs (original) ──┬── claude-code patch ──────────────────┐│
│ │  42: fn validate(t: &str │  42: fn validate(token: &str) -> Resu  ││
│ │  43:   if t.is_empty() { │  43:   if token.is_empty() {           ││
│ │  44:     return Err(..)  │  44:     return Err(AuthError::Empty)  ││
│ │                          │ +45:   if token.len() > 256 {           ││ (+)
│ │                          │ +46:     return Err(AuthError::TooLong) ││ (+)
│ │  45:   }                 │  47:   }                               ││
│ └──────────────────────────┴────────────────────────────────────────┘│
│                                                                       │
│ ┌─ hermes patch ────────────────────────────────────────────────────┐│
│ │  42: fn validate(t: &str) -> Result<(), Error> {                   ││
│ │  43:   if t.is_empty() { return Err(Error::Invalid) }              ││
│ │ +44:   // TODO: add length check                                   ││ (~)
│ └────────────────────────────────────────────────────────────────────┘│
│                                                                       │
│ Legend: (+) added  (-) removed  (~) modified  (!) conflict           │
│ [Tab: next view] [←→: switch sides] [j/k: scroll]                    │
└───────────────────────────────────────────────────────────────────────┘
```

**Layer 3: 分屏查阅（Split）—— 独立滚动每个 Agent 的完整输出**

```
┌─ Fan-out ── [1.Execute│2.Compare│3.Split│4.Extract] ─────────────────┐
│ Focus: [claude-code▼]  [Auto-scroll: on]                              │
│                                                                       │
│ ┌─ claude-code (complete) ───┬── hermes (running 72%) ──────────────┐│
│ │                             │                                       ││
│ │  Found 2 security issues    │  Analyzing auth.rs...                ││
│ │  in the auth module:        │  Found potential issue in login()    ││
│ │                             │  ...                                 ││
│ │  1. SQL Injection in        │                                       ││
│ │     login() at line 42:     │                                       ││
│ │     The token parameter     │                                       ││
│ │     is directly interpolated│                                       ││
│ │     into the SQL query.     │                                       ││
│ │                             │                                       ││
│ │  2. Missing rate limiting   │                                       ││
│ │     on validate() - an      │                                       ││
│ │     attacker could brute    │                                       ││
│ │     force tokens.           │                                       ││
│ │                             │                                       ││
│ └─────────────────────────────┴───────────────────────────────────────┘│
│                                                                       │
│ [Tab: next view] [h/l: switch focus] [j/k: scroll focused]           │
└───────────────────────────────────────────────────────────────────────┘
```

**Layer 4: 关键信息提取（Extract）**

```
┌─ Fan-out ── [1.Execute│2.Compare│3.Split│4.Extract] ─────────────────┐
│ Auto-extracted from all agent outputs       [Severity filter: All▼]   │
│                                                                       │
│ ┌─ ⚠ Warnings ──────────────────────────────────────────────────────┐│
│ │ [claude-code] "login() at L42: token directly interpolated → SQL   ││
│ │                injection. HIGH confidence."                        ││
│ │ [hermes]      "validate() has no rate limit → brute force risk.    ││
│ │                MEDIUM confidence."                                 ││
│ └────────────────────────────────────────────────────────────────────┘│
│ ┌─ ⚡ Errors ────────────────────────────────────────────────────────┐│
│ │ [openclaw]    "Connection timeout after 30s. Agent may be down."   ││
│ └────────────────────────────────────────────────────────────────────┘│
│ ┌─ ❓ Low Confidence ───────────────────────────────────────────────┐│
│ │ [hermes]      "Possible race condition in parallel auth calls.     ││
│ │                LOW confidence — needs verification."               ││
│ │ [claude-code] "Might be related to issue #234. LOW confidence."    ││
│ └────────────────────────────────────────────────────────────────────┘│
│                                                                       │
│ Summary: 2 Warnings · 1 Error · 2 Low-Confidence · 0 Conflicts       │
│                                                                       │
│ [Enter: jump to source] [Tab: next view] [e: export extraction]      │
└───────────────────────────────────────────────────────────────────────┘
```

### 合并模式配置

```rust
pub enum FanOutMergeMode {
    /// 不合并，仅排列各 agent 原始输出（默认）
    None,
    /// Diff 模式：针对代码任务，与原文件 / agent 之间相互 diff
    Diff { baseline: DiffBaseline },
    /// 自动提取异常、警告、低置信度判断
    Extract,
}

pub enum DiffBaseline {
    /// 以原始文件为基线
    OriginalFile(PathBuf),
    /// 以某个 agent 的输出为基线
    AgentOutput(AgentId),
    /// agent 两两对比
    Pairwise,
}
```

#### Tokens Tab

```
┌─ Tokens ──────────────────────────────────────────────────────────────┐
│ Window: [Today▼]  Granularity: [Hourly▼]                             │
│                                                                       │
│ Period: Today | This Month | Total                                   │
│ Today's Total: 1,250,000 tokens (in: 920k / out: 330k)               │
│                                                                       │
│ Agent                 Model              Input      Output    Requests│
│ ──────────────────────────────────────────────────────────────────── │
│ local/claude-code  claude-sonnet-4-6  320,000    100,000      12    │
│ ssh/prod-hermes    deepseek-v4-pro    600,000    200,000      25    │
│ ──────────────────────────────────────────────────────────────────── │
│ TOTAL                                 920,000    300,000      37    │
│                                                                       │
│ Token Sparkline (last 24h, input + output):                          │
│ claude-code  ▁▂▃▅▂▁▃▄▆█▆▄▂▁▂▃▅▄▂▁▁▂▃▄                              │
│ hermes       ▁▁▂▃▃▄▅▃▂▁▁▂▃▄▅▆▄▃▂▁▁▂▃▄▅                              │
│                                                                       │
│ [t: switch window] [g: switch granularity]                           │
└───────────────────────────────────────────────────────────────────────┘
```

### 6.3 快捷键设计

| 键 | 全局 | Tasks | Activity | Models | Perms | Fan-out |
|---|---|---|---|---|---|---|
| `Tab` | 切换面板 | — | — | — | — | — |
| `1-8` | 直接跳 Tab | — | — | — | — | — |
| `q` | 退出 | — | — | — | — | — |
| `:` | 命令模式 | — | — | — | — | — |
| `/` | — | 搜索 | 搜索 | — | — | — |
| `Enter` | — | 任务详情 | — | 切换模型 | — | 执行 |
| `Esc` | — | 取消任务 | 清除过滤 | — | — | — |
| `r` | — | 重试 | — | — | — | — |
| `Space` | — | — | — | — | 循环权限 | 勾选 agent |
| `s` | — | — | — | 设默认 | — | — |
| `m` | — | — | — | — | — | 合并结果 |

### 6.4 命令模式（`:` 前缀）

```
:exec "prompt" --agent <id>          # 单 agent 执行
:fan-out "prompt" --all              # 全 agent 群发
:fan-out "prompt" --agents a,b,c     # 指定 agent 群发
:agent add <def>                     # 热添加适配器
:agent rm <id>                       # 移除适配器
:agent status <id>                   # 查看 agent 详情
:model switch <agent> <model>        # 切换模型
:model default <model>               # 设全局默认模型
:band create <name>                  # 创建隔离环境
:band destroy <name>                 # 销毁隔离环境
:band exec <name> "prompt"           # 在隔离环境中执行
:perm set <agent> <tool> allow|deny|ask
:skill reload <agent> <skill>
:mcp reconnect <agent> <server>
:export tasks|csv|json               # 导出数据
:help                                # 帮助
```

---

## 7. GUI 布局设计

### 7.1 总体布局（Qt / cxx-qt）

```
┌────────────────────────────────────────────────────────────────┐
│  AISH                         3 Online  2 Tasks  1.2M tok  [─□✕] │
├────────────┬───────────────────────────────────────────────────┤
│            │  ┌─ Tasks ──┬─ Activity ──┬─ Tokens ───────────┐ │
│  Agent     │  │          │             │                    │ │
│  Tree      │  │  (Tab content area)                        │ │
│            │  │                                             │ │
│  ● Claude  │  │                                             │ │
│    ├ Tasks │  │                                             │ │
│    ├ Models│  │                                             │ │
│    └ Perms │  │                                             │ │
│  ● Hermes  │  │                                             │ │
│  ● OpenClaw│  └─────────────────────────────────────────────┘ │
│            │                                                   │
│  [+ Add]   │                                                   │
│            │                                                   │
├────────────┴───────────────────────────────────────────────────┤
│  > fan-out "review auth" --all --model claude-sonnet-4           │
└────────────────────────────────────────────────────────────────┘
```

### 7.2 GUI 独有特性

| 特性 | 描述 |
|---|---|
| **拖拽式 Fan-out** | 从 Agent Tree 拖 agent 到 prompt 区域构建 fan-out 组 |
| **Token 趋势图表** | 使用 Qt Charts 画实时 token 消耗曲线 |
| **MCP 拓扑图** | 可视化 MCP server ↔ agent 的连接关系图 |
| **权限矩阵大表** | 可编辑的 agent × tool 权限大表格 |
| **系统托盘** | 最小化到托盘，显示活跃任务数 |
| **通知** | 任务完成/失败时系统通知 |

### 7.3 cxx-qt 架构

```rust
// aish-gui/src/main.rs — 简化示意

#[cxx_qt::bridge]
mod my_object {
    #[cxx_qt::qobject]
    pub struct AishGui {
        #[qproperty]
        agents: Vec<AgentViewModel>,
        #[qproperty]
        tasks: Vec<TaskViewModel>,
        #[qproperty]
        total_tokens: u64,
    }

    impl Default for AishGui { ... }

    impl cxx_qt::QObject<AishGui> {
        #[qinvokable]
        pub fn submit_task(&self, agent: QString, prompt: QString) { ... }
        #[qinvokable]
        pub fn fan_out(&self, agents: Vec<QString>, prompt: QString) { ... }
    }
}
```

GUI 和 TUI 通过共享 `aish-core::AppCore` 复用全部业务逻辑。

---

## 8. 多 Agent 协同

### 8.1 Fan-out 模式

```
                    ┌──────────┐
                    │  User    │
                    │  Prompt  │
                    └────┬─────┘
                         │ "review auth.rs"
                         ▼
              ┌──────────────────┐
              │  Fan-out Router  │
              │  (aish-core)     │
              └────┬──┬──┬──────┘
                   │  │  │
          ┌────────┘  │  └────────┐
          ▼           ▼           ▼
    ┌─────────┐ ┌─────────┐ ┌─────────┐
    │ Claude  │ │ Hermes  │ │OpenClaw │
    │ Code    │ │         │ │         │
    └────┬────┘ └────┬────┘ └────┬────┘
         │           │           │
         ▼           ▼           ▼
    [result_1]  [result_2]  [result_3]
         │           │           │
         └───────────┼───────────┘
                     ▼
              ┌──────────────┐
              │ Result Merger│
              │ (dedup +     │
              │  diff view)  │
              └──────────────┘
```

### 8.2 Fan-out 策略

```rust
pub enum FanOutStrategy {
    /// 并行：所有 agent 同时执行
    Parallel,
    /// 串行：按顺序执行，后一个可以看到前一个的结果
    Sequential,
    /// 竞赛：多个 agent 抢一个任务，谁先完成返回谁的结果
    Race { timeout: Duration },
    /// 投票：多个 agent 执行同一任务，按多数意见决定
    Vote { threshold: f32 },
}
```

### 8.3 结果合并（可配置，运行时切换）

默认不自动合并，每个 Agent 的输出整齐排列。用户可在 4 种视图间切换：

```rust
pub enum FanOutMergeMode {
    /// 不合并，仅排列各 agent 原始输出（默认）
    None,
    /// Diff 模式：代码类任务，与原文件或 agent 间相互 diff
    Diff { baseline: DiffBaseline },
    /// 分屏模式：独立滚动每个 agent 的完整输出
    Split,
    /// 关键信息提取：自动提取每个 agent 返回里的异常、警告、低置信度判断
    Extract,
}

pub enum DiffBaseline {
    /// 以原始文件为基线
    OriginalFile(PathBuf),
    /// 以某个 agent 的输出为基线
    AgentOutput(AgentId),
    /// agent 两两对比
    Pairwise,
}

pub struct MergedResult {
    pub prompt: String,
    pub merge_mode: FanOutMergeMode,
    pub individual_results: Vec<(AgentId, TaskResult)>,
    pub diff_view: Option<DiffData>,        // Diff 模式
    pub extractions: Option<ExtractionSet>,  // Extract 模式
    pub stats: FanOutStats,
}

/// 关键信息提取结果
pub struct ExtractionSet {
    pub warnings: Vec<ExtractedItem>,        // 异常/警告
    pub errors: Vec<ExtractedItem>,          // 错误
    pub low_confidence: Vec<ExtractedItem>,  // 低置信度判断
    pub conflicts: Vec<Conflict>,            // agent 间矛盾
}

pub struct ExtractedItem {
    pub agent_id: AgentId,
    pub severity: Severity,
    pub message: String,
    pub source_location: Option<String>,     // 原文引用
    pub confidence: Option<ConfidenceLevel>,
}

pub enum Severity { Warning, Error, Info }
pub enum ConfidenceLevel { High, Medium, Low }

pub struct FanOutStats {
    pub total_agents: usize,
    pub completed: usize,
    pub failed: usize,
    pub total_duration: Duration,
    pub total_tokens: u64,
}
```

合并模式在 TUI 中通过 Tab 切换（Execute → Compare → Split → Extract），在 GUI 中为可拖拽分屏面板。

---

## 9. Band 隔离环境

### 9.1 设计目标

- 测试新适配器或 prompt 策略时，不影响生产 agent
- 可快速创建/销毁
- 零外部依赖（不需要 Docker、Nix、VM）
- 支持不同隔离级别

### 9.2 隔离层级

```rust
pub enum BandIsolationLevel {
    /// 轻量：仅隔离 HOME 和 config
    Lightweight,
    /// 标准：隔离 HOME + 文件系统（tmpfs）+ 网络命名空间
    Standard,
    /// 完整：VM 级别隔离（未来，macOS Hypervisor.framework / Linux KVM）
    Full,
}
```

### 9.3 实现方案（macOS 优先）

```
Band "test-sandbox" 创建流程：
1. mkdir -p /tmp/aish-bands/test-sandbox/{home,config,data,tmp}
2. 复制 ~/.config/aish/adapters.ron → band/config/
3. 创建 band.toml 写入隔离级别 + 允许的工具白名单
4. 启动适配器进程时设置:
   - HOME=/tmp/aish-bands/test-sandbox/home
   - XDG_CONFIG_HOME=/tmp/aish-bands/test-sandbox/config
   - TMPDIR=/tmp/aish-bands/test-sandbox/tmp
   - AISH_BAND=test-sandbox
5. SQLite 数据库路径重定向到 band/data/aish.db
6. 可选：使用 macOS sandbox-exec 限制文件系统访问
```

```toml
# band.toml 示例
[band]
name = "test-sandbox"
isolation = "standard"
created_at = "2026-05-10T12:00:00Z"

[band.whitelist]
# 允许访问的路径
read_paths = ["/tmp/aish-bands/test-sandbox", "/Users/shx/work/repo"]
write_paths = ["/tmp/aish-bands/test-sandbox"]
# 允许的工具
allowed_tools = ["Read", "Write", "Edit", "Grep", "Glob", "Bash"]
# 禁止访问网络（假 SSH 环回）
network = "loopback-only"

[band.agents]
# band 内部的假 agent 配置
agents = [
    { id = "band/claude-test", adapter = "claude-code", model = "claude-haiku-4-5" },
]
```

### 9.4 Band 命令

```bash
aish band create test-sandbox                  # 创建
aish band ls                                    # 列出
aish band exec test-sandbox "explain main.rs"   # 在 band 中执行
aish band destroy test-sandbox                  # 销毁
aish band shell test-sandbox                    # 进入 band 的 shell
aish band export test-sandbox                   # 导出 band 数据用于分析
```

---

## 10. 数据模型

```sql
-- 适配器注册表
CREATE TABLE adapters (
    id TEXT PRIMARY KEY,                 -- "local/claude-code"
    alias TEXT,
    transport_type TEXT NOT NULL,        -- 'stdio','ssh','unix','tcp'
    transport_config TEXT NOT NULL,      -- JSON
    default_model TEXT,
    timeout_ms INTEGER DEFAULT 300000,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_seen DATETIME
);

-- 任务记录
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES adapters(id),
    fan_out_group_id TEXT,               -- NULL = 单 agent 任务，非 NULL = 属于某个 fan-out
    prompt_preview TEXT NOT NULL,        -- 前 200 字符
    prompt_full TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'queued',
    model TEXT,
    priority INTEGER DEFAULT 0,
    progress REAL DEFAULT 0.0,
    result_json TEXT,                    -- TaskResult JSON
    error TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    started_at DATETIME,
    completed_at DATETIME
);

CREATE INDEX idx_tasks_agent ON tasks(agent_id);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_fanout ON tasks(fan_out_group_id);

-- Fan-out 组（多 agent 协同）
CREATE TABLE fan_out_groups (
    id TEXT PRIMARY KEY,
    prompt TEXT NOT NULL,
    strategy TEXT NOT NULL,              -- 'parallel','sequential','race','vote'
    merged_result_json TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME
);

-- 工具调用记录
CREATE TABLE tool_calls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT REFERENCES tasks(id),
    agent_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    arguments_json TEXT,
    result_summary TEXT,                 -- 截断的结果
    status TEXT,                         -- 'success','error','timeout'
    started_at DATETIME,
    duration_ms INTEGER
);

CREATE INDEX idx_tool_calls_task ON tool_calls(task_id);
CREATE INDEX idx_tool_calls_agent ON tool_calls(agent_id);

-- Token 消耗
CREATE TABLE token_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    task_id TEXT,
    model TEXT NOT NULL,
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    cache_write_tokens INTEGER DEFAULT 0,
    cache_read_tokens INTEGER DEFAULT 0,
    recorded_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_token_usage_agent ON token_usage(agent_id);
CREATE INDEX idx_token_usage_time ON token_usage(recorded_at);

-- 权限变更审计
CREATE TABLE permission_audit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    old_permit TEXT,
    new_permit TEXT NOT NULL,
    reason TEXT,
    changed_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- MCP Server 连接日志
CREATE TABLE mcp_connection_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    server_name TEXT NOT NULL,
    event TEXT NOT NULL,                 -- 'connected','disconnected','error'
    error TEXT,
    recorded_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Band 环境记录
CREATE TABLE bands (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    isolation_level TEXT NOT NULL,
    config_json TEXT NOT NULL,
    status TEXT DEFAULT 'active',        -- 'active','destroyed'
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    destroyed_at DATETIME
);

```

### WAL 模式 + 写入合并

```rust
// aish-store 初始化
pub fn init_db(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    run_migrations(&conn)?;
    Ok(conn)
}
```

所有写入通过一个 `tokio::mpsc` 通道汇聚到单消费者线程，避免 SQLite 并发写入冲突。

---

## 11. Crate 详解

| Crate | 依赖 | 核心内容 |
|---|---|---|
| `aish-core` | tokio, serde, serde_json, uuid, chrono, dashmap | 类型定义、`AgentRegistry`、`TaskScheduler`、`EventBus`、`BandManager` |
| `aish-mcp` | tokio, serde_json | MCP Client 实现、Transport trait（StdioTransport, SshTransport, UnixTransport）、JSON-RPC 编解码 |
| `aish-adapters` | aish-mcp, tokio | 内置适配器二进制 crate：`aish-adapter-claude`、`aish-adapter-hermes`、`aish-adapter-openclaw`、`aish-adapter-generic` |
| `aish-ssh` | russh, tokio, anyhow | SSH 连接池、密钥管理、连接重试、tunnel |
| `aish-store` | rusqlite, serde, chrono | SQLite 初始化、migration、CRUD 封装 |
| `aish-tui` | ratatui, crossterm, tokio, aish-core | TUI 应用、所有面板、命令解析、主题 |
| `aish-gui` | cxx-qt, aish-core, tokio | Qt 桌面应用、QML 组件、图表、托盘 |
| `aish-cli` | clap, aish-core, tokio | `aish exec`、`aish agent`、`aish band` 子命令 |
| `aish-daemon` | aish-core, aish-mcp, tokio | 后台进程：启动所有适配器 + 暴露 MCP Server（Unix socket + TCP 双通道，可配置） |

### 依赖关系图

```
aish-tui ──────┐
aish-gui ──────┼── aish-core ── aish-mcp ── aish-ssh
aish-cli ──────┘       │
aish-daemon ───────────┤
                        ├── aish-store
                        └── aish-adapters (独立进程，通过 MCP 通信)
```

`aish-adapters` 与主进程是**进程隔离**的，不链接到同一个 binary。

---

## 12. 里程碑规划

### M0 — 脚手架 + 协议定型（当前）
- [ ] 设计方案定稿
- [ ] Workspace + 所有 crate 骨架
- [ ] `aish-mcp`: JSON-RPC 2.0 类型定义 + Transport trait
- [ ] `aish-core`: 核心类型定义（types.rs, event.rs）
- [ ] CI: `cargo check --workspace`, `cargo fmt --check`, `cargo clippy`

### M1 — 协议栈 + 单适配器
- [ ] `aish-mcp`: MCP Client 完整实现（initialize, tools/list, tools/call, notifications）
- [ ] `aish-mcp`: StdioTransport + SshTransport
- [ ] `aish-ssh`: russh 封装，连接池
- [ ] `aish-adapters`: aish-adapter-claude（第一个内置适配器）
- [ ] `aish-core`: AgentRegistry + TaskScheduler（单 agent）
- [ ] `aish-store`: SQLite schema + migration + 基本 CRUD
- [ ] `aish-cli`: `aish exec` 单 agent 调用可用

### M2 — TUI + GUI 基础框架
- [ ] `aish-tui`: App 框架 + 事件循环 + 基础面板（Agent 列表 + Tasks + Activity）
- [ ] `aish-gui`: Qt 窗口骨架 + Agent Tree + 基础 Tab
- [ ] `aish-daemon`: 后台进程模式（Unix socket + TCP 双通道）
- [ ] 命令模式（`:` 前缀）
- [ ] 配置热重载（auto watch + manual `:reload`）

### M3 — 全功能 TUI + 多 Agent 协同
- [ ] 模型面板、权限面板、Skill 面板、MCP 面板、Token 面板（仅计数，无费用）
- [ ] Fan-out 面板 4 层视图（Execute / Compare Diff / Split / Extract）
- [ ] Fan-out 面板 + Fan-out Router + 结果合并
- [ ] SSH 远程适配器支持
- [ ] 历史数据查询 + sparkline

### M4 — GUI 完善 + Band 环境 + 发布
- [ ] GUI 特有功能（拖拽 fan-out、拓扑图、趋势图、系统托盘）
- [ ] Band 隔离环境（Lightweight + Standard）
- [ ] 第三方适配器 SDK（Python 库 + 文档）
- [ ] 测试覆盖 + benchmark
- [ ] Homebrew / cargo-binstall 发布

---

## 13. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| MCP 协议不是为 agent 管理设计的，语义需要扩展 | 太多自定义 tool 失去 MCP 互操作性 | 尽量复用 MCP 原语；自定义部分文档化，争取上游化 |
| Claude Code / Hermes 没有 MCP Server 模式 | 适配器需要逆向或包装 CLI | 适配器用伪终端(pty)包装 CLI，解析输出；后续推动上游支持 |
| cxx-qt 生态不成熟 | GUI 开发阻塞 | 准备 fallback：Rust 后端 + PySide6 前端通过本地 MCP daemon 通信 |
| 多进程适配器的启动开销 | TUI 启动慢 | 适配器懒启动；并行初始化；连接池复用 |
| ratatui 不支持复杂图表 | Token 趋势图效果差 | sparkline + 数字表足够；GUI 用 Qt Charts 补足 |
| SSH 断线 | 远程任务丢失 | SQLite 持久化 pending tasks；自动重连 + 恢复 |
| Token 数据膨胀（每笔 tool call 都记录） | SQLite 文件变大 | 按时间分区 + 自动清理超过 90 天的细粒度数据，保留汇总 |

---

## 14. 决策记录

所有关键决策已确认，汇总如下：

| # | 决策点 | 结论 | 落地位置 |
|---|---|---|---|
| 1 | 项目名称 | **AISH**（全大写） | README, DESIGN |
| 2 | GUI 方案 | **TUI（ratatui）+ GUI（cxx-qt）双界面**，共享同一 AppCore；cxx-qt 为 Qt 绑定方案；fallback 为 Rust daemon + PySide6 | §7 |
| 3 | MVP 协议 | **协议抽象优先**——M0/M1 先做 MCP 协议栈 + Agent trait，再做具体适配器 | §2, §11 |
| 4 | Band 环境 | **自建隔离**：临时 HOME + 隔离 env + 可选 sandbox-exec；支持 Lightweight/Standard 两级 | §9 |
| 5 | 多 Agent 协同 | **MVP 包含**——Fan-out Router + 4 种策略 + 4 层视图（Execute/Compare/Split/Extract） | §6.2, §8 |
| 6 | 插件加载 | **外部进程 MCP Adapter**——适配器是独立 MCP Server 进程，通过 JSON-RPC 2.0 通信，跨语言 | §2, §3 |
| 7 | Qt 绑定 | **cxx-qt** | §7.3 |
| 8 | Daemon 暴露 | **Unix socket + TCP 双通道**，配置和 TUI/GUI 界面可选开关 | §2.2.1 |
| 9 | 结果合并 | **可配置，运行时切换**——默认不合并（排列原始输出）；额外提供 Diff/Split/Extract 视图 | §6.2, §8.3 |
| 10 | 配置热重载 | **Auto Watch + Manual `:reload` 双模式**，用户可选 | §3.4 |
| 11 | Token 定价 | **不追踪费用**，只显示 token 输入/输出量（当日/本月/总计） | §4.1, §6.2, §10 |
