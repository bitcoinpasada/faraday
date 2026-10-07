#!/usr/bin/env python3
"""Decrypt an OpenSigner encrypted backup (`osk-backup`, versions 2 and 3).

The format is documented in
`docs/BACKUP.md` and `core/osk-backup/src/oskb.rs` and is not this app's
secret: these ninety lines and two published libraries read it. A person
who has the file and the passphrase does not need OpenSigner to get their
words back.

Usage:

    python3 tools/backup/decrypt.py BACKUP.oskb
    python3 tools/backup/decrypt.py BACKUP.oskb --passphrase PASSPHRASE

With no `--passphrase` the passphrase is read from the terminal without
echo. What the file holds is printed on standard output — the words of a
key, a master seed in hex, a note as its text, a recovery sheet as its
three parts; nothing is written back.

Dependencies — the standard library plus **either**:

  * `argon2-cffi` and `pynacl`  (pip install argon2-cffi pynacl), or
  * `cryptography` 44 or newer, built against OpenSSL 3.2 or newer,
    which is where its Argon2id comes from.

Whichever is installed is used; the first is tried first. The BIP-39
wordlists are read out of this repository's own copies
(`core/osk-bip/src/wordlists/`), and each one is checked against the
SHA-256 of the published file recorded beside it, so nothing is
downloaded and a tampered list is caught.

The layout is written down byte for byte in `docs/BACKUP.md`. In short,
all little-endian:

     0   magic "OSKB", 4 bytes
     4   version, 1 byte, 2 or 3
     5   Argon2id memory cost, u32, KiB
     9   Argon2id passes, u32
    13   Argon2id lanes, u32
    17   salt, 32 bytes                    -- 0..49 is the associated data
    49   nonce, 24 bytes
    73   ciphertext, then a 16-byte tag

The key is Argon2id(passphrase, salt) with a 32-byte output, the cipher
is XChaCha20-Poly1305, and the associated data is bytes 0..49, so a
header that was changed fails the tag.

A version-2 plaintext is 34 bytes and always words: the word count, the
wordlist's index in the order below, and 32 bytes of entropy padded on
the right with zeros. A version-3 plaintext is a kind byte, the
payload's length as a u16, the payload, and zero padding to the next
multiple of 256 bytes. The kinds are 1 words (the version-2 plaintext),
2 a master seed (its length, then 64 bytes zero-padded), 3 a note as
UTF-8 text, and 4 a recovery sheet: the descriptor, the wallet's name
and a note, each a u16 length and that many bytes, in that order.
"""

from __future__ import annotations

import argparse
import getpass
import hashlib
import re
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORDLISTS = ROOT / "core" / "osk-bip" / "src" / "wordlists"

MAGIC = b"OSKB"
VERSIONS = (2, 3)
HEADER_LEN = 49
NONCE_LEN = 24
TAG_LEN = 16
V2_PLAIN_LEN = 34
V2_BACKUP_LEN = HEADER_LEN + NONCE_LEN + V2_PLAIN_LEN + TAG_LEN
STEP = 256
TYPE_LEN = 3

KIND_WORDS = 1
KIND_SEED = 2
KIND_NOTE = 3
KIND_SHEET = 4

# The order `Language::ALL` lists the wordlists in, which is the index
# the plaintext's second byte carries. The names are the module names
# under core/osk-bip/src/wordlists/.
LANGUAGES = [
    "english",
    "japanese",
    "korean",
    "spanish",
    "chinese_simplified",
    "chinese_traditional",
    "french",
    "italian",
    "czech",
    "portuguese",
]


# ----- the two cryptographic primitives -------------------------------------


def argon2id(passphrase: bytes, salt: bytes, memory_kib: int, passes: int, lanes: int) -> bytes:
    """32 bytes of Argon2id, version 1.3."""
    try:
        from argon2.low_level import Type, hash_secret_raw
    except ImportError:
        pass
    else:
        return hash_secret_raw(
            secret=passphrase,
            salt=salt,
            time_cost=passes,
            memory_cost=memory_kib,
            parallelism=lanes,
            hash_len=32,
            type=Type.ID,
        )
    try:
        from cryptography.hazmat.primitives.kdf.argon2 import Argon2id
    except ImportError:
        raise SystemExit(
            "no Argon2id: install argon2-cffi, or cryptography 44+ on OpenSSL 3.2+"
        )
    return Argon2id(
        salt=salt, length=32, iterations=passes, lanes=lanes, memory_cost=memory_kib
    ).derive(passphrase)


