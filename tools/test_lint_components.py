#!/usr/bin/env python3
"""Tests of tools/lint-components.py: a file over its count fails, one under it fails until lowered, a file
not listed allows none, a rule with no counts left is banned, comments and test code are not counted, and
`--write` only lowers.

    python3 -I -B tools/test_lint_components.py

Standard library only; the rule and the counts are stand-ins, nothing reads the tree.
"""
import importlib.util
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("lint_components", HERE / "lint-components.py")
lc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lc)

RULE = lc.Rule("bare", [r"\bBare\(\d+\)"], ["x/"], "the named part")


def check(counts, hits):
    return lc.check([RULE], counts, lambda r: hits)


class CountTests(unittest.TestCase):
    def test_held_count_passes(self):
        self.assertEqual(check({("bare", "x/a.rs"): 2}, {"x/a.rs": [1, 5]}), [])

    def test_over_fails_naming_the_part(self):
        out = check({("bare", "x/a.rs"): 1}, {"x/a.rs": [1, 5]})
        self.assertEqual(len(out), 1)
        self.assertIn("the named part", out[0])

    def test_under_fails_until_lowered(self):
        self.assertEqual(len(check({("bare", "x/a.rs"): 2}, {"x/a.rs": [1]})), 1)
        self.assertEqual(len(check({("bare", "x/a.rs"): 2}, {})), 1)

    def test_unlisted_file_allows_none(self):
        self.assertEqual(len(check({("bare", "x/a.rs"): 1}, {"x/a.rs": [1], "x/b.rs": [3]})), 1)

    def test_rule_without_counts_is_banned_with_file_and_line(self):
        out = check({}, {"x/b.rs": [7]})
        self.assertEqual(len(out), 1)
        self.assertIn("x/b.rs:7", out[0])

    def test_write_lowers_never_adds_or_raises(self):
        counts = {("bare", "x/a.rs"): 3, ("bare", "x/gone.rs"): 1}
        new = lc.lowered([RULE], counts, lambda r: {"x/a.rs": [1], "x/new.rs": [2], "x/b.rs": [1, 2, 3, 4]})
        self.assertEqual(new, {("bare", "x/a.rs"): 1})
        self.assertEqual(lc.lowered([RULE], {("bare", "x/a.rs"): 1}, lambda r: {"x/a.rs": [1, 2]}),
                         {("bare", "x/a.rs"): 1})


class SourceTests(unittest.TestCase):
    SRC = "\n".join([
        "fn a() { Bare(1); }",            # 1 counted
        "// Bare(2)",                      # 2 comment
        "#[cfg(test)]",                    # 3
        "mod tests {",                     # 4
        "    fn t() { Bare(3); }",         # 5 test code
        "}",                               # 6
        "fn b() { Bare(4); }",            # 7 counted
        "#[cfg(test)]",
        "use x::Bare;",
        "fn c() { Bare(5); }",            # 10 counted
    ])

    def lines(self, tests):
        return [n for n, l in lc.code_lines(self.SRC, tests) if RULE.patterns[0].search(l)]

    def test_comments_and_cfg_test_items_are_not_counted(self):
        self.assertEqual(self.lines(False), [1, 7, 10])

    def test_a_rule_may_count_tests_too(self):
        self.assertEqual(self.lines(True), [1, 5, 7, 10])


if __name__ == "__main__":
    unittest.main()
