#!/usr/bin/env python3
"""Open a Faraday vault and print the slot a passphrase opens.

The format is `docs/VAULT.md`. It is not Faraday's secret: this file and
two published libraries read it, so a person with the file and a
passphrase does not need Faraday to get their keys, wallets, notes and
entries back.

Usage:

    python3 tools/vault/open.py vault.ofv
    python3 tools/vault/open.py vault.ofv --passphrase PASSPHRASE

With no `--passphrase` it is read from the terminal without echo. The
records of the slot it opens are printed on standard output; nothing is
written.

Dependencies: the standard library plus either `argon2-cffi` and
`pynacl`, or `cryptography` 44 or newer on OpenSSL 3.2 or newer. The two
primitives and the BIP-39 wordlists are OpenSigner's own reader's,
`tools/backup/decrypt.py`, imported from this repository.

The layout, all little-endian:

     0   magic "OFVT", 4 bytes
     4   version, 1
     5   Argon2id memory, KiB, u32
     9   Argon2id passes, u32
    13   Argon2id lanes, u32
    17   slot plaintext length L, u32
    21   salt, 32 bytes               -- 0..53 is the header
    53   four slots, each a 24-byte nonce, L bytes of ciphertext, a tag

K = Argon2id(passphrase, salt); the slot key is HKDF-SHA256 of K with an
empty salt and the info "Faraday vault v1 slot"; slot i's associated
data is the header and the byte i. Every slot is tried; exactly one
opens.
"""

from __future__ import annotations

import argparse
import getpass
import hashlib
import hmac
import importlib.util
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

_spec = importlib.util.spec_from_file_location("osk_decrypt", ROOT / "tools" / "backup" / "decrypt.py")
osk = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(osk)

MAGIC = b"OFVT"
HEADER_LEN = 53
NONCE_LEN = 24
TAG_LEN = 16
SLOT_SIZES = (65536, 262144, 1048576, 4194304)
SLOT_INFO = b"Faraday vault v1 slot"

RECORDS = {
    1: ("Slot label", {1: "label"}),
    2: ("Bitcoin key", {1: "key", 2: "label", 3: "flags", 4: "BIP-39 passphrase"}),
    3: ("Wallet", {1: "policy", 2: "name"}),
    4: ("Note", {1: "text"}),
    5: ("Recovery sheet", {1: "sheet"}),
    6: ("Entry", {1: "title", 2: "username", 3: "password", 4: "URL", 5: "notes", 6: "TOTP"}),
    7: ("GPG key", {1: "primary seed", 2: "created", 3: "subkey seed", 4: "subkey created", 5: "user ID", 6: "expiry", 7: "certificate"}),
    8: ("Secure Boot keys", {1: "owner GUID", 2: "PK key", 3: "PK certificate", 4: "KEK key", 5: "KEK certificate", 6: "db key", 7: "db certificate"}),
    9: ("Pending signing round", {1: "scheme", 2: "transaction", 3: "fingerprint", 4: "state", 5: "used"}),
    10: ("Signed amounts", {1: "transaction", 2: "amount"}),
}


# The fields that are UTF-8 text; every other field is printed in hex.
TEXT_FIELDS = {(1, 1), (2, 2), (2, 4), (3, 1), (3, 2), (4, 1)} | {(6, n) for n in range(1, 7)} | {(7, 5)}


def hkdf_sha256(ikm: bytes, info: bytes) -> bytes:
    """RFC 5869 with an empty salt, the first 32 bytes."""
    prk = hmac.new(b"\x00" * 32, ikm, hashlib.sha256).digest()
    return hmac.new(prk, info + b"\x01", hashlib.sha256).digest()


def open_vault(blob: bytes, passphrase: bytes) -> bytes:
    """The plaintext of the one slot `passphrase` opens."""
    if blob[:4] != MAGIC or len(blob) < HEADER_LEN or blob[4] != 1:
        raise SystemExit("not a Faraday vault of version 1")
    memory, passes, lanes, slot_len = struct.unpack_from("<4I", blob, 5)
    if slot_len not in SLOT_SIZES or len(blob) != HEADER_LEN + 4 * (slot_len + NONCE_LEN + TAG_LEN):
        raise SystemExit("the slot size or the file length is not one this format writes")
    if not (8192 <= memory <= 4 * 1024 * 1024 and 1 <= passes <= 10 and lanes == 1):
        raise SystemExit("the stated Argon2id cost is outside the format's limits")
    header = blob[:HEADER_LEN]
    salt = blob[21:53]
    key = hkdf_sha256(osk.argon2id(passphrase, salt, memory, passes, lanes), SLOT_INFO)
    opened = []
    for i in range(4):
        at = HEADER_LEN + i * (slot_len + NONCE_LEN + TAG_LEN)
        nonce = blob[at : at + NONCE_LEN]
        sealed = blob[at + NONCE_LEN : at + NONCE_LEN + slot_len + TAG_LEN]
        plain = osk.xchacha20poly1305_open(key, nonce, header + bytes([i]), sealed)
        if plain is not None:
            opened.append(plain)
    if not opened:
        raise SystemExit("that passphrase opens nothing in this vault")
    if len(opened) > 1:
        raise SystemExit("two slots opened under one passphrase; the file is not sound")
    return opened[0]


def records(plain: bytes) -> list[tuple[int, list[tuple[int, bytes]]]]:
    """The slot's records as (type, [(field, bytes)])."""
    if plain[0] != 1:
        raise SystemExit("a slot format this reader does not know")
    at, out = 1, []
    while True:
        kind = plain[at]
        at += 1
        if kind == 0:
            break
        if kind not in RECORDS:
            raise SystemExit(f"record type {kind} is not in docs/VAULT.md")
        (length,) = struct.unpack_from("<I", plain, at)
        at += 4
        body, at = plain[at : at + length], at + length
        fields, p = [], 0
        while p < len(body):
            number = body[p]
            (n,) = struct.unpack_from("<H", body, p + 1)
            fields.append((number, body[p + 3 : p + 3 + n]))
            p += 3 + n
        out.append((kind, fields))
    if any(plain[at:]):
        raise SystemExit("bytes after the end record")
    return out


def show(kind: int, number: int, value: bytes) -> str:
    """One field as text."""
    if kind == 2 and number == 1:
        if len(value) == 34:
            language, words = osk.words_of_payload(value)
            return f"{len(words)} words ({language}): " + " ".join(words)
        return "master seed " + value[1 : 1 + value[0]].hex()
    if kind == 5 and number == 1:
        parts, p = [], 0
        for _ in range(3):
            (n,) = struct.unpack_from("<H", value, p)
            parts.append(value[p + 2 : p + 2 + n].decode())
            p += 2 + n
        return f"descriptor {parts[0]} · name {parts[1]} · note {parts[2]}"
    if (kind, number) in TEXT_FIELDS:
        return value.decode()
    return value.hex()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("vault")
    parser.add_argument("--passphrase")
    args = parser.parse_args()
    blob = Path(args.vault).read_bytes()
    passphrase = args.passphrase if args.passphrase is not None else getpass.getpass("Passphrase: ")
    for kind, fields in records(open_vault(blob, passphrase.encode())):
        name, names = RECORDS[kind]
        print(name)
        for number, value in fields:
            print(f"  {names.get(number, number)}: {show(kind, number, value)}")


if __name__ == "__main__":
    sys.exit(main())
