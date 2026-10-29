#!/usr/bin/env python3
"""The generated places of the manifest (docs/CRYPTO-LEARN.md §3.1).

    gen.py write     rewrite every generated place from docs/crypto/actions.toml
    gen.py check     regenerate in memory; fail on any difference
    gen.py --self-test

Each generated place of docs/STANDARDS.md and docs/AUDIT.md sits between two
marker lines, `<!-- crypto-manifest: begin §3 -->` and
`<!-- crypto-manifest: end §3 -->`, and is never edited by hand:

    STANDARDS §1       the status table, one row per [standard."..."]
    STANDARDS §2-§5    one table per section, one row per [[row]] of that section
    STANDARDS §6       the Conservative own calls, with and without a decision
    STANDARDS §7       the Novel own calls, in the order of `novel_rank`
    AUDIT §3           the "written here" list: the actions whose implementation
                       names no library, grouped by crate

faraday/faraday-learn/src/how.rs (the Learn blocks) is not generated yet:
`gen_how` is its place. Standard library only (`tomllib`, Python 3.11).
"""

import pathlib
import re
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[2]
MANIFEST = "docs/crypto/actions.toml"
STANDARDS = "docs/STANDARDS.md"
AUDIT = "docs/AUDIT.md"
HOW = "faraday/faraday-learn/src/how.rs"


def as_list(v):
    return v if isinstance(v, list) else ([] if v is None else [v])


def cell(s):
    """One table cell: no newline, no bare bar."""
    return " ".join(str(s).split()).replace("|", "\\|")


def sentence_end(s):
    s = s.strip()
    return s if s.endswith((".", "!", "?")) else s + "."


def bare(s):
    return s.strip().rstrip(".")


# ------------------------------------------------------------------ standards

def status_text(std):
    s = std["status"]
    if str(std.get("source", "")).startswith("vendored"):
        s += ", vendored"
    return s


def short_names(keys):
    """'BIP 32, 44, 49' for keys that share a prefix and a number; other keys whole."""
    out, cur = [], None
    for k in keys:
        m = re.fullmatch(r"(BIP|RFC) (\d+)", k)
        if m and cur is not None and cur[0] == m.group(1):
            cur[1].append(m.group(2))
        elif m:
            cur = [m.group(1), [m.group(2)]]
            out.append(cur)
        else:
            cur = None
            out.append(k)
    return ", ".join(f"{o[0]} {', '.join(o[1])}" if isinstance(o, list) else o for o in out)


def standard_cell(data, keys):
    if not keys:
        return "None"
    groups = {}
    for k in keys:
        std = data["standard"][k]
        groups.setdefault(status_text(std), []).append(k)
    return "; ".join(f"{short_names(v)} ({s})" for s, v in groups.items())


def uniq(seq):
    seen, out = set(), []
    for x in seq:
        if x not in seen:
            seen.add(x)
            out.append(x)
    return out


def verified_cell(actions):
    vec = uniq(v for a in actions for v in as_list(a.get("checked", {}).get("vectors")))
    tests = uniq(t for a in actions for t in as_list(a.get("checked", {}).get("tests")))
    areas = uniq(a["checked"]["crosscheck"] for a in actions if a.get("checked", {}).get("crosscheck"))
    nocc = uniq(a["checked"]["not_crosschecked"] for a in actions
                if a.get("checked", {}).get("not_crosschecked"))
    selft = uniq(t for a in actions for t in as_list(a.get("checked", {}).get("selftest")))
    fuzz = uniq(t for a in actions for t in as_list(a.get("checked", {}).get("fuzz")))
    parts = []
    if vec:
        parts.append("vectors " + ", ".join(f"`tools/vectors/{v}`" for v in vec))
    if tests:
        parts.append(f"{len(tests)} test{'s' if len(tests) != 1 else ''}")
    if areas:
        parts.append("F79 area" + ("s " if len(areas) > 1 else " ")
                     + ", ".join(f"`{x}`" for x in areas))
    if nocc:
        parts.append("not cross-checked: " + "; ".join(bare(x) for x in nocc))
    if selft:
        parts.append("self-test " + ", ".join(f"`{x}`" for x in selft))
    if fuzz:
        parts.append("fuzz " + ", ".join(f"`{x}`" for x in fuzz))
    return "; ".join(parts) if parts else "None"


def own_cell(actions):
    outs = []
    for a in actions:
        for o in as_list(a.get("own")):
            dec = o.get("decision", "no decision")
            outs.append(f"{sentence_end(o['text'])} {o['kind']}; {dec}.")
    return " ".join(uniq(outs)) if outs else "None"


# --------------------------------------------------------------------- places

