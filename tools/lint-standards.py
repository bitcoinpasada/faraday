#!/usr/bin/env python3
"""docs/STANDARDS.md §0, held (F79, F109, F136): seven checks over the tree
and the crypto inventory, against the lists in `tools/lint-standards.toml`.

    lint-standards.py [--inventory FILE]     the self-test, then the seven checks
    lint-standards.py --self-test            the checks over a small fixture

1. **Cross-check coverage.** A crate whose normal dependencies include a
   primitive crate (the inventory's `PRIMITIVE_CRATES`), or that has a
   function the inventory finds cryptographic, is in
   `tools/crosscheck/stamp.py`'s `CRATES` or under `[[uncovered]]` with its
   reason. In faraday-core, too large to hash whole, the same holds per
   file against `FILES`. An entry that no longer needs to be there fails.
2. **Out of the app.** The functions §0 keeps out of the app (`[[out_of_app]]`:
   an action of `docs/crypto/actions.toml`, whose roots are its
   `functions`, or the ids listed) are reached by no shipped function of a
   crate under `faraday/`, through calls in its crate and calls into other
   crates (the inventory's `callees` and `cross`, resolved as check.py
   resolves them); what no shipped code reaches is not linked into a
   shipped binary. An entry's `unnamed` is a pattern no line of a
   `faraday/` crate's `src/` may match (a type the app must not name).
3. **Holds listed** (F136). Every hold the code defines, read from the
   functions that say which actions are holds (`[[hold_source]]`), is a
   `[[hold]]` naming its decision and its case; a listed hold the code no
   longer has fails.
3a. **Cautions that still sign and notes** (F136's table). Every warning
   kind `core/faraday-psbt/src/inspect.rs` and its `inspect/` files make at `Level::Caution`
   (read from each `warn(` call) and every kind `signer.rs`'s `Kind::level`
   maps to Caution is a `[[caution]]` naming its decision and case; a kind
   whose levels include Info and not Caution is a `[[note]]`. An entry the
   code no longer makes at that level fails. A level or kind is read only
   when every value it can take is a literal `Level::X` or `WarningKind::X`:
   written in the call, or in the `let` nearest before it (a literal, or an
   `if`/`match` whose every branch is one). It stops the lint at its file and
   line on these forms: a call, another variable or a mix as the level or
   kind, a later assignment, a loop, closure or pattern binding of the
   name, a `Warning {` or `Finding {` literal outside `fn warn` and
   `Finding::new`, a `Self {` in an impl of either, a `fn warn` that is not
   one push of `Warning { level, kind, detail }`, a `.level` or `.kind`
   assigned or borrowed `&mut`, an arm of `Kind::level` that is not
   `Kind::X => Level::Y,`, and a `Level::Caution`, `Level::Info`,
   `Warning {` or `warn(` in another file of the crate. A form not listed
   is not seen (below).
3b. **Named ways past that are not a level.** Each `[[way_past]]` (the dev
   image, the Starter's rescue-disk answer, the quiz, the codex32 check
   and own-randomness skips, the signed-amount memory's seal setting)
   names its decision and case and a file and pattern that must still
   match; one removed or moved fails until its entry goes.
4. **STANDARDS §8 held.** Each `[[disagreement]]` names text that must
   still be in a file (the code or the document that disagrees); §8 has
   one bullet for each. A fix that removes the text fails until §8 and the
   entry go.
5. **§0's shape, which the website reads.** §0 opens with its summary of
   two sentences between the `standards-summary` markers, and each point of
   its rule starts with its bold lead (`- **Lead.** text`): the website's
   /standards and /cryptography are generated from them at the pinned
   commit and stop without them.

The inventory is `crypto-inventory --all` (tools/costs-lint): every shipped
function of every crate, `cfg(test)` and the crates no binary ships left
out. Read from FILE, or made with cargo (about 15 s). Standard library
only.

Not seen: a method call whose receiver's type the inventory cannot tell
(§4.4 of docs/CRYPTO-LEARN.md) in check 2; a way past of a kind no pattern
finds (a new flag, a new answer, a new setting that skips a check: not a hold, not
a level) in check 3b, which review rule `standards.no-way-past` covers; the
dangers and blocked warnings (they refuse), and the destructive-act holds
(read as holds in check 3); in check 3a, `Inspection.warnings` and
`Warning`'s fields are `pub`, so code outside faraday-psbt could push or change
a warning unseen (none does), anything after a `#[cfg(test)]` item in a file
read (the text is cut there), and signer.rs's separate `Info` enum (nonce
added, finalized, field stated), which is not read as notes; a level or kind
changed by a route that names neither `.level`, `.kind` nor `&mut` of them
(`mem::replace` of a whole warning, a method on `Warning` that sets one).
"""

import json
import pathlib
import re
import subprocess
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
DATA = "tools/lint-standards.toml"
INVENTORY_RS = "tools/costs-lint/src/inventory.rs"
CORE = "faraday/faraday-core"

sys.path.insert(0, str(ROOT / "tools" / "crosscheck"))
sys.path.insert(0, str(ROOT / "docs" / "crypto"))


def rust_list(text, name):
    """The string items of `const NAME: &[&str] = &[ … ];` in Rust source."""
    m = re.search(r"const " + name + r": &\[&str\] = &\[(.*?)\];", text, re.S)
    if not m:
        raise SystemExit(f"lint-standards: no {name} in {INVENTORY_RS}")
    return re.findall(r'"([^"]+)"', m.group(1))


