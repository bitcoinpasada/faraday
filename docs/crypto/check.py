#!/usr/bin/env python3
"""The manifest's checker (docs/CRYPTO-LEARN.md §2 to §7, §10.2).

    check.py check [--inventory FILE] [--all]
        coverage, schema, truth of what an entry names, fingerprints
    check.py review ID [--inventory FILE]
        the only writer of a fingerprint (§5.3): `just crypto-review ID`
    check.py changes TAG [--inventory FILE]
        the actions whose fingerprint changed since a tag: `just crypto-changes TAG`
    check.py --self-test
        the checker over a small fixture manifest and inventory

The inventory is `crypto-inventory`'s JSON lines (tools/costs-lint,
docs/CRYPTO-LEARN.md §4.3), read from FILE or, by default, made by
`cargo run -q --locked -p costs-lint --bin crypto-inventory -- --roots LIST`,
LIST being the roots the manifest names (so a root in a crate that is not
cryptographic is found, with its closure), which costs one read and parse
of every source file. Standard library only
(`tomllib`, Python 3.11).

Checked places (§3.2): docs/AUDIT.md §1's and §4's crate rows and §1's "written here"
claims against `implementation.written_here`,
every `Reveal` variant named by an action and the count in words (AUDIT, KEYS,
CLAUDE.md), docs/COMPUTE.md §2.3's roots, the Tools tiles' tags, and the
ratchet in tools/lint-crypto.counts. A tile's tag and COMPUTE's table are read
from their text; a function that takes `&Compute` counting a `Work` kind is not
checked (the inventory does not record parameters).

The screens list (§9.2) is checked by `gen_how.screens_check`; the Learn pages a screen
opens (§9.1) by a test in faraday-core.
The generated places of §3.1 are written by `docs/crypto/gen.py`, which review
runs (`regenerate`); `gen.py check` holds them.

    check.py inventory
        the inventory the manifest's roots ask for, to a file for --inventory
"""

import contextlib
import datetime
import hashlib
import io
import json
import pathlib
import re
import subprocess
import sys
import tempfile
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[2]
MANIFEST = "docs/crypto/actions.toml"
LOCK = "docs/crypto/actions.lock"
GEN = "docs/crypto/gen.py"
LOCK_HEAD = ("# Written by `just crypto-review`, never by hand (docs/CRYPTO-LEARN.md §5):\n"
             "# per action, the parts its fingerprint is the hash of.\n")

STATUSES = {"Deployed", "Complete", "Draft", "Closed", "Informational", "Convention",
            "Proposed Standard", "Internet Standard", "Best Current Practice",
            "Faraday's own", "Not published", "Status not checked"}
KINDS = {"Conservative", "Neutral", "Novel"}
# The cfg a test may sit under and still be built by `just test` on Linux.
CFG_OK = {"cfg(test)", 'cfg(target_pointer_width = "64")', "cfg(unix)",
          'cfg(target_os = "linux")', 'cfg(target_arch = "x86_64")'}

TOP_KEYS = {"schema", "standard", "row", "action", "not_crypto", "vector_no_action", "hold",
            "tag_no_standard", "construction", "name_with_digits", "open_item"}
SCHEMA = 2
# A construction (§18): how a format several actions share is built, drawn in Learn.
CONSTRUCTION_KEYS = {"id", "title", "learn", "actions", "section", "review"}
SECTION_KEYS = {"heading", "para"}
PARA_KEYS = {"text", "table", "words", "planned", "until"}
OPEN_ITEM_KEYS = {"spec", "names"}
NAME_DIGITS_KEYS = {"name", "pattern", "reason"}
HOLD_KEYS = {"action", "check", "case", "decision"}
COUNTS = "tools/lint-crypto.counts"
STANDARD_KEYS = {"name", "status", "checked", "source"}
# A standard may also list `tags`: the spellings a Tools tile names it by (catalog.rs).
STANDARD_OPTIONAL = {"tags"}
# A row describes its own actions (theirs `row` is its id), or other rows' actions it `uses`,
# or none (`no_action`: work not built, or work that computes nothing of its own).
ROW_KEYS = {"id", "section", "what", "completeness", "standard_note", "uses", "no_action",
            "standard", "verified", "own"}
ROW_TEXTS = ("standard", "verified", "own")
STARTER_PAGES = {"write", "confirm", "writing", "done", "start"}
ACTION_KEYS = {"id", "name", "screens_not", "action", "row", "standards", "learn", "screens", "functions", "code",
               "novel_rank", "implementation", "checked", "secrets", "limits", "own", "review"}
IMPL_KEYS = {"crate", "libraries", "adds", "written_here"}
CHECKED_KEYS = {"vectors", "tests", "crosscheck", "not_crosschecked", "selftest", "fuzz",
                "fuzz_none", "other"}
SECRETS_KEYS = {"reveal", "constant_time", "wiped"}
OWN_KEYS = {"text", "kind", "decision"}
REVIEW_KEYS = {"places", "fingerprint", "reviewed"}


def sha(data):
    if isinstance(data, str):
        data = data.encode()
    return hashlib.sha256(data).hexdigest()


class Errors:
    """Failures, each with a category and, where it belongs to one, an action."""

    def __init__(self):
        self.items = []

    def add(self, cat, msg, action=None):
        self.items.append((cat, action, msg))

    def of(self, action):
        return [e for e in self.items if e[1] == action]


# ---------------------------------------------------------------- inventory

class Inventory:
    def __init__(self, lines):
        self.fns = {}
        self.tests = []
        self.sites = []
        for n, line in enumerate(lines, 1):
            line = line.strip()
            if not line:
                continue
            try:
                r = json.loads(line)
            except ValueError as e:
                raise SystemExit(f"inventory line {n}: not JSON: {e}")
            if r.get("kind") == "fn":
                self.fns[r["id"]] = r
            elif r.get("kind") == "test":
                self.tests.append(r)
            elif r.get("kind") == "call_site":
                self.sites.append(r)
        self.by_hash = {}
        for r in self.fns.values():
            self.by_hash.setdefault(r["hash"], []).append(r["id"])
        self.cryptographic = {i for i, r in self.fns.items() if r["rules"]}

    def find_test(self, spec):
        """The test `file::name` names: the inventory's name is module-qualified
        (`tests::name`), so the last segments may stand for it, if only one fits."""
        file, _, name = spec.partition("::")
        hits = [t for t in self.tests if t["file"] == file and t["name"] == name]
        if not hits:
            hits = [t for t in self.tests
                    if t["file"] == file and t["name"].endswith("::" + name)]
        return hits[0] if len(hits) == 1 else None


def manifest_roots(text):
    """Every function the manifest names as a root (an action's `functions`) or as not
    cryptographic: the inventory prints these, and what they reach, in any crate (§4.1)."""
    try:
        data = tomllib.loads(text)
    except tomllib.TOMLDecodeError:
        return []
    roots = []
    for a in data.get("action", []):
        if isinstance(a, dict):
            roots.extend(f for f in as_list(a.get("functions")) if isinstance(f, str))
    for n in data.get("not_crypto", []):
        if isinstance(n, dict) and isinstance(n.get("function"), str):
            roots.append(n["function"])
    return sorted(set(roots))


def cargo_inventory(root, roots=()):
    with tempfile.TemporaryDirectory() as d:
        listing = pathlib.Path(d) / "roots.txt"
        listing.write_text("".join(r + "\n" for r in roots))
        try:
            p = subprocess.run(["cargo", "run", "-q", "--locked", "-p", "costs-lint", "--bin",
                                "crypto-inventory", "--", "--roots", str(listing)],
                               cwd=root, capture_output=True, text=True)
        except FileNotFoundError:
            raise SystemExit("check.py: no cargo here; give --inventory FILE or run it where cargo is")
    if p.returncode != 0:
        sys.stderr.write(p.stderr)
        raise SystemExit("check.py: crypto-inventory failed (a source file did not parse?)")
    return p.stdout.splitlines()


def read_inventory(root, path):
    if path:
        text = sys.stdin.read() if path == "-" else pathlib.Path(path).read_text()
        return Inventory(text.splitlines())
    try:
        manifest = (pathlib.Path(root) / MANIFEST).read_text()
    except OSError:
        manifest = ""
    return Inventory(cargo_inventory(root, manifest_roots(manifest)))


# ------------------------------------------------------------------- lock

def parse_lock(text):
    """{id: [lines]}; the lines as written."""
    out, cur = {}, None
    for line in text.splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        m = re.fullmatch(r"\[([a-z0-9-]+)\]", line)
        if m:
            cur = out.setdefault(m.group(1), [])
        elif cur is not None:
            cur.append(line)
    return out


def fp_line(line):
    """A lock line as the fingerprint reads it: a function without its path."""
    if line.startswith("fn "):
        return " ".join(line.split()[:2])
    return line


def fingerprint_of(lines):
    return sha("\n".join(sorted(fp_line(x) for x in lines)) + "\n")


def lock_text(lock):
    out = [LOCK_HEAD]
    for i in sorted(lock):
        out.append(f"\n[{i}]\n")
        out.extend(x + "\n" for x in sorted(lock[i]))
    return "".join(out)


# ---------------------------------------------------------------- context

class Ctx:
    def __init__(self, root, inv, manifest_text=None, lock_text_=None):
        self.root = pathlib.Path(root)
        self.inv = inv
        self.errors = Errors()
        self.notes = []
        self.text = manifest_text if manifest_text is not None else self.read(MANIFEST)
        self.lock = parse_lock(lock_text_ if lock_text_ is not None else self.read(LOCK))
        self.lock_raw = lock_text_ if lock_text_ is not None else self.read(LOCK)
        try:
            self.data = tomllib.loads(self.text)
        except tomllib.TOMLDecodeError as e:
            raise SystemExit(f"{MANIFEST}: not TOML: {e}")
        self.actions = self.data.get("action", [])
        self.by_id = {a.get("id"): a for a in self.actions if isinstance(a, dict)}
        self._cargo = None
        self.analyse()

    def read(self, rel):
        try:
            return (self.root / rel).read_text()
        except OSError:
            return ""

    def exists(self, rel):
        return (self.root / rel).exists()

    # --- coverage, closures, uses

    def analyse(self):
        inv = self.inv
        self.owner = {}                 # root id -> [action ids]
        for a in self.actions:
            for f in as_list(a.get("functions")):
                self.owner.setdefault(f, []).append(a.get("id"))
        self.closure = {}
        self.uses = {}
        for a in self.actions:
            i = a.get("id")
            roots = [f for f in as_list(a.get("functions")) if f in inv.fns]
            others = {f for f, o in self.owner.items() if any(x != i for x in o)} - set(roots)
            seen, stack, used = set(), list(roots), set()
            while stack:
                f = stack.pop()
                if f in seen:
                    continue
                seen.add(f)
                r = inv.fns[f]
                for c in r["callees"]:
                    if c in others:
                        used.update(x for x in self.owner[c] if x != i)
                    elif c in inv.fns:
                        stack.append(c)
                for c in r["cross"]:
                    for t in self.cross_roots(c):
                        used.update(x for x in self.owner.get(t, []) if x != i and t not in roots)
            self.closure[i] = seen
            self.uses[i] = used
        self.not_crypto = {n.get("function") for n in self.data.get("not_crypto", [])
                           if isinstance(n, dict)}

    def cross_roots(self, call):
        """The action roots a cross-crate call `crate::path::name` can name (§4.4)."""
        crate, _, rest = call.partition("::")
        segs = rest.split("::")
        out = []
        for f in self.owner:
            r = self.inv.fns.get(f)
            if not r or r["crate"] != crate:
                continue
            parts = r["name"].split("::")
            if parts[-1] != segs[-1]:
                continue
            ty = parts[-2] if len(parts) > 1 else None
            prev = segs[-2] if len(segs) > 1 else None
            if prev is None or ty is None or ty == prev:
                out.append(f)
        return out

    def trans_uses(self, i):
        seen, stack = set(), list(self.uses.get(i, ()))
        while stack:
            x = stack.pop()
            if x not in seen and x != i:
                seen.add(x)
                stack.extend(self.uses.get(x, ()))
        return seen

    # --- the parts of a fingerprint (§5.2)

    def cargo_lock(self):
        if self._cargo is None:
            try:
                d = tomllib.loads(self.read("Cargo.lock"))
            except tomllib.TOMLDecodeError:
                d = {}
            self._cargo = {}
            for p in d.get("package", []):
                self._cargo.setdefault(p["name"], []).append(
                    (p.get("version", ""), p.get("checksum", "")))
        return self._cargo

    def vector_hash(self, name):
        base = self.root / "tools" / "vectors" / name
        if not base.is_dir():
            return None
        h = hashlib.sha256()
        for p in sorted(base.rglob("*")):
            if p.is_file() and "__pycache__" not in p.parts:
                h.update(f"{p.relative_to(base).as_posix()}\0{sha(p.read_bytes())}\n".encode())
        return h.hexdigest()

    def own_lines(self, a):
        """The lock lines of an action's own parts: functions, libraries, vectors, area."""
        lines = []
        for f in sorted(self.closure.get(a.get("id"), ())):
            lines.append(f"fn {self.inv.fns[f]['hash']} {f}")
        impl = a.get("implementation", {})
        for lib in as_list(impl.get("libraries")):
            for ver, ck in sorted(self.cargo_lock().get(lib, [])):
                lines.append(f"lib {lib} {ver} {ck}")
        chk = a.get("checked", {})
        for v in as_list(chk.get("vectors")):
            h = self.vector_hash(v)
            if h:
                lines.append(f"vec {v} {h}")
        area = chk.get("crosscheck")
        if area:
            p = self.root / "tools" / "crosscheck" / f"area_{area}.py"
            if p.is_file():
                lines.append(f"area {area} {sha(p.read_bytes())}")
        return lines

    def lines(self, a):
        own = self.own_lines(a)
        for u in sorted(self.trans_uses(a.get("id"))):
            ua = self.by_id.get(u)
            if ua is not None:
                own.append(f"use {u} {fingerprint_of(self.own_lines(ua))}")
        return own

    def fingerprint(self, a):
        return fingerprint_of(self.lines(a))


