#!/usr/bin/env python3
"""Writes `vectors.json` from the BIPs README.md names.

    python3 -I tools/vectors/addresses/extract.py DIR

DIR holds `bip-0173.mediawiki`, `bip-0350.mediawiki`, `bip-0380.mediawiki`,
`bip-0049.mediawiki`, `bip-0084.mediawiki` and `bip-0086.mediawiki` from
the bitcoin/bips repository at the commit README.md names; each is
checked against its SHA-256 before it is read. The values are taken from
the texts, never retyped. Standard library only.
"""

import hashlib
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
SOURCES = {
    "bip-0173.mediawiki": "b19eab06b264bbd15ad7f7d640fa34de8f4a8809fba8a6ecc3e165545b27a543",
    "bip-0350.mediawiki": "63634b06aa8bae88b31929674736e74964c4598684269ac2b0b140c43a7a0dec",
    "bip-0380.mediawiki": "34e6510bb2eba9445a68eacf3d24d5a4e5f24481477488d5552f674ac81dd5fe",
    "bip-0049.mediawiki": "307b62faf7b9927233b5112e93d591272ba0c48303b5dbb6f0bbfbaf2347e695",
    "bip-0084.mediawiki": "1900feec6cafca65b8c09906ca0658d2d742b4c9b44cb15678996985b6bfe627",
    "bip-0086.mediawiki": "d8d01dee331da07c2562615bc1f064c1868ec3fce61184973da76d1196c7f5b0",
}


def section(lines, start, stop):
    """The lines after the first that starts with `start`, up to the
    next that starts with `stop`."""
    i = next(n for n, l in enumerate(lines) if l.startswith(start))
    j = next(n for n in range(i + 1, len(lines)) if lines[n].startswith(stop))
    return lines[i + 1:j]


def bullets(lines):
    """`* <tt>A</tt>: <tt>B</tt>` and `* <tt>A</tt>: reason` as pairs."""
    out = []
    for line in lines:
        if m := re.match(r"^\* <tt>([^<]*)</tt>(?:: (?:<tt>([0-9a-f]*)</tt>|(.*)))?$", line):
            out.append((m.group(1), m.group(2) if m.group(2) is not None else m.group(3)))
    return out


def segwit(texts):
    bip173 = texts["bip-0173.mediawiki"]
    bip350 = texts["bip-0350.mediawiki"]
    valid173 = bullets(section(bip173, "The following list gives valid segwit addresses",
                               "The following list gives invalid"))
    invalid173 = bullets(section(bip173, "The following list gives invalid segwit addresses",
                                 "===Checksum design==="))
    valid350 = bullets(section(bip350, "The following list gives valid segwit addresses",
                               "The following list gives invalid"))
    invalid350 = bullets(section(bip350, "The following list gives invalid segwit addresses",
                                 "==Appendix"))
    valid = [{"address": a, "script": s, "source": "BIP-350"} for a, s in valid350]
    # BIP-173's version 0 addresses stand; its addresses of version 1 and
    # above carry the checksum BIP-350 replaced, and are invalid now.
    valid += [{"address": a, "script": s, "source": "BIP-173"} for a, s in valid173
              if s.startswith("00")]
    invalid = [{"address": a, "reason": r, "source": "BIP-350"} for a, r in invalid350]
    invalid += [{"address": a, "reason": r, "source": "BIP-173"} for a, r in invalid173]
    invalid += [{"address": a, "reason": "Bech32 for a version above 0 (BIP-350)",
                 "source": "BIP-173"} for a, s in valid173 if not s.startswith("00")]
    return valid, invalid


def checksums(lines):
    """BIP-380's checksum and character-set cases: (name, text)."""
    out = []
    for line in section(lines, "The following tests cover the checksum",
                        "The following tests cover key expressions"):
        if m := re.match(r"^\* ([^:]+): <tt>(.*)</tt>$", line):
            out.append({"name": m.group(1), "text": m.group(2)})
    return out


def assignments(lines):
    """`name = value` lines inside the BIP's <pre> block, in order."""
    start = next(n for n, l in enumerate(lines) if l.lower().startswith("==test vectors=="))
    pre = section(lines[start:], "<pre>", "</pre>")
    out = []
    for line in pre:
        if m := re.match(r"^\s*(?://\s*(.*)|(\w+)\s*=\s*(.*?)\s*)$", line):
            out.append(("//", m.group(1)) if m.group(1) is not None else (m.group(2), m.group(3)))
    return out


def accounts(texts):
    """BIP-49's, BIP-84's and BIP-86's accounts: the words, the account
    xpub as the BIP writes it, and each address with its change and
    index."""
    out = []
    for name, purpose in [("bip-0049.mediawiki", 49), ("bip-0084.mediawiki", 84),
                          ("bip-0086.mediawiki", 86)]:
        acct = {"purpose": purpose, "network": "mainnet", "addresses": []}
        where = None
        for key, value in assignments(texts[name]):
            if key == "//":
                m = re.search(r"m/\d+'/\d+'/0'/(\d)/(\d)", value)
                where = (int(m.group(1)), int(m.group(2))) if m else None
                if "root = m/" in value:
                    where = "account"
                continue
            testnet = value.endswith("(testnet)")
            value = value.removesuffix("(testnet)").strip()
            if testnet:
                acct["network"] = "testnet"
            if key in ("mnemonic", "masterseedWords"):
                acct["mnemonic"] = value
            elif where == "account" and key in ("xpub", "account0Xpub"):
                acct["xpub"] = value
            elif key == "address":
                m = re.search(r"([123mn][1-9A-HJ-NP-Za-km-z]{25,34}|bc1\w+|tb1\w+)$", value)
                acct["addresses"].append({"change": where[0] if isinstance(where, tuple) else 0,
                                          "index": where[1] if isinstance(where, tuple) else 0,
                                          "address": m.group(1)})
        out.append(acct)
    return out


def main():
    src = pathlib.Path(sys.argv[1])
    texts = {}
    for name, digest in SOURCES.items():
        data = (src / name).read_bytes()
        got = hashlib.sha256(data).hexdigest()
        if got != digest:
            raise SystemExit(f"{name}: SHA-256 {got}, README records {digest}")
        texts[name] = data.decode().splitlines()
    valid, invalid = segwit(texts)
    out = {
        "segwit_valid": valid,
        "segwit_invalid": invalid,
        "descriptor_checksums": checksums(texts["bip-0380.mediawiki"]),
        "accounts": accounts(texts),
    }
    (HERE / "vectors.json").write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
