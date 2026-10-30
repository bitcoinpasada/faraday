#!/usr/bin/env python3
"""What a change brings with it (docs/IMPACT.md, F149).

Reads the registry tools/impact.toml (and the rule files named in the
environment variable FARADAY_IMPACT_EXTRA, separated by ":") and answers,
from the paths a change touches, what must change with it:

    impact.py plan [<path>...] [--at <moment>] [--long]
    impact.py check [<range>] [--quiet] [--paths <path>...] [--message <file>]
                    (FARADAY_IMPACT_MESSAGE=<file> as --message, relative to the repository)
    impact.py lint
    impact.py docs [--check]
    impact.py --self-test

`plan` is for a task's brief; `check` is the landing check, run over a
range of commits (default: the merge base of main and HEAD, plus the
working tree); `lint` checks the registry against itself and the tree; `docs` writes
docs/CHECKS.md from the public registry, and `docs --check` fails while the
file differs.
Run as `python3 -I -B tools/impact.py`. Python 3.11 or later, git, and the
standard library only. Costs a few git calls and a read of the registry;
no build.

Exit status: 0 nothing unmet, 1 a rule unmet, 2 the registry does not read.
"""
import os
import re
import subprocess
import sys
import tempfile
import tomllib
from functools import lru_cache
from pathlib import Path
from typing import NamedTuple

ROOT = Path(__file__).resolve().parents[1]
REGISTRY = "tools/impact.toml"
AREAS = [
    "repo", "tree", "docs", "decisions", "standards", "crypto", "keys",
    "compute", "speed", "structure", "strings", "design", "tokens", "themes",
    "guide", "learn", "deps", "builds", "fuzz", "release", "impact",
]
KINDS = ("held", "pair", "forbid", "trailer", "review")
STAGES = ("just lint", "just test", "just crosscheck", "release", "website",
          "repository hook")
ONS = ("add", "modify", "delete")
STATUS_ON = {"A": "add", "M": "modify", "D": "delete"}
VERB = {"A": "added", "M": "changed", "D": "deleted"}

# Field name -> type: "s" string, "l" list of strings, "b" bool.
FIELDS = {
    "check": {"id": "s", "run": "s", "stage": "s", "catches": "s",
              "misses": "s", "status": "s"},
    "consumer": {"id": "s", "name": "s", "reads": "l", "act": "s",
                 "site_check": "s", "site_with": "l", "blocking": "b"},
    "rule": {"id": "s", "title": "s", "source": "l", "at": "s", "when": "l",
             "on": "l", "lines": "s", "rises": "b", "kind": "s",
             "held_by": "l", "with": "l", "with_each": "l", "trailer": "s",
             "misses": "s", "consumers": "l", "status": "s"},
    "amend": {"id": "s", "held_by": "l", "misses": "s", "with": "l",
              "consumers": "l"},
}
DEFAULTS = {
    "check": {"run": "", "stage": "just lint", "catches": "", "misses": "",
              "status": "built"},
    "consumer": {"name": "", "reads": [], "act": "", "site_check": "",
                 "site_with": [], "blocking": False},
    "rule": {"title": "", "source": [], "at": "change", "when": [],
             "on": list(ONS), "lines": "", "rises": False, "kind": "review",
             "held_by": [], "with": [], "with_each": [], "trailer": "",
             "misses": "", "consumers": [], "status": "active"},
}


# ---------------------------------------------------------------- matcher


@lru_cache(maxsize=None)
def compile_pattern(pattern):
    """A path pattern (docs/IMPACT.md §1.5) as a regex. `*` is within one
    segment, a whole `**` segment any number of segments, `{name}` one
    segment captured (a name repeated in one pattern must match the same
    text)."""
    segs = pattern.split("/")
    out, names = [], []
    for i, seg in enumerate(segs):
        last = i == len(segs) - 1
        if seg == "**":
            out.append(r".+" if last else r"(?:[^/]+/)*")
            continue
        r = ""
        for tok in re.split(r"(\*|\{[A-Za-z_][A-Za-z0-9_]*\})", seg):
            if tok == "*":
                r += "[^/]*"
            elif len(tok) > 2 and tok[0] == "{" and tok[-1] == "}":
                name = tok[1:-1]
                if name in names:
                    r += f"(?P={name})"
                else:
                    names.append(name)
                    r += f"(?P<{name}>[^/]+)"
            else:
                r += re.escape(tok)
        out.append(r + ("" if last else "/"))
    return re.compile("^" + "".join(out) + "$")


def match_list(patterns, path):
    """The captures (a dict, maybe empty) when `path` matches the list:
    some pattern without `!` matches and no `!` pattern does; else None."""
    for p in patterns:
        if p.startswith("!") and compile_pattern(p[1:]).match(path):
            return None
    for p in patterns:
        if not p.startswith("!"):
            m = compile_pattern(p).match(path)
            if m:
                return m.groupdict()
    return None


def fill(pattern, caps):
    """The pattern with each `{name}` the captures hold written in."""
    return re.sub(r"\{([A-Za-z_][A-Za-z0-9_]*)\}",
                  lambda m: caps.get(m.group(1), m.group(0)), pattern)


def has_wild(pattern):
    return any(c in pattern for c in "*{!")


# ---------------------------------------------------------------- registry


class Registry:
    def __init__(self):
        self.checks, self.consumers, self.rules = [], [], []
        self.errors = []
        self.extra_files = []

    def by_id(self):
        return {t["id"]: t for t in self.checks + self.consumers + self.rules}

    def add_file(self, path, extra):
        label = os.path.basename(str(path))
        try:
            with open(path, "rb") as f:
                data = tomllib.load(f)
        except (OSError, tomllib.TOMLDecodeError) as e:
            self.errors.append(f"{label}: cannot read: {e}")
            return
        if extra:
            self.extra_files.append(label)
        amends = []
        for table, kind in (("check", "check"), ("consumer", "consumer"),
                            ("rule", "rule"), ("amend", "amend")):
            for raw in data.get(table, []):
                item = self.typed(kind, raw, label)
                if item is None:
                    continue
                item["_file"], item["_extra"] = label, extra
                if kind == "amend":
                    amends.append(item)
                else:
                    {"check": self.checks, "consumer": self.consumers,
                     "rule": self.rules}[kind].append(item)
        for key in data:
            if key not in FIELDS:
                self.errors.append(f"{label}: unknown table [[{key}]]")
        for a in amends:
            self.amend(a)

    def typed(self, kind, raw, label):
        ident = raw.get("id", "?")
        ok = True
        for k, v in raw.items():
            want = FIELDS[kind].get(k)
            if want is None:
                self.errors.append(f"{label}: {kind} {ident}: unknown field {k}")
                ok = False
            elif want == "s" and not isinstance(v, str):
                self.errors.append(f"{label}: {kind} {ident}: {k} is not a string")
                ok = False
            elif want == "b" and not isinstance(v, bool):
                self.errors.append(f"{label}: {kind} {ident}: {k} is not a boolean")
                ok = False
            elif want == "l" and not (isinstance(v, list) and all(isinstance(x, str) for x in v)):
                self.errors.append(f"{label}: {kind} {ident}: {k} is not a list of strings")
                ok = False
        if "id" not in raw:
            self.errors.append(f"{label}: a {kind} has no id")
            ok = False
        if not ok:
            return None
        item = {k: (list(v) if isinstance(v, list) else v)
                for k, v in DEFAULTS.get(kind, {}).items()}
        item.update(raw)
        return item

    def amend(self, a):
        target = next((r for r in self.rules if r["id"] == a["id"]), None)
        if target is None:
            self.errors.append(f"{a['_file']}: amend {a['id']}: no such rule")
            return
        for k in ("held_by", "with", "consumers"):
            for v in a.get(k, []):
                if v not in target[k]:
                    target[k].append(v)
        if a.get("misses"):
            target["misses"] = (target["misses"] + "; " if target["misses"] else "") + a["misses"]


def load_registry(root=ROOT, extra_env=None):
    reg = Registry()
    reg.add_file(Path(root) / REGISTRY, False)
    env = os.environ.get("FARADAY_IMPACT_EXTRA", "") if extra_env is None else extra_env
    for p in [x for x in env.split(":") if x]:
        reg.add_file(p, True)
    return reg


# ---------------------------------------------------------------- lint


def recipe_commands(justfile_text, name):
    """The command lines of a justfile recipe (comments and blanks left out)."""
    out, inside = [], False
    for line in justfile_text.splitlines():
        if not inside:
            inside = re.match(rf"^{re.escape(name)}\s*:", line) is not None
            continue
        if line and not line[0].isspace():
            break
        s = line.strip()
        if s and not s.startswith("#"):
            out.append(s)
    return out


def heading_exists(text, sec):
    pat = re.compile(r"^(?:§\s*)?" + re.escape(sec) + r"(?![0-9])(?:[.\s:)]|$)")
    for line in text.splitlines():
        if line.startswith("#") and pat.match(line.lstrip("#").strip()):
            return True
    return False


