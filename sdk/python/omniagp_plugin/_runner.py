"""
Subprocess runner for sandboxed plugin execution.
This script is invoked by SandboxedRunner in an isolated process.
"""

import importlib
import json
import sys
from pathlib import Path


def main():
    if len(sys.argv) != 2:
        print("usage: _runner.py <invocation.json>", file=sys.stderr)
        sys.exit(1)

    with open(sys.argv[1]) as f:
        invocation = json.load(f)

    module_name = invocation["module"]
    class_name = invocation["class"]
    method_name = invocation["method"]
    args = invocation["args"]
    context = invocation["context"]
    policy = invocation["policy"]

    _apply_sandbox(policy)

    module = importlib.import_module(module_name)
    plugin_class = getattr(module, class_name)
    plugin = plugin_class()

    from omniagp_plugin.protocol import PluginContext

    ctx = PluginContext(
        project_dir=Path(context.get("project_dir", ".")),
        plugin_data_dir=Path(context.get("plugin_data_dir", ".")),
        config=context.get("config", {}),
    )
    plugin.activate(ctx)

    method = getattr(plugin, method_name)
    result = method(**_convert_path_args(args))

    plugin.deactivate()

    if isinstance(result, list):
        output = {"result": [str(p) for p in result]}
    elif isinstance(result, bytes):
        import base64
        output = {"result": base64.b64encode(result).decode()}
    elif result is None:
        output = {"result": "ok"}
    else:
        output = {"result": result}

    print(json.dumps(output))


def _apply_sandbox(policy: dict):
    """Apply filesystem restrictions by monkey-patching builtins."""
    import builtins

    original_open = builtins.open
    read_paths = [str(Path(p).resolve()) for p in policy.get("read_paths", [])]
    write_paths = [str(Path(p).resolve()) for p in policy.get("write_paths", [])]

    def sandboxed_open(file, mode="r", *a, **kw):
        path = str(Path(file).resolve())
        is_write = any(c in mode for c in "wxa+")

        if is_write:
            if not any(path.startswith(wp) for wp in write_paths):
                raise PermissionError(f"sandbox: write denied for {file}")
        else:
            if not any(path.startswith(rp) for rp in read_paths + write_paths):
                raise PermissionError(f"sandbox: read denied for {file}")

        return original_open(file, mode, *a, **kw)

    builtins.open = sandboxed_open

    if not policy.get("allow_outbound", False):
        import socket
        _orig_connect = socket.socket.connect

        def blocked_connect(self, address):
            raise PermissionError("sandbox: network access denied")

        socket.socket.connect = blocked_connect


def _convert_path_args(args: dict) -> dict:
    """Convert string path arguments to Path objects."""
    converted = {}
    for k, v in args.items():
        if k.endswith("_path") or k.endswith("_dir"):
            converted[k] = Path(v)
        else:
            converted[k] = v
    return converted


if __name__ == "__main__":
    main()
