# OmniAGP

LLM 驱动的 Agentic 游戏全流程开发引擎 — 从需求描述到完整可游玩游戏。

## 架构

```
OmniAGP/
├── crates/
│   ├── omni-core/          # 核心类型、trait、共享工具
│   ├── omni-llm/           # LLM 提供者抽象层 (OpenAI-compatible API)
│   ├── omni-orchestrator/  # Agent 编排引擎 — 管理多步骤游戏生成管线
│   ├── omni-assets/        # 资产生成管线 (2D/3D/音频)
│   └── omni-godot/         # Godot 项目生成 — GDScript 代码生成 + 场景组装
├── services/
│   ├── api-gateway/        # HTTP API 网关
│   └── worker/             # 后台 Worker — 异步执行管线步骤
├── configs/                # 配置文件
├── scripts/                # 工具脚本
└── godot/templates/        # Godot 项目模板
```

## 技术栈

- **语言**: Rust (底层引擎) + GDScript (生成的游戏代码)
- **游戏引擎**: Godot 4.x
- **LLM**: 兼容 OpenAI API 的任意提供者
- **资产生成**: SDXL (2D), Hunyuan3D-2/TripoSR (3D), MusicGen (音频)

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

# 启动 API 网关
cargo run -p omni-api-gateway

# 创建游戏项目
curl -X POST http://localhost:8080/api/v1/games \
  -H "Content-Type: application/json" \
  -d '{"name": "MyGame", "description": "一个 2D 平台跳跃游戏"}'
```

## 管线流程

1. **游戏设计分析** — LLM 解析需求，输出结构化设计文档
2. **代码生成** — LLM 生成 GDScript 代码
3. **资产生成** — 调用 SDXL/MusicGen 等生成 2D/3D/音频资产
4. **场景组装** — 将代码和资产组装为完整 Godot 项目

## 开发

```bash
# 运行测试
cargo test

# 检查代码
cargo clippy

# 格式化
cargo fmt
```

## License

MIT — see [LICENSE](LICENSE).
