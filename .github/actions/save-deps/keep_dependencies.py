#!/usr/bin/env python3
"""Cuts a cargo target folder down to compiled dependencies (docs/decisions/F212.md).

  keep_dependencies.py TARGET

Run from the workspace root, before the folder goes in GitHub's cache. It keeps
what cargo built for packages from a registry or a git source, and removes
everything else: anything of a package with a path (Faraday's own crates), test
binaries, final binaries, incremental data. What it keeps is named by `cargo
metadata`, so a file it cannot place is removed.
"""
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

HASHED = re.compile(r"^(?:lib)?(?P<name>.+)-[0-9a-f]{16}(?:\.[^/]+)?$")
KEPT_IN_PROFILE = {"deps", "build", ".fingerprint"}


def remove(path):
    if path.is_dir() and not path.is_symlink():
        shutil.rmtree(path)
    else:
        path.unlink()


def outside_names():
    """(package names, crate names) with a source and no path package of that name."""
    meta = json.loads(subprocess.run(["cargo", "metadata", "--locked", "--format-version", "1"],
                                     capture_output=True, text=True, check=True).stdout)
    def crates(p):
        return {t["name"].replace("-", "_") for t in p["targets"]}
    own = [p for p in meta["packages"] if p["source"] is None]
    others = [p for p in meta["packages"] if p["source"] is not None]
    own_packages = {p["name"] for p in own}
    own_crates = set().union(*map(crates, own))
    return ({p["name"] for p in others} - own_packages,
            set().union(*map(crates, others)) - own_crates)


def cut(profile, packages, crates):
    """Returns (entries kept, entries removed) in one profile folder."""
    kept = gone = 0
    for entry in profile.iterdir():
        if entry.name not in KEPT_IN_PROFILE:
            remove(entry)
            gone += 1
    for sub, names in ((".fingerprint", packages), ("build", packages), ("deps", crates)):
        folder = profile / sub
        if not folder.is_dir():
            continue
        for entry in folder.iterdir():
            m = HASHED.match(entry.name)
            if m and m["name"] in names:
                kept += 1
            else:
                remove(entry)
                gone += 1
    return kept, gone


def main():
    target = Path(sys.argv[1])
    packages, crates = outside_names()
    profiles = [p.parent for pattern in ("*/.fingerprint", "*/*/.fingerprint") for p in target.glob(pattern)]
    kept = gone = 0
    for profile in profiles:
        k, g = cut(profile, packages, crates)
        kept, gone = kept + k, gone + g
    # Any other folder at the top: nextest's, the tests' own.
    keep = {q for p in profiles for q in (p, *p.parents)}
    for top in list(target.iterdir()):
        if top.is_dir() and top not in keep:
            remove(top)
    print(f"{kept} kept, {gone} removed, in {len(profiles)} profile folders")


if __name__ == "__main__":
    main()
