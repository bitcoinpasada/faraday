#!/usr/bin/env python3
"""Each fuzz crate's README table against its Cargo.toml (tools/README.md).

Fails when the "The targets" table of fuzz/README.md or faraday/fuzz/README.md
lacks a `[[bin]]` target of its Cargo.toml, or lists one that is not there.
Run from anywhere; `just lint` does. The standard library only.
"""
import pathlib
import re
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parent.parent


def readme_targets(directory):
    """The failures of one fuzz README's target table against its Cargo.toml."""
    bins = {b["name"] for b in tomllib.loads(
        (ROOT / directory / "Cargo.toml").read_text(encoding="utf-8")).get("bin", [])}
    text = (ROOT / directory / "README.md").read_text(encoding="utf-8")
    table = text.split("## The targets", 1)[-1].split("\n## ", 1)[0]
    rows = set(re.findall(r"^\| `(\w+)` \|", table, re.M))
    return ([f"{directory}/README.md: target `{n}` is not in the table" for n in sorted(bins - rows)]
            + [f"{directory}/README.md: `{n}` is in the table but not a target of Cargo.toml"
               for n in sorted(rows - bins)])


def main():
    failures = []
    for d in ("fuzz", "faraday/fuzz"):
        failures += readme_targets(d)
    for m in failures:
        print(m)
    if failures:
        print(f"lint-fuzz-parsers: {len(failures)} failure(s)", file=sys.stderr)
        return 1
    print("lint-fuzz-parsers: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
