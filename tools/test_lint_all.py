#!/usr/bin/env python3
"""Tests of tools/lint-all.py, the runner of the lint's commands: every command runs to the end whatever
fails, the report lists the failures in the recipe's order however the commands finish, the exit code is 1
exactly when one failed, and commands that build in one cargo target folder never run at once.

    python3 -I -B tools/test_lint_all.py

Standard library only; the commands are stand-ins, nothing runs cargo.
"""
import contextlib
import importlib.util
import io
import sys
import threading
import time
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("lint_all", HERE / "lint-all.py")
lint_all = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lint_all)


def result(cmd, code):
    return cmd, code, [f"output of {cmd}"]


class RunnerTests(unittest.TestCase):
    def test_every_command_runs_though_one_fails(self):
        cmds = ["a", "b", "c", "d"]
        seen = []
        results = lint_all.run_all(cmds, lambda c: (seen.append(c), result(c, 1 if c == "a" else 0))[1], 4)
        self.assertEqual(sorted(seen), cmds)
        self.assertEqual(set(results), set(cmds))

    def report(self, cmds, codes):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = lint_all.report(cmds, {c: result(c, codes[c]) for c in cmds})
        return code, out.getvalue()

    def test_exit_status_follows_the_failures(self):
        self.assertEqual(self.report(["a", "b"], {"a": 0, "b": 0})[0], 0)
        self.assertEqual(self.report(["a", "b"], {"a": 0, "b": 2})[0], 1)

    def test_failures_print_in_the_recipes_order(self):
        cmds = ["z", "y", "x"]
        _, text = self.report(cmds, {"z": 1, "y": 0, "x": 1})
        self.assertLess(text.index("--- FAIL: z"), text.index("--- FAIL: x"))
        self.assertIn("2 failed", text)
        self.assertNotIn("--- FAIL: y", text)

    def test_order_of_report_does_not_depend_on_finishing_order(self):
        cmds = ["slow", "fast"]
        def runner(c):
            time.sleep(0.2 if c == "slow" else 0)
            return result(c, 1)
        results = lint_all.run_all(cmds, runner, 2)
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            lint_all.report(cmds, results)
        self.assertLess(out.getvalue().index("FAIL: slow"), out.getvalue().index("FAIL: fast"))

    def test_commands_run_side_by_side(self):
        cmds = [f"script{i}" for i in range(4)]
        barrier = threading.Barrier(4, timeout=5)
        def runner(c):
            barrier.wait()  # returns only when all four run at once; else the test fails
            return result(c, 0)
        self.assertEqual(len(lint_all.run_all(cmds, runner, 4)), 4)

    def test_builds_in_one_target_folder_never_overlap(self):
        cmds = ["cargo run -q -p costs-lint", "python3 tools/clippy.py --locked", "cargo nextest run",
                "cargo deny check", "tools/lint-tree.sh"]
        active, worst, order = [0], [0], []
        lock = threading.Lock()
        def runner(c):
            with lock:
                active[0] += c in cmds[:3]
                worst[0] = max(worst[0], active[0])
                order.append(c)
            time.sleep(0.05)
            with lock:
                active[0] -= c in cmds[:3]
            return result(c, 0)
        lint_all.run_all(cmds, runner, 8)
        self.assertEqual(worst[0], 1)
        self.assertEqual([c for c in order if c in cmds[:3]], cmds[:3])

    def test_other_manifests_are_their_own_folder(self):
        a = "cargo check --locked --manifest-path fuzz/Cargo.toml"
        b = "cargo check --locked --manifest-path faraday/fuzz/Cargo.toml"
        self.assertEqual(sorted(map(len, lint_all.chains([a, b, "cargo vet --locked"]))), [1, 1, 1])

    def test_workers_are_bounded(self):
        self.assertEqual(lint_all.workers(1), 2)
        self.assertEqual(lint_all.workers(4), 4)
        self.assertEqual(lint_all.workers(64), 8)

    def test_the_justfile_lint_is_read(self):
        self.assertGreater(len(lint_all.recipe("lint")), len(lint_all.recipe("lint-repo")))
        self.assertIn("python3 -I -B tools/impact.py lint", lint_all.recipe("lint-repo"))


if __name__ == "__main__":
    unittest.main()
