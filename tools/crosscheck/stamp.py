#!/usr/bin/env python3
"""The cross-check's stamp (F79, docs/CROSSCHECK.md §10.2).

A passing `just crosscheck` writes `tools/crosscheck/stamp`: one hash
over the crypto sources, the cross-check itself and its references, and
the `Cargo.lock` entries of the crypto crates, with the seed, the
Python it ran under, Bitcoin Core's version and tarball SHA-256, how
long it took and each area's corpus case count. `just lint-release` runs
`stamp.py check`, which hashes the same inputs and fails when they no
longer match, or when the stamp's Bitcoin Core is not the release
`tools/reference/core/release.pin` pins: a release does not carry a
change to the crypto, a version bump of a crypto crate or a new Core
without a passing cross-check. Between releases the tip is not held to
it (F197). It is a record,
not a security control. Standard library only; about a tenth of a
second.

The same inputs are also split in two (F239, `parts`): what the areas
share, and each area's own (its `area_<name>.py` and the reference folders
only it uses), so that a release's cross-check can be made of two runs at
one seed, the second running only the areas whose own inputs changed
(`crosscheck.py --combine`).

    stamp.py check     exit 1 when the inputs changed since the stamp
    stamp.py show      print what is hashed
    stamp.py parts     print which files are each area's own
"""

import ast
import hashlib
import pathlib
import platform
import re
import sys
import time
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[2]
HERE = ROOT / "tools" / "crosscheck"
sys.path.insert(0, str(HERE))
STAMP = ROOT / "tools" / "crosscheck" / "stamp"

# The code whose results the cross-check checks, and the cross-check:
# each crate's sources and manifest.
CRATES = [
    "core/faraday-crypto",
    "core/faraday-bip",
    "core/faraday-psbt",
    "core/faraday-entropy",
    "core/faraday-codec",
    "faraday/faraday-intake",
    "faraday/faraday-kef",
    "faraday/faraday-keys",
    "faraday/faraday-ots",
    "faraday/faraday-qr",
    "faraday/faraday-vault",
    "third_party/rqrr",
    "tools/crosscheck",
]
# Single files of crates too large to hash whole, whose results an area
# checks: the session's fresh randomness and what a person adds
# (`own-randomness`), Faraday Demo's dice rollers (`roller`), the
# picture a Codex32QR is written as (`seedqr`), the device's own draw mixed
# with a person's (`device_own`, `own-randomness`) and Decode's txid, wtxid
# and output addresses (`tx`) and a Core wallet's cached account keys
# (`core_wallet`).
FILES = ["faraday/faraday-core/src/fresh.rs", "faraday/faraday-core/src/fresh/roller/mod.rs",
         "faraday/faraday-core/src/fresh/roller/five.rs", "faraday/faraday-core/src/picture.rs",
         "faraday/faraday-core/src/keygen/mod.rs", "faraday/faraday-core/src/decode/mod.rs",
         "faraday/faraday-core/src/core_wallet/normal.rs"]
# The published vectors the cross-check reads (tools/vectors/<name>/).
VECTORS = ["addresses", "aezeed", "bip129", "bip32", "bip327", "bip352", "bip374", "bip375", "bip39", "bip85", "bip93",
           "codex32-booklet", "codex32-correction", "codex32-volvelle", "codex32qr", "entropy",
           "kdf", "message", "miniscript", "opentimestamps", "psbt", "psbt-bips", "seedqr", "sighash", "slip39", "ur", "vault",
           "vault-primitives", "wallet-files", "xor"]
# Other wallets' files from their test suites, which the `import` area reads.
FOLDERS = ["faraday/faraday-core/tests/vectors/interop"]
# Everything under tools/crosscheck/ that is the harness, not its output.
HARNESS_SKIP = {"stamp", "__pycache__", "README.md"}

# Crates in that graph that compute nothing the cross-check checks: test
# dependencies, serialisation, macros and the operating system's
# randomness. Everything else the covered core crates pull in from
# crates.io, transitively, counts as crypto.
NOT_CRYPTO = {
    "bip39", "serde_json", "serde", "serde_core", "serde_derive",
    "proc-macro2", "quote", "syn", "unicode-ident", "libc", "getrandom", "r-efi",
}
CORE = ["faraday-crypto", "faraday-bip", "faraday-psbt", "faraday-entropy",
        "faraday-vault", "faraday-qr", "faraday-kef"]


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


