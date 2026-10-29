#!/usr/bin/env python3
"""Writes `two-keys.fvlt`: a 64 KiB vault one of whose slots verifies
under the slot keys of two passphrases, the slot `docs/VAULT.md` §4's
commitment block refuses.

    python3 -I -B tools/vectors/vault/two_keys.py tools/vectors/vault

XChaCha20-Poly1305 does not commit to its key. Len, Grubbs and
Ristenpart ("Partitioning Oracle Attacks", USENIX Security 2021, §3)
make a ciphertext whose Poly1305 tag is the same under two keys by
solving for one free 16-byte block: the tag is
(h + s) mod 2^128 with h = sum of m_j r^(n - j) mod 2^130 - 5, so with
every block but one fixed, the free block's value x enters each key's h
as x r_k^e, and

    x (r_1^e - r_2^e) = (s_2 - s_1) - (B_1 - B_2)   (mod 2^130 - 5)

gives the x for which h_1 - h_2 = s_2 - s_1. A solution is a block when
2^128 <= x < 2^129 (the block's bytes and Poly1305's appended 1 bit)
and the two tags agree mod 2^128 (h_1 - h_2 not off by the prime); the
nonce is drawn again until both hold.

Here the slot is first sealed honestly under the first passphrase's key,
its plaintext a commitment block, a seal count and a slot label; then the
ciphertext's first block, which is the first half of the commitment block,
is replaced by the solution. Under the first key every byte after it
reads as a sound slot, so the commitment block alone refuses it; under the
second key the whole plaintext is keystream. PyNaCl's
XChaCha20-Poly1305 checks that the slot verifies under both keys before
anything is written.

Everything is fixed: the passphrases, the cost (the vectors' 8 MiB, one
pass), the salt, the other three slots and the nonces tried, each from
SHA-256 of a label. The magic, the header's layout and the slot key are
`tools/vault/open.py`'s, so this file follows the format's.
"""

from __future__ import annotations

import hashlib
import importlib.util
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

_spec = importlib.util.spec_from_file_location("vault_open", ROOT / "tools" / "vault" / "open.py")
ov = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(ov)

import nacl.bindings as sodium  # noqa: E402

NAME = "two-keys.fvlt"
PASSPHRASES = ("first of two keys", "second of two keys")
SLOT_LEN = 65536
COST = (8192, 1, 1)
POSITION = 2
P = (1 << 130) - 5
CLAMP = 0x0FFFFFFC0FFFFFFC0FFFFFFC0FFFFFFF


def filler(label: bytes, n: int) -> bytes:
    """`n` fixed bytes: SHA-256 of the label and a counter."""
    out = b""
    i = 0
    while len(out) < n:
        out += hashlib.sha256(b"two-keys " + label + struct.pack("<I", i)).digest()
        i += 1
    return out[:n]


def chacha20_block(key: bytes, counter: int, nonce12: bytes) -> bytes:
    """RFC 8439 §2.3: one 64-byte block."""
    m = 0xFFFFFFFF
    s = [0x61707865, 0x3320646E, 0x79622D32, 0x6B206574]
    s += list(struct.unpack("<8I", key)) + [counter] + list(struct.unpack("<3I", nonce12))
    w = list(s)

    def q(a, b, c, d):
        for x, y, z, r in ((a, b, d, 16), (c, d, b, 12), (a, b, d, 8), (c, d, b, 7)):
            w[x] = (w[x] + w[y]) & m
            w[z] ^= w[x]
            w[z] = ((w[z] << r) | (w[z] >> (32 - r))) & m

    for _ in range(10):
        q(0, 4, 8, 12); q(1, 5, 9, 13); q(2, 6, 10, 14); q(3, 7, 11, 15)  # noqa: E702
        q(0, 5, 10, 15); q(1, 6, 11, 12); q(2, 7, 8, 13); q(3, 4, 9, 14)  # noqa: E702
    return struct.pack("<16I", *[(w[i] + s[i]) & m for i in range(16)])


