#!/usr/bin/env python3
"""migration: the mapping of `main` onto `next` (docs/spec/10-migration.md §1, §2.1, §4, §5.3).

`main` is read only at the tag `history/main-2026-11`, which must peel to `d89dde5de` (MG1). The
mapping is `docs/migration/mapping.toml`: rows of `[[module]]` with `main`, `fate`, `next`,
`feature`, `contract`, `changed` and `why`. Commands:

  check    MG4: every path of the tag falls under a row (the most specific one names it); no row
           names a path absent at the tag; every fate is one of four; a port or rewrite row names
           an inventory slug that exists, or (a rewrite) a contract; a drop row has a `why`; a copy
           row has one `next` path. Every failure is reported in one run; exit 1 on any.
  done     MG5: for each inventory file and each crate, the rows done and the `todo!()` left in
           their `next` paths. A row is done when its `next` paths exist and hold no `todo!()`;
           for a copy of a directory, every file of `main` it covers is also there, at its moved
           path. A drop row is done once written. Nothing is written back.
  diff P   MG3: the difference between the file (or each file under the directory) P of `main`
           and its place on `next`, each moved directory prefix folded so a file only moved shows
           none.
  parity   MG8: the rows of `docs/migration/inventory/fixes.md`, of a feature whose crates hold no
           `todo!()`, that nothing accounts for (a test whose Regression line ends `(main <hash>)`,
           or an entry of `docs/migration/parity.toml`: `[[fix]] commit = "<hash>"`, `held =
           "compiler: ..." | "lint: ..." | "gone: ..." | "device: ..."`), and every entry naming a
           commit not in `fixes.md`.

Standard library only: run as `python3 -I -B tools/migration.py <command>`. `--root DIR` reads
another checkout (the tests', or a scratch clone that carries a tag of its own).
"""

import argparse
import difflib
import re
import subprocess
import sys
import tomllib
from collections import Counter
from pathlib import Path

sys.dont_write_bytecode = True

TAG = "history/main-2026-11"
FROZEN = "d89dde5de"
FATES = ("copy", "port", "rewrite", "drop")
KEYS = {"main", "fate", "next", "feature", "contract", "changed", "why"}
HELD = ("compiler:", "lint:", "gone:", "device:")
MAPPING = "docs/migration/mapping.toml"
PARITY = "docs/migration/parity.toml"
FEATURES = "docs/migration/inventory/features"
FIXES = "docs/migration/inventory/fixes.md"
TODO = re.compile(r"\btodo!\s*\(")


class Stop(Exception):
    """A failure that ends a command: the message is what is printed."""


def git(root, *args, text=True):
    p = subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=text)
    if p.returncode:
        err = p.stderr if text else p.stderr.decode(errors="replace")
        raise Stop(f"git {' '.join(args)}: {err.strip()}")
    return p.stdout


def frozen_commit(root):
    """MG1: the commit the tag peels to, or a Stop saying the tag is missing or wrong."""
    p = subprocess.run(["git", "-C", str(root), "rev-parse", "--verify", "-q", f"refs/tags/{TAG}^{{commit}}"],
                       capture_output=True, text=True)
    if p.returncode:
        raise Stop(f"MG1: the tag {TAG} is missing from this clone (fetch tags: `git fetch --tags`); "
                   f"main is read only at {FROZEN} through it")
    sha = p.stdout.strip()
    if not sha.startswith(FROZEN):
        raise Stop(f"MG1: the tag {TAG} peels to {sha[:9]}, not to {FROZEN}")
    return sha


def main_files(root):
    frozen_commit(root)
    out = git(root, "ls-tree", "-r", "-z", "--name-only", f"refs/tags/{TAG}")
    return sorted(p for p in out.split("\0") if p)


def main_blob(root, path):
    frozen_commit(root)
    return git(root, "show", f"refs/tags/{TAG}:{path}", text=False)


def norm(p):
    return p.strip().rstrip("/")


class Row:
    def __init__(self, i, d):
        self.i = i
        self.main = norm(str(d.get("main", "")))
        self.fate = d.get("fate", "")
        self.next = d.get("next", [])
        self.feature = d.get("feature", [])
        self.contract = d.get("contract", "")
        self.changed = d.get("changed", [])
        self.why = d.get("why", "")
        self.raw = d

    def name(self):
        return f"row {self.i} ({self.main or '?'})"