def as_list(v):
    return v if isinstance(v, list) else ([] if v is None else [v])


# ------------------------------------------------------------------ cargo

def members(root):
    ws = tomllib.loads((root / "Cargo.toml").read_text())
    out = {}
    for m in ws["workspace"]["members"]:
        pkg = tomllib.loads((root / m / "Cargo.toml").read_text())["package"]["name"]
        out[pkg] = m
    return out


def normal_deps(root, member):
    t = tomllib.loads((root / member / "Cargo.toml").read_text())
    tables = [t.get("dependencies", {})]
    tables += [v.get("dependencies", {}) for v in t.get("target", {}).values()]
    for tab in tables:
        for k, v in tab.items():
            yield v.get("package", k) if isinstance(v, dict) else k


# -------------------------------------------------------------- inventory

def inventory_lines(root, path):
    if path:
        return pathlib.Path(path).read_text().splitlines()
    p = subprocess.run(["cargo", "run", "-q", "--locked", "-p", "costs-lint", "--bin",
                        "crypto-inventory", "--", "--all"], cwd=root, capture_output=True, text=True)
    if p.returncode != 0:
        sys.stderr.write(p.stderr)
        raise SystemExit("lint-standards: crypto-inventory failed")
    return p.stdout.splitlines()


def functions(lines):
    out = {}
    for line in lines:
        if line.strip():
            r = json.loads(line)
            if r.get("kind") == "fn":
                out[r["id"]] = r
    return out


# --------------------------------------------------------------- check 1

def check_coverage(root, data, fns, stamp_crates, stamp_files, primitive, not_shipped):
    errs = []
    by_pkg = members(root)
    listed_crates = {u["crate"]: u for u in data.get("uncovered", []) if "crate" in u}
    listed_files = {u["file"]: u for u in data.get("uncovered", []) if "file" in u}
    crypto_crates = {}
    for pkg, path in by_pkg.items():
        if pkg in not_shipped:
            continue
        hit = sorted(set(normal_deps(root, path)) & set(primitive))
        if hit:
            crypto_crates[path] = "depends on " + ", ".join(hit)
    core_files = {}
    for fid, r in sorted(fns.items()):
        if not r["rules"]:
            continue
        path = by_pkg.get(r["crate"])
        if path is None:
            continue
        crypto_crates.setdefault(path, f"has cryptographic functions ({fid})")
        if path == CORE:
            core_files.setdefault(r["file"], fid)
    for path, why in sorted(crypto_crates.items()):
        if path in stamp_crates or path in listed_crates:
            continue
        if path == CORE:
            continue  # held per file, below
        errs.append(f"{path} {why}, and is neither in tools/crosscheck/stamp.py's CRATES "
                    f"nor an [[uncovered]] crate of {DATA}")
    for f, fid in sorted(core_files.items()):
        if f not in stamp_files and f not in listed_files:
            errs.append(f"{f} is cryptography in faraday-core ({fid}) that is neither in "
                        f"stamp.py's FILES nor an [[uncovered]] file of {DATA}")
    for path in sorted(listed_crates):
        if path in stamp_crates:
            errs.append(f"[[uncovered]] crate {path} is in stamp.py's CRATES: remove the entry")
        elif path not in crypto_crates:
            errs.append(f"[[uncovered]] crate {path} has no cryptography: remove the entry")
    for f in sorted(listed_files):
        if f in stamp_files:
            errs.append(f"[[uncovered]] file {f} is in stamp.py's FILES: remove the entry")
        elif f not in core_files:
            errs.append(f"[[uncovered]] file {f} has no cryptographic function: remove the entry")
    for u in data.get("uncovered", []):
        if not str(u.get("reason", "")).strip():
            errs.append(f"[[uncovered]] {u.get('crate') or u.get('file')} gives no reason")
    return errs


# --------------------------------------------------------------- check 2

def resolver(fns):
    """`crate::a::b` calls to function ids, by crate and last two segments."""
    index = {}
    for i, r in fns.items():
        parts = r["name"].split("::")
        stem = pathlib.PurePosixPath(r["file"]).stem
        module = pathlib.PurePosixPath(r["file"]).parent.name if stem in ("mod", "lib") else stem
        prev = parts[-2] if len(parts) > 1 else module
        index.setdefault((r["crate"], parts[-1]), []).append((prev, i))

    def resolve(call):
        crate, _, rest = call.partition("::")
        segs = rest.split("::")
        prev = segs[-2] if len(segs) > 1 else None
        return [i for p, i in index.get((crate, segs[-1]), []) if prev is None or p == prev]
    return resolve


def check_out_of_app(root, data, fns, actions):
    errs = []
    resolve = resolver(fns)
    reach, stack = set(), [i for i, r in fns.items() if r["file"].startswith("faraday/")]
    via = {i: i for i in stack}
    while stack:
        f = stack.pop()
        if f in reach:
            continue
        reach.add(f)
        r = fns[f]
        nexts = [c for c in r["callees"] if c in fns]
        nexts += [t for c in r["cross"] for t in resolve(c)]
        for n in nexts:
            if n not in reach:
                via.setdefault(n, via[f])
                stack.append(n)
    for o in data.get("out_of_app", []):
        roots = list(as_list(o.get("functions")))
        if o.get("action"):
            a = actions.get(o["action"])
            if a is None:
                errs.append(f"[[out_of_app]] {o['what']}: action {o['action']} is not in the manifest")
                continue
            roots += as_list(a.get("functions"))
        for f in roots:
            if f not in fns:
                errs.append(f"[[out_of_app]] {o['what']}: {f} is not a shipped function the "
                            "inventory finds (renamed? update the entry)")
            elif f in reach:
                errs.append(f"{f} is kept out of the app ({o['what']}, {o['decision']}) and the "
                            f"app reaches it, from {via[f]}")
        if o.get("unnamed"):
            rx = re.compile(o["unnamed"])
            for p in sorted(root.glob("faraday/**/src/**/*.rs")):
                # The Learn blocks generated from the manifest name functions
                # as text and call none (F148).
                if p.relative_to(root).as_posix() == "faraday/faraday-learn/src/how.rs":
                    continue
                for n, line in enumerate(p.read_text().splitlines(), 1):
                    if rx.search(line):
                        errs.append(f"{p.relative_to(root)}:{n} names what is kept out of the app "
                                    f"({o['what']}, {o['decision']})")
    return errs


