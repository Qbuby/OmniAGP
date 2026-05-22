# OmniAGP Plugin SDK (Python)

Build custom plugins for the OmniAGP game generation platform using Python.

## Plugin Types

| Type | Base Class | Purpose |
|------|-----------|---------|
| Style | `StylePlugin` | Apply custom art styles to generated assets |
| PostProcess | `PostProcessPlugin` | Post-processing effects on generated content |
| Generator | `GeneratorPlugin` | Custom content generators (terrain, levels, etc.) |
| Exporter | `ExporterPlugin` | Export projects to custom formats |

## Quick Start

```python
from pathlib import Path
from typing import Any
from omniagp_plugin import StylePlugin, PluginContext

class NeonGlowStyle(StylePlugin):
    def name(self) -> str:
        return "neon-glow-style"

    def version(self) -> str:
        return "1.0.0"

    def activate(self, ctx: PluginContext) -> None:
        self.intensity = ctx.config.get("intensity", 0.8)

    def deactivate(self) -> None:
        pass

    def apply_style(self, input_path: Path, output_path: Path, params: dict[str, Any]) -> None:
        # Transform the input asset with neon glow effect
        ...

    def supported_formats(self) -> list[str]:
        return ["png", "bmp"]

    def preview(self, input_path: Path, params: dict[str, Any]) -> bytes:
        return b""
```

## Plugin Manifest

Create a `plugin.json` at your plugin root:

```json
{
  "name": "neon-glow-style",
  "version": "1.0.0",
  "description": "Applies neon glow effects to game assets",
  "author": "Your Name",
  "license": "MIT",
  "plugin_type": "Style",
  "entry_point": "neon_glow",
  "min_omniagp_version": "0.1.0"
}
```

## Sandbox

Plugins run in an isolated subprocess with restricted access:
- Filesystem: only declared paths are accessible
- Network: blocked unless explicitly allowed
- CPU: time-limited execution
- Memory: capped at declared limit

## Install

```bash
pip install omniagp-plugin
```