def lint(reg, root, tracked, justfile_text):
    """The registry against itself and the tree (docs/IMPACT.md §2.5);
    the list of problems."""
    root = Path(root)
    errs = list(reg.errors)
    seen = {}
    for t in reg.checks + reg.consumers + reg.rules:
        i = t["id"]
        if not re.match(r"^[a-z0-9][a-z0-9.-]*$", i):
            errs.append(f"{i}: not a lower-case id")
        if i in seen:
            errs.append(f"{i}: id used twice ({seen[i]}, {t['_file']})")
        seen[i] = t["_file"]
    check_ids = {c["id"]: c for c in reg.checks}
    consumer_ids = {c["id"] for c in reg.consumers}
    for c in reg.checks:
        if c["stage"] not in STAGES:
            errs.append(f"{c['id']}: stage {c['stage']!r} is none of {', '.join(STAGES)}")
        if c["status"] not in ("built", "planned"):
            errs.append(f"{c['id']}: status {c['status']!r} is not built or planned")
        if not c["run"]:
            errs.append(f"{c['id']}: no run")
    for c in reg.consumers:
        if not re.match(r"^(app|website)\.", c["id"]):
            errs.append(f"{c['id']}: a consumer's id starts app. or website.")
    last_area = -1
    for r in reg.rules:
        i = r["id"]
        if r["kind"] not in KINDS:
            errs.append(f"{i}: kind {r['kind']!r} is none of {', '.join(KINDS)}")
        if r["status"] not in ("active", "planned"):
            errs.append(f"{i}: status {r['status']!r} is not active or planned")
        for o in r["on"]:
            if o not in ONS:
                errs.append(f"{i}: on {o!r} is none of {', '.join(ONS)}")
        if not r["_extra"]:
            area = i.split(".")[0]
            if area not in AREAS:
                errs.append(f"{i}: area {area!r} is not one of the registry's areas")
            else:
                n = AREAS.index(area)
                if n < last_area:
                    errs.append(f"{i}: out of the areas' order ({area})")
                last_area = max(last_area, n)
        for h in r["held_by"]:
            if h not in check_ids:
                errs.append(f"{i}: held_by {h} is no check")
        for c in r["consumers"]:
            if c not in consumer_ids:
                errs.append(f"{i}: consumers {c} is no consumer")
        if r["kind"] == "held" and r["status"] == "active" and not any(
                check_ids.get(h, {}).get("status") == "built" for h in r["held_by"]):
            errs.append(f"{i}: a held rule names no built check")
        if r["kind"] == "pair" and not (r["with"] or r["with_each"]):
            errs.append(f"{i}: a pair names nothing in with or with_each")
        if r["kind"] == "trailer" and not r["trailer"]:
            errs.append(f"{i}: a trailer rule names no trailer")
        if r["lines"]:
            try:
                re.compile(r["lines"])
            except re.error as e:
                errs.append(f"{i}: lines is not a regex: {e}")
        if r["_extra"]:
            continue  # an extra file may name what git does not track
        for s in r["source"]:
            path, _, sec = s.partition(" §")
            if not (root / path).is_file():
                errs.append(f"{i}: source {path} does not exist")
            elif sec:
                tok = re.split(r"[\s,;]", sec.strip())[0].rstrip(".,;:)")
                text = (root / path).read_text(encoding="utf-8", errors="replace")
                if tok and not heading_exists(text, tok):
                    errs.append(f"{i}: source {path} has no section {tok}")
        for field in ("when", "with", "with_each"):
            for p in r[field]:
                if p.startswith("!"):
                    continue
                if not has_wild(p):
                    if p not in tracked and not (root / p).exists():
                        errs.append(f"{i}: {field} {p} does not exist")
                elif field == "when" and r["on"] != ["add"]:
                    negs = [x for x in r["when"] if x.startswith("!")]
                    if not any(match_list([p] + negs, t) is not None for t in tracked):
                        errs.append(f"{i}: when {p} matches no tracked path")
    # The lint recipe and the checks of stage "just lint" are one list.
    recipe = recipe_commands(justfile_text, "lint")
    built = [c["run"] for c in reg.checks if c["stage"] == "just lint" and c["status"] == "built"]
    for cmd in recipe:
        if cmd not in built:
            errs.append(f"justfile lint: `{cmd}` is no [[check]] of stage just lint")
    for cmd in built:
        if cmd not in recipe:
            errs.append(f"check `{cmd}` is not a command of the justfile's lint recipe")
    return errs


# ---------------------------------------------------------------- git


class Change(NamedTuple):
    status: str  # A, M or D
    path: str
    lines: tuple  # changed lines, each with its leading + or -


class Git:
    def __init__(self, root):
        self.root = str(root)

    def run(self, *args, check=True):
        p = subprocess.run(["git", "-c", "core.quotepath=off", *args], cwd=self.root,
                           capture_output=True)
        if check and p.returncode != 0:
            raise RuntimeError(p.stderr.decode(errors="replace").strip() or "git failed")
        return p.stdout.decode(errors="replace")

    def ok(self, *args):
        try:
            return self.run(*args).strip()
        except (RuntimeError, OSError):
            return None

    def tracked(self):
        """Tracked and not-ignored untracked paths, or None outside a checkout."""
        out = self.ok("ls-files", "-z", "-c", "-o", "--exclude-standard")
        return None if out is None else {p for p in out.split("\0") if p}


def parse_name_status(text):
    """`git diff --name-status -M -z` -> [(status, path)]; a rename is its
    old path deleted and its new path added."""
    parts = text.split("\0")
    out, i = [], 0
    while i < len(parts) and parts[i]:
        st = parts[i][0]
        if st in "RC":
            old, new = parts[i + 1], parts[i + 2]
            i += 3
            if st == "R":
                out.append(("D", old))
            out.append(("A", new))
        else:
            out.append((st if st in "AD" else "M", parts[i + 1]))
            i += 2
    return out


def parse_diff(text):
    """`git diff -U0` -> {path: [changed line with its + or -]}."""
    files, cur, in_hunk, old = {}, None, False, None
    for line in text.splitlines():
        if line.startswith("diff --git "):
            cur, in_hunk, old = None, False, None
        elif in_hunk and line[:1] in "+-":
            files.setdefault(cur, []).append(line)
        elif line.startswith("@@"):
            in_hunk = cur is not None
        elif not in_hunk and line.startswith("--- "):
            old = line[4:].split("\t")[0]
        elif not in_hunk and line.startswith("+++ "):
            new = line[4:].split("\t")[0]
            if new != "/dev/null":
                cur = new[2:]
            elif old and old != "/dev/null":
                cur = old[2:]
            if cur is not None:
                files.setdefault(cur, [])
    return files


def file_lines(root, path):
    try:
        data = (Path(root) / path).read_bytes()
    except OSError:
        return ()
    if b"\0" in data[:8000]:
        return ()
    return tuple("+" + l for l in data.decode(errors="replace").splitlines())


def build_changes(statuses, diff_lines):
    return [Change(st, p, tuple(diff_lines.get(p, ()))) for st, p in statuses]


def trailers(message):
    """The trailers of a commit message: [(key, value)] from its last
    paragraph, when the message has a paragraph before it."""
    paras = re.split(r"\n[ \t]*\n", message.strip("\n"))
    if len(paras) < 2:
        return []
    out = []
    for line in paras[-1].splitlines():
        m = re.match(r"^([A-Za-z][A-Za-z0-9-]*):[ \t]*(.*)$", line)
        if m:
            out.append((m.group(1), m.group(2).strip()))
        elif line[:1] in " \t" and out:
            out[-1] = (out[-1][0], (out[-1][1] + " " + line.strip()).strip())
    return out


# ---------------------------------------------------------------- rules


def rises(lines):
    """Whether a counts file's numbers rose: some line's last integer is
    higher than the same line's (without it) before, or the line is new."""
    def table(prefix):
        t = {}
        for l in lines:
            if l.startswith(prefix):
                nums = re.findall(r"\d+", l)
                if nums:
                    key = l[1:].rsplit(nums[-1], 1)[0].strip()
                    t[key] = int(nums[-1])
        return t
    old, new = table("-"), table("+")
    return any(v > old.get(k, 0) for k, v in new.items())


def triggers(rule, changes):
    """[(change, captures)] for the changes that trigger the rule."""
    out = []
    for c in changes:
        if STATUS_ON[c.status] not in rule["on"]:
            continue
        caps = match_list(rule["when"], c.path) if rule["when"] else {}
        if caps is None:
            continue
        if rule["lines"] and not any(re.search(rule["lines"], l) for l in c.lines):
            continue
        if rule["rises"] and not rises(c.lines):
            continue
        out.append((c, caps))
    return out


def partners(rule, caps):
    return ([fill(p, caps) for p in rule["with"]], [fill(p, caps) for p in rule["with_each"]])


def pair_unmet(rule, changes, trig):
    """The unmet messages of a pair rule: one for each trigger left without
    a partner among the other changed paths."""
    out = []
    for c, caps in trig:
        others = [x.path for x in changes if x.path != c.path]
        anyp, each = partners(rule, caps)
        missing = []
        if anyp and not any(match_list(anyp, o) is not None for o in others):
            missing.append(" or ".join(p for p in anyp if not p.startswith("!")))
        for p in each:
            if not p.startswith("!") and not any(compile_pattern(p).match(o) for o in others):
                missing.append(p)
        if missing:
            msg = f"{c.path} {VERB[c.status]}; change {' and '.join(missing)}"
            if msg not in out:
                out.append(msg)
    return out