def xchacha20poly1305_open(key: bytes, nonce: bytes, aad: bytes, sealed: bytes) -> bytes | None:
    """The plaintext of `sealed` (ciphertext ‖ tag), or None on a bad tag."""
    try:
        from nacl.bindings import crypto_aead_xchacha20poly1305_ietf_decrypt
    except ImportError:
        pass
    else:
        try:
            return crypto_aead_xchacha20poly1305_ietf_decrypt(sealed, aad, nonce, key)
        except Exception:
            return None
    try:
        from cryptography.exceptions import InvalidTag
        from cryptography.hazmat.primitives.ciphers.aead import ChaCha20Poly1305
    except ImportError:
        raise SystemExit("no XChaCha20-Poly1305: install pynacl, or cryptography")
    # XChaCha20 is ChaCha20 under a subkey: HChaCha20 of the first 16
    # nonce bytes gives the key, and the IETF nonce is four zero bytes
    # and the last eight (draft-irtf-cfrg-xchacha, §2.3).
    subkey = hchacha20(key, nonce[:16])
    try:
        return ChaCha20Poly1305(subkey).decrypt(b"\x00" * 4 + nonce[16:], sealed, aad)
    except InvalidTag:
        return None


def hchacha20(key: bytes, nonce16: bytes) -> bytes:
    """HChaCha20: twenty ChaCha rounds, no feed-forward, first and last rows."""
    state = [0x61707865, 0x3320646E, 0x79622D32, 0x6B206574]
    state += list(struct.unpack("<8I", key))
    state += list(struct.unpack("<4I", nonce16))

    def rot(v, n):
        return ((v << n) | (v >> (32 - n))) & 0xFFFFFFFF

    def quarter(s, a, b, c, d):
        s[a] = (s[a] + s[b]) & 0xFFFFFFFF
        s[d] = rot(s[d] ^ s[a], 16)
        s[c] = (s[c] + s[d]) & 0xFFFFFFFF
        s[b] = rot(s[b] ^ s[c], 12)
        s[a] = (s[a] + s[b]) & 0xFFFFFFFF
        s[d] = rot(s[d] ^ s[a], 8)
        s[c] = (s[c] + s[d]) & 0xFFFFFFFF
        s[b] = rot(s[b] ^ s[c], 7)

    for _ in range(10):
        quarter(state, 0, 4, 8, 12)
        quarter(state, 1, 5, 9, 13)
        quarter(state, 2, 6, 10, 14)
        quarter(state, 3, 7, 11, 15)
        quarter(state, 0, 5, 10, 15)
        quarter(state, 1, 6, 11, 12)
        quarter(state, 2, 7, 8, 13)
        quarter(state, 3, 4, 9, 14)
    return struct.pack("<8I", *(state[:4] + state[12:]))


# ----- BIP-39 ---------------------------------------------------------------


def unescape(literal: str) -> str:
    """A Rust string literal's text: the generator writes non-ASCII as
    `\\u{…}` so the modules stay ASCII."""
    return re.sub(r"\\u\{([0-9a-fA-F]+)\}", lambda m: chr(int(m.group(1), 16)), literal)


def wordlist_of(language: str) -> list[str]:
    """The 2048 words of `language`, checked against their digest."""
    path = WORDLISTS / f"{language}.rs"
    if not path.is_file():
        raise SystemExit(f"no wordlist at {path}")
    text = path.read_text(encoding="utf-8")
    digest = re.search(r'pub const SHA256: &str = "([0-9a-f]{64})";', text)
    body = re.search(r"pub static WORDS: \[&str; 2048\] = \[\n(.*?)\n\];", text, re.S)
    if digest is None or body is None:
        raise SystemExit(f"{path}: not a generated wordlist module")
    words = [unescape(w) for w in re.findall(r'^\s*"(.*)",$', body.group(1), re.M)]
    if len(words) != 2048:
        raise SystemExit(f"{path}: {len(words)} words, expected 2048")
    joined = "".join(w + "\n" for w in words).encode("utf-8")
    if hashlib.sha256(joined).hexdigest() != digest.group(1):
        raise SystemExit(f"{path}: the words do not match the recorded SHA-256")
    return words


