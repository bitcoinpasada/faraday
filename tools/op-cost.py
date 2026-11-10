#!/usr/bin/env python3
"""Named costly operations held by an instruction count (F238).

    op-cost.py            build tools/op-cost, count each operation, compare
    op-cost.py --write    write tools/op-cost.counts from this run

`cargo build --locked --release -p op-cost`, then each operation `op-cost
--list` names runs once under `valgrind --tool=callgrind` and the
"Collected" instruction count is read from its summary. The counts are
held to `tools/op-cost.counts`: an operation more than 25% over its count
fails, and so does an operation with no count and a count with no
operation. A fall is not a failure; it says to lower the count with
--write. A count is raised only in a commit that says why. Needs valgrind.
Standard library only.
"""

import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
COUNTS = ROOT / "tools/op-cost.counts"
BIN = ROOT / "target/release/op-cost"
LIMIT = 1.25
HEADER = """\
# Held by tools/op-cost.py (F238). Instructions one run of each named costly
# operation takes under valgrind's callgrind, from `tools/op-cost`. An
# operation more than 25% over its count fails the check; a count is raised
# only in a commit that says why. A fall is lowered with
# `python3 -I -B tools/op-cost.py --write`.
"""
COLLECTED = re.compile(r"Collected\s*:\s*(\d+)")


def read_counts(text):
    counts = {}
    for line in text.splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            name, n = line.split()
            counts[name] = int(n)
    return counts


def collected(stderr):
    """The instruction count of callgrind's summary."""
    m = COLLECTED.search(stderr)
    if not m:
        raise ValueError("no Collected line in valgrind's output:\n" + stderr[-600:])
    return int(m.group(1))


def judge(counts, measured):
    """(failures, falls) of a run against the committed counts."""
    failures, falls = [], []
    for name in sorted(set(counts) | set(measured)):
        if name not in measured:
            failures.append(f"{name}: a count with no operation")
        elif name not in counts:
            failures.append(f"{name}: an operation with no count ({measured[name]})")
        elif measured[name] > counts[name] * LIMIT:
            over = (measured[name] / counts[name] - 1) * 100
            failures.append(
                f"{name}: {measured[name]} instructions, {over:.0f}% over its count {counts[name]}"
            )
        elif measured[name] < counts[name] * 0.9:
            falls.append(f"{name}: {measured[name]}, under its count {counts[name]}")
    return failures, falls


def self_test():
    assert collected("==1== Collected : 1234\n") == 1234
    assert judge({"a": 100}, {"a": 125}) == ([], [])
    assert len(judge({"a": 100}, {"a": 126})[0]) == 1
    assert len(judge({"a": 100}, {})[0]) == 1
    assert len(judge({}, {"a": 1})[0]) == 1
    assert judge({"a": 100}, {"a": 50}) == ([], ["a: 50, under its count 100"])


def run(cmd):
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)


def main():
    self_test()
    r = subprocess.run(
        ["cargo", "build", "--locked", "--release", "-p", "op-cost", "-q"], cwd=ROOT
    )
    if r.returncode:
        return 1
    names = run([str(BIN), "--list"]).stdout.split()
    measured = {}
    for name in names:
        r = run(
            [
                "valgrind",
                "--tool=callgrind",
                "--callgrind-out-file=/dev/null",
                str(BIN),
                name,
            ]
        )
        if r.returncode:
            print(f"op-cost: {name} failed under valgrind:\n{r.stderr[-800:]}")
            return 1
        measured[name] = collected(r.stderr)
        print(f"{name:24} {measured[name]:>14}")
    if "--write" in sys.argv[1:]:
        COUNTS.write_text(HEADER + "".join(f"{n} {c}\n" for n, c in measured.items()))
        print(f"op-cost: wrote {COUNTS.relative_to(ROOT)}")
        return 0
    counts = read_counts(COUNTS.read_text()) if COUNTS.exists() else {}
    failures, falls = judge(counts, measured)
    for line in falls:
        print(f"op-cost: {line}; lower it with --write")
    for line in failures:
        print(f"op-cost: {line}")
    if failures:
        print("op-cost: raise a count only with `--write` in a commit that says why (F238)")
        return 1
    print("op-cost: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
