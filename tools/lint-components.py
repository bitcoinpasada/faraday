#!/usr/bin/env python3
# Components lint: the standard parts stay in place and new work lands in its
# proper place. Each rule below counts the lines that do a thing by hand where
# a standard part does it; counts per rule and file are held in
# tools/lint-components.counts and only fall, as lint-tokens and lint-layout do:
#
#   - a file over its count fails (use the part named);
#   - a file under its count fails until the count is lowered (`--write`);
#   - a file not listed allows none;
#   - a rule with no line left in the counts file is banned: any hit fails.
#
# Lines in comments, in `#[cfg(test)]` items and under a `tests/` directory are
# not counted unless the rule sets `tests=True`. A rule's `skip` paths (the
# part's own folder) are not looked at.
#
# Add a rule: one `Rule(...)` entry in RULES, then `--write` (it records the
# counts the tree has). Raising a count is the owner's decision, recorded in
# DECISIONS; `--write` never raises or adds one.
#
# Run from anywhere as `python3 -I -B tools/lint-components.py [--write]`;
# standard library only. Prints one line per failure and exits non-zero.

import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
COUNTS = "tools/lint-components.counts"
HEADER = """\
# Counts held by tools/lint-components.py. Each only falls: a file over its count
# fails, one under it fails until the count is lowered (`--write`), a file not
# listed allows none, and a rule with no line here is banned. Raising or adding
# a count is the owner's decision, recorded in DECISIONS.
#
# Format: RULE PATH COUNT
"""


class Rule:
    def __init__(self, name, patterns, paths, use, skip=(), tests=False):
        self.name = name
        self.patterns = [re.compile(p) for p in patterns]
        self.paths = paths    # path prefixes looked in
        self.skip = skip      # path prefixes not looked in
        self.use = use        # the part to use instead
        self.tests = tests    # also count tests and cfg(test) code


RULES = [
    # A wallet export kind written as a number where a `PublicFile` variant
    # belongs; the actions keep `u8` on the wire (`from_wire` / `.wire()`).
    Rule(
        "public-kind-number",
        [r"\bAction::PublicOut\([^()]*,\s*\d+\s*\)",
         r"\bAction::PublicRemove\([^()]*,\s*\d+\s*\)",
         r"\bAction::BOut\(\s*\d+\s*\)"],
        ["faraday/"],
        "a `PublicFile` variant, `.wire()` (faraday-core public_kind.rs)",
        skip=["faraday/faraday-core/src/public_kind.rs"],
    ),
]


def tracked(prefixes):
    out = subprocess.run(["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard", "*.rs"],
                         check=True, capture_output=True, text=True, cwd=ROOT).stdout
    return sorted(f for f in set(out.split("\0"))
                  if f and f.startswith(tuple(prefixes)) and os.path.exists(os.path.join(ROOT, f)))


def code_lines(text, tests):
    """(line number, line) of the code lines: comments and, unless `tests`, cfg(test) items dropped."""
    out, depth, skip_until, pending = [], 0, None, False
    for n, line in enumerate(text.split("\n"), 1):
        code = re.sub(r"//.*", "", line)
        if not tests:
            if re.match(r"\s*#\[cfg\(test\)\]", code):
                pending = True
                continue
            if pending and skip_until is None and code.strip():
                skip_until = depth
                pending = False
                if "{" not in code and code.rstrip().endswith(";"):
                    skip_until = None
                    continue
            if skip_until is not None:
                depth += code.count("{") - code.count("}")
                if depth <= skip_until and "}" in code or (depth <= skip_until and "{" not in code
                                                           and code.rstrip().endswith(";")):
                    skip_until = None
                continue
        depth += code.count("{") - code.count("}")
        out.append((n, code))
    return out


def scan(rule, read=None):
    """{path: [line numbers]} of the rule's hits."""
    read = read or (lambda p: open(os.path.join(ROOT, p), encoding="utf-8").read())
    hits = {}
    for f in tracked(rule.paths):
        if f.startswith(tuple(rule.skip)) or (not rule.tests and "/tests/" in f):
            continue
        found = [n for n, l in code_lines(read(f), rule.tests) if any(p.search(l) for p in rule.patterns)]
        if found:
            hits[f] = found
    return hits


def load(path):
    counts = {}
    if os.path.exists(path):
        for l in open(path, encoding="utf-8"):
            l = l.strip()
            if l and not l.startswith("#"):
                rule, f, n = l.split()
                counts[(rule, f)] = int(n)
    return counts


def check(rules, counts, hits_of):
    """Failure lines for the hits against the held counts."""
    failures = []
    for r in rules:
        hits = hits_of(r)
        held = {f: n for (name, f), n in counts.items() if name == r.name}
        for f, lines in sorted(hits.items()):
            if not held:
                for n in lines:
                    failures.append(f"lint-components: {f}:{n}: {r.name}: use {r.use}")
            elif len(lines) > held.get(f, 0):
                failures.append(f"lint-components: {f}: {r.name}: {len(lines)} over {held.get(f, 0)} "
                                f"(lines {','.join(map(str, lines))}): use {r.use}")
        for f, n in sorted(held.items()):
            if len(hits.get(f, [])) < n:
                failures.append(f"lint-components: {f}: {r.name}: {len(hits.get(f, []))} under {n}: "
                                f"lower the count (`--write`)")
    names = {r.name for r in rules}
    for (name, f) in sorted(counts):
        if name not in names:
            failures.append(f"lint-components: {COUNTS}: unknown rule {name}")
    return failures


def lowered(rules, counts, hits_of):
    """The counts after `--write`: held counts lowered to the tree's, none added or raised."""
    new = {}
    for r in rules:
        hits = hits_of(r)
        for (name, f), n in counts.items():
            if name == r.name and len(hits.get(f, [])) > 0:
                new[(name, f)] = min(n, len(hits[f]))
    return new


def main(argv):
    os.chdir(ROOT)
    counts = load(COUNTS)
    cache = {}
    def hits_of(r):
        if r.name not in cache:
            cache[r.name] = scan(r)
        return cache[r.name]
    if "--write" in argv:
        new = lowered(RULES, counts, hits_of)
        body = "".join(f"{name} {f} {n}\n" for (name, f), n in sorted(new.items()))
        open(COUNTS, "w", encoding="utf-8").write(HEADER + "\n" + body)
        print(f"lint-components: wrote {COUNTS}")
    failures = check(RULES, counts, hits_of) if "--write" not in argv else check(RULES, new, hits_of)
    for f in failures:
        print(f)
    if not failures:
        print("lint-components: ok")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
