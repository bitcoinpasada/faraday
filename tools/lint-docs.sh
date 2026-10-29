#!/usr/bin/env bash
# Documentation lint (docs/README.md, docs/STRUCTURE-PASS.md phase 2).
#
# Six rules, so that a document says what kind it is, cites only what
# exists, and holds what is true now or still to be built, not how it was
# built (CLAUDE.md, "Docs").
#
# 1. Header. Every tracked .md under docs/ (not docs/history/,
#    docs/learn/ or docs/deps/) carries `**Kind:**` and `**Status:**`
#    in its first 12 lines. The root README.md is the project's front
#    page and has none. Each .md under docs/history/
#    carries `**Kind:** History` there.
#
# 2. Paths. Every backticked repository path in a .md file resolves: a
#    span that starts with core/, faraday/, tools/, docs/, fuzz/,
#    third_party/, design/ or supply-chain/, or is a root file
#    (README.md, CLAUDE.md, Cargo.toml, justfile and the like).
#    Before the check a trailing `:123`, `#anchor`, ` §...` and trailing
#    punctuation are stripped. Not checked:
#      - spans containing * < { … or $ (patterns and templates)
#      - fenced code blocks
#      - anything under docs/history/ as a source (records)
#      - out/ paths and target directories (build outputs)
#    A local/ path anywhere outside CLAUDE.md is an error: local/ is
#    private and never cited.
#
# 3. Open work. A Spec carries `**Open:**` in its first 12 lines, saying
#    what is not built; a Reference or Record carries none. A Spec with
#    nothing open is Reference, and its plan goes to docs/history/.
#
# 4. No plan in a Reference. A Spec is a plan and may hold one; a
#    Reference is what is built, so a heading of plan or build log in it
#    (rules for the implementing agent, batches, order of work, how it
#    gets built, what changes from today, estimates, questions for the
#    owner, docs-and-tour task lists) fails, and so does a line of build
#    log in its text: a dated "Built 2026-10-19", "Specified, not built",
#    or words for the implementing agent. A section whose whole body is a
#    stub naming where it moved ("Moved to `docs/history/X-LOG.md`.")
#    passes, so a cited section number keeps resolving.
#
# 5. Decisions. docs/DECISIONS.md's table has one row per decision, F1
#    upward with none missing or twice; each row links its file, under
#    docs/history/decisions/ when its status is Replaced, Superseded or
#    Withdrawn and under docs/decisions/ otherwise; the file's Status
#    line is the row's; and every file in either folder has a row.
#
# 6. Held by. Every decision in docs/decisions/ (Status Active or
#    Amended by ...) carries `**Held by:**` in its first 12 lines: a comma
#    list of rule ids of tools/impact.toml, each of which exists, or one
#    of the words `review`, `tests` or `none`; `tests` only where a Rust
#    file of the tree names the decision (a test or the code it tests).
#    Every rule whose `source` names a decision file is named in that
#    decision's line (docs/IMPACT.md §8). Retired decisions are exempt.
#
# 7. Recipes. Every `just <recipe>` that starts a backticked span or a line
#    of a fenced block in a .md file (not docs/history/) names a recipe of
#    justfile, faraday/faraday.just or an import of them. Variable
#    assignments (`build=compatible`) and options (`--fresh`) before the
#    recipe are skipped; its arguments are not checked.
#
# Prints `file:line: message` for each failure and exits non-zero if
# there is one. Run from the repository root; `just lint` does.
set -u
cd "$(dirname "$0")/.."

exec python3 - <<'PY'
import os
import re
import subprocess
import sys

def ls(*args):
    return subprocess.run(["git", "ls-files", *args, "-z"], check=True,
                          capture_output=True, text=True).stdout.split("\0")


# Tracked files, and files not yet added below the root (a document or a
# tool being written is checked before it is committed; a stray file at
# the root is not).
listing = ls("--cached") + [f for f in ls("--others", "--exclude-standard")
                            if "/" in f]
files = sorted(f for f in listing if f and os.path.exists(f))
known = set(files)
dirs = set()
for f in files:
    d = os.path.dirname(f)
    while d:
        dirs.add(d)
        d = os.path.dirname(d)
root_files = {f for f in files if "/" not in f}
ROOT_NAMES = {"README.md", "CLAUDE.md", "Cargo.toml", "Cargo.lock", "justfile",
              "LICENSE", "SECURITY.md", "deny.toml"}

PREFIXES = ("core/", "faraday/", "tools/", "docs/", "fuzz/",
            "third_party/", "design/", "supply-chain/")
SKIP_CHARS = ("*", "<", "{", "…", "$")
failures = []


def fail(path, line, msg):
    failures.append(f"{path}:{line}: {msg}")


# Rule 4: no plan in a Reference.
PLAN = re.compile(
    r"implementing agent|for the agent|order of work|how (this|it) gets built"
    r"|building it|what changes from today|docs and (snapshots|tour)"
    r"|\bbatch(es)?\b|estimate|questions for the owner|open questions", re.I)
