#!/usr/bin/env python3
"""Every reason a secret leaves the store has a leak flow (docs/SECRET-LEAKS.md §4).

`faraday_keys::Reveal` (and the `Part` it carries) is the closed list of
reveals. `faraday/faraday-core/tests/leaks/reveals.rs` matches on it with no
wildcard arm (the compiler holds that every variant has an arm) and names
the flows of each arm. This reads the enums and the arms and fails on:

- a variant of `Reveal` or `Part` with no arm in the file;
- an arm with no flow, unless it is in a part function (`vault_entry`,
  `outbox_file`, `backup_stick`: a part the app never asks for there);
- a flow name that is no `#[test]` function of tests/leaks/ and no flow
  label of areas.rs.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
KEYS = ROOT / "faraday/faraday-keys/src/session.rs"
LEAKS = ROOT / "faraday/faraday-core/tests/leaks"


def variants(src: str, name: str) -> list[str]:
    m = re.search(r"pub enum %s \{(.*?)\n\}" % name, src, re.S)
    if not m:
        sys.exit(f"lint-leak-reveals: no enum {name} in {KEYS}")
    body = re.sub(r"///.*", "", m.group(1))
    return re.findall(r"^\s{4}([A-Z]\w*)", body, re.M)


def arms(src: str, fn: str) -> dict[str, list[str]]:
    """The arms of fn: the names of the variants matched, and the strings."""
    m = re.search(r"\nfn %s\(.*?\n\}\n" % fn, src, re.S)
    if not m:
        sys.exit(f"lint-leak-reveals: no fn {fn} in reveals.rs")
    out: dict[str, list[str]] = {}
    pending: list[str] = []
    for arm in re.split(r"(?<=[,}])\n(?=\s{8}\S)", m.group(0)):
        names = re.findall(r"(?:Reveal|Part)::(\w+)", arm.split("=>")[0])
        if "=>" not in arm:
            continue
        flows = re.findall(r'"([^"]+)"', arm.split("=>", 1)[1])
        for n in names:
            out[n] = flows
    return out


def main() -> int:
    keys = KEYS.read_text()
    reveals = (LEAKS / "reveals.rs").read_text()
    tests = set()
    labels = set()
    for f in LEAKS.glob("*.rs"):
        t = f.read_text()
        tests |= set(re.findall(r"#\[test\]\s*\n(?:#\[[^\n]*\]\s*\n)*(?:pub )?fn (\w+)", t))
    labels |= set(re.findall(r'(?:typed|typed_in_make|kept)\(\s*"([^"]+)"', (LEAKS / "areas.rs").read_text()))
    bad = []
    checks = [
        ("flows", variants(keys, "Reveal"), True),
        ("vault_entry", variants(keys, "Part"), False),
        ("outbox_file", variants(keys, "Part"), False),
        ("backup_stick", variants(keys, "Part"), False),
    ]
    for fn, want, needs_flow in checks:
        got = arms(reveals, fn)
        for v in want:
            if v not in got:
                bad.append(f"{fn}: no arm for {v}")
            elif needs_flow and not got[v] and v not in ("VaultEntry", "OutboxFile", "BackupStick"):
                bad.append(f"{fn}: {v} names no flow")
        for v, names in got.items():
            for n in names:
                if n not in tests and n not in labels:
                    bad.append(f"{fn}: {v} names {n!r}, no leak test or flow label")
    if bad:
        print("lint-leak-reveals: a reveal with no leak flow (docs/SECRET-LEAKS.md §4):")
        for b in bad:
            print("  " + b)
        return 1
    print("lint-leak-reveals: every reveal reaches a leak flow")
    return 0


if __name__ == "__main__":
    sys.exit(main())
