"""Tests of tools/migration.py: each rule of the mapping check, the tag rule, and each command's
reading, on a fixture repository made in a temporary directory (a tiny git repository with a tag)."""

import io
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fixture import load  # noqa: E402

mg = load("migration")

ENV = {**os.environ, "GIT_AUTHOR_NAME": "fixture", "GIT_AUTHOR_EMAIL": "fixture",
       "GIT_COMMITTER_NAME": "fixture", "GIT_COMMITTER_EMAIL": "fixture"}
HEAD = '[[module]]\nmain = "{}"\nfate = "{}"\n'


def git(root, *args, stdin=None):
    p = subprocess.run(["git", "-C", str(root), *args], input=stdin, capture_output=True, text=True, env=ENV)
    assert p.returncode == 0, p.stderr
    return p.stdout.strip()


class Repo:
    """A git repository whose commit holds `main_files`, tagged as the tool expects (or not), and
    whose working tree holds `next_files`. Built with plumbing, which runs no hooks."""

    def __init__(self, main_files, next_files=None, tag=True):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        git(self.root, "init", "-q")
        for path, text in main_files.items():
            blob = git(self.root, "hash-object", "-w", "--stdin", stdin=text)
            git(self.root, "update-index", "--add", "--cacheinfo", f"100644,{blob},{path}")
        tree = git(self.root, "write-tree")
        self.commit = git(self.root, "commit-tree", "--no-gpg-sign", tree, "-m", "main")
        if tag:
            git(self.root, "update-ref", f"refs/tags/{mg.TAG}", self.commit)
        self.put({"docs/migration/inventory/features/lock.md": "# Lock\n"})
        self.put(next_files or {})
        self.saved = mg.FROZEN
        mg.FROZEN = self.commit[:9]

    def put(self, files):
        for path, text in files.items():
            p = self.root / path
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(text)

    def mapping(self, text):
        self.put({mg.MAPPING: text})

    def close(self):
        mg.FROZEN = self.saved
        self._tmp.cleanup()


class Base(unittest.TestCase):
    def repo(self, *a, **kw):
        r = Repo(*a, **kw)
        self.addCleanup(r.close)
        return r

    def run_cmd(self, fn, r, *args):
        out = []
        code = fn(r.root, *args, out=out.append)
        return code, "\n".join(out)


MAIN = {"a/x.rs": "fn x() {}\n", "a/y.rs": "fn y() {}\n", "b/z.txt": "z\n"}


class Tag(Base):
    def test_missing_tag_is_refused_saying_so(self):
        """Invariant: A check that reads `main` fails, saying the tag is missing, when the tag is not in the clone.
        Regression: A clone without tags reads nothing and passes an empty mapping as whole.
        Owner: tool"""
        r = self.repo(MAIN, tag=False)
        r.mapping("")
        code, out = self.run_cmd(mg.check, r)
        self.assertEqual(code, 1)
        self.assertIn("MG1", out)
        self.assertIn("missing", out)

    def test_tag_on_another_commit_is_refused(self):
        """Invariant: A tag that does not peel to the frozen commit is refused, saying where it peels.
        Regression: A moved or mistyped tag makes every check read a tree that is not `main`'s.
        Owner: tool"""
        r = self.repo(MAIN)
        r.mapping("")
        mg.FROZEN = "0" * 9
        code, out = self.run_cmd(mg.check, r)
        self.assertEqual(code, 1)
        self.assertIn("peels to", out)

    def test_other_failures_still_reported_without_the_tag(self):
        """Invariant: A run without the tag still reports the failures that need no tree.
        Regression: The first failure ends the run and the rest are found one run at a time.
        Owner: tool"""
        r = self.repo(MAIN, tag=False)
        r.mapping(HEAD.format("a", "swap"))
        code, out = self.run_cmd(mg.check, r)
        self.assertIn("MG1", out)
        self.assertIn("fate", out)


