"""Fixture workspaces for the tests of the lints of `tools/`: a small tree on disk and the
`cargo metadata --no-deps` document that describes it, in cargo's own key names, so the lints run
over it without cargo."""

import importlib.util
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent.parent


def load(name):
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), TOOLS / f"{name}.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


P22 = '#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic))]\n'


class Crate:
    def __init__(self, name, layer, deps=(), dev=(), build=(), files=None, lints="inherit", rename=None, **md):
        self.name, self.layer = name, layer
        self.deps, self.dev, self.build = list(deps), list(dev), list(build)
        self.files = dict(files or {})
        self.lints = lints
        self.rename = rename or {}
        self.md = {"layer": layer, **md}

    def lib(self):
        head = ""
        if self.md.get("no_std"):
            head += "#![no_std]\n"
        if self.layer in ("port", "app", "runtime"):
            head += P22
        return head + self.files.get("src/lib.rs", "")


def kit(name, level, **kw):
    return Crate(name, "kit", level=level, **kw)


def app(name, role, **kw):
    return Crate(name, "app", role=role, **kw)


def write(crates, root):
    """The tree and its metadata."""
    root = Path(root)
    (root / "Cargo.toml").write_text('[workspace.lints.rust]\nunsafe_code = "forbid"\n')
    by = {c.name: c for c in crates}
    packages = []
    for c in crates:
        d = root / c.layer / c.name
        (d / "src").mkdir(parents=True, exist_ok=True)
        lints = "[lints]\nworkspace = true\n" if c.lints == "inherit" else (c.lints or "")
        (d / "Cargo.toml").write_text(f'[package]\nname = "{c.name}"\nversion = "0.0.0"\n\n{lints}')
        files = dict(c.files)
        files["src/lib.rs"] = c.lib()
        for rel, text in files.items():
            (d / rel).parent.mkdir(parents=True, exist_ok=True)
            (d / rel).write_text(text)
        deps = []
        for kind, names in ((None, c.deps), ("dev", c.dev), ("build", c.build)):
            for n in names:
                deps.append({"name": n, "kind": kind, "rename": c.rename.get(n), "path": str(root / by[n].layer / n), "source": None})
        packages.append(
            {
                "id": f"{c.name} 0.0.0 (path+file://{d})",
                "name": c.name,
                "manifest_path": str(d / "Cargo.toml"),
                "dependencies": deps,
                "targets": [{"kind": ["lib"], "name": c.name, "src_path": str(d / "src/lib.rs")}],
                "metadata": {"faraday": c.md},
            }
        )
    return {"packages": packages, "workspace_members": [p["id"] for p in packages], "workspace_root": str(root)}


class Workspace:
    """A temporary workspace: `Workspace(crates)` is cleaned up by `close()`."""

    def __init__(self, crates):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = self.tmp.name
        self.meta = write(crates, self.root)

    def close(self):
        self.tmp.cleanup()
