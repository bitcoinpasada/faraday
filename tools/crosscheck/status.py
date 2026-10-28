#!/usr/bin/env python3
"""How far a cross-check has got (`just crosscheck-status`), read from
`out/crosscheck/progress.json`, which a running `just crosscheck`
rewrites as it goes; works from another terminal or over ssh while the
run goes on. Standard library only; reads, never writes.

    status.py                 the state now, once
    status.py --follow        again every few seconds until the run ends
    status.py --log           the progress log so far (progress.log)
    status.py --dir DIR       another progress folder (crosscheck.py --progress)
"""

import argparse
import json
import os
import pathlib
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[2]
ENDED = ("passed", "failed")


def say_time(seconds):
    seconds = int(seconds)
    if seconds < 60:
        return f"{seconds} s"
    if seconds < 3600:
        return f"{seconds // 60} min {seconds % 60} s"
    return f"{seconds // 3600} h {seconds // 60 % 60} min"


def alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def read(folder):
    try:
        return json.loads((folder / "progress.json").read_text())
    except FileNotFoundError:
        return None
    except json.JSONDecodeError:
        return None


def render(p):
    phase = p["phase"]
    if phase not in ENDED and not alive(p["pid"]):
        phase += " (the run is no longer running)"
    age = time.time() - p["updated"]
    out = [f"crosscheck: {phase}; seed {p['seed']}, scale {p['scale']}, {p['jobs']} workers; "
           f"{say_time(p['elapsed'])} into this phase, updated {say_time(age)} ago"]
    if p["phase"] not in ENDED:
        left = "" if p["seconds_left"] is None else f", about {say_time(p['seconds_left'])} left"
        out.append(f"  {p['cases_done']} of {p['cases_total']} cases, {p['cases_per_second']} a "
                   f"second, {p['workers_busy']} of {p['jobs']} workers busy{left}")
    width = max((len(a) for a in p["areas"]), default=4)
    for name, a in sorted(p["areas"].items(), key=lambda kv: (
            ["failed", "running", "queued", "done"].index(kv[1]["state"]), kv[0])):
        pct = 100 * a["cases_done"] // a["cases_total"] if a["cases_total"] else 100
        extra = ""
        if a["known"]:
            extra += f", {a['known']} known differences"
        if a["disagreements"]:
            extra += f", {a['disagreements']} DISAGREEMENT"
        if a["errors"]:
            extra += f", {a['errors']} ERROR"
        out.append(f"  {name:<{width}}  {a['state']:<7} {a['cases_done']:>6}/{a['cases_total']:<6}"
                   f" {pct:>3}%  {a['answers']} answers{extra}")
    for d in p["disagreements"]:
        where = f" at case {d['case']}" if d.get("case") else ""
        out.append(f"  {d['status'].upper()} in {d['area']}{where}: {d['op']}")
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--dir", default=str(ROOT / "out" / "crosscheck"))
    ap.add_argument("--follow", action="store_true")
    ap.add_argument("--log", action="store_true")
    ap.add_argument("--every", type=float, default=5.0, help="seconds between --follow's prints")
    args = ap.parse_args()
    folder = pathlib.Path(args.dir)
    if args.log:
        try:
            sys.stdout.write((folder / "progress.log").read_text())
        except FileNotFoundError:
            print(f"crosscheck-status: no progress log in {folder}")
            return 1
        return 0
    while True:
        p = read(folder)
        if p is None:
            try:
                last = (folder / "progress.log").read_text().splitlines()[-1]
                print(f"crosscheck: starting; the log's last line: {last}", flush=True)
                if " FAILED" in last:
                    return 1
            except (FileNotFoundError, IndexError):
                print(f"crosscheck-status: no run's progress in {folder}", flush=True)
                if not args.follow:
                    return 1
        else:
            print(render(p), flush=True)
            if not args.follow or p["phase"] in ENDED or not alive(p["pid"]):
                return 0 if p["phase"] != "failed" else 1
        time.sleep(args.every)
        print()


if __name__ == "__main__":
    sys.exit(main())
