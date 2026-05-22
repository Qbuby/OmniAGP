# OmniAGP Plugin SDK

## Overview

The OmniAGP Plugin SDK allows developers to extend the platform with custom functionality. Plugins run in a sandboxed environment with controlled access to the filesystem and network.

## Plugin Types

| Type | Trait | Purpose |
|------|-------|---------|
| Style | `StylePlugin` | Apply custom art styles to generated assets |
| PostProcess | `PostProcessPlugin` | Post-processing effects on generated content |
| Generator | `GeneratorPlugin` | Custom content generators (terrain, levels, etc.) |
| Exporter | `ExporterPlugin` | Export projects to custom formats |

## Quick Start

### 1. Create a new plugin crate

```toml
# Cargo.toml
[package]
name = "my-awesome-plugin"
version = "0.1.0"
edition = "2021"

[dependencies]
omni-plugin = { path = "../../crates/omni-plugin" }
anyhow = "1"
serde_json = "1"
```

### 2. Implement the Plugin trait

```rust
use anyhow::Result;
use omni_plugin::{Plugin, PluginContext, PluginType, StylePlugin};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct MyStylePlugin {
    name: String,
    version: String,
}

impl Plugin for MyStylePlugin {
    fn name(&self) -> &str { &self.name }
    fn version(&self) -> &str { &self.version }
    fn plugin_type(&self) -> PluginType { PluginType::Style }

    fn activate(&mut self, ctx: &PluginContext) -> Result<()> {
        // Read configuration, initialize resources
        Ok(())
    }

    fn deactivate(&mut self) -> Result<()> {
        // Cleanup resources
        Ok(())
    }
}

impl StylePlugin for MyStylePlugin {
    fn apply_style(
        &self,
        input_path: &PathBuf,
        output_path: &PathBuf,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        // Transform the input asset with your custom style
        Ok(())
    }

    fn supported_formats(&self) -> Vec<String> {
        vec!["png".into(), "bmp".into()]
    }

    fn preview(
        &self,
        input_path: &PathBuf,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<u8>> {
        // Generate a preview thumbnail
        Ok(vec![])
    }
}
```

### 3. Create a plugin manifest

```toml
# plugin.toml
name = "my-awesome-plugin"
version = "0.1.0"
description = "My custom style plugin"
author = "Your Name"
license = "MIT"
plugin_type = "Style"
entry_point = "libmy_awesome_plugin"
min_omniagp_version = "0.1.0"

[permissions.filesystem]
read_paths = ["$PROJECT_DIR"]
write_paths = ["$PLUGIN_DATA_DIR"]

[permissions.network]
allowed_hosts = []
allow_outbound = false

[permissions]
max_memory_mb = 512
max_cpu_seconds = 60

[[config_schema]]
key = "intensity"
label = "Effect Intensity"
description = "How strong the style effect should be"
required = false
[config_schema.field_type]
type = "number"
min = 0.0
max = 1.0
```

## Plugin Lifecycle

```
install → activate → execute → deactivate → uninstall
```

1. **Install**: Plugin files are copied to the plugins directory
2. **Activate**: Plugin is initialized with project context and configuration
3. **Execute**: Plugin performs its work (style application, generation, export, etc.)
4. **Deactivate**: Plugin releases resources
5. **Uninstall**: Plugin files are removed

## Sandbox Security

Plugins run with restricted permissions:

- **Filesystem**: Only paths declared in `permissions.filesystem` are accessible
- **Network**: Outbound connections are blocked unless explicitly allowed
- **Memory**: Hard limit enforced (default 512MB)
- **CPU**: Execution time limit (default 60 seconds)

Attempting to access resources outside the sandbox will result in a `sandbox violation` error.

## Configuration Schema

Plugins declare their configuration using a typed schema. The Web UI automatically renders configuration forms based on this schema.

Supported field types:
- `string` — text input with optional min/max length
- `number` — numeric input with optional min/max
- `integer` — integer input with optional min/max
- `boolean` — toggle switch
- `select` — dropdown with predefined options
- `color` — color picker
- `file_path` — file selector with extension filter

## Version Compatibility

Plugins declare `min_omniagp_version` and optionally `max_omniagp_version`. The registry validates compatibility before installation and automatically disables incompatible plugins after core upgrades.

## Examples

See the `examples/` directory for complete working plugins:

- `plugin-cyberpunk-style` — StylePlugin that applies cyberpunk pixel art aesthetics
- `plugin-retro-exporter` — ExporterPlugin that exports to retro ROM format
- `plugin-terrain-generator` — GeneratorPlugin for procedural terrain

## Publishing

1. Package your plugin: ensure `plugin.toml` is at the crate root
2. Submit via the marketplace API: `POST /api/v1/contributions`
3. Automated pipeline: security scan → compatibility test → review
4. Once approved, your plugin appears in the marketplace
