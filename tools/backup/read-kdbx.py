#!/usr/bin/env python3
"""Open a KDBX 4 database OpenSigner wrote and print what is in it.

OpenSigner writes KDBX 4 so that an heir opens an export in a KeePass
app with no knowledge of this project (`docs/PLANNING.md` §16.112). This
script is the check that it really is KDBX: it opens the file the way
another implementation would and prints every entry's title, notes and
password.

Usage:

    python3 tools/backup/read-kdbx.py FILE.kdbx
    python3 tools/backup/read-kdbx.py FILE.kdbx --passphrase PASSPHRASE

With no `--passphrase` the passphrase is read from the terminal without
echo. Nothing is written back.

Two readers, in this order:

  * `pykeepass` (pip install pykeepass), which is the library KeePass
    tooling uses and knows nothing about this repository. Where it is
    installed, it is what runs, and its agreement is the real check.
  * a reader in this file, over `cryptography` 44 or newer (for
    Argon2id and ChaCha20) or `argon2-cffi` plus `cryptography`. It
    parses the format from KeePass's own published description, a copy
    of which is in `tools/reference/kdbx4/`. It is a second
    implementation of the same reading, not an independent one: it was
    written against the same pages as the writer, so it catches a file
    that does not hold together and would not catch a place where both
    read the specification the same wrong way. `pykeepass` is what
    catches that.

Which reader ran is printed on standard error.

The format, all little-endian:

     0   signature 1, u32 = 0x9AA2D903
     4   signature 2, u32 = 0xB54BFB67
     8   version, u32; the high word is the major version
    12   header fields, each an id byte, a u32 length and that many
         bytes: 2 cipher UUID, 3 compression, 4 master seed, 7
         encryption IV, 11 the KDF parameters, 0 end of header
    ..   SHA-256 of the header, 32 bytes
    ..   HMAC-SHA-256 of the header, 32 bytes
    ..   the HMAC block stream: each block a 32-byte HMAC, a u32 length
         and that many bytes of ciphertext, ending with a zero-length
         block

and the keys:

    R = SHA-256(SHA-256(passphrase))
    T = Argon2id(R, salt, cost)
    encryption key = SHA-256(master_seed ‖ T)
    base           = SHA-512(master_seed ‖ T ‖ 0x01)
    the header's HMAC key is SHA-512(0xFFFFFFFFFFFFFFFF ‖ base)
    block i's is    SHA-512(i ‖ base)

The body is the inner header — 1 the stream cipher's id, 2 its key, 0
the end — and one XML document. A `Value` marked `Protected="True"` is
Base64 of bytes encrypted with the inner stream, which is ChaCha20
under SHA-512(key)[0:32] with the nonce SHA-512(key)[32:44], one
running keystream over every protected value in document order.
"""

from __future__ import annotations

import argparse
import base64
import getpass
import hashlib
import hmac
import struct
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

SIGNATURE = bytes.fromhex("03d9a29a67fb4bb5")
CIPHER_CHACHA20 = bytes.fromhex("d6038a2b8b6f4cb5a524339a31dbb59a")
KDF_ARGON2D = bytes.fromhex("ef636ddf8c29444b91f7a9a403e30a0c")
KDF_ARGON2ID = bytes.fromhex("9e298b1956db4773b23dfc3ec6f0a1e6")

HERE = Path(__file__).resolve().parent


# ----- the reader in this file ----------------------------------------------


def argon2id(password: bytes, salt: bytes, memory_kib: int, passes: int, lanes: int) -> bytes:
    """32 bytes of Argon2id 1.3, through whichever library is here."""
    sys.path.insert(0, str(HERE))
    import decrypt

    return decrypt.argon2id(password, salt, memory_kib, passes, lanes)


def chacha20(key: bytes, nonce: bytes, data: bytes) -> bytes:
    """ChaCha20 (RFC 8439) over `data`, counter starting at zero."""
    from cryptography.hazmat.primitives.ciphers import Cipher, algorithms

    # `cryptography` takes a 16-byte nonce: the 4-byte counter first,
    # then the 12 bytes the file carries.
    full = (0).to_bytes(4, "little") + nonce
    cipher = Cipher(algorithms.ChaCha20(key, full), mode=None).encryptor()
    return cipher.update(data) + cipher.finalize()