def waivers(commits):
    """[(rule, reason, label)] from the commits' Impact-skip trailers."""
    out = []
    for label, message in commits:
        for key, value in trailers(message):
            if key.lower() == "impact-skip":
                rule, _, reason = value.partition(":")
                if rule.strip() and reason.strip():
                    out.append((rule.strip(), reason.strip(), label))
    return out


def trailer_values(commits, name):
    return [(v, label) for label, message in commits for k, v in trailers(message)
            if k.lower() == name.lower() and v]


def reviewed(commits):
    """{rule: label} from the commits' Reviewed trailers, each of which
    names one or more rules, comma-separated."""
    out = {}
    for label, message in commits:
        for key, value in trailers(message):
            if key.lower() == "reviewed":
                for name in value.split(","):
                    if name.strip():
                        out.setdefault(name.strip(), label)
    return out


def evaluate(reg, changes, commits):
    """The check: ([(state, rule, text)], counts) with state one of unmet,
    waived, met, review, held, planned."""
    results = []
    wv = waivers(commits)
    notes = []
    known = {r["id"] for r in reg.rules}
    rv = reviewed(commits)
    attestable = set()
    for rule, _, _ in wv:
        if rule not in known:
            notes.append(f"waiver of {rule}: no such rule")
    for r in reg.rules:
        if r["at"] != "change":
            continue
        trig = triggers(r, changes)
        if not trig:
            continue
        mine = [w for w in wv if w[0] == r["id"]]
        if r["status"] == "planned":
            results.append(("planned", r, r["title"]))
        elif r["kind"] == "held":
            results.append(("held", r, ""))
        elif r["kind"] == "review" and (r["_extra"] or standing(r)):
            results.append(("review", r, r["title"]))
        elif r["kind"] == "review":
            attestable.add(r["id"])
            if r["id"] in rv:
                results.append(("met", r, f'"Reviewed: {r["id"]}" ({rv[r["id"]]})'))
            elif mine:
                results.append(("waived", r, f'"{mine[0][1]}" ({mine[0][2]})'))
            else:
                paths = ", ".join(dict.fromkeys(c.path for c, _ in trig))
                results.append(("unmet", r, f'{paths}; add "Reviewed: {r["id"]}" once checked'))
        else:
            if r["kind"] == "pair":
                unmet = pair_unmet(r, changes, trig)
                why = "; ".join(unmet)
            elif r["kind"] == "forbid":
                # `with` names a move that allows the change: a path in it
                # (captures filled) changing in the same range.
                unmet = []
                for c, caps in trig:
                    anyp = [fill(p, caps) for p in r["with"]]
                    if anyp and any(match_list(anyp, x.path) is not None
                                    for x in changes if x.path != c.path):
                        continue
                    unmet.append(f"{c.path} {VERB[c.status]}, which is refused")
                why = "; ".join(unmet)
            else:  # trailer
                have = trailer_values(commits, r["trailer"])
                unmet = [] if have else [1]
                why = f"add a `{r['trailer']}: <value>` trailer to the commit"
                if have:
                    results.append(("met", r, f'"{r["trailer"]}: {have[0][0]}" ({have[0][1]})'))
                    continue
            if not unmet:
                results.append(("met", r, "paired in the range"))
            elif mine:
                results.append(("waived", r, f'"{mine[0][1]}" ({mine[0][2]})'))
            else:
                results.append(("unmet", r, why))
    for rule in rv:
        if rule not in known:
            notes.append(f"Reviewed: {rule}: no such rule")
        elif rule not in attestable:
            notes.append(f"Reviewed: {rule}: not triggered, or not a review rule of the registry")
    return results, notes


def render(results, quiet):
    order = ["unmet", "waived", "met", "review", "planned"]
    lines = []
    for state in order:
        if quiet and state != "unmet":
            continue
        for st, r, text in results:
            if st != state:
                continue
            if r["_extra"]:
                text = f"(a rule of {r['_file']})"
            if st == "unmet" and r["kind"] == "review":
                lines.append(f"impact: unreviewed {r['id']}: {text}")
                continue
            lines.append(f"impact: {st} {r['id']}: {text}")
    counts = {}
    for st, _, _ in results:
        counts[st] = counts.get(st, 0) + 1
    parts = [f"{counts[s]} {s}" for s in ("unmet", "waived", "held", "met", "review", "planned")
             if counts.get(s)]
    lines.append(f"impact: {len(results)} rules triggered" + (": " + ", ".join(parts) if parts else ""))
    return lines, counts.get("unmet", 0)


# ---------------------------------------------------------------- plan


def expand_paths(paths, tracked):
    out = []
    for p in paths:
        p = p.rstrip("/")
        if has_wild(p):
            found = sorted(t for t in tracked if match_list([p], t) is not None)
        else:
            found = sorted(t for t in tracked if t == p or t.startswith(p + "/"))
        for f in found or [p]:
            if f not in out:
                out.append(f)
    return out


def standing(r):
    """A review rule whose only trigger is every file (`**`, exclusions aside):
    it applies to every change, so it is listed, never attested per change
    (F151). The push hook or a check of its own holds such a rule."""
    pos = [p for p in r["when"] if not p.startswith("!")]
    return bool(pos) and all(p == "**" for p in pos)


def plan(reg, paths, at, long):
    rules = []
    for r in reg.rules:
        if r["at"] == "change":
            hit = [p for p in paths if (match_list(r["when"], p) is not None if r["when"] else True)]
        elif r["at"] == at:
            hit = list(paths)
        else:
            continue
        if hit:
            rules.append((r, hit))
    n = len(paths)
    out = [f"impact: {n} path{'s' if n != 1 else ''}, {len(rules)} rule{'s' if len(rules) != 1 else ''}"]
    change, trail, forbid, review, attest, held = [], [], [], [], [], []
    for r, hit in rules:
        for h in r["held_by"]:
            if h not in held:
                held.append(h)
        if r["kind"] == "pair":
            ps = []
            for p in hit:
                caps = (match_list(r["when"], p) or {}) if r["when"] else {}
                anyp, each = partners(r, caps)
                text = " or ".join(x for x in anyp if not x.startswith("!"))
                text = " and ".join(x for x in ([text] if text else []) + each)
                if text not in ps:
                    ps.append(text)
            change.append((r["id"], "; ".join(ps)))
        elif r["kind"] == "review" and (r["with"] or r["with_each"]):
            change.append((r["id"], ("(needs Reviewed) " if not r["_extra"] else "(review) ") + r["title"]))
        elif r["kind"] == "review":
            (review if r["_extra"] or standing(r) else attest).append(r["id"])
        elif r["kind"] == "trailer":
            trail.append(f"{r['id']} ({r['trailer']}: <value>)")
        elif r["kind"] == "forbid":
            forbid.append(f"{r['id']} {r['title']}".rstrip())
    if change:
        out.append("change with it:")
        w = max(len(i) for i, _ in change)
        out += [f"  {i:<{w}}  {t}" for i, t in change]
    if held:
        out.append("held by: " + ", ".join(held))
    out += [f"trailer: {t}" for t in trail]
    out += [f"forbidden: {t}" for t in forbid]
    if attest:
        out.append("needs a Reviewed: <rule> line in the commit: " + ", ".join(attest))
    if review:
        out.append("review: " + ", ".join(review))
    reads = []
    for c in reg.consumers:
        if c["id"] in [x for r, _ in rules for x in r["consumers"]] or any(
                match_list(c["reads"], p) is not None for p in paths if c["reads"]):
            reads.append(c["id"])
    if reads:
        out.append("read by: " + ", ".join(reads))
    if long:
        out.append("rules:")
        for r, _ in rules:
            out.append(f"  {r['id']}: {r['title']}")
            if r["source"]:
                out.append("    source: " + ", ".join(r["source"]))
            if r["misses"]:
                out.append("    misses: " + r["misses"])
    return out


# ---------------------------------------------------------------- commands


def load_or_die(root=ROOT):
    reg = load_registry(root)
    if reg.errors:
        for e in reg.errors:
            print(f"impact: lint: {e}")
        sys.exit(2)
    return reg


def tracked_set(git, root):
    t = git.tracked()
    if t is not None:
        return t
    found = set()
    for d, dirs, files in os.walk(root):
        dirs[:] = [x for x in dirs if x not in (".git", "target", "out")]
        for f in files:
            found.add(os.path.relpath(os.path.join(d, f), root).replace(os.sep, "/"))
    return found


# The packages each app consumer is built from (docs/IMPACT.md §9.2).
APP_ROOTS = {
    "app.faraday": ("faraday-desktop", "faraday-stick"),
    "app.starter": ("faraday-starter-shell",),
}


