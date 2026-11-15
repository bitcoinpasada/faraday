#!/usr/bin/env python3
"""Every Check row's flow has a Tools tile (docs/WALLETS.md, 7. Tools).

Tools (faraday-core/src/catalog.rs) is the master list of every feature; the
Check place (faraday-core/src/check/mod.rs) is a second way in. Fails when a
`Row` variant of the Check place is not opened by a `Row::X =>` arm, or when
a flow a row or a backup form opens (`Go::Y`) has no `go: Go::Y` tile in
catalog.rs. Run from anywhere; `just lint` does. The standard library only.
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CORE = ROOT / "faraday" / "faraday-core" / "src"


def main():
    check = (CORE / "check" / "mod.rs").read_text(encoding="utf-8")
    catalog = (CORE / "catalog.rs").read_text(encoding="utf-8")
    enum = re.search(r"pub enum Row \{(.*?)\n\}", check, re.S)
    rows = re.findall(r"^    ([A-Z]\w*),", enum.group(1), re.M) if enum else []
    tiles = set(re.findall(r"go: (Go::\w+(?:\([^)]*\))?)", catalog))
    bad = []
    if not rows:
        bad.append("check/mod.rs: the `Row` enum is not found")
    handled = check.split("fn press", 1)[-1] if "fn press" in check else check
    for r in rows:
        if f"Row::{r}" not in handled:
            bad.append(f"check/mod.rs: row `{r}` is not opened by any arm")
    for go in sorted(set(re.findall(r"=> (Go::\w+(?:\([^)]*\))?)", handled))):
        if go not in tiles:
            bad.append(f"check/mod.rs: a row opens `{go}`, which has no tile in catalog.rs")
    for b in bad:
        print(b)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
