"""
Plugin sandbox runner — executes Python plugins in an isolated subprocess
with restricted filesystem and network access.
"""

import json
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass
class SandboxPolicy:
    read_paths: list[str]
    write_paths: list[str]
    allowed_hosts: list[str]
    allow_outbound: bool = False
    max_memory_mb: int = 512
    max_cpu_seconds: int = 60


class SandboxViolation(Exception):
    pass


class SandboxedRunner:
    """Runs a plugin in an isolated subprocess with resource limits."""

    def __init__(self, policy: SandboxPolicy):
        self.policy = policy

    def validate_path_read(self, path: Path) -> None:
        resolved = path.resolve()
        for allowed in self.policy.read_paths:
            if str(resolved).startswith(str(Path(allowed).resolve())):
                return
        raise SandboxViolation(f"read access denied: {path}")

    def validate_path_write(self, path: Path) -> None:
        resolved = path.resolve()
        for allowed in self.policy.write_paths:
            if str(resolved).startswith(str(Path(allowed).resolve())):
                return
        raise SandboxViolation(f"write access denied: {path}")

    def validate_network(self, host: str) -> None:
        if not self.policy.allow_outbound:
            raise SandboxViolation("network access disabled")
        if self.policy.allowed_hosts and host not in self.policy.allowed_hosts:
            raise SandboxViolation(f"network access denied for host: {host}")

    def run_plugin(
        self,
        plugin_module: str,
        plugin_class: str,
        method: str,
        args: dict[str, Any],
        context: dict[str, Any],
    ) -> dict[str, Any]:
        """Execute a plugin method in an isolated subprocess."""
        invocation = {
            "module": plugin_module,
            "class": plugin_class,
            "method": method,
            "args": args,
            "context": context,
            "policy": {
                "read_paths": self.policy.read_paths,
                "write_paths": self.policy.write_paths,
                "allowed_hosts": self.policy.allowed_hosts,
                "allow_outbound": self.policy.allow_outbound,
            },
        }

        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".json", delete=False
        ) as f:
            json.dump(invocation, f)
            invocation_file = f.name

        runner_script = Path(__file__).parent / "_runner.py"

        try:
            result = subprocess.run(
                [sys.executable, str(runner_script), invocation_file],
                capture_output=True,
                text=True,
                timeout=self.policy.max_cpu_seconds,
            )
        except subprocess.TimeoutExpired:
            raise SandboxViolation(
                f"plugin exceeded CPU time limit ({self.policy.max_cpu_seconds}s)"
            )

        if result.returncode != 0:
            raise RuntimeError(f"plugin execution failed: {result.stderr}")

        return json.loads(result.stdout)
