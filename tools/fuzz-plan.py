#!/usr/bin/env python3
"""The coverage map of a fuzz run and the targets a later run runs (F241).

  python3 -I -B tools/fuzz-plan.py map WORK OUT
  python3 -I -B tools/fuzz-plan.py plan EARLIER [TARGETS]

`map` is the last step of a full run (`tools/fuzz-release.sh`). WORK holds,
for each target, `NAME.json` (`llvm-cov export -summary-only` of the
target's instrumented binary over the corpus the run left) and `NAME.d`
(cargo's dep-info for the binary: the files compiled into it), and
`header` (its `Commit:` and `Toolchain:` lines). OUT is the map: its
commit, and for each target the repository's files in which the corpus
executes code (`reach`), the crates it compiles in (`crate`), and the
files compiled in that hold no code of their own or are not Rust (`dep`);
`code` lists the Rust files with code in any target.

`plan` reads the folder EARLIER (the earlier run's `fuzz-release.txt` and
`fuzz-map.txt`) and TARGETS (the lines "DIR TARGET" that
`tools/fuzz-common.sh` lists; the earlier run's targets when left out), and prints the targets the checkout at
HEAD must run again, one a line, with each reason on stderr: the rules of
`docs/decisions/F241.md`. It exits 2, printing nothing, when the earlier
run is not one full run of the commit its map names. Reads the files it is
given and runs `git` in the checkout; the standard library only.
"""
import json
import os
import pathlib
import re
import subprocess
import sys

MAP_TITLE = "Fuzz coverage map"
RECORD_TITLE = "Fuzz run before a release"
# A change to these reruns every target.
WHOLE = ("fuzz/Cargo.toml", "fuzz/Cargo.lock", "fuzz/rust-toolchain.toml",
         "faraday/fuzz/Cargo.toml", "faraday/fuzz/Cargo.lock", "faraday/fuzz/rust-toolchain.toml")
WHOLE_PREFIX = ("tools/fuzz-",)
CONSTANT = re.compile(r"\b(?:const|static)\s+(?:mut\s+)?([A-Za-z_][A-Za-z0-9_]*)")
# An entry of a table of text (`name: "wording",`, the strings crate's rows).
ENTRY = re.compile(r'^\s*([a-z_][a-z0-9_]*)\s*:\s*r?#*"')


def fail(msg):
    print(f"fuzz-plan: {msg}", file=sys.stderr)
    sys.exit(2)


def git(*args):
    done = subprocess.run(["git", *args], capture_output=True, text=True)
    if done.returncode not in (0, 1):
        fail(f"git {' '.join(args[:2])} failed: {done.stderr.strip()}")
    return done.stdout


def relative(name, root):
    """A path of the repository, or None for one outside it."""
    if name.startswith(root):
        return os.path.normpath(name[len(root):])
    if name.startswith("/"):
        return None
    return os.path.normpath(name)


def crate_dir(path, root):
    d = os.path.dirname(path)
    while d and not os.path.exists(os.path.join(root, d, "Cargo.toml")):
        d = os.path.dirname(d)
    return d


def make_map(work, out):
    work = pathlib.Path(work)
    root = git("rev-parse", "--show-toplevel").strip() + "/"
    header = (work / "header").read_text().splitlines()
    reach, code, dep, crates, targets = [], set(), [], [], []
    for j in sorted(work.glob("*.json")):
        name = j.stem
        targets.append(name)
        covered = set()
        for f in json.loads(j.read_text())["data"][0]["files"]:
            fn = relative(f["filename"], root)
            if fn is None or fn.startswith("fuzz_targets/") or "/fuzz_targets/" in fn:
                continue
            if f["summary"]["regions"]["count"] > 0:
                code.add(fn)
            if f["summary"]["regions"]["covered"] > 0:
                covered.add(fn)
        reach += [(name, fn) for fn in sorted(covered)]
        text = (work / f"{name}.d").read_text().replace("\\\n", " ")
        for fn in text.split("\n")[0].split(": ", 1)[1].split():
            fn = relative(fn, root)
            if fn is None:
                continue
            dep.append((name, fn))
    # `dep`: compiled in with no code of its own (not Rust, or no region).
    lines = [MAP_TITLE, *header, ""]
    lines += [f"target\t{t}" for t in targets]
    lines += [f"code\t{f}" for f in sorted(code)]
    lines += [f"reach\t{t}\t{f}" for t, f in reach]
    for t, f in dep:
        if not (f.endswith(".rs") and f in code):
            lines.append(f"dep\t{t}\t{f}")
    for t, d in sorted({(t, crate_dir(f, root)) for t, f in dep}):
        lines.append(f"crate\t{t}\t{d}")
    pathlib.Path(out).write_text("\n".join(lines) + "\n")