class Check(Base):
    def test_whole_mapping_passes(self):
        """Invariant: A mapping in which every path has a row, and every row is well formed, passes.
        Regression: A rule of the check refuses a correct row and the map can never be made whole.
        Owner: tool"""
        r = self.repo(MAIN)
        r.mapping(HEAD.format("a", "copy") + 'next = ["core/a"]\n'
                  + HEAD.format("b", "drop") + 'why = "gone"\n')
        code, out = self.run_cmd(mg.check, r)
        self.assertEqual(code, 0, out)
        self.assertIn("3 of 3 paths", out)

    def test_path_without_a_row_is_listed(self):
        """Invariant: Every path of the tag falls under a row; one that does not is reported with its directory.
        Regression: A file of `main` is left out of the map and is never carried or dropped.
        Owner: tool"""
        r = self.repo(MAIN)
        r.mapping(HEAD.format("a", "copy") + 'next = ["core/a"]\n')
        code, out = self.run_cmd(mg.check, r)
        self.assertEqual(code, 1)
        self.assertIn("no row for 1 path under b/", out)
        self.assertIn("1 left", out)

    def test_more_specific_row_wins_inside_a_directory_row(self):
        """Invariant: A directory row covers every file under it that no more specific row names.
        Regression: A drop row inside a copied crate is reported as a second row for the same path.
        Owner: tool"""
        r = self.repo(MAIN)
        r.mapping(HEAD.format("a", "copy") + 'next = ["core/a"]\n'
                  + HEAD.format("a/y.rs", "drop") + 'why = "needs the app"\n'
                  + HEAD.format("b", "drop") + 'why = "gone"\n')
        code, out = self.run_cmd(mg.check, r)
        self.assertEqual(code, 0, out)

    def test_row_naming_a_path_absent_at_the_tag(self):
        """Invariant: A row that names a path the tag does not hold is refused.
        Regression: A mistyped path covers nothing and the file it meant stays unmapped unnoticed.
        Owner: tool"""
        r = self.repo(MAIN)
        r.mapping(HEAD.format("a/nope.rs", "drop") + 'why = "x"\n')
        _, out = self.run_cmd(mg.check, r)
        self.assertIn("a/nope.rs is not in the tree", out)

    def test_two_rows_for_one_path(self):
        """Invariant: No two rows name the same `main` path.
        Regression: A path with two fates is carried by whichever reader looks first.
        Owner: tool"""
        r = self.repo(MAIN)
        r.mapping(HEAD.format("b", "drop") + 'why = "x"\n' + HEAD.format("b", "drop") + 'why = "y"\n')
        _, out = self.run_cmd(mg.check, r)
        self.assertIn("is also row 1", out)

    def test_fate_must_be_one_of_four(self):
        """Invariant: A fate other than copy, port, rewrite or drop is refused.
        Regression: A fate such as `skip` is accepted and the row is never carried out.
        Owner: tool"""
        r = self.repo(MAIN)
        r.mapping(HEAD.format("b", "skip"))
        _, out = self.run_cmd(mg.check, r)
        self.assertIn("not one of copy, port, rewrite, drop", out)

    def test_port_needs_an_existing_slug_and_rewrite_a_slug_or_contract(self):
        """Invariant: A port row names an inventory slug that exists; a rewrite names a slug or a contract.
        Regression: A port row for a feature that has no inventory file is carried out from nothing.
        Owner: tool"""
        r = self.repo(MAIN)
        r.mapping(HEAD.format("a/x.rs", "port") + 'next = ["app/x.rs"]\nfeature = ["no-such"]\n'
                  + HEAD.format("a/y.rs", "rewrite") + 'next = ["app/y.rs"]\n'
                  + HEAD.format("b", "rewrite") + 'next = ["app/b.rs"]\ncontract = "05 §4"\n')
        _, out = self.run_cmd(mg.check, r)
        self.assertIn("slug 'no-such' does not exist", out)
        self.assertIn("row 2 (a/y.rs): a rewrite row names an inventory slug or a `contract`", out)
        self.assertNotIn("row 3", out)

    def test_drop_needs_why_and_no_next(self):
        """Invariant: A drop row says why and names no `next` path.
        Regression: A module is dropped with no word on what holds its rule now.
        Owner: tool"""
        r = self.repo(MAIN)
        r.mapping(HEAD.format("b", "drop") + 'next = ["x"]\n')
        _, out = self.run_cmd(mg.check, r)
        self.assertIn("says why", out)
        self.assertIn("no `next` path", out)

    def test_copy_has_exactly_one_next_path(self):
        """Invariant: A copy row has exactly one `next` path.
        Regression: A copy to two places leaves one of them a file nobody reviewed against `main`.
        Owner: tool"""
        r = self.repo(MAIN)
        r.mapping(HEAD.format("b", "copy") + 'next = ["x", "y"]\n')
        _, out = self.run_cmd(mg.check, r)
        self.assertIn("exactly one `next` path", out)

    def test_every_failure_in_one_run(self):
        """Invariant: One run reports every failure, not the first.
        Regression: A map with ten faults is mended one run at a time.
        Owner: tool"""
        r = self.repo(MAIN)
        r.mapping(HEAD.format("a/nope", "drop") + HEAD.format("b", "swap"))
        _, out = self.run_cmd(mg.check, r)
        self.assertGreaterEqual(out.count("FAIL"), 4, out)