def as_list(v):
    return v if isinstance(v, list) else ([] if v is None else [v])


# ------------------------------------------------------------ schema, truth

def str_ok(v):
    return isinstance(v, str) and v.strip() != ""


def unknown(e, where, d, allowed, who=None):
    for k in d:
        if k not in allowed:
            e.add("schema", f"{where}: unknown key `{k}`", who)


def check_schema(c):
    e = c.errors
    d = c.data
    if d.get("schema") != SCHEMA:
        e.add("schema", f"{MANIFEST}: `schema = {SCHEMA}` is required")
    unknown(e, MANIFEST, d, TOP_KEYS)
    for name, s in d.get("standard", {}).items():
        w = f"standard.{name}"
        if not isinstance(s, dict):
            e.add("schema", f"{w}: not a table")
            continue
        unknown(e, w, s, STANDARD_KEYS | STANDARD_OPTIONAL)
        if "tags" in s and not (isinstance(s["tags"], list) and all(str_ok(x) for x in s["tags"])):
            e.add("schema", f"{w}: `tags` is not a list of strings")
        for k in STANDARD_KEYS:
            if not str_ok(s.get(k)):
                e.add("schema", f"{w}: `{k}` is missing or empty")
        if str_ok(s.get("status")) and s["status"].split(" (")[0] not in STATUSES:
            e.add("schema", f"{w}: status `{s['status']}` is not one of {sorted(STATUSES)}")
        if str_ok(s.get("checked")) and not re.fullmatch(r"\d{4}-\d{2}-\d{2}", s["checked"]):
            e.add("schema", f"{w}: `checked` is not a date")
    rows = set()
    for r in d.get("row", []):
        w = f"row {r.get('id')}"
        unknown(e, w, r, ROW_KEYS)
        for k in ("id", "what", "completeness"):
            if not str_ok(r.get(k)):
                e.add("schema", f"{w}: `{k}` is missing or empty")
        if r.get("section") not in (2, 3, 4, 5):
            e.add("schema", f"{w}: `section` is 2 to 5")
        if r.get("id") in rows:
            e.add("schema", f"{w}: id twice")
        rows.add(r.get("id"))
        for k in ("standard_note", "no_action") + ROW_TEXTS:
            if k in r and not str_ok(r[k]):
                e.add("schema", f"{w}: `{k}` is empty")
        if "uses" in r and not (isinstance(r["uses"], list) and r["uses"]
                                and all(str_ok(x) for x in r["uses"])):
            e.add("schema", f"{w}: `uses` is not a list of action ids")
    ids = set()
    for a in c.actions:
        i = a.get("id")
        if not (str_ok(i) and re.fullmatch(r"[a-z0-9]+(-[a-z0-9]+)*", i)):
            e.add("schema", f"action id `{i}` is not lower-case words with hyphens")
            continue
        if i in ids:
            e.add("schema", "id twice", i)
        ids.add(i)
        schema_action(c, a, i)
    used_rows = {a.get("row") for a in c.actions}
    for r in d.get("row", []):
        w, own_acts = f"row {r.get('id')}", r.get("id") in used_rows
        kinds = [k for k in ("uses", "no_action") if k in r]
        if len(kinds) > 1 or (kinds and own_acts):
            e.add("schema", f"{w}: a row has its own actions, or `uses` others', or "
                  "`no_action`: one of the three")
        elif not kinds and not own_acts:
            e.add("schema", f"{w} has no action: give an action this row, `uses` with the "
                  "actions it describes, or `no_action` with its reason")
        elif not kinds and any(k in r for k in ROW_TEXTS):
            e.add("schema", f"{w}: a row with its own actions takes its cells from them; "
                  f"`{'`, `'.join(k for k in ROW_TEXTS if k in r)}` is for a row that `uses` "
                  "others' actions or has none")
        if "no_action" in r and not all(str_ok(r.get(k)) for k in ROW_TEXTS):
            e.add("schema", f"{w}: a row with no action gives `standard`, `verified` and `own`")
        for u in as_list(r.get("uses")):
            if u not in ids:
                e.add("schema", f"{w}: `uses` names {u}, which is not an action")
    for n in d.get("not_crypto", []):
        unknown(e, "not_crypto", n, {"function", "reason"})
        if not str_ok(n.get("function")) or not str_ok(n.get("reason")):
            e.add("schema", f"not_crypto {n.get('function')}: `function` and `reason` are required")
    for h in d.get("hold", []):
        unknown(e, "hold", h, HOLD_KEYS)
        if not all(str_ok(h.get(k)) for k in HOLD_KEYS):
            e.add("schema", f"hold {h.get('action')}: `action`, `check`, `case` and `decision` "
                  "are all required")
        elif h["action"] not in ids:
            e.add("schema", f"hold: action `{h['action']}` is not an action")
        elif not (re.fullmatch(r"F\d+", h["decision"]) and c.exists(f"docs/decisions/{h['decision']}.md")):
            e.add("schema", f"hold {h['action']}: decision {h['decision']} is not a file "
                  "under docs/decisions/")
    for n in d.get("tag_no_standard", []):
        unknown(e, "tag_no_standard", n, {"tag", "reason"})
        if not str_ok(n.get("tag")) or not str_ok(n.get("reason")):
            e.add("schema", "tag_no_standard: `tag` and `reason` are required")
    for n in d.get("vector_no_action", []):
        unknown(e, "vector_no_action", n, {"folder", "reason"})
        if not str_ok(n.get("folder")) or not str_ok(n.get("reason")):
            e.add("schema", "vector_no_action: `folder` and `reason` are required")


def schema_action(c, a, i):
    e = c.errors

    def bad(msg):
        e.add("schema", msg, i)

    unknown(e, f"action {i}", a, ACTION_KEYS, i)
    for k in ("name", "action", "row", "learn"):
        if not str_ok(a.get(k)):
            bad(f"`{k}` is missing or empty")
    if str_ok(a.get("name")) and not 2 <= len(a["name"].split()) <= 6:
        bad("`name` is the block's heading, 2 to 6 words")
    sn = a.get("screens_not", [])
    if not isinstance(sn, list):
        bad("`screens_not` is not an array of tables")
        sn = []
    for n in sn:
        unknown(e, f"action {i}.screens_not", n, {"screen", "reason"}, i)
        if not str_ok(n.get("screen")) or not str_ok(n.get("reason")):
            bad("screens_not: `screen` and `reason` are required")
    for k in ("standards", "screens", "functions", "code"):
        if not isinstance(a.get(k), list) or not all(str_ok(x) for x in a[k]):
            bad(f"`{k}` is not a list of strings")
    if not a.get("functions"):
        bad("`functions` is empty")
    for f in as_list(a.get("code")):
        if f not in as_list(a.get("functions")):
            bad(f"`code` names {f}, which is not in `functions`")
    if not a.get("code"):
        bad("`code` is empty")
    if "novel_rank" in a and not isinstance(a["novel_rank"], int):
        bad("`novel_rank` is not a number")
    tables = (("implementation", IMPL_KEYS), ("checked", CHECKED_KEYS),
              ("secrets", SECRETS_KEYS), ("review", REVIEW_KEYS))
    for t, keys in tables:
        v = a.get(t)
        if t == "review" and v is None:
            continue
        if not isinstance(v, dict):
            bad(f"table `{t}` is missing")
            continue
        unknown(e, f"action {i}.{t}", v, keys, i)
    impl, chk, sec = a.get("implementation", {}), a.get("checked", {}), a.get("secrets", {})
    if isinstance(impl, dict):
        for k in ("crate", "adds"):
            if not str_ok(impl.get(k)):
                bad(f"implementation.{k} is missing or empty")
        if not isinstance(impl.get("libraries", []), list):
            bad("implementation.libraries is not a list")
        if "written_here" in impl and not str_ok(impl["written_here"]):
            bad("implementation.written_here is empty")
    if isinstance(chk, dict):
        for k in ("vectors", "tests", "selftest", "fuzz", "other"):
            if k in chk and not (isinstance(chk[k], list) and all(str_ok(x) for x in chk[k])):
                bad(f"checked.{k} is not a list of strings")
        if bool(chk.get("crosscheck")) == bool(chk.get("not_crosschecked")):
            bad("checked: exactly one of `crosscheck` and `not_crosschecked`")
        if bool(chk.get("fuzz")) == bool(chk.get("fuzz_none")):
            bad("checked: exactly one of `fuzz` and `fuzz_none`")
    if isinstance(sec, dict):
        rv = sec.get("reveal")
        if not (rv == "none" or (isinstance(rv, list) and rv and all(str_ok(x) for x in rv))):
            bad('secrets.reveal is "none" or a list of `Reveal::…`')
        for k in ("constant_time", "wiped"):
            if not str_ok(sec.get(k)):
                bad(f"secrets.{k} is missing or empty")
    lim = a.get("limits")
    if not (isinstance(lim, dict) and isinstance(lim.get("lines"), list) and lim["lines"]
            and all(str_ok(x) for x in lim["lines"])):
        bad("limits.lines is missing or empty")
    own = a.get("own", [])
    if not isinstance(own, list):
        bad("`own` is not an array of tables")
        own = []
    for o in own:
        unknown(e, f"action {i}.own", o, OWN_KEYS, i)
        if not str_ok(o.get("text")):
            bad("own: `text` is missing")
        if o.get("kind") not in KINDS:
            bad(f"own: `kind` is one of {sorted(KINDS)}")
    if not a.get("standards") and not own:
        bad("no standard and no own call: a standard, or the own call that says it is Faraday's own")
    rv = a.get("review", {})
    if isinstance(rv, dict):
        if rv.get("reviewed") is not None and not re.fullmatch(r"\d{4}-\d{2}-\d{2}",
                                                               str(rv.get("reviewed"))):
            bad("review.reviewed is not a date")