def variant_dictionary(blob: bytes) -> dict[str, tuple[int, bytes]]:
    """A KDBX VariantDictionary as {name: (type, value)}."""
    if len(blob) < 3 or blob[1] != 1:
        raise SystemExit("not a VariantDictionary version 1.x")
    out: dict[str, tuple[int, bytes]] = {}
    at = 2
    while at < len(blob) and blob[at] != 0:
        kind = blob[at]
        at += 1
        (n,) = struct.unpack_from("<I", blob, at)
        at += 4
        name = blob[at : at + n].decode()
        at += n
        (v,) = struct.unpack_from("<I", blob, at)
        at += 4
        out[name] = (kind, blob[at : at + v])
        at += v
    return out


def read_header(data: bytes) -> tuple[int, dict[int, bytes]]:
    """The header's length and its fields."""
    if data[:8] != SIGNATURE:
        raise SystemExit("not a KDBX file")
    (version,) = struct.unpack_from("<I", data, 8)
    if version >> 16 != 4:
        raise SystemExit(f"not KDBX 4 (version {version >> 16}.{version & 0xFFFF})")
    fields: dict[int, bytes] = {}
    at = 12
    while True:
        ident = data[at]
        (n,) = struct.unpack_from("<I", data, at + 1)
        at += 5
        fields[ident] = data[at : at + n]
        at += n
        if ident == 0:
            return at, fields


def keys(master_seed: bytes, transformed: bytes) -> tuple[bytes, bytes]:
    """The encryption key and the base every HMAC key comes from."""
    encryption = hashlib.sha256(master_seed + transformed).digest()
    base = hashlib.sha512(master_seed + transformed + b"\x01").digest()
    return encryption, base


def block_key(base: bytes, i: int) -> bytes:
    return hashlib.sha512(struct.pack("<Q", i) + base).digest()


def read_blocks(data: bytes, base: bytes) -> bytes:
    """The ciphertext of the HMAC block stream, every block checked."""
    out = b""
    at = 0
    i = 0
    while True:
        tag = data[at : at + 32]
        (n,) = struct.unpack_from("<I", data, at + 32)
        at += 36
        block = data[at : at + n]
        at += n
        want = hmac.new(
            block_key(base, i), struct.pack("<Q", i) + struct.pack("<I", n) + block, hashlib.sha256
        ).digest()
        if not hmac.compare_digest(tag, want):
            raise SystemExit(f"block {i} fails its HMAC")
        if n == 0:
            if at != len(data):
                raise SystemExit("bytes after the last block")
            return out
        out += block
        i += 1


def read_inner_header(body: bytes) -> tuple[int, bytes, int]:
    """The inner stream's id and key, and where the XML starts."""
    at = 0
    stream_id = 0
    stream_key = b""
    while True:
        ident = body[at]
        (n,) = struct.unpack_from("<I", body, at + 1)
        at += 5
        value = body[at : at + n]
        at += n
        if ident == 0:
            return stream_id, stream_key, at
        if ident == 1:
            (stream_id,) = struct.unpack("<I", value)
        elif ident == 2:
            stream_key = value


def _entries(xml: bytes, plain: bytes) -> list[dict[str, str]]:
    """The entries of `xml`, with protected values replaced in order."""
    root = ET.fromstring(xml)
    values = [v for v in root.iter("Value") if v.get("Protected") == "True"]
    at = 0
    for value in values:
        n = len(base64.b64decode(value.text or ""))
        value.text = plain[at : at + n].decode()
        at += n
    out = []
    for entry in root.iter("Entry"):
        fields: dict[str, str] = {}
        for string in entry.findall("String"):
            key = string.findtext("Key") or ""
            fields[key] = string.findtext("Value") or ""
        out.append(fields)
    return out


