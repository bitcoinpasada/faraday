#!/usr/bin/env python3
"""Writes `bip143.json` from BIP-143's examples, as README.md names the
file.

    python3 -I tools/vectors/sighash/extract.py bip-0143.mediawiki

The file is checked against the SHA-256 the README records. Each example
section's unsigned transaction is read, then every signature hash it
states with the scriptCode, amount and nHashType stated before it, the
input found by its outpoint (the first input where a section states
none). Nothing is retyped. Standard library only.
"""

import hashlib
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
SHA256 = "62bc71351563e68baeb12643c68355d217953ae9eb6a6e68b2b0323275b6beec"


def outpoints(tx):
    """The outpoints of a serialised transaction without witnesses."""
    b = bytes.fromhex(tx)
    at, n = 5, b[4]
    out = []
    for _ in range(n):
        out.append(b[at:at + 36].hex())
        script_len = b[at + 36]
        at += 36 + 1 + script_len + 4
    return out


def main():
    data = pathlib.Path(sys.argv[1]).read_bytes()
    got = hashlib.sha256(data).hexdigest()
    if got != SHA256:
        raise SystemExit(f"SHA-256 {got}, README records {SHA256}")
    lines = data.decode().splitlines()
    cases, section, tx, take_tx = [], None, None, False
    state = {}
    for line in lines:
        if m := re.match(r"^=== (.+) ===$", line):
            section, tx, state = m.group(1), None, {}
            continue
        s = line.strip()
        if m := re.match(r"^The following is an unsigned transaction:\s*([0-9a-f]*)$", s):
            tx, take_tx = (m.group(1), False) if m.group(1) else (None, True)
            continue
        if take_tx and re.fullmatch(r"[0-9a-f]+", s):
            tx, take_tx = s, False
            continue
        if m := re.match(r"^(outpoint|scriptCode|amount|nHashType):\s+([0-9a-f]+)$", s):
            state[m.group(1)] = m.group(2)
        elif m := re.match(r"^sigHash:\s+([0-9a-f]{64})$", s):
            if tx is None or not {"scriptCode", "amount", "nHashType"} <= set(state):
                raise SystemExit(f"{section}: a sigHash without its inputs")
            ops = outpoints(tx)
            index = ops.index(state["outpoint"]) if "outpoint" in state else 0
            code = bytes.fromhex(state["scriptCode"])
            # The scriptCode is written with its length in front.
            if code[0] != len(code) - 1:
                raise SystemExit(f"{section}: a scriptCode whose length is not its first byte")
            cases.append({
                "section": section, "tx": tx, "index": index, "script_code": code[1:].hex(),
                "amount": int.from_bytes(bytes.fromhex(state["amount"]), "little"),
                "hash_type": int.from_bytes(bytes.fromhex(state["nHashType"]), "little"),
                "sighash": m.group(1),
            })
    (HERE / "bip143.json").write_text(json.dumps(cases, indent=1) + "\n")


if __name__ == "__main__":
    main()