def enum_variants(text, name):
    m = re.search(rf"pub enum {name}\b[^{{]*\{{(.*?)\n\}}", text, re.S)
    if not m:
        return None
    return set(re.findall(r"^    ([A-Z]\w*)\b", m.group(1), re.M))


def learn_pages(c):
    en = c.read("faraday/faraday-learn/src/pages/en.rs")
    lib = c.read("faraday/faraday-learn/src/lib.rs")
    pages = set(re.findall(r"^    (\w+): Page \{", en, re.M))
    m = re.search(r"const OWN[^=]*=\s*\[(.*?)\n\];", lib, re.S)
    own = set(re.findall(r'"([a-z0-9-]+)",\s*include_str!', m.group(1))) if m else set()
    return pages, own


def fuzz_targets(c):
    out = set()
    for rel in ("fuzz/Cargo.toml", "faraday/fuzz/Cargo.toml"):
        try:
            d = tomllib.loads(c.read(rel))
        except tomllib.TOMLDecodeError:
            continue
        out.update(b.get("name") for b in d.get("bin", []))
    return out


def selftest_checks(c):
    return set(re.findall(r"^fn (\w+)\(_?c: &Compute\) -> bool", c.read(
        "core/faraday-selftest/src/lib.rs"), re.M))


def areas_present(c):
    d = c.root / "tools" / "crosscheck"
    return {p.name[5:-3] for p in d.glob("area_*.py")} if d.is_dir() else set()


def check_truth(c):
    e, inv = c.errors, c.inv
    pages, own_pages = learn_pages(c)
    reveals = enum_variants(c.read("faraday/faraday-keys/src/session.rs"), "Reveal")
    screens = enum_variants(c.read("faraday/faraday-core/src/lib.rs"), "Screen")
    fuzz = fuzz_targets(c)
    selftest = selftest_checks(c)
    areas = areas_present(c)
    standards = c.data.get("standard", {})
    rows = {r.get("id") for r in c.data.get("row", [])}
    for a in c.actions:
        i = a.get("id")
        if not str_ok(i):
            continue
        for f in as_list(a.get("functions")) + as_list(a.get("code")):
            if f not in inv.fns:
                msg = f"{f} is not a function the inventory finds"
                moved = moved_target(c, i, f)
                if moved:
                    msg += f" (moved, body unchanged, to {moved}: run just crypto-review {i})"
                e.add("untrue", msg, i)
        crate = a.get("implementation", {}).get("crate")
        if crate and not (c.root / crate).is_dir():
            e.add("untrue", f"implementation.crate {crate} does not exist", i)
        for lib in as_list(a.get("implementation", {}).get("libraries")):
            if lib not in c.cargo_lock():
                e.add("untrue", f"library {lib} is not in Cargo.lock", i)
        for s in as_list(a.get("standards")):
            if s not in standards:
                e.add("untrue", f"standard {s} has no [standard.\"{s}\"] table", i)
        if a.get("row") not in rows:
            e.add("untrue", f"row {a.get('row')} has no [[row]]", i)
        chk = a.get("checked", {}) if isinstance(a.get("checked"), dict) else {}
        for t in as_list(chk.get("tests")):
            r = inv.find_test(t)
            if r is None:
                e.add("untrue", f"test {t} is not a #[test] function", i)
            elif r["ignored"]:
                e.add("untrue", f"test {t} is #[ignore]d", i)
            elif any(p not in CFG_OK for p in r["cfg"].split(" & ") if p):
                e.add("untrue", f"test {t} sits under {r['cfg']}, which `just test` may not build", i)
        for v in as_list(chk.get("vectors")):
            if not (c.root / "tools" / "vectors" / v).is_dir():
                e.add("untrue", f"vector folder tools/vectors/{v} does not exist", i)
        if chk.get("crosscheck") and chk["crosscheck"] not in areas:
            e.add("untrue", f"cross-check area tools/crosscheck/area_{chk['crosscheck']}.py "
                  "does not exist", i)
        for t in as_list(chk.get("fuzz")):
            if t not in fuzz:
                e.add("untrue", f"fuzz target {t} is not a [[bin]] of fuzz/ or faraday/fuzz/", i)
        for t in as_list(chk.get("selftest")):
            if t not in selftest:
                e.add("untrue", f"self-test check {t} is not in core/faraday-selftest/src/lib.rs", i)
        rv = a.get("secrets", {}).get("reveal") if isinstance(a.get("secrets"), dict) else None
        for r in as_list(rv) if rv != "none" else []:
            m = re.fullmatch(r"Reveal::(\w+)", str(r))
            if not m or reveals is None or m.group(1) not in reveals:
                e.add("untrue", f"reveal {r} is not a variant of Reveal", i)
        lp = a.get("learn")
        if lp and lp != "none":
            if lp.startswith("faraday:"):
                ok = lp[8:] in own_pages
            else:
                ok = lp in pages
            if not ok:
                e.add("untrue", f"Learn page `{lp}` does not exist", i)
        for s in as_list(a.get("screens")) + [n.get("screen") for n in as_list(a.get("screens_not"))]:
            if s.startswith("starter:"):
                ok = s[8:] in STARTER_PAGES
            else:
                ok = screens is not None and s in screens
            if not ok:
                e.add("untrue", f"screen {s} is not a Screen variant or a Starter page", i)
        for n in as_list(a.get("screens_not")):
            if n.get("screen") in as_list(a.get("screens")):
                e.add("untrue", f"screen {n.get('screen')} is in `screens` and `screens_not`", i)
        for o in as_list(a.get("own")):
            d = o.get("decision")
            if d and d != "no decision":
                if not (re.fullmatch(r"F\d+", d) and c.exists(f"docs/decisions/{d}.md")):
                    e.add("untrue", f"decision {d} is not a file under docs/decisions/", i)
        rv = a.get("review", {}) if isinstance(a.get("review"), dict) else {}
        for p in as_list(rv.get("places")):
            path, _, heading = str(p).partition("#")
            if not c.exists(path):
                e.add("untrue", f"review place {path} does not exist", i)
            elif heading and not has_heading(c.read(path), heading):
                e.add("untrue", f"review place {path} has no heading `{heading}`", i)
    for n in c.data.get("not_crypto", []):
        f = n.get("function")
        if f not in inv.fns:
            e.add("untrue", f"not_crypto {f} is not a function the inventory finds")
        elif f in c.owner:
            e.add("untrue", f"not_crypto {f} is also a root of {c.owner[f]}")


def has_heading(text, heading):
    want = heading.strip().lower()
    return any(re.sub(r"^#+\s*", "", ln).strip().lower() == want
               for ln in text.splitlines() if ln.startswith("#"))


def moved_target(c, i, path):
    """The function with the body hash the lock holds for `path`, when one fits."""
    old = None
    for ln in c.lock.get(i, []):
        p = ln.split()
        if p[0] == "fn" and len(p) >= 3 and p[2] == path:
            old = p[1]
    if not old:
        return None
    taken = {f for f in c.owner}
    cands = [x for x in c.inv.by_hash.get(old, []) if x not in taken]
    same = [x for x in cands if x.split("::")[-1] == path.split("::")[-1]]
    pick = same if len(same) == 1 else cands if len(cands) == 1 else []
    return pick[0] if pick else None


# ----------------------------------------------------------------- coverage

def check_coverage(c):
    e = c.errors
    for f, who in sorted(c.owner.items()):
        if len(who) > 1:
            e.add("coverage", f"{f} is the root of {len(who)} actions: {', '.join(who)}")
    covered = set().union(*c.closure.values()) if c.closure else set()
    for f in sorted(c.inv.cryptographic - covered - c.not_crypto):
        r = c.inv.fns[f]
        e.add("uncovered", f"{f} is cryptographic (rule {', '.join(map(str, r['rules']))}) and no "
              "action covers it: add it to an action's functions, or to [[not_crypto]] with its reason")
    named_v = {v for a in c.actions for v in as_list(a.get("checked", {}).get("vectors"))}
    named_v |= {n.get("folder") for n in c.data.get("vector_no_action", [])}
    vdir = c.root / "tools" / "vectors"
    for p in sorted(vdir.iterdir()) if vdir.is_dir() else []:
        if p.is_dir() and p.name not in named_v:
            e.add("unnamed", f"vector folder tools/vectors/{p.name} is named by no action "
                  "and not in [[vector_no_action]]")
    named_a = {a.get("checked", {}).get("crosscheck") for a in c.actions}
    for n in sorted(areas_present(c) - named_a):
        e.add("unnamed", f"cross-check area {n} is named by no action")
    named_s = {s for a in c.actions for s in as_list(a.get("checked", {}).get("selftest"))}
    for n in sorted(selftest_checks(c) - named_s):
        e.add("unnamed", f"self-test check {n} is named by no action")


# ------------------------------------------------------------- fingerprints

def diff_parts(old_lines, new_lines):
    def key(ln):
        p = ln.split()
        return (p[0] + " " + p[2]) if p[0] == "fn" and len(p) >= 3 else (p[0] + " " + p[1])

    old = {key(x): x for x in old_lines}
    new = {key(x): x for x in new_lines}
    out = []
    for k in sorted(set(old) | set(new)):
        if old.get(k) == new.get(k):
            continue
        out.append(("added " if k not in old else "removed " if k not in new else "changed ") + k)
    return out


def check_fingerprints(c):
    e = c.errors
    for a in c.actions:
        i = a.get("id")
        if not str_ok(i):
            continue
        review = a.get("review") if isinstance(a.get("review"), dict) else {}
        stored = review.get("fingerprint")
        locked = c.lock.get(i)
        now = c.lines(a)
        if not stored or locked is None:
            e.add("fingerprint", "never reviewed: no fingerprint or no lock lines; "
                  f"run: just crypto-review {i}", i)
            continue
        if fingerprint_of(locked) != stored:
            e.add("fingerprint", "the fingerprint is not the hash of its lock lines "
                  f"(edited by hand?); run: just crypto-review {i}", i)
        if sorted(map(fp_line, locked)) != sorted(map(fp_line, now)):
            ch = diff_parts(locked, now)
            e.add("fingerprint", f"the code changed since it was reviewed on "
                  f"{review.get('reviewed', '?')}\n    changed: " + "\n             ".join(ch[:12])
                  + (f"\n             … {len(ch) - 12} more" if len(ch) > 12 else "")
                  + f"\n    run: just crypto-review {i}", i)
    for con in constructions(c):
        i = con.get("id")
        if not str_ok(i):
            continue
        review = con.get("review") if isinstance(con.get("review"), dict) else {}
        stored, locked = review.get("fingerprint"), c.lock.get(i)
        now = construction_lines(c, con)
        if not stored or locked is None:
            e.add("fingerprint", "construction never reviewed: no fingerprint or no lock lines; "
                  f"run: just crypto-review {i}", i)
            continue
        if fingerprint_of(locked) != stored:
            e.add("fingerprint", "the fingerprint is not the hash of its lock lines "
                  f"(edited by hand?); run: just crypto-review {i}", i)
        if sorted(locked) != sorted(now):
            ch = diff_parts(locked, now)
            e.add("fingerprint", f"the code the construction reads changed since it was reviewed "
                  f"on {review.get('reviewed', '?')}\n    changed: " + "\n             ".join(ch[:12])
                  + f"\n    run: just crypto-review {i}", i)
    names = {con.get("id") for con in constructions(c)}
    for f in c.lock:
        if f not in c.by_id and f not in names:
            e.add("fingerprint", f"{LOCK} holds lines for {f}, which is no action or construction")


def primitive_crates(c):
    m = re.search(r"const PRIMITIVE_CRATES[^=]*=\s*&\[(.*?)\];",
                  c.read("tools/costs-lint/src/inventory.rs"), re.S)
    return set(re.findall(r'"([^"]+)"', m.group(1))) if m else set()