LOG_LINE = re.compile(
    r"(^|[.;:]\s+|\()\**Built( on)? \(?\d{4}-\d{2}-\d{2}"
    r"|specified, not built|implementing agent", re.I)
STUB = re.compile(r"^(.*: )?[Mm]oved to `docs/history/[^`]+`\.?$")


def check_log(f, kind):
    if kind != "Reference":
        return
    lines = open(f, encoding="utf-8").read().split("\n")
    fenced = False
    heads = []
    for n, l in enumerate(lines):
        if re.match(r"\s*(```|~~~)", l):
            fenced = not fenced
            continue
        if fenced:
            continue
        m = re.match(r"(#+)\s+(.*)", l)
        if m:
            heads.append((n, len(m.group(1)), m.group(2)))
        elif not STUB.match(l.strip()) and LOG_LINE.search(l):
            fail(f, n + 1, "a line of build log in a Reference; move it to "
                           "docs/history/")
    for i, (n, level, title) in enumerate(heads):
        if not PLAN.search(title):
            continue
        end = next((m for m, lv, _ in heads[i + 1:] if lv <= level), len(lines))
        body = [l.strip() for l in lines[n + 1:end] if l.strip()]
        if len(body) == 1 and STUB.match(body[0]):
            continue
        fail(f, n + 1, f"heading `{title}` is plan or build log in a "
                       "Reference; move the section to docs/history/ and "
                       "leave a one-line stub")


# Rule 5: the decisions and their index.
def check_decisions():
    index = "docs/DECISIONS.md"
    if index not in known:
        return
    rows = {}
    for n, l in enumerate(open(index, encoding="utf-8").read().split("\n"), 1):
        m = re.match(r"\| \[F(\d+)\]\(([^)]*)\) \|.*\| ([^|]+?) \| [^|]+ \|$", l)
        if not m:
            continue
        num, link, status = int(m.group(1)), m.group(2), m.group(3)
        if num in rows:
            fail(index, n, f"F{num} has two rows")
        rows[num] = (n, link, status)
        retired = status.startswith(("Replaced", "Superseded", "Withdrawn"))
        want = ("history/decisions/" if retired else "decisions/") + f"F{num}.md"
        if link != want:
            fail(index, n, f"F{num} links `{link}`; with status "
                           f"`{status}` its file is `docs/{want}`")
            continue
        path = "docs/" + link
        if path not in known:
            fail(index, n, f"`{path}` does not exist")
            continue
        head = open(path, encoding="utf-8").read().split("\n")[:12]
        st = next((l[len("**Status:** "):].strip() for l in head
                   if l.startswith("**Status:**")), None)
        if st != status:
            fail(path, 1, f"Status `{st}` is not the index's `{status}`")
    for num in range(1, max(rows, default=0) + 1):
        if num not in rows:
            fail(index, 1, f"no row for F{num}")
    for f in files:
        m = re.match(r"docs/(history/)?decisions/F(\d+)\.md$", f)
        if m and int(m.group(2)) not in rows:
            fail(f, 1, f"F{m.group(2)} has no row in docs/DECISIONS.md")


check_decisions()


# Rule 6: each active decision says what holds it.
def check_held_by():
    import tomllib
    try:
        with open("tools/impact.toml", "rb") as fh:
            rules = tomllib.load(fh).get("rule", [])
    except OSError:
        return
    ids = {r.get("id") for r in rules}
    named = {}
    for r in rules:
        for src in r.get("source", []):
            m = re.match(r"docs/decisions/(F\d+)\.md", src)
            if m:
                named.setdefault(m.group(1), []).append(r["id"])
    import subprocess
    cited = set(subprocess.run(["git", "grep", "-ohwE", "F[0-9]+", "--", "*.rs"],
                               capture_output=True, text=True).stdout.split())
    for f in files:
        m = re.match(r"docs/decisions/(F\d+)\.md$", f)
        if not m:
            continue
        head = open(f, encoding="utf-8").read().split("\n")[:12]
        status = next((l[len("**Status:** "):] for l in head
                       if l.startswith("**Status:**")), "")
        if not status.startswith(("Active", "Amended")):
            continue
        line = next((l[len("**Held by:**"):].strip() for l in head
                     if l.startswith("**Held by:**")), None)
        if line is None:
            fail(f, 1, "no `**Held by:**` line in the first 12 lines")
            continue
        words = [w.strip() for w in line.split(",")]
        if words in (["review"], ["tests"], ["none"]):
            if m.group(1) in named:
                fail(f, 1, f"`**Held by:** {words[0]}` but rules "
                           f"{', '.join(named[m.group(1)])} name it as source")
            elif words == ["tests"] and m.group(1) not in cited:
                fail(f, 1, f"`**Held by:** tests` but no Rust file names "
                           f"{m.group(1)}: name it in its test, or say review")
            continue
        for w in words:
            if w not in ids:
                fail(f, 1, f"Held by `{w}` is not a rule of "
                           f"tools/impact.toml (or review, tests, none)")
        for rid in named.get(m.group(1), []):
            if rid not in words:
                fail(f, 1, f"rule `{rid}` has this decision as its source "
                           f"and is not in its Held by line")