# --------------------------------------------------------------- check 3

def fn_body(text, name):
    m = re.search(r"\bfn " + re.escape(name) + r"\b[^{]*\{", text)
    if not m:
        return None
    depth, i = 1, m.end()
    while depth and i < len(text):
        depth += {"{": 1, "}": -1}.get(text[i], 0)
        i += 1
    return text[m.end():i - 1]


def holds_in(root, src):
    body = fn_body((root / src["file"]).read_text(), src["function"])
    if body is None:
        return None
    if src.get("arms"):
        found = re.findall(r"(\w+)::(\w+)\s*=>\s*([^,\n]+)", body)
        return {f"{a}::{b}" for a, b, rhs in found if rhs.strip() != "false"}
    return {f"{a}::{b}" for a, b in re.findall(r"(\w+)::(\w+)\b(?!\s*\()(?!::)", body)}


def check_holds(root, data):
    errs = []
    found = {}
    for src in data.get("hold_source", []):
        hs = holds_in(root, src)
        if hs is None:
            errs.append(f"[[hold_source]] {src['file']}: no fn {src['function']}")
            continue
        for h in hs:
            found[(src["where"], h)] = src
    listed = {}
    for h in data.get("hold", []):
        listed[(h["where"], h["hold"])] = h
        dec = h.get("decision", "")
        if not re.fullmatch(r"F\d+", dec) or not (
                (root / "docs" / "decisions" / f"{dec}.md").is_file()
                or (root / "docs" / "history" / "decisions" / f"{dec}.md").is_file()):
            errs.append(f"[[hold]] {h['where']} {h['hold']}: decision {dec!r} is no decision's file")
        if not str(h.get("case", "")).strip():
            errs.append(f"[[hold]] {h['where']} {h['hold']}: no case")
    for k in sorted(set(found) - set(listed)):
        errs.append(f"{k[1]} is a hold in {found[k]['file']} ({k[0]}) and no [[hold]] of {DATA} "
                    "names it: list it with the decision that names its case (F136)")
    for k in sorted(set(listed) - set(found)):
        errs.append(f"[[hold]] {k[0]} {k[1]} is no longer a hold the code defines: remove it")
    return errs

# -------------------------------------------------------------- check 3a

INSPECT = "core/faraday-psbt/src/inspect.rs"
SIGNER = "core/faraday-psbt/src/signer.rs"
PSBT_SRC = "core/faraday-psbt/src"


def split_args(text, start):
    """The top-level arguments of the call whose `(` ends just before `start`."""
    depth, args, cur, i = 1, [], "", start
    while depth:
        if i >= len(text):
            return None
        c = text[i]
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        if depth == 1 and c == ",":
            args.append(cur.strip())
            cur = ""
        elif depth:
            cur += c
        i += 1
    args.append(cur.strip())
    return args


def _close(text, i):
    """The index just past the bracket that closes the one opened at text[i], or None."""
    depth = 0
    for j in range(i, len(text)):
        if text[j] in "([{":
            depth += 1
        elif text[j] in ")]}":
            depth -= 1
            if depth == 0:
                return j + 1
    return None


def _top_split(text):
    """`text` split at its top-level commas."""
    parts, depth, cur = [], 0, ""
    for c in text:
        depth += (c in "([{") - (c in ")]}")
        if c == "," and depth == 0:
            parts.append(cur)
            cur = ""
        else:
            cur += c
    parts.append(cur)
    return parts


def _first_brace(text):
    """The index of the first `{` outside brackets, or None."""
    depth = 0
    for j, c in enumerate(text):
        if c == "{" and depth == 0:
            return j
        depth += (c in "([") - (c in ")]")
    return None


