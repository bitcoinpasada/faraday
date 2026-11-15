#!/usr/bin/env python3
"""The table of docs/DECISIONS.md, made from each decision's header.

    decisions-index.py write     rewrite the table's rows
    decisions-index.py check     fail naming each row that differs
    decisions-index.py --self-test

One row per `docs/decisions/F<n>.md` and `docs/history/decisions/F<n>.md`,
in number order: the number linking the file, the title (the first line
minus "F<n> " and the date in brackets), the date, the `**Status:**` line
and the `**Area:**` line. The text above the table is not touched. The
rows are never edited by hand: change a decision's header.
"""
import glob
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
INDEX = os.path.join(ROOT, "docs", "DECISIONS.md")
TOP = "| F# | Title | Date | Status | Area |"
SEP = "|---|---|---|---|---|"


def row(rel, text):
    """(number, row) of a decision file's text, or an error string."""
    lines = text.split("\n")
    m = re.match(r"# F(\d+) (.+) \((?:owner, )?(\d{4}-\d\d-\d\d)\)$", lines[0])
    if not m:
        return f"{rel}: first line is not `# F<n> <title> (<date>)`"
    head = lines[:12]

    def field(name):
        return next((l[len(name):].strip() for l in head
                     if l.startswith(name)), None)
    status, area = field("**Status:**"), field("**Area:**")
    if not (status and area):
        return f"{rel}: no Status or Area in its header"
    link = rel[len("docs/"):]
    return int(m.group(1)), (f"| [F{m.group(1)}]({link}) | {m.group(2)} | "
                             f"{m.group(3)} | {status} | {area} |")


def build(root=ROOT):
    rows = {}
    for sub in ("decisions", "history/decisions"):
        for f in glob.glob(os.path.join(root, "docs", sub, "F*.md")):
            rel = f"docs/{sub}/" + os.path.basename(f)
            r = row(rel, open(f, encoding="utf-8").read())
            if isinstance(r, str):
                sys.exit(f"decisions-index: {r}")
            if r[0] in rows:
                sys.exit(f"decisions-index: F{r[0]} has two files")
            rows[r[0]] = r[1]
    return [rows[n] for n in sorted(rows)]


def split(text):
    """(above, rows, below) around the table."""
    lines = text.split("\n")
    if TOP not in lines:
        sys.exit(f"decisions-index: {INDEX} lacks the table header")
    i = lines.index(TOP) + 2
    j = i
    while j < len(lines) and lines[j].startswith("|"):
        j += 1
    return lines[:i], lines[i:j], lines[j:]


def self_test():
    r = row("docs/decisions/F1.md",
            "# F1 A, b (2026-10-08)\n\n**Status:** Active\n**Area:** x\n")
    assert r == (1, "| [F1](decisions/F1.md) | A, b | 2026-10-08 | Active | x |"), r
    assert isinstance(row("docs/decisions/F2.md", "# F2 T\n"), str)
    a, rows, b = split(f"x\n{TOP}\n{SEP}\n| r |\ny\n")
    assert (a, rows, b) == (["x", TOP, SEP], ["| r |"], ["y", ""]), (a, rows, b)
    print("decisions-index: self-test ok")


def key(line):
    return int(re.match(r"\| \[F(\d+)\]", line).group(1))


def main(argv):
    if argv == ["--self-test"]:
        return self_test()
    if argv not in (["write"], ["check"]):
        sys.exit(__doc__)
    text = open(INDEX, encoding="utf-8").read()
    above, old, below = split(text)
    new = build()
    if argv == ["write"]:
        open(INDEX, "w", encoding="utf-8").write("\n".join(above + new + below))
        return
    have = {key(l): l for l in old if l.startswith("| [F")}
    want = {key(l): l for l in new}
    bad = [f"  F{k}: row is `{have.get(k)}`, header makes `{want.get(k)}`"
           for k in sorted(set(have) | set(want)) if have.get(k) != want.get(k)]
    if bad:
        sys.exit("decisions-index: rows differ from the decisions' headers; "
                 "run `python3 -I -B tools/decisions-index.py write`\n"
                 + "\n".join(bad))


main(sys.argv[1:])
