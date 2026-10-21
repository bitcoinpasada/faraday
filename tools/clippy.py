#!/usr/bin/env python3
"""Clippy with warnings as errors, failing too on a warning about a
clippy.toml.

`cargo clippy -- -D warnings` turns the lints' warnings into errors, but
not clippy's warnings about its own configuration: a path in
`faraday/clippy.toml` that names nothing (mistyped, or moved in a
dependency) is printed as a warning and clippy exits 0, so the entry
silently guards nothing (docs/COMPUTE.md §7). This runs clippy with the
arguments given, prints what it prints, and fails when any message
points into a clippy.toml.

Run from the repository root; `just lint` and tools/check-macos.sh do:

    python3 tools/clippy.py --workspace --all-targets
"""

import json
import subprocess
import sys


def main() -> int:
    cmd = [
        "cargo",
        "clippy",
        *sys.argv[1:],
        "--message-format=json-diagnostic-rendered-ansi",
        "--",
        "-D",
        "warnings",
    ]
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, text=True)
    config = []
    assert proc.stdout is not None
    for line in proc.stdout:
        try:
            msg = json.loads(line)
        except ValueError:
            sys.stdout.write(line)
            continue
        if msg.get("reason") != "compiler-message":
            continue
        diag = msg.get("message", {})
        rendered = diag.get("rendered")
        if rendered:
            sys.stderr.write(rendered)
        spans = diag.get("spans", [])
        if any(s.get("file_name", "").endswith("clippy.toml") for s in spans):
            first = diag.get("message", "")
            if first not in config:
                config.append(first)
    status = proc.wait()
    if config:
        print("clippy: its configuration has warnings; each entry must name something:", file=sys.stderr)
        for c in config:
            print(f"  {c}", file=sys.stderr)
        return status or 1
    return status


if __name__ == "__main__":
    sys.exit(main())
