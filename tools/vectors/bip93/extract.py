#!/usr/bin/env python3
"""Extract BIP 93's inline test vectors from bip-0093.mediawiki.

The BIP carries its vectors as prose and bullet lists rather than as a
data file, so this script pulls them out into vectors.json beside it.
Run from this directory:

    python3 extract.py > vectors.json

The output has four keys:

  secrets     unshared codex32 secrets (vectors 1, 4, 5, 6, 7 and 8),
              each with its master seed and, where the BIP gives one,
              its master node xprv.
  sets        the two share sets (vectors 2 and 3): the published
              shares, the shares the BIP derives from them, the secret
              they recover, the master seed and the xprv.
  alternates  the other codex32 secrets the BIP lists as equally valid
              encodings of one master seed (vectors 3 and 4), which
              differ only in the padding bits.
  invalid     every string the BIP rejects, grouped under the sentence
              that says why.
"""

import json
import re
import sys

SRC = "bip-0093.mediawiki"
CODE = re.compile(r"<code>([^<]*)</code>")


def section(text, heading):
    start = text.index("===" + heading + "===")
    end = text.index("===", start + 3 + len(heading) + 3)
    return text[start:end]


def one(body, label):
    """The last <code> on the first line carrying `label`."""
    for line in body.splitlines():
        if label in line:
            found = CODE.findall(line)
            if found:
                return found[-1]
    raise SystemExit("not found: " + label)


def bullets(body, prefix):
    """Every bullet that is nothing but one <code> starting with `prefix`."""
    out = []
    for line in body.splitlines():
        m = re.fullmatch(r"\* <code>(" + prefix + r"[^<]*)</code>", line.strip())
        if m:
            out.append(m.group(1))
    return out


def main():
    with open(SRC, encoding="utf-8") as f:
        text = f.read()

    secrets = []
    sets = []
    alternates = []

    v1 = section(text, "Test vector 1")
    secrets.append(
        {
            "vector": "1",
            "string": one(v1, "codex32 secret (bech32)"),
            "seed": one(v1, "Master seed (hex)"),
            "xprv": one(v1, "master node xprv"),
        }
    )

    v2 = section(text, "Test vector 2")
    sets.append(
        {
            "vector": "2",
            "shares": [one(v2, "Share with index <code>A"), one(v2, "Share with index <code>C")],
            "derived": {"D": one(v2, "Derived share with index")},
            "secret": one(v2, "Recovered secret seed"),
            "seed": one(v2, "Master seed (hex)"),
            "xprv": one(v2, "master node xprv"),
        }
    )

    v3 = section(text, "Test vector 3")
    derived3 = {}
    for line in v3.splitlines():
        m = re.match(r"\* Derived share with index <code>(.)</code>: <code>([^<]*)</code>", line)
        if m:
            derived3[m.group(1)] = m.group(2)
    secret3 = one(v3, "codex32-encoded master seed with index")
    sets.append(
        {
            "vector": "3",
            "shares": [
                one(v3, "Share with index <code>a"),
                one(v3, "Share with index <code>c"),
            ],
            "derived": derived3,
            "secret": secret3,
            "seed": one(v3, "Master seed (hex)"),
            "xprv": one(v3, "master node xprv"),
        }
    )
    alternates.append(
        {
            "vector": "3",
            "seed": one(v3, "Master seed (hex)"),
            "strings": [s for s in bullets(v3, "ms13cash") if s != secret3],
        }
    )

    v4 = section(text, "Test vector 4")
    secret4 = one(v4, "codex32 secret")
    secrets.append(
        {
            "vector": "4",
            "string": secret4,
            "seed": one(v4, "Master seed (hex)"),
            "xprv": one(v4, "master node xprv"),
        }
    )
    alternates.append(
        {
            "vector": "4",
            "seed": one(v4, "Master seed (hex)"),
            "strings": [s for s in bullets(v4, "ms10leet") if s != secret4],
        }
    )

    v5 = section(text, "Test vector 5")
    secrets.append(
        {
            "vector": "5",
            "string": one(v5, "codex32 secret"),
            "seed": one(v5, "Master seed (hex)"),
            "xprv": one(v5, "master node xprv"),
            "identifier": one(v5, "identifier (bech32)"),
            "payload": one(v5, "payload (bech32)"),
            "checksum": one(v5, "checksum"),
        }
    )

    v678 = section(text, "Test vectors 6, 7, and 8")
    seed = None
    n = 6
    for line in v678.splitlines():
        if line.startswith("* ") and "master seed (hex)" in line:
            seed = CODE.search(line).group(1)
        elif line.startswith("** codex32 secret"):
            secrets.append({"vector": str(n), "string": CODE.search(line).group(1), "seed": seed})
            n += 1

    inv = text[text.index("===Invalid test vectors==="):text.index("==Appendix==")]
    invalid = []
    for line in inv.splitlines():
        if line.startswith("* <code>"):
            invalid[-1]["strings"].append(CODE.search(line).group(1))
        elif line.startswith("Th"):
            if invalid and not invalid[-1]["strings"]:
                invalid[-1]["reason"] += " " + line.strip()
            else:
                invalid.append({"reason": line.strip(), "strings": []})
    for group in invalid:
        group["reason"] = re.sub(r"\s+", " ", group["reason"])

    json.dump(
        {"secrets": secrets, "sets": sets, "alternates": alternates, "invalid": invalid},
        sys.stdout,
        indent=1,
    )
    sys.stdout.write("\n")


main()
