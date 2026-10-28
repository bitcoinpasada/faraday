#!/usr/bin/env python3
"""The cross-check gives the same report whatever runs it
(`just crosscheck-identity`, through `run.sh --identity`): the real
areas against the Rust command, at a small scale, under one worker and
under several, and under CPython and PyPy when PyPy is found; the
reports must be the same byte for byte once the seconds are taken out.

    identity.py --rust EXE [--seed S] [--scale 0.05] [--jobs 1,4] [--python cpython,pypy]
"""

import argparse
import difflib
import pathlib
import re
import shutil
import subprocess
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
TIMES = [(re.compile(r"\(\d+(\.\d+)? s\)"), "(T s)"), (re.compile(r"after \d+(\.\d+)? s"), "after T s")]


def untimed(text):
    for pattern, repl in TIMES:
        text = pattern.sub(repl, text)
    return text


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rust", required=True)
    ap.add_argument("--seed", default="faraday-crosscheck-identity")
    ap.add_argument("--scale", default="0.05")
    ap.add_argument("--jobs", default="1,4")
    ap.add_argument("--python", default=None, help="default: cpython, and pypy when on the PATH")
    ap.add_argument("--area", action="append", default=[])
    args = ap.parse_args()
    pythons = (args.python or ("cpython,pypy" if shutil.which("pypy3") else "cpython")).split(",")
    runs = [(j, py) for py in pythons for j in args.jobs.split(",")]
    reports = {}
    for jobs, py in runs:
        cmd = [sys.executable, "-I", "-B", str(HERE / "crosscheck.py"), "--rust", args.rust,
               "--seed", args.seed, "--scale", args.scale, "--jobs", jobs, "--python", py,
               "--progress", ""]
        for a in args.area:
            cmd += ["--area", a]
        started = time.monotonic()
        out = subprocess.run(cmd, capture_output=True, text=True)
        took = time.monotonic() - started
        reports[(jobs, py)] = untimed(out.stdout)
        verdict = out.stdout.strip().splitlines()[-2:] if out.stdout.strip() else [out.stderr[-500:]]
        print(f"identity: --jobs {jobs} --python {py}: exit {out.returncode} in {took:.1f} s; "
              + " / ".join(verdict), flush=True)
    first = runs[0]
    same = True
    for run in runs[1:]:
        if reports[run] != reports[first]:
            same = False
            print(f"identity: --jobs {run[0]} --python {run[1]} DIFFERS from --jobs {first[0]}"
                  f" --python {first[1]}:")
            sys.stdout.writelines(difflib.unified_diff(
                reports[first].splitlines(True), reports[run].splitlines(True)))
    print("identity: " + ("every report is the same" if same else "FAILED"))
    return 0 if same else 1


if __name__ == "__main__":
    sys.exit(main())