def load_rows(root, fails):
    path = Path(root) / MAPPING
    if not path.exists():
        fails.append(f"{MAPPING} does not exist")
        return []
    try:
        data = tomllib.loads(path.read_text())
    except (tomllib.TOMLDecodeError, UnicodeDecodeError) as e:
        fails.append(f"{MAPPING} does not parse: {e}")
        return []
    return [Row(i + 1, d) for i, d in enumerate(data.get("module", []))]


def slugs(root):
    d = Path(root) / FEATURES
    return {p.stem for p in d.glob("*.md")} if d.is_dir() else set()


def row_failures(rows, known):
    """The failures that need no tree: shape, fate, slugs, `why`, duplicates."""
    fails = []
    seen = {}
    nexts = {}
    for r in rows:
        extra = set(r.raw) - KEYS
        if extra:
            fails.append(f"{r.name()}: unknown key {', '.join(sorted(extra))}")
        if not r.main:
            fails.append(f"{r.name()}: no `main` path")
        elif r.main in seen:
            fails.append(f"{r.name()}: `main` {r.main} is also row {seen[r.main]}")
        else:
            seen[r.main] = r.i
        for key, kind in (("next", list), ("feature", list), ("changed", list), ("contract", str), ("why", str)):
            if not isinstance(getattr(r, key), kind):
                fails.append(f"{r.name()}: `{key}` is not a {kind.__name__}")
                setattr(r, key, kind())
        if r.fate not in FATES:
            fails.append(f"{r.name()}: fate {r.fate!r} is not one of {', '.join(FATES)}")
            continue
        for p in r.next:
            if not isinstance(p, str) or not p or p.startswith("/") or ".." in Path(p).parts:
                fails.append(f"{r.name()}: `next` path {p!r} is not a path inside the repository")
            else:
                nexts.setdefault(norm(p), []).append(r.i)
        if r.fate == "copy" and len(r.next) != 1:
            fails.append(f"{r.name()}: a copy row has exactly one `next` path, this has {len(r.next)}")
        if r.fate == "drop":
            if r.next:
                fails.append(f"{r.name()}: a drop row has no `next` path")
            if not r.why.strip():
                fails.append(f"{r.name()}: a drop row says why in `why`")
        if r.fate in ("port", "rewrite") and not r.next:
            fails.append(f"{r.name()}: a {r.fate} row names its `next` paths")
        bad = [s for s in r.feature if s not in known]
        for s in bad:
            fails.append(f"{r.name()}: inventory slug {s!r} does not exist")
        if r.fate == "port" and not r.feature:
            fails.append(f"{r.name()}: a port row names an inventory slug in `feature`")
        if r.fate == "rewrite" and not r.feature and not r.contract.strip():
            fails.append(f"{r.name()}: a rewrite row names an inventory slug or a `contract`")
    for p, who in sorted(nexts.items()):
        if len(who) > 1 and not all(rows[i - 1].fate in ("port", "rewrite") for i in who):
            fails.append(f"`next` path {p} is named by rows {', '.join(map(str, who))}")
    return fails


def index(rows):
    return {r.main: r for r in rows}


def owner_fast(by_main, path):
    p = path
    while True:
        if p in by_main:
            return by_main[p]
        if "/" not in p:
            return None
        p = p.rsplit("/", 1)[0]


def check(root, out=print):
    fails = []
    rows = load_rows(root, fails)
    known = slugs(root)
    if not known:
        fails.append(f"{FEATURES} holds no inventory file")
    fails += row_failures(rows, known)
    covered = left = 0
    try:
        files = main_files(root)
    except Stop as e:
        fails.append(str(e))
        files = None
    if files is not None:
        fileset = set(files)
        dirs = {f.rsplit("/", 1)[0] for f in files if "/" in f}
        for d in list(dirs):
            while "/" in d:
                d = d.rsplit("/", 1)[0]
                dirs.add(d)
        for r in rows:
            if r.main and r.main not in fileset and r.main not in dirs:
                fails.append(f"{r.name()}: `main` path {r.main} is not in the tree at {TAG}")
        by_main = index(rows)
        uncovered = [f for f in files if owner_fast(by_main, f) is None]
        covered, left = len(files) - len(uncovered), len(uncovered)
        groups = Counter(f.rsplit("/", 1)[0] if "/" in f else "." for f in uncovered)
        for d, n in sorted(groups.items()):
            fails.append(f"no row for {n} path{'s' if n > 1 else ''} under {d}/")
    for f in fails:
        out(f"FAIL {f}")
    total = covered + left if files is not None else "?"
    out(f"migration check: {len(rows)} rows, {covered} of {total} paths of {TAG} covered, "
        f"{left if files is not None else '?'} left, {len(fails)} failure{'s' if len(fails) != 1 else ''}")
    return 1 if fails else 0


