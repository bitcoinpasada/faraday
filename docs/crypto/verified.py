#!/usr/bin/env python3
"""Every row of STANDARDS §2 to §5 says what checks it, and every check is
some row's (docs/STANDARDS.md §0, "Checked against an implementation
Faraday did not write"; docs/CRYPTO-LEARN.md §3.1).

    verified.py check     the self-test, then exit 1 on a row with no check, or a check no row names
    verified.py --self-test

Read from the manifest, `docs/crypto/actions.toml`, through `check.py`'s
reader and `gen.py`'s `row_actions`, never from the Markdown `gen.py`
writes. A row's Verified by cell is its actions' `checked` fields; the row
passes when one of its actions names a cross-check area (`crosscheck`), a
vectors folder the cross-check reads (`tools/crosscheck/stamp.py`'s
`VECTORS`) or a test file that exists (`tests`). A row with `no_action`
describes work not built, or work that computes nothing of its own, and
names no check by rule. Every cross-check area (`tools/crosscheck/area_*.py`)
and every folder of `tools/vectors/` is named by a row's actions, or the
folder is in `[[vector_no_action]]`. Every action with an area says what
its cross-check compares Faraday with (F160, F177): `independence`
(`outside`, `outside-and-own` or `own`), with the outside implementation's
`author` for the first two and, for the last two, `ours`, the part
Faraday's authors wrote; an action with a `novel_rank` carries the `label`
of an own specification. So a new area, which some action must name,
declares its class. Standard library only; reads no source file but the manifest
and two directory listings.

Not checked: STANDARDS §9, which is written by hand; whether a named test
tests the row's standard (check.py's truth checks hold the names).
"""

import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(ROOT / "tools" / "crosscheck"))

import check  # noqa: E402  the manifest's reader
import gen  # noqa: E402  which actions a row describes
import stamp  # noqa: E402  the vectors the cross-check reads


def checks_of(data, row):
    """(areas, vectors, tests) the row's actions name."""
    acts = gen.row_actions(data, row)
    areas = {a.get("checked", {}).get("crosscheck") for a in acts} - {None}
    vectors = {v for a in acts for v in check.as_list(a.get("checked", {}).get("vectors"))}
    tests = {t for a in acts for t in check.as_list(a.get("checked", {}).get("tests"))}
    return areas, vectors, tests


def independence_problems(data):
    """An action with an area and no class, or a novel action with no label (F160, F177)."""
    out = []
    for a in data.get("action", []):
        chk, i = a.get("checked", {}), a.get("id")
        if chk.get("crosscheck"):
            ind = chk.get("independence")
            ind = ind if isinstance(ind, dict) else {}
            cls = ind.get("class")
            if cls not in check.INDEPENDENCE:
                out.append(f"action {i}: its cross-check has no `independence` class "
                           f"({', '.join(sorted(check.INDEPENDENCE))})")
            if cls != "own" and not check.str_ok(ind.get("author")):
                out.append(f"action {i}: no `independence.author`, the outside implementation "
                           "its cross-check compares with")
            if cls in ("outside-and-own", "own") and not check.str_ok(ind.get("ours")):
                out.append(f"action {i}: no `independence.ours`, the part Faraday's authors wrote")
        if "novel_rank" in a and a.get("label") not in check.LABELS:
            out.append(f"action {i}: a novel action with no own-specification `label`")
    return out


def problems(root, data, vectors_read):
    root = pathlib.Path(root)
    out = independence_problems(data)
    named_areas, named_vectors = set(), set()
    for row in data.get("row", []):
        rid = row.get("id")
        areas, vectors, tests = checks_of(data, row)
        named_areas |= areas
        named_vectors |= vectors
        if row.get("no_action"):
            continue
        files = {t.split("::")[0] for t in tests}
        if not (areas or vectors & set(vectors_read) or any((root / f).is_file() for f in files)):
            out.append(f"row {rid}: its Verified by names no cross-check area, no vectors folder "
                       "the cross-check reads and no test file that exists")
    present = {p.name[5:-3] for p in (root / "tools" / "crosscheck").glob("area_*.py")}
    for a in sorted(present - named_areas):
        out.append(f"cross-check area {a} is named by no row's actions")
    vdir = root / "tools" / "vectors"
    folders = {p.name for p in vdir.iterdir() if p.is_dir()} if vdir.is_dir() else set()
    exempt = {n.get("folder") for n in data.get("vector_no_action", [])}
    for v in sorted((folders | set(vectors_read)) - named_vectors - exempt):
        out.append(f"tools/vectors/{v} is named by no row's actions and not in [[vector_no_action]]")
    return out


def cmd_check(root):
    c = check.Ctx(root, check.Inventory([]))
    out = problems(root, c.data, stamp.VECTORS)
    for p in out:
        print(f"verified: {p}")
    if out:
        print("verified: name the check in the action's [action.checked] (docs/CRYPTO-LEARN.md §2.2)")
        return 1
    rows = len(c.data.get("row", []))
    print(f"verified: ok ({rows} rows)")
    return 0


def self_test():
    import tempfile
    with tempfile.TemporaryDirectory() as d:
        root = pathlib.Path(d)
        (root / "tools" / "crosscheck").mkdir(parents=True)
        (root / "tools" / "crosscheck" / "area_one.py").write_text("")
        (root / "tools" / "vectors" / "vec").mkdir(parents=True)
        (root / "t.rs").write_text("")
        good = {
            "row": [{"id": "r1"}, {"id": "r2"}, {"id": "r3", "no_action": "Not built."}],
            "action": [
                {"id": "a1", "row": "r1", "checked": {"crosscheck": "one", "vectors": ["vec"],
                                                      "independence": {"class": "outside",
                                                                       "author": "them"}}},
                {"id": "a2", "row": "r2", "checked": {"tests": ["t.rs::x"]}},
            ],
        }
        fails = []
        if problems(root, good, ["vec"]):
            fails.append(f"a good manifest fails: {problems(root, good, ['vec'])}")
        bad = {"row": [{"id": "r1"}], "action": [{"id": "a1", "row": "r1", "checked": {"tests": ["gone.rs::x"]}}]}
        p = problems(root, bad, ["vec"])
        for want in ("row r1", "area one", "tools/vectors/vec"):
            if not any(want in x for x in p):
                fails.append(f"missed: {want}")
        unlabelled = {"action": [
            {"id": "b1", "checked": {"crosscheck": "one"}},
            {"id": "b2", "checked": {"crosscheck": "one",
                                     "independence": {"class": "own"}}},
            {"id": "b3", "novel_rank": 1, "checked": {"not_crosschecked": "none"}},
        ]}
        p = independence_problems(unlabelled)
        for want in ("b1: its cross-check has no `independence`", "b1: no `independence.author`",
                     "b2: no `independence.ours`", "b3: a novel action"):
            if not any(want in x for x in p):
                fails.append(f"missed: {want}")
        for f in fails:
            print(f"verified self-test: {f}")
        print("verified self-test: " + ("FAILED" if fails else "ok"))
        return 1 if fails else 0


def main(argv):
    if argv == ["--self-test"]:
        return self_test()
    if argv == ["check"]:
        return self_test() or cmd_check(ROOT)
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
