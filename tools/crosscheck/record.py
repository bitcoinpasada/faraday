#!/usr/bin/env python3
"""The cross-check's record (F79, docs/CROSSCHECK.md §10): what a release
carries beside SHA256SUMS to say what was checked.

A full `just crosscheck --fresh` writes `out/crosscheck-record.txt`, plain
text in a stable order: the commit, the seed, the scale and the UTC date;
each area with its corpus cases, answers compared, known differences and
result; each reference under `tools/reference/` with the commit its README
names and the SHA-256 of its files, and Bitcoin Core's tarball with its
version and SHA-256; and either "every answer agreed" or the first
disagreement. Paths in it are relative to the repository. The release
flow renames it `crosscheck-<version>.txt`
(`tools/README.md`, "Before a release"). Standard library only.
"""

import datetime
import hashlib
import pathlib
import re
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[2]
REFERENCES = ROOT / "tools" / "reference"
NAME = "crosscheck-record.txt"


def git(*args):
    """The output of a git command in the repository, or None."""
    run = subprocess.run(["git", "-c", f"safe.directory={ROOT}", "-C", str(ROOT), *args],
                         capture_output=True, text=True)
    return run.stdout.strip() if run.returncode == 0 else None


def commit():
    """Costs a git call. The commit the run was made at, marked when the
    tree differs from it."""
    head = git("rev-parse", "HEAD")
    if not head:
        return "unknown"
    return head + (" (uncommitted changes)" if git("status", "--porcelain") else "")


def folder_digest(folder):
    """Costs a read of the folder's files. `(files, SHA-256)` over each
    file's relative path and content hash, in path order."""
    h = hashlib.sha256()
    n = 0
    for f in sorted(p for p in folder.rglob("*") if p.is_file() and "__pycache__" not in p.parts):
        h.update(f.relative_to(folder).as_posix().encode() + b"\0"
                 + hashlib.sha256(f.read_bytes()).digest())
        n += 1
    return n, h.hexdigest()


def named_commit(folder):
    """The first 40-digit commit the folder's README names, or `-`."""
    readme = folder / "README.md"
    m = re.search(r"\b[0-9a-f]{40}\b", readme.read_text()) if readme.exists() else None
    return m.group(0) if m else "-"


def references():
    lines = []
    for d in sorted(p for p in REFERENCES.iterdir() if p.is_dir()):
        n, digest = folder_digest(d)
        lines.append(f"  {d.relative_to(ROOT).as_posix()}  commit {named_commit(d)}  "
                     f"{n} files  sha256 {digest}")
    return lines


def text(seed, scale, results, core, interpreters, jobs, allow_network=False, now=None):
    """The record. `results` is each area's merged result, in the order run."""
    now = now or datetime.datetime.now(datetime.timezone.utc)
    out = ["Faraday crypto cross-check record",
           f"commit {commit()}",
           f"seed {seed}",
           f"scale {scale}",
           f"date {now.strftime('%Y-%m-%d %H:%M:%S')} UTC",
           f"workers {jobs}",
           "python " + ", ".join(interpreters)]
    if allow_network:
        out.append("network allowed (not a release run)")
    out += ["", "areas:"]
    first = None
    for r in results:
        known = r.get("known", 0)
        out.append(f"  {r['area']}: {r['status']}, {r.get('cases', 0)} cases, "
                   f"{r.get('compared', 0)} answers, {known} known differences")
        if r["status"] != "pass" and first is None:
            first = r
    out += ["", "references:"] + references()
    if core:
        out.append(f"  bitcoin-core {core['version']}  {core['tarball']}  sha256 {core['sha256']}")
    else:
        out.append("  bitcoin-core: not run")
    out.append("")
    if first is None:
        out.append("result: every answer agreed")
    else:
        out.append(f"result: first disagreement in {first['area']}"
                   + (f" at case {first['case']}" if first.get("case") else ""))
        out += ["  " + line for line in first.get("report", "").splitlines()]
    return "\n".join(out) + "\n"