def poly_key(slot_key: bytes, nonce: bytes) -> tuple[int, int]:
    """XChaCha20-Poly1305's one-time key under `slot_key` and the 24-byte
    `nonce`: block 0 of ChaCha20 under HChaCha20's subkey, as (r, s)."""
    sub = ov.prim.hchacha20(slot_key, nonce[:16])
    k = chacha20_block(sub, 0, bytes(4) + nonce[16:])[:32]
    return int.from_bytes(k[:16], "little") & CLAMP, int.from_bytes(k[16:], "little")


def blocks(ad: bytes, ct: bytes) -> list[int]:
    """The AEAD's Poly1305 input (RFC 8439 §2.8) as blocks with their
    appended 1 bit: every block is full."""
    pad = lambda b: b + bytes(-len(b) % 16)  # noqa: E731
    data = pad(ad) + pad(ct) + struct.pack("<QQ", len(ad), len(ct))
    return [int.from_bytes(data[i:i + 16], "little") + (1 << 128) for i in range(0, len(data), 16)]


def horner(ms: list[int], r: int) -> int:
    h = 0
    for m in ms:
        h = (h + m) * r % P
    return h


def body() -> bytes:
    """A sound slot after its commitment block: format 1, the seal count
    1, a slot label, the end record and zeros."""
    def record(kind, fields):
        b = b"".join(bytes([n]) + struct.pack("<H", len(v)) + v for n, v in fields)
        return bytes([kind]) + struct.pack("<I", len(b)) + b
    out = bytes([1]) + record(16, [(1, struct.pack("<Q", 1))]) + record(1, [(1, b"Two keys")])
    out += bytes([0])
    return out + bytes(SLOT_LEN - 32 - len(out))


def make() -> bytes:
    salt = filler(b"salt", 32)
    header = ov.MAGIC + bytes([1]) + struct.pack("<4I", *COST, SLOT_LEN) + salt
    keys = [ov.hkdf_sha256(ov.prim.argon2id(p.encode(), salt, *COST), ov.SLOT_INFO)
            for p in PASSPHRASES]
    ad = header + bytes([POSITION])
    plain = bytes(32) + body()
    first = (len(ad) + 15) // 16
    for attempt in range(1000):
        nonce = filler(b"nonce %d" % attempt, 24)
        sealed = sodium.crypto_aead_xchacha20poly1305_ietf_encrypt(plain, ad, nonce, keys[0])
        ct = bytearray(sealed[:-16])
        (r1, s1), (r2, s2) = (poly_key(k, nonce) for k in keys)
        ms = blocks(ad, bytes(ct))
        if (horner(ms, r1) + s1) % (1 << 128) != int.from_bytes(sealed[-16:], "little"):
            raise SystemExit("this Poly1305 is not libsodium's")
        e = len(ms) - first
        ms[first] = 0
        b1, b2 = horner(ms, r1), horner(ms, r2)
        e1, e2 = pow(r1, e, P), pow(r2, e, P)
        if e1 == e2:
            continue
        x = ((s2 - s1) - (b1 - b2)) * pow(e1 - e2, -1, P) % P
        if not (1 << 128) <= x < (1 << 129):
            continue
        t1 = ((b1 + x * e1) % P + s1) % (1 << 128)
        t2 = ((b2 + x * e2) % P + s2) % (1 << 128)
        if t1 != t2:
            continue
        ct[:16] = (x - (1 << 128)).to_bytes(16, "little")
        tag = t1.to_bytes(16, "little")
        for k in keys:
            # Raises if the tag fails under either key.
            sodium.crypto_aead_xchacha20poly1305_ietf_decrypt(bytes(ct) + tag, ad, nonce, k)
        break
    else:
        raise SystemExit("no nonce gave a block")
    slots = bytearray(filler(b"slots", 4 * (SLOT_LEN + 40)))
    at = POSITION * (SLOT_LEN + 40)
    slots[at:at + SLOT_LEN + 40] = nonce + bytes(ct) + tag
    return header + bytes(slots)


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: two_keys.py DIRECTORY")
    path = Path(sys.argv[1]) / NAME
    path.write_bytes(make())
    print(f"{path} {path.stat().st_size} bytes")


if __name__ == "__main__":
    main()