check_held_by()


# Rule 1: headers.
for f in files:
    if not f.endswith(".md"):
        continue
    covered = (
        f.startswith("docs/")
        and not f.startswith(("docs/history/", "docs/learn/", "docs/deps/"))
    )
    history = f.startswith("docs/history/")
    if not (covered or history):
        continue
    head = open(f, encoding="utf-8").read().split("\n")[:12]
    if history:
        if not any(l.startswith("**Kind:** History") for l in head):
            fail(f, 1, "no `**Kind:** History` in the first 12 lines")
        if not any(l.startswith("**Status:**") for l in head):
            fail(f, 1, "no `**Status:**` in the first 12 lines")
        continue
    kinds = [l for l in head if l.startswith("**Kind:**")]
    if not kinds:
        fail(f, 1, "no `**Kind:**` in the first 12 lines")
    elif not re.match(r"\*\*Kind:\*\* (Reference|Spec|Record)\s*$", kinds[0]):
        fail(f, 1, "Kind is not Reference, Spec or Record")
    if not any(l.startswith("**Status:**") and len(l) > 12 for l in head):
        fail(f, 1, "no `**Status:**` line in the first 12 lines")
    kind = kinds[0].split()[-1] if kinds else ""
    opens = [l for l in head if l.startswith("**Open:**")]
    if kind == "Spec" and not any(len(l) > 10 for l in opens):
        fail(f, 1, "a Spec with no `**Open:**` line; if nothing is open, "
                   "it is Reference and its plan goes to docs/history/")
    if kind in ("Reference", "Record") and opens:
        fail(f, 1, f"an `**Open:**` line in a {kind}; open work makes it a Spec")
    check_log(f, kind)


# Rule 2: paths.
def resolves(p):
    return p in known or p in dirs


def check_span(f, line, span):
    if any(c in span for c in SKIP_CHARS):
        return
    tok = span.split()
    if not tok:
        return
    p = tok[0]
    if p.startswith("local/") or "/local/" in p:
        return  # reported by the local/ check
    if not (p.startswith(PREFIXES)
            or re.sub(r"[:#].*$", "", p) in root_files | ROOT_NAMES):
        return
    p = re.sub(r"#.*$", "", p)
    p = re.sub(r":\d+(-\d+)?$", "", p)
    p = re.sub(r"[.,;:)\]]+$", "", p)
    if p.startswith("out/") or p.endswith("/target"):
        return
    q = p.rstrip("/")
    if q and not resolves(q):
        fail(f, line, f"path `{p}` does not exist")


for f in files:
    if not f.endswith(".md") or f.startswith("docs/history/"):
        continue
    raw = open(f, encoding="utf-8").read().split("\n")
    fenced = False
    lines = []
    for l in raw:
        if re.match(r"\s*(```|~~~)", l):
            fenced = not fenced
            lines.append("")
        else:
            lines.append("" if fenced else l)
    text = "\n".join(lines)
    for m in re.finditer(r"`([^`\n]*(?:\n[^`\n]+)?)`", text):
        line = text.count("\n", 0, m.start()) + 1
        check_span(f, line, m.group(1))
    if f != "CLAUDE.md":
        for n, l in enumerate(lines, 1):
            if re.search(r"(?<![\w/.-])local/", l):
                fail(f, n, "cites local/, which is private")

# Rule 7: recipes named in docs exist.
def recipes():
    names = set()
    for jf in ["justfile", "faraday/faraday.just"] + [
            f for f in files if f.endswith(".just")]:
        if not os.path.exists(jf):
            continue
        for l in open(jf, encoding="utf-8"):
            m = re.match(r"@?([A-Za-z_][\w-]*)(?![\w-])", l)
            if (m and m.group(1) not in ("set", "import", "alias", "mod", "export")
                    and not re.match(r"@?[\w-]+\s*(:=|=)", l)
                    and re.search(r":(?!=)", l)):
                names.add(m.group(1))
            m = re.match(r"alias\s+([\w-]+)\s*:=", l)
            if m:
                names.add(m.group(1))
    return names


JUST = re.compile(r"^(?:\$ )?just((?: (?:[A-Za-z_][\w-]*=\S*|-\S+))*) ([A-Za-z_][\w-]*)")
RECIPES = recipes()


def check_just(f, n, text):
    m = JUST.match(text.strip())
    if m and m.group(2) not in RECIPES:
        fail(f, n, f"`just {m.group(2)}` is not a recipe of justfile or "
                   "faraday/faraday.just")


for f in files:
    if not f.endswith(".md") or f.startswith("docs/history/"):
        continue
    fenced = False
    for n, l in enumerate(open(f, encoding="utf-8").read().split("\n"), 1):
        if re.match(r"\s*(```|~~~)", l):
            fenced = not fenced
        elif fenced:
            check_just(f, n, l)
        else:
            for m in re.finditer(r"`([^`\n]+)`", l):
                check_just(f, n, m.group(1))

for m in failures:
    print(m)
if failures:
    print(f"lint-docs: {len(failures)} failure(s)", file=sys.stderr)
    sys.exit(1)
PY
