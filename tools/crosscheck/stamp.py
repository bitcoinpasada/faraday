#!/usr/bin/env python3
"""The cross-check's stamp (F79, docs/MULTIVENDOR.md §10.2).

A passing `just crosscheck` writes `tools/crosscheck/stamp`: one hash
over the crypto sources, the cross-check itself and its references, and
the `Cargo.lock` entries of the crypto crates, with the seed, the
Python it ran under and how long it took. `just lint` runs
`stamp.py check`, which hashes the same inputs and fails when they no
longer match: a change to the crypto, or a version bump of a crypto
crate, is not committed without a passing cross-check. It is a record,
not a security control. Standard library only; about a tenth of a
second.

    stamp.py check     exit 1 when the inputs changed since the stamp
    stamp.py show      print what is hashed
"""

import hashlib
import pathlib
import platform
import sys
import time
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[2]
STAMP = ROOT / "tools" / "crosscheck" / "stamp"

# The code whose results the cross-check checks, and the cross-check:
# each crate's sources and manifest.
CRATES = [
    "core/faraday-crypto",
    "core/faraday-bip",
    "core/faraday-psbt",
    "core/faraday-entropy",
    "core/faraday-backup",
    "core/faraday-codec",
    "faraday/faraday-keys",
    "faraday/faraday-qr",
    "faraday/faraday-vault",
    "tools/crosscheck",
]
# Single files of crates too large to hash whole, whose results an area
# checks: the session's fresh randomness and what a person adds
# (`own-randomness`), Faraday Desktop's dice rollers (`roller`), and the
# picture a Codex32QR is written as (`seedqr`).
FILES = ["faraday/faraday-core/src/fresh.rs", "faraday/faraday-core/src/fresh/roller/mod.rs",
         "faraday/faraday-core/src/fresh/roller/five.rs", "faraday/faraday-core/src/picture.rs"]
# The published vectors the cross-check reads (tools/vectors/<name>/).
VECTORS = ["addresses", "aezeed", "bip32", "bip327", "bip352", "bip39", "bip445", "bip85", "bip93",
           "codex32-booklet", "codex32-correction", "codex32-volvelle", "codex32qr", "entropy",
           "kdf", "message", "psbt-bips", "seedqr", "sighash", "slip39", "ur", "vault",
           "vault-primitives", "xor"]
# Everything under tools/crosscheck/ that is the harness, not its output.
HARNESS_SKIP = {"stamp", "found", "__pycache__", "README.md"}

# Crates in that graph that compute nothing the cross-check checks: test
# dependencies, serialisation, macros and the operating system's
# randomness. Everything else the covered core crates pull in from
# crates.io, transitively, counts as crypto.
NOT_CRYPTO = {
    "bip39", "serde_json", "serde", "serde_core", "serde_derive",
    "proc-macro2", "quote", "syn", "unicode-ident", "libc", "getrandom", "r-efi",
}
CORE = ["faraday-crypto", "faraday-bip", "faraday-psbt", "faraday-entropy", "faraday-backup",
        "faraday-vault", "faraday-qr"]


def source_files():
    out = []
    for crate in CRATES:
        base = ROOT / crate
        if crate == "tools/crosscheck":
            files = [f for f in base.rglob("*") if f.is_file()
                     and not HARNESS_SKIP & set(f.relative_to(base).parts)]
        else:
            files = [base / "Cargo.toml"] + [f for f in (base / "src").rglob("*") if f.is_file()]
        out.extend(files)
    out.extend(ROOT / f for f in FILES)
    for d in VECTORS:
        out.extend(f for f in (ROOT / "tools" / "vectors" / d).rglob("*") if f.is_file())
    manifest = ROOT / "tools" / "crosscheck" / "references.sha256"
    for line in manifest.read_text().splitlines():
        if line.strip() and not line.startswith("#"):
            out.append(ROOT / line.split(None, 1)[1].strip())
    return sorted(set(out))


def lock_entries():
    """`name version checksum` of every crypto crate in Cargo.lock: the
    crates.io dependencies of the covered core crates, and theirs."""
    lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
    by_name = {}
    for p in lock["package"]:
        by_name.setdefault(p["name"], []).append(p)

    def deps(p):
        for d in p.get("dependencies", []):
            name, *version = d.split(" ")
            for q in by_name.get(name, []):
                if not version or q["version"] == version[0]:
                    yield q

    todo = [d for c in CORE for d in deps(by_name[c][0])
            if "source" in d and d["name"] not in NOT_CRYPTO]
    seen = {}
    while todo:
        p = todo.pop()
        key = (p["name"], p["version"])
        if key in seen or "source" not in p or p["name"] in NOT_CRYPTO:
            continue
        seen[key] = f'{p["name"]} {p["version"]} {p.get("checksum", p["source"])}'
        todo.extend(deps(p))
    return sorted(seen.values())


def digest():
    h = hashlib.sha256()
    for f in source_files():
        rel = f.relative_to(ROOT).as_posix()
        h.update(rel.encode() + b"\0" + hashlib.sha256(f.read_bytes()).digest())
    for line in lock_entries():
        h.update(line.encode() + b"\n")
    return h.hexdigest()


def write(seed, scale, seconds):
    refs = hashlib.sha256((ROOT / "tools/crosscheck/references.sha256").read_bytes()).hexdigest()
    STAMP.write_text(
        "# The last passing crypto cross-check (`just crosscheck`, F79).\n"
        "# `just lint` fails when `inputs` no longer matches the crypto sources,\n"
        "# the cross-check and the crypto crates' Cargo.lock entries.\n"
        f"inputs {digest()}\n"
        f"references {refs}\n"
        f"python {platform.python_version()}\n"
        f"{distribution_versions()}"
        f"seed {seed}\n"
        f"scale {scale}\n"
        f"seconds {seconds}\n"
    )


def distribution_versions():
    """The versions of the distribution's packages the vault and kdbx
    areas use (F79, docs/MULTIVENDOR.md §10.4), a line each."""
    from importlib import metadata
    out = ""
    for name in ("argon2-cffi", "PyNaCl", "pykeepass"):
        try:
            out += f"{name} {metadata.version(name)}\n"
        except metadata.PackageNotFoundError:
            out += f"{name} missing\n"
    return out


def recorded():
    if not STAMP.exists():
        return None
    for line in STAMP.read_text().splitlines():
        if line.startswith("inputs "):
            return line.split()[1]
    return None


def main():
    cmd = sys.argv[1] if len(sys.argv) > 1 else "check"
    if cmd == "show":
        for f in source_files():
            print(f.relative_to(ROOT).as_posix())
        for line in lock_entries():
            print(line)
        print(digest())
        return 0
    if cmd != "check":
        print(__doc__)
        return 2
    started = time.monotonic()
    if recorded() != digest():
        print("lint-crosscheck: crypto changed since the last cross-check: run `just crosscheck`")
        return 1
    print(f"lint-crosscheck: ok ({time.monotonic() - started:.2f} s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