def gen_status_table(data):
    lines = ["| Standard | Name | Status | Read from | Checked |", "|---|---|---|---|---|"]
    for k, s in data["standard"].items():
        lines.append(f"| {cell(k)} | {cell(s['name'])} | {cell(s['status'])} | "
                     f"{cell(s['source'])} | {cell(s['checked'])} |")
    return "\n".join(lines)


def row_cells(data, row):
    acts = [a for a in data["action"] if a.get("row") == row["id"]]
    stds = uniq(s for a in acts for s in as_list(a.get("standards")))
    return [row["what"], standard_cell(data, stds), row["completeness"],
            verified_cell(acts), own_cell(acts)]


def gen_row_table(data, section):
    lines = ["| What | Standard (status) | Completeness | Verified by | Faraday's own call |",
             "|---|---|---|---|---|"]
    for r in data.get("row", []):
        if r["section"] == section:
            lines.append("| " + " | ".join(cell(c) for c in row_cells(data, r)) + " |")
    return "\n".join(lines)


def own_calls(data, kind):
    """(action, own) for every own call of a kind, in the manifest's order."""
    return [(a, o) for a in data["action"] for o in as_list(a.get("own")) if o.get("kind") == kind]


def gen_conservative(data):
    calls = own_calls(data, "Conservative")
    decided, undecided = [], []
    for a, o in calls:
        d = o.get("decision", "no decision")
        (undecided if d == "no decision" else decided).append((o["text"], d))
    out = ["Recorded in a decision:", ""]
    for t, d in uniq(decided):
        out.append(f"- {sentence_end(t)} ({d})")
    out += ["", "Not recorded in a decision, each refusing what a standard allows:", ""]
    for t, d in uniq(undecided):
        out.append(f"- {sentence_end(t)}")
    return "\n".join(out)


def gen_novel(data):
    ranked = []
    for n, a in enumerate(data["action"]):
        novel = [o for o in as_list(a.get("own")) if o.get("kind") == "Novel"]
        if novel or "novel_rank" in a:
            rank = a.get("novel_rank")
            ranked.append((0 if rank is not None else 1, rank if rank is not None else 0, n, a, novel))
    ranked.sort(key=lambda x: x[:3])
    out = []
    for i, (_, _, _, a, novel) in enumerate(ranked, 1):
        calls = " ".join(f"{sentence_end(o['text'])} ({o.get('decision', 'no decision')})"
                         for o in novel)
        out.append(f"{i}. **{bare(a['action'])}.** {calls}".rstrip())
    return "\n".join(out)


def gen_written_here(data):
    by_crate = {}
    for a in data["action"]:
        impl = a.get("implementation", {})
        if not as_list(impl.get("libraries")):
            by_crate.setdefault(impl.get("crate", "?"), []).append(a)
    out = []
    for crate, acts in sorted(by_crate.items(), key=lambda kv: kv[0].rsplit("/", 1)[-1]):
        out.append(f"- `{crate.rsplit('/', 1)[-1]}`")
        for a in acts:
            out.append(f"  - {bare(a['action'])}")
    return "\n".join(out)


def gen_how(data):
    """The Learn blocks (faraday/faraday-learn/src/how.rs, §3.1): not generated yet.

    Another change is fixing the shape of that file; until it lands, `write` leaves
    it as it is and `check` does not compare it."""
    return None


PLACES = [
    (STANDARDS, "§1", gen_status_table),
    (STANDARDS, "§2", lambda d: gen_row_table(d, 2)),
    (STANDARDS, "§3", lambda d: gen_row_table(d, 3)),
    (STANDARDS, "§4", lambda d: gen_row_table(d, 4)),
    (STANDARDS, "§5", lambda d: gen_row_table(d, 5)),
    (STANDARDS, "§6", gen_conservative),
    (STANDARDS, "§7", gen_novel),
    (AUDIT, "§3", gen_written_here),
]


# -------------------------------------------------------------------- markers

def begin(name):
    return f"<!-- crypto-manifest: begin {name} -->"


def end(name):
    return f"<!-- crypto-manifest: end {name} -->"


def replace_region(text, name, body, path):
    b, e = begin(name), end(name)
    if text.count(b) != 1 or text.count(e) != 1:
        raise SystemExit(f"{path}: needs exactly one `{b}` and one `{e}` line")
    i, j = text.index(b), text.index(e)
    if j < i:
        raise SystemExit(f"{path}: `{e}` comes before `{b}`")
    return text[:i + len(b)] + "\n" + body + "\n" + text[j:]


def generated(root, data=None):
    """{path: new text} for every file that has generated places."""
    root = pathlib.Path(root)
    if data is None:
        data = tomllib.loads((root / MANIFEST).read_text())
    texts = {}
    for path, name, fn in PLACES:
        text = texts.get(path)
        if text is None:
            text = (root / path).read_text()
        texts[path] = replace_region(text, name, fn(data), path)
    gen_how(data)
    return texts


