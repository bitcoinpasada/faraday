"""Tests of tools/lint-test-headers.py: each of the eight checks of 07 §4 has a fixture tree that
breaks it, which the lint must refuse under that check's number, and the tree mended, which must pass."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fixture import Crate, Workspace, kit, load  # noqa: E402

lh = load("lint-test-headers")

GOOD = "/// Invariant: A vault ends locked.\n/// Regression: Create skips the unlock.\n/// Owner: domain\n#[test]\nfn a_new_vault_ends_locked() {}\n"


def header(inv="A vault ends locked.", reg="Create skips the unlock.", owner="domain", name="t", extra=""):
    return f"/// Invariant: {inv}\n/// Regression: {reg}\n/// Owner: {owner}\n{extra}#[test]\nfn {name}() {{}}\n"


class Base(unittest.TestCase):
    def fails(self, crates, copied=(), recipes=("lint-touch-targets",)):
        ws = Workspace(crates)
        self.addCleanup(ws.close)
        return lh.run(ws.meta, ws.root, copied=list(copied), recipe_names=set(recipes))

    def refused(self, check, crates, **kw):
        out = self.fails(crates, **kw)
        self.assertIn(check, [c for c, _, _ in out], f"check {check} not refused; got {out}")

    def clean(self, crates, **kw):
        self.assertEqual(self.fails(crates, **kw), [])


def dom(src, name="a", **kw):
    return Crate(name, "domain", files={"src/lib.rs": src}, **kw)


class Checks(Base):
    def test_check_1_header_present_and_in_order(self):
        """Invariant: A test whose doc comment does not open with Invariant, Regression and Owner in that order is refused.
        Regression: A test is added with no header, or with its lines swapped.
        Owner: tool"""
        self.refused(1, [dom("#[test]\nfn t() {}\n")])
        self.refused(1, [dom("/// Does a thing.\n#[test]\nfn t() {}\n")])
        self.refused(1, [dom("/// Regression: x\n/// Invariant: y\n/// Owner: domain\n#[test]\nfn t() {}\n")])
        self.refused(1, [dom("// Invariant: not a doc comment\n#[test]\nfn t() {}\n")])
        self.clean([dom(GOOD)])
        self.clean([dom(header(extra="#[allow(unused)]\n"))])
        self.clean([dom("/// Invariant: A vault ends locked.\n/// Regression: Create skips the unlock.\n/// Owner: domain\n/// More words.\n#[test]\nfn t() {}\n")])

    def test_check_1_covers_property_test_cases(self):
        """Invariant: A property-test case is held to the same header as a plain test.
        Regression: The lint looks only for the plain test attribute.
        Owner: tool"""
        self.refused(1, [dom("proptest! {\n#[test]\nfn t(x in 0..9) {}\n}\n")])
        self.refused(1, [dom("#[tokio::test]\nasync fn t() {}\n")])
        self.clean([dom("proptest! {\n" + header(name="t") + "}\n")])

    def test_check_2_owner_is_one_of_six(self):
        """Invariant: An Owner that is not one of the six layer names is refused.
        Regression: A test names the port, a feature or a typo as its owner.
        Owner: tool"""
        self.refused(2, [dom(header(owner="port"))])
        self.refused(2, [dom(header(owner="feature"))])
        self.refused(2, [dom(header(owner="domain, kit"))])

    def test_check_3_owner_is_the_crates_layer(self):
        """Invariant: A test's Owner is the layer of the crate it is in, or for a tool crate the layer its `checks` names.
        Regression: An update test is written in a kit crate under Owner: app.
        Owner: tool"""
        self.refused(3, [dom(header(owner="app"))])
        self.refused(3, [Crate("t", "tool", files={"src/lib.rs": header(owner="app")})])
        self.clean([Crate("t", "tool", checks="app", files={"src/lib.rs": header(owner="app")})])
        self.refused(3, [Crate("p", "port", files={"src/lib.rs": header(owner="domain")})])
        self.clean([kit("k", "atoms", files={"src/lib.rs": header(owner="kit")})])

    def test_check_4_invariant_named_once(self):
        """Invariant: An Invariant sentence is named once in the workspace, compared with case, spacing and the final stop folded.
        Regression: A test is copied from another and keeps its sentence.
        Owner: tool"""
        a = dom(header(name="one"))
        b = Crate("b", "domain", files={"src/lib.rs": header(inv="a  VAULT ends locked", reg="Other change.", name="two")})
        self.refused(4, [a, b])
        c = Crate("b", "domain", files={"src/lib.rs": header(inv="A vault ends unlocked.", name="two")})
        self.clean([a, c])
        self.refused(4, [dom(header(name="one") + header(name="two"))])

    def test_check_5_invariant_and_regression_say_something(self):
        """Invariant: An empty Invariant or Regression, or a Regression equal to its Invariant, is refused.
        Regression: A header is filled with a placeholder so that check 1 passes.
        Owner: tool"""
        self.refused(5, [dom(header(inv=""))])
        self.refused(5, [dom(header(reg=""))])
        self.refused(5, [dom(header(inv="Same words.", reg="same  words"))])

    def test_check_6_copied_list(self):
        """Invariant: A file on the copied list that has a header in every test, or that does not exist, is refused, and a headerless listed file is not held to checks 1 to 5.
        Regression: A change gives a copied file its headers and leaves it on the list.
        Owner: tool"""
        bare = dom("#[test]\nfn t() {}\n")
        self.clean([bare], copied=["domain/a/src/lib.rs"])
        self.refused(6, [dom(GOOD)], copied=["domain/a/src/lib.rs"])
        self.refused(6, [bare], copied=["domain/a/src/gone.rs"])
        self.refused(6, [dom("pub fn f() {}\n")], copied=["domain/a/src/lib.rs"])
        self.refused(1, [bare], copied=[])

    def test_check_7_tests_do_not_name_the_tokens(self):
        """Invariant: A test names no item of the kit's tokens crate, in a copied file as in a new one.
        Regression: A test asserts a token's value.
        Owner: tool"""
        tokens = kit("faraday-tokens", "tokens")
        direct = header().replace("fn t() {}", "fn t() { let _ = faraday_tokens::dims::GAP; }")
        self.refused(7, [tokens, dom(direct)])
        imported = "use faraday_tokens::dims::GAP as G;\n" + header().replace("fn t() {}", "fn t() { let _ = G; }")
        self.refused(7, [tokens, dom(imported)])
        self.refused(7, [tokens, dom("#[test]\nfn t() { faraday_tokens::x(); }\n")], copied=["domain/a/src/lib.rs"])
        self.clean([tokens, dom("use faraday_tokens::dims::GAP;\npub const X: u8 = 1;\n" + GOOD)])

    def test_check_8_ignored_tests_name_their_recipe(self):
        """Invariant: An ignored test carries a reason that names the recipe that runs it.
        Regression: A test is ignored with no reason and runs nowhere.
        Owner: tool"""
        self.refused(8, [dom(header(extra="#[ignore]\n"))])
        self.refused(8, [dom(header(extra='#[ignore = "slow"]\n'))])
        self.clean([dom(header(extra='#[ignore = "the ratchet: `just lint-touch-targets` runs it"]\n'))])


class Recipes(unittest.TestCase):
    def test_recipe_names_are_read_from_the_justfile(self):
        """Invariant: The recipes an ignore reason may name are the justfile's.
        Regression: A reason names a recipe that was renamed away.
        Owner: tool"""
        import tempfile

        with tempfile.TemporaryDirectory() as d:
            j = Path(d) / "justfile"
            j.write_text("x := 'a'\ndefault: check lint\n\nlint-touch-targets arg='1':\n    echo\n# comment: no\n")
            self.assertEqual(lh.recipes(j), {"default", "lint-touch-targets"})


if __name__ == "__main__":
    unittest.main()
