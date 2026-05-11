# AISH — AI Agent Shell

通过类 SSH 的方式接入本地/远程的 Claude Code、OpenClaw、Hermes 等 AI Agent，提供统一的 TUI + GUI 界面进行多 Agent 协同、任务监控、模型管理、权限审计和 Token 消耗追踪。

**核心协议：MCP（Model Context Protocol）**——所有 Agent 适配器都是 MCP Server，AISH 作为 MCP Client 统一编排。

## 一句话

一台终端管理所有 AI 编码代理——TUI 看任务流水、GUI 拖拽协同、MCP 协议热插拔适配器。

## 核心功能

- **多 Agent 协同**：一个 prompt 并行发给多个 agent，汇总结果（fan-out / fan-in）
- **Agent 接入**：基于 MCP 协议（JSON-RPC 2.0 over stdio/SSH/Unix Socket）热插拔适配器
- **任务看板**：实时查看/取消/重试任意 agent 上的任务，带进度条和 ETA
- **活动流水**：可搜索、可过滤的 invocation 时间线（prompt → tool calls → result）
- **模型面板**：模型列表、当前激活模型、模型切换（per-agent or global）、模型状态监控
- **权限审计**：每个 agent 的工具权限矩阵，允许/拒绝清单，变更历史
- **Skill 注册表**：查看已加载 skills、skill 调用统计、热加载
- **MCP 监控**：MCP server 连接状态、resource 列表、tool 列表
- **Token 仪表盘**：按 agent/model/time 维度的 token 输入/输出量（当日/本月/总计），不追踪费用
- **Band 隔离测试**：自建虚拟环境，不影响本地 agent 正常运行
- **TUI + GUI 双界面**：ratatui 终端界面 + Qt 桌面应用，共享同一核心

## 技术栈

| 层 | 选择 | 理由 |
|---|---|---|
| 语言 | Rust | 性能 + 安全 + 系统级控制 |
| 异步运行时 | tokio | 事实标准，生态最全 |
| 适配器协议 | MCP (JSON-RPC 2.0) | 标准协议，外部进程热插拔，跨语言 |
| TUI | ratatui | 成熟稳定，社区活跃 |
| GUI | Qt via cxx-qt | 桌面级控件体系，跨平台 |
| SSH | russh | 纯 Rust 实现，无需 libssh |
| 持久化 | SQLite + rusqlite | 轻量、单文件、WAL 模式 |
| 序列化 | serde + JSON | 协议层 JSON，配置层 RON |
| Band 隔离 | 自建：tmpfs + namespace + 假 SSH | 零外部依赖 |

## 快速开始（规划中）

```bash
# 安装
cargo install aish

# 进入 TUI
aish tui

# 启动 GUI
aish gui

# 作为后台 daemon 运行（GUI / 外部 MCP Client 连接）
aish daemon

# 注册一个本地 Claude Code 适配器
aish adapter add claude-code --local --name "dev-claude"

# 注册一个远程 Hermes 适配器（SSH）
aish adapter add hermes --remote --name "prod-hermes" \
  --host 10.0.0.5 --port 22 --user root

# 给所有 agent 群发一个任务（fan-out）
aish exec --all "review src/main.rs for security issues"

# 给指定 agent 发任务
aish exec --agent dev-claude "explain the auth module"

# 启动 band 隔离测试环境
aish band create --name test-sandbox
aish band exec test-sandbox "run the test suite"
```

## 项目结构

```
AISH/
├── Cargo.toml              # workspace
├── README.md               # 本文件
├── DESIGN.md               # 详细设计方案
├── crates/
│   ├── aish-core/          # 核心类型、事件总线、注册中心、调度器
│   ├── aish-mcp/           # MCP 协议实现（client + transport）
│   ├── aish-adapters/      # 内置适配器（claude/hermes/openclaw/generic）
│   ├── aish-ssh/           # SSH 连接层（russh 封装）
│   ├── aish-store/         # 状态持久化（SQLite schema + 迁移）
│   ├── aish-tui/           # ratatui TUI 应用
│   ├── aish-gui/           # Qt GUI 应用（cxx-qt）
│   ├── aish-cli/           # CLI 入口（exec / agent / band 子命令）
│   └── aish-daemon/        # 后台 daemon（Unix socket + TCP 双通道，外部 MCP Client 可接入）
├── bands/                  # Band 隔离环境定义
│   ├── default/            # 默认 band：临时 HOME + 隔离 config + 假 SSH
│   └── full/               # 完整隔离：VM/nspawn（未来）
├── config/                 # 默认配置模板
└── tests/                  # 集成测试
```

## License

MIT