def check_deps(c):
    """A deps page that says `computes` names a crate in the inventory's PRIMITIVE_CRATES (§4.1),
    and a listed crate that has a page says it."""
    prim = primitive_crates(c)
    pages = c.root / "docs" / "deps"
    for p in sorted(pages.glob("*.md")) if pages.is_dir() else []:
        says = re.search(r"^\*\*Cryptography:\*\* computes\s*$", p.read_text(), re.M) is not None
        if says and p.stem not in prim:
            c.errors.add("deps", f"docs/deps/{p.name} says computes, and {p.stem} is not in "
                         "PRIMITIVE_CRATES (tools/costs-lint/src/inventory.rs)")
        if p.stem in prim and not says:
            c.errors.add("deps", f"docs/deps/{p.name}: {p.stem} is in PRIMITIVE_CRATES, and the "
                         "page lacks the line `**Cryptography:** computes`")


# ------------------------------------------------- the places checked (§3.2)

NUMBER_WORDS = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
                "ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen",
                "seventeen", "eighteen", "nineteen", "twenty", "twenty-one", "twenty-two",
                "twenty-three", "twenty-four", "twenty-five", "twenty-six", "twenty-seven",
                "twenty-eight", "twenty-nine", "thirty"]
# Where the count of `Reveal` variants is written in words.
REVEAL_COUNT_FILES = ("docs/AUDIT.md", "docs/KEYS.md", "CLAUDE.md", "README.md",
                      "docs/SECURITY-MODEL.md")
# The Work kinds of COMPUTE §2.3 whose roots an action must cover.
COMPUTE_KINDS = {"WordsParse", "Kdf", "MasterKey", "KeyDerive", "DescriptorParse", "Address",
                 "PsbtParse", "PsbtInspect", "Sign", "QrEncode", "QrDecode", "FileHash"}
# The screen(s) a Tools tile (`Go::…` in faraday-core's catalog.rs) leads to.
TILE_SCREENS = {
    "NewKey": ("KeyGen",), "NewShares": ("KeyGen",), "AddKey": ("KeyGen",),
    "SeedQr": ("Entry",), "Explore": ("Explore",), "Bip85": ("Bip85",),
    "Vanity": ("Vanity",), "Silent": ("Silent",), "Lightning": ("Lightning",),
    "Create": ("Create",), "Restore": ("Restore",), "Backup": ("BackupFlow", "Backup"),
    "Sign": ("Spend",), "NonceCheck": ("Spend",), "Decode": ("Decode",),
    "CheckAddress": ("Files",), "Scan": ("Files",), "SignMessage": ("Message",),
    "CheckMessage": ("CheckMessage",),
    # The vault list's two ways in: Create a vault and Unlock.
    "Vaults": ("Vaults", "CreateVault", "Unlock"), "Codes": ("Codes",),
    "CreateVault": ("CreateVault",), "VaultCategory": ("VaultContents",),
    "Calculator": ("Tools",), "WordList": ("Tools",),
}


def audit_section(text, n):
    m = re.search(rf"^## {n}\..*?(?=^## {n + 1}\.)", text, re.S | re.M)
    return m.group(0) if m else ""


def audit_crate_rows(c, section=1):
    """{crate: the row's 'written here' claim} from the tables of a section of docs/AUDIT.md."""
    rows = {}
    for ln in audit_section(c.read("docs/AUDIT.md"), section).split("\n"):
        m = re.match(r"\| `([a-z0-9-]+)` \|", ln)
        if m:
            cells = ln.split("|")
            what = cells[3] if len(cells) > 3 else ""
            claim = re.search(r"written here", re.sub(r"[Nn]o primitive is written here", "", what),
                              re.I) is not None
            rows[m.group(1)] = claim
    return rows


def package_name(c, crate_dir):
    """The package name of a crate directory (its Cargo.toml), else the directory's name."""
    try:
        return tomllib.loads(c.read(f"{crate_dir}/Cargo.toml"))["package"]["name"]
    except (tomllib.TOMLDecodeError, KeyError):
        return crate_dir.rsplit("/", 1)[-1]


def check_audit_crates(c):
    """AUDIT §1 and §4: a row for every crate an action names, an action for every row of §1,
    and §1's 'written here' agrees with its actions' `implementation.written_here`; a crate of §4
    (it touches cryptographic bytes and computes none) writes nothing here."""
    e = c.errors
    rows, carries = audit_crate_rows(c, 1), audit_crate_rows(c, 4)
    if not rows:
        e.add("audit", "docs/AUDIT.md §1 has no crate rows (| `crate` | … |)")
        return
    named = {}
    for a in c.actions:
        crate = a.get("implementation", {}).get("crate") if isinstance(a.get("implementation"), dict) else None
        if crate:
            named.setdefault(package_name(c, crate), []).append(a)
    for crate, acts in sorted(named.items()):
        ids = ", ".join(a.get("id") for a in acts[:4]) + (", …" if len(acts) > 4 else "")
        here = [a.get("id") for a in acts if a["implementation"].get("written_here")]
        if crate in carries and crate not in rows:
            if here:
                e.add("audit", f"docs/AUDIT.md §4 has `{crate}` computing nothing, and the action(s) "
                      f"{', '.join(here[:4])} say what is written here: move its row to §1, or "
                      "remove implementation.written_here")
            continue
        if crate not in rows:
            e.add("audit", f"docs/AUDIT.md §1 and §4 have no row for `{crate}`, which the actions "
                  f"{ids} name as their implementation: add a row to the table of its tier "
                  f"(or change implementation.crate)")
            continue
        if here and not rows[crate]:
            e.add("audit", f"docs/AUDIT.md §1: the row for `{crate}` does not say 'written here', "
                  f"and the action(s) {', '.join(here[:4])} give implementation.written_here: say in "
                  "the row what is written here, or remove it from the actions")
        if rows[crate] and not here:
            e.add("audit", f"docs/AUDIT.md §1: the row for `{crate}` says 'written here', and no "
                  "action of that crate gives implementation.written_here: change the row, or say "
                  "in the action what is written here")
    for crate in sorted(set(rows) - set(named)):
        e.add("audit", f"docs/AUDIT.md §1 has a row for `{crate}`, and no action's "
              "implementation.crate names it: remove the row, or add the action")


def count_word(n):
    return NUMBER_WORDS[n] if n < len(NUMBER_WORDS) else str(n)


def check_reveals(c):
    """Every Reveal variant is named by an action; the count in words is the variants' count."""
    e = c.errors
    variants = enum_variants(c.read("faraday/faraday-keys/src/session.rs"), "Reveal")
    if variants is None:
        e.add("reveal", "faraday/faraday-keys/src/session.rs has no `pub enum Reveal`")
        return
    named = set()
    for a in c.actions:
        sec = a.get("secrets") if isinstance(a.get("secrets"), dict) else {}
        for r in as_list(sec.get("reveal")):
            m = re.fullmatch(r"Reveal::(\w+)", str(r))
            if m:
                named.add(m.group(1))
    for v in sorted(variants - named):
        e.add("reveal", f"Reveal::{v} is named by no action's secrets.reveal: add "
              f"\"Reveal::{v}\" to the action that reveals it")
    word = count_word(len(variants))
    for rel in REVEAL_COUNT_FILES:
        text = c.read(rel)
        for m in re.finditer(r"\b([A-Za-z][A-Za-z-]*) (?:named (?:reasons|reveals)|`Reveal` variants)",
                             text):
            if m.group(1).lower() != word:
                line = text.count("\n", 0, m.start()) + 1
                e.add("reveal", f"{rel}:{line} says `{m.group(0)}`, and Reveal has {len(variants)} "
                      f"variants: write `{word}`")


def strip_parens(s):
    """The text without its parenthesised prose; backticked spans are kept whole."""
    out, depth, tick = [], 0, False
    for ch in s:
        if ch == "`":
            tick = not tick
        if not tick and ch == "(":
            depth += 1
        elif not tick and ch == ")" and depth:
            depth -= 1
        elif depth == 0:
            out.append(ch)
    return "".join(out)


def compute_roots(c):
    """[(kind-cell, token, crate hint, path tail, bare)] from the table of COMPUTE §2.3."""
    m = re.search(r"^### 2\.3.*?(?=^## )", c.read("docs/COMPUTE.md"), re.S | re.M)
    out = []
    for ln in (m.group(0) if m else "").split("\n"):
        cells = ln.split("|")
        if len(cells) < 4 or not cells[1].strip().startswith("`"):
            continue
        kinds = set(re.findall(r"`(\w+)`", cells[1]))
        roots = strip_parens(re.split(r"Through these", cells[2], maxsplit=1, flags=re.I)[0])
        carry = (None, [])
        for seg in roots.split(";"):
            carry = (carry[0], [])
            for tok in re.findall(r"`([^`]+)`", seg):
                tok = re.sub(r"\(.*", "", tok).strip()
                parts = tok.split("::")
                if "-" in tok and len(parts) == 1:
                    continue                      # a crate's name, not a function
                crate = None
                if parts[0].startswith("faraday_"):
                    crate = parts[0].replace("_", "-")
                    parts = parts[1:]
                if len(parts) == 1:
                    crate = crate or carry[0]
                    tail = carry[1] + parts
                else:
                    tail = parts
                carry = (crate, tail[:-1])
                out.append((kinds, tok, crate, tail))
    return out


def fns_for_root(c, crate, tail):
    last = tail[-1]
    prev = tail[-2] if len(tail) > 1 else None
    hits = []
    for r in c.inv.fns.values():
        if crate and r["crate"].replace("_", "-") != crate:
            continue
        name = r["name"]
        if prev and prev[:1].isupper():
            ok = name == f"{prev}::{last}" or name.endswith(f"::{prev}::{last}")
        elif prev:
            stem = r["file"].rsplit("/", 1)[-1].removesuffix(".rs")
            parent = r["file"].rsplit("/", 2)[-2] if r["file"].count("/") >= 2 else ""
            ok = (name == last or name.endswith("::" + last)) and prev in (stem, parent)
        else:
            ok = name == last
        if ok:
            hits.append(r["id"])
    return hits


def check_compute(c):
    """COMPUTE §2.3: each root of the listed kinds is a function some action covers."""
    e = c.errors
    roots = compute_roots(c)
    if not roots:
        e.add("compute", "docs/COMPUTE.md §2.3 has no table of roots")
        return
    covered = set().union(*c.closure.values()) if c.closure else set()
    covered |= c.not_crypto
    for kinds, tok, crate, tail in roots:
        want = sorted(kinds & COMPUTE_KINDS)
        if not want:
            continue
        hits = fns_for_root(c, crate, tail)
        kind = "/".join(want)
        if not hits:
            e.add("compute", f"docs/COMPUTE.md §2.3 names `{'::'.join(tail)}` ({kind}), and the inventory "
                  "finds no function of that name: write the root in full, or remove it from the table")
            continue
        for f in hits:
            if f not in covered:
                e.add("compute", f"docs/COMPUTE.md §2.3 root `{'::'.join(tail)}` ({kind}) is {f}, which no "
                      "action covers: add it to an action's functions, or to [[not_crypto]] with its reason")


def tile_tags(c):
    """[(tag, Go variant)] for every tile of the Tools page."""
    text = c.read("faraday/faraday-core/src/catalog.rs")
    out = []
    for m in re.finditer(r"tags:\s*&\[(.*?)\],\s*go:\s*Go::(\w+)", text, re.S):
        for t in re.findall(r'"([^"]*)"', m.group(1)):
            out.append((t, m.group(2)))
    return out


