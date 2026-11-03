#!/usr/bin/env python3
"""Makes the KDBX files that faraday-vault's tests read (tools/vectors/kdbx/).

Every file is written by pykeepass, a KDBX implementation Faraday's
authors did not write, at the lowest cost Faraday's reader accepts
(Argon2 with 8 MiB of memory) under the passphrase below, holding one
entry whose password is `hunter2`. Usage:

    python3 -I -B tools/vectors/kdbx/make.py OUT_DIR

It needs pykeepass (4.1 or later) and its construct library.
"""
import hashlib
import os
import sys

import argon2

from pykeepass import PyKeePass, create_database

PASSPHRASE = "kdbx vector passphrase"
ARGON2ID = b"\x9e\x29\x8b\x19\x56\xdb\x47\x73\xb2\x3d\xfc\x3e\xc6\xf0\xa1\xe6"


# The large file's attachment: more than one block of the HMAC stream
# (1 MiB), bytes anyone can make again.
ATTACHMENT_LEN = 1200 * 1024


def attachment():
    return hashlib.shake_256(b"kdbx vector attachment").digest(ATTACHMENT_LEN)


def make(path, cipher="aes256", argon2id=False, gzip=True, minor=0, attach=False):
    kp = create_database(path, password=PASSPHRASE)
    entry = kp.add_entry(kp.root_group, "Example", "alice", "hunter2", url="https://example.org/")
    if attach:
        entry.add_attachment(kp.add_binary(attachment()), "attachment.bin")
    header = kp.kdbx.header.value
    dyn = header.dynamic_header
    dyn.cipher_id.data = cipher
    if cipher == "chacha20":
        dyn.encryption_iv.data = os.urandom(12)
    elif cipher == "twofish":
        dyn.encryption_iv.data = os.urandom(16)
    dyn.compression_flags.data.compression = gzip
    params = dyn.kdf_parameters.data.dict
    params["M"].value = 8 * 1024 * 1024
    params["I"].value = 2
    params["P"].value = 2
    if argon2id:
        params["$UUID"].value = ARGON2ID
    header.minor_version = minor
    # pykeepass keeps the transformed key it derived for the cost it made
    # the database at, so the new cost's is passed in.
    key = hashlib.sha256(hashlib.sha256(PASSPHRASE.encode()).digest()).digest()
    transformed = argon2.low_level.hash_secret_raw(
        secret=key,
        salt=params["S"].value,
        time_cost=params["I"].value,
        memory_cost=params["M"].value // 1024,
        parallelism=params["P"].value,
        hash_len=32,
        type=argon2.low_level.Type.ID if argon2id else argon2.low_level.Type.D,
        version=0x13,
    )
    # The header is built from its parsed bytes unless they are dropped.
    for field in ("data", "length", "offset1", "offset2"):
        kp.kdbx.header.pop(field, None)
    kp.save(transformed_key=transformed)
    PyKeePass(path, password=PASSPHRASE)  # reads back under pykeepass


def main():
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    for name, kw in [
        ("aes-argon2d-gzip", {}),
        ("aes-argon2id-gzip", {"argon2id": True}),
        ("chacha20-argon2d-gzip", {"cipher": "chacha20"}),
        ("chacha20-argon2id-gzip", {"cipher": "chacha20", "argon2id": True}),
        ("chacha20-argon2id-plain", {"cipher": "chacha20", "argon2id": True, "gzip": False}),
        ("aes-argon2id-gzip-v41", {"argon2id": True, "minor": 1}),
        ("twofish-argon2d-gzip", {"cipher": "twofish"}),
        (
            "chacha20-argon2id-plain-large",
            {"cipher": "chacha20", "argon2id": True, "gzip": False, "attach": True},
        ),
    ]:
        try:
            make(out + "/pykeepass-" + name + ".kdbx", **kw)
            print("made", name)
        except Exception as e:
            print("FAILED", name, e)


main()