def _values(expr, prefix, idx=None):
    """The set of `prefix::X` names `expr` can be, when every value it can take is
    that literal: the literal (with the fields of a variant), an `if`/`else` chain
    or a `match` whose every branch is one, or `return`/`continue`. With `idx`, a
    branch is a tuple and the literal is its element `idx`. None for anything else
    (a call, a variable, a block with statements): the caller stops the lint."""
    e = expr.strip().rstrip(";").strip()
    if re.fullmatch(r"(return|continue)\b.*", e, re.S) and not e.startswith("return ("):
        return set()
    if idx is None:
        m = re.fullmatch(prefix + r"::(\w+)\s*(\{.*\}|\(.*\))?", e, re.S)
        if m and (not m.group(2) or _close(e, e.index(m.group(2)[0], len(prefix))) == len(e)):
            return {m.group(1)}
    elif e.startswith("(") and _close(e, 0) == len(e):
        parts = _top_split(e[1:-1])
        if len(parts) > idx:
            m = re.fullmatch(prefix + r"::(\w+)\s*(\{.*\}|\(.*\))?", parts[idx].strip(), re.S)
            if m:
                return {m.group(1)}
    if re.match(r"if\b", e):
        out, rest = set(), e
        while True:
            rest = re.sub(r"^(else\s+)?if\b", "", rest.strip()).strip() if rest.strip().startswith(("if", "else if")) else rest
            i = _first_brace(rest)
            j = _close(rest, i) if i is not None else None
            if j is None:
                return None
            got = _values(rest[i + 1:j - 1], prefix, idx)
            if got is None:
                return None
            out |= got
            rest = rest[j:].strip()
            if rest.startswith("else if"):
                rest = rest[len("else "):]
                continue
            if rest.startswith("else"):
                rest = rest[len("else"):].strip()
                if not rest.startswith("{") or _close(rest, 0) != len(rest):
                    return None
                got = _values(rest[1:-1], prefix, idx)
                return None if got is None else out | got
            return None
    if re.match(r"match\b", e):
        i = _first_brace(e)
        if i is None or _close(e, i) != len(e):
            return None
        out = set()
        for arm in _top_split(e[i + 1:-1]):
            if not arm.strip():
                continue
            if "=>" not in arm:
                return None
            got = _values(arm.split("=>", 1)[1], prefix, idx)
            if got is None:
                return None
            out |= got
        return out
    return None


def _resolve(text, pos, arg, prefix):
    """The `prefix::X` names an argument can be, when every value it can take is a
    literal: written in the argument, or in the `let` nearest before the call that
    binds the variable it is (an `if`/`match` of literals). None otherwise: a call,
    another variable, a mix, an assignment after the `let`, a `let` not found."""
    got = _values(arg, prefix)
    if got:
        return got
    if not re.fullmatch(r"[a-z_]\w*", arg):
        return None
    before = text[:pos]
    fn = before.rfind("\nfn ")
    fn = max(fn, before.rfind(" fn "), 0)
    before = before[fn:]
    lets = list(re.finditer(r"\blet (?:mut )?(\(?[^=;]*\b" + re.escape(arg) + r"\b[^=;]*)=(?!=)", before))
    if not lets:
        return None
    last = lets[-1]
    if re.search(r"\b" + re.escape(arg) + r"\s*=(?!=)", before[last.end():]):
        return None  # assigned again between the `let` and the call
    between = before[last.end():]
    if (re.search(r"\bfor\b[^{;]*\b" + re.escape(arg) + r"\b[^{;]*\bin\b", between)
            or re.search(r"\|[^|;]*\b" + re.escape(arg) + r"\b[^|;]*\|", between)
            or re.search(r"\w\([^()]*\b" + re.escape(arg) + r"\b[^()]*\)\s*(=>|=(?!=))", between)):
        return None  # shadowed by a loop, closure or pattern binding
    pat = last.group(1).strip()
    idx = None
    if pat.startswith("("):
        try:
            idx = [x.strip().removeprefix("mut ").strip() for x in pat.strip("() ").split(",")].index(arg)
        except ValueError:
            return None  # a pattern like `ref level`
    depth, end = 0, None
    for j in range(last.end(), len(before)):
        depth += (before[j] in "([{") - (before[j] in ")]}")
        if before[j] == ";" and depth == 0:
            end = j
            break
    if end is None:
        return None
    return _values(before[last.end():end], prefix, idx) or None


def construction_sites(text, label):
    """Errors for a `Warning {` or `Finding {` literal, or an assignment to a `.level`, in
    a file whose `warn(` calls and `Kind::level` the lint reads, outside the forms it
    reads: the body of `fn warn` (`Warning { level, kind, detail }`) and
    `Finding { level: kind.level(), .. }`."""
    text = text.split("#[cfg(test)]", 1)[0]
    errs = []
    for m in re.finditer(r"\.(level|kind)\s*=(?!=)|&mut\s+[\w.\[\]]*\.(level|kind)\b", text):
        errs.append(f"{label}:{text[:m.start()].count(chr(10)) + 1}: changes a `.level` or `.kind`: "
                    "the lint cannot read it (unreadable site)")
    for m in re.finditer(r"(?<!->)(?<!-> )\bSelf\s*\{", text):
        impls = re.findall(r"\bimpl\b[^{]*?\b(\w+)\s*\{", text[:m.start()])
        if impls and impls[-1] in ("Warning", "Finding"):
            errs.append(f"{label}:{text[:m.start()].count(chr(10)) + 1}: builds a `Self {{` of a "
                        "warning or finding (unreadable site)")
    body = fn_body(text, "warn")
    if body is not None and not re.fullmatch(
            r"\s*warnings\.push\(Warning \{\s*level,\s*kind,\s*detail,?\s*\}\);\s*", body):
        errs.append(f"{label}: `fn warn` is not exactly `warnings.push(Warning {{ level, kind, detail }});` "
                    "(unreadable site)")
    for m in re.finditer(r"\b(Warning|Finding)\s*\{", text):
        if re.search(r"\b(struct|impl|for)\s+$", text[:m.start()]):
            continue
        rest = text[m.end():]
        if m.group(1) == "Warning":
            fns = re.findall(r"\bfn (\w+)", text[:m.start()])
            ok = bool(fns) and fns[-1] == "warn" and re.match(r"\s*level,\s*kind,\s*detail,?\s*\}", rest)
        else:
            ok = re.match(r"\s*level: kind\.level\(\),", rest)
        if not ok:
            errs.append(f"{label}:{text[:m.start()].count(chr(10)) + 1}: builds a `{m.group(1)} {{` "
                        "in a form the lint does not read (unreadable site)")
    return errs