# ----- What the areas share, and each area's own (F239) ----------------------

MANIFEST = HERE / "references.sha256"
# How an area names a reference folder it uses: through `use_reference` or
# `use_wheels`, or in a path to it.
USES = (r'(?:use_reference|use_wheels)\(\s*["\']{n}["\']'
        r'|reference/{n}(?![\w.-])'
        r'|["\']reference["\']\s*[,/]\s*["\']{n}["\']'
        r'|REFERENCES?\s*/\s*["\']{n}["\']')
# Anything else that could name it: the folder's name as a string, or a path;
# in Rust (Faraday's answering program and the outside programs built
# beside it, whose strings name operations), a path.
NAMES = r'["\']{n}["\']|reference/{n}(?![\w.-])'
PATHS = r'reference/{n}(?![\w.-])'


def harness_texts(folder=None):
    """{file: text} of the harness's files, as an area's own files are
    told from them: the manifest, which lists every folder, this file,
    which names every input, and the harness's own tests, which never run
    in a cross-check, left out; in
    `crosscheck.py`, the area table (each area's declaration, not a use)
    left out, and so is the test areas' table (`areas.json`). A file that
    is not text names nothing."""
    files = [f for f in source_files() if f.is_relative_to(HERE)]
    if folder:
        files += folder_files(folder)
    out = {}
    for f in files:
        if f in (MANIFEST, HERE / "stamp.py") or f.name == "test_harness.py" \
                or folder and f == pathlib.Path(folder) / "areas.json":
            continue
        try:
            text = f.read_text()
        except (UnicodeDecodeError, OSError):
            continue
        if f == HERE / "crosscheck.py":
            text = without_area_table(text)
        out[f] = text
    return out


def without_area_table(text):
    lines = text.splitlines(keepends=True)
    for node in ast.parse(text).body:
        if isinstance(node, ast.Assign) and getattr(node.targets[0], "id", "") == "AREAS":
            del lines[node.lineno - 1:node.end_lineno]
            break
    return "".join(lines)


def folder_files(folder):
    """The code of the harness's own test areas (`--test-areas`): their
    Python files and their table, not what a run writes beside them."""
    folder = pathlib.Path(folder)
    return sorted(f for f in folder.rglob("*") if f.is_file() and "__pycache__" not in f.parts
                  and (f.suffix == ".py" or f.name == "areas.json"))


def reference_folders():
    """{folder name: [manifest lines]} of the vendored references."""
    out = {}
    for line in MANIFEST.read_text().splitlines():
        if line.strip() and not line.startswith("#"):
            path = line.split(None, 1)[1].strip()
            parts = path.split("/")
            if len(parts) > 3 and parts[:2] == ["tools", "reference"]:
                out.setdefault(parts[2], []).append(line)
    return out


def owners(areas, folder=None):
    """({file: area}, {reference folder: area}): the inputs only one area
    uses. An area's module is its own when no other area runs it and no
    other harness file names it; a reference folder is an area's own when
    that area's module uses it and no other harness file names it. Every
    other input is shared: a file two areas use, or whose user cannot be
    told, is shared. `areas` is {area: module}."""
    texts = harness_texts(folder)
    base = pathlib.Path(folder) if folder else HERE
    modules = list(areas.values())
    files = {}
    for area, module in areas.items():
        path = base / f"{module}.py"
        if modules.count(module) != 1 or path not in texts:
            continue
        word = re.compile(rf"(?<![\w]){re.escape(module)}(?![\w])")
        if any(word.search(t) for f, t in texts.items() if f != path):
            continue
        files[path] = area
    folders = {}
    for name in reference_folders():
        uses = re.compile(USES.format(n=re.escape(name)))
        names = re.compile(NAMES.format(n=re.escape(name)))
        paths = re.compile(PATHS.format(n=re.escape(name)))
        users = [f for f in files if uses.search(texts[f])]
        if len(users) != 1:
            continue
        if any((paths if f.suffix == ".rs" else names).search(t)
               for f, t in texts.items() if f != users[0]):
            continue
        folders[name] = files[users[0]]
    return files, folders


def tool_versions(interpreters=()):
    """The tools the areas ran with, as the shared inputs take them: the
    Python, the distribution's packages and the judges' versions, and any
    other interpreter's."""
    return (f"python {platform.python_version()}\n" + distribution_versions()
            + "".join(f"interpreter {interpreter(p)}\n" for p in list(interpreters)[1:]))


