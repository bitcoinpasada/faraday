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
    tools/crosscheck/README.md, independence
                       what each area compares Faraday with: one row per area
                       and class (F160, F177), outside or own, with the actions

A row's cells come from the actions whose `row` is its id. A row with `uses`
(the actions it describes as well as the rows they belong to) takes them from
those actions; one with `no_action` (work not built, or work that computes
nothing of its own) has none. Either kind may give `standard`, `verified` and
`own` as text, which a no-action row must; a given text stands in its cell in
place of the generated one. `standard_note` follows the standards a row's
actions name.

The Verified by cell names a cross-check area by its `name` and says what it is
compared against: the area's `reference` up to its first clause, read from
tools/crosscheck/area_<file>.py without importing it, and, where the row's
actions on that area compare Faraday, wholly or in part, with an implementation
Faraday's authors wrote (`independence`, F160, F177), that it is, or is in part,
not checked against an outside source.

faraday/faraday-learn/src/how.rs (the Learn blocks) is written by gen_how.py,
called from `generated`, with its CONSTRUCTIONS (docs/CRYPTO-LEARN.md §18) from
here: each paragraph's placeholders filled from the code and each table written
from it (`render_construction`, which check.py also runs). Standard library only
(`tomllib`, Python 3.11).
"""

import ast
import hashlib
import math
import pathlib
import re
import sys
import tomllib

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gen_how  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parents[2]
MANIFEST = "docs/crypto/actions.toml"
STANDARDS = "docs/STANDARDS.md"
AUDIT = "docs/AUDIT.md"
HOW = "faraday/faraday-learn/src/how.rs"
AREAS = "tools/crosscheck"
CC_README = "tools/crosscheck/README.md"

# What each class compares Faraday with (F160, F177), the one checked against an outside
# source least last.
INDEPENDENCE = {
    "outside": "an outside implementation",
    "outside-and-own": "an outside implementation and, for a part, one Faraday's authors wrote",
    "own": "an implementation Faraday's authors wrote",
}
NOT_OUTSIDE = {
    "outside-and-own": "in part not checked against an outside source",
    "own": "not checked against an outside source",
}


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
        classes = {(a["checked"].get("independence") or {}).get("class") for a in actions
                   if a.get("checked", {}).get("crosscheck") == stem}
        weakest = next((k for k in reversed(INDEPENDENCE) if k in classes), "outside")
        note = f" ({NOT_OUTSIDE[weakest]})" if weakest in NOT_OUTSIDE else ""
        parts.append(f"F79 area `{name}`" + (f" against {against}" if against else "") + note)
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
        if any(a.get("label") for a, _ in groups[key]):
            parts.append("Own specification, not yet reviewed outside Faraday (F160, F177).")
        out.append(f"{i}. " + " ".join(parts))
    return "\n".join(out)


def gen_independence(data, areas):
    """tools/crosscheck/README.md: one row per area and class, what it compares Faraday
    with, the outside implementation, the part Faraday's authors wrote, and the actions
    (F160, F177)."""
    groups = {}
    for a in data["action"]:
        chk = a.get("checked", {})
        stem = chk.get("crosscheck")
        if not stem:
            continue
        name = areas.get(stem, (stem, ""))[0]
        ind = chk.get("independence") or {}
        key = (name, ind.get("class", "?"), ind.get("author", ""), ind.get("ours", ""))
        groups.setdefault(key, []).append(a["id"])
    lines = ["| Area | Compared with | Outside implementation | Written by Faraday's authors | Actions |",
             "|---|---|---|---|---|"]
    for (name, ind, by, ours), ids in groups.items():
        lines.append(f"| `{name}` | {cell(INDEPENDENCE.get(ind, ind))} | {cell(by) if by else 'none'} | "
                     f"{cell(ours) if ours else 'none'} | {', '.join(ids)} |")
    return "\n".join(lines)


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


# ------------------------------------------------------------- constructions

# A construction (docs/CRYPTO-LEARN.md §18) is drawn from its paragraphs, whose
# numbers are placeholders filled from the code here, and its tables, written
# from the code here; nothing in them is typed by hand.

PLANNED_LEAD = "Planned, not built: "
TABLE_KINDS = ("vault-header", "vault-sizes", "diceware-bits")
VAULT = "faraday/faraday-vault/src/lib.rs"
OPEN_PY = "tools/vault/open.py"
DICEWARE = "core/faraday-bip/src/diceware"
NUMBER_WORDS = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
                "ten", "eleven", "twelve"]
# `{v:<name>}`: a value the code states as a literal inside a function, read from
# that function's body; the reader fails when the code no longer has that shape.
CODE_VALUES = {
    "argon2-version": (VAULT, "stretch", r"Version::V0x(\d)(\d)", lambda m: f"{m[1]}.{m[2]}"),
    "argon2-out": (VAULT, "stretch", r"Some\((\d+)\)", lambda m: int(m[1])),
    "lanes": (VAULT, "check_cost", r"cost\.lanes != (\d+)", lambda m: int(m[1])),
    "min-passes": (VAULT, "check_cost", r"\((\d+)\.\.=MAX_PASSES\)", lambda m: int(m[1])),
}
# The header's fields, by what `Header::to_bytes` copies into each.
HEADER_FIELDS = {
    "MAGIC": "magic",
    "VERSION": "version, {VERSION}",
    "self.cost.memory_kib": "Argon2id memory, in KiB",
    "self.cost.passes": "Argon2id passes",
    "self.cost.lanes": "Argon2id lanes",
    "self.slot_len": "L",
    "self.salt": "Argon2id salt",
}
FILE_LEN = "HEADER_LEN + SLOTS * (slot_len as usize + NONCE_LEN + TAG_LEN)"


class Unresolved(Exception):
    """A placeholder or a table the code does not give."""


def fmt_num(x):
    """A number as the account writes it: to a tenth where it is not whole, its
    digits in threes from a thousand."""
    if isinstance(x, float) and abs(x - round(x)) > 1e-9:
        return f"{x:.1f}"
    n = int(round(x))
    s = f"{abs(n):,}".replace(",", " ") if abs(n) >= 1000 else str(abs(n))
    return ("-" if n < 0 else "") + s


def word_of(x):
    whole = not isinstance(x, float) or abs(x - round(x)) < 1e-9
    if not whole or not 0 <= int(round(x)) < len(NUMBER_WORDS):
        raise Unresolved(f"{fmt_num(x)} has no word (`|words` is for zero to twelve)")
    return NUMBER_WORDS[int(round(x))]


def rust_body(text, name):
    """The body of `fn name` in a Rust source, braces matched; None if absent."""
    m = re.search(rf"\bfn {re.escape(name)}\b[^{{]*\{{", text)
    if not m:
        return None
    depth, i = 1, m.end()
    while i < len(text) and depth:
        depth += {"{": 1, "}": -1}.get(text[i], 0)
        i += 1
    return text[m.end():i - 1]


class Code:
    """What a construction's placeholders and tables read from the tree. Each
    value read is noted as a lock line: the construction's fingerprint (§18.6)."""

    def __init__(self, root):
        self.root = pathlib.Path(root)
        self.files = {}
        self.lines = set()

    def read(self, rel):
        if rel not in self.files:
            p = self.root / rel
            if not rel or not p.is_file():
                raise Unresolved(f"{rel or 'a placeholder'}: no such file")
            self.files[rel] = p.read_text()
        return self.files[rel]

    def item(self, rel, name):
        """(type, value) of `const` or `static` `name` in a Rust file, as written."""
        m = re.search(rf"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const|static)\s+{re.escape(name)}\s*:"
                      r"\s*(?P<ty>[^=]+?)\s*=\s*(?P<val>.*?);\s*$", self.read(rel), re.M | re.S)
        if not m:
            raise Unresolved(f"{rel} has no constant or static `{name}`")
        return m.group("ty"), m.group("val")

    def const(self, rel, name, depth=0):
        """An integer constant: an expression of literals and the file's other constants."""
        if depth > 8:
            raise Unresolved(f"{rel}::{name} refers to itself")
        _, val = self.item(rel, name)
        v = self.int_expr(rel, val, f"{rel}::{name}", depth)
        self.lines.add(f"val c:{rel}::{name} {v}")
        return v

    def int_expr(self, rel, expr, what, depth=0):
        expr = re.sub(r"\bas\s+(?:u8|u16|u32|u64|usize|i32|i64)\b", "", expr)
        expr = re.sub(r"(?<=\d)_(?=\d)", "", expr)
        expr = re.sub(r"\b(\d+)(?:u8|u16|u32|u64|usize)\b", r"\1", expr)
        try:
            tree = ast.parse(expr.strip(), mode="eval")
        except SyntaxError:
            raise Unresolved(f"{what} is not an integer expression gen.py evaluates")

        def ev(n):
            if isinstance(n, ast.Expression):
                return ev(n.body)
            if isinstance(n, ast.Constant) and type(n.value) is int:
                return n.value
            if isinstance(n, ast.Name):
                return self.const(rel, n.id, depth + 1)
            if isinstance(n, ast.BinOp) and type(n.op) in (ast.Add, ast.Sub, ast.Mult, ast.Div):
                a, b = ev(n.left), ev(n.right)
                if isinstance(n.op, ast.Div) and not b:
                    raise Unresolved(f"{what} divides by zero")
                return {ast.Add: lambda: a + b, ast.Sub: lambda: a - b, ast.Mult: lambda: a * b,
                        ast.Div: lambda: a // b}[type(n.op)]()
            raise Unresolved(f"{what} is not an integer expression gen.py evaluates")

        return ev(tree)

    def length(self, rel, name):
        ty, _ = self.item(rel, name)
        m = re.fullmatch(r"\[[^;\]]+;\s*([^\]]+)\]", ty.strip())
        if not m:
            raise Unresolved(f"{rel}::{name} is not an array of a stated length")
        v = self.int_expr(rel, m.group(1), f"the length of {rel}::{name}")
        self.lines.add(f"val len:{rel}::{name} {v}")
        return v

    def int_list(self, rel, name):
        _, val = self.item(rel, name)
        m = re.fullmatch(r"\[(.*)\]", val.strip(), re.S)
        if not m:
            raise Unresolved(f"{rel}::{name} is not an array literal")
        out = [self.int_expr(rel, x, f"an element of {rel}::{name}")
               for x in m.group(1).split(",") if x.strip()]
        self.lines.add(f"val list:{rel}::{name} {' '.join(map(str, out))}")
        return out

    def byte_string(self, rel, name):
        _, val = self.item(rel, name)
        m = re.fullmatch(r'\*?b"([ -~]*)"', val.strip())
        if not m or "\\" in m.group(1):
            raise Unresolved(f"{rel}::{name} is not a byte string literal")
        self.lines.add(f"val b:{rel}::{name} {m.group(1)}")
        return m.group(1)

    def body(self, rel, name):
        b = rust_body(self.read(rel), name)
        if b is None:
            raise Unresolved(f"{rel} has no function `{name}`")
        return b

    def function(self, rel, name):
        self.body(rel, name.split("::")[-1])
        self.lines.add(f"val fn:{rel}::{name}")
        return name

    def value(self, name):
        if name not in CODE_VALUES:
            raise Unresolved(f"`{{v:{name}}}` is no value gen.py reads ({', '.join(CODE_VALUES)})")
        rel, fn, pat, get = CODE_VALUES[name]
        m = re.search(pat, self.body(rel, fn))
        if not m:
            raise Unresolved(f"{rel}::{fn} no longer has the shape `{{v:{name}}}` reads ({pat})")
        v = get(m)
        self.lines.add(f"val v:{name} {v}")
        return v


def placeholders(text):
    """[(start, end, inner)] of the outermost `{…}` spans of a text."""
    out, depth, start = [], 0, None
    for i, ch in enumerate(text):
        if ch == "{":
            if depth == 0:
                start = i
            depth += 1
        elif ch == "}":
            if not depth:
                raise Unresolved("a `}` closes nothing")
            depth -= 1
            if depth == 0:
                out.append((start, i + 1, text[start + 1:i]))
    if depth:
        raise Unresolved("a `{` is not closed")
    return out


def split_mod(inner):
    """(placeholder, modifier): `|words` after it, outside any inner placeholder."""
    depth, cut = 0, -1
    for i, ch in enumerate(inner):
        depth += {"{": 1, "}": -1}.get(ch, 0)
        if ch == "|" and depth == 0:
            cut = i
    return (inner, "") if cut < 0 else (inner[:cut], inner[cut + 1:])


def resolve(code, inner, planned=False):
    """A placeholder's value: a number, or text."""
    body, mod = split_mod(inner)
    if mod and mod != "words":
        raise Unresolved(f"`|{mod}` is no modifier (only `|words`)")
    kind, _, ref = body.partition(":")
    rel, _, name = ref.partition("::")
    if kind in ("c", "len", "b", "fn") and not (rel and name):
        raise Unresolved(f"`{{{inner}}}` names no `<file>::<name>`")
    if kind == "c":
        v = code.const(rel, name)
    elif kind == "len":
        v = code.length(rel, name)
    elif kind == "b":
        v = code.byte_string(rel, name)
    elif kind == "fn":
        v = code.function(rel, name)
    elif kind == "v":
        v = code.value(ref)
    elif kind == "planned":
        if not planned:
            raise Unresolved(f"`{{{inner}}}`, a number of work not built, is only for a planned "
                             "paragraph")
        if not re.fullmatch(r"\d+", ref):
            raise Unresolved(f"`{{{inner}}}` is not a whole number")
        v = int(ref)
    elif kind == "f":
        v = arith(code, ref, planned)
    else:
        raise Unresolved(f"`{{{inner}}}` is no placeholder (c, len, f, b, v, fn, planned)")
    if mod == "words":
        if isinstance(v, str):
            raise Unresolved(f"`{{{inner}}}` is text, which has no word")
        return word_of(v)
    return v


def arith(code, expr, planned):
    """`{f:…}`: + - × / over numbers and placeholders, and `log2`, to a tenth."""
    parts, last = [], 0
    for s, e, inner in placeholders(expr):
        parts.append(expr[last:s])
        v = resolve(code, inner, planned)
        if isinstance(v, str):
            raise Unresolved(f"`{{{inner}}}` in an expression is not a number")
        parts.append(repr(v))
        last = e
    parts.append(expr[last:])
    try:
        tree = ast.parse("".join(parts).strip(), mode="eval")
    except SyntaxError:
        raise Unresolved(f"`{{f:{expr}}}` is not arithmetic")

    def ev(n):
        if isinstance(n, ast.Expression):
            return ev(n.body)
        if isinstance(n, ast.Constant) and type(n.value) in (int, float):
            return n.value
        if isinstance(n, ast.BinOp) and type(n.op) in (ast.Add, ast.Sub, ast.Mult, ast.Div):
            a, b = ev(n.left), ev(n.right)
            if isinstance(n.op, ast.Div):
                if not b:
                    raise Unresolved(f"`{{f:{expr}}}` divides by zero")
                return a / b
            return {ast.Add: a + b, ast.Sub: a - b, ast.Mult: a * b}[type(n.op)]
        if (isinstance(n, ast.Call) and isinstance(n.func, ast.Name) and n.func.id == "log2"
                and len(n.args) == 1 and not n.keywords):
            a = ev(n.args[0])
            if a <= 0:
                raise Unresolved(f"`{{f:{expr}}}` takes log2 of {a}")
            return math.log2(a)
        raise Unresolved(f"`{{f:{expr}}}` is not arithmetic gen.py evaluates")

    v = ev(tree)
    if abs(v - round(v)) < 1e-9:
        return int(round(v))
    return round(v, 1)


def render_text(code, text, planned=False):
    """A paragraph with its placeholders filled."""
    out, last = [], 0
    for s, e, inner in placeholders(text):
        out.append(text[last:s])
        v = resolve(code, inner, planned)
        out.append(fmt_num(v) if isinstance(v, (int, float)) else v)
        last = e
    out.append(text[last:])
    return "".join(out)


def table_vault_header(code):
    body = code.body(VAULT, "to_bytes")
    rows, at = [], 0
    for m in re.finditer(r"h\[(\d+)(?:\.\.(\d+))?\](?:\.copy_from_slice\(&(.+?)\);| = (\w+);)", body):
        start = int(m[1])
        end = int(m[2]) if m[2] else start + 1
        src = (m[3] or m[4]).removesuffix(".to_le_bytes()")
        if start != at:
            raise Unresolved(f"{VAULT}::Header::to_bytes writes a field at {start}, where {at} "
                             "was expected")
        if src not in HEADER_FIELDS:
            raise Unresolved(f"{VAULT}::Header::to_bytes copies `{src}`, which gen.py's "
                             "HEADER_FIELDS does not name")
        what = HEADER_FIELDS[src].replace("{VERSION}", str(code.const(VAULT, "VERSION")))
        rows.append([fmt_num(start), fmt_num(end - start), what])
        at = end
    header_len = code.const(VAULT, "HEADER_LEN")
    if not rows or at != header_len:
        raise Unresolved(f"{VAULT}::Header::to_bytes writes {at} bytes, and HEADER_LEN is "
                         f"{header_len}")
    check_open_py(code, rows)
    return ["At", "Bytes", "What"], rows


def check_open_py(code, header_rows=None):
    """tools/vault/open.py, which reads the vectors, has the Rust code's sizes and offsets."""
    py = code.read(OPEN_PY)
    for name in ("HEADER_LEN", "NONCE_LEN", "TAG_LEN"):
        m = re.search(rf"^{name} = (\d+)$", py, re.M)
        if not m or int(m[1]) != code.const(VAULT, name):
            raise Unresolved(f"{OPEN_PY}: {name} is not {VAULT}'s")
    m = re.search(r"^SLOT_SIZES = \(([\d, ]+)\)$", py, re.M)
    if not m or [int(x) for x in m[1].split(",") if x.strip()] != code.int_list(VAULT, "SLOT_SIZES"):
        raise Unresolved(f"{OPEN_PY}: SLOT_SIZES is not {VAULT}'s")
    if header_rows:
        at = {r[2]: int(r[0].replace(" ", "")) for r in header_rows}
        salt = re.search(r"salt = blob\[(\d+):(\d+)\]", py)
        ints = re.search(r'struct\.unpack_from\("<4I", blob, (\d+)\)', py)
        if (not salt or int(salt[1]) != at.get("Argon2id salt")
                or int(salt[2]) != code.const(VAULT, "HEADER_LEN")):
            raise Unresolved(f"{OPEN_PY}: the salt's slice is not the header's")
        if not ints or int(ints[1]) != at.get("Argon2id memory, in KiB"):
            raise Unresolved(f"{OPEN_PY}: the four integers are not read where the header has them")


def table_vault_sizes(code):
    body = " ".join(code.body(VAULT, "file_len").split())
    if body != FILE_LEN:
        raise Unresolved(f"{VAULT}::file_len is no longer `{FILE_LEN}`")
    h, s = code.const(VAULT, "HEADER_LEN"), code.const(VAULT, "SLOTS")
    n, t = code.const(VAULT, "NONCE_LEN"), code.const(VAULT, "TAG_LEN")
    check_open_py(code)
    return ["L", "File"], [[fmt_num(sl), fmt_num(h + s * (sl + n + t))]
                           for sl in code.int_list(VAULT, "SLOT_SIZES")]


def diceware_lists(code):
    d = code.root / DICEWARE
    files = sorted(p.name for p in d.glob("*.rs") if p.name != "mod.rs") if d.is_dir() else []
    if not files:
        raise Unresolved(f"{DICEWARE} has no word lists")
    out = []
    for f in files:
        m = re.fullmatch(r"(large)|short(\d)", f.removesuffix(".rs"))
        if not m:
            raise Unresolved(f"{DICEWARE}/{f} is no list gen.py names")
        name = "EFF large list" if m[1] else f"EFF short list {m[2]}"
        out.append((name, code.length(f"{DICEWARE}/{f}", "WORDS")))
    return out


def table_diceware(code, words):
    if not (isinstance(words, list) and len(words) == 2 and all(type(w) is int for w in words)
            and 0 < words[0] <= words[1]):
        raise Unresolved("diceware-bits takes `words = [first, last]`")
    lists = diceware_lists(code)
    head = ["Words"] + [f"{name}: {fmt_num(n)} words, {math.log2(n):.1f} bits a word"
                        for name, n in lists]
    rows = [[str(w)] + [f"{w * math.log2(n):.1f}" for _, n in lists]
            for w in range(words[0], words[1] + 1)]
    return head, rows


def render_table(code, para):
    kind = para.get("table")
    if kind == "vault-header":
        return table_vault_header(code)
    if kind == "vault-sizes":
        return table_vault_sizes(code)
    if kind == "diceware-bits":
        return table_diceware(code, para.get("words"))
    raise Unresolved(f"table `{kind}` is not one gen.py writes ({', '.join(TABLE_KINDS)})")


def render_construction(root, con):
    """(sections, problems, lock lines): each section (heading, [item]), an item
    ("para", text) or ("table", head, rows)."""
    code = Code(root)
    problems, sections = [], []
    for s in as_list(con.get("section")):
        items = []
        for p in as_list(s.get("para")):
            try:
                if "table" in p:
                    head, rows = render_table(code, p)
                    items.append(("table", head, rows))
                    digest = hashlib.sha256(repr((head, rows)).encode()).hexdigest()[:16]
                    code.lines.add(f"table {p['table']} {digest}")
                else:
                    planned = "planned" in p
                    t = render_text(code, str(p.get("text", "")), planned)
                    items.append(("para", PLANNED_LEAD + t if planned else t))
            except Unresolved as e:
                problems.append(f"{s.get('heading')}: {e}")
        sections.append((s.get("heading", ""), items))
    return sections, problems, sorted(code.lines)


def rs_str(s):
    return '"' + str(s).replace("\\", "\\\\").replace('"', '\\"') + '"'


def gen_constructions(root, data):
    """The CONSTRUCTIONS part of faraday/faraday-learn/src/how.rs, which gen_how.py puts
    after the blocks."""
    out = ["use crate::construction::{Construction, Item, Part, Table};", "",
           "/// Every construction (`docs/CRYPTO-LEARN.md` §18), in the manifest's order.",
           "#[rustfmt::skip]",
           "pub static CONSTRUCTIONS: &[Construction] = &["]
    for con in data.get("construction", []):
        sections, problems, _ = render_construction(root, con)
        if problems:
            raise SystemExit(f"construction {con.get('id')}: " + "; ".join(problems)
                             + " (docs/crypto/check.py check says more)")
        out += ["    Construction {",
                f"        id: {rs_str(con.get('id'))},",
                f"        title: {rs_str(con.get('title'))},",
                f"        learn: {rs_str(con.get('learn'))},",
                "        actions: &[" + ", ".join(rs_str(a) for a in as_list(con.get("actions"))) + "],",
                "        sections: &["]
        for heading, items in sections:
            out += ["            Part {", f"                heading: {rs_str(heading)},",
                    "                items: &["]
            for it in items:
                if it[0] == "para":
                    out.append(f"                    Item::Para({rs_str(it[1])}),")
                    continue
                out += ["                    Item::Table(Table {",
                        "                        head: &[" + ", ".join(map(rs_str, it[1])) + "],",
                        "                        rows: &["]
                out += ["                            &[" + ", ".join(map(rs_str, r)) + "],"
                        for r in it[2]]
                out += ["                        ],", "                    }),"]
            out += ["                ],", "            },"]
        out += ["        ],", "    },"]
    out.append("];")
    return "\n".join(out)


PLACES = [
    (STANDARDS, "§1", lambda d, ar: gen_status_table(d)),
    (STANDARDS, "§2", lambda d, ar: gen_row_table(d, 2, ar)),
    (STANDARDS, "§3", lambda d, ar: gen_row_table(d, 3, ar)),
    (STANDARDS, "§4", lambda d, ar: gen_row_table(d, 4, ar)),
    (STANDARDS, "§5", lambda d, ar: gen_row_table(d, 5, ar)),
    (STANDARDS, "§6", lambda d, ar: gen_conservative(d)),
    (STANDARDS, "§7", lambda d, ar: gen_novel(d)),
    (AUDIT, "§3", lambda d, ar: gen_written_here(d)),
    (CC_README, "independence", gen_independence),
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
    texts[HOW] = gen_how.how_text(root, data, gen_constructions(root, data))
    return texts


def cmd_write(root):
    changed = 0
    for path, text in generated(root).items():
        p = pathlib.Path(root) / path
        if p.read_text() != text:
            p.write_text(text)
            changed += 1
            print(f"wrote {path}")
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
    print("constructions:")
    import tempfile
    with tempfile.TemporaryDirectory() as d:
        (pathlib.Path(d) / "k.rs").write_text(
            "pub const LEN: usize = 4 + 1 + SALT;\nconst SALT: usize = 1_024;\n"
            "pub static W: [&str; SALT] = [];\nconst TAG: &[u8] = b\"a v1 b\";\n")
        code = Code(d)
        t = render_text(code, "{c:k.rs::LEN} bytes, {f:{c:k.rs::LEN} - 1025|words}, "
                        "{f:log2({len:k.rs::W})} and {f:log2(7776)}, \"{b:k.rs::TAG}\"")
        expect(t == "1 029 bytes, four, 10 and 12.9, \"a v1 b\"",
               f"constants, lengths, arithmetic, words and byte strings fill a paragraph: {t}")
        expect(sorted(code.lines)[0] == "val b:k.rs::TAG a v1 b", "each value read is a lock line")
        for bad in ("{c:k.rs::NONE}", "{f:{c:k.rs::LEN}|lots}", "{c:k.rs::LEN|words}",
                    "{planned:3}", "{q:k.rs::LEN}", "{c:k.rs::LEN"):
            try:
                render_text(Code(d), bad)
                expect(False, f"{bad} fails")
            except Unresolved:
                expect(True, f"{bad} fails")
        expect(render_text(Code(d), "{planned:32}", planned=True) == "32",
               "a number of work not built, in a planned paragraph")
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