def inspect_levels(text, label=INSPECT):
    """{ 'WarningKind::X': {levels} } from every `warn(` call of a file of inspect.rs's module
    outside its tests. A site that makes Caution or Info of a kind it cannot name is an error."""
    errs = construction_sites(text, label)
    text = text.split("#[cfg(test)]", 1)[0]
    out = {}
    for m in re.finditer(r"\bwarn\(", text):
        if text[max(0, m.start() - 3):m.start()] == "fn ":
            continue
        line = text[:m.start()].count("\n") + 1
        args = split_args(text, m.end())
        if args is None or len(args) < 3:
            errs.append(f"{label}:{line}: a `warn(` call that cannot be read")
            continue
        levels = _resolve(text, m.start(), args[1], "Level")
        if not levels:
            errs.append(f"{label}:{line}: the level of this warning is `{args[1]}`, which the "
                        "lint cannot read (write Level::X, or bind it in the nearest `let` as Level::X, "
                        "an if/else or a match of them only)")
            continue
        if not levels & {"Caution", "Info"}:
            continue  # a Danger or Blocked site makes neither
        kinds = _resolve(text, m.start(), args[2], "WarningKind")
        if not kinds:
            errs.append(f"{label}:{line}: a warning made at {sorted(levels)} has a kind "
                        f"`{args[2][:40]}` the lint cannot read")
            continue
        for k in kinds:
            out.setdefault("WarningKind::" + k, set()).update(levels)
    return out, errs


def signer_levels(text):
    """{ 'Kind::X': {level} } from the arms of `Kind::level` in signer.rs: every arm is
    `Kind::A | Kind::B { .. } => Level::X,`; any other arm is an error."""
    errs = construction_sites(text, SIGNER)
    body = fn_body(text, "level")
    if body is None:
        return {}, errs + [f"{SIGNER}: no fn level"]
    out = {}
    head = re.match(r"\s*match self \{", body)
    if not head:
        return {}, errs + [f"{SIGNER}: `Kind::level` is not a `match self` the lint reads"]
    pos = head.end()
    while "=>" in body[pos:]:
        i = body.index("=>", pos)
        pat = body[pos:i]
        rest = re.compile(r"\s*Level::(\w+),").match(body, i + 2)
        left = re.sub(r"Kind::\w+|\{[^{}]*\}|\([^()]*\)|\|", "", pat).strip()
        names = re.findall(r"\bKind::(\w+)", pat)
        if not rest or left or not names:
            errs.append(f"{SIGNER}: an arm of `Kind::level` the lint cannot read (not `Kind::X => "
                        f"Level::Y,`): {' '.join((pat + body[i:i + 40]).split())[:70]}")
            return out, errs
        for k in names:
            out["Kind::" + k] = {rest.group(1)}
        pos = rest.end()
    if body[pos:].strip() != "}":  # the match's own closing brace
        errs.append(f"{SIGNER}: after the last arm of `Kind::level`: {body[pos:].strip()[:50]}")
    if not out:
        errs.append(f"{SIGNER}: no arm of `Kind::level` read")
    return out, errs


def warning_levels(root):
    errs = []
    levels = {}
    inspect_files = [INSPECT] + sorted(
        p.relative_to(root).as_posix() for p in (root / PSBT_SRC / "inspect").glob("*.rs"))
    ins = {}
    for rel in inspect_files:
        got, e = inspect_levels((root / rel).read_text(), rel)
        errs += e
        for k, v in got.items():
            ins.setdefault(k, set()).update(v)
    sig, e = signer_levels((root / SIGNER).read_text())
    errs += e
    levels.update(ins)
    levels.update(sig)
    for p in sorted((root / PSBT_SRC).glob("**/*.rs")):
        rel = p.relative_to(root).as_posix()
        if rel in inspect_files or rel == SIGNER:
            continue
        body = p.read_text().split("#[cfg(test)]", 1)[0]
        for n, line in enumerate(body.splitlines(), 1):
            if line.lstrip().startswith("//"):
                continue
            if re.search(r"\bLevel::(Caution|Info)\b|\bWarning \{|\bwarn\(", line):
                errs.append(f"{rel}:{n}: makes a warning or a level outside the two files "
                            "the lint reads (inspect.rs, its inspect/ files, signer.rs); read it there or extend check 3a")
    return levels, errs


def check_levels(root, data):
    levels, errs = warning_levels(root)
    cautions = {k for k, v in levels.items() if "Caution" in v}
    notes = {k for k, v in levels.items() if "Info" in v and "Caution" not in v}
    for table, found, what in (("caution", cautions, "Caution"), ("note", notes, "Info")):
        listed = {}
        for e in data.get(table, []):
            listed[e["kind"]] = e
            dec = e.get("decision", "")
            if not re.fullmatch(r"F\d+", dec) or not (
                    (root / "docs" / "decisions" / f"{dec}.md").is_file()
                    or (root / "docs" / "history" / "decisions" / f"{dec}.md").is_file()):
                errs.append(f"[[{table}]] {e['kind']}: decision {dec!r} is no decision's file")
            if not str(e.get("case", "")).strip():
                errs.append(f"[[{table}]] {e['kind']}: no case")
        for k in sorted(found - set(listed)):
            errs.append(f"{k} is made at {what} in the code and no [[{table}]] of {DATA} names it: "
                        "list it with the decision and the case that name it (F136's table)")
        for k in sorted(set(listed) - found):
            errs.append(f"[[{table}]] {k} is no longer made at {what}: remove it")
    return errs


