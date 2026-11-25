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
    def __init__(self, name, patterns, paths, use, skip=(), tests=False, not_in_fn=False, near=()):
        self.name = name
        self.patterns = [re.compile(p) for p in patterns]
        self.paths = paths    # path prefixes looked in
        self.skip = skip      # path prefixes not looked in
        self.use = use        # the part to use instead
        self.tests = tests    # also count tests and cfg(test) code
        # (trigger, context): a line with the trigger is a hit when it or the next line has the context
        self.near = [(re.compile(a), re.compile(b)) for a, b in near]
        self.not_in_fn = not_in_fn  # a hit inside a fn's parameter list is not counted


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
    # A step screen that draws its own column, foot or paging: every step flow
    # is drawn by the flow container (docs/FLOW-CONTAINER.md), which owns them.
    # Whether a flow declares `fn contained` cannot be counted by a line
    # pattern (the trait's default returns None), so the rule holds the calls.
    Rule(
        "flow-own-layout",
        [r"\bcolumn_foot\(", r"\bcolumn_wide\(", r"\bpaged\("],
        ["faraday/"],
        "the flow container (docs/FLOW-CONTAINER.md, faraday-core flow.rs)",
        skip=["faraday/faraday-core/src/flow.rs", "faraday/faraday-core/src/flow/"],
    ),
    # A menu page doing its own page width or column rule: the page width and
    # the tile minimum belong to the container (`docs/MENU-PAGE.md`), which
    # works out the columns; a count of columns from a minimum and a gap is
    # that rule written by hand.
    Rule(
        "menu-own-grid",
        [r"\bMENU_PAGE_W\b", r"\bREADING_MENU_W\b", r"\bCATALOG_TILE_MIN\b",
         r"\+\s*\w*GAP\w*\s*\)\s*/\s*\(\s*\w*(MIN|TILE)\w*\s*\+"],
        ["faraday/"],
        "the menu page container (faraday-core screens/menu_page.rs, `docs/MENU-PAGE.md`)",
        skip=["faraday/faraday-core/src/screens/menu_page.rs",
              "faraday/faraday-core/src/screens/hub_layout/",
              "faraday/faraday-core/src/screens/mod.rs",
              "faraday/faraday-core/src/screens/session_pane/"],
    ),
    # A page or a flow working out where a pane stands: the panes' widths and
    # the room a pane leaves belong to the pane container
    # (`docs/HUB-LAYOUT.md`); a page declares its panes and asks for the place.
    Rule(
        "pane-own-place",
        [r"\b(LIST_PANE|SUPPORT_PANE|CHOICES_PANE_MIN|CHOICES_PANE_MAX|FLOW_CARD_MIN|MENU_TILE_WIDE_MAX)\b",
         r"\bside_panel_w\(", r"\bsupport_pane_room\("],
        ["faraday/faraday-core/src/"],
        "the pane container (faraday-core screens/hub_layout/, `docs/HUB-LAYOUT.md`)",
        skip=["faraday/faraday-core/src/screens/hub_layout/"],
    ),
    # A page timing a change of page itself: how long it takes at the Speed
    # and what a Motion style does in it belong to the pane container. Reading
    # the two settings to show them, or handing them to it, is not counted.
    Rule(
        "page-own-time",
        [r"\bSTEP_FADE_MS\b", r"\bmotion_speed\(\)\s*\.ms\(", r"\bKind::of\([^)]*motion_style",
         r"\bMotionStyle::\w+\s*=>\s*(Some\()?Kind\b"],
        ["faraday/faraday-core/src/"],
        "the pane container's `page_turn` and `STEP_MS` (faraday-core screens/hub_layout/, `docs/HUB-LAYOUT.md`)",
        skip=["faraday/faraday-core/src/screens/hub_layout/"],
    ),
    # A file drawing a row of what the session holds itself: the rows of
    # keys, wallets and what waits, and the card that holds them, belong to
    # the session pane (`docs/SESSION-PANE.md`), which every page draws them
    # through.
    Rule(
        "session-pane-own-rows",
        [r"\bSessionRow\b"],
        ["faraday/"],
        "the session pane (faraday-core screens/session_pane/, `docs/SESSION-PANE.md`)",
        skip=["faraday/faraday-core/src/screens/session_pane/"],
    ),
    # A flow listing the keys or wallets the session holds to make a row of
    # its pick: a flow declares what it takes, refuses and offers as ways in,
    # and the session pane lists, orders and draws the rows
    # (`screens/session_pane/pick.rs`, `docs/WORKSPACE.md` §2, "The pick").
    # The chain rustfmt splits (`.keys` on one line, `.all()` on the next) is
    # held too. An alias (`let ks = &app.session.keys;`), a helper that
    # returns the list, `for k in &keys` and `keys.len()` with `keys.info(i)`
    # are not: a line lint cannot hold them.
    Rule(
        "pick-own-rows",
        [r"\bkeys\s*\.all\(\)", r"\bwallets\s*\.iter\(\)"],
        ["faraday/faraday-core/src/"],
        "the session pane's pick (faraday-core screens/session_pane/pick.rs: `Decl::keys`, `wallets`, `files`)",
        skip=["faraday/faraday-core/src/screens/session_pane/"],
        near=[(r"\b(keys|wallets)\s*$", r"\b(keys\s+\.all|wallets\s+\.iter)\(\)")],
    ),
    # A page drawing its own section card or row: the card, its list rows,
    # dividers and columns belong to the container (`docs/MENU-PAGE.md` §4); a
    # page gives it sections of faces.
    Rule(
        "menu-own-card",
        [r"\bMENU_ROW_\w+\b", r"\bMENU_CARD_COLUMNS\b", r"\bMENU_TAG_\w+\b"],
        ["faraday/"],
        "the menu page container (faraday-core screens/menu_page.rs, `docs/MENU-PAGE.md`)",
        skip=["faraday/faraday-core/src/screens/menu_page.rs"],
    ),
    # A fact in an outline (F332, docs/WORKSPACE.md §19): `Ui::pill` and
    # `pill_in` draw an outline and take no press, so a call is a fact that
    # looks like a button. A fact is a tag (`Ui::tag`, a status or neutral).
    Rule(
        "fact-in-outline",
        [r"\.pill\(", r"\.pill_in\("],
        ["faraday/"],
        "a neutral tag (`Ui::tag` with `Status::Neutral`, faraday-ui)",
        skip=["faraday/faraday-ui/src/lib.rs"],
    ),
    # Text in the accent in a status or tag: accent text is for presses only
    # (F332); a status is a tag in its container's colours. A line lint sees
    # the tag and the accent within a line of each other.
    Rule(
        "accent-in-status",
        [],
        ["faraday/faraday-core/src/", "faraday/faraday-starter/src/"],
        "a status tag (`Ui::tag` with a `Status`, faraday-ui)",
        near=[(r"\bRole::Primary\b", r"\b(tag|status|chip|badge)\w*\b")],
    ),
    # A chevron drawn by hand: the one that ends a row is `Ui::row_chevron`
    # at the token `CHEVRON` (F332, DESIGN §4.13). The older tokens are gone.
    Rule(
        "chevron-own-size",
        [r"\bIcon::ChevronRight\b", r"\bROW_CHEVRON\b", r"\bMENU_ROW_CHEVRON\b"],
        ["faraday/"],
        "`Ui::row_chevron` (faraday-ui) at the token `CHEVRON`",
        skip=["faraday/faraday-ui/src/lib.rs"],
    ),
    # A passphrase held or filtered outside the one field (docs/KEYS.md §2): a
    # struct field of type `SecretText` or `TextBox` named `pass*`,
    # `*passphrase*` or `again`, and the printable-ASCII key filter where it
    # feeds a passphrase (a line that names one; a filter of a value that is
    # not a passphrase, such as a copy of a backup typed back whole, is not
    # counted). A job's or a result's copy handed on (`PassField::take`,
    # `job_copy`) is an `Option<SecretText>`, which a screen does not type into,
    # and a function's parameter is a copy passed in, not a field: neither is
    # counted. The filter's arm and its body may be on two lines: the filter
    # counts when it or the line after names a passphrase. A field named
    # `text: TextBox` is counted too (`vault::Prompt.text`, a passphrase for a
    # key or an export; the one pair of prompts still to move onto the field).
    # A qualified path (`crate::secret_text::SecretText`) counts as the
    # bare type.
    Rule(
        "pass-field-outside",
        [r"^\s*(pub(\([a-z:]+\))?\s+)?(pass\w*|\w*passphrase\w*|again)\s*:\s*(\w+::)*(SecretText|TextBox)\s*,?\s*$",
         r"^\s*(pub(\([a-z:]+\))?\s+)?text\s*:\s*(\w+::)*TextBox\s*,?\s*$"],
        ["faraday/faraday-core/"],
        "`pass_field::PassField` (faraday-core src/pass_field/, docs/KEYS.md §2)",
        skip=["faraday/faraday-core/src/pass_field/"],
        not_in_fn=True,
        near=[(r"\(' '\.\.='~'\)", r"(?i)pass")],
    ),
    # A list of key forms written out by hand: an array, slice or `vec!` of two
    # or more `Form::` variants (faraday-forms' `Form`, docs/KEY-FORMS.md). The
    # registry's own `Form::ALL` and the openers' `forms` in `ways_in` are the
    # lists; a screen or flow reads them. An exhaustive `match` on a form is the
    # registry's own map and is not a list, so `|` patterns are not counted.
    Rule(
        "form-list-outside",
        [r"\[\s*(\w+::)*(?<![A-Za-z0-9_])Form::(Words|Slip39|Codex32|Xor|Krux)\b\s*,\s*(\w+::)*(?<![A-Za-z0-9_])Form::(Words|Slip39|Codex32|Xor|Krux)\b"],
        ["faraday/faraday-core/"],
        "the registry's `Form::ALL` or an opener's forms (faraday-core src/ways_in/, docs/KEY-FORMS.md)",
        skip=["faraday/faraday-core/src/ways_in/", "faraday/faraday-core/src/forms.rs"],
    ),
    # A way in to a key drawn by hand: a row or button whose action is one of
    # the entries Type the words, Scan, a form, a word count, built outside the
    # ways-in component (docs/DESIGN.md §4). A match arm that handles the action
    # (`=>` or `|` after it) and an action run at once (`act(`, `press(`) are
    # not a way in.
    Rule(
        "way-in-outside",
        [r"(?<!act\()(?<!press\()(?<!act\(crate::)(?<!press\(crate::)\bAction::(ScanSeed|RSlotScan|RSlotWords|RSlotForm|EntryForm|EntryWalletForm|EntryWordCount)\b(\([^()]*\))?\s*[,)]",
         r"\bSignerPress::Seed(Scan|Words|Form)\b(\([^()]*\))?\s*[,)]"],
        ["faraday/faraday-core/src/"],
        "the ways-in component (faraday-core src/ways_in/, docs/DESIGN.md §4)",
        skip=["faraday/faraday-core/src/ways_in/", "faraday/faraday-core/src/work/"],
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


def fn_signature_lines(lines):
    """The line numbers inside a fn signature, from `fn name` to its `{` or `;` outside the parentheses."""
    out, open_sig, depth = set(), False, 0
    for n, line in lines:
        if not open_sig and re.search(r"\bfn\s+\w+", line):
            open_sig, depth = True, 0
        if open_sig:
            out.add(n)
            depth += line.count("(") - line.count(")")
            if depth <= 0 and re.search(r"[{;]\s*$", line):
                open_sig = False
    return out


def hit_lines(rule, text):
    """The line numbers of the rule's hits in one file's text."""
    lines = code_lines(text, rule.tests)
    sig = fn_signature_lines(lines) if rule.not_in_fn else set()
    return [n for i, (n, l) in enumerate(lines)
            if n not in sig and (any(p.search(l) for p in rule.patterns)
                                 or any(a.search(l) and b.search(l + " " + (lines[i + 1][1] if i + 1 < len(lines) else ""))
                                        for a, b in rule.near))]


def scan(rule, read=None):
    """{path: [line numbers]} of the rule's hits."""
    read = read or (lambda p: open(os.path.join(ROOT, p), encoding="utf-8").read())
    hits = {}
    for f in tracked(rule.paths):
        if f.startswith(tuple(rule.skip)) or (not rule.tests and "/tests/" in f):
            continue
        found = hit_lines(rule, read(f))
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
