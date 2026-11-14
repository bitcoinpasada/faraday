#!/usr/bin/env python3
"""Every command of the justfile's `lint` recipe, run to the end. The commands that do not call cargo run four at a time beside the
cargo ones.

`just lint` stops at the first command that fails, so a second failure shows
only on the next run. This runs each command of the recipe (and of the recipes
it depends on, in just's order) from the repository's root, prints one line
per command, then the end of the output of each that failed and the command to
run it again. Exits 1 if any failed. The list of commands is the one
tools/impact.py reads from the justfile, so the two cannot differ.
"""
import importlib.util
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TAIL = 60


def recipe():
    sys.dont_write_bytecode = True  # importing impact.py leaves no __pycache__ in tools/
    spec = importlib.util.spec_from_file_location("impact", ROOT / "tools/impact.py")
    impact = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(impact)
    return impact.recipe_commands((ROOT / "justfile").read_text(), "lint")


def run(cmd):
    t0 = time.monotonic()
    r = subprocess.run(["sh", "-cu", cmd], cwd=ROOT, capture_output=True, text=True)
    secs = time.monotonic() - t0
    print(f"{'ok  ' if r.returncode == 0 else 'FAIL'} {secs:6.1f}s  {cmd}", flush=True)
    return cmd, r.returncode, (r.stdout + r.stderr).splitlines()


def main():
    # A command that does not call cargo (the scripts, single-threaded) runs in a small pool beside
    # the cargo ones, which run one after another as they always did (cargo locks its target folder).
    cmds = recipe()
    results = {}
    with ThreadPoolExecutor(max_workers=4) as pool:
        side = {c: pool.submit(run, c) for c in cmds if "cargo" not in c}
        for c in cmds:
            if c not in side:
                results[c] = run(c)
        for c, f in side.items():
            results[c] = f.result()
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


if __name__ == "__main__":
    sys.exit(main())
