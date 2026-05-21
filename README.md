# OmniAGP

AI-driven game production pipeline — from concept to playable build.

## Structure

```
orchestrator/   — Rust orchestration layer
agents/         — Agent definitions
pipelines/      — Asset generation pipelines
runtime/        — Godot integration
templates/      — GDScript template library
docs/           — Documentation
```

## Getting Started

### Prerequisites

- Rust 1.75+ (with cargo)
- Godot 4.x (for runtime integration)

### Build

```bash
cargo build
```

Or use the justfile:

```bash
just build
```

### Lint

```bash
just lint
```

## License

MIT — see [LICENSE](LICENSE).
