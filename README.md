# OmniAGP

AI-driven game production pipeline — Rust orchestration layer + GDScript code generation.

## Structure

```
OmniAGP/
├── orchestrator/      # Rust orchestration layer (binary crate)
├── runtime/           # Godot headless integration (library crate)
├── agents/            # Agent role definitions and prompts
├── pipelines/         # Asset generation pipelines (Python sidecar)
├── templates/         # Parameterized GDScript template library
├── docs/              # Technical documentation
└── .github/workflows/ # CI configuration
```

## Prerequisites

- Rust 1.75+ (edition 2021)
- Python 3.10+ (for pipelines)
- Godot 4.x (for runtime integration)

## Build

```bash
# Build all workspace crates
cargo build --workspace

# Run lints
cargo clippy --workspace -- -D warnings

# Run tests
cargo test --workspace
```

Or use the Makefile:

```bash
make build
make lint
make test
make all   # build + lint + test
```

## License

MIT
