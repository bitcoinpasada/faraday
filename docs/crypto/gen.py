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
    STANDARDS §7       the Novel own calls, in the order of `novel_rank`; actions
                       of one rank are one item
    AUDIT §3           the "written here" list: each action's
                       `implementation.written_here`, grouped by crate

A row's cells come from the actions whose `row` is its id. A row with `uses`
(the actions it describes as well as the rows they belong to) takes them from
those actions; one with `no_action` (work not built, or work that computes
nothing of its own) has none. Either kind may give `standard`, `verified` and
`own` as text, which a no-action row must; a given text stands in its cell in
place of the generated one. `standard_note` follows the standards a row's
actions name.

The Verified by cell names a cross-check area by its `name` and says what it is
compared against: the area's `reference` up to its first clause, read from
tools/crosscheck/area_<file>.py without importing it.

faraday/faraday-learn/src/how.rs (the Learn blocks) is not generated yet:
`gen_how` is its place. Standard library only (`tomllib`, Python 3.11).
"""

import ast
import pathlib
import re
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[2]
MANIFEST = "docs/crypto/actions.toml"
STANDARDS = "docs/STANDARDS.md"
AUDIT = "docs/AUDIT.md"
HOW = "faraday/faraday-learn/src/how.rs"
AREAS = "tools/crosscheck"


def as_list(v):
    return v if isinstance(v, list) else ([] if v is None else [v])


def cell(s):
    """One table cell: no newline, no bare bar."""
    return " ".join(str(s).split()).replace("|", "\\|")


def sentence_end(s):
    s = s.strip()
    return s if s.endswith((".", "!", "?", ")")) and not s.endswith("..)") else s + "."


def bare(s):
    return s.strip().rstrip(".")


def lower_first(s):
    """A clause after a colon: its first word lower-cased unless it is a name."""
    w = s.split(" ", 1)[0]
    if w[:1].isupper() and (w[1:].islower() or len(w) == 1) and w not in ("Faraday", "Faraday's", "Core", "Core's"):
        return s[:1].lower() + s[1:]
    return s


def uniq(seq):
    seen, out = set(), []
    for x in seq:
        if x not in seen:
            seen.add(x)
            out.append(x)
    return out


# ---------------------------------------------------------------------- areas

def read_areas(root):
    """{file stem: (area name, what it is compared against)} from the area files."""
    out = {}
    d = pathlib.Path(root) / AREAS
    for p in sorted(d.glob("area_*.py")) if d.is_dir() else []:
        try:
            tree = ast.parse(p.read_text())
        except SyntaxError:
            continue
        for node in tree.body:
            if not isinstance(node, ast.ClassDef):
                continue
            vals = {}
            for s in node.body:
                if (isinstance(s, ast.Assign) and len(s.targets) == 1
                        and isinstance(s.targets[0], ast.Name)):
                    try:
                        vals[s.targets[0].id] = ast.literal_eval(s.value)
                    except ValueError:
                        pass
            if isinstance(vals.get("name"), str):
                out[p.stem.removeprefix("area_")] = (vals["name"], first_clause(vals.get("reference", "")))
    return out


CLAUSE_ENDS = (";", ":", " (", ", over ", ", from ", ", written ", ", with ")


def first_clause(s):
    """A reference up to where its first clause ends (CLAUSE_ENDS), outside backticks."""
    tick = False
    for i, ch in enumerate(s):
        if ch == "`":
            tick = not tick
        elif not tick and any(s.startswith(x, i) for x in CLAUSE_ENDS):
            return s[:i].strip()
    return s.strip()


# ------------------------------------------------------------------ standards

def status_text(std):
    s = std["status"]
    if str(std.get("source", "")).startswith("vendored"):
        s += ", vendored"
    return s


def runs(nums):
    """'380, 382 to 386' from numbers: a run of three or more is written as a range."""
    out, i = [], 0
    while i < len(nums):
        j = i
        while j + 1 < len(nums) and int(nums[j + 1]) == int(nums[j]) + 1:
            j += 1
        out.append(f"{nums[i]} to {nums[j]}" if j - i >= 2 else ", ".join(nums[i:j + 1]))
        i = j + 1
    return ", ".join(out)


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
    return ", ".join(f"{o[0]} {runs(sorted(o[1], key=int))}" if isinstance(o, list) else o
                     for o in out)


def standard_cell(data, keys, note=None):
    if not keys:
        return f"None: {note}" if note else "None"
    groups = {}
    for k in keys:
        std = data["standard"][k]
        groups.setdefault(status_text(std), []).append(k)
    text = "; ".join(f"{short_names(v)} ({s})" for s, v in groups.items())
    return f"{text}. {sentence_end(note)}" if note else text


def crate_of(test):
    """The crate a test file is in: `faraday-bip` for core/faraday-bip/tests/…."""
    parts = test.split("::")[0].split("/")
    if "src" in parts:
        parts = parts[:parts.index("src")]
    elif "tests" in parts:
        parts = parts[:parts.index("tests")]
    return parts[-1] if parts else "?"


def verified_cell(actions, areas):
    def each(key):
        return uniq(v for a in actions for v in as_list(a.get("checked", {}).get(key)))
    vec, tests, other = each("vectors"), each("tests"), each("other")
    selft, fuzz, nocc = each("selftest"), each("fuzz"), each("not_crosschecked")
    ccs = uniq(a["checked"]["crosscheck"] for a in actions if a.get("checked", {}).get("crosscheck"))
    parts = []
    if vec:
        parts.append("vectors " + ", ".join(f"`tools/vectors/{v}`" for v in vec))
    for stem in ccs:
        name, against = areas.get(stem, (stem, ""))
        parts.append(f"F79 area `{name}`" + (f" against {against}" if against else ""))
    parts += [bare(o) for o in other]
    if tests:
        crates = uniq(crate_of(t) for t in tests)
        parts.append(f"{len(tests)} test{'s' if len(tests) != 1 else ''} ({', '.join(crates)})")
    if selft:
        parts.append("self-test " + ", ".join(f"`{x}`" for x in selft))
    if fuzz:
        parts.append("fuzz " + ", ".join(f"`{x}`" for x in fuzz))
    if nocc:
        parts.append("not cross-checked: " + "; ".join(lower_first(bare(x)) for x in nocc))
    return "; ".join(parts) if parts else "None"


def own_cell(actions):
    outs = []
    for a in actions:
        for o in as_list(a.get("own")):
            dec = o.get("decision", "no decision")
            outs.append(f"{sentence_end(o['text'])} {o['kind']}; {dec}.")
    return " ".join(uniq(outs)) if outs else "None"


# --------------------------------------------------------------------- places

def standard_order(k):
    """BIPs, then RFCs, by number; then the rest by name."""
    m = re.fullmatch(r"(BIP|RFC) (\d+)", k)
    return (0 if m and m.group(1) == "BIP" else 1 if m else 2, int(m.group(2)) if m else 0, k.lower())


def gen_status_table(data):
    lines = ["| Standard | Name | Status | Read from | Checked |", "|---|---|---|---|---|"]
    for k, s in sorted(data["standard"].items(), key=lambda kv: standard_order(kv[0])):
        lines.append(f"| {cell(k)} | {cell(s['name'])} | {cell(s['status'])} | "
                     f"{cell(s['source'])} | {cell(s['checked'])} |")
    return "\n".join(lines)


def row_actions(data, row):
    """The actions a row describes: its own, or those it `uses`."""
    by_id = {a["id"]: a for a in data["action"]}
    own = [a for a in data["action"] if a.get("row") == row["id"]]
    return own + [by_id[i] for i in as_list(row.get("uses")) if i in by_id]


def row_cells(data, row, areas=None):
    areas = areas or {}
    acts = row_actions(data, row)
    stds = uniq(s for a in acts for s in as_list(a.get("standards")))
    std = row.get("standard") or standard_cell(data, stds, row.get("standard_note"))
    ver = row.get("verified") or verified_cell(acts, areas)
    own = row.get("own") or own_cell(acts)
    return [row["what"], std, row["completeness"], ver, own]


def gen_row_table(data, section, areas):
    lines = ["| What | Standard (status) | Completeness | Verified by | Faraday's own call |",
             "|---|---|---|---|---|"]
    for r in data.get("row", []):
        if r["section"] == section:
            lines.append("| " + " | ".join(cell(c) for c in row_cells(data, r, areas)) + " |")
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
    out += ["", "Not recorded in a decision:", ""]
    for t, d in uniq(undecided):
        out.append(f"- {sentence_end(t)}")
    return "\n".join(out)


def gen_novel(data):
    """One item per rank: the Novel calls of the actions of that rank, unranked last."""
    groups = {}
    for n, a in enumerate(data["action"]):
        novel = [o for o in as_list(a.get("own")) if o.get("kind") == "Novel"]
        if not novel:
            continue
        rank = a.get("novel_rank")
        key = (0, rank) if rank is not None else (1, n)
        groups.setdefault(key, []).append((a, novel))
    out = []
    for i, key in enumerate(sorted(groups), 1):
        parts = []
        for a, novel in groups[key]:
            calls = " ".join(f"{sentence_end(o['text'])} ({o.get('decision', 'no decision')})"
                             for o in novel)
            parts.append(f"**{bare(a['action'])}.** {calls}")
        out.append(f"{i}. " + " ".join(parts))
    return "\n".join(out)


def gen_written_here(data):
    by_crate = {}
    for a in data["action"]:
        impl = a.get("implementation", {})
        if impl.get("written_here"):
            name = impl.get("crate", "?").rsplit("/", 1)[-1]
            by_crate.setdefault(name, []).append(bare(impl["written_here"]))
    out = []
    for crate, texts in by_crate.items():
        out += [f"- {t}" for t in uniq(texts)]
    return "\n".join(out)


def gen_how(data):
    """The Learn blocks (faraday/faraday-learn/src/how.rs, §3.1): not generated yet.

    Another change is fixing the shape of that file; until it lands, `write` leaves
    it as it is and `check` does not compare it."""
    return None


PLACES = [
    (STANDARDS, "§1", lambda d, ar: gen_status_table(d)),
    (STANDARDS, "§2", lambda d, ar: gen_row_table(d, 2, ar)),
    (STANDARDS, "§3", lambda d, ar: gen_row_table(d, 3, ar)),
    (STANDARDS, "§4", lambda d, ar: gen_row_table(d, 4, ar)),
    (STANDARDS, "§5", lambda d, ar: gen_row_table(d, 5, ar)),
    (STANDARDS, "§6", lambda d, ar: gen_conservative(d)),
    (STANDARDS, "§7", lambda d, ar: gen_novel(d)),
    (AUDIT, "§3", lambda d, ar: gen_written_here(d)),
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
    areas = read_areas(root)
    texts = {}
    for path, name, fn in PLACES:
        text = texts.get(path)
        if text is None:
            text = (root / path).read_text()
        texts[path] = replace_region(text, name, fn(data, areas), path)
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
            "BIP 45": {"name": "n", "status": "Deployed", "source": "bips master", "checked": "d"},
            "BIP 46": {"name": "n", "status": "Deployed", "source": "bips master", "checked": "d"},
            "BIP 93": {"name": "n", "status": "Draft", "source": "vendored: x", "checked": "d"},
            "Seed XOR": {"name": "n", "status": "Convention", "source": "x", "checked": "d"},
        },
        "row": [{"id": "r", "section": 2, "what": "Wh|at", "completeness": "Full",
                 "standard_note": "The path is own"},
                {"id": "u", "section": 2, "what": "U", "completeness": "As in r", "uses": ["b"],
                 "own": "Typed."},
                {"id": "n", "section": 3, "what": "N", "completeness": "Not built",
                 "no_action": "Not built", "standard": "Its own", "verified": "None",
                 "own": "None"}],
        "action": [
            {"id": "a", "action": "Does a thing.", "row": "r",
             "standards": ["BIP 32", "BIP 44", "BIP 45", "BIP 46", "BIP 93", "Seed XOR"],
             "implementation": {"crate": "core/faraday-x", "libraries": [],
                                "written_here": "The thing."},
             "checked": {"vectors": ["v"], "tests": ["core/faraday-x/tests/t.rs::t"],
                         "not_crosschecked": "No second one.", "other": ["Core end to end."]},
             "own": [{"text": "Refuses it", "kind": "Conservative", "decision": "F90"},
                     {"text": "Draws it", "kind": "Novel", "decision": "no decision"}]},
            {"id": "b", "action": "Does b.", "row": "r", "standards": [],
             "novel_rank": 1, "implementation": {"crate": "core/faraday-y", "libraries": ["k"]},
             "checked": {"crosscheck": "ar"},
             "own": [{"text": "Mixes it.", "kind": "Novel", "decision": "F86"}]},
            {"id": "c", "action": "Does c.", "row": "n", "novel_rank": 1, "standards": [],
             "implementation": {"crate": "core/faraday-y", "libraries": ["k"]}, "checked": {},
             "own": [{"text": "Folds it.", "kind": "Novel", "decision": "no decision"}]},
        ],
    }
    areas = {"ar": ("area", "a reference")}
    print("tables:")
    r = row_cells(data, data["row"][0], areas)
    expect(r[1] == "BIP 32, 44 to 46 (Deployed); BIP 93 (Draft, vendored); Seed XOR (Convention)."
           " The path is own.", f"standards grouped by status, runs as ranges, the note after: {r[1]}")
    expect("`tools/vectors/v`" in r[3] and "1 test (faraday-x)" in r[3]
           and "F79 area `area` against a reference" in r[3] and "Core end to end" in r[3]
           and "not cross-checked: no second one" in r[3], f"checks are the union: {r[3]}")
    expect("Refuses it. Conservative; F90." in r[4], f"own calls carry kind and decision: {r[4]}")
    expect("Wh\\|at" in gen_row_table(data, 2, areas), "a bar in a cell is escaped")
    u = row_cells(data, data["row"][1], areas)
    expect("`area`" in u[3] and u[4] == "Typed.", f"a row that uses an action: {u}")
    n = row_cells(data, data["row"][2], areas)
    expect(n[1:] == ["Its own", "Not built", "None", "None"], f"a row with no action: {n}")
    expect(first_clause("python-mnemonic (`bip39/`); more") == "python-mnemonic",
           "an area's reference up to its first clause")
    expect(first_clause("`a; b` written, over c") == "`a; b` written", "a backticked semicolon is kept")
    expect(first_clause("BIP-1, 2 and 3 written out, over c") == "BIP-1, 2 and 3 written out",
           "a list's comma is kept")
    order = [k for k, _ in sorted({"RFC 9": 0, "Seed": 0, "BIP 10": 0, "BIP 9": 0, "aez": 0}.items(),
                                  key=lambda kv: standard_order(kv[0]))]
    expect(order == ["BIP 9", "BIP 10", "RFC 9", "aez", "Seed"], f"the status table's order: {order}")
    print("lists:")
    expect("- Refuses it. (F90)" in gen_conservative(data), "Conservative calls with their decision")
    nv = gen_novel(data).split("\n")
    expect(nv[0].startswith("1. **Does b.** Mixes it. (F86) **Does c.** Folds it.")
           and nv[1].startswith("2. **Does a thing.**"),
           f"Novel calls by novel_rank, one item per rank, unranked after: {nv}")
    expect(gen_written_here(data) == "- The thing", "written here, one line each, by crate")
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
