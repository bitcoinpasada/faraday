#!/usr/bin/env python3
"""The cross-check's stamp (F79, docs/CROSSCHECK.md §10.2).

A passing `just crosscheck` writes `tools/crosscheck/stamp`: one hash
over the crypto sources, the cross-check itself and its references, and
the `Cargo.lock` entries of the crypto crates, with the seed, the
Python it ran under, Bitcoin Core's version and tarball SHA-256, how
long it took and each area's corpus case count. `just lint` runs
`stamp.py check`, which hashes the same inputs and fails when they no
longer match, or when the stamp's Bitcoin Core is not the release
`tools/reference/core/release.pin` pins: a change to the crypto, a
version bump of a crypto crate or a new Core is not committed without a
passing cross-check. It is a record,
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
sys.path.insert(0, str(ROOT / "tools" / "crosscheck"))
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
    "faraday/faraday-intake",
    "faraday/faraday-keys",
    "faraday/faraday-qr",
    "faraday/faraday-vault",
    "tools/crosscheck",
]
# Single files of crates too large to hash whole, whose results an area
# checks: the session's fresh randomness and what a person adds
# (`own-randomness`), Faraday Demo's dice rollers (`roller`), and the
# picture a Codex32QR is written as (`seedqr`).
FILES = ["faraday/faraday-core/src/fresh.rs", "faraday/faraday-core/src/fresh/roller/mod.rs",
         "faraday/faraday-core/src/fresh/roller/five.rs", "faraday/faraday-core/src/picture.rs"]
# The published vectors the cross-check reads (tools/vectors/<name>/).
VECTORS = ["addresses", "aezeed", "bip129", "bip32", "bip327", "bip352", "bip374", "bip375", "bip39", "bip85", "bip93",
           "codex32-booklet", "codex32-correction", "codex32-volvelle", "codex32qr", "entropy",
           "kdf", "message", "miniscript", "psbt", "psbt-bips", "seedqr", "sighash", "slip39", "ur", "vault",
           "vault-primitives", "xor"]
# Other wallets' files from their test suites, which the `import` area reads.
FOLDERS = ["faraday/faraday-core/tests/vectors/interop"]
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
    for d in FOLDERS:
        out.extend(f for f in (ROOT / d).rglob("*") if f.is_file())
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


def interpreter(path):
    """`implementation version` of a Python, asked."""
    import subprocess
    out = subprocess.run([path, "-I", "-c", "import platform; print("
                          "platform.python_implementation(), platform.python_version())"],
                         capture_output=True, text=True)
    return out.stdout.strip() or "unknown"


def write(seed, scale, seconds, jobs=1, interpreters=(), cases=None, core=None):
    refs = hashlib.sha256((ROOT / "tools/crosscheck/references.sha256").read_bytes()).hexdigest()
    workers = ", ".join(interpreter(p) for p in interpreters[1:])
    workers = f"workers {jobs}" + (f"; {workers} for the areas it speeds" if workers else "") + "\n"
    STAMP.write_text(
        "# The last passing crypto cross-check (`just crosscheck`, F79).\n"
        "# `just lint` fails when `inputs` no longer matches the crypto sources,\n"
        "# the cross-check and the crypto crates' Cargo.lock entries.\n"
        f"inputs {digest()}\n"
        f"references {refs}\n"
        f"python {platform.python_version()}\n"
        f"{distribution_versions()}"
        + (f"bitcoin-core {core['version']} {core['tarball']} {core['sha256']}\n" if core else "")
        + f"seed {seed}\n"
        f"scale {scale}\n"
        f"{workers}"
        f"seconds {seconds}\n"
        + "".join(f"cases {area} {n}\n" for area, n in (cases or {}).items())
    )


def distribution_versions():
    """The versions of the distribution's packages the vault and kdbx
    areas and the second-lineage checks use (F79, docs/CROSSCHECK.md §10.4), a line each."""
    from importlib import metadata
    out = ""
    for name in ("argon2-cffi", "PyNaCl", "cryptography", "pykeepass", "ecdsa"):
        try:
            out += f"{name} {metadata.version(name)}\n"
        except metadata.PackageNotFoundError:
            out += f"{name} missing\n"
    return out + f"{go_version()}\n"


def go_version():
    """`go <version>` of the toolchain on the PATH that builds LND's answer
    program (tools/reference/lnd), or `go missing`."""
    import shutil
    import subprocess
    exe = shutil.which("go")
    if not exe:
        return "go missing"
    run = subprocess.run([exe, "env", "GOVERSION"], capture_output=True, text=True)
    version = run.stdout.strip().removeprefix("go")
    return f"go {version}" if version else "go unknown"


def recorded(key="inputs"):
    """The rest of the stamp's line that starts with `key`, or None."""
    if not STAMP.exists():
        return None
    for line in STAMP.read_text().splitlines():
        if line.startswith(key + " "):
            return line.split(None, 1)[1]
    return None


def core_matches_pin():
    """None when the stamp's Bitcoin Core is the pinned release (its
    version and one of its tarballs' SHA-256), else what is wrong."""
    import core_node
    p = core_node.pin()
    line = recorded("bitcoin-core")
    if line is None:
        return "the stamp names no Bitcoin Core"
    version, tarball, digest = line.split()
    if not version.startswith(p["version"] + ".") and version != p["version"]:
        return f"the stamp's Bitcoin Core is {version}, pinned {p['version']}"
    if p["sha256"].get(tarball) != digest:
        return f"the stamp's Bitcoin Core tarball {tarball} {digest} is not the pinned one"
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
    if (recorded() or "").split()[:1] != [digest()]:
        print("lint-crosscheck: crypto changed since the last cross-check: run `just crosscheck`")
        return 1
    wrong = core_matches_pin()
    if wrong:
        print(f"lint-crosscheck: {wrong}: run `just crosscheck`")
        return 1
    print(f"lint-crosscheck: ok ({time.monotonic() - started:.2f} s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