def parts(areas, tools, folder=None):
    """Costs a read of every input, as `digest`, and of the harness's
    files once more. (shared, {area: own}): SHA-256 over what the areas
    share (the inputs `digest` hashes but the areas' own, the crypto
    crates' Cargo.lock entries and `tools`) and over each area's own
    files and reference folders, its lines of the manifest included."""
    files, folders = owners(areas, folder)
    shared = hashlib.sha256()
    own = {area: hashlib.sha256() for area in areas}
    every = source_files() + (folder_files(folder) if folder else [])
    for f in every:
        if f == MANIFEST:
            continue
        rel = (f.relative_to(ROOT).as_posix() if f.is_relative_to(ROOT)
               else "test-areas/" + f.relative_to(folder).as_posix())
        area = files.get(f) or folders.get(rel.split("/")[2] if rel.startswith("tools/reference/")
                                            else None)
        (own[area] if area else shared).update(
            rel.encode() + b"\0" + hashlib.sha256(f.read_bytes()).digest())
    for name, lines in reference_folders().items():
        h = own[folders[name]] if name in folders else shared
        for line in lines:
            h.update(line.encode() + b"\n")
    for line in MANIFEST.read_text().splitlines():
        if not line.strip() or line.startswith("#") or not line.split(None, 1)[1].strip() \
                .startswith("tools/reference/"):
            shared.update(line.encode() + b"\n")
    for line in lock_entries():
        shared.update(line.encode() + b"\n")
    shared.update(tools.encode())
    return shared.hexdigest(), {area: h.hexdigest() for area, h in own.items()}


def interpreter(path):
    """`implementation version` of a Python, asked."""
    import subprocess
    out = subprocess.run([path, "-I", "-c", "import platform; print("
                          "platform.python_implementation(), platform.python_version())"],
                         capture_output=True, text=True)
    return out.stdout.strip() or "unknown"


def write(seed, scale, seconds, jobs=1, interpreters=(), cases=None, core=None, runs=()):
    refs = hashlib.sha256((ROOT / "tools/crosscheck/references.sha256").read_bytes()).hexdigest()
    # The number of workers is the machine's, not the cross-check's: it is
    # left out of the stamp.
    faster = ", ".join(interpreter(p) for p in interpreters[1:])
    workers = f"{faster} for the areas it speeds\n" if faster else ""
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
        # A run made of two (F239): each run, its date, tree and areas (the
        # last one's tree is the stamp's own commit's, which it cannot name).
        + "".join(f"run {r}\n" for r in runs)
    )


def distribution_versions():
    """The versions of the distribution's packages the vault area
    and the second-lineage checks use (F79, docs/CROSSCHECK.md §10.4), a line each."""
    from importlib import metadata
    out = ""
    for name in ("argon2-cffi", "PyNaCl", "cryptography", "pykeepass", "ecdsa"):
        try:
            out += f"{name} {metadata.version(name)}\n"
        except metadata.PackageNotFoundError:
            out += f"{name} missing\n"
    try:
        from cryptography.hazmat.backends.openssl import backend
        out += f"OpenSSL {backend.openssl_version_text()}\n"
    except ImportError:
        out += "OpenSSL missing\n"
    import judges
    return out + f"{go_version()}\n{libsodium_version()}\n" + judges.versions()


def libsodium_version():
    """`libsodium <version>` of the library Core Lightning's answer program
    (tools/reference/cln) is linked with, or `libsodium missing`."""
    import shutil
    import subprocess
    exe = shutil.which("pkg-config")
    if not exe:
        return "libsodium missing"
    run = subprocess.run([exe, "--modversion", "libsodium"], capture_output=True, text=True)
    version = run.stdout.strip()
    return f"libsodium {version}" if run.returncode == 0 and version else "libsodium missing"


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
    if cmd == "parts":
        import facts
        files, folders = owners(dict(facts.areas()))
        for area, _ in facts.areas():
            own = [f.relative_to(ROOT).as_posix() for f, a in files.items() if a == area]
            own += [f"tools/reference/{n}/" for n, a in folders.items() if a == area]
            print(f"{area}: {', '.join(own) or 'nothing of its own'}")
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