# -------------------------------------------------------------- check 3b

def check_way_past(root, data):
    errs = []
    for w in data.get("way_past", []):
        name = w.get("what", "?")
        dec = w.get("decision", "")
        if not re.fullmatch(r"F\d+", dec) or not (
                (root / "docs" / "decisions" / f"{dec}.md").is_file()
                or (root / "docs" / "history" / "decisions" / f"{dec}.md").is_file()):
            errs.append(f"[[way_past]] {name}: decision {dec!r} is no decision's file")
        if not str(w.get("case", "")).strip():
            errs.append(f"[[way_past]] {name}: no case")
        p = root / w.get("file", "")
        if not p.is_file() or not re.search(w.get("pattern", "(?!)"), p.read_text(), re.M):
            errs.append(f"[[way_past]] {name}: {w.get('file')} no longer has /{w.get('pattern')}/: "
                        "if the way past is gone or moved, change or remove the entry")
    return errs


# --------------------------------------------------------------- check 4

def section(text, n):
    m = re.search(rf"^## {n}\. .*?$(.*?)(?=^## |\Z)", text, re.M | re.S)
    return m.group(1) if m else None


def check_disagreements(root, data):
    errs = []
    sec = section((root / "docs" / "STANDARDS.md").read_text(), 8)
    if sec is None:
        return ["docs/STANDARDS.md has no §8"]
    bullets = len(re.findall(r"^- ", sec, re.M))
    entries = data.get("disagreement", [])
    if bullets != len(entries):
        errs.append(f"STANDARDS §8 has {bullets} disagreements and {DATA} holds {len(entries)}: "
                    "each bullet has one [[disagreement]], and a fixed one leaves both")
    for d in entries:
        for f, rx in d["holds"].items():
            p = root / f
            if not p.is_file() or not re.search(rx, p.read_text(), re.M):
                errs.append(f"STANDARDS §8 ({d['what']}): {f} no longer has /{rx}/: if it is "
                            "fixed, remove the bullet and the [[disagreement]]; otherwise say "
                            "what it is now")
    return errs


# --------------------------------------------------------------- check 5

SUMMARY = re.compile(r"^<!-- standards-summary: begin -->\n(.*?)\n<!-- standards-summary: end -->$",
                     re.M | re.S)


def sentences(text):
    """The sentences of one paragraph: a stop, then a space and a capital."""
    text = " ".join(text.split())
    return [s for s in re.split(r"(?<=[.!?])\s+(?=[A-Z])", text) if s]


def check_rule(root):
    """§0's summary between its markers, two sentences, and a bold lead on every point of
    its rule (the bullets before §0.1)."""
    sec = section((root / "docs" / "STANDARDS.md").read_text(), 0)
    if sec is None:
        return ["docs/STANDARDS.md has no §0"]
    errs = []
    m = SUMMARY.search(sec)
    if not m or not m.group(1).strip():
        errs.append("STANDARDS §0 has no summary between its standards-summary markers, "
                    "which the website's /standards and /cryptography read")
    elif len(sentences(m.group(1))) != 2:
        errs.append(f"STANDARDS §0's summary is {len(sentences(m.group(1)))} sentences; "
                    "it is two, which the website's pages show as they are")
    points = re.findall(r"^- (.*)$", sec.split("\n### ", 1)[0], re.M)
    if not points:
        errs.append("STANDARDS §0 has no points of its rule (a bullet list before §0.1)")
    for p in points:
        if not re.match(r"\*\*[^*]+\*\*", p):
            errs.append(f"STANDARDS §0: a point of the rule has no bold lead: {p[:60]}")
    return errs


# ------------------------------------------------------------------ main

def run(root, inv_path):
    import stamp
    import check
    data = tomllib.loads((root / DATA).read_text())
    inv_rs = (root / INVENTORY_RS).read_text()
    primitive = rust_list(inv_rs, "PRIMITIVE_CRATES")
    not_shipped = rust_list(inv_rs, "NOT_SHIPPED_CRATES")
    fns = functions(inventory_lines(root, inv_path))
    c = check.Ctx(root, check.Inventory([]))
    errs = []
    errs += check_coverage(root, data, fns, stamp.CRATES, stamp.FILES, primitive, not_shipped)
    errs += check_out_of_app(root, data, fns, c.by_id)
    errs += check_holds(root, data)
    errs += check_levels(root, data)
    errs += check_way_past(root, data)
    errs += check_disagreements(root, data)
    errs += check_rule(root)
    return errs, data


def main(argv):
    if argv == ["--self-test"]:
        return self_test()
    inv = None
    if argv[:1] == ["--inventory"] and len(argv) == 2:
        inv = argv[1]
    elif argv:
        print(__doc__)
        return 2
    if self_test():
        return 1
    errs, data = run(ROOT, inv)
    for e in errs:
        print(f"lint-standards: {e}")
    if errs:
        return 1
    print(f"lint-standards: ok ({len(data.get('uncovered', []))} not cross-checked, "
          f"{len(data.get('out_of_app', []))} out of the app, {len(data.get('hold', []))} holds, "
          f"{len(data.get('caution', []))} cautions, {len(data.get('note', []))} notes, "
          f"{len(data.get('way_past', []))} other ways past, "
          f"{len(data.get('disagreement', []))} in STANDARDS §8)")
    return 0