def cmd_write(root):
    changed = 0
    for path, text in generated(root).items():
        p = pathlib.Path(root) / path
        if p.read_text() != text:
            p.write_text(text)
            changed += 1
            print(f"wrote {path}")
    print(f"{HOW}: not generated yet (its shape is being fixed)")
    print(f"generated places: {len(PLACES)} in {len({p for p, _, _ in PLACES})} files, "
          f"{changed} rewritten")
    return 0


def cmd_check(root):
    bad = 0
    for path, text in generated(root).items():
        old = (pathlib.Path(root) / path).read_text()
        if old == text:
            continue
        bad += 1
        a, b = old.split("\n"), text.split("\n")
        n = next((i for i in range(min(len(a), len(b))) if a[i] != b[i]), min(len(a), len(b)))
        print(f"{path}: differs from the manifest, first at line {n + 1}")
        print(f"  file:      {(a[n] if n < len(a) else '')[:120]}")
        print(f"  generated: {(b[n] if n < len(b) else '')[:120]}")
        print("  run: python3 -I -B docs/crypto/gen.py write")
    return 1 if bad else 0


# ------------------------------------------------------------------ self-test

def self_test():
    failures = []

    def expect(cond, what):
        print(f"  {'ok  ' if cond else 'FAIL'} {what}")
        if not cond:
            failures.append(what)

    data = {
        "standard": {
            "BIP 32": {"name": "n", "status": "Deployed", "source": "bips master", "checked": "d"},
            "BIP 44": {"name": "n", "status": "Deployed", "source": "bips master", "checked": "d"},
            "BIP 93": {"name": "n", "status": "Draft", "source": "vendored: x", "checked": "d"},
            "Seed XOR": {"name": "n", "status": "Convention", "source": "x", "checked": "d"},
        },
        "row": [{"id": "r", "section": 2, "what": "Wh|at", "completeness": "Full"}],
        "action": [
            {"id": "a", "action": "Does a thing.", "row": "r",
             "standards": ["BIP 32", "BIP 44", "BIP 93", "Seed XOR"],
             "implementation": {"crate": "core/faraday-x", "libraries": []},
             "checked": {"vectors": ["v"], "tests": ["f::t"], "not_crosschecked": "No second one."},
             "own": [{"text": "Refuses it", "kind": "Conservative", "decision": "F90"},
                     {"text": "Draws it", "kind": "Novel", "decision": "no decision"}]},
            {"id": "b", "action": "Does b.", "row": "r", "standards": [],
             "novel_rank": 1, "implementation": {"crate": "core/faraday-y", "libraries": ["k"]},
             "checked": {"crosscheck": "ar"},
             "own": [{"text": "Mixes it.", "kind": "Novel", "decision": "F86"}]},
        ],
    }
    print("tables:")
    r = row_cells(data, data["row"][0])
    expect(r[1] == "BIP 32, 44 (Deployed); BIP 93 (Draft, vendored); Seed XOR (Convention)",
           f"standards grouped by status: {r[1]}")
    expect("`tools/vectors/v`" in r[3] and "1 test" in r[3] and "`ar`" in r[3]
           and "not cross-checked" in r[3], f"checks are the union: {r[3]}")
    expect("Refuses it. Conservative; F90." in r[4], f"own calls carry kind and decision: {r[4]}")
    expect("Wh\\|at" in gen_row_table(data, 2), "a bar in a cell is escaped")
    print("lists:")
    expect("- Refuses it. (F90)" in gen_conservative(data), "Conservative calls with their decision")
    nv = gen_novel(data).split("\n")
    expect(nv[0].startswith("1. **Does b.**") and nv[1].startswith("2. **Does a thing.**"),
           f"Novel calls in novel_rank order, unranked after: {nv}")
    expect(gen_written_here(data) == "- `faraday-x`\n  - Does a thing", "written here: no library")
    print("regions:")
    t = f"x\n{begin('§1')}\nold\n{end('§1')}\ny\n"
    n = replace_region(t, "§1", "new", "t")
    expect(n == f"x\n{begin('§1')}\nnew\n{end('§1')}\ny\n", "a region is replaced between its markers")
    expect(replace_region(n, "§1", "new", "t") == n, "and replacing again changes nothing")
    try:
        replace_region("none", "§1", "x", "t")
        expect(False, "a missing marker fails")
    except SystemExit:
        expect(True, "a missing marker fails")
    if failures:
        print(f"{len(failures)} failed")
        return 1
    print("gen.py self-test passed")
    return 0


def main(argv):
    if argv[1:] == ["--self-test"]:
        return self_test()
    if len(argv) == 2 and argv[1] in ("write", "check"):
        return (cmd_write if argv[1] == "write" else cmd_check)(ROOT)
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
