#!/usr/bin/env python3
"""Fetches the pinned Core Lightning source release once, for the
cross-check's BOLT #12 judge (F79, F232, docs/CROSSCHECK.md §10.4). Run by a
person, with the network; the cross-check itself never fetches anything.

    python3 -I -B tools/crosscheck/fetch_cln.py            fetch the archive
    python3 -I -B tools/crosscheck/fetch_cln.py --from DIR  an archive already downloaded

Fetches the release's `SHA256SUMS-<version>` and the archive named in
`tools/reference/bolt12/release.pin`, requires the archive's SHA-256 to
equal both the pin and the release's list, and keeps it in
`FARADAY_CLN_SOURCE` (default `~/.cache/faraday/core-lightning`). Standard
library only.
"""

import argparse
import hashlib
import os
import pathlib
import shutil
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[2]
PIN = ROOT / "tools/reference/bolt12/release.pin"


def say(text):
    print(f"fetch-cln: {text}", flush=True)


def fail(text):
    print(f"fetch-cln: FAILED: {text}", file=sys.stderr)
    sys.exit(1)


def read_pin():
    pin = {"sha256": {}}
    for line in PIN.read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        key, _, rest = line.partition(" ")
        if key == "sha256":
            name, _, digest = rest.partition(" ")
            pin["sha256"][name] = digest
        else:
            pin[key] = rest
    return pin


def cache_dir():
    return pathlib.Path(os.environ.get("FARADAY_CLN_SOURCE")
                        or pathlib.Path.home() / ".cache/faraday/core-lightning")


def sha256_of(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def download(url, dest):
    """Costs a download."""
    say(f"fetching {url}")
    with urllib.request.urlopen(url, timeout=120) as r, open(dest, "wb") as f:
        shutil.copyfileobj(r, f)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--from", dest="src", help="a folder holding the archive already")
    given = ap.parse_args().src
    pin = read_pin()
    (name, want), = pin["sha256"].items()
    dest = cache_dir()
    dest.mkdir(parents=True, exist_ok=True)
    out = dest / name
    tmp = dest / (name + ".part")
    sums = dest / ("SHA256SUMS-" + pin["version"])
    if given:
        shutil.copyfile(pathlib.Path(given) / name, tmp)
    else:
        download(pin["url"] + "SHA256SUMS-" + pin["version"], sums)
        download(pin["url"] + name, tmp)
    got = sha256_of(tmp)
    if got != want:
        tmp.unlink()
        fail(f"{name} hashes to {got}, the pin says {want}")
    if not given:
        listed = [line.split()[0] for line in sums.read_text().splitlines()
                  if line.split()[-1:] == [name]]
        if listed != [want]:
            tmp.unlink()
            fail(f"the release's SHA256SUMS does not list {want} for {name}")
    tmp.replace(out)
    say(f"{out} matches the pin")


if __name__ == "__main__":
    main()