def todo_count(root, rel):
    p = Path(root) / rel
    files = [p] if p.is_file() else sorted(p.rglob("*.rs")) if p.is_dir() else []
    n = 0
    for f in files:
        if f.suffix != ".rs":
            continue
        for line in f.read_text(errors="replace").splitlines():
            if not line.lstrip().startswith("//") and TODO.search(line):
                n += 1
    return n


def crate_of(root, rel):
    """The nearest directory at or above the path that holds a Cargo.toml, else its first two parts."""
    p = Path(rel)
    for q in [p, *p.parents]:
        if str(q) != "." and (Path(root) / q / "Cargo.toml").exists():
            return str(q)
    return "/".join(p.parts[:2])


def move_of(row, path):
    """Where a file of `main` under a copy row lands on `next`."""
    dest = norm(row.next[0])
    return dest if path == row.main else dest + path[len(row.main):]


def row_state(root, r, files, by_main):
    """(done, todo!() left) of one row."""
    todo = 0
    for p in r.next:
        todo += todo_count(root, p)
    if r.fate == "drop":
        return True, 0
    if not r.next or any(not (Path(root) / p).exists() for p in r.next):
        return False, todo
    if r.fate == "copy" and files is not None and len(r.next) == 1:
        for f in files:
            if f.startswith(r.main + "/") and owner_fast(by_main, f) is r:
                if not (Path(root) / move_of(r, f)).exists():
                    return False, todo
    return todo == 0, todo


def done(root, out=print):
    fails = []
    rows = load_rows(root, fails)
    if fails:
        for f in fails:
            out(f"FAIL {f}")
        return 1
    try:
        files = main_files(root)
    except Stop as e:
        out(f"FAIL {e}")
        return 1
    by_main = index(rows)
    state = {r.i: row_state(root, r, files, by_main) for r in rows if r.fate in FATES}
    per_slug, per_crate = {}, {}
    for s in sorted(slugs(root)):
        per_slug[s] = [0, 0, 0]
    for r in rows:
        if r.i not in state:
            continue
        ok, todo = state[r.i]
        for s in r.feature:
            a = per_slug.setdefault(s, [0, 0, 0])
            a[0] += 1
            a[1] += ok
            a[2] += todo
        for c in sorted({crate_of(root, p) for p in r.next}):
            a = per_crate.setdefault(c, [0, 0, 0])
            a[0] += 1
            a[1] += ok
            a[2] += sum(todo_count(root, p) for p in r.next if crate_of(root, p) == c)
    for title, table in (("inventory file", per_slug), ("crate", per_crate)):
        out(f"{title:<28} rows done / rows   todo!() left")
        for k, (n, d, t) in sorted(table.items()):
            out(f"{k:<28} {d:>4} / {n:<4}       {t}")
        out("")
    n = len(state)
    d = sum(1 for ok, _ in state.values() if ok)
    out(f"migration done: {d} of {n} rows done, {sum(t for _, t in state.values())} todo!() left in their paths")
    return 0


def fold_pairs(rows):
    pairs = []
    for r in rows:
        if r.fate == "copy" and len(r.next) == 1 and norm(r.next[0]) != r.main:
            pairs.append((r.main + "/", norm(r.next[0]) + "/"))
    return sorted(pairs, key=lambda p: -len(p[0]))