def tag_standard(tag, standards):
    m = re.fullmatch(r"(BIP|RFC|SLIP)[- ]?0*(\d+)", tag)
    if m:
        for k in standards:
            mk = re.fullmatch(r"(BIP|RFC|SLIP)[- ]?0*(\d+)", k)
            if mk and mk.groups() == m.groups():
                return k
        return None

    def n(s):
        return re.sub(r"[^a-z0-9]", "", s.lower())

    for k, v in standards.items():
        if tag in as_list(v.get("tags")):
            return k
    for k, v in standards.items():
        if n(tag) and (n(tag) in n(k) or n(tag) in n(v.get("name", ""))):
            return k
    return None


def check_tiles(c):
    """Each tag on a Tools tile is a standard of an action whose screens include the tile's."""
    e = c.errors
    tags = tile_tags(c)
    if not tags:
        e.add("tiles", "faraday/faraday-core/src/catalog.rs has no tile with tags")
        return
    standards = c.data.get("standard", {})
    no_standard = {n.get("tag") for n in c.data.get("tag_no_standard", [])}
    seen = set()
    for tag, go in tags:
        if tag in no_standard:
            continue
        if (tag, go) in seen:
            continue
        seen.add((tag, go))
        screens = TILE_SCREENS.get(go)
        if screens is None:
            e.add("tiles", f"catalog.rs: a tile leads to Go::{go}, which check.py's TILE_SCREENS "
                  "does not know: add the screens it opens")
            continue
        std = tag_standard(tag, standards)
        if std is None:
            e.add("tiles", f"catalog.rs: the tag \"{tag}\" on a Go::{go} tile is no standard of the "
                  "manifest: remove the tag, or add its [standard] and an action")
            continue
        # A tile is on the Tools page and opens its flow's screen: an action on either backs it.
        want = set(screens) | {"Tools"}
        if not any(std in as_list(a.get("standards")) and want & set(as_list(a.get("screens")))
                   for a in c.actions):
            e.add("tiles", f"catalog.rs: the tag \"{tag}\" ({std}) on a Go::{go} tile: no action "
                  f"with {std} has {' or '.join(sorted(want))} among its screens: remove the "
                  "tag, or back the standard with an action on that screen")


def check_ratchet_line(c, key, n):
    """`key N` of tools/lint-crypto.counts only falls."""
    m = re.search(rf"^{key}\s+(\d+)\s*$", c.read(COUNTS), re.M)
    if not m:
        c.errors.add("ratchet", f"{COUNTS} has no line `{key} N`")
    elif n > int(m.group(1)):
        c.errors.add("ratchet", f"{n} `{key}` entries and {COUNTS} allows {m.group(1)}: have "
                     "the owner raise the count (recorded in DECISIONS)")
    elif n < int(m.group(1)):
        c.notes.append(f"{COUNTS} allows {m.group(1)} `{key}` entries and there are {n}: "
                       f"lower the count to {n}")


def check_ratchet(c):
    """tools/lint-crypto.counts holds the count of [[not_crypto]] entries; it only falls."""
    check_ratchet_line(c, "screens_not",
                       sum(len(as_list(a.get("screens_not"))) for a in c.actions))
    n = len(c.data.get("not_crypto", []))
    m = re.search(r"^not_crypto\s+(\d+)\s*$", c.read(COUNTS), re.M)
    if not m:
        c.errors.add("ratchet", f"{COUNTS} has no line `not_crypto N`")
    elif n > int(m.group(1)):
        c.errors.add("ratchet", f"[[not_crypto]] has {n} entries and {COUNTS} allows {m.group(1)}: "
                     "cover the function with an action, or have the owner raise the count "
                     "(recorded in DECISIONS)")
    elif n < int(m.group(1)):
        c.notes.append(f"{COUNTS} allows {m.group(1)} [[not_crypto]] entries and there are {n}: "
                       f"lower the count to {n}")


def check_screens(c):
    """§9.2: a screen whose code calls an action's root is in its `screens` or `screens_not`."""
    sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
    try:
        import gen_how  # noqa: PLC0415
        found = gen_how.screens_check(c.root, c.data)
    finally:
        sys.path.pop(0)
    for a, s, f in found:
        c.errors.add("screens", f"{s} calls a root of the action ({f}) and is in neither "
                     "`screens` nor `screens_not`", a)


# ------------------------------------------------------- constructions (§18)

_GEN = None


def gen_module():
    """docs/crypto/gen.py, loaded by path (`-I` keeps this directory off sys.path): its
    placeholders and tables are what check holds a construction to."""
    global _GEN
    if _GEN is None:
        import importlib.util
        spec = importlib.util.spec_from_file_location(
            "crypto_gen", pathlib.Path(__file__).resolve().parent / "gen.py")
        _GEN = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(_GEN)
    return _GEN


def constructions(c):
    v = c.data.get("construction", [])
    return [x for x in v if isinstance(x, dict)] if isinstance(v, list) else []


def construction_lines(c, con):
    """A construction's lock lines: every value its placeholders and tables read (§18.6)."""
    return gen_module().render_construction(c.root, con)[2]


def open_line(c, spec):
    m = re.search(r"^\*\*Open:\*\*(.*)$", c.read(spec), re.M)
    return m.group(1) if m else None


def digit_free(c, text):
    """`text` without its placeholders, its standards' keys and the names that hold
    digits ([[name_with_digits]]): a digit left in it was typed."""
    g = gen_module()
    out, last = [], 0
    for s, e_, _ in g.placeholders(text):
        out.append(text[last:s] + " ")
        last = e_
    out.append(text[last:])
    t = "".join(out)
    for k in sorted(c.data.get("standard", {}), key=len, reverse=True):
        t = t.replace(k, " ")
    for n in c.data.get("name_with_digits", []):
        if isinstance(n, dict) and str_ok(n.get("name")):
            t = t.replace(n["name"], " ")
        if isinstance(n, dict) and str_ok(n.get("pattern")):
            t = re.sub(n["pattern"], " ", t)
    return t


def check_constructions(c):
    """§18.6: a construction's schema, the pages and actions it names, its open items,
    that no number is typed, and that every placeholder and table resolves."""
    e, g = c.errors, gen_module()
    v = c.data.get("construction", [])
    if not isinstance(v, list):
        e.add("construction", "`construction` is not an array of tables ([[construction]])")
        return
    items = c.data.get("open_item", {})
    if not isinstance(items, dict):
        e.add("construction", "`open_item` is not a table of items ([open_item.<key>])")
        items = {}
    for k, it in items.items():
        if not isinstance(it, dict):
            e.add("construction", f"open_item.{k}: not a table")
            continue
        unknown(e, f"open_item.{k}", it, OPEN_ITEM_KEYS)
        if not (str_ok(it.get("spec")) and str_ok(it.get("names"))):
            e.add("construction", f"open_item.{k}: `spec` and `names` are required")
            continue
        line = open_line(c, it["spec"])
        if line is None:
            e.add("construction", f"open_item.{k}: {it['spec']} has no **Open:** line, so the item "
                  "is built: rewrite its paragraphs as built, drop their `until` paragraphs and "
                  "the item")
        elif it["names"] not in line:
            e.add("construction", f"open_item.{k}: {it['spec']}'s Open line no longer names "
                  f"\"{it['names']}\", so the item is built: rewrite its `planned` paragraphs as "
                  "built, drop its `until` paragraphs and the item")
    for n in c.data.get("name_with_digits", []):
        if not isinstance(n, dict):
            continue
        unknown(e, "name_with_digits", n, NAME_DIGITS_KEYS)
        if not str_ok(n.get("reason")) or str_ok(n.get("name")) == str_ok(n.get("pattern")):
            e.add("construction", f"name_with_digits {n.get('name') or n.get('pattern')}: a `name` "
                  "or a `pattern`, and its `reason`")
    used = {k: False for k in items}
    pages, own_pages = learn_pages(c)
    seen = set()
    for con in v:
        i = con.get("id") if isinstance(con, dict) else None
        if not (str_ok(i) and re.fullmatch(r"[a-z0-9]+(-[a-z0-9]+)*", i)):
            e.add("construction", f"construction id `{i}` is not lower-case words with hyphens")
            continue
        if i in seen or i in c.by_id:
            e.add("construction", "the id is another construction's or an action's", i)
        seen.add(i)
        unknown(e, f"construction {i}", con, CONSTRUCTION_KEYS, i)
        for k in ("title", "learn"):
            if not str_ok(con.get(k)):
                e.add("construction", f"`{k}` is missing or empty", i)
        lp = str(con.get("learn", ""))
        if lp and not (lp[8:] in own_pages if lp.startswith("faraday:") else lp in pages):
            e.add("construction", f"Learn page `{lp}` does not exist", i)
        acts = con.get("actions")
        if not (isinstance(acts, list) and acts and all(str_ok(a) for a in acts)):
            e.add("construction", "`actions` is not a list of action ids", i)
            acts = []
        for a in acts:
            if a not in c.by_id:
                e.add("construction", f"action {a} does not exist", i)
        secs = con.get("section")
        if not (isinstance(secs, list) and secs):
            e.add("construction", "no [[construction.section]]", i)
            continue
        for s in secs:
            unknown(e, f"construction {i}.section", s, SECTION_KEYS, i)
            h = s.get("heading")
            if not str_ok(h):
                e.add("construction", "a section has no heading", i)
            elif re.search(r"\d", digit_free(c, h)):
                e.add("construction", f"the heading \"{h}\" holds a digit", i)
            paras = s.get("para")
            if not (isinstance(paras, list) and paras):
                e.add("construction", f"section \"{h}\" has no [[construction.section.para]]", i)
                continue
            for p in paras:
                check_para(c, g, i, h, p, items, used)
        try:
            render = g.render_construction(c.root, con)
        except Exception as x:              # a shape the generator does not know
            e.add("construction", f"gen.py cannot render it: {x}", i)
            continue
        for msg in render[1]:
            e.add("construction", msg, i)
    for k, u in used.items():
        if not u:
            e.add("construction", f"open_item.{k} is named by no paragraph's `planned` or `until`")


def check_para(c, g, i, h, p, items, used):
    e = c.errors
    where = f"section \"{h}\""
    unknown(e, f"construction {i}.para", p, PARA_KEYS, i)
    if ("text" in p) == ("table" in p):
        e.add("construction", f"{where}: a paragraph has `text` or `table`, one of the two", i)
        return
    if "planned" in p and "until" in p:
        e.add("construction", f"{where}: a paragraph is `planned` or `until`, not both", i)
    for k in ("planned", "until"):
        if k in p:
            if p[k] not in items:
                e.add("construction", f"{where}: `{k} = \"{p[k]}\"` names no [open_item.{p[k]}]: "
                      "an item of a Spec's Open line", i)
            else:
                used[p[k]] = True
    if "words" in p and p.get("table") != "diceware-bits":
        e.add("construction", f"{where}: `words` is for the table diceware-bits", i)
    if "table" in p:
        if p["table"] not in g.TABLE_KINDS:
            e.add("construction", f"{where}: table `{p['table']}` is not one gen.py writes "
                  f"({', '.join(g.TABLE_KINDS)})", i)
        return
    text = p.get("text")
    if not str_ok(text):
        e.add("construction", f"{where}: an empty paragraph", i)
        return
    if "`" in text:
        e.add("construction", f"{where}: a code name in backticks is not a placeholder: "
              f"\"{text[:60]}…\"", i)
    try:
        rest = digit_free(c, text)
    except g.Unresolved as x:
        e.add("construction", f"{where}: {x}", i)
        return
    m = re.search(r"\S*\d\S*", rest)
    if m:
        e.add("construction", f"{where}: `{m.group(0)}` is a number typed: a placeholder gives it, "
              "or a name that holds digits is in [[name_with_digits]] with its reason", i)


def run_checks(c):
    check_screens(c)
    check_deps(c)
    check_schema(c)
    check_constructions(c)
    check_truth(c)
    check_coverage(c)
    check_fingerprints(c)
    check_audit_crates(c)
    check_reveals(c)
    check_compute(c)
    check_tiles(c)
    check_ratchet(c)
    return c.errors


