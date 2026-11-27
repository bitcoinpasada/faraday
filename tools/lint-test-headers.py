#!/usr/bin/env python3
"""lint-test-headers: every test opens with its Invariant, Regression and Owner, the Owner is the
layer that owns the test, and no invariant is named twice (docs/spec/07-tests.md §3 and §4).

It reads every Rust file of the workspace's crates (`src/` and `tests/`) and the layer metadata from
`cargo metadata --format-version 1 --no-deps --locked` (or a file given with `--metadata`), and
reports every failure in one run. The checks are numbered as in 07 §4:

  1. a test whose doc comment does not start with the three lines of §3.1 in order
  2. an Owner that is not one of the six names
  3. an Owner that is not the layer of the crate (for a tool crate, its `checks` layer)
  4. an Invariant named twice in the workspace
  5. an empty Invariant or Regression, or a Regression equal to its Invariant
  6. a file on `tools/test-headers.copied` that has a header in every test, or that is missing
  7. a test that names the tokens crate (TS11)
  8. an `#[ignore]` without a reason that names the recipe that runs it

Checks 1 to 5 skip the files of the copied list (07 §4.1); 6 to 8 do not. Standard library only:
run as `python3 -I -B tools/lint-test-headers.py`.
"""

import argparse
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent


def _load(name):
    spec = importlib.util.spec_from_file_location(name, HERE / f"{name}.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


lex = _load("rustlex")

OWNERS = ("domain", "kit", "app", "runtime", "adapter", "root")
HEADER = ("Invariant", "Regression", "Owner")
COPIED_LIST = "tools/test-headers.copied"
TEST_ATTRS = ("test", "proptest")


class Test:
    def __init__(self, path, name, line, doc, attrs, body):
        self.path, self.name, self.line = path, name, line
        self.doc, self.attrs, self.body = doc, attrs, body  # doc: list of str; attrs: token lists; body: tokens
        self.header = None  # (invariant, regression, owner) once read
        self.ok_shape = False


def _doc_lines(doc_toks):
    """The lines of an outer doc comment, in order, each without its marker and indentation."""
    lines = []
    for t in doc_toks:
        for ln in t.text.split("\n"):
            ln = ln.strip()
            if ln.startswith("*"):
                ln = ln[1:]
            lines.append(ln.strip())
    return lines


def _is_test_attr(attr):
    """An attribute `#[test]`, `#[proptest]`, or the same with a path (`tokio::test`)."""
    return bool(attr) and all(t.kind == "id" or t.text == "::" for t in attr) and attr[-1].text in TEST_ATTRS


def find_tests(path, toks):
    """Every test function of a file: those with a `#[test]` (or property-test) attribute."""
    out = []
    n = len(toks)
    for i, t in enumerate(toks):
        if t.kind != "id" or t.text != "fn" or i + 1 >= n or toks[i + 1].kind != "id":
            continue
        attrs, docs = [], []
        j = i - 1
        while j >= 0:
            tj = toks[j]
            if tj.kind == "doc":
                docs.append(tj)
                j -= 1
            elif tj.kind == "idoc":
                break
            elif tj.kind == "id" and tj.text in ("pub", "async", "unsafe", "const", "extern", "crate"):
                j -= 1
            elif tj.kind == "p" and tj.text == ")":
                depth, k = 0, j
                while k >= 0:
                    depth += toks[k].text == ")"
                    depth -= toks[k].text == "("
                    if depth == 0:
                        break
                    k -= 1
                if k > 0 and toks[k - 1].text == "pub":
                    j = k - 1
                else:
                    break
            elif tj.kind == "p" and tj.text == "]":
                depth, k = 0, j
                while k >= 0:
                    depth += toks[k].text == "]"
                    depth -= toks[k].text == "["
                    if depth == 0:
                        break
                    k -= 1
                if k > 0 and toks[k - 1].text == "#":
                    attrs.append(toks[k + 1 : j])
                    j = k - 2
                else:
                    break
            else:
                break
        if not any(_is_test_attr(a) for a in attrs):
            continue
        # the body: the first `{` after the parameter list
        k = i + 2
        while k < n and toks[k].text != "(":
            k += 1
        k = lex._match_close(toks, k) + 1 if k < n else n
        while k < n and toks[k].text not in ("{", ";"):
            k += 1
        end = lex._match_close(toks, k) if k < n and toks[k].text == "{" else k
        out.append(Test(path, toks[i + 1].text, t.line, _doc_lines(reversed(docs)), attrs, toks[k:end]))
    return out


def read_header(test):
    """Fill `test.header` and `test.ok_shape` from its doc lines. Check 1's shape: the first three
    lines are `Invariant:`, `Regression:` and `Owner:`, in this order."""
    d = test.doc
    if len(d) < 3:
        return
    vals = []
    for want, line in zip(HEADER, d[:3]):
        m = re.match(rf"{want}:(.*)$", line)
        if not m:
            return
        vals.append(m.group(1).strip())
    test.header = tuple(vals)
    test.ok_shape = True


def norm(s):
    """07 §4, check 4: case folded, whitespace collapsed, a final full stop dropped."""
    s = " ".join(s.casefold().split())
    return s[:-1].rstrip() if s.endswith(".") else s


def recipes(justfile):
    names = set()
    try:
        text = justfile.read_text()
    except OSError:
        return names
    for ln in text.splitlines():
        m = re.match(r"^@?([A-Za-z_][A-Za-z0-9_-]*)(?:[ \t]+[^:\n]*?)?:(?!=)", ln)
        if m and m.group(1) not in ("set", "alias", "import", "mod", "export"):
            names.add(m.group(1))
    return names


def ignore_reason(test):
    """(has an ignore attribute, its reason string or None)."""
    for a in test.attrs:
        if a and a[0].text == "ignore":
            for t in a[1:]:
                if t.kind == "str":
                    return True, t.text
            return True, None
    return False, None


def rust_files(crate_dir):
    for sub in ("src", "tests"):
        d = crate_dir / sub
        if d.is_dir():
            yield from sorted(d.rglob("*.rs"))


def run(meta, root, copied=None, recipe_names=None):
    """The failures: [(check number, where, message)]."""
    root = Path(root)
    fails = []
    members = set(meta.get("workspace_members", []))
    pkgs = [p for p in meta["packages"] if not members or p["id"] in members]
    layers = {}
    tokens_idents = set()
    for p in pkgs:
        md = (p.get("metadata") or {}).get("faraday") or {}
        layers[p["name"]] = md
        if md.get("layer") == "kit" and md.get("level") == "tokens":
            tokens_idents.add(lex.crate_ident(p["name"]))
    if copied is None:
        try:
            lines = (root / COPIED_LIST).read_text().splitlines()
        except OSError:
            lines = []
        copied = [ln.strip() for ln in lines if ln.strip() and not ln.strip().startswith("#")]
    copied = set(copied)
    if recipe_names is None:
        recipe_names = recipes(root / "justfile")

    def rel(p):
        try:
            return str(Path(p).resolve().relative_to(root.resolve()))
        except ValueError:
            return str(p)

    seen_files = set()
    invariants = {}  # normalised -> [where]
    for p in pkgs:
        md = layers[p["name"]]
        crate_dir = Path(p["manifest_path"]).parent
        for f in rust_files(crate_dir):
            seen_files.add(rel(f))
            try:
                src = f.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            toks = lex.tokenize(src, docs=True)
            tests = find_tests(f, toks)
            if not tests:
                if rel(f) in copied:
                    fails.append((6, rel(f), f"the file holds no test and is still on `{COPIED_LIST}`"))
                continue
            uses, _ = lex.split_uses([t for t in toks if t.kind not in ("doc", "idoc")])
            amap = lex.alias_map(uses)
            glob_tokens = [u for u in uses if u.path[-1] == "*" and u.path[0] in tokens_idents]
            is_copied = rel(f) in copied
            for t in tests:
                where = f"{rel(f)}:{t.line}"
                for t_use in glob_tokens:
                    fails.append((7, f"{rel(f)}:{t_use.line}", f"`{'::'.join(t_use.path)}` brings the tokens crate into a file of tests (TS11)"))
                    break
                # 7: a test names the tokens crate
                for path, line in lex.paths(t.body):
                    if lex.resolve(path, amap)[0] in tokens_idents:
                        fails.append((7, f"{rel(f)}:{line}", f"test `{t.name}` names the tokens crate (TS11)"))
                        break
                # 8: #[ignore] with a reason naming the recipe
                has, reason = ignore_reason(t)
                if has:
                    words = set(re.split(r"[^A-Za-z0-9_-]+", reason or ""))
                    if not reason:
                        fails.append((8, where, f"test `{t.name}` is `#[ignore]` without a reason naming the recipe that runs it"))
                    elif not (words & recipe_names):
                        fails.append((8, where, f"test `{t.name}` is ignored with a reason that names no recipe of the justfile"))
                if is_copied:
                    read_header(t)
                    continue
                read_header(t)
                if not t.ok_shape:
                    fails.append((1, where, f"test `{t.name}` does not open with Invariant, Regression and Owner lines in this order"))
                    continue
                inv, reg, own = t.header
                if own not in OWNERS:
                    fails.append((2, where, f"test `{t.name}`: Owner `{own}` is not one of {', '.join(OWNERS)}"))
                else:
                    layer = md.get("layer")
                    want = md.get("checks") if layer == "tool" else layer
                    if layer == "tool" and not want:
                        fails.append((3, where, f"test `{t.name}` is in a tool crate with no `checks = \"<layer>\"`"))
                    elif own != want:
                        fails.append((3, where, f"test `{t.name}`: Owner `{own}`, but the crate `{p['name']}` is `{want}`"))
                if not inv or not reg:
                    fails.append((5, where, f"test `{t.name}` has an empty {'Invariant' if not inv else 'Regression'}"))
                elif norm(inv) == norm(reg):
                    fails.append((5, where, f"test `{t.name}`: the Regression equals the Invariant"))
                if inv:
                    invariants.setdefault(norm(inv), []).append(where)
            if is_copied and tests and all(x.ok_shape for x in tests):
                fails.append((6, rel(f), f"every test has its header, so the file is taken off `{COPIED_LIST}`"))
    for key, wheres in invariants.items():
        if len(wheres) > 1:
            fails.append((4, wheres[0], "the Invariant is named twice: " + ", ".join(wheres)))
    for name in sorted(copied):
        if name not in seen_files and not (root / name).exists():
            fails.append((6, name, f"listed in `{COPIED_LIST}` and does not exist"))
    return sorted(fails, key=lambda x: (x[0], x[1]))


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--metadata", help="a `cargo metadata --no-deps` JSON file instead of running cargo")
    ap.add_argument("--root", default=str(HERE.parent), help="the workspace root")
    args = ap.parse_args(argv)
    if args.metadata:
        meta = json.loads(Path(args.metadata).read_text())
    else:
        p = subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"],
            cwd=args.root, capture_output=True, text=True,
        )  # fmt: skip
        if p.returncode != 0:
            print(p.stderr, file=sys.stderr)
            print("lint-test-headers: cargo metadata failed", file=sys.stderr)
            return 2
        meta = json.loads(p.stdout)
    fails = run(meta, args.root)
    for n, where, why in fails:
        print(f"check {n}: {where}: {why}")
    if fails:
        print(f"lint-test-headers: {len(fails)} failures", file=sys.stderr)
        return 1
    print("lint-test-headers: every test has its header")
    return 0


if __name__ == "__main__":
    sys.exit(main())
