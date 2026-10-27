#!/usr/bin/env python3
"""`just vet-refresh`: fetch the imported audit sets, prune what they now
cover, and keep the exemptions the fuzz workspaces still need (F71).

cargo vet sees one workspace at a time. `fuzz/` and `faraday/fuzz/` are
workspaces of their own, vetted against the same `supply-chain/` with
`--manifest-path` and `--store-path` (`just lint`). Run in the main
workspace, `cargo vet` and `cargo vet prune` drop every exemption the
main graph does not use, `arbitrary` and `libfuzzer-sys` among them,
and the fuzz workspaces then fail. cargo vet has no setting that keeps an
exemption it finds unused, and pruning from a fuzz workspace would drop
the main workspace's. So this:

1. records the exemptions before anything changes;
2. runs `cargo vet` (fetches the imports, writes `imports.lock`) and
   `cargo vet prune` in the main workspace;
3. puts back each exemption that went, by name and version, whose crate
   is in a fuzz workspace's Cargo.lock, and lets `cargo vet fmt` order
   the file;
4. checks the main workspace and each fuzz workspace offline, as
   `just lint` does, and fails if any fails.

An exemption put back may by then be covered by an import for the fuzz
graph too; it is kept rather than guessed at. Online. Run from the
repository root.
"""

import json
import subprocess
import sys
import tomllib

CONFIG = "supply-chain/config.toml"
FUZZ = ["fuzz/Cargo.toml", "faraday/fuzz/Cargo.toml"]


def vet(*args: str) -> None:
    cmd = ["cargo", "vet", *args]
    print("+", " ".join(cmd), flush=True)
    subprocess.run(cmd, check=True)


def exemptions() -> dict:
    with open(CONFIG, "rb") as f:
        return tomllib.load(f).get("exemptions", {})


def fuzz_crates() -> set:
    """(name, version) of every registry crate a fuzz workspace locks."""
    out = set()
    for manifest in FUZZ:
        lock = manifest.removesuffix("Cargo.toml") + "Cargo.lock"
        with open(lock, "rb") as f:
            for pkg in tomllib.load(f).get("package", []):
                if "source" in pkg:
                    out.add((pkg["name"], pkg["version"]))
    return out


def toml_value(v) -> str:
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, list):
        return "[" + ", ".join(toml_value(x) for x in v) + "]"
    # A JSON string is a valid TOML basic string.
    return json.dumps(v, ensure_ascii=False)


def main() -> int:
    before = exemptions()
    vet()
    vet("prune")
    after = exemptions()

    need = fuzz_crates()
    kept = {(n, e["version"]) for n, es in after.items() for e in es}
    restore = [
        (name, entry)
        for name, entries in before.items()
        for entry in entries
        if (name, entry["version"]) not in kept and (name, entry["version"]) in need
    ]
    if restore:
        with open(CONFIG, "a", encoding="utf-8") as f:
            for name, entry in restore:
                f.write(f"\n[[exemptions.{name}]]\n")
                for key, value in entry.items():
                    f.write(f"{key} = {toml_value(value)}\n")
                print(f"kept for the fuzz workspaces: {name} {entry['version']}")
        vet("fmt")

    vet("--locked")
    for manifest in FUZZ:
        vet("--locked", "--manifest-path", manifest, "--store-path", "supply-chain")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except subprocess.CalledProcessError as e:
        sys.exit(e.returncode)
