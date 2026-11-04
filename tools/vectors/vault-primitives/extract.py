#!/usr/bin/env python3
"""Writes `vectors.json` from the known answer in `tools/vectors/kdf/README.md`.

    python3 -I tools/vectors/vault-primitives/extract.py

The values are taken from that text, never retyped. Standard library only.
"""

import json
import pathlib
import re

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def argon2id():
    """The known answer `tools/vectors/kdf/README.md` records."""
    text = (ROOT / "tools/vectors/kdf/README.md").read_text()
    row = dict(re.findall(r"^\| (\w+) \| `?([^`|]+?)`?(?: \(ASCII\))? \|$", text, re.M))
    return {
        "password": row["password"].encode().hex(),
        "salt": row["salt"].encode().hex(),
        "memory_kib": int(row["memory"].split()[0]),
        "passes": int(row["passes"]),
        "lanes": int(row["lanes"]),
        "key": row["key"],
    }


def main():
    out = {"argon2id": argon2id()}
    (HERE / "vectors.json").write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
