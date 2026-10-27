#!/usr/bin/env bash
# Layout lint (F74, docs/STRUCTURE-PASS.md §7.1 and §7.2): the code map
# says where everything is, and no file or function grows back into a
# monolith.
#
# 1. The code map. Every module faraday-core's lib.rs declares is named,
#    in backticks, in lib.rs's `//!` code map, and every .rs file and
#    folder at the top of faraday/faraday-core/src/ is a declared module.
#    Every `Screen` variant has a row in the map's table, and the file
#    the row names is the one screens/mod.rs's dispatch draws it from.
#
# 2. File size. No tracked .rs file is over 2,500 lines, except those
#    listed in tools/lint-layout.counts at their size (`file PATH LINES`).
#    A listed file that grows fails; one that shrinks fails too until its
#    line is lowered (or removed, at 2,500 or under), so the count only
#    falls. Not counted, as they are data whose size the content decides:
#    the strings file (every string is in it, by rule) and the published
#    word lists.
#
# 3. Function size. Clippy's `too_many_lines` is on for every crate
#    (Cargo.toml's workspace lints, `clippy -D warnings`), so a function
#    over 100 lines fails unless it carries
#    `#[expect(clippy::too_many_lines, …)]`. tools/lint-layout.counts holds
#    how many such expects there are (`expect too_many_lines N`), and how
#    many `#[allow(clippy::too_many_arguments)]` (`allow
#    too_many_arguments N`). Each count only falls, as in 2.
#
# 4. Faraday's shape (docs/STRUCTURE-PASS.md §7.4.5). The number of
#    top-level fields of `pub struct Faraday` in lib.rs (`fields Faraday
#    N`) only falls, and so does the list of files under faraday/ with an
#    inherent `impl Faraday {` (`impl-faraday PATH`): a file not listed
#    fails, and so does a listed file that has none.
#
# Raising a count is the owner's decision, recorded in DECISIONS (F74).
# Prints each failure and exits non-zero if there is one. Run from the
# repository root; `just lint` does.
set -u
cd "$(dirname "$0")/.."

exec python3 - <<'PY'
import os
import re
import subprocess
import sys

LIMIT = 2500
COUNTS = "tools/lint-layout.counts"
CORE = "faraday/faraday-core/src"
NOT_COUNTED = re.compile(
    r"^(faraday/faraday-strings/src/en\.rs"
    r"|core/faraday-bip/src/wordlists/.*"
    r"|core/faraday-bip/src/diceware/large\.rs)$"
)

failures = []


def fail(msg):
    failures.append(msg)


def tracked_rs():
    out = subprocess.run(["git", "ls-files", "-z", "--cached", "--others",
                          "--exclude-standard", "*.rs"],
                         check=True, capture_output=True, text=True).stdout
    return sorted(f for f in set(out.split("\0")) if f and os.path.exists(f))


# ---- 1. The code map ----
lib = open(f"{CORE}/lib.rs", encoding="utf-8").read()
doc = "\n".join(l[3:].removeprefix(" ") for l in lib.split("\n") if l.startswith("//!"))
mods = re.findall(r"^(?:pub(?:\([a-z]+\))? )?mod (\w+);", lib, re.M)
named = set(re.findall(r"`([a-z_0-9]+)(?:/[^`]*)?`", doc))
for m in mods:
    if m not in named:
        fail(f"{CORE}/lib.rs: module `{m}` is not named in the code map")
for entry in sorted(os.listdir(CORE)):
    if entry == "lib.rs":
        continue
    stem = entry[:-3] if entry.endswith(".rs") else entry
    if (entry.endswith(".rs") or os.path.isdir(f"{CORE}/{entry}")) and stem not in mods:
        fail(f"{CORE}/{entry}: not a module lib.rs declares")

enum = re.search(r"pub enum Screen \{(.*?)\n\}", lib, re.S)
variants = re.findall(r"^\s+([A-Z]\w*),", enum.group(1), re.M) if enum else []
if not variants:
    fail(f"{CORE}/lib.rs: no `Screen` enum found")
table = dict(re.findall(r"^\| `(\w+)` \| `([^`]+)` \|$", doc, re.M))
dispatch_src = open(f"{CORE}/screens/mod.rs", encoding="utf-8").read()
dispatch = dict(re.findall(r"Screen::(\w+) => ((?:crate::)?[a-z_0-9:]+)::\w+\(",
                           dispatch_src))
for v in variants:
    row = table.get(v)
    if row is None:
        fail(f"{CORE}/lib.rs: `Screen::{v}` has no row in the code map's table")
        continue
    if not os.path.exists(f"{CORE}/{row}"):
        fail(f"{CORE}/lib.rs: `Screen::{v}`'s row names {row}, which does not exist")
    path = dispatch.get(v)
    if path is None:
        fail(f"{CORE}/screens/mod.rs: `Screen::{v}` is not dispatched to a draw function")
        continue
    if path.startswith("crate::"):
        parts = path[len("crate::"):].split("::")
    else:
        parts = ["screens"] + path.split("::")
    drawn = "/".join(parts) + ".rs"
    if not os.path.exists(f"{CORE}/{drawn}"):
        drawn = "/".join(parts) + "/mod.rs"
    if drawn != row:
        fail(f"{CORE}/lib.rs: `Screen::{v}` is drawn in {drawn}, the code map says {row}")
