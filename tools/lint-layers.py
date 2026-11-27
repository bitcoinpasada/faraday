#!/usr/bin/env python3
"""lint-layers: every crate declares its layer, and every edge and every path a layer names is one
the layers allow (docs/spec/01-layers.md §1 to §11 and the `lint-layers` rows of sections 02 to 06
and 09).

It reads `cargo metadata --format-version 1 --no-deps --locked` (or a file given with
`--metadata`), each crate's manifest for `[lints]`, and the Rust source of each crate (`src/`; `tests/`
for rule A9). Every refusal names the crate, the edge or the file and line, and the rule number; all
of them are reported in one run. Exit 1 if any.

Standard library only: run as `python3 -I -B tools/lint-layers.py`.
"""

import argparse
import importlib.util
import json
import subprocess
import sys
import tomllib
from collections import defaultdict
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent


def _load(name):
    spec = importlib.util.spec_from_file_location(name, HERE / f"{name}.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


lex = _load("rustlex")

# ---------------------------------------------------------------------------------------------
# The vocabulary of section 01 §1 and 09 §1.3.

LAYERS = ("domain", "port", "kit", "app", "runtime", "adapter", "root", "tool")
LEVELS = ("tokens", "atoms", "molecules", "organisms", "templates")
ROLES = ("kernel", "present", "part", "feature", "composite", "product")
CHECKS = ("domain", "kit", "app", "runtime", "adapter", "root")  # 07 §3.1: the six owners
LEVEL_RANK = {name: i for i, name in enumerate(LEVELS)}

# key -> (what its value must be, the layer that may carry it or None for any)
KEYS = {
    "layer": ("layer", None),
    "level": ("level", "kit"),
    "role": ("role", "app"),
    "owner": ("true", "app"),
    "online": ("true", None),
    "offline": ("true", "root"),
    "ships": ("false", "root"),
    "checks": ("checks", "tool"),
    "no_std": ("true", None),
    "boundary": ("true", "domain"),
    "wording": ("true", "domain"),
    "process": ("true", "adapter"),
    "values": ("str", "port"),
    "reexports": ("strlist", "kit"),
}

# D2 (01 §2.1): the crates that are `no_std` on `main` and stay so. `faraday-codec` is `no_std` only
# without its `decode` feature, so its metadata is not required here. The kit's crates made from
# `faraday-draw` are the tokens and the draw crates (09 §3.1).
D2_CRATES = (
    "faraday-text", "faraday-crypto", "faraday-bip", "faraday-psbt", "faraday-entropy",
    "faraday-selftest", "faraday-kef", "faraday-pgp", "faraday-fat", "faraday-ots", "faraday-sb",
    "faraday-vault", "faraday-port", "faraday-tokens", "faraday-draw",
)  # fmt: skip

# §2.2 and §2.3: the crates whose metadata names `wording` and `boundary`.
WORDING_CRATES = ("faraday-strings", "faraday-guide")
BOUNDARY_CRATES = ("faraday-keys", "faraday-vault")

# U2 (01 §10): the closed list of crates that set `unsafe_code = "deny"` and allow it on one item.
UNSAFE_CRATES = (
    "faraday-crypto", "faraday-storage", "faraday-grant", "faraday-v4l2", "faraday-mediafoundation",
    "faraday-window", "faraday-os-adapter", "faraday-alloc-count",
    "faraday-leak-scan",
)  # fmt: skip

# K1 (01 §6.2): a kit crate may depend on a crate of its own level only where the manifest names the
# edge (09 §3.1: the atoms level is three crates: the drawing base, the derive and the controls).
SAME_LEVEL_EDGES = (("faraday-atoms", "faraday-draw"), ("faraday-atoms", "faraday-atoms-derive"))

# §2.2: a wording crate depends only on these domain crates and the other wording crate.
WORDING_MAY_DEPEND = ("faraday-text", "faraday-crypto", "faraday-bip")
# §2.3: the input parsers and the registry a boundary crate may not depend on, nor depend on a crate that does.
BOUNDARY_NEVER = ("faraday-intake", "faraday-forms")
# P5: the port's own dependencies among the workspace's crates.
PORT_MAY_DEPEND = ("faraday-text", "faraday-crypto")
# 01 §1: the presentation kernel may name the passphrase part, for `pass_model` through its read view (A3).
PASS_FIELD_CRATE = "faraday-pass-field"
PASS_FIELD_READ_VIEW = "Read"
# S10: the device composite.
DEVICE_COMPOSITE = "faraday-device"

# 01 §5 and 04 §2.2: the modules of a feature crate, and the modules each never names.
MODULES = ("state", "msg", "update", "model", "present", "page")
SIBLINGS_NEVER = {
    "state": ("present", "model", "page"),
    "msg": ("present", "model", "page"),
    "update": ("present", "model", "page"),
    "model": ("state", "update"),
    "present": ("update", "page"),
    "page": ("state", "update", "present"),
}


class Crate:
    def __init__(self, pkg, root):
        self.pkg = pkg
        self.name = pkg["name"]
        self.ident = lex.crate_ident(self.name)
        self.manifest = Path(pkg["manifest_path"])
        self.dir = self.manifest.parent
        self.rel = _rel(self.dir, root)
        faraday = (pkg.get("metadata") or {}).get("faraday")
        self.has_meta = faraday is not None
        self.md = faraday if isinstance(faraday, dict) else {}
        self.layer = self.md.get("layer")
        self.level = self.md.get("level")
        self.role = self.md.get("role")
        self.deps = []  # (target name, kind, rename)

    def flag(self, key):
        return self.md.get(key) is True

    @property
    def ships(self):
        return self.layer == "root" and self.md.get("ships", True) is not False

    def level_rank(self):
        return LEVEL_RANK.get(self.level, -1)


def _rel(path, root):
    try:
        return str(Path(path).resolve().relative_to(Path(root).resolve()))
    except ValueError:
        return str(path)


class Refusals:
    def __init__(self):
        self.items = []

    def add(self, rule, where, why):
        self.items.append((rule, where, why))


def load(meta, root):
    members = set(meta.get("workspace_members", []))
    pkgs = [p for p in meta["packages"] if not members or p["id"] in members]
    crates = {p["name"]: Crate(p, root) for p in pkgs}
    for c in crates.values():
        for d in c.pkg.get("dependencies", []):
            if d["name"] in crates and (d.get("path") or d.get("source") is None):
                c.deps.append((d["name"], d.get("kind") or "normal", d.get("rename")))
    return crates


# ---------------------------------------------------------------------------------------------
# 01 §1, 09 §1.3: the metadata.


def check_metadata(crates, r):
    for c in crates.values():
        where = c.rel
        if not c.has_meta or "layer" not in c.md:
            r.add("§1", where, "the crate has no `[package.metadata.faraday] layer`")
            continue
        for key, val in c.md.items():
            if key not in KEYS:
                r.add("§1", where, f"the key `{key}` is not one the layers' table allows")
                continue
            kind, only = KEYS[key]
            ok = (
                (kind == "layer" and val in LAYERS)
                or (kind == "level" and val in LEVELS)
                or (kind == "role" and val in ROLES)
                or (kind == "checks" and val in CHECKS)
                or (kind == "true" and val is True)
                or (kind == "false" and val is False)
                or (kind == "str" and isinstance(val, str) and val)
                or (kind == "strlist" and isinstance(val, list) and all(isinstance(x, str) for x in val))
            )
            if not ok:
                r.add("§1", where, f"`{key} = {json.dumps(val)}` is not a value the table allows ({kind})")
            if only and c.layer != only:
                r.add("§1", where, f"`{key}` is for `layer = \"{only}\"` crates, this crate is `{c.layer}`")
        if c.layer == "kit" and c.level is None:
            r.add("K1", where, "a kit crate has no `level`")
        if c.layer == "app" and c.role is None:
            r.add("§1", where, "an application crate has no `role`")
        if c.layer == "tool" and "checks" not in c.md:
            pass  # a tool crate that holds tests needs `checks`: lint-test-headers (07 §4, check 3) holds it
        for name in D2_CRATES:
            if name == c.name and not c.flag("no_std"):
                r.add("D2", where, "a crate that is `no_std` on `main` does not say `no_std = true`")
        if c.name in WORDING_CRATES and not c.flag("wording"):
            r.add("§2.2", where, "a wording crate does not say `wording = true`")
        if c.name in BOUNDARY_CRATES and not c.flag("boundary"):
            r.add("§2.3", where, "a key-boundary crate does not say `boundary = true`")
        for dep, kind, rename in c.deps:
            if rename:
                r.add("09 §1.2", f"{c.rel} -> {dep}", f"the workspace dependency is renamed `{rename}`; none is, so the module rule reads exact paths")
        if c.flag("online") and c.layer in ("port", "runtime"):
            r.add("S8", where, f"a `{c.layer}` crate is marked `online`")


# ---------------------------------------------------------------------------------------------
# 01 §1 to §8: the edges.


def _klass(c):
    if c.layer == "app":
        return c.role or "app"
    return c.layer


def _kit_ok_for_page(b):
    """K6: a feature's kit edges are the templates and organisms."""
    return b.level in ("organisms", "templates")


def edge_refusals(a, b, kind, crates):
    """[(rule, why)] for one edge of workspace crate `a` onto `b`. `kind` is normal, build or dev."""
    out = []
    dev = kind == "dev"
    la, lb = a.layer, b.layer
    ka, kb = _klass(a), _klass(b)

    # Rules that hold for every kind of edge.
    if ka == "feature" and kb == "feature":
        out.append(("A5", "a feature crate never depends on another feature crate"))
    if kb == "feature" and la not in ("app", "tool") or (kb == "feature" and la == "app" and ka not in ("composite", "product")):
        out.append(("A6", f"only a composite or a product depends on a feature; this is {ka}"))
    if kb == "product" and la not in ("root", "tool"):
        out.append(("A6", f"only a root or a tool depends on a product; this is {ka}"))
    if b.flag("owner") and not (a.flag("owner") or ka == "product" or la == "tool"):
        out.append(("A8", "an `owner` crate is named only by another owner, a product or a tool"))
    if b.flag("process"):
        out.append(("S2", "nothing depends on a disk or grant process: the root's adapters speak to it over a pipe"))
    if a.flag("process") and lb not in ("domain", "tool"):
        out.append(("S2", f"a `process` adapter depends on the domain only, not on a `{lb}` crate"))
    if la == "root" and lb == "root":
        out.append(("S3", "a root never depends on another root"))
    if la == "tool" and lb == "root":
        out.append(("T1", "a tool never depends on a root"))
    if lb == "tool" and la != "tool" and not dev and not (la == "root" and not a.ships):
        out.append(("T1", "nothing that ships depends on a tool except as a dev-dependency; only the tour root (`ships = false`) names one normally"))
    if a.flag("wording") and not dev and not (b.flag("wording") or b.name in WORDING_MAY_DEPEND):
        out.append(("§2.2", "a wording crate depends only on faraday-text, faraday-crypto, faraday-bip and the other wording crate"))
    if a.flag("boundary") and b.name in BOUNDARY_NEVER:
        out.append(("§2.3", f"a boundary crate depends on no input parser and not on the registry ({b.name})"))
    if a.layer == "port" and not dev and b.name not in PORT_MAY_DEPEND:
        out.append(("P5", "the port depends on faraday-text and faraday-crypto among the workspace's crates and nothing else"))

    # The table of §1 (and the sections' rows), for normal and build edges; dev edges (D4, T2) may
    # also name test support (a tool) and a crate of the crate's own layer.
    why = _table(a, b, ka, kb, dev)
    if why and not (dev and (lb == "tool" or (la == lb and not out))):
        out.append(why)
    return out


def _table(a, b, ka, kb, dev):
    """The table of 01 §1 for a normal edge: None if allowed, else (rule, why)."""
    la, lb = a.layer, b.layer
    if la == "tool":
        return None
    if la == "domain":
        if lb == "domain":
            return None
        if lb == "kit" and b.level == "tokens":
            return None
        if lb == "kit":
            return ("§6.3", "the tokens crate is the one kit crate a domain crate may depend on")
        return ("D1", f"a domain crate depends on domain crates only, not on a `{lb}` crate")
    if la == "port":
        return None if lb == "domain" else ("P2", f"the port depends on domain crates only, not on a `{lb}` crate")
    if la == "kit":
        if lb in ("domain", "port"):
            return None
        if lb == "kit":
            if a.level == "tokens":
                return ("K2", "the tokens crate depends on nothing")
            ra, rb = a.level_rank(), b.level_rank()
            if rb < ra:
                return None
            if rb == ra and (a.name, b.name) in SAME_LEVEL_EDGES:
                return None
            if rb == ra:
                return ("K1", f"a kit crate depends on its own level ({a.level}) only where the manifest names the edge")
            return ("K1", f"a {a.level} crate depends on lower levels, not on {b.level}")
        return ("K3", f"no kit crate names an app type: an edge onto a `{lb}` crate")
    if la == "app":
        if lb in ("domain", "port"):
            return None
        if lb in ("adapter", "root"):
            return ("P3", f"no application crate depends on an adapter or a root ({lb})")
        if lb == "runtime":
            return None if ka == "product" else ("§1", "only a product crate depends on the runtime")
        if lb == "kit":
            if ka in ("kernel", "part"):
                return ("K3" if ka == "kernel" else "A2", f"a {ka} crate names no kit crate")
            if ka == "present" or ka in ("feature", "composite", "product"):
                return None if _kit_ok_for_page(b) else ("K6", f"a {ka} crate depends on the kit's templates and organisms only, not on `{b.level}`")
            return None
        # lb == app
        if ka == "kernel":
            return None if kb == "kernel" else ("A1", f"the kernel depends on no {kb} crate")
        if ka == "present":
            if kb == "kernel":
                return None
            if kb == "part" and b.name == PASS_FIELD_CRATE:
                return None
            return ("§1", f"the presentation kernel depends on the kernel and the passphrase part, not on a {kb} crate")
        if ka == "part":
            return None if kb in ("kernel", "part") else ("A2", f"a part depends on the kernel and other parts, not on a {kb} crate")
        if ka == "feature":
            return None if kb in ("kernel", "present", "part") else ("A4", f"a feature depends on the kernel, the presentation kernel and parts, not on a {kb} crate")
        if ka == "composite":
            return None if kb in ("kernel", "present", "part", "feature", "composite") else ("A6", f"a composite depends on its children, parts and the kernels, not on a {kb} crate")
        if ka == "product":
            return None if kb in ("kernel", "present", "part", "feature", "composite") else ("§1", f"a product depends on composites, features, parts and the kernels, not on a {kb} crate")
        return None
    if la == "runtime":
        if lb in ("domain", "port"):
            return None
        if lb == "kit":
            return None if b.level in ("tokens", "atoms") else ("R1", f"the runtime names the kit's atoms (`Ui`) and nothing above, not `{b.level}`")
        if lb == "app":
            return None if kb == "kernel" else ("R1", f"the runtime depends on the kernel only, not on a {kb} crate (no feature, no presentation kernel, no product)")
        return ("R1", f"the runtime does not depend on a `{lb}` crate")
    if la == "adapter":
        if lb in ("domain", "port", "adapter"):
            return None
        if lb == "kit":
            return None if b.level == "tokens" else ("§6.3", "an adapter may name the kit's tokens and nothing above")
        return ("P3", f"no adapter depends on the application, the runtime or a root ({lb})")
    if la == "root":
        if dev:
            return None if lb != "root" else ("S3", "a root never depends on another root")
        if lb in ("domain", "kit") and not a.ships:
            return None
        if not a.ships:
            return None if lb != "root" else ("S3", "a root never depends on another root")
        if lb == "port" or lb == "adapter" or lb == "runtime":
            return None
        if kb == "product":
            return None
        return ("S5", f"a shipping root depends on its product crate, the runtime, the port and its adapters, not on a `{lb}` crate ({b.name})")
    return None


def check_edges(crates, r):
    for a in crates.values():
        if not a.layer:
            continue
        for dep, kind, _ in a.deps:
            b = crates[dep]
            if not b.layer:
                continue
            for rule, why in edge_refusals(a, b, kind, crates):
                r.add(rule, f"{a.rel} -> {b.rel} ({kind})", why)


def closure(crates, start, kinds=("normal", "build")):
    """Names reachable from `start` over the given kinds of edge, `start` excluded."""
    seen, stack = set(), [start]
    while stack:
        n = stack.pop()
        for dep, kind, _ in crates[n].deps:
            if kind in kinds and dep not in seen:
                seen.add(dep)
                stack.append(dep)
    return seen


def check_graph(crates, r):
    # Cycles, over normal and build edges (a dev cycle is allowed by D4 and T1).
    index, low, on, stack, sccs, counter = {}, {}, set(), [], [], [0]

    def strong(v):
        index[v] = low[v] = counter[0]
        counter[0] += 1
        stack.append(v)
        on.add(v)
        for w, kind, _ in crates[v].deps:
            if kind == "dev":
                continue
            if w not in index:
                strong(w)
                low[v] = min(low[v], low[w])
            elif w in on:
                low[v] = min(low[v], index[w])
        if low[v] == index[v]:
            comp = []
            while True:
                w = stack.pop()
                on.discard(w)
                comp.append(w)
                if w == v:
                    break
            if len(comp) > 1:
                sccs.append(sorted(comp))

    sys.setrecursionlimit(10000)
    for v in sorted(crates):
        if v not in index:
            strong(v)
    for comp in sccs:
        layers = {crates[n].layer for n in comp}
        roles = {crates[n].role for n in comp}
        rule = "A2" if roles == {"part"} else "S1" if layers == {"adapter"} else "K1" if layers == {"kit"} else "§1"
        r.add(rule, ", ".join(comp), "these crates depend on each other in a cycle")

    roots = [c for c in crates.values() if c.layer == "root"]
    # T1 and S8 over each root's resolved graph; S5's one product; S7 across roots.
    products_of = {}
    for root in roots:
        graph = closure(crates, root.name)
        if root.ships:
            for n in sorted(graph):
                if crates[n].layer == "tool":
                    r.add("T1", f"{root.rel} -> {crates[n].rel}", "a root that ships has a tool in its graph")
            prods = [d for d, k, _ in root.deps if k == "normal" and crates[d].role == "product"]
            if len(prods) != 1:
                r.add("S5", root.rel, f"a root that ships depends on exactly one product crate, not {len(prods)}")
            products_of[root.name] = (set(prods) | {root.name})
        if root.flag("offline"):
            for n in sorted(graph):
                if crates[n].flag("online"):
                    r.add("S8", f"{root.rel} -> {crates[n].rel}", "an `online` crate is in the graph of the device root (`offline = true`)")
    for root in roots:
        if not root.ships:
            continue
        graph = closure(crates, root.name)
        for other, held in products_of.items():
            if other != root.name and graph & held:
                n = sorted(graph & held)[0]
                r.add("S7", f"{root.rel} -> {crates[n].rel}", "a product's root depends on a crate of another product's product or root")

    # §2.3: a boundary crate depends on no crate that depends on an input parser or the registry.
    for c in crates.values():
        if c.flag("boundary"):
            for n in sorted(closure(crates, c.name)):
                if n in BOUNDARY_NEVER:
                    r.add("§2.3", f"{c.rel} -> {crates[n].rel}", "a boundary crate depends on an input parser or the registry through its dependencies")
    # S10: the device composite names no product and no online crate.
    dev = crates.get(DEVICE_COMPOSITE)
    if dev is not None:
        for n in sorted(closure(crates, dev.name)):
            m = crates[n]
            if m.flag("online") or m.role == "product" or m.name.startswith("faraday-starter"):
                r.add("S10", f"{dev.rel} -> {m.rel}", "the device composite holds no Transfer, no strip, no Starter feature and no product")
    # A8 / T3 / D4 over dev edges are in edge_refusals.


# ---------------------------------------------------------------------------------------------
# The Rust source of each crate: U1, D2, P22, and the module rule.


def target_roots(c):
    out = []
    for t in c.pkg.get("targets", []):
        kinds = set(t.get("kind", []))
        if kinds & {"lib", "proc-macro", "rlib", "cdylib", "staticlib"}:
            out.append(Path(t["src_path"]))
    if not out:
        for t in c.pkg.get("targets", []):
            if "bin" in t.get("kind", []):
                out.append(Path(t["src_path"]))
                break
    return out


def read_toks(path, docs=False):
    try:
        return lex.tokenize(path.read_text(encoding="utf-8", errors="replace"), docs)
    except OSError:
        return []


def source_files(c, tests=False):
    dirs = [c.dir / "src"] + ([c.dir / "tests"] if tests else [])
    for d in dirs:
        if d.is_dir():
            yield from sorted(d.rglob("*.rs"))


def test_only_files(c):
    """Files behind a `#[cfg(test)] mod name;` of the crate's own source."""
    out = set()
    for f in source_files(c):
        toks = read_toks(f)
        base = f.parent if f.name in ("lib.rs", "main.rs", "mod.rs") else f.parent / f.stem
        for i in range(len(toks) - 2):
            if toks[i].text == "#" and i + 1 < len(toks) and toks[i + 1].text == "[":
                close = lex._match_close(toks, i + 1)
                if lex._is_test_cfg(toks[i + 2 : close]):
                    j = close + 1
                    while j < len(toks) and toks[j].text in ("pub", "(", ")", "crate"):
                        j += 1
                    if j + 2 < len(toks) and toks[j].text == "mod" and toks[j + 2].text == ";":
                        out.add(base / f"{toks[j + 1].text}.rs")
                        out.add(base / toks[j + 1].text / "mod.rs")
    return out


def check_manifests(crates, root, r):
    rootman = Path(root) / "Cargo.toml"
    try:
        top = tomllib.loads(rootman.read_text())
    except (OSError, tomllib.TOMLDecodeError):
        top = None
    if top is not None:
        forbid = (((top.get("workspace") or {}).get("lints") or {}).get("rust") or {}).get("unsafe_code")
        if forbid != "forbid":
            r.add("U1", "Cargo.toml", '`[workspace.lints.rust] unsafe_code` is not "forbid"')
    for name in UNSAFE_CRATES:
        c = crates.get(name)
        if c is not None and c.layer in ("app", "kit", "port", "runtime"):
            r.add("U2", c.rel, "no crate of the application, the kit, the port or the runtime is on the unsafe list")
    for c in crates.values():
        try:
            man = tomllib.loads(c.manifest.read_text())
        except (OSError, tomllib.TOMLDecodeError) as e:
            r.add("U1", c.rel, f"the manifest cannot be read ({e})")
            continue
        lints = man.get("lints") or {}
        inherits = lints.get("workspace") is True
        own = (lints.get("rust") or {}).get("unsafe_code")
        if c.name in UNSAFE_CRATES:
            if own != "deny":
                r.add("U2", c.rel, 'a crate on the unsafe list sets `unsafe_code = "deny"` in its own `[lints.rust]`')
            if inherits:
                r.add("U2", c.rel, "a crate on the unsafe list does not inherit the workspace lints")
        elif not inherits:
            r.add("U1", c.rel, "the crate does not inherit the workspace lints (`[lints] workspace = true`) and is not on the unsafe list")


def _attrs_inner(toks):
    """Inner attributes `#![...]` of a file: a list of token lists."""
    out = []
    for i in range(len(toks) - 2):
        if toks[i].text == "#" and toks[i + 1].text == "!" and toks[i + 2].text == "[":
            close = lex._match_close(toks, i + 2)
            out.append(toks[i + 3 : close])
    return out


def check_roots(crates, r):
    for c in crates.values():
        for f in target_roots(c):
            toks = read_toks(f)
            attrs = _attrs_inner(toks)
            where = f"{c.rel}/{_rel(f, c.dir)}"
            if c.flag("no_std") and not any(any(t.text == "no_std" for t in a) for a in attrs):
                r.add("D2", where, "the crate says `no_std = true` and its root has no `#![no_std]`")
            if c.layer in ("port", "app", "runtime"):
                want = {"unwrap_used", "expect_used", "panic"}
                ok = any(
                    a and a[0].text == "cfg_attr"
                    and [t.text for t in a[:7]] == ["cfg_attr", "(", "not", "(", "test", ")", ","]
                    and any(t.text == "deny" for t in a)
                    and want <= {t.text for t in a}
                    for a in attrs
                )
                if not ok:
                    r.add("P22", where, "the crate root lacks `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic))]`")


def module_of(c, f):
    """The first module under `src/` that holds the file ('' for the crate root)."""
    try:
        rel = f.relative_to(c.dir / "src")
    except ValueError:
        return None
    parts = rel.parts
    if len(parts) == 1:
        stem = Path(parts[0]).stem
        return "" if stem in ("lib", "main") else stem
    return parts[0]


def names(path, crate_ident, seq, globbed=()):
    """Does the resolved `path` name `seq` in crate `crate_ident` (through the crate's path, or bare
    where the file globs from that crate)?"""
    if path and path[0] == crate_ident and lex.has_seq(path[1:], seq):
        return True
    return crate_ident in globbed and path[: len(seq)] == tuple(seq)


class SourceFile:
    def __init__(self, c, f, keep_tests=False):
        self.c, self.f = c, f
        toks = read_toks(f)
        if not keep_tests:
            toks = lex.drop_test_items(toks)
        self.toks = toks
        self.uses, self.rest = lex.split_uses(toks)
        self.amap = lex.alias_map(self.uses)
        self.globbed = {g[0] for g in lex.globs(self.uses) if g}
        self.named = [(tuple(u.path), u.line) for u in self.uses]
        self.named += [(lex.resolve(p, self.amap), line) for p, line in lex.paths(self.rest)]
        self.where = f"{c.rel}/{_rel(f, c.dir)}"

    def hits(self, crate_ident, seq):
        return [line for p, line in self.named if names(p, crate_ident, seq, self.globbed)]

    def hits_pred(self, pred):
        return [(p, line) for p, line in self.named if pred(p)]


def check_sources(crates, root, r):
    port = next((c for c in crates.values() if c.layer == "port"), None)
    values_mod = port.md.get("values") if port else None
    kit_low = {c.ident for c in crates.values() if c.layer == "kit" and c.level in ("tokens", "atoms", "molecules")}
    kit_idents = {c.ident: c for c in crates.values() if c.layer == "kit"}
    feature_like = {c.ident for c in crates.values() if c.role in ("feature", "composite", "product")}

    for c in sorted(crates.values(), key=lambda x: x.name):
        skip = test_only_files(c)
        files = [f for f in source_files(c) if f not in skip]
        for f in files:
            sf = SourceFile(c, f)
            module = module_of(c, f)
            _module_rule(c, sf, module, crates, kit_low, kit_idents, r)
            _wherever_rules(c, sf, crates, r, port, values_mod)
        _owner_rules(c, [SourceFile(c, f) for f in files], r)
        if c.layer == "domain":
            for f in files:
                _io_rule(c, SourceFile(c, f), r)
        _a9(c, crates, feature_like, r)


IO_NAMES = (
    ("std", "fs"), ("std", "net"), ("std", "process"), ("std", "env"),
    ("std", "thread", "spawn"), ("std", "time", "SystemTime", "now"), ("std", "time", "Instant", "now"),
)  # fmt: skip


def _io_rule(c, sf, r):
    """D1: the domain's library code names no I/O."""
    for p, line in sf.named:
        for name in IO_NAMES:
            if p[: len(name)] == name:
                r.add("D1", f"{sf.where}:{line}", f"a domain crate's library code names `{'::'.join(name)}`")


def _wherever_rules(c, sf, crates, r, port, values_mod):
    # V7: no `pub use` of the wording crate outside it.
    if c.ident != "faraday_strings":
        for u in sf.uses:
            if u.vis and u.path and u.path[0] == "faraday_strings":
                r.add("V7", f"{sf.where}:{u.line}", f"`{u.vis} use {'::'.join(u.path)}` re-exports the wording crate")
    # K7: a kit crate's `pub use` of a lower level is listed in its `reexports`.
    if c.layer == "kit":
        listed = set(c.md.get("reexports") or [])
        for u in sf.uses:
            if not u.vis or not u.path:
                continue
            tgt = next((k for k in crates.values() if k.layer == "kit" and k.ident == u.path[0]), None)
            if tgt is not None and tgt.level_rank() < c.level_rank():
                text = "::".join(u.path)
                if text not in listed:
                    r.add("K7", f"{sf.where}:{u.line}", f"`pub use {text}` re-exports a lower level and is not in this crate's `reexports`")
        # P4: a kit crate names only the port's values module.
        if port is not None:
            for p, line in sf.named:
                if p and p[0] == "faraday_port" and (len(p) < 2 or p[1] != values_mod):
                    r.add("P4", f"{sf.where}:{line}", f"a kit crate names `{'::'.join(p)}`; only `faraday_port::{values_mod}` is the kit's")
    # R6, A24, vaults::Close: names only some crates may use.
    if c.layer == "app":
        for seq, what in ((("Order", "Exit"), "Order::Exit"), (("Order", "Clean"), "Order::Clean"), (("Ask", "Keep"), "Ask::Keep"), (("Ask", "Forget"), "Ask::Forget")):
            for line in sf.hits("faraday_port", seq):
                r.add("R6", f"{sf.where}:{line}", f"an application crate names `{what}`; only the runtime does (S11 for Exit)")
        for p, line in sf.named:
            if p and p[0] == "faraday_port" and p[-1] == "Exit" and "Order" not in p:
                r.add("R6", f"{sf.where}:{line}", "an application crate names `port::Exit`; only the runtime does")
        if c.role != "kernel":
            if not c.flag("owner"):
                for line in sf.hits("faraday_kernel", ("Top", "Shared")):
                    r.add("R6", f"{sf.where}:{line}", "only an owner's crate names `Top::Shared`")
                for line in sf.hits("faraday_kernel", ("Fx", "Done")):
                    r.add("A24", f"{sf.where}:{line}", "only a crate with `owner = true` names `Fx::Done`")
                for line in sf.hits("faraday_kernel", ("Effects", "done")):
                    r.add("A24", f"{sf.where}:{line}", "only a crate with `owner = true` names `Effects::done`")
                for i, t in enumerate(sf.toks[:-2]):
                    if t.kind == "p" and t.text == "." and sf.toks[i + 1].text == "done" and sf.toks[i + 2].text == "(":
                        r.add("A24", f"{sf.where}:{sf.toks[i + 1].line}", "only a crate with `owner = true` calls `Effects::done`")
            if c.name != "faraday-vaults":
                for line in sf.hits("faraday_kernel", ("vaults", "Close")):
                    r.add("A22", f"{sf.where}:{line}", "no application crate but the vault owner's names `vaults::Close`")
    # S6: a root builds no port Event.
    if c.layer == "root":
        for p, line in sf.named:
            if p and p[0] == "faraday_port" and "Event" in p[1:-1]:
                r.add("S6", f"{sf.where}:{line}", f"a root crate builds or matches `{'::'.join(p)}`; every event comes from an adapter's `poll`")


def _module_rule(c, sf, module, crates, kit_low, kit_idents, r):
    if c.layer != "app" or c.role not in ("feature", "composite", "product") or module not in MODULES:
        return
    where = sf.where
    for p, line in sf.named:
        if p and p[0] in ("crate", "super") and len(p) > 1:
            segs = [s for s in p if s not in ("crate", "super", "self")]
            if segs and segs[0] in SIBLINGS_NEVER[module]:
                r.add("V1" if module == "page" else "§5", f"{where}:{line}", f"the `{module}` module names `{segs[0]}` (`{'::'.join(p)}`)")

    def refuse(rule, line, what):
        r.add(rule, f"{where}:{line}", f"the `{module}` module names {what}")

    if module in ("state", "msg", "update"):
        for p, line in sf.named:
            if p and p[0] == "faraday_strings":
                refuse("A30", line, "`faraday_strings`")
            elif p and p[0] in kit_idents:
                refuse("§5", line, f"the kit crate `{p[0]}`")
        for line in sf.hits("faraday_kernel", ("Show",)):
            refuse("A30", line, "`Show`")
        for line in sf.hits("faraday_kernel", ("Memo",)):
            refuse("§5", line, "`Memo`")
        for line in sf.hits("faraday_kernel", ("Worded",)):
            refuse("A30", line, "`Worded` (an update forms no text)")
    if module in ("model", "present"):
        for p, line in sf.named:
            if p and (p[0] in kit_low or p[0] == "faraday_tokens"):
                refuse("V9" if p[0] in ("faraday_draw", "faraday_tokens") else "§5", line, f"the kit's drawing or tokens (`{'::'.join(p)}`)")
            if p and p[0] == "faraday_draw" and lex.has_seq(p, ("geom",)):
                refuse("V9", line, "`faraday_draw::geom`")
            if p and p[-1] == "Ui" and (p[0] in kit_idents or len(p) == 1):
                refuse("§5", line, "`Ui`")
    if module == "present":
        for p, line in sf.named:
            if p and p[-1] == "Compute" and (p[0] == "faraday_crypto" or len(p) == 1):
                refuse("V2", line, "`Compute`")
    if module == "page":
        for p, line in sf.named:
            if p and p[0] == "faraday_strings":
                refuse("V4", line, "`faraday_strings`")
            elif p and p[0] in kit_low:
                refuse("K6", line, f"the kit below organisms (`{'::'.join(p)}`)")
        for item in ("View", "Show", "Ctx"):
            for line in sf.hits("faraday_kernel", (item,)):
                refuse("V1", line, f"`{item}`")
    if module in ("model", "present", "page") and c.name != PASS_FIELD_CRATE:
        for p, line in sf.named:
            if p and p[0] == "faraday_pass_field" and len(p) > 1 and p[1] in ("State", "update"):
                refuse("A3", line, "the passphrase field's state; only its read view")


def _a9(c, crates, feature_like, r):
    """A9: outside the kernel, only a feature's own crate names its `update`, tests included."""
    if c.role == "kernel":
        return
    for f in source_files(c, tests=True):
        sf = SourceFile(c, f, keep_tests=True)
        for p, line in sf.named:
            if len(p) > 1 and p[0] in feature_like and p[0] != c.ident and p[1] == "update":
                r.add("A9", f"{sf.where}:{line}", f"`{'::'.join(p)}`: only {p[0].replace('_', '-')} names its own `update`")


def _impl_rows(toks):
    """(trait name, type name, line) for each `impl Trait for Type` in a token list."""
    out = []
    for i, t in enumerate(toks):
        if t.kind == "id" and t.text == "impl":
            j, depth, trait, saw_for = i + 1, 0, None, False
            if j < len(toks) and toks[j].text == "<":  # the impl's own generics
                d = 0
                while j < len(toks):
                    d += toks[j].text == "<"
                    d -= toks[j].text == ">"
                    j += 1
                    if d == 0:
                        break
            while j < len(toks) and toks[j].text not in ("{", ";"):
                tj = toks[j]
                if tj.text == "<":
                    depth += 1
                elif tj.text == ">":
                    depth -= 1
                elif tj.kind == "id" and tj.text == "for" and depth == 0:
                    saw_for = True
                    k = j + 1
                    while k < len(toks) and toks[k].text in ("&", "mut", "crate", "::", "self", "super"):
                        k += 1
                    if k < len(toks) and toks[k].kind == "id":
                        m = k
                        while m + 2 < len(toks) and toks[m + 1].text == "::" and toks[m + 2].kind == "id":
                            m += 2
                        out.append((trait, toks[m].text, t.line))
                    break
                elif tj.kind == "id" and depth == 0 and tj.text not in ("unsafe", "where"):
                    trait = tj.text
                j += 1
            _ = saw_for
    return out


def _owner_rules(c, sfs, r):
    """V12 and A7, over a crate's library code."""
    if c.layer != "app":
        return
    rows = []
    for sf in sfs:
        rows += [(sf, *row) for row in _impl_rows(sf.toks)]
    flows = {ty for _, tr, ty, _ in rows if tr == "StepFlow"}
    for sf, tr, ty, line in rows:
        if tr in ("Present", "Page") and ty in flows:
            r.add("V12", f"{sf.where}:{line}", f"`impl {tr} for {ty}`, a type that implements `StepFlow`: a flow writes only `ShowFlow`")
    if c.role == "kernel":
        return
    locked = {ty for _, tr, ty, _ in rows if tr == "AtLock"}
    for sf in sfs:
        toks = sf.toks
        for i, t in enumerate(toks):
            if t.kind == "p" and t.text == "#" and i + 1 < len(toks) and toks[i + 1].text == "[":
                close = lex._match_close(toks, i + 1)
                attr = toks[i + 2 : close]
                if attr and attr[0].text == "derive" and any(a.text == "AtLock" for a in attr):
                    j = close + 1
                    while j < len(toks) and toks[j].text == "#":
                        j = lex._match_close(toks, j + 1) + 1
                    while j < len(toks) and toks[j].text in ("pub", "(", ")", "crate"):
                        j += 1
                    if j + 1 < len(toks) and toks[j].text in ("struct", "enum"):
                        locked.add(toks[j + 1].text)
    for sf in sfs:
        toks = sf.toks
        for i, t in enumerate(toks):
            if t.kind == "id" and t.text == "struct" and i + 1 < len(toks) and toks[i + 1].text in locked:
                j = i + 2
                while j < len(toks) and toks[j].text not in ("{", "(", ";"):
                    j += 1
                if j >= len(toks) or toks[j].text == ";":
                    continue
                close = lex._match_close(toks, j)
                depth = 0
                for k in range(j + 1, close):
                    tk = toks[k]
                    if tk.text in "([{" and tk.kind == "p":
                        depth += 1
                    elif tk.text in ")]}" and tk.kind == "p":
                        depth -= 1
                    elif depth == 0 and tk.kind == "id" and tk.text == "pub" and not (k + 1 < close and toks[k + 1].text == "("):
                        r.add("A7", f"{sf.where}:{tk.line}", f"`{toks[i + 1].text}` implements `AtLock` and has a `pub` field")


# ---------------------------------------------------------------------------------------------


def run(meta, root):
    r = Refusals()
    crates = load(meta, root)
    check_metadata(crates, r)
    check_edges(crates, r)
    check_graph(crates, r)
    check_manifests(crates, root, r)
    check_roots(crates, r)
    check_sources(crates, root, r)
    return crates, r.items


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
            print("lint-layers: cargo metadata failed", file=sys.stderr)
            return 2
        meta = json.loads(p.stdout)
    crates, items = run(meta, args.root)
    for rule, where, why in sorted(items, key=lambda x: (x[0], x[1])):
        print(f"{rule}: {where}: {why}")
    if items:
        print(f"lint-layers: {len(items)} refusals over {len(crates)} crates", file=sys.stderr)
        return 1
    print(f"lint-layers: {len(crates)} crates, every layer rule holds")
    return 0


if __name__ == "__main__":
    sys.exit(main())