def app_reads_lint(reg, root, metadata=None):
    """Every workspace crate an app is built from is read by that app's
    consumer (docs/IMPACT.md §9.2): the problems, from `cargo metadata
    --offline`. Costs a cargo metadata run. When cargo cannot run, prints
    a note and finds none."""
    import json
    if metadata is None:
        try:
            p = subprocess.run(["cargo", "metadata", "--offline", "--format-version", "1"],
                               cwd=str(root), capture_output=True, timeout=120)
            if p.returncode != 0:
                raise OSError(p.stderr.decode(errors="replace").strip()[-200:] or "failed")
            metadata = json.loads(p.stdout)
        except (OSError, ValueError, subprocess.SubprocessError) as e:
            print(f"impact: lint: cargo metadata not available ({e}); "
                  "app consumers not compared with the workspace")
            return []
    base = str(metadata["workspace_root"]) + "/"
    pkgs = {p["id"]: p for p in metadata["packages"]}
    members = set(metadata["workspace_members"])
    deps = {n["id"]: [d["pkg"] for d in n["deps"]
                      if any(k["kind"] in (None, "build") for k in d["dep_kinds"])]
            for n in metadata["resolve"]["nodes"]}
    errs = []
    for cid, roots in APP_ROOTS.items():
        cons = next((c for c in reg.consumers if c["id"] == cid), None)
        if cons is None:
            errs.append(f"{cid}: no such consumer")
            continue
        seen, todo = set(), [i for i, p in pkgs.items() if p["name"] in roots]
        while todo:
            i = todo.pop()
            if i not in seen:
                seen.add(i)
                todo.extend(deps.get(i, ()))
        for i in sorted(seen & members, key=lambda i: pkgs[i]["name"]):
            manifest = pkgs[i]["manifest_path"].replace(base, "")
            if match_list(cons["reads"], manifest) is None:
                errs.append(f"{cid}: workspace crate {pkgs[i]['name']} ({manifest}) "
                            "is used by the app and read by no pattern of reads")
    return errs


def run_lint(reg, root=ROOT):
    git = Git(root)
    jf = Path(root) / "justfile"
    text = jf.read_text(encoding="utf-8") if jf.is_file() else ""
    return lint(reg, root, tracked_set(git, root), text) + app_reads_lint(reg, root)


def cmd_lint(args):
    reg = load_registry()
    errs = run_lint(reg)
    for e in errs:
        print(f"impact: lint: {e}")
    if errs:
        print(f"impact: lint: {len(errs)} problem(s)")
        return 2
    print(f"impact: lint: {len(reg.checks)} checks, {len(reg.consumers)} consumers, "
          f"{len(reg.rules)} rules, ok")
    return 0


def working_changes(git, base):
    """The changes of the working tree against `base` (a commit, or None for
    the first commit's empty tree)."""
    raw = git.run("diff", "--name-status", "-M", "-z", base)
    diff = parse_diff(git.run("diff", "-U0", "-M", "--no-color", "--no-ext-diff", base))
    changes = build_changes(parse_name_status(raw), diff)
    for p in git.run("ls-files", "-z", "-o", "--exclude-standard").split("\0"):
        if p:
            changes.append(Change("A", p, file_lines(git.root, p)))
    return changes


def range_changes(git, a, b):
    raw = git.run("diff", "--name-status", "-M", "-z", a, b)
    diff = parse_diff(git.run("diff", "-U0", "-M", "--no-color", "--no-ext-diff", a, b))
    return build_changes(parse_name_status(raw), diff)


def log_commits(git, a, b):
    out = git.run("log", "--format=%H%x1f%B%x1e", f"{a}..{b}")
    commits = []
    for rec in out.split("\x1e"):
        rec = rec.strip("\n")
        if "\x1f" in rec:
            h, msg = rec.split("\x1f", 1)
            commits.append((h[:8], msg))
    return commits


def paths_changes(git, paths):
    changes = []
    tracked = git.tracked() or set()
    for p in paths:
        full = Path(git.root) / p
        if not full.exists():
            changes.append(Change("D", p, ()))
        elif p in tracked and git.ok("ls-files", "--error-unmatch", p) is not None:
            d = parse_diff(git.run("diff", "-U0", "--no-color", "--no-ext-diff", "HEAD", "--", p))
            changes.append(Change("M", p, tuple(d.get(p, ()))))
        else:
            changes.append(Change("A", p, file_lines(git.root, p)))
    return changes


def collect(args, git):
    """(code, changes, commits): code is None to go on, or the exit code
    to return after the message `collect` printed."""
    commits = []
    if args.paths:
        return None, paths_changes(git, args.paths), commits
    head = git.ok("rev-parse", "--verify", "HEAD")
    if head is None:
        print("impact: not a git checkout with commits; nothing to compare, not checked")
        return 0, [], []
    with_tree = False
    if args.range:
        if "..." in args.range:
            x, y = args.range.split("...", 1)
            y = y or "HEAD"
            a = git.ok("merge-base", x or "HEAD", y)
        elif ".." in args.range:
            x, y = args.range.split("..", 1)
            a, y = x or "HEAD", y or "HEAD"
        else:
            a, y = args.range, "HEAD"
        if a is None or git.ok("rev-parse", "--verify", y) is None:
            print(f"impact: cannot read the range {args.range}")
            return 2, [], []
        b = y
        with_tree = git.ok("rev-parse", y) == head and "..." not in args.range and ".." not in args.range
    else:
        a = git.ok("merge-base", "main", "HEAD")
        if a is None:
            print("impact: no main to compare with; not checked (whoever lands it checks the range)")
            return 0, [], []
        b, with_tree = "HEAD", True
    commits = log_commits(git, a, b)
    changes = range_changes(git, a, b)
    if with_tree:
        tree = working_changes(git, "HEAD")
        merged = {c.path: c for c in changes}
        for c in tree:
            if c.path in merged:
                old = merged[c.path]
                merged[c.path] = Change(old.status if old.status == "A" else c.status,
                                        c.path, old.lines + c.lines)
            else:
                merged[c.path] = c
        changes = list(merged.values())
    return None, changes, commits


def cmd_check(args):
    reg = load_or_die()
    errs = run_lint(reg)
    if errs:
        for e in errs:
            print(f"impact: lint: {e}")
        return 2
    if (args.site or args.site_since) and not args.consumer:
        print("impact: --site and --site-since go with --consumer")
        return 2
    sel = None
    if args.consumer:
        sel = select_consumers(reg, args.consumer)
        if not sel:
            print(f"impact: no consumer {args.consumer}")
            return 2
        if args.site and not Path(args.site).is_dir():
            print(f"impact: --site {args.site} is not a directory")
            return 2
    git = Git(args.repo or ROOT)
    code, changes, commits = collect(args, git)
    if code is not None:
        return code
    # FARADAY_IMPACT_MESSAGE names the same file as --message, for a check
    # run inside another recipe (`just`) on a tree whose commit is not yet
    # made: a release's bump, checked before it is committed.
    message = args.message or os.environ.get("FARADAY_IMPACT_MESSAGE")
    if message:
        path = Path(message)
        if not path.is_absolute():
            path = Path(args.repo or ROOT) / path
        try:
            msg = path.read_text(encoding="utf-8")
        except OSError as e:
            print(f"impact: cannot read {message}: {e}")
            return 2
        commits.append(("message", msg))
    results, notes = evaluate(reg, changes, commits)
    if sel is not None:
        site = None
        if args.site:
            site = (args.site, site_commits(args.site, args.site_since))
        lines, pending = render_consumers(sel, changes, commits, results, site, args.quiet)
        print("\n".join(lines))
        return 1 if pending else 0
    lines, unmet = render(results, args.quiet)
    if not args.quiet:
        lines = [f"impact: {n}" for n in notes] + lines
    print("\n".join(lines))
    return 1 if unmet else 0


# ---------------------------------------------------------------- consumers


def select_consumers(reg, name):
    """The consumers named `name`: the id itself, or every id under it
    (`website` is `website.guide`, `website.tour`, ...)."""
    return [c for c in reg.consumers if c["id"] == name or c["id"].startswith(name + ".")]


def site_commits(tree, since):
    """[(label, message, paths)] of the website's commits after `since`, or
    [] where there is no commit to compare from. Reads the website's git."""
    if not since:
        return []
    out = Git(tree).ok("log", "--name-only", "--format=%x1e%H%x1f%B%x1f", f"{since}..HEAD")
    commits = []
    for rec in (out or "").split("\x1e"):
        if rec.count("\x1f") < 2:
            continue
        h, msg, names = rec.split("\x1f", 2)
        commits.append((h[:8], msg, [n for n in names.split("\n") if n.strip()]))
    return commits


def run_site_check(cmd, tree):
    """Runs the consumer's `site_check` in the website's tree: True when it
    exits 0. Costs what the command costs."""
    try:
        return subprocess.run(cmd, shell=True, cwd=tree, capture_output=True,
                              timeout=600).returncode == 0
    except (OSError, subprocess.TimeoutExpired):
        return False


def consumer_trigger(c, changes, results):
    """Why the consumer is affected: the first changed path it reads, or the
    triggered rule that names it. None when it is not affected."""
    for ch in changes:
        if c["reads"] and match_list(c["reads"], ch.path) is not None:
            return f"{ch.path} {VERB[ch.status]}"
    for _, r, _ in results:
        if c["id"] in r["consumers"]:
            return f"rule {r['id']} triggered"
    return None