def report(errors, show_all=False, out=sys.stdout):
    cats = {}
    for cat, a, msg in errors.items:
        cats.setdefault(cat, []).append((a, msg))
    for cat in ("deps", "schema", "construction", "untrue", "coverage", "uncovered", "unnamed",
                "fingerprint", "audit", "reveal", "compute", "tiles", "screens", "ratchet"):
        items = cats.get(cat, [])
        if not items:
            continue
        print(f"{cat}: {len(items)}", file=out)
        for a, msg in items if show_all else items[:15]:
            print(f"  {a + ': ' if a else ''}{msg}", file=out)
        if len(items) > 15 and not show_all:
            print(f"  … {len(items) - 15} more (--all lists them)", file=out)


def cmd_inventory(root):
    """The inventory the manifest's roots ask for, as JSON lines (for --inventory FILE)."""
    try:
        manifest = (pathlib.Path(root) / MANIFEST).read_text()
    except OSError:
        manifest = ""
    sys.stdout.write("\n".join(cargo_inventory(root, manifest_roots(manifest))) + "\n")
    return 0


def cmd_check(root, inv_path, show_all):
    inv = read_inventory(root, inv_path)
    c = Ctx(root, inv)
    run_checks(c)
    report(c.errors, show_all)
    for n in c.notes:
        print(f"note: {n}")
    n_unc = sum(1 for x in c.errors.items if x[0] == "uncovered")
    print(f"{len(c.actions)} actions; {len(inv.cryptographic)} cryptographic functions, "
          f"{n_unc} uncovered; {len(c.errors.items)} failures")
    return 1 if c.errors.items else 0


# ------------------------------------------------------------------- review

def action_block(text, i, table="action"):
    """(start, end) of an action's (or a construction's) lines in the manifest text, as
    line indexes."""
    lines = text.split("\n")
    head = f"[[{table}]]"
    starts = [n for n, ln in enumerate(lines) if ln.strip() == head]
    for s in starts:
        e = len(lines)
        for n in range(s + 1, len(lines)):
            if re.match(rf"\[(?!\[?{table}\b)", lines[n]) or lines[n].strip() == head:
                e = n
                break
        for n in range(s + 1, e):
            if re.fullmatch(rf'id\s*=\s*"{re.escape(i)}"\s*', lines[n]):
                return lines, s, e
    return lines, None, None


def edit_repoint(text, i, mapping):
    lines, s, e = action_block(text, i)
    if s is None:
        return text
    for n in range(s, e):
        for old, new in mapping.items():
            lines[n] = lines[n].replace(f'"{old}"', f'"{new}"')
    return "\n".join(lines)


def edit_stamp(text, i, fp, date, table="action"):
    lines, s, e = action_block(text, i, table)
    if s is None:
        raise SystemExit(f"{MANIFEST}: no [[{table}]] with id \"{i}\"")
    new = [f'fingerprint = "{fp}"', f'reviewed = "{date}"']
    head = next((n for n in range(s, e) if lines[n].strip() == f"[{table}.review]"), None)
    if head is None:
        end = e
        while end > s + 1 and not lines[end - 1].strip():
            end -= 1
        lines[end:end] = ["", f"[{table}.review]"] + new
    else:
        stop = next((n for n in range(head + 1, e) if lines[n].startswith("[")), e)
        body = [ln for ln in lines[head + 1:stop]
                if not re.match(r"(fingerprint|reviewed)\s*=", ln)]
        lines[head + 1:stop] = new + body
    return "\n".join(lines)


def regenerate(root):
    """The generated places of §3.1, from the manifest as it now stands (`gen.py write`)."""
    gen = pathlib.Path(root) / GEN
    if not gen.exists():
        print(f"generated places: {GEN} is not built yet; nothing regenerated")
        return True
    p = subprocess.run([sys.executable, "-I", "-B", str(gen), "write"], cwd=root)
    return p.returncode == 0


def stamp_ok(root):
    s = pathlib.Path(root) / "tools" / "crosscheck" / "stamp.py"
    if not s.exists():
        return True
    p = subprocess.run([sys.executable, "-I", "-B", str(s), "check"], cwd=root,
                       capture_output=True, text=True)
    if p.returncode != 0:
        sys.stdout.write(p.stdout + p.stderr)
    return p.returncode == 0


def git(root, *args):
    try:
        p = subprocess.run(["git", *args], cwd=root, capture_output=True, text=True)
        return p.stdout if p.returncode == 0 else ""
    except OSError:
        return ""


def touchpoints(c, a):
    i = a["id"]
    out = [f"Learn page: {a.get('learn')}; screens: {', '.join(as_list(a.get('screens'))) or 'none'}",
           f"STANDARDS row: {a.get('row')} (docs/STANDARDS.md); standards: "
           f"{', '.join(as_list(a.get('standards'))) or 'none'}",
           f"AUDIT §1 crate row and §3 line for {a.get('implementation', {}).get('crate')} "
           "(docs/AUDIT.md)"]
    sec = a.get("secrets", {})
    out.append(f"reveal: {sec.get('reveal')} (docs/KEYS.md, docs/AUDIT.md §2)")
    out.append("COMPUTE kinds: the Work kinds of the action's functions (docs/COMPUTE.md §2.3)")
    chk = a.get("checked", {})
    out.append(f"cross-check area: {chk.get('crosscheck') or chk.get('not_crosschecked')}; "
               f"vectors: {', '.join(as_list(chk.get('vectors'))) or 'none'}; "
               f"fuzz: {', '.join(as_list(chk.get('fuzz'))) or chk.get('fuzz_none')}; "
               f"self-test: {', '.join(as_list(chk.get('selftest'))) or 'none'}")
    out.append("Tools tiles' tags (faraday/faraday-core/src/catalog.rs)")
    for p in as_list(a.get("review", {}).get("places")):
        out.append(f"named for review: {p}")
    for u in sorted(u for u, s in c.uses.items() if i in c.trans_uses(u) or i in s):
        out.append(f"used by {u}: fails until it is reviewed in turn")
    return out


def cmd_review_construction(root, i):
    """Stamp a construction (§18.6): what its placeholders and tables read is its
    fingerprint, and reading it needs no inventory."""
    c = Ctx(root, Inventory([]))
    con = next(x for x in constructions(c) if x.get("id") == i)
    check_constructions(c)
    mine = [x for x in c.errors.items if x[0] == "construction" and x[1] in (i, None)]
    locked, now = c.lock.get(i), construction_lines(c, con)
    review = con.get("review") if isinstance(con.get("review"), dict) else {}
    if (locked is not None and review.get("fingerprint") == fingerprint_of(locked)
            and sorted(locked) == sorted(now) and not mine):
        print(f"{i}: nothing changed since the last review")
        return 0
    lines, s, e = action_block(c.text, i, "construction")
    print(f"== construction {i}")
    print("\n".join(lines[s:e]).rstrip())
    print("\n== what it reads from the code")
    for x in (diff_parts(locked, now) if locked is not None else now):
        print(f"  {x}")
    print(f"\n== touchpoints: Learn page {con.get('learn')}; actions "
          f"{', '.join(as_list(con.get('actions')))}; the website's /cryptography#{i}\n")
    if mine:
        print("review refuses; nothing was written:")
        for cat, _, msg in mine:
            print(f"  [{cat}] {msg}")
        return 1
    if not regenerate(root):
        print("review refuses; nothing was written: the generator failed")
        return 1
    lock = dict(c.lock)
    lock[i] = sorted(now)
    text = edit_stamp(c.text, i, fingerprint_of(now), datetime.date.today().isoformat(),
                      "construction")
    tomllib.loads(text)
    (root / MANIFEST).write_text(text)
    (root / LOCK).write_text(lock_text(lock))
    print(f"{i}: reviewed; fingerprint {fingerprint_of(now)[:12]}")
    return 0


def cmd_review(root, i, inv_path, inv=None):
    root = pathlib.Path(root)
    try:
        cons = {x.get("id") for x in tomllib.loads((root / MANIFEST).read_text())
                .get("construction", []) if isinstance(x, dict)}
    except (OSError, tomllib.TOMLDecodeError):
        cons = set()
    if i in cons:
        return cmd_review_construction(root, i)
    inv = inv or read_inventory(root, inv_path)
    c = Ctx(root, inv)
    if i not in c.by_id:
        print(f"review: no action {i} in {MANIFEST}")
        return 1
    manifest, mapping = c.text, {}
    a = c.by_id[i]
    for f in as_list(a.get("functions")):
        if f not in inv.fns:
            t = moved_target(c, i, f)
            if t:
                mapping[f] = t
    if mapping:
        manifest = edit_repoint(manifest, i, mapping)
        c = Ctx(root, inv, manifest, c.lock_raw)
        a = c.by_id[i]
        for old, new in mapping.items():
            print(f"{i}: moved, body unchanged: {old} -> {new}")
    check_schema(c)
    check_truth(c)
    check_coverage(c)
    mine = [x for x in c.errors.items if x[1] in (i, None) and x[0] in ("schema", "untrue", "coverage")]
    mine = [x for x in mine if x[0] != "coverage" or i in x[2]]
    locked = c.lock.get(i)
    now = c.lines(a)
    review = a.get("review") if isinstance(a.get("review"), dict) else {}
    clean = (locked is not None and review.get("fingerprint") == fingerprint_of(locked)
             and sorted(map(fp_line, locked)) == sorted(map(fp_line, now)))
    # Only the cheap part of the diff and the entry are printed when something changed.
    if not clean:
        print_report(c, a, i, locked, now, root)
    if mine:
        print("\nreview refuses; nothing was written:")
        for cat, _, msg in mine:
            print(f"  [{cat}] {msg}")
        return 1
    if not clean and a.get("checked", {}).get("crosscheck") and not stamp_ok(root):
        print("\nreview refuses; nothing was written: tools/crosscheck/stamp.py check fails "
              "(run just crosscheck)")
        return 1
    lock = dict(c.lock)
    if clean:
        if not mapping:
            print(f"{i}: nothing changed since the last review")
            return 0
        lock[i] = sorted(now)
        (root / MANIFEST).write_text(manifest)
        (root / LOCK).write_text(lock_text(lock))
        return 0
    if not regenerate(root):
        print("review refuses; nothing was written: the generator failed")
        return 1
    lock[i] = sorted(now)
    text = edit_stamp(manifest, i, fingerprint_of(now), datetime.date.today().isoformat())
    tomllib.loads(text)
    (root / MANIFEST).write_text(text)
    (root / LOCK).write_text(lock_text(lock))
    print(f"\n{i}: reviewed; fingerprint {fingerprint_of(now)[:12]}")
    return 0


def print_report(c, a, i, locked, now, root):
    lines, s, e = action_block(c.text, i)
    print(f"== entry {i}")
    print("\n".join(lines[s:e]).rstrip())
    print("\n== changed parts")
    if locked is None:
        print("  (no lock lines: never reviewed)")
        ch = []
    else:
        ch = diff_parts(locked, now)
        for x in ch or ["(the fingerprint or the lock was edited; the parts match)"]:
            print(f"  {x}")
    stored = (a.get("review") or {}).get("fingerprint") if isinstance(a.get("review"), dict) else None
    if stored and ch:
        commit = git(root, "log", "-1", "--format=%H", f"-S{stored}", "--", MANIFEST).strip()
        files = sorted({x.split()[1].split("::")[0] for x in ch if x.startswith(("changed fn", "added fn", "removed fn"))})
        for f in files[:6]:
            d = git(root, "diff", commit, "--", f) if commit else ""
            print(f"\n-- {f} since {commit[:10] or 'the last stamp'}")
            print("\n".join(d.splitlines()[:80]))
    print("\n== touchpoints to read before it is stamped")
    for t in touchpoints(c, a):
        print(f"  {t}")
    print()


# ------------------------------------------------------------------ changes

