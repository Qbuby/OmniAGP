"""
OmniAGP Plugin Protocol — Python SDK

Defines the plugin interface protocol that Python plugins must implement.
Mirrors the Rust trait definitions in omni-plugin crate.
"""

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any


class PluginType(Enum):
    STYLE = "Style"
    POST_PROCESS = "PostProcess"
    GENERATOR = "Generator"
    EXPORTER = "Exporter"


@dataclass
class PluginContext:
    project_dir: Path
    plugin_data_dir: Path
    config: dict[str, Any] = field(default_factory=dict)


@dataclass
class ConfigField:
    key: str
    label: str
    description: str
    field_type: str
    required: bool = False
    default_value: Any = None
    constraints: dict[str, Any] = field(default_factory=dict)


@dataclass
class PluginManifest:
    name: str
    version: str
    description: str
    author: str
    license: str
    plugin_type: PluginType
    entry_point: str
    min_omniagp_version: str
    max_omniagp_version: str | None = None
    dependencies: dict[str, str] = field(default_factory=dict)
    config_schema: list[ConfigField] = field(default_factory=list)
    permissions: dict[str, Any] = field(default_factory=lambda: {
        "filesystem": {"read_paths": ["$PROJECT_DIR"], "write_paths": ["$PLUGIN_DATA_DIR"]},
        "network": {"allowed_hosts": [], "allow_outbound": False},
        "max_memory_mb": 512,
        "max_cpu_seconds": 60,
    })


class Plugin(ABC):
    """Base plugin protocol. All plugins must implement this."""

    @abstractmethod
    def name(self) -> str: ...

    @abstractmethod
    def version(self) -> str: ...

    @abstractmethod
    def plugin_type(self) -> PluginType: ...

    @abstractmethod
    def activate(self, ctx: PluginContext) -> None: ...

    @abstractmethod
    def deactivate(self) -> None: ...

    def manifest(self) -> PluginManifest:
        return PluginManifest(
            name=self.name(),
            version=self.version(),
            description="",
            author="",
            license="MIT",
            plugin_type=self.plugin_type(),
            entry_point=self.__class__.__module__,
            min_omniagp_version="0.1.0",
        )


class StylePlugin(Plugin):
    """Plugin that applies custom art styles to generated assets."""

    def plugin_type(self) -> PluginType:
        return PluginType.STYLE

    @abstractmethod
    def apply_style(
        self, input_path: Path, output_path: Path, params: dict[str, Any]
    ) -> None: ...

    @abstractmethod
    def supported_formats(self) -> list[str]: ...

    @abstractmethod
    def preview(self, input_path: Path, params: dict[str, Any]) -> bytes: ...


class PostProcessPlugin(Plugin):
    """Plugin that applies post-processing effects on generated content."""

    def plugin_type(self) -> PluginType:
        return PluginType.POST_PROCESS

    @abstractmethod
    def process(
        self, input_path: Path, output_path: Path, params: dict[str, Any]
    ) -> None: ...

    @abstractmethod
    def supported_input_types(self) -> list[str]: ...


class GeneratorPlugin(Plugin):
    """Plugin that generates custom content (terrain, levels, etc.)."""

    def plugin_type(self) -> PluginType:
        return PluginType.GENERATOR

    @abstractmethod
    def generate(
        self, output_dir: Path, params: dict[str, Any]
    ) -> list[Path]: ...

    @abstractmethod
    def generator_type(self) -> str: ...


class ExporterPlugin(Plugin):
    """Plugin that exports projects to custom formats."""

    def plugin_type(self) -> PluginType:
        return PluginType.EXPORTER

    @abstractmethod
    def export(
        self, project_dir: Path, output_path: Path, params: dict[str, Any]
    ) -> None: ...

    @abstractmethod
    def export_format(self) -> str: ...