class Done(Base):
    def test_copy_row_is_done_only_when_every_covered_file_is_there(self):
        """Invariant: A copied directory is done when each file of `main` it covers is at its moved path and holds no `todo!()`.
        Regression: A skeleton directory with a manifest alone counts as the copied crate.
        Owner: tool"""
        r = self.repo(MAIN, {"core/a/Cargo.toml": "", "core/a/x.rs": "fn x() {}\n"})
        r.mapping(HEAD.format("a", "copy") + 'next = ["core/a"]\n')
        _, out = self.run_cmd(mg.done, r)
        self.assertIn("0 of 1 rows done", out)
        r.put({"core/a/y.rs": "fn y() { todo!() }\n"})
        _, out = self.run_cmd(mg.done, r)
        self.assertIn("0 of 1 rows done, 1 todo!() left", out)
        r.put({"core/a/y.rs": "fn y() {}\n"})
        _, out = self.run_cmd(mg.done, r)
        self.assertIn("1 of 1 rows done, 0 todo!() left", out)

    def test_progress_by_inventory_file_and_crate(self):
        """Invariant: Progress is printed for each inventory file and each crate, with the `todo!()` left.
        Regression: A feature's rows are counted under no inventory file and its progress is unreadable.
        Owner: tool"""
        r = self.repo(MAIN, {"app/f/Cargo.toml": "", "app/f/lib.rs": "fn a() { todo!() }\nfn b() { todo!() }\n"})
        r.mapping(HEAD.format("a/x.rs", "port") + 'next = ["app/f/lib.rs"]\nfeature = ["lock"]\n')
        _, out = self.run_cmd(mg.done, r)
        self.assertRegex(out, r"lock\s+0 / 1\s+2")
        self.assertRegex(out, r"app/f\s+0 / 1\s+2")

    def test_drop_row_is_done_once_written(self):
        """Invariant: A drop row is done as soon as it is written.
        Regression: A drop row waits for a `next` path it does not have and the map never reads as done.
        Owner: tool"""
        r = self.repo(MAIN)
        r.mapping(HEAD.format("b", "drop") + 'why = "x"\n')
        _, out = self.run_cmd(mg.done, r)
        self.assertIn("1 of 1 rows done", out)


