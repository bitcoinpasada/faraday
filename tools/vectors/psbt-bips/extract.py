#!/usr/bin/env python3
"""Writes `vectors.json`, BIP-174's, BIP-370's and BIP-371's PSBT test
vectors, from the BIPs' texts as README.md names them.

    python3 -I tools/vectors/psbt-bips/extract.py DIR

DIR holds `bip-0174.mediawiki`, `bip-0370.mediawiki` and
`bip-0371.mediawiki`; each is checked against its SHA-256 first. Every
`** Bytes in Hex` line of the Test
Vectors section is one case, with the `* Case:` line before it (if any)
and what the paragraph above says of it: `invalid`, `valid`,
`signer-fails` (a valid PSBT a signer must refuse), or for BIP-370's
timelock cases `locktime N` and `locktime none`. Nothing is retyped.
Standard library only.
"""

import hashlib
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
SOURCES = {
    "bip-0174.mediawiki": "f2a8e1a9c9e31cc7f607b3c7e2419c63eeccc4bd5b725968cde54ab8cfa1d410",
    "bip-0370.mediawiki": "b2b6e9099592fa04179383f9a1fd8b201fbeef4470f4c3f1c050d25589979c57",
    "bip-0371.mediawiki": "c2e8143a3c9aee0ee38d863cf40947ddf8ee67b4f40a748fb6925ccc5ad4b791",
}


def cases(bip, lines):
    start = lines.index("==Test Vectors==")
    out, expect, name = [], None, None
    for line in lines[start + 1:]:
        if line.startswith("==") and not line.startswith("==="):
            break
        if line.startswith("The following are invalid"):
            expect = "invalid"
        elif line.startswith("The following are valid"):
            expect = "valid"
        elif line.startswith("Fails Signer checks"):
            expect = "signer-fails"
        elif m := re.match(r"^The timelock for the following PSBTs should be computed to be (\d+):",
                           line):
            expect = f"locktime {m.group(1)}"
        elif line.startswith("The timelock for the following PSBTs cannot be computed"):
            expect = "locktime none"
        elif line.startswith("The private keys in the tests below") or line.startswith(
                "The following test vectors"):
            if bip == 174:
                break
        elif m := re.match(r"^\* Case: (.*)$", line):
            name = m.group(1).strip()
        elif m := re.match(r"^\*\* Bytes in Hex: <pre>([0-9a-f]+)</pre>", line):
            out.append({"bip": bip, "case": name or "(unnamed)", "expect": expect,
                        "hex": m.group(1)})
            name = None
    return out


def main():
    src = pathlib.Path(sys.argv[1])
    out = []
    for name, digest in SOURCES.items():
        data = (src / name).read_bytes()
        got = hashlib.sha256(data).hexdigest()
        if got != digest:
            raise SystemExit(f"{name}: SHA-256 {got}, README records {digest}")
        out += cases(int(name[4:8]), data.decode().splitlines())
    (HERE / "vectors.json").write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
