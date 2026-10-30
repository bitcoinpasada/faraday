#!/usr/bin/env python3
"""docs/STANDARDS.md §0, held (F79, F109, F136): five checks over the tree
and the crypto inventory, against the lists in `tools/lint-standards.toml`.

    lint-standards.py [--inventory FILE]     the self-test, then the five checks
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
(§4.4 of docs/CRYPTO-LEARN.md) in check 2; a way past a check that is not
a hold (a caution that still signs, F136's table) in check 3.
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
    for f in fails:
        print(f"lint-standards self-test: {f}")
    print("lint-standards self-test: " + ("FAILED" if fails else "ok"))
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