def consumer_states(sel, changes, commits, results, site):
    """[(state, consumer, text)]: pending (blocking, not yet done), waived,
    met, or listed (never blocks). Without `site` a blocking consumer is
    pending: nothing here can tell it is done."""
    wv = waivers(commits)
    out = []
    for c in sel:
        why = consumer_trigger(c, changes, results)
        if why is None:
            continue
        text = f"{why}; {c['act']}" if c["act"] else why
        if c["_extra"]:
            text = f"(a consumer of {c['_file']})"
        if not c["blocking"]:
            out.append(("listed", c, text))
            continue
        if site is not None:
            tree, scommits = site
            if c["site_check"] and run_site_check(c["site_check"], tree):
                out.append(("met", c, "site_check passed"))
                continue
            done = [h for h, _, names in scommits if c["site_with"]
                    and any(match_list(c["site_with"], n) is not None for n in names)]
            if done:
                out.append(("met", c, f"site changed ({done[0]})"))
                continue
            skip = [w for w in wv + waivers([(h, m) for h, m, _ in scommits]) if w[0] == c["id"]]
            if skip:
                out.append(("waived", c, f'"{skip[0][1]}" ({skip[0][2]})'))
                continue
        out.append(("pending", c, text))
    return out


def render_consumers(sel, changes, commits, results, site, quiet):
    """(lines, pending count) of `check --consumer`: the consumers affected,
    then the triggered rules that name them."""
    ids = {c["id"] for c in sel}
    bearing = [t for t in results if ids & set(t[1]["consumers"])]
    states = consumer_states(sel, changes, commits, results, site)
    lines = []
    for want in ("pending", "waived", "met", "listed"):
        if quiet and want != "pending":
            continue
        for st, c, text in states:
            if st == want:
                tag = " (blocking)" if c["blocking"] else ""
                lines.append(f"impact: consumer {st} {c['id']}{tag}: {text}")
    lines += render(bearing, quiet)[0][:-1]
    counts = {}
    for st, _, _ in states:
        counts[st] = counts.get(st, 0) + 1
    parts = [f"{counts[k]} {k}" for k in ("pending", "waived", "met", "listed") if counts.get(k)]
    lines.append(f"impact: {len(states)} consumers affected" + (": " + ", ".join(parts) if parts else ""))
    return lines, counts.get("pending", 0)


def cmd_plan(args):
    reg = load_or_die()
    git = Git(ROOT)
    tracked = tracked_set(git, ROOT)
    if args.path:
        paths = expand_paths(args.path, tracked)
    else:
        base = git.ok("rev-parse", "--verify", "HEAD")
        paths = [c.path for c in working_changes(git, "HEAD")] if base else []
    print("\n".join(plan(reg, paths, args.at, args.long)))
    return 0


# ---------------------------------------------------------------- self-test


# ---------------------------------------------------------------- docs

CHECKS_MD = "docs/CHECKS.md"


def cell(text):
    """A table cell: one line, pipes escaped."""
    return " ".join(str(text).split()).replace("|", "\\|") or "-"


def code_list(items):
    return "<br>".join(f"`{x}`" for x in items) or "-"


def source_text(sources):
    out = []
    for s in sources:
        path, sep, sec = s.partition(" §")
        out.append(f"`{path}`" + (f" §{sec}" if sep else ""))
    return "<br>".join(out) or "-"


def trigger_text(r):
    parts = [code_list(r["when"])]
    if r["at"] != "change":
        parts.append(f"at {r['at']}")
    if r["on"] != list(ONS):
        parts.append("on " + ", ".join(r["on"]))
    if r["lines"]:
        parts.append("lines matching `" + r["lines"].replace("|", "\\|") + "`")
    if r["rises"]:
        parts.append("a number that rises")
    return "<br>".join(parts)


def change_with(r):
    if r["kind"] == "trailer":
        return f"a `{r['trailer']}:` trailer"
    items = []
    if r["with"]:
        items.append(("one of " if len(r["with"]) > 1 else "") + code_list(r["with"]))
    if r["with_each"]:
        items.append(("each of " if len(r["with_each"]) > 1 else "") + code_list(r["with_each"]))
    return "<br>".join(items) or "-"


def render_checks_md(reg, report=False):
    """docs/CHECKS.md as text, from the public registry (docs/IMPACT.md §7);
    with `report`, the map of §2.4 instead: the extra files' rules and
    consumers too, each marked with its file, and the rules of each moment."""
    mark = (lambda t: f" (of {t['_file']})" if t["_extra"] else "") if report else (lambda t: "")
    if report:
        head = [
            "# Impact map: every check, rule and consumer",
            "",
            "Written by `tools/impact.py report` from `tools/impact.toml`"
            + (" and " + ", ".join(f"`{f}`" for f in reg.extra_files) if reg.extra_files else "")
            + "; for reading, never committed.",
            "",
        ]
    else:
        head = [
            "# Checks: what holds each rule",
            "",
            "**Kind:** Reference",
            "**Status:** Built (F149); generated by `tools/impact.py docs` from `tools/impact.toml`, never edited by hand",
            "",
            "Every check the repository has, when it runs and what it catches and",
            "misses; every rule of the registry by area, with what holds it; and",
            "the parts of the suite that read the repository's paths. `just lint`",
            "fails while this file differs from the registry. The reasons are",
            "`docs/IMPACT.md`'s and `docs/decisions/F149.md`'s; which tool or lint",
            "does what is `tools/README.md`'s.",
            "",
        ]
    out = head + [
        "## 1. Checks",
        "",
        "| Id | Stage | Run | Catches | Misses | Status |",
        "|---|---|---|---|---|---|",
    ]
    for c in reg.checks:
        out.append(f"| `{c['id']}` | {c['stage']} | `{cell(c['run'])}` | {cell(c['catches'])} "
                   f"| {cell(c['misses'])} | {c['status']} |")
    out += ["", "## 2. Rules by area", ""]
    n = 0
    areas = AREAS + sorted({r["id"].split(".")[0] for r in reg.rules} - set(AREAS))
    for area in areas:
        rules = [r for r in reg.rules if r["id"].split(".")[0] == area]
        if not rules:
            continue
        n += 1
        out += [f"### 2.{n} {area}", "",
                "| Rule | Triggered by | Kind | Must change with it | Held by | Misses | Source |",
                "|---|---|---|---|---|---|---|"]
        for r in rules:
            name = f"`{r['id']}`" + (" (planned)" if r["status"] == "planned" else "") + mark(r)
            held = ", ".join(f"`{h}`" for h in r["held_by"]) or "-"
            out.append(f"| {name}: {cell(r['title'])} | {trigger_text(r)} | {r['kind']} "
                       f"| {change_with(r)} | {held} | {cell(r['misses'])} | {source_text(r['source'])} |")
        out.append("")
    out += ["## 3. Review only", "",
            "No script holds these; a reviewer reads this list for the change.", "",
            "| Rule | Source |", "|---|---|"]
    for r in reg.rules:
        if r["kind"] == "review":
            out.append(f"| `{r['id']}`{mark(r)}: {cell(r['title'])} | {source_text(r['source'])} |")
    if report:
        out += ["", "## 4. Read by", "",
                "| Part | Reads | What to do when it changes | Blocking |", "|---|---|---|---|"]
        for c in reg.consumers:
            out.append(f"| `{c['id']}`{mark(c)}: {cell(c['name'])} | {code_list(c['reads'])} "
                       f"| {cell(c['act'])} | {'yes' if c['blocking'] else 'no'} |")
        out += ["", "## 5. Rules of each moment", ""]
        for at in sorted({r["at"] for r in reg.rules}, key=lambda x: (x != "change", x)):
            ids = ", ".join(f"`{r['id']}`" for r in reg.rules if r["at"] == at)
            out.append(f"- **{at}:** {ids}")
        return "\n".join(out) + "\n"
    out += ["", "## 4. Read by", "",
            "| Part | Reads | What to do when it changes |", "|---|---|---|"]
    for c in reg.consumers:
        out.append(f"| `{c['id']}`: {cell(c['name'])} | {code_list(c['reads'])} | {cell(c['act'])} |")
    return "\n".join(out) + "\n"


def cmd_docs(args):
    reg = load_registry(ROOT, "")
    errs = run_lint(reg)
    if errs:
        for e in errs:
            print(f"impact: lint: {e}")
        return 2
    text = render_checks_md(reg)
    path = ROOT / CHECKS_MD
    if args.check:
        have = path.read_text(encoding="utf-8") if path.is_file() else None
        if have != text:
            print(f"impact: {CHECKS_MD} differs from the registry: run "
                  "`python3 -I -B tools/impact.py docs`")
            return 1
        return 0
    path.write_text(text, encoding="utf-8")
    print(f"impact: wrote {CHECKS_MD}")
    return 0


def cmd_report(args):
    reg = load_or_die()
    errs = run_lint(reg)
    if errs:
        for e in errs:
            print(f"impact: lint: {e}")
        return 2
    text = render_checks_md(reg, report=True)
    if args.out:
        Path(args.out).write_text(text, encoding="utf-8")
        print(f"impact: wrote {args.out}")
    else:
        sys.stdout.write(text)
    return 0


def mkrule(**kw):
    r = {k: (list(v) if isinstance(v, list) else v) for k, v in DEFAULTS["rule"].items()}
    r.update(kw)
    r.setdefault("id", "x.y")
    r["_file"], r["_extra"] = "impact.toml", kw.get("_extra", False)
    return r


def _standing_test(ok):
    r = {"when": ["**", "!LICENSE"]}
    ok(standing(r), "a rule on every file is standing")
    ok(not standing({"when": ["**/screen.rs"]}), "a narrower rule is not standing")
    ok(not standing({"when": []}), "a rule with no trigger is not standing")