def cmd_changes(root, tag, inv_path):
    old_text = git(root, "show", f"{tag}:{MANIFEST}")
    if not old_text:
        print(f"changes: {MANIFEST} does not exist at {tag}")
        return 1
    old = {a["id"]: (a.get("review") or {}).get("fingerprint")
           for a in tomllib.loads(old_text).get("action", [])}
    new = {a["id"]: (a.get("review") or {}).get("fingerprint")
           for a in tomllib.loads((pathlib.Path(root) / MANIFEST).read_text()).get("action", [])}
    n = 0
    for i in sorted(set(old) | set(new)):
        what = ("added" if i not in old else "removed" if i not in new
                else "changed" if old[i] != new[i] else None)
        if what:
            n += 1
            print(f"{i}: {what}")
    print(f"{n} actions changed since {tag}")
    return 0


# ---------------------------------------------------------------- self-test

def fn_rec(file, name, h, rules=(3,), callees=(), cross=(), crate="c"):
    return {"kind": "fn", "crate": crate, "file": file, "name": name, "id": f"{file}::{name}",
            "line": 1, "public": True, "rules": list(rules), "hash": h,
            "callees": list(callees),
            "cross": list(cross)}


FIXTURE_MANIFEST = '''schema = 2

[standard."BIP 1"]
name = "A standard"
status = "Deployed"
checked = "2026-10-23"
source = "bips master"

[[row]]
id = "row-one"
section = 2
what = "Thing"
completeness = "All"

[[action]]
id = "one"
name = "Do one"
action = "Does one."
row = "row-one"
standards = ["BIP 1"]
learn = "none"
screens = ["Spend"]
functions = ["src/a.rs::f"]
code = ["src/a.rs::f"]

[action.implementation]
crate = "src"
libraries = ["lib1"]
adds = "Nothing."

[action.checked]
vectors = ["v1"]
tests = ["src/a.rs::tests::t_one"]
not_crosschecked = "none here"
selftest = []
fuzz_none = "none here"

[action.secrets]
reveal = "none"
constant_time = "not stated by the library"
wiped = "nothing held"

[action.limits]
lines = ["One."]

# An own call before the review table: the review finds the table after it.
[[action.own]]
text = "Refuses it."
kind = "Conservative"
decision = "no decision"

[action.review]
places = []

[[action]]
id = "two"
name = "Do two"
action = "Does two."
row = "row-one"
standards = ["BIP 1"]
learn = "none"
screens = []
functions = ["src/a.rs::g"]
code = ["src/a.rs::g"]

[action.implementation]
crate = "src"
libraries = []
adds = "Nothing."
written_here = "Two."

[action.checked]
vectors = []
tests = []
not_crosschecked = "none here"
selftest = []
fuzz_none = "none here"

[action.secrets]
reveal = ["Reveal::Words"]
constant_time = "not stated by the library"
wiped = "nothing held"

[action.limits]
lines = ["Two."]
'''

FIXTURE_FILES = {
    "docs/AUDIT.md": "# Audit\n\n## 1. Crates\n\n| Crate | Computes | Written here | Checked |\n"
                     "|---|---|---|---|\n| `src` | things | **written here** | v |\n\n"
                     "## 2. The key boundary\n\nA secret leaves through one named reasons.\n\n## 3. x\n",
    "docs/KEYS.md": "- it leaves through these one named reasons;\n",
    "CLAUDE.md": "its one named reveals (F70)\n",
    "faraday/faraday-keys/src/session.rs": "pub enum Reveal {\n    Words,\n}\n",
    "faraday/faraday-core/src/lib.rs": "pub enum Screen {\n    Spend,\n}\n",
    "docs/COMPUTE.md": "### 2.3 What takes a token\n\n| Kind | Roots |\n|---|---|\n"
                       "| `Sign` | `a::g` |\n| Faraday's own | `plan::map` |\n\n## 3. Who\n",
    "faraday/faraday-core/src/catalog.rs": "pub const TILES: &[Tile] = &[\n    Tile {\n"
                                           "        tags: &[\"BIP-1\"],\n        go: Go::Sign,\n    },\n];\n",
    "tools/lint-crypto.counts": "not_crypto 0\nscreens_not 0\n",
    "faraday/faraday-learn/src/lib.rs": 'const OWN: [(&str, &str); 1] = [\n'
                                        '    ("page", include_str!("page.md")),\n];\n',
    "src/k.rs": "pub const SIDES: usize = 2 + EXTRA;\nconst EXTRA: usize = 4;\n"
                "pub static WORDS: [&str; 3] = [\"a\", \"b\", \"c\"];\n",
    "docs/SPEC.md": "# A spec\n\n**Kind:** Spec\n**Status:** Specified\n"
                    "**Open:** the dice of §2, the coins of §3\n",
}

FIXTURE_CONSTRUCTION = '''
[open_item.dice]
spec = "docs/SPEC.md"
names = "the dice of §2"

[[name_with_digits]]
name = "SHA-256"
reason = "A name."

[[construction]]
id = "dice"
title = "How dice are read"
learn = "faraday:page"
actions = ["one"]

[[construction.section]]
heading = "The dice"

[[construction.section.para]]
text = "A die has {c:src/k.rs::SIDES} sides, {c:src/k.rs::SIDES|words} in words; SHA-256 (BIP 1) of {len:src/k.rs::WORDS} words is {f:log2({c:src/k.rs::SIDES} * 2)} bits."

[[construction.section.para]]
text = "they are thrown {planned:3} times."
planned = "dice"
'''

FIXTURE_CARGO = '''[[package]]
name = "lib1"
version = "1.0.0"
checksum = "abc"
'''


def fixture_inventory(g_hash="g1", f_hash="f1", h_hash="h1", moved=False, extra=False,
                      ignored=False, test=True):
    fl = "src/b.rs" if moved else "src/a.rs"
    recs = [
        fn_rec(fl, "f", f_hash, callees=["src/a.rs::h", "src/a.rs::g"]),
        fn_rec("src/a.rs", "g", g_hash),
        fn_rec("src/a.rs", "h", h_hash, rules=()),
    ]
    if extra:
        recs.append(fn_rec("src/a.rs", "k", "k1"))
    if test:
        recs.append({"kind": "test", "crate": "c", "file": "src/a.rs", "name": "tests::t_one",
                     "line": 9, "ignored": ignored, "cfg": "cfg(test)"})
    return Inventory(json.dumps(r) for r in recs)