class Diff(Base):
    def setUp(self):
        self.rows = (HEAD.format("a", "copy") + 'next = ["core/a"]\n')

    def test_a_file_only_moved_shows_no_difference(self):
        """Invariant: A copied file that only moved, its directory prefix folded, shows no difference.
        Regression: Every path inside a moved file's text shows as changed and hides the real edits.
        Owner: tool"""
        r = self.repo({"a/x.rs": 'include_str!("a/y.rs");\n', "a/y.rs": "y\n"},
                      {"core/a/x.rs": 'include_str!("core/a/y.rs");\n', "core/a/y.rs": "y\n"})
        r.mapping(self.rows)
        code, out = self.run_cmd(mg.diff, r, "a/x.rs")
        self.assertEqual(code, 0)
        self.assertIn("1 the same, 0 differ", out)

    def test_an_edit_shows_as_a_difference(self):
        """Invariant: A copied file changed beyond its move shows the change.
        Regression: The diff folds too much and a real edit to a copied file passes review unseen.
        Owner: tool"""
        r = self.repo({"a/x.rs": "one\n"}, {"core/a/x.rs": "two\n"})
        r.mapping(self.rows)
        code, out = self.run_cmd(mg.diff, r, "a/x.rs")
        self.assertIn("-one", out)
        self.assertIn("+two", out)

    def test_a_missing_file_is_a_failure(self):
        """Invariant: A copied file absent from `next` is reported, not skipped.
        Regression: A directory diff of a half-copied crate prints nothing and reads as clean.
        Owner: tool"""
        r = self.repo({"a/x.rs": "one\n"}, {})
        r.mapping(self.rows)
        code, out = self.run_cmd(mg.diff, r, "a")
        self.assertEqual(code, 1)
        self.assertIn("not on next at core/a/x.rs", out)


class Parity(Base):
    FIXES = ("| hash | feature | what | rule |\n|---|---|---|---|\n"
             "| `abc123456` | [Lock](features/lock.md) | broke | rule |\n"
             "| `def123456` | [Lock](features/lock.md) | broke too | rule |\n")

    def setup_repo(self, parity=None, rust=""):
        r = self.repo(MAIN, {"app/f/Cargo.toml": "", "app/f/lib.rs": rust or "fn a() {}\n"})
        r.put({mg.FIXES: self.FIXES})
        r.mapping(HEAD.format("a/x.rs", "port") + 'next = ["app/f/lib.rs"]\nfeature = ["lock"]\n')
        if parity:
            r.put({mg.PARITY: parity})
        return r

    def test_unaccounted_fix_of_a_finished_feature_is_listed(self):
        """Invariant: A fix of a feature with no `todo!()` left that nothing accounts for is listed.
        Regression: A fixed bug of `main` comes back in a ported feature with nothing to catch it.
        Owner: tool"""
        r = self.setup_repo()
        code, out = self.run_cmd(mg.parity, r)
        self.assertEqual(code, 1)
        self.assertIn("abc123456", out)
        self.assertIn("def123456", out)

    def test_a_feature_with_todo_is_not_asked_yet(self):
        """Invariant: A feature whose crate still holds a `todo!()` is not asked for its fixes.
        Regression: Every fix of the skeleton is listed and the real gaps are lost among them.
        Owner: tool"""
        r = self.setup_repo(rust="fn a() { todo!() }\n")
        code, out = self.run_cmd(mg.parity, r)
        self.assertEqual(code, 0, out)

    def test_test_and_entry_account_and_orphan_entry_fails(self):
        """Invariant: A Regression line ending `(main <hash>)` or a `parity.toml` entry accounts for a fix; an entry naming no fix fails.
        Regression: A typed hash in `parity.toml` accounts for nothing and nobody notices.
        Owner: tool"""
        text = ('[[fix]]\ncommit = "abc123456"\nheld = "gone: no such field"\n'
                '[[fix]]\ncommit = "ffffffff0"\nheld = "lint: x"\n')
        r = self.setup_repo(parity=text, rust="/// Regression: it broke (main def123456)\nfn a() {}\n")
        code, out = self.run_cmd(mg.parity, r)
        self.assertEqual(code, 1)
        self.assertNotIn("fix abc123456", out)
        self.assertNotIn("fix def123456", out)
        self.assertIn("ffffffff0 is not a commit", out)


if __name__ == "__main__":
    unittest.main()