for v in table:
    if v not in variants:
        fail(f"{CORE}/lib.rs: the code map's table names `{v}`, which is no Screen")

# ---- the counts file ----
sizes = {}
counts = {}
impl_listed = set()
for n, line in enumerate(open(COUNTS, encoding="utf-8"), 1):
    line = line.split("#", 1)[0].strip()
    if not line:
        continue
    f = line.split()
    if f[0] == "file" and len(f) == 3:
        sizes[f[1]] = int(f[2])
    elif f[0] in ("expect", "allow") and len(f) == 3:
        counts[f"{f[0]} {f[1]}"] = int(f[2])
    elif f[0] == "fields" and len(f) == 3 and f[1] == "Faraday":
        counts["fields Faraday"] = int(f[2])
    elif f[0] == "impl-faraday" and len(f) == 2:
        impl_listed.add(f[1])
    else:
        fail(f"{COUNTS}:{n}: not `file PATH LINES`, `expect|allow LINT N`, "
             f"`fields Faraday N` or `impl-faraday PATH`")


for key in ("expect too_many_lines", "allow too_many_arguments", "fields Faraday"):
    if key not in counts:
        fail(f"{COUNTS} has no `{key} N` line")


def ratchet(what, now, kept, where):
    if now > kept:
        fail(f"{where}: {what} is {now}, above {kept} in {COUNTS}: "
             f"bring it down (raising it is the owner's decision, F74)")
    elif now < kept:
        fail(f"{where}: {what} is {now}, below {kept}: lower it in {COUNTS}")


# ---- 2. File size ----
files = tracked_rs()
for path in files:
    if NOT_COUNTED.match(path):
        continue
    with open(path, encoding="utf-8", errors="replace") as fh:
        lines = sum(1 for _ in fh)
    if path in sizes:
        if lines <= LIMIT:
            fail(f"{path}: {lines} lines, at or under {LIMIT}: remove its line from {COUNTS}")
        else:
            ratchet("its length", lines, sizes[path], path)
    elif lines > LIMIT:
        fail(f"{path}: {lines} lines, over {LIMIT}: split it")
for path in sizes:
    if path not in files:
        fail(f"{COUNTS}: {path} is no tracked file: remove its line")

# ---- 3. Function size ----
# Every member crate turns the lint on: through the workspace's lints, or
# in its own `[lints.clippy]` where it sets its own `[lints.rust]`.
members = subprocess.run(["git", "ls-files", "-z", "*Cargo.toml"], check=True,
                         capture_output=True, text=True).stdout.split("\0")
for manifest in sorted(m for m in members if m and m != "Cargo.toml"):
    if manifest.startswith(("fuzz/", "faraday/fuzz/", "third_party/")):
        continue
    text = open(manifest, encoding="utf-8").read()
    if "[package]" not in text:
        continue
    inherits = re.search(r"^\[lints\]\s*\nworkspace = true", text, re.M)
    own = re.search(r"^\[lints\.clippy\][^\[]*^too_many_lines = \"warn\"", text, re.M | re.S)
    if not (inherits or own):
        fail(f"{manifest}: clippy's too_many_lines is not on (workspace lints, or [lints.clippy])")

expects = allows = 0
for path in files:
    text = open(path, encoding="utf-8", errors="replace").read()
    expects += len(re.findall(r"#\[expect\(\s*clippy::too_many_lines\b", text))
    allows += len(re.findall(r"#\[allow\(\s*clippy::too_many_arguments\b", text))
ratchet("the number of `expect(clippy::too_many_lines)`", expects,
        counts.get("expect too_many_lines", 0), "functions")
ratchet("the number of `allow(clippy::too_many_arguments)`", allows,
        counts.get("allow too_many_arguments", 0), "functions")

# ---- 4. Faraday's shape ----
start = lib.find("pub struct Faraday {\n")
if start < 0:
    fail(f"{CORE}/lib.rs: no `pub struct Faraday {{` found")
else:
    body = lib[start:].split("\n")[1:]
    fields = 0
    for line in body:
        if line == "}":
            break
        if re.match(r"^    (pub(\([a-z]+\))? )?[a-z_][a-z0-9_]*:", line):
            fields += 1
    ratchet("Faraday's top-level fields", fields, counts.get("fields Faraday", 0),
            f"{CORE}/lib.rs")

IMPL = re.compile(r"^impl(<[^>]*>)? (crate::)?Faraday \{", re.M)
having = set()
for path in files:
    if not path.startswith("faraday/"):
        continue
    with open(path, encoding="utf-8", errors="replace") as fh:
        if IMPL.search(fh.read()):
            having.add(path)
for path in sorted(having - impl_listed):
    fail(f"{path}: a new `impl Faraday`: move it into the area's struct or its glue file")
for path in sorted(impl_listed - having):
    if path in files:
        fail(f"{COUNTS}: {path} has no `impl Faraday` now: remove its line")
    else:
        fail(f"{COUNTS}: {path} is no tracked file: remove its line")

for f in failures:
    print(f"lint-layout: {f}")
if failures:
    sys.exit(1)
print("lint-layout: ok")
PY
