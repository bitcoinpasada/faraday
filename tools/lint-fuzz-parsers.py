#!/usr/bin/env python3
"""A fuzz target per parser (tools/fuzz-parsers.toml; tools/README.md).

Finds every `pub fn` in core/'s non-test code named `parse*`, `decode*`,
`from_bytes`, `from_str` or `from_slice` that takes a `&[u8]` or `&str`
(or `Vec<u8>`, `impl AsRef<[u8]>`), and fails when its (file, name) is not
an entry of tools/fuzz-parsers.toml, when an entry names a function that is
no longer there, or when its fuzz target is not a file or does not mention
the function's name. Entries for faraday/ are checked the same way against
faraday/, but a new parser there is not required to be listed. Code after
the first `#[cfg(test)]` of a file is not read. Run from anywhere; `just
lint` does. The standard library only.
"""
import pathlib
import re
import subprocess
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parent.parent
LIST = ROOT / "tools/fuzz-parsers.toml"
FN = re.compile(
    r"^\s*pub(?:\([a-z]+\))?\s+(?:const\s+)?fn\s+"
    r"((?:parse|decode)\w*|from_bytes|from_str|from_slice)\b\s*(?:<[^>]*>)?\(([^)]*)\)",
    re.S | re.M)
TAKES = re.compile(r"&\s*(?:'\w+\s+)?(?:\[u8\]|str)|AsRef<\[u8\]>|impl AsRef<str>|Vec<u8>")


def parsers(prefix):
    """The (file, name) of each parser under `prefix`. Reads the files."""
    files = subprocess.run(["git", "ls-files", "-z", "--cached", "--others",
                            "--exclude-standard", "--", f"{prefix}*.rs"],
                           cwd=ROOT, check=True, capture_output=True,
                           text=True).stdout.split("\0")
    found = set()
    for f in sorted(files):
        if not f or any(s in f for s in ("/tests/", "/fuzz/", "/benches/")):
            continue
        if not (ROOT / f).exists():
            continue
        text = (ROOT / f).read_text(encoding="utf-8")
        cut = text.find("#[cfg(test)]")
        if cut >= 0:
            text = text[:cut]
        for m in FN.finditer(text):
            if TAKES.search(m.group(2)):
                found.add((f, m.group(1)))
    return found


def main():
    listed = tomllib.loads(LIST.read_text(encoding="utf-8")).get("parser", [])
    failures = []
    keys = set()
    for e in listed:
        key = (e.get("file"), e.get("fn"))
        where = f"tools/fuzz-parsers.toml: {key[0]} {key[1]}"
        if None in key or ("target" in e) == ("excuse" in e):
            failures.append(f"{where}: needs file, fn and exactly one of target, excuse")
            continue
        if key in keys:
            failures.append(f"{where}: listed twice")
        keys.add(key)
        if "target" in e:
            t = ROOT / e["target"]
            if not t.is_file():
                failures.append(f"{where}: target {e['target']} does not exist")
            elif key[1] not in t.read_text(encoding="utf-8"):
                failures.append(f"{where}: target {e['target']} does not call it")
        elif not e["excuse"].strip():
            failures.append(f"{where}: empty excuse")
    for prefix in ("core/", "faraday/"):
        found = parsers(prefix)
        for f, name in sorted(found - keys if prefix == "core/" else set()):
            failures.append(
                f"{f}: parser `{name}` has no entry in tools/fuzz-parsers.toml; "
                "add a fuzz target and list it, or list it with an excuse")
        for key in sorted(k for k in keys if k[0].startswith(prefix)):
            if key not in found:
                failures.append(f"tools/fuzz-parsers.toml: {key[0]} {key[1]} "
                                "is not a parser of the tree any more; remove it")
    for m in failures:
        print(m)
    if failures:
        print(f"lint-fuzz-parsers: {len(failures)} failure(s)", file=sys.stderr)
        return 1
    print(f"lint-fuzz-parsers: ok ({len(keys)} parsers listed)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
