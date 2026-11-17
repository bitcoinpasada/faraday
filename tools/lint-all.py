#!/usr/bin/env python3
"""Every command of the justfile's `lint` recipe, run to the end. The commands run side by side, as many as the machine has
processors (two to eight); the ones that build in one cargo target folder run one after another.

`just lint` stops at the first command that fails, so a second failure shows
only on the next run. This runs each command of the recipe (and of the recipes
it depends on, in just's order) from the repository's root, prints one line
per command, then the end of the output of each that failed and the command to
run it again. Exits 1 if any failed. An argument names another recipe
(`lint-repo`) to run the same way. The list of commands is the one
tools/impact.py reads from the justfile, so the two cannot differ.
"""
import importlib.util
import os
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TAIL = 60


def recipe(name="lint"):
    sys.dont_write_bytecode = True  # importing impact.py leaves no __pycache__ in tools/
    spec = importlib.util.spec_from_file_location("impact", ROOT / "tools/impact.py")
    impact = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(impact)
    return impact.recipe_commands((ROOT / "justfile").read_text(), name)


def run(cmd):
    t0 = time.monotonic()
    r = subprocess.run(["sh", "-cu", cmd], cwd=ROOT, capture_output=True, text=True)
    secs = time.monotonic() - t0
    print(f"{'ok  ' if r.returncode == 0 else 'FAIL'} {secs:6.1f}s  {cmd}", flush=True)
    return cmd, r.returncode, (r.stdout + r.stderr).splitlines()


# A cargo command that builds takes the build lock of its target folder, so two on one folder would only
# wait for each other: they run one after another, in the recipe's order, on one worker. The folder is
# the manifest's (the fuzz crates and the cross-check programs are workspaces of their own), else the
# workspace's. Every other command (cargo fmt, deny, vet, tree and the scripts) writes nothing a second
# command reads and runs beside the rest.
BUILDS = ("run", "build", "check", "test", "nextest", "clippy")


def target_of(cmd):
    """The target folder a command builds in, or None when it builds nothing."""
    words = cmd.split()
    if "clippy.py" in cmd or "op-cost.py" in cmd:
        return "workspace"
    if len(words) > 1 and words[0] == "cargo" and words[1] in BUILDS:
        return words[words.index("--manifest-path") + 1] if "--manifest-path" in words else "workspace"
    return None


def chains(cmds):
    """The commands as lists run in order on one worker: those on one target folder in a chain,
    the longest chain first, every other command alone, in the recipe's order."""
    by = {}
    for c in cmds:
        t = target_of(c)
        if t is not None:
            by.setdefault(t, []).append(c)
    built = sorted(by.values(), key=len, reverse=True)
    return built + [[c] for c in cmds if target_of(c) is None]


def workers(cpus=None):
    """Bounded by the machine's processors: at least two, at most eight."""
    return max(2, min(cpus or os.cpu_count() or 2, 8))


def run_all(cmds, runner, jobs):
    """Every command run to the end, `jobs` chains at a time. Returns {command: runner's result}."""
    results = {}

    def chain(cs):
        for c in cs:
            results[c] = runner(c)

    with ThreadPoolExecutor(max_workers=jobs) as pool:
        for f in [pool.submit(chain, cs) for cs in chains(cmds)]:
            f.result()
    return results


def report(cmds, results):
    """Prints the end of each failed command's output in the recipe's order; returns the exit code."""
    failed = [(c, results[c][2]) for c in cmds if results[c][1] != 0]
    for cmd, out in failed:
        print(f"\n--- FAIL: {cmd} (last {min(TAIL, len(out))} lines)")
        print("\n".join(out[-TAIL:]))
    if failed:
        print(f"\n{len(failed)} failed. Run again on its own:")
        for cmd, _ in failed:
            print(f"  {cmd}")
        return 1
    return 0


def main(argv):
    name = argv[0] if argv else "lint"
    cmds = recipe(name)
    return report(cmds, run_all(cmds, run, workers()))


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
