#!/usr/bin/env python3
"""Fetches the pinned Bitcoin Core release once, for the cross-check
(F79, docs/CROSSCHECK.md §10.4). Run by a person, with the network; the
cross-check itself never fetches anything.

    python3 -I -B tools/crosscheck/fetch_core.py           this machine's tarball
    python3 -I -B tools/crosscheck/fetch_core.py --machine aarch64
    python3 -I -B tools/crosscheck/fetch_core.py --from DIR   files already downloaded

1. Fetches `SHA256SUMS` and `SHA256SUMS.asc` from the release's url in
   `tools/reference/core/release.pin`, and the guix.sigs repository at
   the pinned commit (git checks the commit's hash).
2. Imports every key in guix.sigs' `builder-keys/` into a keyring of its
   own, and requires good signatures over `SHA256SUMS` from at least the
   pinned number of those builders, and no bad one.
3. Requires `SHA256SUMS`' line for the tarball to equal the pin.
4. Fetches the tarball, requires its SHA-256 to equal the pin, and keeps
   it, with the two signed files, in `FARADAY_BITCOIN_CORE` (default
   `~/.cache/faraday/bitcoin-core`).

Needs `git` and `gpg`. Standard library otherwise. With `--from DIR`,
`SHA256SUMS`, `SHA256SUMS.asc` and the tarball are read from DIR instead
of fetched (guix.sigs is still fetched unless DIR holds a `guix.sigs`
checkout at the pinned commit); every check is the same.
"""

import argparse
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import urllib.request

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import core_node  # noqa: E402


def say(text):
    print(f"fetch-core: {text}", flush=True)


def fail(text):
    print(f"fetch-core: FAILED: {text}", file=sys.stderr)
    sys.exit(1)


def download(url, dest):
    """Costs a download."""
    say(f"fetching {url}")
    with urllib.request.urlopen(url, timeout=120) as r, open(dest, "wb") as f:
        shutil.copyfileobj(r, f)


def guix_sigs(p, work, given):
    """The guix.sigs checkout at the pinned commit: `given` when it is
    one, otherwise fetched. Costs a git fetch."""
    if given and (given / ".git").exists():
        head = subprocess.run(["git", "-C", str(given), "rev-parse", "HEAD"],
                              capture_output=True, text=True).stdout.strip()
        if head != p["guix_commit"]:
            fail(f"{given} is at {head}, pinned {p['guix_commit']}")
        return given
    dest = work / "guix.sigs"
    say(f"fetching {p['guix_repo']} at {p['guix_commit']}")
    for cmd in (["git", "init", "-q", str(dest)],
                ["git", "-C", str(dest), "fetch", "-q", "--depth", "1", p["guix_repo"],
                 p["guix_commit"]],
                ["git", "-C", str(dest), "-c", "advice.detachedHead=false", "checkout", "-q",
                 "FETCH_HEAD"]):
        if subprocess.run(cmd).returncode != 0:
            fail(f"{' '.join(cmd)} failed")
    head = subprocess.run(["git", "-C", str(dest), "rev-parse", "HEAD"],
                          capture_output=True, text=True).stdout.strip()
    if head != p["guix_commit"]:
        fail(f"guix.sigs checked out at {head}, pinned {p['guix_commit']}")
    return dest


def builder_signatures(sums, asc, keys, work):
    """The builders whose keys sign `sums` with a good signature, and the
    bad signatures, read from gpg's status lines. Costs a gpg verify."""
    home = work / "gnupg"
    home.mkdir(mode=0o700)
    env = dict(os.environ, GNUPGHOME=str(home))
    owners = {}
    for key in sorted(keys.glob("*.gpg")):
        before = set(_fingerprints(env))
        subprocess.run(["gpg", "--batch", "--quiet", "--import", str(key)], env=env,
                       capture_output=True)
        for fpr in set(_fingerprints(env)) - before:
            owners[fpr] = key.stem
    if not owners:
        fail(f"no builder key could be imported from {keys}")
    out = subprocess.run(["gpg", "--batch", "--status-fd", "1", "--verify", str(asc), str(sums)],
                         env=env, capture_output=True, text=True)
    good, bad = set(), []
    for line in out.stdout.splitlines():
        parts = line.split()
        if len(parts) < 3 or parts[0] != "[GNUPG:]":
            continue
        if parts[1] == "VALIDSIG":
            primary = parts[-1]
            if primary in owners:
                good.add(owners[primary])
        elif parts[1] == "BADSIG":
            bad.append(" ".join(parts[2:]))
    return good, bad


def _fingerprints(env):
    out = subprocess.run(["gpg", "--batch", "--with-colons", "--list-keys"], env=env,
                         capture_output=True, text=True).stdout
    fprs, primary = [], False
    for line in out.splitlines():
        fields = line.split(":")
        if fields[0] == "pub":
            primary = True
        elif fields[0] == "fpr" and primary:
            fprs.append(fields[9])
            primary = False
    return fprs


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--machine", help="x86_64 or aarch64 (default: this machine's)")
    ap.add_argument("--from", dest="given", type=pathlib.Path,
                    help="a folder holding SHA256SUMS, SHA256SUMS.asc and the tarball")
    args = ap.parse_args()
    for tool in ("git", "gpg"):
        if not shutil.which(tool):
            fail(f"{tool} is not on the PATH")
    p = core_node.pin()
    try:
        name = core_node.tarball_name(p, args.machine)
    except core_node.CoreError as e:
        fail(str(e))
    dest = core_node.folder()
    with tempfile.TemporaryDirectory(prefix="faraday-fetch-core-") as tmp:
        work = pathlib.Path(tmp)
        files = {}
        for f in ("SHA256SUMS", "SHA256SUMS.asc", name):
            files[f] = work / f
            if args.given:
                shutil.copyfile(args.given / f, files[f])
            elif f != name:
                download(p["url"] + f, files[f])
        sigs = guix_sigs(p, work, args.given / "guix.sigs" if args.given else None)

        good, bad = builder_signatures(files["SHA256SUMS"], files["SHA256SUMS.asc"],
                                       sigs / "builder-keys", work)
        if bad:
            fail(f"SHA256SUMS.asc holds a bad signature: {'; '.join(bad)}")
        if len(good) < p["min_signatures"]:
            fail(f"SHA256SUMS is signed by {len(good)} guix.sigs builders "
                 f"({', '.join(sorted(good)) or 'none'}), {p['min_signatures']} required")
        say(f"SHA256SUMS signed by {len(good)} guix.sigs builders: {', '.join(sorted(good))}")

        listed = {}
        for line in files["SHA256SUMS"].read_text().splitlines():
            digest, _, fname = line.partition("  ")
            listed[fname.strip()] = digest.strip()
        if listed.get(name) != p["sha256"][name]:
            fail(f"SHA256SUMS gives {name} {listed.get(name)}, "
                 f"pinned {p['sha256'][name]} in tools/reference/core/release.pin")

        if not args.given:
            download(p["url"] + name, files[name])
        got = core_node.sha256_file(files[name])
        if got != p["sha256"][name]:
            fail(f"{name} has SHA-256 {got}, pinned and signed {p['sha256'][name]}")

        dest.mkdir(parents=True, exist_ok=True)
        for f in ("SHA256SUMS", "SHA256SUMS.asc", name):
            part = dest / (f + ".part")
            shutil.copyfile(files[f], part)
            part.replace(dest / f)
    say(f"Bitcoin Core {p['version']} kept in {dest / name} (SHA-256 {got})")
    if os.environ.get(core_node.ENV_DIR) is None:
        say(f"the cross-check finds it there; elsewhere, set {core_node.ENV_DIR}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
