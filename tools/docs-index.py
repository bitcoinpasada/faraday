#!/usr/bin/env python3
"""The "Every document" list of docs/README.md, made from each document's header.

    docs-index.py write     rewrite the list between the markers
    docs-index.py check     fail when the list differs from what write makes
    docs-index.py --self-test

One line per `docs/*.md` (not the subfolders), grouped by the document's
`**Kind:**` (Reference, Spec, Record) and sorted by path: the path, the title
(the first heading), and the first sentence of the `**Status:**` line. The
list sits between `<!-- docs-index: begin -->` and `<!-- docs-index: end -->`
in docs/README.md and is never edited by hand: change a document's header.
"""
import glob
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
README = os.path.join(ROOT, "docs", "README.md")
BEGIN = "<!-- docs-index: begin -->"
END = "<!-- docs-index: end -->"
KINDS = ("Reference", "Spec", "Record")


def first_sentence(text):
    m = re.search(r"\.(\s|$)", text)
    return (text[:m.start()] if m else text).strip()


def entry(path, text):
    """(kind, line) of a document's text, or an error string."""
    lines = text.split("\n")
    head = lines[:12]
    title = next((l[2:].strip() for l in lines if l.startswith("# ")), None)
    kind = next((l[len("**Kind:**"):].strip() for l in head
                 if l.startswith("**Kind:**")), None)
    status = next((l[len("**Status:**"):].strip() for l in head
                   if l.startswith("**Status:**")), None)
    if not (title and kind in KINDS and status):
        return f"{path}: no title, Kind or Status in its header"
    return kind, f"- `{path}`: {title}. {first_sentence(status)}."


def build(root=ROOT):
    groups = {k: [] for k in KINDS}
    for f in sorted(glob.glob(os.path.join(root, "docs", "*.md"))):
        rel = "docs/" + os.path.basename(f)
        e = entry(rel, open(f, encoding="utf-8").read())
        if isinstance(e, str):
            sys.exit(f"docs-index: {e}")
        groups[e[0]].append(e[1])
    out = []
    for k in KINDS:
        out += [f"**{k}**", ""] + groups[k] + [""]
    return "\n".join(out).rstrip("\n")


def splice(readme, body):
    i, j = readme.find(BEGIN), readme.find(END)
    if i < 0 or j < i:
        sys.exit(f"docs-index: {README} lacks the marker lines")
    return readme[:i + len(BEGIN)] + "\n\n" + body + "\n\n" + readme[j:]


def self_test():
    assert first_sentence("Built (F1). More.") == "Built (F1)"
    assert first_sentence("Built; §10.5 open") == "Built; §10.5 open"
    e = entry("docs/X.md", "# T\n\n**Kind:** Spec\n**Status:** Partly built. Rest.\n")
    assert e == ("Spec", "- `docs/X.md`: T. Partly built."), e
    assert isinstance(entry("docs/Y.md", "# T\n"), str)
    s = splice(f"a\n{BEGIN}\nold\n{END}\nb\n", "new")
    assert s == f"a\n{BEGIN}\n\nnew\n\n{END}\nb\n", s
    print("docs-index: self-test ok")


def main(argv):
    if argv == ["--self-test"]:
        return self_test()
    if argv not in (["write"], ["check"]):
        sys.exit(__doc__)
    old = open(README, encoding="utf-8").read()
    new = splice(old, build())
    if argv == ["write"]:
        open(README, "w", encoding="utf-8").write(new)
    elif new != old:
        sys.exit("docs-index: the list in docs/README.md differs from the "
                 "documents' headers; run `python3 -I -B tools/docs-index.py write`")


main(sys.argv[1:])
