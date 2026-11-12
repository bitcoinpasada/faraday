#!/usr/bin/env bash
# Layout lint (F74, F196, F248, docs/STRUCTURE-PASS.md §7.1 and §7.2): the code map
# says where everything is, and code stays mapped and shaped by function.
#
# 1. The code map. Every module faraday-core's lib.rs declares is named,
#    in backticks, in lib.rs's `//!` code map, and every .rs file and
#    folder at the top of faraday/faraday-core/src/ is a declared module.
#    Every `Screen` variant has a row in the map's table, and the file
#    the row names is the one screens/mod.rs's dispatch draws it from.
#
# 2. A file has no size limit (F248): code is split by function, not length.
#
# 3. Function size. Clippy's `too_many_lines` is on for every crate
#    (Cargo.toml's workspace lints, `clippy -D warnings`), so a function
#    over 100 lines fails unless it carries
#    `#[expect(clippy::too_many_lines, …)]`. The number of those markers
#    and of `#[allow(clippy::too_many_arguments)]` is not counted, but
#    each marker (over one line or several) carries `reason = "…"`; one
#    without fails, naming file:line.
#
# 4. Faraday's shape (docs/STRUCTURE-PASS.md §7.4.5). The number of
#    top-level fields of `pub struct Faraday` in lib.rs (`fields Faraday
#    N`) only falls, and so does the list of files under faraday/ with an
#    inherent `impl Faraday {` (`impl-faraday PATH`): a file not listed
#    fails, and so does a listed file that has none.
#
# 5. Dependency direction: core/ crates stay inside core/; nothing under
#    core/ or faraday/ depends on aqrv/; the drawing and
#    string crates do not depend on faraday-core or faraday-starter.
#
# Raising a count is the owner's decision, recorded in DECISIONS (F74, F196).
# Prints each failure and exits non-zero if there is one. Run from the
# repository root; `just lint` does.
set -u
cd "$(dirname "$0")/.."

exec python3 - <<'PY'
import os
import re
import subprocess
import sys

COUNTS = "tools/lint-layout.counts"
CORE = "faraday/faraday-core/src"

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
counts = {}
impl_listed = set()
for n, line in enumerate(open(COUNTS, encoding="utf-8"), 1):
    line = line.split("#", 1)[0].strip()
    if not line:
        continue
    f = line.split()
    if f[0] == "fields" and len(f) == 3 and f[1] == "Faraday":
        counts["fields Faraday"] = int(f[2])
    elif f[0] == "impl-faraday" and len(f) == 2:
        impl_listed.add(f[1])
    else:
        fail(f"{COUNTS}:{n}: not `fields Faraday N` or `impl-faraday PATH`")


for key in ("fields Faraday",):
    if key not in counts:
        fail(f"{COUNTS} has no `{key} N` line")


def ratchet(what, now, kept, where):
    if now > kept:
        fail(f"{where}: {what} is {now}, above {kept} in {COUNTS}: "
             f"bring it down (raising it is the owner's decision, F74)")
    elif now < kept:
        fail(f"{where}: {what} is {now}, below {kept}: lower it in {COUNTS}")


files = tracked_rs()

# ---- 3. Function size ----
# Every member crate turns the lint on: through the workspace's lints, or
# in its own `[lints.clippy]` where it sets its own `[lints.rust]`.
members = subprocess.run(["git", "ls-files", "-z", "*Cargo.toml"], check=True,
                         capture_output=True, text=True).stdout.split("\0")
for manifest in sorted(m for m in members if m and m != "Cargo.toml"):
    # Others' code, copied unchanged (tools/reference/*/README.md), is not ours to edit.
    if manifest.startswith(("fuzz/", "faraday/fuzz/", "third_party/", "tools/reference/")):
        continue
    text = open(manifest, encoding="utf-8").read()
    if "[package]" not in text:
        continue
    inherits = re.search(r"^\[lints\]\s*\nworkspace = true", text, re.M)
    own = re.search(r"^\[lints\.clippy\][^\[]*^too_many_lines = \"warn\"", text, re.M | re.S)
    if not (inherits or own):
        fail(f"{manifest}: clippy's too_many_lines is not on (workspace lints, or [lints.clippy])")

marker = re.compile(
    r"#\[(?:expect|allow)\(\s*clippy::too_many_(?:lines|arguments)\b.*?\)\]", re.S)
for path in tracked_rs():
    text = open(path, encoding="utf-8").read()
    for m in marker.finditer(text):
        if not re.search(r'\breason\s*=\s*"', m.group(0)):
            fail(f"{path}:{text.count(chr(10), 0, m.start()) + 1}: a "
                 "too_many_lines / too_many_arguments marker without `reason = \"…\"`")


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

# 5. Dependency direction (docs/STRUCTURE-PASS.md §7). A `core/*` crate has
#    no `path` dependency outside core/, and faraday-ui, faraday-draw,
#    faraday-strings and faraday-learn depend (normal, build or dev) on
#    neither faraday-core nor faraday-starter. No crate under core/ or
#    faraday/ has a path dependency into aqrv/ (F245: the Argon QR Vault
#    is a test tool the apps do not depend on).
import tomllib

LOWER = {"faraday-ui", "faraday-draw", "faraday-strings", "faraday-learn"}
UPPER = {"faraday-core", "faraday-starter"}
with open("Cargo.toml", "rb") as fh:
    members = tomllib.load(fh).get("workspace", {}).get("members", [])
for member in members:
    manifest = os.path.join(member, "Cargo.toml")
    if not os.path.exists(manifest):
        continue
    with open(manifest, "rb") as fh:
        data = tomllib.load(fh)
    tables = [data]
    tables += list(data.get("target", {}).values())
    deps = {}
    for t in tables:
        for kind in ("dependencies", "build-dependencies", "dev-dependencies"):
            deps.update(t.get(kind, {}))
    name = data.get("package", {}).get("name", member)
    for dep, spec in deps.items():
        real = spec.get("package", dep) if isinstance(spec, dict) else dep
        if member.startswith("core/") and isinstance(spec, dict) and "path" in spec:
            target = os.path.normpath(os.path.join(member, spec["path"]))
            if not target.startswith("core/"):
                fail(f"{manifest}: {name} depends on {target}, outside core/")
        if (member.startswith(("core/", "faraday/")) and isinstance(spec, dict)
                and "path" in spec):
            target = os.path.normpath(os.path.join(member, spec["path"]))
            if target.startswith("aqrv/"):
                fail(f"{manifest}: {name} depends on {target}, a crate of the Argon QR Vault")
        if name in LOWER and real in UPPER:
            fail(f"{manifest}: {name} depends on {real}; it is a layer below it")

for f in failures:
    print(f"lint-layout: {f}")
if failures:
    sys.exit(1)
print("lint-layout: ok")
PY
