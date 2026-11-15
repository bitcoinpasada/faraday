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

Run from the repository root; `just lint` does:

    python3 tools/clippy.py --workspace --all-targets
"""

import json
import re
import subprocess
import sys
from pathlib import Path

# Clippy reads the nearest clippy.toml, and `disallowed_methods` is on or off
# per source file, so the crates that hold secrets each carry a copy of one
# file (docs/COMPUTE.md section 7). The copies of a group must be identical,
# and under faraday/ each must hold every path of faraday/clippy.toml that is
# not the key boundary's.
COPIES = [
    ["core/faraday-bip", "core/faraday-crypto", "core/faraday-entropy"],
    ["faraday/faraday-keys", "faraday/faraday-vault", "faraday/faraday-pgp"],
]

# The key boundary (docs/KEYS.md): faraday/clippy.toml's entries whose reason
# starts "key boundary" refuse key material outside faraday-keys. The crates
# that make it (faraday-keys, -vault, -pgp, -sb) take none of them; faraday-kef
# takes every one but its own (`faraday_kef::`), which it calls itself.
BOUNDARY = "key boundary"
INSIDE = ["faraday/faraday-keys", "faraday/faraday-vault", "faraday/faraday-pgp", "faraday/faraday-sb"]


def copies_differ() -> list[str]:
    root = Path(__file__).resolve().parent.parent
    problems = []
    for group in COPIES:
        texts = {c: (root / c / "clippy.toml").read_text() for c in group}
        first = texts[group[0]]
        for c, t in texts.items():
            if t != first:
                problems.append(f"{c}/clippy.toml differs from {group[0]}/clippy.toml")
    path = re.compile(r'path = "([^"]+)"')

    def paths(file: str) -> tuple[set[str], set[str]]:
        plain, boundary = set(), set()
        for line in (root / file / "clippy.toml").read_text().splitlines():
            m = path.search(line)
            if m:
                (boundary if BOUNDARY in line else plain).add(m.group(1))
        return plain, boundary

    plain, boundary = paths("faraday")
    for c in INSIDE + ["faraday/faraday-kef"]:
        have, have_boundary = paths(c)
        for m in sorted(plain - have):
            problems.append(f"{c}/clippy.toml lacks faraday/clippy.toml's {m}")
        if c in INSIDE and have_boundary:
            problems.append(f"{c}/clippy.toml holds key boundary entries, which this crate makes")
    sb, _ = paths("faraday/faraday-sb")
    if sb != plain:
        problems.append("faraday/faraday-sb/clippy.toml differs from faraday/clippy.toml's entries outside the key boundary")
    _, kef = paths("faraday/faraday-kef")
    want = {p for p in boundary if not p.startswith("faraday_kef::")}
    for m in sorted(want - kef):
        problems.append(f"faraday/faraday-kef/clippy.toml lacks faraday/clippy.toml's {m}")
    for m in sorted(kef - want):
        problems.append(f"faraday/faraday-kef/clippy.toml holds {m}, which is not a key boundary entry of faraday/clippy.toml, or is its own")
    return problems


def main() -> int:
    problems = copies_differ()
    if problems:
        print("clippy: the per-crate clippy.toml copies drifted:", file=sys.stderr)
        for p in problems:
            print(f"  {p}", file=sys.stderr)
        return 1
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
