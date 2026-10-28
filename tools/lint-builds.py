#!/usr/bin/env python3
"""faraday/builds.toml, the list of what this repository builds, against
the tree: every id unique, every field one the file's header names, a
built or untried entry's recipe a `just` recipe and its board and panel
directories that exist, a planned entry's document one that exists, and every directory under
faraday/image/boards and faraday/image/panels named by a built or untried
entry; and the [[hardware]] entries the download page's tiles come from:
every id unique, every field known, a built or planned image offered under
one of them with its build, no two images of one hardware alike, every
hardware with an image, and tested only where an image is built. Run by
`just lint`; standard library only."""
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FILE = ROOT / "faraday/builds.toml"
FIELDS = {"id", "label", "group", "status", "board", "panel", "recipe", "vars", "args", "env",
          "needs", "dev", "outputs", "release", "spec", "hardware", "build", "option"}
GROUPS = {"app", "image", "starter"}
HW_FIELDS = {"id", "label", "fits", "soon", "tested"}
KINDS = {"strict", "compatible"}


def main():
    errors = []
    try:
        data = tomllib.loads(FILE.read_text())
        builds, hardware = data["build"], data["hardware"]
    except (OSError, KeyError, tomllib.TOMLDecodeError) as e:
        print(f"lint-builds: cannot read {FILE.relative_to(ROOT)}: {e}", file=sys.stderr)
        return 1
    recipes = set(subprocess.run(["just", "--summary"], cwd=ROOT, capture_output=True, text=True,
                                 check=True).stdout.split())
    boards = {d.name for d in (ROOT / "faraday/image/boards").iterdir() if d.is_dir()}
    panels = {d.name for d in (ROOT / "faraday/image/panels").iterdir() if d.is_dir()}
    seen, named_boards, named_panels = set(), set(), set()
    for b in builds:
        bid = b.get("id", "?")
        say = lambda m: errors.append(f"{bid}: {m}")  # noqa: E731
        if bid in seen:
            say("the id is used twice")
        seen.add(bid)
        for k in sorted(set(b) - FIELDS):
            say(f"unknown field {k}")
        for k in ("id", "label", "group", "status"):
            if k not in b:
                say(f"no {k}")
        if b.get("group") not in GROUPS:
            say(f"group is {b.get('group')!r}, not one of {sorted(GROUPS)}")
        if b.get("status") in ("built", "untried"):
            if b.get("recipe") not in recipes:
                say(f"recipe {b.get('recipe')!r} is not a just recipe")
            if not b.get("outputs"):
                say("a built entry names its outputs")
            if b.get("group") == "image":
                if b.get("board") not in boards:
                    say(f"board {b.get('board')!r} is not a directory under faraday/image/boards")
                if b.get("panel") not in panels:
                    say(f"panel {b.get('panel')!r} is not a directory under faraday/image/panels")
                named_boards.add(b.get("board"))
                named_panels.add(b.get("panel"))
        elif b.get("status") == "planned":
            if not b.get("spec") or not (ROOT / b["spec"]).is_file():
                say(f"a planned entry names the document that plans it (spec), got {b.get('spec')!r}")
        else:
            say(f"status is {b.get('status')!r}, not built, untried or planned")
    errors += check_hardware(builds, hardware)
    for d in sorted(boards - named_boards):
        errors.append(f"faraday/image/boards/{d}: no built or untried entry in faraday/builds.toml builds it")
    for d in sorted(panels - named_panels):
        errors.append(f"faraday/image/panels/{d}: no built or untried entry in faraday/builds.toml builds it")
    for e in errors:
        print(f"lint-builds: {e}", file=sys.stderr)
    return 1 if errors else 0


def check_hardware(builds, hardware):
    """The tiles' entries and the images offered under them."""
    errors, ids = [], set()
    for h in hardware:
        hid = h.get("id", "?")
        if hid in ids:
            errors.append(f"hardware {hid}: the id is used twice")
        ids.add(hid)
        for k in sorted(set(h) - HW_FIELDS):
            errors.append(f"hardware {hid}: unknown field {k}")
        for k in ("id", "label", "fits", "tested"):
            if k not in h:
                errors.append(f"hardware {hid}: no {k}")
    offered, built = {}, set()
    for b in builds:
        bid = b.get("id", "?")
        if b.get("group") != "image" or b.get("status") == "untried":
            for k in ("hardware", "build", "option"):
                if k in b:
                    errors.append(f"{bid}: {k} is for an image that is built or planned")
            continue
        if b.get("hardware") not in ids:
            errors.append(f"{bid}: hardware {b.get('hardware')!r} is not a [[hardware]] id")
        if b.get("build") not in KINDS:
            errors.append(f"{bid}: build is {b.get('build')!r}, not one of {sorted(KINDS)}")
        key = (b.get("hardware"), b.get("build"), b.get("option"))
        if key in offered:
            errors.append(f"{bid}: offered as {offered[key]} is, under the same hardware, build and option")
        offered[key] = bid
        if b.get("status") == "built":
            built.add(b.get("hardware"))
    for h in hardware:
        hid = h.get("id", "?")
        if not any(k[0] == hid for k in offered):
            errors.append(f"hardware {hid}: no image is offered under it")
        if h.get("tested") and hid not in built:
            errors.append(f"hardware {hid}: tested, but no image of it is built")
    return errors


if __name__ == "__main__":
    sys.exit(main())