def self_test():
    failures = []

    def expect(cond, what):
        print(f"  {'ok  ' if cond else 'FAIL'} {what}")
        if not cond:
            failures.append(what)

    def msgs(c, cat=None):
        return [m for k, _, m in c.errors.items if cat is None or k == cat]

    with tempfile.TemporaryDirectory() as d:
        root = pathlib.Path(d)
        (root / "docs/crypto").mkdir(parents=True)
        (root / "tools/vectors/v1").mkdir(parents=True)
        (root / "tools/vectors/v1/data.txt").write_text("vector\n")
        (root / "Cargo.lock").write_text(FIXTURE_CARGO)
        for rel, text in FIXTURE_FILES.items():
            (root / rel).parent.mkdir(parents=True, exist_ok=True)
            (root / rel).write_text(text)
        (root / "src").mkdir(exist_ok=True)
        (root / MANIFEST).write_text(FIXTURE_MANIFEST)
        (root / LOCK).write_text(LOCK_HEAD)

        def check(inv):
            c = Ctx(root, inv)
            run_checks(c)
            return c

        def review(i, inv):
            buf = io.StringIO()
            with contextlib.redirect_stdout(buf):
                rc = cmd_review(root, i, None, inv)
            return rc, buf.getvalue()

        base = fixture_inventory()
        print("an unreviewed manifest:")
        c = check(base)
        expect(any("never reviewed" in m for m in msgs(c, "fingerprint")), "an action never reviewed fails")
        print("review stamps the actions, used one first:")
        rc2, _ = review("two", base)
        rc1, _ = review("one", base)
        expect(rc1 == 0 and rc2 == 0, "review writes both")
        c = check(base)
        expect(not c.errors.items, f"check passes after review {msgs(c)}")
        expect(sorted(c.uses["one"]) == ["two"], "one uses two (f calls g)")

        print("coverage:")
        c = check(fixture_inventory(extra=True))
        expect(any("src/a.rs::k is cryptographic" in m for m in msgs(c, "uncovered")),
               "an uncovered function fails")

        print("a changed body:")
        c = check(fixture_inventory(g_hash="g2"))
        fails = {a for k, a, _ in c.errors.items if k == "fingerprint"}
        expect(fails == {"one", "two"}, f"a changed g fails two and the action using it: {fails}")
        c = check(fixture_inventory(h_hash="h2"))
        fails = {a for k, a, _ in c.errors.items if k == "fingerprint"}
        expect(fails == {"one"}, f"a changed helper of f fails one only: {fails}")
        c = check(fixture_inventory(f_hash="f2"))
        fails = {a for k, a, _ in c.errors.items if k == "fingerprint"}
        expect(fails == {"one"}, f"a changed f fails one only: {fails}")

        print("a moved function:")
        moved = fixture_inventory(moved=True)
        c = check(moved)
        expect(any("moved, body unchanged" in m for m in msgs(c, "untrue")),
               "a moved root fails as untrue, naming where it went")
        before = (root / MANIFEST).read_text()
        rc, out = review("one", moved)
        after = (root / MANIFEST).read_text()
        expect(rc == 0 and "src/b.rs::f" in after and "src/a.rs::f" not in after,
               "review re-points the entry")
        expect(re.findall(r"fingerprint = .*", before) == re.findall(r"fingerprint = .*", after)
               and re.findall(r"reviewed = .*", before) == re.findall(r"reviewed = .*", after),
               "the fingerprint and the review date did not change")
        c = check(moved)
        expect(not c.errors.items, f"check passes after the re-point {msgs(c)}")

        print("a missing test or vector:")
        c = check(fixture_inventory(test=False, moved=True))
        expect(any("t_one" in m for m in msgs(c, "untrue")), "a missing test fails")
        c = check(fixture_inventory(ignored=True, moved=True))
        expect(any("#[ignore]" in m for m in msgs(c, "untrue")), "an ignored test fails")
        (root / "tools/vectors/v1/data.txt").rename(root / "tools/vectors/v1/other.txt")
        (root / "tools/vectors/v1").rename(root / "tools/vectors/v2")
        c = check(moved)
        expect(any("tools/vectors/v1" in m for m in msgs(c, "untrue")), "a missing vector folder fails")
        (root / "tools/vectors/v2").rename(root / "tools/vectors/v1")
        (root / "tools/vectors/v1/other.txt").rename(root / "tools/vectors/v1/data.txt")

        sample = ('[[action]]\nid = "one"\n\n[[action.own]]\ntext = "t"\n\n'
                  '[action.review]\nplaces = []\n\n[[action]]\nid = "two"\n')
        stamped = edit_stamp(sample, "one", "f1", "2026-01-01")
        try:
            one = tomllib.loads(stamped)["action"][0]["review"]
        except tomllib.TOMLDecodeError:
            one = None
        expect(one == {"fingerprint": "f1", "reviewed": "2026-01-01", "places": []},
               "a stamp goes into the review table after the action's own calls")
        print("review refuses while a named test is missing:")
        gone = fixture_inventory(test=False, moved=True, g_hash="g9")
        m0, l0 = (root / MANIFEST).read_text(), (root / LOCK).read_text()
        rc, out = review("one", gone)
        expect(rc == 1 and "refuses" in out, "review exits 1 and says it refuses")
        expect((root / MANIFEST).read_text() == m0 and (root / LOCK).read_text() == l0,
               "and writes nothing")
        print("the checked places (§3.2), each broken in turn:")

        def broken(rel, old, new, cat, needle, inv=None):
            """Check with `old` replaced by `new` in a file; True if `needle` is among the
            failures of `cat`. The file is put back."""
            path = root / rel
            before = path.read_text()
            assert old in before, (rel, old)
            path.write_text(before.replace(old, new, 1) if old else before + new)
            try:
                c = check(inv or moved)
            finally:
                path.write_text(before)
            return any(needle in m for m in msgs(c, cat)), c

        c = check(moved)
        expect(not c.errors.items and not c.notes, f"the fixture's places agree {msgs(c)}")
        ok, _ = broken("docs/AUDIT.md", "| `src` | things", "| `other` | things", "audit",
                       "no row for `src`")
        expect(ok, "AUDIT §1: a crate an action names with no row fails")
        ok, _ = broken("docs/AUDIT.md", "| v |\n", "| v |\n| `gone` | x | y | z |\n", "audit",
                       "row for `gone`")
        expect(ok, "AUDIT §1: a row no action names fails")
        ok, _ = broken("docs/AUDIT.md", "**written here**", "a crate", "audit",
                       "does not say 'written here'")
        expect(ok, "AUDIT §1: an action that writes here and a row that does not say so fails")
        ok, _ = broken("docs/AUDIT.md", "**written here**", "No primitive is written here", "audit",
                       "does not say 'written here'")
        expect(ok, "AUDIT §1: 'No primitive is written here' is not the claim")
        tail = "\n\n## 2. The key boundary\n\nA secret leaves through one named reasons.\n\n## 3. x\n"
        ok, _ = broken("docs/AUDIT.md", "| `src` | things | **written here** | v |" + tail,
                       "| `other` | things | **written here** | v |" + tail
                       + "\n## 4. Carries\n\n| `src` | carries |\n\n## 5. y\n", "audit",
                       "§4 has `src` computing nothing")
        expect(ok, "AUDIT §4: a crate that computes nothing and an action that writes there fails")
        ok, _ = broken("faraday/faraday-keys/src/session.rs", "    Words,\n", "    Words,\n    Seed,\n",
                       "reveal", "Reveal::Seed is named by no action")
        expect(ok, "a Reveal variant no action names fails")
        ok, _ = broken("docs/KEYS.md", "one named", "fourteen named", "reveal", "write `one`")
        expect(ok, "the count of reveals in words must be the variants' count")
        ok, _ = broken("CLAUDE.md", "one named reveals", "N named reveals", "reveal", "CLAUDE.md:1")
        expect(ok, "and in CLAUDE.md")
        ok, _ = broken("docs/COMPUTE.md", "| Faraday's own", "| `Kdf` | `a::zzz` |\n| Faraday's own",
                       "compute", "names `a::zzz`")
        expect(ok, "COMPUTE §2.3: a root the inventory does not find fails")
        ok, _ = broken("docs/COMPUTE.md", "| Faraday's own", "| `Kdf` | `a::k` |\n| Faraday's own",
                       "compute", "which no action covers", fixture_inventory(extra=True, moved=True))
        expect(ok, "COMPUTE §2.3: a root no action covers fails")
        ok, _ = broken("docs/COMPUTE.md", "| Faraday's own", "| `Other` | `a::zzz` |\n| Faraday's own",
                       "compute", "zzz")
        expect(not ok, "COMPUTE §2.3: a kind outside the list is not checked")
        ok, _ = broken("faraday/faraday-core/src/catalog.rs", '"BIP-1"', '"BIP-1", "BIP-370"', "tiles",
                       "\"BIP-370\"")
        expect(ok, "a Tools tag that is no standard of the manifest fails")
        ok, _ = broken("faraday/faraday-core/src/catalog.rs", '"BIP-1"', '"BIP-1", "sat"', "tiles",
                       '"sat"')
        expect(ok, "a tag that names no standard fails")
        ok, c = broken("docs/crypto/actions.toml", "", '\n[[tag_no_standard]]\ntag = "sat"\n'
                       'reason = "A unit."\n', "tiles", '"sat"')
        expect(not ok, f"and passes once [[tag_no_standard]] gives its reason {msgs(c, 'tiles')}")
        catalog = root / "faraday/faraday-core/src/catalog.rs"
        before = catalog.read_text()
        catalog.write_text(before.replace('"BIP-1"', '"BIP-1", "One"'))
        try:
            ok, c = broken("docs/crypto/actions.toml", 'name = "A standard"', 'name = "A standard"\n'
                           'tags = ["One"]', "tiles", '"One"')
        finally:
            catalog.write_text(before)
        expect(not ok, f"a tag a standard lists in `tags` names it {msgs(c, 'tiles')}")
        ok, _ = broken("faraday/faraday-core/src/catalog.rs", "Go::Sign", "Go::Decode", "tiles",
                       "\"BIP-1\" (BIP 1) on a Go::Decode tile")
        expect(ok, "a Tools tag no action on the tile's screen backs fails")
        ok, _ = broken("faraday/faraday-core/src/catalog.rs", "Go::Sign", "Go::Mystery", "tiles",
                       "TILE_SCREENS")
        expect(ok, "a tile that leads somewhere check.py does not know fails")
        ok, c = broken("docs/crypto/actions.toml", "", '\n[[not_crypto]]\nfunction = "src/a.rs::h"\n'
                       'reason = "A lookup."\n', "ratchet", "allows 0")
        expect(ok, "the not_crypto count rising above tools/lint-crypto.counts fails")
        ok, c = broken("tools/lint-crypto.counts", "not_crypto 0", "not_crypto 3", "ratchet", "allows")
        expect(not ok and any("lower the count to 0" in n for n in c.notes),
               "the count falling is a note to lower the file")
        ok, _ = broken("docs/crypto/actions.toml", "", '\n[[hold]]\naction = "nope"\ncheck = "x"\n'
                       'case = "y"\ndecision = "F1"\n', "schema", "action `nope` is not an action")
        expect(ok, "a [[hold]] names an action")
        row2 = '\n[[row]]\nid = "row-two"\nsection = 3\nwhat = "W"\ncompleteness = "C"\n'
        ok, _ = broken("docs/crypto/actions.toml", "", row2, "schema", "row-two has no action")
        expect(ok, "a row with no action, no `uses` and no `no_action` fails")
        ok, c = broken("docs/crypto/actions.toml", "", row2 + 'no_action = "Not built."\n'
                       'standard = "S"\nverified = "None"\nown = "None"\n', "schema", "row-two")
        expect(not ok, f"a row with no action and its three texts passes {msgs(c, 'schema')}")
        ok, _ = broken("docs/crypto/actions.toml", "", row2 + 'no_action = "Not built."\n', "schema",
                       "gives `standard`, `verified` and `own`")
        expect(ok, "a row with no action gives its three texts")
        ok, c = broken("docs/crypto/actions.toml", "", row2 + 'uses = ["one"]\n', "schema", "row-two")
        expect(not ok, f"a row that uses another row's action passes {msgs(c, 'schema')}")
        ok, _ = broken("docs/crypto/actions.toml", "", row2 + 'uses = ["nope"]\n', "schema",
                       "names nope, which is not an action")
        expect(ok, "a row uses only actions that exist")
        ok, _ = broken("docs/crypto/actions.toml", 'completeness = "All"\n',
                       'completeness = "All"\nown = "Typed."\n', "schema", "takes its cells from them")
        expect(ok, "a row with its own actions types no cell")
        c = check(moved)
        expect(not c.errors.items, f"and the fixture is whole again {msgs(c)}")

        print("a construction (§18.6):")
        m0, l0 = (root / MANIFEST).read_text(), (root / LOCK).read_text()
        (root / MANIFEST).write_text(m0 + FIXTURE_CONSTRUCTION)
        c = check(moved)
        expect(not msgs(c, "construction") and any("construction never reviewed" in m
                                                   for m in msgs(c, "fingerprint")),
               f"a whole construction fails only as never reviewed {msgs(c)}")
        g = gen_module()
        secs, probs, _ = g.render_construction(root, tomllib.loads(
            (root / MANIFEST).read_text())["construction"][0])
        expect(not probs and secs[0][1] == [
            ("para", "A die has 6 sides, six in words; SHA-256 (BIP 1) of 3 words is 3.6 bits."),
            ("para", "Planned, not built: they are thrown 3 times.")],
            f"its placeholders are filled from the code, a planned paragraph led as planned {secs}")
        rc, out = review("dice", moved)
        c = check(moved)
        expect(rc == 0 and not c.errors.items, f"review stamps it, and check passes {msgs(c)}")
        ok, c = broken("src/k.rs", "EXTRA: usize = 4", "EXTRA: usize = 5", "fingerprint",
                       "the code the construction reads changed")
        expect(ok, "a value it reads changed in the code fails it until it is reviewed again")
        cases = [
            ("{c:src/k.rs::SIDES} sides", "{c:src/k.rs::NOPE} sides", "has no constant",
             "a placeholder that does not resolve fails"),
            ("{c:src/k.rs::SIDES} sides", "{c:src/gone.rs::SIDES} sides", "no such file",
             "a placeholder naming a file that does not exist fails"),
            ("{c:src/k.rs::SIDES} sides", "6 sides", "`6` is a number typed",
             "a typed digit fails"),
            ("A die has", "A `die` has", "backticks", "a code name in backticks fails"),
            ('planned = "dice"', 'planned = "coins"', "names no [open_item.coins]",
             "a planned item that is no open item fails"),
            ('names = "the dice of §2"', 'names = "the dice of §9"', "no longer names",
             "an open item no Spec's Open line names fails"),
            ("in words;", "in words, {planned:7};", "only for a planned paragraph",
             "a number of work not built outside a planned paragraph fails"),
            ('learn = "faraday:page"', 'learn = "faraday:none"', "Learn page",
             "a Learn page that does not exist fails"),
            ('actions = ["one"]', 'actions = ["nine"]', "action nine does not exist",
             "an action that does not exist fails"),
            ("[[construction.section.para]]\ntext = \"they",
             "[[construction.section.para]]\ntable = \"plates\"\n\n[[construction.section.para]]\n"
             "text = \"they", "table `plates`", "a table gen.py does not write fails"),
        ]
        for old, new, needle, what in cases:
            ok, c = broken(MANIFEST, old, new, "construction", needle)
            expect(ok, f"{what} {msgs(c, 'construction')[:2] if not ok else ''}")
        (root / MANIFEST).write_text(m0)
        (root / LOCK).write_text(l0)
    print("the roots the inventory is asked for:")
    sample = ('[[action]]\nid = "a"\nfunctions = ["x/src/a.rs::f", "y/src/b.rs::T::g"]\n'
              '[[action]]\nid = "b"\nfunctions = ["x/src/a.rs::f"]\n'
              '[[not_crypto]]\nfunction = "z/src/c.rs::h"\nreason = "r"\n')
    expect(manifest_roots(sample) == ["x/src/a.rs::f", "y/src/b.rs::T::g", "z/src/c.rs::h"],
           "every root and not_crypto function, once, in any crate")
    expect({"Proposed Standard", "Internet Standard", "Best Current Practice"} <= STATUSES,
           "the RFC statuses are accepted")
    if failures:
        print(f"\n{len(failures)} self-test checks failed")
        return 1
    print("\nself-test passed")
    return 0


def main(argv):
    if argv[:1] == ["--self-test"]:
        return self_test()
    inv_path, show_all, rest = None, False, []
    it = iter(argv)
    for x in it:
        if x == "--inventory":
            inv_path = next(it, None)
        elif x == "--all":
            show_all = True
        else:
            rest.append(x)
    if rest[:1] == ["check"] and len(rest) == 1:
        return cmd_check(ROOT, inv_path, show_all)
    if rest[:1] == ["inventory"] and len(rest) == 1:
        return cmd_inventory(ROOT)
    if rest[:1] == ["review"] and len(rest) == 2:
        return cmd_review(ROOT, rest[1], inv_path)
    if rest[:1] == ["changes"] and len(rest) == 2:
        return cmd_changes(ROOT, rest[1], inv_path)
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