def self_test():
    ok = lambda c, *m: c or sys.exit("impact self-test failed: " + " ".join(map(str, m)))
    _standing_test(ok)

    # Pattern forms.
    m = lambda pats, path: match_list(pats, path)
    ok(m(["docs/*.md"], "docs/A.md") == {}, "star in a segment")
    ok(m(["docs/*.md"], "docs/x/A.md") is None, "star stays in its segment")
    ok(m(["docs/**/*.md"], "docs/A.md") == {}, "double star matches no segment")
    ok(m(["docs/**/*.md"], "docs/x/y/A.md") == {}, "double star matches several")
    ok(m(["**/screen.rs"], "screen.rs") == {} and m(["**/screen.rs"], "a/b/screen.rs") == {}, "leading **")
    ok(m(["docs/**"], "docs/a/b") == {} and m(["docs/**"], "docs") is None, "trailing **")
    ok(m(["**"], "any/path.txt") == {}, "sole **")
    ok(m(["core/*/src/**"], "core/faraday-psbt/src/a/b.rs") == {}, "star then **")
    ok(m(["core/*/src/**"], "core/faraday-psbt/tests/a.rs") is None, "wrong middle")
    ok(m(["docs/decisions/F{n}.md"], "docs/decisions/F149.md") == {"n": "149"}, "capture")
    ok(m(["docs/{a}/{a}.md"], "docs/x/x.md") == {"a": "x"} and m(["docs/{a}/{a}.md"], "docs/x/y.md") is None, "repeated capture")
    ok(m(["**/*.rs", "!**/tests/**"], "a/src/b.rs") == {}, "exclusion keeps")
    ok(m(["**/*.rs", "!**/tests/**"], "a/tests/b.rs") is None, "exclusion drops")
    ok(m(["!docs/**"], "docs/a.md") is None, "only exclusions match nothing")
    ok(fill("docs/history/{n}-LOG.md", {"n": "SPEC"}) == "docs/history/SPEC-LOG.md", "fill")
    ok(fill("a/{z}", {}) == "a/{z}", "unfilled stays")
    ok(m(["a.b"], "aXb") is None, "dot is literal")

    # Triggers: on, lines, rises, captures.
    ch = lambda st, p, *ls: Change(st, p, tuple(ls))
    r = mkrule(when=["docs/*.md"], on=["add"], kind="pair", **{"with": ["docs/README.md"]})
    ok(len(triggers(r, [ch("A", "docs/N.md")])) == 1, "add triggers")
    ok(not triggers(r, [ch("M", "docs/N.md")]), "modify does not trigger an add rule")
    ok(not triggers(r, [ch("D", "docs/N.md")]), "delete does not trigger an add rule")
    r2 = mkrule(when=["justfile"], lines=r"^[+-]\s+cargo ")
    ok(triggers(r2, [ch("M", "justfile", "+    cargo test")]), "lines matches an added line")
    ok(triggers(r2, [ch("M", "justfile", "-    cargo test")]), "lines matches a removed line")
    ok(not triggers(r2, [ch("M", "justfile", "+# cargo", "+    echo")]), "lines mismatch")
    r3 = mkrule(when=["tools/*.counts"], rises=True)
    ok(triggers(r3, [ch("M", "tools/a.counts", "-file a.rs 2900", "+file a.rs 2950")]), "count rises")
    ok(not triggers(r3, [ch("M", "tools/a.counts", "-file a.rs 2950", "+file a.rs 2900")]), "count falls")
    ok(triggers(r3, [ch("M", "tools/a.counts", "+file new.rs 2600")]), "a new line is a rise")
    ok(not triggers(r3, [ch("M", "tools/a.counts", "-file gone.rs 2600")]), "a removed line is no rise")
    r4 = mkrule(when=["any/**"], on=["delete"])
    ok(triggers(r4, [ch("D", "any/x")]) and not triggers(r4, [ch("A", "any/x")]), "on delete")
    ok(len(triggers(mkrule(when=[]), [ch("M", "a"), ch("M", "b")])) == 2, "empty when is any change")

    # Pairs.
    base = [ch("A", "docs/IMPACT.md")]
    res, _ = evaluate(_reg(r), base, [])
    ok(res[0][0] == "unmet" and "docs/IMPACT.md added; change docs/README.md" in res[0][2], res)
    res, _ = evaluate(_reg(r), base + [ch("M", "docs/README.md")], [])
    ok(res[0][0] == "met", res)
    res, _ = evaluate(_reg(r), base, [("a1b2c3d4", "Subject\n\nBody.\n\nImpact-skip: x.y: not a document")])
    ok(res[0][0] == "waived" and "not a document" in res[0][2], res)
    res, notes = evaluate(_reg(r), base, [("a1b2c3d4", "Subject\n\nImpact-skip: other.rule: why")])
    ok(res[0][0] == "unmet" and notes, "a waiver of another rule waives nothing")
    res, _ = evaluate(_reg(r), base, [("a1b2c3d4", "Subject\n\nImpact-skip: x.y:")])
    ok(res[0][0] == "unmet", "a waiver without a reason waives nothing")
    rc = mkrule(when=["docs/decisions/F{n}.md"], kind="pair", **{"with_each": ["docs/history/decisions/F{n}-x.md", "docs/DECISIONS.md"]})
    res, _ = evaluate(_reg(rc), [ch("A", "docs/decisions/F7.md"), ch("M", "docs/DECISIONS.md")], [])
    ok(res[0][0] == "unmet" and "docs/history/decisions/F7-x.md" in res[0][2] and "DECISIONS.md" not in res[0][2], res)
    res, _ = evaluate(_reg(rc), [ch("A", "docs/decisions/F7.md"), ch("M", "docs/DECISIONS.md"),
                                 ch("A", "docs/history/decisions/F7-x.md")], [])
    ok(res[0][0] == "met", "with_each filled from the capture", res)
    rp = mkrule(when=["docs/decisions/F{n}.md"], kind="pair", **{"with": ["docs/DECISIONS.md"]})
    res, _ = evaluate(_reg(rp), [ch("A", "docs/decisions/F7.md"), ch("A", "docs/decisions/F8.md")], [])
    ok(res[0][0] == "unmet", "several triggers, one partner missing")

    # Forbid, held, review, planned, at.
    rf = mkrule(id="f.f", when=[".github/workflows/**"], on=["add"], kind="forbid", title="no hosted CI")
    res, _ = evaluate(_reg(rf), [ch("A", ".github/workflows/a.yml")], [])
    ok(res[0][0] == "unmet", "forbid refuses")
    res, _ = evaluate(_reg(rf), [ch("A", ".github/workflows/a.yml")], [("h", "S\n\nImpact-skip: f.f: a fork's file")])
    ok(res[0][0] == "waived", "forbid waived")
    rm = mkrule(id="m.m", when=["docs/{f}.md"], on=["delete"], kind="forbid",
                **{"with": ["docs/history/{f}.md"]})
    res, _ = evaluate(_reg(rm), [ch("D", "docs/X.md")], [])
    ok(res[0][0] == "unmet", "forbid with a move: a plain delete is refused")
    res, _ = evaluate(_reg(rm), [ch("D", "docs/X.md"), ch("A", "docs/history/X.md")], [])
    ok(res[0][0] == "met", "forbid with a move: the move passes")
    res, _ = evaluate(_reg(rm), [ch("D", "docs/X.md"), ch("A", "docs/history/Y.md")], [])
    ok(res[0][0] == "unmet", "forbid with a move: another file's move does not")
    meta = {"workspace_root": "/w", "workspace_members": ["a", "b"],
            "packages": [{"id": "a", "name": "faraday-desktop", "manifest_path": "/w/app/Cargo.toml"},
                         {"id": "b", "name": "lib", "manifest_path": "/w/lib/Cargo.toml"}],
            "resolve": {"nodes": [{"id": "a", "deps": [{"pkg": "b", "dep_kinds": [{"kind": None}]}]},
                                  {"id": "b", "deps": []}]}}
    regc = _reg()
    regc.consumers = [{"id": "app.faraday", "reads": ["app/**"]},
                      {"id": "app.starter", "reads": []}]
    errs = app_reads_lint(regc, ".", meta)
    ok(len(errs) == 1 and "lib" in errs[0], "an app crate no pattern reads", errs)
    regc.consumers[0]["reads"] = ["app/**", "lib/**"]
    ok(app_reads_lint(regc, ".", meta) == [], "every crate read")
    rh = mkrule(id="h.h", when=["a"], kind="held", held_by=["c"])
    rr = mkrule(id="r.r", when=["a"], kind="review", title="look", _extra=True)
    rpl = mkrule(id="p.p", when=["a"], kind="pair", status="planned", **{"with": ["b"]})
    rel = mkrule(id="e.e", when=["a"], at="release", kind="pair", **{"with": ["b"]})
    res, _ = evaluate(_reg(rh, rr, rpl, rel), [ch("M", "a")], [])
    ok([s for s, _, _ in res] == ["held", "review", "planned"], res)
    lines, unmet = render(res, False)
    ok(unmet == 0 and lines[-1] == "impact: 3 rules triggered: 1 held, 1 review, 1 planned", lines)

    # Review rules of the registry are attested in the commit.
    rv1 = mkrule(id="v.one", when=["a"], kind="review", title="look")
    rv2 = mkrule(id="v.two", when=["a"], kind="review", title="look again")
    c1 = [ch("M", "a")]
    res, _ = evaluate(_reg(rv1), c1, [])
    ok(res[0][0] == "unmet", "a review rule with no trailer")
    lines, unmet = render(res, True)
    ok(unmet == 1 and lines[0] == 'impact: unreviewed v.one: a; add "Reviewed: v.one" once checked', lines)
    res, _ = evaluate(_reg(rv1), c1, [("a1b2c3d4", "S\n\nBody\n\nReviewed: v.one\n")])
    ok(res[0][0] == "met" and "a1b2c3d4" in res[0][2], res)
    res, _ = evaluate(_reg(rv1, rv2), c1, [("a1b2c3d4", "S\n\nReviewed: v.two, v.one\n")])
    ok([s for s, _, _ in res] == ["met", "met"], res)
    res, _ = evaluate(_reg(rv1, rv2), c1, [("a1b2c3d4", "S\n\nReviewed: v.one\n")])
    ok([s for s, _, _ in res] == ["met", "unmet"], res)
    res, _ = evaluate(_reg(rv1), c1, [("a1b2c3d4", "S\n\nImpact-skip: v.one: nothing to judge\n")])
    ok(res[0][0] == "waived", res)
    res, _ = evaluate(_reg(rv1), c1, [("message", "S\n\nReviewed: v.one\n")])
    ok(res[0][0] == "met", "the message file counts as a commit")
    res, notes = evaluate(_reg(rv1, rh), c1, [("a1b2c3d4", "S\n\nReviewed: h.h, v.one, nope.x\n")])
    ok(any("h.h" in n for n in notes) and any("nope.x" in n for n in notes) and "unmet" not in [s for s, _, _ in res],
       (res, notes))
    res, notes = evaluate(_reg(rv1), [ch("M", "zzz")], [("a1b2c3d4", "S\n\nReviewed: v.one\n")])
    ok(res == [] and any("v.one" in n for n in notes), "a Reviewed line for a rule not triggered is a note")

    # Trailer rule and trailer parsing.
    rt = mkrule(id="t.t", when=["**/screen.rs"], kind="trailer", trailer="Material")
    res, _ = evaluate(_reg(rt), [ch("M", "a/screen.rs")], [])
    ok(res[0][0] == "unmet", "trailer missing")
    res, _ = evaluate(_reg(rt), [ch("M", "a/screen.rs")], [("e5f6a7b8", "S\n\nBody\n\nMaterial: Lists, Top app bar\n")])
    ok(res[0][0] == "met" and "Lists, Top app bar" in res[0][2], res)
    res, _ = evaluate(_reg(rt), [ch("M", "a/screen.rs")], [("e5f6a7b8", "S\n\nMaterial:\n")])
    ok(res[0][0] == "unmet", "a trailer without a value")
    res, _ = evaluate(_reg(rt), [ch("M", "a/screen.rs")], [("e5f6a7b8", "S\n\nMaterial: Lists\n\nprose after")])
    ok(res[0][0] == "unmet", "a trailer outside the last paragraph is prose")
    res, _ = evaluate(_reg(rt), [ch("M", "a/screen.rs")], [("e5f6a7b8", "S\n\nImpact-skip: t.t: only a label moved\n")])
    ok(res[0][0] == "waived", "trailer waived")
    ok(trailers("S\n\nA: b\nC: d e\n  f\n") == [("A", "b"), ("C", "d e f")], "continuation")
    ok(trailers("Material: Lists") == [], "a subject alone has no trailers")

    # Extra rules are named by id and file only.
    rx = mkrule(id="x.private", when=["a"], kind="pair", title="secret words", _extra=True, **{"with": ["b"]})
    rx["_file"] = "mine.toml"
    res, _ = evaluate(_reg(rx), [ch("M", "a")], [])
    lines, _ = render(res, True)
    ok(lines[0] == "impact: unmet x.private: (a rule of mine.toml)" and "secret" not in "".join(lines), lines)

    # git output parsing.
    ns = parse_name_status("M\0a.rs\0A\0b.rs\0D\0c.rs\0R095\0old.rs\0new.rs\0")
    ok(ns == [("M", "a.rs"), ("A", "b.rs"), ("D", "c.rs"), ("D", "old.rs"), ("A", "new.rs")], ns)
    d = parse_diff(
        "diff --git a/x.txt b/x.txt\n--- a/x.txt\n+++ b/x.txt\n@@ -1 +1 @@\n-old\n+new\n"
        "diff --git a/gone.txt b/gone.txt\n--- a/gone.txt\n+++ /dev/null\n@@ -1 +0,0 @@\n-bye\n"
        "diff --git a/n.txt b/n.txt\n--- /dev/null\n+++ b/n.txt\n@@ -0,0 +1 @@\n+--- hi\n")
    ok(d == {"x.txt": ["-old", "+new"], "gone.txt": ["-bye"], "n.txt": ["+--- hi"]}, d)

    # Plan.
    reg = _reg(mkrule(id="docs.a", when=["docs/*.md"], kind="pair", **{"with": ["docs/README.md"]}),
               mkrule(id="docs.b", when=["docs/decisions/F{n}.md"], kind="pair", **{"with": ["docs/DECISIONS.md"], "with_each": ["docs/x-{n}.md"]}),
               mkrule(id="s.h", when=["a/**"], kind="held", held_by=["c"]),
               mkrule(id="s.t", when=["a/**"], kind="trailer", trailer="Material"),
               mkrule(id="s.r", when=["a/**"], kind="review"),
               mkrule(id="s.rel", when=["a/**"], at="release", kind="review"))
    reg.consumers.append({"id": "app.z", "reads": ["a/**"], "_file": "", "_extra": False})
    out = plan(reg, ["a/b.rs", "docs/decisions/F9.md"], "change", False)
    ok(out[0] == "impact: 2 paths, 4 rules", out)
    ok(any("docs.b" in l and "docs/DECISIONS.md and docs/x-9.md" in l for l in out), out)
    ok("held by: c" in out and "trailer: s.t (Material: <value>)" in out and "needs a Reviewed: <rule> line in the commit: s.r" in out
       and "read by: app.z" in out, out)
    ok("s.rel" not in " ".join(out) and "s.rel" in " ".join(plan(reg, ["a/b.rs"], "release", False)), "moments")
    ok(plan(reg, [], "change", False) == ["impact: 0 paths, 0 rules"], "empty plan")
    ok(expand_paths(["docs/**/*.md", "tools", "new/file"], {"docs/a.md", "tools/x", "tools/y/z"}) ==
       ["docs/a.md", "tools/x", "tools/y/z", "new/file"], "expand_paths")

    # Lint, on a small tree.
    with tempfile.TemporaryDirectory() as td:
        def put(rel, text):
            p = Path(td) / rel
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(text, encoding="utf-8")
        put("docs/A.md", "# A\n\n## 1. First\n\n### 1.2 Second\n")
        put("tools/l.sh", "")
        put("justfile", "lint:\n    tools/l.sh\n    # a comment\n\n    tools/m.sh\n\nother:\n    x\n")
        put("tools/impact.toml", """
[[check]]
id = "l"
run = "tools/l.sh"
[[check]]
id = "m"
run = "tools/m.sh"
[[rule]]
id = "docs.a"
source = ["docs/A.md §1.2"]
when = ["tools/*.sh"]
kind = "held"
held_by = ["l"]
consumers = ["app.q"]
[[consumer]]
id = "app.q"
reads = ["tools/**"]
""")
        tracked = {"docs/A.md", "tools/l.sh", "justfile", "tools/impact.toml"}
        reg = load_registry(td, "")
        jf = (Path(td) / "justfile").read_text()
        errs = lint(reg, td, tracked, jf)
        ok(errs == [], errs)
        put("tools/impact.toml", (Path(td) / "tools/impact.toml").read_text()
            + """
[[rule]]
id = "docs.a"
source = ["docs/A.md §9", "docs/Gone.md"]
when = ["tools/*.py", "docs/Nope.md"]
kind = "held"
held_by = ["nothing"]
consumers = ["web.q"]
[[rule]]
id = "repo.late"
kind = "pair"
[[rule]]
id = "docs.misc"
kind = "held"
status = "active"
""")
        errs = lint(load_registry(td, ""), td, tracked, jf.replace("    tools/l.sh\n", "    tools/l.sh\n    tools/extra.sh\n"))
        text = "\n".join(errs)
        for want in ("docs.a: id used twice", "has no section 9", "Gone.md does not exist",
                     "when tools/*.py matches no tracked path", "Nope.md does not exist",
                     "held_by nothing is no check", "consumers web.q is no consumer",
                     "repo.late: out of the areas' order", "a pair names nothing",
                     "docs.misc: a held rule names no built check",
                     "`tools/extra.sh` is no [[check]]"):
            ok(want in text, want, "|", text)
        ok(lint(load_registry(td, ""), td, tracked, "lint:\n    tools/l.sh\n")[-1:] != [] , "recipe missing a check")
        # An amend adds, an extra rule keeps its file's name.
        put("extra.toml", '[[amend]]\nid = "docs.a"\nheld_by = ["m"]\nmisses = "more"\n'
            '[[rule]]\nid = "x.mine"\nwhen = ["private/**"]\nkind = "review"\n')
        reg = load_registry(td, str(Path(td) / "extra.toml"))
        a = next(r for r in reg.rules if r["id"] == "docs.a")
        ok("m" in a["held_by"] and "more" in a["misses"], a)
        ok(not [e for e in lint(reg, td, tracked, jf) if "x.mine" in e], "an extra rule may name untracked paths")
        put("bad.toml", '[[rule]]\nid = "r"\nbogus = 1\nkind = 3\n')
        ok(load_registry(td, str(Path(td) / "bad.toml")).errors, "unknown field and wrong type")
    # docs/CHECKS.md: generated, escaped, stable.
    reg = _reg(mkrule(id="docs.a", title="a | b", when=["docs/*.md"], kind="pair",
                      **{"with": ["docs/README.md"]}, source=["CLAUDE.md"]),
               mkrule(id="impact.z", kind="review", status="planned"))
    reg.checks = [{"id": "c", "run": "tools/c.sh", "stage": "just lint", "catches": "x|y",
                   "misses": "", "status": "built"}]
    reg.consumers = [{"id": "app.q", "name": "Q", "reads": ["a/b"], "act": "redo"}]
    md1 = render_checks_md(reg)
    ok(md1 == render_checks_md(reg), "docs output is stable")
    ok("a \\| b" in md1 and "x\\|y" in md1, "pipes escaped in cells")
    ok("**Kind:** Reference" in md1 and "### 2.1 docs" in md1 and "(planned)" in md1, "docs shape")
    ok(md1.count("`impact.z`") == 2, "a review rule is in its area and in the review list")
    ok("`app.q`" in md1 and "| redo |" in md1, "consumers listed")
    # report: the extra file's rules marked, the public docs never.
    rx2 = mkrule(id="docs.extra", kind="review", _extra=True)
    rx2["_file"] = "mine.toml"
    reg2 = _reg(mkrule(id="docs.a", kind="review"), rx2)
    reg2.extra_files = ["mine.toml"]
    rep = render_checks_md(reg2, report=True)
    ok("`docs.extra` (of mine.toml)" in rep and "`mine.toml`" in rep, "report marks an extra rule with its file")
    ok("**Kind:**" not in rep and "## 5. Rules of each moment" in rep, "report shape")
    ok("(of mine.toml)" not in render_checks_md(reg2), "docs marks nothing")
    # check --consumer: selection, states, blocking, the site's side.
    mkc = lambda cid, **kw: {"id": cid, "name": "", "reads": [], "act": "redo", "site_check": "",
                             "site_with": [], "blocking": False, "_file": "impact.toml",
                             "_extra": False, **kw}
    cg = mkc("website.guide", reads=["docs/guide/**"], blocking=True, site_with=["site/guide/**"])
    ct = mkc("website.tour", reads=["a/screen.rs"])
    ca = mkc("app.x", reads=["a/**"], blocking=True)
    reg3 = _reg(mkrule(id="guide.pictures", kind="review", when=["a/screen.rs"],
                       consumers=["website.guide"]))
    reg3.consumers = [cg, ct, ca]
    ok([c["id"] for c in select_consumers(reg3, "website")] == ["website.guide", "website.tour"], "prefix selects")
    ok([c["id"] for c in select_consumers(reg3, "website.tour")] == ["website.tour"], "exact id selects")
    ok(not select_consumers(reg3, "webs"), "a part of a name selects nothing")
    sel = select_consumers(reg3, "website")
    chs = [ch("M", "docs/guide/a.md"), ch("M", "a/screen.rs")]
    res3, _ = evaluate(reg3, chs, [])
    st = {c["id"]: s for s, c, _ in consumer_states(sel, chs, [], res3, None)}
    ok(st == {"website.guide": "pending", "website.tour": "listed"}, "blocking pending, picture listed", st)
    lines, pending = render_consumers(sel, chs, [], res3, None, False)
    ok(pending == 1 and lines[0].startswith("impact: consumer pending website.guide (blocking): docs/guide/a.md changed"),
       "pending line", lines)
    ok(any("consumer listed website.tour" in l for l in lines) and "guide.pictures" in "".join(lines),
       "listed consumer and the rule bearing on it", lines)
    ok(lines[-1].startswith("impact: 2 consumers affected: 1 pending, 1 listed"), "count line", lines)
    lines, pending = render_consumers(sel, [ch("M", "a/screen.rs")], [], evaluate(reg3, [ch("M", "a/screen.rs")], [])[0], None, True)
    ok(pending == 1 and not any("consumer listed" in l for l in lines), "quiet drops the listed", lines)
    lines, pending = render_consumers(select_consumers(reg3, "website.tour"), [ch("M", "a/screen.rs")], [],
                                      evaluate(reg3, [ch("M", "a/screen.rs")], [])[0], None, False)
    ok(pending == 0, "a picture never blocks")
    ok(not consumer_states(sel, [ch("M", "README.md")], [], [], None), "an unrelated change affects none")
    ok(consumer_states(sel, [ch("M", "x")], [("h", "S\n\nImpact-skip: website.guide: no page changed")], [], None) == [],
       "no change, no consumer")
    sk = consumer_states(sel, chs[:1], [("h1", "S\n\nImpact-skip: website.guide: only a typo")], [], None)
    ok(sk[0][0] == "pending", "without the site, a waiver in the Faraday range is not enough to know it is done")
    with tempfile.TemporaryDirectory() as td:
        git = lambda *a: subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", "-C", td, *a],
                                        capture_output=True, check=True)
        git("init", "-q")
        (Path(td) / "a").write_text("1")
        git("add", "."), git("commit", "-q", "-m", "first")
        since = git("rev-parse", "HEAD").stdout.decode().strip()
        ok(site_commits(td, since) == [] and site_commits(td, "") == [], "no commits after the pin")
        site = (td, site_commits(td, since))
        ok(consumer_states(sel, chs[:1], [], [], site)[0][0] == "pending", "site untouched: pending")
        (Path(td) / "site" / "guide").mkdir(parents=True)
        (Path(td) / "site" / "guide" / "p.html").write_text("x")
        git("add", "."), git("commit", "-q", "-m", "guide made again")
        site = (td, site_commits(td, since))
        r1 = consumer_states(sel, chs[:1], [], [], site)
        ok(r1[0][0] == "met" and "site changed" in r1[0][2], "a site_with path changed: met", r1)
        (Path(td) / "b").write_text("2")
        git("add", "."), git("commit", "-q", "-m", "other\n\nImpact-skip: website.guide: no page differs")
        site = (td, site_commits(td, site_commits(td, since)[-1][0]))
        r2 = consumer_states(sel, chs[:1], [], [], site)
        ok(r2[0][0] == "waived" and "no page differs" in r2[0][2], "a website commit's waiver", r2)
        cg2 = dict(cg, site_check="true", site_with=[])
        ok(consumer_states([cg2], chs[:1], [], [], (td, []))[0][0] == "met", "site_check passing: met")
        cg3 = dict(cg, site_check="false", site_with=[])
        ok(consumer_states([cg3], chs[:1], [], [], (td, []))[0][0] == "pending", "site_check failing: pending")
        # --repo: the range is read from another repository than the script's.
        (Path(td) / "c").write_text("3")
        git("add", "."), git("commit", "-q", "-m", "third")
        run = subprocess.run([sys.executable, "-I", "-B", str(Path(__file__).resolve()), "check",
                              f"{since}..HEAD", "--repo", td], capture_output=True)
        ok(b"cannot read the range" not in run.stdout and run.returncode in (0, 1), "--repo reads that repository's range",
           run.stdout)
    print("impact self-test: ok")


