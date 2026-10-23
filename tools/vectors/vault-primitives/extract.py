#!/usr/bin/env python3
"""Writes `vectors.json` from the published texts named in README.md.

    python3 -I tools/vectors/vault-primitives/extract.py DIR

DIR holds `rfc5869.txt`, `rfc8439.txt` and
`draft-irtf-cfrg-xchacha-03.txt` as downloaded; each is checked against
the SHA-256 the README records before anything is read. The values are
taken from the texts, never retyped. Standard library only.
"""

import hashlib
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]

SOURCES = {
    "rfc5869.txt": "7a40eb3835b35fc947eb12a2ed614db079d43b26e50dbc537c31fba16397089c",
    "rfc8439.txt": "25bef70fbf7a07ff45c2fe4cb7c6ce954eac687413d8610603268b4e4415324c",
    "draft-irtf-cfrg-xchacha-03.txt": "fa796b50265eeee383d40e82fed880267c7835e1b3d64c50c4f06162adaa1cfd",
}


def page_free(text):
    """The text without the page footers and headers RFCs carry."""
    out = []
    for line in text.splitlines():
        if re.match(r"^(Krawczyk|Nir|Arciszewski) .*\[Page \d+\]$", line):
            continue
        if re.match(r"^(RFC \d+|Internet-Draft) {2,}", line):
            continue
        out.append(line)
    return out


def hkdf(lines):
    """RFC 5869 A.1 to A.3, the SHA-256 cases."""
    cases = []
    for n in (1, 2, 3):
        start = lines.index(next(l for l in lines if l.startswith(f"A.{n}.  Test Case {n}")))
        end = lines.index(next(l for l in lines[start + 1:] if re.match(r"^A\.\d+\.  ", l)))
        block = lines[start:end]
        case = {}
        key = None
        for line in block:
            m = re.match(r"^   (IKM|salt|info|L|PRK|OKM) += (.*)$", line)
            if m:
                key, value = m.group(1), m.group(2)
                case[key] = value
            elif key and re.match(r"^ {10}[0-9a-f]+", line):
                case[key] += line.strip()
            else:
                key = None if not line.strip() else key
        out = {}
        for k, v in case.items():
            v = re.sub(r"\(\d+ octets\)", "", v).strip()
            if k == "L":
                out[k] = int(v)
            else:
                out[k.lower()] = "" if v == "" else v.removeprefix("0x").replace(" ", "")
        if "Hash = SHA-256" not in "\n".join(block):
            raise SystemExit(f"case {n} is not SHA-256")
        cases.append(out)
    return cases


def hexdump(lines, i):
    """The bytes of an RFC 8439 hex dump starting at line i."""
    out = ""
    while i < len(lines):
        m = re.match(r"^  \d{3}  ((?:[0-9a-f]{2} ?)+)", lines[i])
        if not m:
            break
        out += m.group(1).replace(" ", "")
        i += 1
    return out


def chacha20(lines):
    """RFC 8439 A.1, the five ChaCha20 block function vectors."""
    start = lines.index("A.1.  The ChaCha20 Block Functions")
    end = lines.index("A.2.  ChaCha20 Encryption")
    block = lines[start:end]
    cases = []
    for i, line in enumerate(block):
        if re.match(r"^  Test Vector #\d+:$", line):
            cases.append({})
        elif line.strip() in ("Key:", "Nonce:", "Keystream:"):
            cases[-1][line.strip()[:-1].lower()] = hexdump(block, i + 1)
        elif line.strip().startswith("Block Counter = "):
            cases[-1]["counter"] = int(line.strip().split("=")[1])
    return cases


def xchacha(lines):
    """draft-irtf-cfrg-xchacha-03 A.3.1, the AEAD vector."""
    start = lines.index("A.3.1.  AEAD_XCHACHA20_POLY1305")
    end = lines.index("A.3.2.  XChaCha20")
    block = lines[start:end]
    out = {}
    key = None
    for line in block:
        s = line.strip()
        if s.endswith(":"):
            key = {"Plaintext:": "plaintext", "AAD:": "aad", "Key:": "key", "IV:": "nonce",
                   "Ciphertext:": "ciphertext", "Tag:": "tag"}.get(s)
            if key:
                out[key] = ""
        elif key and re.fullmatch(r"[0-9a-f]+", s):
            out[key] += s
    return out


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
    src = pathlib.Path(sys.argv[1])
    texts = {}
    for name, digest in SOURCES.items():
        data = (src / name).read_bytes()
        got = hashlib.sha256(data).hexdigest()
        if got != digest:
            raise SystemExit(f"{name}: SHA-256 {got}, README records {digest}")
        texts[name] = page_free(data.decode())
    out = {
        "hkdf_sha256": hkdf(texts["rfc5869.txt"]),
        "chacha20_block": chacha20(texts["rfc8439.txt"]),
        "xchacha20poly1305": xchacha(texts["draft-irtf-cfrg-xchacha-03.txt"]),
        "argon2id": argon2id(),
    }
    (HERE / "vectors.json").write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
