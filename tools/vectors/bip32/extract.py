#!/usr/bin/env python3
"""Writes `vectors.json`, BIP-32's test vectors 1 to 4, from
`bip-0032.mediawiki` as README.md names it.

    python3 -I tools/vectors/bip32/extract.py bip-0032.mediawiki

The file is checked against the SHA-256 the README records; the seeds,
paths and keys are taken from it, never retyped. Standard library only.
"""

import hashlib
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
SHA256 = "e5e00a8289db2f681052cf24a745320afc225e66b25d1e489a7c884d2fc7f11f"


def main():
    data = pathlib.Path(sys.argv[1]).read_bytes()
    got = hashlib.sha256(data).hexdigest()
    if got != SHA256:
        raise SystemExit(f"SHA-256 {got}, README records {SHA256}")
    vectors = []
    for line in data.decode().splitlines():
        if m := re.match(r"^===Test vector (\d)===$", line):
            if m.group(1) == "5":
                break
            vectors.append({"seed": None, "chains": []})
        elif m := re.match(r"^Seed \(hex\): ([0-9a-f]+)$", line):
            vectors[-1]["seed"] = m.group(1)
        elif m := re.match(r"^\* Chain (m[0-9/<>a-zH]*)$", line):
            path = m.group(1).replace("<sub>H</sub>", "h")
            vectors[-1]["chains"].append({"path": path})
        elif m := re.match(r"^\*\* ext (pub|prv): (\w+)$", line):
            vectors[-1]["chains"][-1]["x" + m.group(1)] = m.group(2)
    (HERE / "vectors.json").write_text(json.dumps(vectors, indent=1) + "\n")


if __name__ == "__main__":
    main()