def diff(root, target, out=print):
    fails = []
    rows = load_rows(root, fails)
    if fails:
        for f in fails:
            out(f"FAIL {f}")
        return 1
    try:
        files = main_files(root)
    except Stop as e:
        out(f"FAIL {e}")
        return 1
    target = norm(target)
    under = [f for f in files if f == target or f.startswith(target + "/")]
    if not under:
        out(f"FAIL {target} is not a path of {TAG}")
        return 1
    pairs = fold_pairs(rows)
    by_main = index(rows)
    bad = shown = same = 0
    for f in under:
        r = owner_fast(by_main, f)
        if r is None or r.fate != "copy" or len(r.next) != 1:
            out(f"FAIL {f}: its row is not a copy row; there is nothing to compare")
            bad += 1
            continue
        dest = move_of(r, f)
        p = Path(root) / dest
        if not p.is_file():
            out(f"FAIL {f}: not on next at {dest}")
            bad += 1
            continue
        old, new = main_blob(root, f), p.read_bytes()
        try:
            old_t, new_t = old.decode(), new.decode()
        except UnicodeDecodeError:
            if old != new:
                out(f"binary file {f} differs from {dest}")
                shown += 1
            else:
                same += 1
            continue
        for a, b in pairs:
            old_t = old_t.replace(a, b)
        if old_t == new_t:
            same += 1
            continue
        shown += 1
        out("".join(difflib.unified_diff(old_t.splitlines(True), new_t.splitlines(True),
                                         f"a/{f}", f"b/{dest}")).rstrip("\n"))
    out(f"migration diff: {len(under)} files, {same} the same, {shown} differ, {bad} not comparable")
    return 1 if bad else 0


def fix_rows(root):
    p = Path(root) / FIXES
    if not p.exists():
        return []
    rows = []
    for line in p.read_text().splitlines():
        m = re.match(r"\|\s*`([0-9a-f]{7,40})`\s*\|\s*\[[^\]]*\]\(features/([^)]+)\.md\)\s*\|\s*(.*?)\s*\|", line)
        if m:
            rows.append((m.group(1), m.group(2), m.group(3)))
    return rows


def regression_hashes(root):
    """The hashes that a Regression line of some Rust file ends with, as `(main <hash>)`."""
    found = set()
    pat = re.compile(r"Regression:.*\(main ([0-9a-f]{7,40})\)\s*$")
    for p in Path(root).rglob("*.rs"):
        if any(part in ("target", ".git", "out") for part in p.parts):
            continue
        for line in p.read_text(errors="replace").splitlines():
            m = pat.search(line.lstrip("/! *"))
            if m:
                found.add(m.group(1))
    return found


def same_commit(a, b):
    return a.startswith(b) or b.startswith(a)


def parity(root, out=print):
    fails = []
    rows = load_rows(root, fails)
    if fails:
        for f in fails:
            out(f"FAIL {f}")
        return 1
    fixes = fix_rows(root)
    held = {}
    pp = Path(root) / PARITY
    if pp.exists():
        try:
            data = tomllib.loads(pp.read_text())
        except tomllib.TOMLDecodeError as e:
            out(f"FAIL {PARITY} does not parse: {e}")
            return 1
        for e in data.get("fix", []):
            c, h = str(e.get("commit", "")), str(e.get("held", ""))
            if not h.startswith(HELD):
                fails.append(f"{PARITY}: {c or '?'}: `held` starts with one of {', '.join(HELD)}")
            if not any(same_commit(c, f[0]) for f in fixes):
                fails.append(f"{PARITY}: {c or '?'} is not a commit of {FIXES}")
            held[c] = h
    tests = regression_hashes(root)
    live = set()
    for r in rows:
        crates = {crate_of(root, p) for p in r.next}
        if r.next and all(todo_count(root, c) == 0 and (Path(root) / c).exists() for c in crates):
            live.update(r.feature)
    left = 0
    for c, slug, what in fixes:
        if slug not in live:
            continue
        if any(same_commit(c, h) for h in tests) or any(same_commit(c, h) for h in held):
            continue
        left += 1
        fails.append(f"fix {c} ({slug}) is not accounted for: {what[:80]}")
    for f in fails:
        out(f"FAIL {f}")
    out(f"migration parity: {len(fixes)} fixes, {len(live)} features without todo!(), {left} not accounted for")
    return 1 if fails else 0


def main(argv=None):
    ap = argparse.ArgumentParser(prog="migration.py", description=__doc__.split("\n")[0])
    ap.add_argument("--root", default=str(Path(__file__).resolve().parent.parent), help="the checkout to read")
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("check")
    sub.add_parser("done")
    sub.add_parser("parity")
    d = sub.add_parser("diff")
    d.add_argument("path", help="a path of main, a file or a directory")
    a = ap.parse_args(argv)
    if a.cmd == "check":
        return check(a.root)
    if a.cmd == "done":
        return done(a.root)
    if a.cmd == "parity":
        return parity(a.root)
    return diff(a.root, a.path)


if __name__ == "__main__":
    sys.exit(main())
