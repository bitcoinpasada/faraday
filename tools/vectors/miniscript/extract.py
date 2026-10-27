#!/usr/bin/env python3
"""Writes `core-miniscript.json` from Bitcoin Core's miniscript unit
tests, as README.md names the file.

    python3 -I tools/vectors/miniscript/extract.py miniscript_tests.cpp

The file is checked against the SHA-256 the README records. Every call
`Test("<miniscript>", "<P2WSH script>", "<Tapscript script>", <mode>...)`
written on one line with literal strings is read: the miniscript, the
two scripts as Core expects them ("?" where the test states none, the
P2WSH one where it writes "="), the mode's flags, and the operation
count and stack size the test states, where it states them. The calls that
build their miniscript in a loop are left out. Nothing is retyped.
Standard library only.
"""

import hashlib
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
SHA256 = "510cf2af017b6c7b8def36ad9a4cc486f046b25a361130a6a2ee281b4537ffc1"

FLAGS = {
    "TESTMODE_INVALID": 0,
    "TESTMODE_VALID": 1,
    "TESTMODE_NONMAL": 2,
    "TESTMODE_NEEDSIG": 4,
    "TESTMODE_TIMELOCKMIX": 8,
    "TESTMODE_P2WSH_INVALID": 16,
    "TESTMODE_TAPSCRIPT_INVALID": 32,
}

CALL = re.compile(r'^\s*Test\("([^"]*)", "([^"]*)", "([^"]*)", ([A-Z_0-9 |]+?)(?:, ([^)]*))?\);')


def main():
    data = pathlib.Path(sys.argv[1]).read_bytes()
    got = hashlib.sha256(data).hexdigest()
    if got != SHA256:
        raise SystemExit(f"SHA-256 {got}, README records {SHA256}")
    cases = []
    for number, line in enumerate(data.decode().splitlines(), 1):
        m = CALL.match(line)
        if not m:
            continue
        ms, wsh, tap, mode, rest = m.groups()
        flags = 0
        for name in mode.split("|"):
            flags |= FLAGS[name.strip()]
        limits = [x.strip() for x in rest.split(",")] if rest else []
        ops = int(limits[0]) if limits and limits[0] not in ("-1", "{}") else None
        stack = int(limits[1]) if len(limits) > 1 and limits[1] not in ("-1", "{}") else None
        cases.append({
            "line": number,
            "ms": ms,
            "wsh": wsh,
            "tap": wsh if tap == "=" else tap,
            "mode": flags,
            "ops": ops,
            "stack": stack,
        })
    out = {
        "source": "bitcoin/bitcoin src/test/miniscript_tests.cpp, tag v31.1",
        "flags": FLAGS,
        "cases": cases,
    }
    (HERE / "core-miniscript.json").write_text(json.dumps(out, indent=1) + "\n")
    print(f"{len(cases)} cases")


if __name__ == "__main__":
    main()
