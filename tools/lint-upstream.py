#!/usr/bin/env python3
"""The project Faraday was forked from is named only in LICENSE (F73).

    lint-upstream.py           the self-test, then fail on a tracked file that names it
    lint-upstream.py --self-test

The name is read from LICENSE's notice, `Copyright (c) <year> The <Name>
Authors`, and is written nowhere else, this lint included. A LICENSE
without that notice names no upstream project: there is then nothing to
look for. A file names it when its bytes hold the name, or the name
without its last capitalised word, in any case, its words run together or
joined by `-` or `_`. Every file git tracks is read, binary files too, and
any occurrence fails. A path and a count are printed, never the name.
Standard library only.
"""

import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
NOTICE = re.compile(r"^Copyright \(c\) \d{4} The (\S+) Authors$", re.M)


def pattern(license_text):
    """The upstream name's forms, or None when LICENSE carries no notice."""
    m = NOTICE.search(license_text)
    if not m:
        return None
    words = re.findall(r"[A-Z][a-z0-9]*|[a-z0-9]+", m.group(1))
    forms = [words] + ([words[:-1]] if len(words) > 2 else [])
    alts = ["[-_]?".join(re.escape(w) for w in ws) for ws in forms]
    return re.compile(("|".join(alts)).encode(), re.I)


def found(root, files, rx):
    out = {}
    for f in files:
        if f == "LICENSE":
            continue
        p = root / f
        if p.is_file() and not p.is_symlink():
            n = len(rx.findall(p.read_bytes()))
            if n:
                out[f] = n
    return out


def compare(hits):
    return [f"{f} names the upstream project {n} time(s); F73 allows it in LICENSE only"
            for f, n in sorted(hits.items())]


def tracked(root):
    p = subprocess.run(["git", "-C", str(root), "ls-files", "-z"], capture_output=True, text=True)
    if p.returncode != 0:
        raise SystemExit("lint-upstream: git ls-files failed")
    return [f for f in p.stdout.split("\0") if f]


def self_test():
    fails = []
    rx = pattern("Copyright (c) 2026 The ExampleThingKit Authors\n")
    for text, n in ((b"exAmplethingkit", 1), (b"Example-Thing", 1), (b"example_thing_kit", 1),
                    (b"Example Thing", 0), (b"ExampleKit", 0)):
        if len(rx.findall(text)) != n:
            fails.append(f"{text!r} should match {n} time(s)")
    if compare({"a": 1}) == [] or compare({}):
        fails.append("a file naming the project passes, or a clean tree fails")
    if pattern("Copyright (c) 2026 Someone\n") is not None:
        fails.append("a LICENSE without the notice yields a name")
    for f in fails:
        print(f"lint-upstream self-test: {f}")
    print("lint-upstream self-test: " + ("FAILED" if fails else "ok"))
    return 1 if fails else 0


def main(argv):
    if argv == ["--self-test"]:
        return self_test()
    if argv:
        print(__doc__)
        return 2
    if self_test():
        return 1
    rx = pattern((ROOT / "LICENSE").read_text())
    if rx is None:
        print("lint-upstream: ok (LICENSE names no upstream project)")
        return 0
    errs = compare(found(ROOT, tracked(ROOT), rx))
    for e in errs:
        print(f"lint-upstream: {e}")
    if errs:
        return 1
    print("lint-upstream: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
