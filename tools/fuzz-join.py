#!/usr/bin/env python3
"""Joins the parts of a fuzz run split into partitions into one record
and one corpus (fuzz/README.md, "Partitions").

  python3 -I -B tools/fuzz-join.py OUT PART...      (`just fuzz-join OUT PART...`)

Each PART is a folder holding one part's `fuzz-release.txt`, written by
`tools/fuzz-release.sh` with `FARADAY_FUZZ_PARTITION` naming its
partitions, and, after `tools/fuzz-corpus-export.sh` there, its
`fuzz-corpus/` (the `out/` where the part ran). OUT gets `fuzz-release.txt` in the
form a whole run writes, each target once, and `fuzz-corpus/` with every
target's folder, in place of what OUT held.

The parts must be one run: the same commit, nightly, cargo-fuzz, starting
corpus and seconds a target, one count of partitions, each partition in
one part, and each corpus holding its own part's targets. Otherwise
nothing is written and it exits 2. A crash in any part is in the record,
which is written, and the exit is 1, as for a whole run. Reads only the
files it is given; the standard library only.
"""
import pathlib
import re
import shutil
import sys

SAME = ("Commit", "Toolchain", "cargo-fuzz", "Starting corpus", "Time per target")
TITLE = "Fuzz run before a release"
PARTITION = re.compile(r"^(\d+(?:,\d+)*) of (\d+), (\d+) of (\d+) targets$")


def fail(msg):
    print(f"fuzz-join: {msg}", file=sys.stderr)
    sys.exit(2)


def read(part):
    """One part's record: its header fields, its partitions, their count,
    the whole run's targets, and its rows by target. Reads the file."""
    path = part / "fuzz-release.txt"
    try:
        lines = path.read_text().splitlines()
    except OSError as e:
        fail(f"{path}: {e.strerror}")
    if not lines or lines[0] != TITLE:
        fail(f"{path} is not a fuzz-release record")
    head, i = {}, 1
    while i < len(lines) and lines[i]:
        key, _, value = lines[i].partition(": ")
        head[key] = value
        i += 1
    if "Partition" not in head:
        fail(f"{path} is a whole run's record, not a part's (no Partition line)")
    m = PARTITION.match(head["Partition"])
    if not m:
        fail(f"{path}: Partition is not 'M of N, K of T targets'")
    taken = [int(x) for x in m.group(1).split(",")]
    count, ran, total = int(m.group(2)), int(m.group(3)), int(m.group(4))
    rows = {}
    for line in lines[i + 2:]:
        if not line:
            break
        name = line.split(maxsplit=1)[0]
        rows[name] = line
    if len(rows) != ran:
        fail(f"{path} says it ran {ran} targets and lists {len(rows)}")
    return head, taken, count, total, rows


def main(argv):
    if len(argv) < 2:
        print(__doc__.split("\n\n")[1], file=sys.stderr)
        return 2
    out, parts = pathlib.Path(argv[0]), [pathlib.Path(p) for p in argv[1:]]
    records = [read(p) for p in parts]
    first, _, count, total, _ = records[0]
    rows, seen = {}, set()
    for part, (head, taken, c, n, part_rows) in zip(parts, records):
        for key in SAME:
            if head.get(key) != first.get(key):
                fail(f"{part}'s {key} is {head.get(key)!r}, the first part's {first.get(key)!r}")
        if c != count or n != total:
            fail(f"{part} is of {c} partitions and {n} targets, the first part of {count} and {total}")
        again = seen & set(taken)
        if again:
            fail(f"partition {', '.join(map(str, sorted(again)))} is in more than one part")
        seen |= set(taken)
        both = rows.keys() & part_rows.keys()
        if both:
            fail(f"{', '.join(sorted(both))} ran in more than one part")
        rows.update(part_rows)
    missing = set(range(1, count + 1)) - seen
    if missing:
        fail(f"no part ran partition {', '.join(map(str, sorted(missing)))} of {count}")
    if len(rows) != total:
        fail(f"the parts ran {len(rows)} of the {total} targets")

    corpora = [p / "fuzz-corpus" for p in parts]
    have = [c.is_dir() for c in corpora]
    if any(have) and not all(have):
        fail("some parts have a fuzz-corpus folder and some do not")
    for corpus, (_, _, _, _, part_rows) in zip(corpora if all(have) else [], records):
        got = {d.name for d in corpus.iterdir() if d.is_dir()}
        if got != part_rows.keys():
            fail(f"{corpus} holds other targets than its record: "
                 f"{', '.join(sorted(got ^ part_rows.keys()))}")

    crashed = sum("CRASH" in line for line in rows.values())
    text = [TITLE, f"Date: {max(r[0]['Date'] for r in records)}"]
    text += [f"{key}: {first[key]}" for key in SAME]
    text += ["", f"{'target':<16} {'seconds':>8} {'executions':>12} {'corpus':>9} {'bytes':>12}  result"]
    text += [rows[name] for name in sorted(rows)]
    text += ["", f"Result: {crashed} target(s) crashed" if crashed else f"Result: {total} targets, no crashes"]
    out.mkdir(parents=True, exist_ok=True)
    (out / "fuzz-release.txt").write_text("\n".join(text) + "\n")
    if all(have):
        joined, new = out / "fuzz-corpus", out / "fuzz-corpus.joining"
        shutil.rmtree(new, ignore_errors=True)
        new.mkdir()
        for corpus in corpora:
            for target in corpus.iterdir():
                if target.is_dir():
                    shutil.copytree(target, new / target.name)
        shutil.rmtree(joined, ignore_errors=True)
        new.rename(joined)
    print((out / "fuzz-release.txt").read_text(), end="")
    return 1 if crashed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
