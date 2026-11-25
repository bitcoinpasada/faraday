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


class PaneContainerTests(unittest.TestCase):
    """The rules of the pane container, on the lines they name and on near misses."""

    def hits(self, name, line):
        rule = next(r for r in lc.RULES if r.name == name)
        return any(p.search(line) for p in rule.patterns)

    def test_a_page_placing_a_pane_is_counted(self):
        for line in ("let x = room - LIST_PANE;", "let w = side_panel_w(cw);", "room - support_pane_room()"):
            self.assertTrue(self.hits("pane-own-place", line), line)
        self.assertFalse(self.hits("pane-own-place", "PANES.place(room, guide)"))

    def test_a_page_timing_its_change_is_counted(self):
        for line in ("let ms = s.motion_speed().ms();", "motion::STEP_FADE_MS", "Kind::of(s.motion_style())"):
            self.assertTrue(self.hits("page-own-time", line), line)
        for line in ("app.motion_speed(),", "let style = app.motion_style();", "motion::DROP_MS", "Kind::of(&path)"):
            self.assertFalse(self.hits("page-own-time", line), line)

    def test_the_container_is_not_looked_at(self):
        for name in ("pane-own-place", "page-own-time"):
            rule = next(r for r in lc.RULES if r.name == name)
            self.assertIn("faraday/faraday-core/src/screens/hub_layout/", rule.skip)


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


class PassFieldRuleTests(unittest.TestCase):
    RULE = next(r for r in lc.RULES if r.name == "pass-field-outside")

    def hit(self, line):
        return any(p.search(line) for p in self.RULE.patterns)

    def test_a_passphrase_field_outside_the_one_field_is_a_hit(self):
        for line in ["    pass: TextBox,", "    pub(crate) pass_box: SecretText,", "    again: TextBox,",
                     "    pub vault_passphrase: SecretText,", "    pass: crate::secret_text::SecretText,",
                     "    pub(crate) again: crate::vault::TextBox,"]:
            self.assertTrue(self.hit(line), line)

    def test_other_fields_and_values_are_not(self):
        for line in ["    name: TextBox,", "    passes: u32,", "    again: TextBox::default(),",
                     "    pass: String,", "    passphrase: Option<SecretText>,",
                     "    passphrase: Option<crate::secret_text::SecretText>,", "    let again: TextBox = x;"]:
            self.assertFalse(self.hit(line), line)

    def test_a_prompt_text_box_is_a_hit(self):
        self.assertTrue(self.hit("    pub text: TextBox,"))
        self.assertFalse(self.hit("    pub text: String,"))

    def hits(self, *src):
        return lc.hit_lines(self.RULE, "\n".join(src))

    def test_the_printable_ascii_key_filter_is_a_hit(self):
        self.assertEqual(self.hits("KeyIn::Char(c) if (' '..='~').contains(&c) => self.passphrase.push(c),"), [1])
        self.assertEqual(self.hits("Kind::Bip39 => (' '..='~').contains(&pass_char),"), [1])
        self.assertEqual(self.hits("KeyIn::Char(c) => {"), [])

    def test_a_filter_whose_arm_and_body_are_on_two_lines_is_a_hit(self):
        self.assertEqual(self.hits("KeyIn::Char(c) if (' '..='~').contains(&c) => {",
                                   "    self.type_pass_char(c);", "}"), [1])

    def test_the_filter_of_a_value_that_is_not_a_passphrase_is_not(self):
        self.assertEqual(self.hits("KeyIn::Char(ch) if (' '..='~').contains(&ch) => {", "    typed.push(ch);"), [])

    def test_a_parameter_of_a_function_is_not_a_field(self):
        src = ["pub struct J {", "    passphrase: SecretText,", "}", "fn job(", "    file: &File,",
               "    passphrase: SecretText,", ") -> Job {", "    let again: TextBox = x;", "}"]
        lines = list(enumerate(src, 1))
        self.assertEqual(sorted(lc.fn_signature_lines(lines)), [4, 5, 6, 7])
        self.assertEqual(sorted(lc.fn_signature_lines(list(enumerate(["fn a(x: u8) {", "    y,", "}"], 1)))), [1])


class FormListRuleTests(unittest.TestCase):
    RULE = next(r for r in lc.RULES if r.name == "form-list-outside")

    def hit(self, line):
        return any(p.search(line) for p in self.RULE.patterns)

    def test_a_list_of_forms_written_out_is_a_hit(self):
        for line in ["    for f in [Form::Words, Form::Codex32] {", "    forms: &[Form::Slip39, Form::Xor],",
                     "let l = [faraday_forms::Form::Krux, faraday_forms::Form::Words];"]:
            self.assertTrue(self.hit(line), line)

    def test_a_map_over_forms_and_other_lists_are_not(self):
        for line in ["            Form::Words | Form::Krux => None,", "    let l = [Form::Words];",
                     "    pub const ALL: [BackupForm; 2] = [BackupForm::Words, BackupForm::Codex32];",
                     "    for form in [Form::Short, Form::Number] {", "    [KeyForm::Xor, KeyForm::Krux]"]:
            self.assertFalse(self.hit(line), line)


class WayInRuleTests(unittest.TestCase):
    RULE = next(r for r in lc.RULES if r.name == "way-in-outside")

    def hit(self, line):
        return any(p.search(line) for p in self.RULE.patterns)

    def test_a_row_built_with_an_entry_action_is_a_hit(self):
        for line in ["            Action::RSlotScan,", "        _ => Action::ScanSeed,",
                     "            Action::EntryWordCount(n),", "    (words, Action::SignerCheck(SignerPress::SeedWords)),"]:
            self.assertTrue(self.hit(line), line)

    def test_a_handler_and_an_action_run_at_once_are_not(self):
        for line in ["            Action::ScanSeed => self.scan(),", "        Action::EntryForm(f) | Action::RSlotForm(f) => {",
                     "        self.act(Action::EntryForm(code));", "    app.press(Action::ScanSeed);",
                     "        self.act(crate::Action::ScanSeed);"]:
            self.assertFalse(self.hit(line), line)


class PickOwnRowsRuleTests(unittest.TestCase):
    RULE = next(r for r in lc.RULES if r.name == "pick-own-rows")

    def hit(self, line):
        return any(p.search(line) for p in self.RULE.patterns)

    def test_a_flow_listing_the_keys_or_wallets_is_a_hit(self):
        for line in ["        app.session.keys.all()", "    let session = &app.session; session.keys.all()",
                     "        .wallets.iter()", "    for (i, w) in app.session.wallets.iter().enumerate() {"]:
            self.assertTrue(self.hit(line), line)

    def test_a_listing_split_over_two_lines_is_a_hit(self):
        text = "fn f() {\n    let v = app\n        .session\n        .keys\n        .all();\n}\n"
        self.assertEqual(lc.hit_lines(self.RULE, text), [4])

    def test_the_pane_and_other_lists_are_not(self):
        for line in ["        .keys(app, |i, k| {", "    let n = app.session.wallets.len();",
                     "    app.inbox.iter()"]:
            self.assertFalse(self.hit(line), line)

    def test_the_session_pane_is_not_looked_in(self):
        self.assertIn("faraday/faraday-core/src/screens/session_pane/", self.RULE.skip)


if __name__ == "__main__":
    unittest.main()