def _reg(*rules):
    reg = Registry()
    reg.rules = list(rules)
    return reg


def main(argv):
    import argparse
    ap = argparse.ArgumentParser(prog="impact.py", description=__doc__.split("\n")[0])
    ap.add_argument("--self-test", action="store_true")
    sub = ap.add_subparsers(dest="cmd")
    p = sub.add_parser("plan")
    p.add_argument("path", nargs="*")
    p.add_argument("--at", default="change")
    p.add_argument("--long", action="store_true")
    p = sub.add_parser("check")
    p.add_argument("range", nargs="?")
    p.add_argument("--quiet", action="store_true")
    p.add_argument("--message")
    p.add_argument("--paths", nargs="+")
    p.add_argument("--repo")
    p.add_argument("--consumer")
    p.add_argument("--site")
    p.add_argument("--site-since")
    sub.add_parser("lint")
    p = sub.add_parser("report")
    p.add_argument("--out")
    p = sub.add_parser("docs")
    p.add_argument("--check", action="store_true")
    args = ap.parse_args(argv)
    if args.self_test:
        self_test()
        return 0
    if args.cmd == "plan":
        return cmd_plan(args)
    if args.cmd == "check":
        return cmd_check(args)
    if args.cmd == "lint":
        return cmd_lint(args)
    if args.cmd == "docs":
        return cmd_docs(args)
    if args.cmd == "report":
        return cmd_report(args)
    ap.print_usage()
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
