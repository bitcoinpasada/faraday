"""Tests of tools/clippy.py: the copies check of docs/spec/09-manifest.md section 1.4 on fixture
directories, and the live files, which must agree."""

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fixture import load  # noqa: E402

cl = load("clippy")

FLOW = ['    { path = "faraday_templates::flow::FlowModel::new", reason = "V12: filled once" },']
MINT = ['    { path = "faraday_crypto::work::Compute::new", reason = "a mint" },']
KEY = ['    { path = "faraday_keys::Held::release", reason = "key boundary" },']


def write(root, d, lines):
    p = Path(root) / d / "clippy.toml"
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text("disallowed-methods = [\n" + "\n".join(lines) + "\n]\n")


class Copies(unittest.TestCase):
    def tree(self, **per_dir):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        for d in cl.HOLDS:
            lines = MINT + KEY
            if "flow" in cl.HOLDS[d]:
                lines = lines + FLOW
            write(tmp.name, d, per_dir.get(d, lines))
        return Path(tmp.name)

    def test_live_files_agree(self):
        """Invariant: Every clippy.toml of the repository holds the sets section 1.4 gives it, with the entries of its copies.
        Regression: A new entry is added to one directory's file and not to its copies, so one crate builds what another refuses.
        Owner: tools"""
        self.assertEqual([], cl.copies_differ(Path(__file__).resolve().parents[2]))

    def test_a_missing_flow_entry_is_named(self):
        """Invariant: A directory that lacks an entry of a set its copies hold is refused, naming the entry.
        Regression: The owners' file lacks the step-flow builders, and an owner crate builds a FlowModel unchecked.
        Owner: tools"""
        root = self.tree(**{"app/owners": MINT + KEY})
        out = cl.copies_differ(root)
        self.assertTrue(any("app/owners" in p and "FlowModel::new" in p for p in out), out)

    def test_a_set_not_given_is_refused(self):
        """Invariant: A directory holding a set that section 1.4 does not give it is refused.
        Regression: The domain's file takes the flow set, and its tests cannot name the builders.
        Owner: tools"""
        root = self.tree(core=MINT + KEY + FLOW)
        self.assertTrue(any(p.startswith("core/") and "flow set" in p for p in cl.copies_differ(root)))


if __name__ == "__main__":
    unittest.main()
