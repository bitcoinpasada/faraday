#!/usr/bin/env python3
"""faraday/builds.toml, the list of what this repository builds, against
the tree: every id unique, every field one the file's header names, a
built entry's recipe a `just` recipe and its board and panel directories
that exist, a planned entry's document one that exists, and every
directory under faraday/image/boards and faraday/image/panels named by a
built entry. Run by `just lint`; standard library only."""
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FILE = ROOT / "faraday/builds.toml"
FIELDS = {"id", "label", "group", "status", "board", "panel", "recipe", "vars", "args", "env",
          "needs", "dev", "outputs", "release", "spec"}
GROUPS = {"app", "image", "starter"}


def main():
    errors = []
    try:
        builds = tomllib.loads(FILE.read_text())["build"]
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
        if b.get("status") == "built":
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
            say(f"status is {b.get('status')!r}, not built or planned")
    for d in sorted(boards - named_boards):
        errors.append(f"faraday/image/boards/{d}: no built entry in faraday/builds.toml builds it")
    for d in sorted(panels - named_panels):
        errors.append(f"faraday/image/panels/{d}: no built entry in faraday/builds.toml builds it")
    for e in errors:
        print(f"lint-builds: {e}", file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
