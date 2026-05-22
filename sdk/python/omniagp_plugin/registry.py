"""
Plugin registry — discovers, loads, and manages Python plugins.
"""

import importlib
import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from omniagp_plugin.protocol import Plugin, PluginContext, PluginManifest, PluginType


@dataclass
class InstalledPlugin:
    manifest: PluginManifest
    install_path: Path
    active: bool = False
    instance: Plugin | None = None


class PluginRegistry:
    """Manages installed Python plugins."""

    def __init__(self, plugins_dir: Path, data_dir: Path):
        self.plugins_dir = plugins_dir
        self.data_dir = data_dir
        self.plugins: dict[str, InstalledPlugin] = {}

    def discover(self) -> list[PluginManifest]:
        """Scan plugins directory for installed plugins."""
        found = []
        if not self.plugins_dir.exists():
            return found

        for plugin_dir in self.plugins_dir.iterdir():
            if not plugin_dir.is_dir():
                continue
            manifest_path = plugin_dir / "plugin.json"
            if not manifest_path.exists():
                continue
            manifest = self._load_manifest(manifest_path)
            if manifest:
                found.append(manifest)
        return found

    def install(self, manifest: PluginManifest, source_path: Path) -> None:
        """Install a plugin from source."""
        plugin_id = f"{manifest.name}@{manifest.version}"
        if plugin_id in self.plugins:
            raise ValueError(f"plugin {plugin_id} already installed")

        install_path = self.plugins_dir / manifest.name / manifest.version
        install_path.mkdir(parents=True, exist_ok=True)

        import shutil
        if source_path.is_dir():
            shutil.copytree(source_path, install_path, dirs_exist_ok=True)
        else:
            shutil.copy2(source_path, install_path)

        data_path = self.data_dir / manifest.name
        data_path.mkdir(parents=True, exist_ok=True)

        self.plugins[plugin_id] = InstalledPlugin(
            manifest=manifest, install_path=install_path
        )

    def uninstall(self, name: str, version: str) -> None:
        """Remove an installed plugin."""
        plugin_id = f"{name}@{version}"
        if plugin_id not in self.plugins:
            raise ValueError(f"plugin {plugin_id} not found")

        entry = self.plugins.pop(plugin_id)
        if entry.active and entry.instance:
            entry.instance.deactivate()

        import shutil
        if entry.install_path.exists():
            shutil.rmtree(entry.install_path)

    def activate(self, name: str, version: str, ctx: PluginContext) -> Plugin:
        """Activate a plugin and return its instance."""
        plugin_id = f"{name}@{version}"
        entry = self.plugins.get(plugin_id)
        if not entry:
            raise ValueError(f"plugin {plugin_id} not found")

        import sys
        sys.path.insert(0, str(entry.install_path))

        module = importlib.import_module(entry.manifest.entry_point)
        plugin_class = _find_plugin_class(module)
        instance = plugin_class()
        instance.activate(ctx)

        entry.instance = instance
        entry.active = True
        return instance

    def deactivate(self, name: str, version: str) -> None:
        """Deactivate a running plugin."""
        plugin_id = f"{name}@{version}"
        entry = self.plugins.get(plugin_id)
        if not entry or not entry.active:
            return
        if entry.instance:
            entry.instance.deactivate()
        entry.active = False
        entry.instance = None

    def list_by_type(self, plugin_type: PluginType) -> list[InstalledPlugin]:
        return [p for p in self.plugins.values() if p.manifest.plugin_type == plugin_type]

    def list_all(self) -> list[InstalledPlugin]:
        return list(self.plugins.values())

    def check_compatibility(self, manifest: PluginManifest, core_version: str) -> bool:
        """Check if a plugin is compatible with the current core version."""
        from packaging.version import Version
        core = Version(core_version)
        min_ver = Version(manifest.min_omniagp_version)
        if core < min_ver:
            return False
        if manifest.max_omniagp_version:
            max_ver = Version(manifest.max_omniagp_version)
            if core > max_ver:
                return False
        return True

    def _load_manifest(self, path: Path) -> PluginManifest | None:
        try:
            data = json.loads(path.read_text())
            return PluginManifest(
                name=data["name"],
                version=data["version"],
                description=data.get("description", ""),
                author=data.get("author", ""),
                license=data.get("license", "MIT"),
                plugin_type=PluginType(data["plugin_type"]),
                entry_point=data["entry_point"],
                min_omniagp_version=data.get("min_omniagp_version", "0.1.0"),
                max_omniagp_version=data.get("max_omniagp_version"),
                dependencies=data.get("dependencies", {}),
            )
        except (json.JSONDecodeError, KeyError):
            return None


def _find_plugin_class(module) -> type:
    """Find the Plugin subclass in a module."""
    for attr_name in dir(module):
        attr = getattr(module, attr_name)
        if (
            isinstance(attr, type)
            and issubclass(attr, Plugin)
            and attr is not Plugin
            and not getattr(attr, "__abstractmethods__", None)
        ):
            return attr
    raise ValueError(f"no concrete Plugin class found in {module.__name__}")