def read_header(lines, title, path):
    if not lines or lines[0] != title:
        fail(f"{path} is not a {title.lower()}")
    head, i = {}, 1
    while i < len(lines) and lines[i]:
        key, _, value = lines[i].partition(": ")
        head[key] = value
        i += 1
    return head, i


def plan(earlier, targets_file):
    earlier = pathlib.Path(earlier)
    try:
        rec_lines = (earlier / "fuzz-release.txt").read_text().splitlines()
        map_lines = (earlier / "fuzz-map.txt").read_text().splitlines()
    except OSError as e:
        fail(f"{e.filename}: {e.strerror}")
    rec, i = read_header(rec_lines, RECORD_TITLE, earlier / "fuzz-release.txt")
    for key in ("Partition", "Selected", "Later run"):
        if key in rec:
            fail(f"the earlier run is not one full run (its record has a {key} line): a third run is made against a full run")
    mp, _ = read_header(map_lines, MAP_TITLE, earlier / "fuzz-map.txt")
    base = mp.get("Commit", "")
    if not base or not base.startswith(rec.get("Commit", "-")):
        fail(f"the map is of commit {base or '(none)'}, the record of {rec.get('Commit')}")
    if git("cat-file", "-t", base).strip() != "commit":
        fail(f"the earlier run's commit {base} is not in this checkout")
    passed = {}
    for line in rec_lines[i + 2:]:
        if not line:
            break
        parts = line.split(maxsplit=5)
        passed[parts[0]] = parts[5].startswith("no crashes") if len(parts) == 6 else False

    known, reach, code, dep, crates = set(), {}, set(), {}, {}
    for line in map_lines:
        kind, _, rest = line.partition("\t")
        fields = rest.split("\t")
        if kind == "target":
            known.add(fields[0])
        elif kind == "code":
            code.add(fields[0])
        elif kind == "reach":
            reach.setdefault(fields[1], set()).add(fields[0])
        elif kind == "dep":
            dep.setdefault(fields[1], set()).add(fields[0])
        elif kind == "crate":
            crates.setdefault(fields[1], set()).add(fields[0])
    if targets_file:
        current = [line.split()[1] for line in open(targets_file) if line.strip()]
    else:
        current = sorted(known)
    why = {}

    def add(names, reason):
        for n in names:
            if n in current:
                why.setdefault(n, reason)

    for n in current:
        if n not in known:
            add([n], "a target the earlier run did not have")
        elif not passed.get(n, False):
            add([n], "it did not pass in the earlier run")

    changed = [f for f in git("diff", "--name-only", base, "HEAD").splitlines() if f]
    named = {}
    for f in changed:
        if f.endswith(".rs"):
            diff = git("diff", "-U0", base, "HEAD", "--", f)
            body = [l[1:] for l in diff.splitlines()
                    if l[:1] in "+-" and not l.startswith(("+++", "---"))]
            names = {m for l in body for m in CONSTANT.findall(l)}
            if f not in code:
                names |= {m.group(1) for l in body if (m := ENTRY.match(l))}
            for n in names:
                for user in git("grep", "-l", "-w", "-F", "-e", n, "HEAD", "--", "*.rs").splitlines():
                    user = user.split(":", 1)[-1]
                    if user != f:
                        named.setdefault(user, f"{n} in {f} changed")
            if names:
                named.setdefault(f, "")
    for f in changed:
        if f in WHOLE or f.startswith(WHOLE_PREFIX):
            add(current, f"{f} changed")
            continue
        own = re.match(r"(?:faraday/)?fuzz/(?:fuzz_targets|corpus|dict)/([^/.]+)", f)
        if own:
            add([own.group(1)], f"{f} changed")
            continue
        if f.endswith("/Cargo.toml"):
            add(crates.get(os.path.dirname(f), ()), f"{f} changed")
            continue
        if f in reach:
            add(reach[f], f"its corpus reaches {f}, which changed")
        elif f not in code and f in dep and f not in named:
            add(dep[f], f"{f} is compiled in and its change cannot be followed")
    for f, reason in named.items():
        if f in reach and reason:
            add(reach[f], f"its corpus reaches {f}, which names what changed ({reason})")
    for n in sorted(why):
        print(f"fuzz-plan: {n}: {why[n]}", file=sys.stderr)
    print("\n".join(sorted(why)))


def main(argv):
    if len(argv) == 3 and argv[0] == "map":
        make_map(argv[1], argv[2])
    elif len(argv) in (2, 3) and argv[0] == "plan":
        plan(argv[1], argv[2] if len(argv) == 3 else None)
    else:
        print(__doc__.split("\n\n")[1], file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
