#!/usr/bin/env python3
"""The primitives `tools/vault/open.py` reads a vault with: Argon2id and
XChaCha20-Poly1305 through two published libraries, and the BIP-39
wordlists read out of this repository's own copies
(`core/faraday-bip/src/wordlists/`), each checked against the SHA-256 of
the published file recorded beside it.

Dependencies, the standard library plus **either**:

  * `argon2-cffi` and `pynacl`  (pip install argon2-cffi pynacl), or
  * `cryptography` 44 or newer, built against OpenSSL 3.2 or newer,
    which is where its Argon2id comes from.

A words payload is 34 bytes (`docs/VAULT.md` §7): the word count, the
wordlist's index in the order below, and 32 bytes of entropy padded on
the right with zeros.
"""

from __future__ import annotations

import hashlib
import re
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORDLISTS = ROOT / "core" / "faraday-bip" / "src" / "wordlists"

WORDS_LEN = 34

# The order `Language::ALL` lists the wordlists in, which is the index
# the plaintext's second byte carries. The names are the module names
# under core/faraday-bip/src/wordlists/.
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



# ----- the words payload ----------------------------------------------------


def words_of_payload(payload: bytes) -> tuple[str, list[str]]:
    """The wordlist and the words a words payload holds."""
    if len(payload) != WORDS_LEN:
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