# ------------------------------------------------------------- self-test

def self_test():
    import tempfile
    fails = []

    def expect(cond, what):
        if not cond:
            fails.append(what)

    with tempfile.TemporaryDirectory() as d:
        root = pathlib.Path(d)
        (root / "Cargo.toml").write_text('[workspace]\nmembers = ["a", "b", "faraday/faraday-core"]\n')
        for m, deps in (("a", 'sha2 = "1"'), ("b", ""), ("faraday/faraday-core", "")):
            (root / m).mkdir(parents=True)
            name = m.split("/")[-1]
            (root / m / "Cargo.toml").write_text(f'[package]\nname = "{name}"\n[dependencies]\n{deps}\n')
        fns = {
            "faraday/faraday-core/src/x.rs::f": {"crate": "faraday-core", "file": "faraday/faraday-core/src/x.rs",
                                                 "name": "f", "rules": [3], "callees": [],
                                                 "cross": ["b::out::gone"]},
            "b/src/out.rs::gone": {"crate": "b", "file": "b/src/out.rs", "name": "gone", "rules": [],
                                   "callees": [], "cross": []},
        }
        e = check_coverage(root, {}, fns, [], [], ["sha2"], [])
        expect(any(x.startswith("a depends on sha2") for x in e), "a crate depending on sha2 not covered")
        expect(any("faraday/faraday-core/src/x.rs is cryptography" in x for x in e), "a core file")
        e = check_coverage(root, {"uncovered": [{"crate": "a", "reason": "r"},
                                                {"file": "faraday/faraday-core/src/x.rs", "reason": "r"},
                                                {"crate": "b", "reason": "r"}]}, fns, [], [], ["sha2"], [])
        expect(e == ["[[uncovered]] crate b has no cryptography: remove the entry"], f"listed: {e}")
        e = check_out_of_app(root, {"out_of_app": [{"what": "w", "decision": "F1",
                                              "functions": ["b/src/out.rs::gone"]}]}, fns, {})
        expect(len(e) == 1 and "the app reaches it" in e[0], f"out of app: {e}")
        (root / "h.rs").write_text("fn is_hold(a: A) -> bool {\n    matches!(a, A::One | A::W(B::Two))\n}\n"
                                   "fn arms(a: S) -> bool { match a { S::Yes => true, S::No => false, _ => false } }\n")
        (root / "docs" / "decisions").mkdir(parents=True)
        (root / "docs" / "decisions" / "F1.md").write_text("")
        src = [{"where": "x", "file": "h.rs", "function": "is_hold"},
               {"where": "y", "file": "h.rs", "function": "arms", "arms": True}]
        hold = [{"where": "x", "hold": "A::One", "decision": "F1", "case": "c"},
                {"where": "x", "hold": "B::Gone", "decision": "F2", "case": "c"}]
        e = check_holds(root, {"hold_source": src, "hold": hold})
        expect(any("B::Two is a hold" in x for x in e), f"a new hold: {e}")
        expect(any("S::Yes is a hold" in x for x in e), f"an arm: {e}")
        expect(not any("S::No" in x for x in e), "a false arm is no hold")
        expect(any("F2" in x for x in e), "a decision with no file")
        expect(any("B::Gone is no longer" in x for x in e), "a stale hold")
        (root / "docs" / "STANDARDS.md").write_text("## 8. Disagree\n\n- one\n\n## 9. Next\n")
        e = check_disagreements(root, {"disagreement": [{"what": "w", "holds": {"h.rs": "A::One"}}]})
        expect(e == [], f"a held disagreement: {e}")
        e = check_disagreements(root, {"disagreement": [{"what": "w", "holds": {"h.rs": "A::Fixed"}}]})
        expect(len(e) == 1, "a changed disagreement")
        e = check_disagreements(root, {})
        expect(len(e) == 1, "a bullet with no entry")
        rule = ("## 0. What goes in\n\n<!-- standards-summary: begin -->\nOne sentence. And a second\n"
                "one.\n<!-- standards-summary: end -->\n\n- **A point.** Its text\n  goes on.\n"
                "- **Another** (F1). Text.\n\n### 0.1 Left out\n\n- not a point\n\n## 1. Next\n")
        (root / "docs" / "STANDARDS.md").write_text(rule)
        e = check_rule(root)
        expect(e == [], f"a summary and its points: {e}")
        (root / "docs" / "STANDARDS.md").write_text(rule.replace("<!-- standards-summary: end -->", ""))
        e = check_rule(root)
        expect(len(e) == 1 and "no summary" in e[0], f"a missing marker: {e}")
        (root / "docs" / "STANDARDS.md").write_text(rule.replace(" And a second\none.", ""))
        e = check_rule(root)
        expect(len(e) == 1 and "1 sentences" in e[0], f"one sentence: {e}")
        (root / "docs" / "STANDARDS.md").write_text(rule.replace("- **Another** (F1).", "- Another (F1)."))
        e = check_rule(root)
        expect(len(e) == 1 and "no bold lead" in e[0], f"a point with no lead: {e}")
        # 3a: warnings read from synthetic sources
        def ins(body):
            return inspect_levels("fn f(w: &mut V) {\n" + body + "\n}\n", "i.rs")
        got, e = ins("warn(w, Level::Caution, WarningKind::New, d);\nwarn(w, Level::Info, WarningKind::Nt, d);")
        expect(not e and got == {"WarningKind::New": {"Caution"}, "WarningKind::Nt": {"Info"}}, f"literal sites: {got} {e}")
        got, e = ins("let (level, d) = if a { (Level::Caution, 1) } else if b { (Level::Info, 2) } else { return; };\n"
                     "warn(w, level, WarningKind::T, d);")
        expect(not e and got == {"WarningKind::T": {"Caution", "Info"}}, f"a tuple let of literals: {got} {e}")
        for what, body in (
                ("a call as the level", "warn(w, pick(x), WarningKind::A, d);"),
                ("a mix of a literal and a variable", "let level = if big { Level::Danger } else { base };\nwarn(w, level, WarningKind::A, d);"),
                ("a mix inline", "warn(w, if big { Level::Danger } else { lvl }, WarningKind::A, d);"),
                ("a level let a call makes", "let level = pick(x);\nwarn(w, Level::Danger, WarningKind::A, d);\nwarn(w, level, WarningKind::B, d);"),
                ("a kind a call makes", "let kind = make();\nwarn(w, Level::Caution, kind, d);"),
                ("a level assigned again", "let mut level = Level::Info;\nlevel = other;\nwarn(w, level, WarningKind::A, d);"),
                ("a warning pushed", "w.push(Warning { level: Level::Caution, kind: WarningKind::N, detail: d });"),
                ("a level assigned", "x.level = Level::Caution;"),
                ("a kind assigned", "x.kind = WarningKind::Other;"),
                ("a level through a reference", "let l = &mut x.level;"),
                ("a ref pattern", "let (ref level, d) = pair;\nwarn(w, level, WarningKind::A, d);"),
                ("a loop shadowing the let", "let level = Level::Danger;\nfor level in lv { warn(w, level, WarningKind::A, d); }"),
                ("a Self literal", "}\nimpl Warning { fn mk() -> Self { Self { level: Level::Caution } }")):
            expect(len(ins(body)[1]) == 1, f"unreadable site not stopped ({what}): {ins(body)}")
        sg = "impl Kind { pub fn level(&self) -> Level { match self {\n%s\n} } }\n"
        got, e = signer_levels(sg % "Kind::A | Kind::B { .. } => Level::Caution,\nKind::C(_) => Level::Danger,")
        expect(not e and got["Kind::B"] == {"Caution"} and got["Kind::C"] == {"Danger"}, f"signer arms: {got} {e}")
        for what, arms in (("a call", "Kind::X(f) => f.level(),\nKind::Y => Level::Danger,"),
                           ("an if", "Kind::X => if c { Level::Caution } else { Level::Danger },\nKind::Y => Level::Danger,"),
                           ("a catch-all", "Kind::X => Level::Danger,\n_ => Level::Caution,"),
                           ("a guard", "Kind::X if c => Level::Danger,")):
            expect(signer_levels(sg % arms)[1], f"signer arm not stopped ({what})")
        e = inspect_levels("fn warn(warnings: &mut V, level: Level, kind: WarningKind, detail: D) {\n"
                           "    let level = if kind == K::N { Level::Caution } else { level };\n"
                           "    warnings.push(Warning { level, kind, detail });\n}\n", "i.rs")[1]
        expect(len(e) == 1 and "fn warn" in e[0], f"a level changed inside fn warn: {e}")
        e = inspect_levels("fn warn(warnings: &mut V, level: Level, kind: WarningKind, detail: D) {\n"
                           "    warnings.push(Warning { level, kind, detail });\n}\n", "i.rs")[1]
        expect(not e, f"fn warn as it is: {e}")
        expect(signer_levels("fn f() { Finding { level: Level::Caution, at } }\n" + sg % "Kind::A => Level::Danger,")[1],
               "a Finding built at a level not stopped")
        # 3a and 3b against lists, over a small tree
        psbt = root / PSBT_SRC
        (psbt / "inspect").mkdir(parents=True)
        (psbt / "inspect.rs").write_text("fn f(w: &mut V) {\nwarn(w, Level::Caution, WarningKind::Unlisted, d);\n"
                                         "warn(w, Level::Info, WarningKind::Loose, d);\n}\n")
        (psbt / "signer.rs").write_text(sg % "Kind::Z => Level::Danger,")
        data = {"caution": [{"kind": "WarningKind::Stale", "decision": "F1", "case": "c"}]}
        e = " | ".join(check_levels(root, data))
        expect("WarningKind::Unlisted is made at Caution" in e, f"an unlisted caution: {e}")
        expect("WarningKind::Loose is made at Info" in e, f"an unlisted note: {e}")
        expect("[[caution]] WarningKind::Stale is no longer" in e, f"a stale caution: {e}")
        (psbt / "inspect.rs").write_text("fn f(w: &mut V) {\nwarn(w, lvl, WarningKind::A, d);\n}\n")
        expect(any("cannot read" in x for x in check_levels(root, {})), "an unreadable site in the tree")
        (root / "w.rs").write_text("fn go() { skip(); }\n")
        wp = {"what": "w", "decision": "F1", "case": "c", "file": "w.rs", "pattern": "skip\\("}
        expect(check_way_past(root, {"way_past": [wp]}) == [], "a way past still there")
        e = check_way_past(root, {"way_past": [dict(wp, pattern="gone\\(")]})
        expect(len(e) == 1 and "no longer has" in e[0], f"a way past whose pattern is gone: {e}")
    for f in fails:
        print(f"lint-standards self-test: {f}")
    print("lint-standards self-test: " + ("FAILED" if fails else "ok"))
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
