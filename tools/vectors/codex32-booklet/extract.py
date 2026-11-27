"""Reads the codex32 booklet's tables out of `tables.txt` into JSON.

    cd tools/vectors/codex32-booklet && python3 -I extract.py > tables.json

`tables.txt` is the table lines of the booklet's PDF as `pdftotext
-layout` writes them (README.md says which pages and how). This script
checks every row's length against its header and writes:

- `addition`, `translation`, `recovery`, `fusion`: the principal tables
  (page 4), each as its column heads and one string a row;
- `derivation`: every derivation table (pages 16, 48 and 49), its
  threshold, its initial shares (the row labels) and one string a row;
- `checksum`: the checksum table (pages 22 and 23), the 13 symbols each
  leading pair selects.

pdftotext writes the Symbol font's Delta and mu as U+2206 and U+00B5;
they are written here as the Greek letters U+0394 and U+03BC, which is
what the booklet's list of symbols names them.
"""

import json
import re
import sys

GLYPHS = {"∆": "Δ", "µ": "μ"}


def norm(s):
    return "".join(GLYPHS.get(c, c) for c in s)


def pages(text):
    out = {}
    for chunk in text.split("=== page ")[1:]:
        num, _, body = chunk.partition(" ===\n")
        out[int(num)] = body.splitlines()
    return out


def principal(lines):
    """The two pairs of side-by-side tables on page 4."""
    tables = {}
    names = []
    heads = None
    for line in lines:
        words = line.split()
        if not words:
            continue
        if words in (["Addition", "Translation"], ["Recovery", "Fusion"]):
            names = [w.lower() for w in words]
            heads = None
            continue
        if heads is None:
            heads = [norm(w) for w in words]
            for name, head in zip(names, heads):
                tables[name] = {"columns": head, "rows": {}}
            continue
        for name, word in zip(names, words):
            word = norm(word)
            table = tables[name]
            label, row = word[0], word[1:]
            if len(row) != len(table["columns"]):
                raise SystemExit(f"{name} row {label}: {len(row)} cells")
            table["rows"][label] = row
    return tables


def derivation(lines, page):
    out = []
    table = None
    for line in lines:
        s = line.strip()
        if s.startswith("k = "):
            k, _, cols = s[4:].partition(" ")
            table = {"page": page, "k": int(k), "columns": "".join(cols.split()), "rows": {}}
            out.append(table)
            continue
        if not s or table is None:
            continue
        label, cells = s[0], norm("".join(s[1:].split()))
        if len(cells) != len(table["columns"]):
            raise SystemExit(f"page {page} k = {table['k']} row {label}: {len(cells)} cells")
        table["rows"][label] = cells
    for t in out:
        t["initial"] = "".join(t["rows"])
        if len(t["initial"]) != t["k"]:
            raise SystemExit(f"page {page} k = {t['k']}: {len(t['initial'])} rows")
    return out


def checksum(lines):
    table = {}
    for line in lines:
        for pair, row in re.findall(r"\b([A-Z0-9]{2}) ([A-Z0-9]{13})\b", line):
            if pair in table:
                raise SystemExit(f"checksum table: {pair} twice")
            table[pair] = row
    if len(table) != 1024:
        raise SystemExit(f"checksum table: {len(table)} rows")
    return dict(sorted(table.items()))


def main():
    text = open("tables.txt", encoding="utf-8").read()
    p = pages(text)
    out = {
        "source": "The codex32 booklet, black and white, 2023-03-07 (README.md)",
        **principal(p[4]),
        "derivation": derivation(p[16], 16) + derivation(p[48], 48) + derivation(p[49], 49),
        "checksum": checksum(p[22] + p[23]),
    }
    json.dump(out, sys.stdout, ensure_ascii=False, indent=1)
    sys.stdout.write("\n")


main()
