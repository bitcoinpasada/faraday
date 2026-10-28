#!/usr/bin/env python3
"""What each cross-check area says about itself (docs/CRYPTO-LEARN.md §10.1).

Every `area_*.py` class states, beside `name`, the attributes below. This
reads them without importing an area (no reference, no Rust), writes the
README's "What runs" and "Known differences" from them between marker
lines, and checks the README is that text.

    facts.py check     exit 1 when an attribute is missing or empty, a path in
                       `faraday` does not resolve, or the README is not current
    facts.py write     rewrite the README between its marker lines

Standard library only; run as `python3 -I -B tools/crosscheck/facts.py`.
"""

import ast
import pathlib
import re
import sys
import textwrap

ROOT = pathlib.Path(__file__).resolve().parents[2]
HERE = ROOT / "tools" / "crosscheck"
README = HERE / "README.md"

# The attributes, in the order a class states them. `differences` is a list
# of strings, the rest are strings; "none" says there is nothing to state.
STRINGS = ("faraday", "reference", "reference_checked", "vectors", "corpus", "not_compared")
LISTS = ("differences",)
ATTRIBUTES = STRINGS + LISTS
NONE = "none"


def areas():
    """[(name, module)] in the order `crosscheck.py` runs them."""
    tree = ast.parse((HERE / "crosscheck.py").read_text())
    for node in ast.walk(tree):
        if isinstance(node, ast.Assign) and getattr(node.targets[0], "id", "") == "AREAS":
            return [(k.value, v.elts[0].value) for k, v in zip(node.value.keys, node.value.values)]
    raise SystemExit("facts: AREAS not found in crosscheck.py")


def read(name, module):
    """The attributes of the area's class, as written (not imported)."""
    tree = ast.parse((HERE / f"{module}.py").read_text())
    for cls in tree.body:
        if not isinstance(cls, ast.ClassDef):
            continue
        values = {}
        for node in cls.body:
            if isinstance(node, ast.Assign) and len(node.targets) == 1 \
                    and isinstance(node.targets[0], ast.Name):
                try:
                    values[node.targets[0].id] = ast.literal_eval(node.value)
                except ValueError:
                    pass
        if values.get("name") == name:
            return {k: values[k] for k in ATTRIBUTES if k in values}
    return None


def all_facts():
    return {name: read(name, module) for name, module in areas()}


# ---- the checks -----------------------------------------------------------------------

def crate_dir(crate):
    for base in ("core", "faraday"):
        d = ROOT / base / crate.replace("_", "-")
        if (d / "Cargo.toml").is_file():
            return d
    return None


def resolves(token):
    """Whether a path a `faraday` attribute names exists: `crate::module::item`
    (a module is a file or folder of the crate's `src/`, an item a word in the
    deepest module's file), `file.rs` or `folder/` somewhere under `core/` or
    `faraday/`. None for a token that is not a Faraday path (a library's)."""
    if re.fullmatch(r"faraday_[a-z0-9]+(::\w+)*", token):
        crate, *rest = token.split("::")
        d = crate_dir(crate)
        if d is None:
            return False
        folder, current = d / "src", d / "src" / "lib.rs"
        while rest:
            if (folder / f"{rest[0]}.rs").is_file():
                current, folder = folder / f"{rest[0]}.rs", folder / rest[0]
            elif (folder / rest[0] / "mod.rs").is_file():
                current, folder = folder / rest[0] / "mod.rs", folder / rest[0]
            else:
                break
            rest.pop(0)
        return current.is_file() and all(
            re.search(rf"\b{re.escape(w)}\b", current.read_text()) for w in rest)
    if token.endswith(".rs") or token.endswith("/"):
        for base in ("core", "faraday"):
            for hit in (ROOT / base).rglob(token.rstrip("/")):
                if "target" not in hit.parts and (hit.is_file() == token.endswith(".rs")):
                    return True
        return False
    return None


def problems(facts):
    out = []
    for name, got in facts.items():
        if got is None:
            out.append(f"{name}: no class with name = {name!r}")
            continue
        for attr in ATTRIBUTES:
            value = got.get(attr)
            if attr in LISTS:
                ok = isinstance(value, list) and value and all(
                    isinstance(v, str) and v.strip() for v in value)
            else:
                ok = isinstance(value, str) and value.strip()
            if not ok:
                out.append(f"{name}: `{attr}` is missing or empty")
        named = 0
        for token in re.findall(r"`([^`]+)`", got.get("faraday") or ""):
            found = resolves(token)
            if found is None:
                continue
            named += 1
            if not found:
                out.append(f"{name}: `{token}` named in `faraday` does not resolve")
        if got.get("faraday") and not named:
            out.append(f"{name}: `faraday` names no path in the repository")
    return out


# ---- the README -----------------------------------------------------------------------

def cell(text):
    return text.replace("|", "\\|")


def wrap(text, first, rest):
    return textwrap.fill(text, width=72, initial_indent=first, subsequent_indent=rest,
                         break_long_words=False, break_on_hyphens=False)


def what_runs(facts):
    lines = ["| Area | Faraday | Reference (`tools/reference/`) | Published vectors | Random corpus |",
             "|---|---|---|---|---|"]
    for name, f in facts.items():
        lines.append(f"| `{name}` | " + " | ".join(
            cell(f[k]) for k in ("faraday", "reference", "vectors", "corpus")) + " |")
    lines += ["", "### How each reference is checked", "",
              wrap("What each area checks its reference by before any case, and what it "
                   "leaves uncompared.", "", ""), ""]
    for name, f in facts.items():
        text = f["reference_checked"]
        if f["not_compared"] != NONE:
            text += ". Not compared: " + f["not_compared"]
        lines.append(wrap(f"`{name}`: {text.rstrip('.')}.", "- ", "  "))
    return "\n".join(lines)


def known_differences(facts):
    lines = ["### Known differences", "",
             wrap("Each is a place where Faraday and the reference differ, or differed once "
                  "and now agree. The corpus keeps out of it, or the area checks it to "
                  "still hold exactly and counts it in the run's output (\"N of them a "
                  "known difference\"), and says why.", "", ""), ""]
    for name, f in facts.items():
        if f["differences"] == [NONE]:
            continue
        lines.append(f"- `{name}`")
        for d in f["differences"]:
            lines.append(wrap(d, "  - ", "    "))
    return "\n".join(lines)


def render(facts):
    return {"what-runs": what_runs(facts), "known-differences": known_differences(facts)}


def splice(text, regions):
    for key, body in regions.items():
        begin, end = f"<!-- facts:begin {key} -->\n", f"<!-- facts:end {key} -->"
        a, b = text.find(begin), text.find(end)
        if a < 0 or b < a:
            raise SystemExit(f"facts: README has no marker lines for {key}")
        text = text[:a + len(begin)] + body + "\n" + text[b:]
    return text


def main():
    cmd = sys.argv[1] if len(sys.argv) > 1 else "check"
    if cmd not in ("check", "write"):
        print(__doc__)
        return 2
    facts = all_facts()
    bad = problems(facts)
    if bad:
        print("facts: the areas' attributes:")
        for line in bad:
            print("  " + line)
        return 1
    current = README.read_text()
    wanted = splice(current, render(facts))
    if cmd == "write":
        README.write_text(wanted)
        return 0
    if wanted != current:
        print("facts: tools/crosscheck/README.md is not what the areas say: "
              "run `python3 -I -B tools/crosscheck/facts.py write`")
        return 1
    print(f"facts: ok ({len(facts)} areas)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