def words_of(entropy: bytes, language: str) -> list[str]:
    """The BIP-39 sentence for `entropy` in `language`."""
    wordlist = wordlist_of(language)
    checksum = hashlib.sha256(entropy).digest()[0] >> (8 - len(entropy) * 8 // 32)
    bits = int.from_bytes(entropy, "big")
    bits = (bits << (len(entropy) * 8 // 32)) | checksum
    count = len(entropy) * 3 // 4
    return [wordlist[(bits >> (11 * (count - 1 - i))) & 0x7FF] for i in range(count)]


# ----- the backup -----------------------------------------------------------


def decrypt(blob: bytes, passphrase: bytes) -> tuple[str, str]:
    """`(kind, text)` of `blob` under `passphrase`, ready to print."""
    if blob[:4] != MAGIC:
        raise SystemExit("not an osk-backup: wrong magic")
    version = blob[4]
    if version not in VERSIONS:
        raise SystemExit(f"osk-backup version {version}, this script reads {VERSIONS}")
    sealed_len = len(blob) - HEADER_LEN - NONCE_LEN - TAG_LEN
    if version == 2:
        if len(blob) != V2_BACKUP_LEN:
            raise SystemExit(f"{len(blob)} bytes is not the length of a version-2 backup")
    elif sealed_len <= 0 or sealed_len % STEP:
        raise SystemExit(f"{len(blob)} bytes is not the length of a version-3 backup")
    memory_kib, passes, lanes = struct.unpack("<III", blob[5:17])
    salt = blob[17:HEADER_LEN]
    nonce = blob[HEADER_LEN : HEADER_LEN + NONCE_LEN]
    sealed = blob[HEADER_LEN + NONCE_LEN :]

    key = argon2id(passphrase, salt, memory_kib, passes, lanes)
    plain = xchacha20poly1305_open(key, nonce, blob[:HEADER_LEN], sealed)
    if plain is None:
        raise SystemExit("wrong passphrase")

    if version == 2:
        return "words", " ".join(words_of_payload(plain)[1])

    kind = plain[0]
    length = int.from_bytes(plain[1:3], "little")
    if length > len(plain) - TYPE_LEN:
        raise SystemExit("the payload is longer than the plaintext")
    if any(plain[TYPE_LEN + length :]):
        raise SystemExit("the padding after the payload is not zero")
    payload = plain[TYPE_LEN : TYPE_LEN + length]

    if kind == KIND_WORDS:
        language, words = words_of_payload(payload)
        print(language, file=sys.stderr)
        return "words", " ".join(words)
    if kind == KIND_SEED:
        n = payload[0]
        if not 16 <= n <= 64 or len(payload) != 65 or any(payload[1 + n :]):
            raise SystemExit("not a master seed")
        return "seed", payload[1 : 1 + n].hex()
    if kind == KIND_NOTE:
        return "note", payload.decode("utf-8")
    if kind == KIND_SHEET:
        parts, at = [], 0
        for _ in range(3):
            n = int.from_bytes(payload[at : at + 2], "little")
            at += 2
            if at + n > len(payload):
                raise SystemExit("a recovery sheet's field runs past its payload")
            parts.append(payload[at : at + n].decode("utf-8"))
            at += n
        if at != len(payload):
            raise SystemExit("a recovery sheet has bytes after its three fields")
        descriptor, name, note = parts
        return "sheet", f"descriptor: {descriptor}\nname: {name}\nnote: {note}"
    raise SystemExit(f"unknown payload kind {kind}")


def words_of_payload(payload: bytes) -> tuple[str, list[str]]:
    """The wordlist and the words a words payload holds."""
    if len(payload) != V2_PLAIN_LEN:
        raise SystemExit("a words payload is 34 bytes")
    count, language_index = payload[0], payload[1]
    if language_index >= len(LANGUAGES):
        raise SystemExit(f"unknown wordlist index {language_index}")
    if count % 3 or not 12 <= count <= 24:
        raise SystemExit(f"{count} is not a BIP-39 word count")
    entropy, pad = payload[2 : 2 + count * 4 // 3], payload[2 + count * 4 // 3 :]
    if any(pad):
        raise SystemExit("the padding after the entropy is not zero")
    language = LANGUAGES[language_index]
    return language, words_of(entropy, language)


def main() -> None:
    parser = argparse.ArgumentParser(description="Decrypt an OpenSigner encrypted backup.")
    parser.add_argument("backup", type=Path, help="the .oskb file")
    parser.add_argument(
        "--passphrase",
        help="the backup passphrase; read from the terminal without echo when absent",
    )
    args = parser.parse_args()

    blob = args.backup.read_bytes()
    passphrase = args.passphrase
    if passphrase is None:
        passphrase = getpass.getpass("Backup passphrase: ")
    kind, text = decrypt(blob, passphrase.encode("utf-8"))
    print(kind, file=sys.stderr)
    print(text)


if __name__ == "__main__":
    main()