def read_here(path: Path, passphrase: bytes) -> list[dict[str, str]]:
    """The entries of `path`, read by this file."""
    data = path.read_bytes()
    header_len, fields = read_header(data)
    if fields.get(2) != CIPHER_CHACHA20:
        raise SystemExit("the file's cipher is not ChaCha20")
    if fields.get(3, b"\x00\x00\x00\x00") != b"\x00\x00\x00\x00":
        raise SystemExit("the file is compressed; this writer never compresses")
    master_seed = fields[4]
    iv = fields[7]
    kdf = variant_dictionary(fields[11])
    uuid = kdf["$UUID"][1]
    if uuid not in (KDF_ARGON2ID, KDF_ARGON2D):
        raise SystemExit("the file's KDF is not Argon2")
    if uuid != KDF_ARGON2ID:
        raise SystemExit("the file's KDF is Argon2d, and this writer uses Argon2id")
    salt = kdf["S"][1]
    passes = struct.unpack("<Q", kdf["I"][1])[0]
    memory_kib = struct.unpack("<Q", kdf["M"][1])[0] // 1024
    lanes = struct.unpack("<I", kdf["P"][1])[0]

    stated = data[header_len : header_len + 32]
    if hashlib.sha256(data[:header_len]).digest() != stated:
        raise SystemExit("the header's SHA-256 does not match")

    composite = hashlib.sha256(hashlib.sha256(passphrase).digest()).digest()
    transformed = argon2id(composite, salt, memory_kib, passes, lanes)
    encryption, base = keys(master_seed, transformed)

    want = hmac.new(
        block_key(base, 0xFFFF_FFFF_FFFF_FFFF), data[:header_len], hashlib.sha256
    ).digest()
    if not hmac.compare_digest(data[header_len + 32 : header_len + 64], want):
        raise SystemExit("the header's HMAC does not match: wrong passphrase, or a changed file")

    body = chacha20(encryption, iv, read_blocks(data[header_len + 64 :], base))
    stream_id, stream_key, at = read_inner_header(body)
    if stream_id != 3:
        raise SystemExit(f"inner stream {stream_id} is not ChaCha20")
    wide = hashlib.sha512(stream_key).digest()
    xml = body[at:]
    root = ET.fromstring(xml)
    values = [v for v in root.iter("Value") if v.get("Protected") == "True"]
    blob = b"".join(base64.b64decode(v.text or "") for v in values)
    plain = chacha20(wide[:32], wide[32:44], blob)
    return _entries(xml, plain)


# ----- pykeepass -------------------------------------------------------------


def read_with_pykeepass(path: Path, passphrase: str) -> list[dict[str, str]] | None:
    """The entries as `pykeepass` sees them, or None where it is absent."""
    try:
        from pykeepass import PyKeePass
    except ImportError:
        return None
    db = PyKeePass(str(path), password=passphrase)
    return [
        {
            "Title": entry.title or "",
            "Password": entry.password or "",
            "Notes": entry.notes or "",
        }
        for entry in db.entries
    ]


# ----- the command line ------------------------------------------------------


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("file", type=Path, help="the .kdbx file")
    parser.add_argument("--passphrase", help="read from the terminal when absent")
    parser.add_argument(
        "--here",
        action="store_true",
        help="use this file's reader even where pykeepass is installed",
    )
    args = parser.parse_args()

    passphrase = args.passphrase
    if passphrase is None:
        passphrase = getpass.getpass("Passphrase: ")

    entries = None if args.here else read_with_pykeepass(args.file, passphrase)
    if entries is None:
        entries = read_here(args.file, passphrase.encode())
        print("read by tools/backup/read-kdbx.py", file=sys.stderr)
    else:
        print("read by pykeepass", file=sys.stderr)

    for i, entry in enumerate(entries, start=1):
        print(f"--- entry {i}")
        print(f"title: {entry.get('Title', '')}")
        print(f"password: {entry.get('Password', '')}")
        print("notes:")
        print(entry.get("Notes", ""))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
