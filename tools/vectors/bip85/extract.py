#!/usr/bin/env python3
"""Extract BIP-85's inline test vectors from bip-0085.mediawiki.

The BIP writes its vectors as INPUT/OUTPUT bullet lists under each
application's heading rather than as a data file, so this script pulls
them out into vectors.json beside it. Run from this directory:

    python3 extract.py > vectors.json

The output has three keys:

  master        the one BIP32 root key every vector in the BIP is
                derived from.
  entropy       the two raw-entropy test cases of the Specification
                section, each with its path, derived key and the 64
                bytes of derived entropy, plus the DRNG case.
  applications  one list per application this tree derives, keyed by
                the name used in core/osk-bip/src/bip85.rs: bip39, wif,
                xprv, hex, base64 and base85. Every entry carries the
                BIP's path, the parameters read back out of that path,
                the derived entropy and the application's own output.

The applications the BIP defines that this tree does not derive (RSA,
RSA GPG, DICE, Nostr) are skipped; nothing in the file depends on them.
"""

import json
import re
import sys

SRC = "bip-0085.mediawiki"

# Which heading holds which application, and what the OUTPUT bullet that
# carries its value is called there.
APPLICATIONS = [
    ("bip39", "12 English words", "BIP39 MNEMONIC", "mnemonic"),
    ("bip39", "18 English words", "BIP39 MNEMONIC", "mnemonic"),
    ("bip39", "24 English words", "BIP39 MNEMONIC", "mnemonic"),
    ("wif", "HD-Seed WIF", "WIF", "wif"),
    ("xprv", "XPRV", "XPRV", "xprv"),
    ("hex", "HEX", None, None),
    ("base64", "PWD BASE64", "PWD", "password"),
    ("base85", "PWD BASE85", "PWD", "password"),
]

# m/83696968'/39'/0'/12'/0' -> ["39", "0", "12", "0"]
PATH = re.compile(r"m/83696968'/([0-9'/]+)")


def section(text, heading):
    """The body under `heading`, down to the next heading of the same or
    a higher level. The BIP nests four levels deep, so the level the
    heading was found at is what ends the section."""
    m = re.search(r"^(=+)" + re.escape(heading) + r"=+\s*$", text, re.M)
    if m is None:
        raise SystemExit("no heading: " + heading)
    start = m.end()
    level = len(m.group(1))
    nxt = re.search(r"^={1," + str(level) + r"}(?!=)", text[start:], re.M)
    return text[start : start + nxt.start()] if nxt else text[start:]


def bullet(body, label):
    """The value after `label:` or `label=` on the first bullet with it."""
    for line in body.splitlines():
        line = line.strip()
        if not line.startswith("*"):
            continue
        stripped = line.lstrip("* ").strip()
        for sep in (":", "="):
            if stripped.startswith(label + sep):
                return stripped[len(label) + 1 :].strip()
    raise SystemExit("not found: " + label)


def path_parts(path):
    """The hardened numbers after BIP-85's own purpose, as strings."""
    m = PATH.search(path)
    if m is None:
        raise SystemExit("not a BIP-85 path: " + path)
    return [p.rstrip("'") for p in m.group(1).split("/") if p]


def main():
    with open(SRC, encoding="utf-8") as f:
        text = f.read()

    spec = section(text, "Test vectors")
    cases = []
    for name in ("Test case 1", "Test case 2"):
        body = section(spec, name)
        cases.append(
            {
                "case": name,
                "path": bullet(body, "PATH"),
                "key": bullet(body, "DERIVED KEY"),
                "entropy": bullet(body, "DERIVED ENTROPY"),
            }
        )
    drng = section(section(text, "BIP85-DRNG"), "Test Vectors")
    drng_case = {
        "case": "DRNG",
        "entropy": bullet(drng, "DERIVED ENTROPY"),
        "drng80": bullet(drng, "DRNG(80 bytes)"),
    }

    master = bullet(section(text, "Test case 1"), "MASTER BIP32 ROOT KEY")

    applications = {}
    for app, heading, label, key in APPLICATIONS:
        body = section(text, heading)
        if bullet(body, "MASTER BIP32 ROOT KEY") != master:
            raise SystemExit("a second master key under " + heading)
        path = bullet(body, "PATH")
        parts = path_parts(path)
        entry = {
            "section": heading,
            "path": path,
            "index": int(parts[-1]),
            "entropy": bullet(body, "DERIVED ENTROPY"),
        }
        if app == "bip39":
            entry["language"] = int(parts[1])
            entry["words"] = int(parts[2])
        elif app == "hex":
            entry["bytes"] = int(parts[1])
        elif app in ("base64", "base85"):
            entry["length"] = int(parts[1])
        if label is not None:
            entry[key] = bullet(body, "DERIVED " + label)
        applications.setdefault(app, []).append(entry)

    json.dump(
        {
            "master": master,
            "entropy": cases + [drng_case],
            "applications": applications,
        },
        sys.stdout,
        indent=1,
    )
    sys.stdout.write("\n")


main()
