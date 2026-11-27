#!/usr/bin/env python3
"""clippy: the clippy.toml copies agree, and clippy runs with warnings as errors, failing too on a
warning about a clippy.toml (docs/spec/09-manifest.md section 1.4).

Clippy reads the nearest clippy.toml and nothing above it, so each directory's file holds every set
the directory needs. This checks, before it runs clippy, that:

  1. each set held by a directory has exactly the entries it has everywhere else (the entry's whole
     line, path and reason), and
  2. a directory holds no entry of a set that section 1.4 does not give it.

A set is known by its entries' paths, the SETS rules below (the script keeps the rule, the files
keep no marker); entries no rule names are the keys set. Then it runs `cargo clippy` with the
arguments given, prints what it prints, and fails when any message points into a clippy.toml: an
entry whose path resolves to nothing is only a warning to clippy, and `-D warnings` does not turn
it into an error. The `core/faraday-*` crate files, the per-crate copies of `main`, are outside the
sets (they hold `Compute::new` and their crate's own paths).

Run from the repository root:

    python3 -I -B tools/clippy.py --workspace --all-targets --locked -- -D warnings

Arguments are passed to `cargo clippy` as given. Read-only.
"""

import json
import re
import subprocess
import sys
from pathlib import Path

# A set's name and the rule that says which paths are its entries, tried in this order; the paths
# no rule names are the keys set.
SETS = [
    ("flow", re.compile(r"^faraday_templates::flow::FlowModel::")),
    ("mint", re.compile(r"::Compute::")),
    ("owner", re.compile(r"^faraday_kernel::Job::(heavy|seal)$")),
    ("runtime-owned", re.compile(r"^faraday_kernel::(\w+::into_(parts|list)|Memo::)")),
    ("app", re.compile(r"^(faraday_kernel::|faraday_runtime::Runtime::record_ink$)")),
    ("io-rt", re.compile(r"^std::(fs|net|process|env)::")),
    ("io-time", re.compile(r"^std::(thread::spawn$|time::(SystemTime|Instant)::now$)")),
]
KEYS = "keys"

# Section 1.4's second table: the sets each directory holds. io is io-rt and io-time; the runtime's
# app set leaves out the runtime-owned entries
# (the `into_parts` and `into_list` calls and the `Memo`), which the runtime itself calls. io-rt is
# compared by path: the runtime's reason is R3's, the kit's and the application's D1's.
HOLDS = {
    "core": ["mint", KEYS],
    "kit": ["io-rt", "io-time", "mint", KEYS, "flow"],
    "app": ["io-rt", "io-time", "mint", KEYS, "app", "runtime-owned", "owner", "flow"],
    "app/owners": ["io-rt", "io-time", "mint", KEYS, "app", "runtime-owned", "flow"],
    "runtime": ["io-rt", "mint", KEYS, "app"],
    "adapters": ["mint", KEYS, "app", "runtime-owned"],
    "roots": ["mint", KEYS, "app", "runtime-owned"],
    "roots/faraday-snapshot": ["mint", KEYS],
    "tools": ["mint", KEYS],
}

ENTRY = re.compile(r'^\s*\{\s*path\s*=\s*"([^"]+)"')


def set_of(path: str) -> str:
    for name, rule in SETS:
        if rule.search(path):
            return name
    return KEYS


def entries(file: Path) -> dict[str, dict[str, str]]:
    """The file's entries by set: path to the entry's line."""
    by_set: dict[str, dict[str, str]] = {}
    for line in file.read_text().splitlines():
        m = ENTRY.match(line)
        if m:
            by_set.setdefault(set_of(m.group(1)), {})[m.group(1)] = line.strip()
    return by_set


def copies_differ(root: Path) -> list[str]:
    problems = []
    held = {d: entries(root / d / "clippy.toml") for d in HOLDS}
    for d, sets in held.items():
        for s in sorted(set(sets) - set(HOLDS[d])):
            problems.append(f"{d}/clippy.toml holds the {s} set, which section 1.4 does not give it: {', '.join(sorted(sets[s]))}")
    reference: dict[str, tuple[str, dict[str, str]]] = {}
    for d, want in HOLDS.items():
        for s in want:
            have = held[d].get(s, {})
            if s not in reference:
                reference[s] = (d, have)
                if not have:
                    problems.append(f"{d}/clippy.toml holds no {s} entry")
                continue
            ref_dir, ref = reference[s]
            for p in sorted(set(ref) - set(have)):
                problems.append(f"{d}/clippy.toml lacks the {s} entry {p} of {ref_dir}/clippy.toml")
            for p in sorted(set(have) - set(ref)):
                problems.append(f"{d}/clippy.toml holds the {s} entry {p}, which {ref_dir}/clippy.toml lacks")
            for p in sorted(set(have) & set(ref)):
                if have[p] != ref[p] and s != "io-rt":
                    problems.append(f"{d}/clippy.toml's {s} entry {p} differs from {ref_dir}/clippy.toml's")
    return problems


def main() -> int:
    problems = copies_differ(Path(__file__).resolve().parent.parent)
    if problems:
        print("clippy: the clippy.toml copies drifted:", file=sys.stderr)
        for p in problems:
            print(f"  {p}", file=sys.stderr)
        return 1
    cmd = ["cargo", "clippy", *_with_json(sys.argv[1:])]
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, text=True)
    config: list[str] = []
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
        if any(s.get("file_name", "").endswith("clippy.toml") for s in diag.get("spans", [])):
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


def _with_json(args: list[str]) -> list[str]:
    """The arguments with the message format before any `--`."""
    fmt = "--message-format=json-diagnostic-rendered-ansi"
    if "--" in args:
        i = args.index("--")
        return [*args[:i], fmt, *args[i:]]
    return [*args, fmt]


if __name__ == "__main__":
    sys.exit(main())
