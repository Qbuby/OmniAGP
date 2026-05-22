# OmniAGP

LLM 驱动的 Agentic 游戏全流程开发引擎 — 从需求描述到完整可游玩游戏。

## 架构

```
OmniAGP/
├── crates/
│   ├── omni-core/            # 核心类型、trait、共享工具
│   ├── omni-llm/             # LLM 提供者抽象层 (OpenAI-compatible API)
│   ├── omni-orchestrator/    # Agent 编排引擎 — 管理多步骤游戏生成管线
│   ├── omni-assets/          # 资产生成管线 (2D/3D/音频)
│   ├── omni-godot/           # Godot 项目生成 — GDScript 代码生成 + 场景组装
│   ├── omni-rag/             # RAG 检索增强 (Qdrant 向量数据库)
│   ├── omni-designer/        # 游戏设计 Agent
│   ├── omni-qa/              # QA 自动测试 Agent
│   ├── omni-fixer/           # 自动修复 Agent
│   ├── omni-scheduler/       # 分布式任务调度器 (NATS JetStream)
│   ├── omni-plugin/          # 插件 SDK — 第三方扩展接口
│   ├── omni-templates/       # 游戏模板系统 (platformer/shooter/puzzle/RPG)
│   ├── omni-marketplace/     # 资产/插件市场
│   ├── omni-collaboration/   # 多人实时协作 (WebSocket CRDT)
│   ├── omni-workflow/        # 评审工作流引擎
│   ├── omni-auth/            # 认证授权 (OAuth2/JWT)
│   ├── omni-notify/          # 通知系统 (Webhook/Slack/Discord)
│   ├── omni-publish/         # 平台发布 (itch.io/Steam)
│   └── omnigp-cli/           # 命令行工具
├── services/
│   ├── api-gateway/          # HTTP API 网关 + Web Dashboard
│   ├── worker/               # GPU Worker — 分布式任务执行
│   ├── scheduler/            # 调度服务 — 任务分配与监控
│   ├── marketplace/          # 市场服务
│   └── collaboration-server/ # 协作服务 — 实时同步
├── docker/                   # Docker Compose + Kubernetes 部署
├── examples/                 # 插件示例
│   ├── plugin-cyberpunk-style/
│   ├── plugin-retro-exporter/
│   └── plugin-terrain-generator/
├── web/                      # React + TailwindCSS 前端 Dashboard
├── pipelines/                # GPU 推理管线 (3D/Audio)
└── tests/smoke/              # 端到端冒烟测试
```

## 技术栈

- **语言**: Rust (底层引擎) + GDScript (生成的游戏代码)
- **游戏引擎**: Godot 4.x
- **LLM**: 兼容 OpenAI API 的任意提供者
- **资产生成**: SDXL (2D), Hunyuan3D-2/TripoSR (3D), MusicGen (音频)
- **消息队列**: NATS JetStream (分布式任务调度)
- **向量数据库**: Qdrant (RAG 检索)
- **监控**: Prometheus + Grafana
- **前端**: React + TailwindCSS + WebSocket 实时推送
- **认证**: OAuth2 (GitHub) + JWT

## 快速开始

```bash
# 克隆仓库
git clone https://github.com/Qbuby/OmniAGP.git
cd OmniAGP

# 配置环境变量
cp .env.example .env
# 编辑 .env 填入你的 LLM API 配置

# 编译
cargo build

# 使用 Docker Compose 启动完整平台
cd docker
docker compose up -d

# 或者单独启动 API 网关（开发模式）
cargo run -p omni-api-gateway
```

访问 http://localhost:8080 打开 Web Dashboard。

## 管线流程

1. **游戏设计分析** — LLM 解析需求，输出结构化设计文档
2. **代码生成** — LLM 生成 GDScript 代码
3. **资产生成** — 调用 SDXL/MusicGen 等生成 2D/3D/音频资产
4. **场景组装** — 将代码和资产组装为完整 Godot 项目
5. **QA 测试** — 自动化测试验证生成结果
6. **自动修复** — 根据 QA 反馈自动修复问题

## 分布式架构

```
┌─────────────┐     ┌───────────┐     ┌──────────────┐
│ Web Dashboard│────▶│API Gateway│────▶│  NATS Queue  │
└─────────────┘     └───────────┘     └──────┬───────┘
                                              │
                    ┌─────────────┐           │
                    │  Scheduler  │◀──────────┘
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │ Worker 1 │ │ Worker 2 │ │ Worker N │
        │ (GPU)    │ │ (GPU)    │ │ (Cloud)  │
        └──────────┘ └──────────┘ └──────────┘
```

- **智能调度**: 根据任务类型 (2D/3D/Audio/LLM) 分配最优 GPU 节点
- **自动扩缩容**: 基于队列深度动态调整 Worker 数量
- **混合部署**: 本地 GPU + 云端 GPU 透明切换
- **计费系统**: 按 GPU 分钟计费

## 插件系统

```bash
# 安装示例插件
cargo run -p omnigp-cli -- plugin install examples/plugin-cyberpunk-style

# 使用模板创建游戏
cargo run -p omnigp-cli -- new --template platformer "MyGame"
```

## 多人协作

- 实时协作编辑游戏设计文档 (WebSocket CRDT)
- 版本对比与回滚
- 评审工作流: 设计评审 → 生成 → QA → 发布
- 团队权限管理 (Owner/Editor/Viewer)
- Webhook 集成 (Slack/Discord 通知)

## 开发

```bash
# 运行测试
cargo test

# 端到端冒烟测试
cargo run -p omni-smoke-test

# 检查代码
cargo clippy

# 格式化
cargo fmt

# 构建前端
cd web && npm install && npm run build
```

## License

MIT — see [LICENSE](LICENSE).
