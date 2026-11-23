#!/usr/bin/env python3
"""Searches a finished binary for work kept out of the app (F325 point 3).

    binary-crate.py absent BINARY...    fails if one holds the handwriting reader
    binary-crate.py present BINARY...   fails if one does not (the positive
                                        control, for a dev build of Faraday Demo)

The reader is found by the first 32 bytes of each of its weight files
(`faraday/faraday-handwriting/weights/`), which the crate includes whole
into any binary that uses it. The recipes that build the shipped binaries
(the images, the Starter, Faraday Demo of a release) run `absent`; Faraday
Demo's dev build (`just dev=1 faraday-linux-bin`) is the control: `present`
finds the weights once the build calls the reader (a crate nothing calls is
dropped by the linker, so before that the control finds nothing).

Run from the repository root.
"""
import pathlib
import sys

WEIGHTS = pathlib.Path("faraday/faraday-handwriting/weights")
HEAD = 32


def markers():
    files = sorted(WEIGHTS.glob("*.bin"))
    if not files:
        raise SystemExit(f"binary-crate: no weight files in {WEIGHTS}; moved or renamed?")
    out = {}
    for f in files:
        head = f.read_bytes()[:HEAD]
        if len(head) < HEAD:
            raise SystemExit(f"binary-crate: {f} is shorter than {HEAD} bytes")
        out[f.name] = head
    return out


def found(path, marks):
    data = pathlib.Path(path).read_bytes()
    return [name for name, m in marks.items() if m in data]


def main(argv):
    if len(argv) < 2 or argv[0] not in ("absent", "present"):
        print(__doc__)
        return 2
    mode, marks, bad = argv[0], markers(), 0
    for path in argv[1:]:
        hits = found(path, marks)
        if mode == "absent" and hits:
            print(f"binary-crate: {path} holds the handwriting reader's weights ({', '.join(hits)})")
            bad = 1
        elif mode == "present" and not hits:
            print(f"binary-crate: {path} holds none of the handwriting reader's weights")
            bad = 1
        else:
            print(f"binary-crate: {path}: {mode}")
    return bad


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
