#!/usr/bin/env python3
"""Makes and reads a KDBX 4 file that needs a key file or a YubiKey's secret,
with pykeepass, for faraday/faraday-vault/tests/keyed.rs (docs/VAULT.md §3.2).

    keyed.py make OUT PASSPHRASE [--keyfile PATH] [--secret HEX40] [--key HEX64]
    keyed.py read FILE PASSPHRASE [--keyfile PATH] [--secret HEX40] [--key HEX64]

A key file goes to pykeepass as it is. A secret has no support in pykeepass:
the response is worked out here, as KeePassXC does (the Argon2 salt padded to
64 bytes, HMAC-SHA-1 over it less the padding, the composite key
SHA-256(SHA-256(passphrase) || key || SHA-256(response))), and the file
is made and read under the key stretched from it. `--key` is the key file's
32 bytes when a secret goes with a key file.

`read` prints `title<TAB>username<TAB>password` for each entry.
"""
import argparse
import hashlib
import hmac
import os
import sys

from argon2.low_level import Type, hash_secret_raw
from pykeepass import PyKeePass, create_database

ap = argparse.ArgumentParser()
ap.add_argument("mode")
ap.add_argument("path")
ap.add_argument("passphrase")
ap.add_argument("--keyfile")
ap.add_argument("--secret")
ap.add_argument("--key", default="")
a = ap.parse_args()


def stretched(kd, passphrase):
    """The transformed key of a file whose KDF parameters are `kd`, under
    the passphrase, the key and the response for the salt `kd` holds."""
    salt = kd["S"].value if hasattr(kd["S"], "value") else kd["S"]
    padded = salt + bytes([64 - len(salt)]) * (64 - len(salt))
    n = 64
    while n and padded[n - 1] == padded[63]:
        n -= 1
    response = hmac.new(bytes.fromhex(a.secret), padded[:n], "sha1").digest()
    composite = hashlib.sha256(
        hashlib.sha256(passphrase.encode()).digest()
        + bytes.fromhex(a.key)
        + hashlib.sha256(response).digest()
    ).digest()
    kind = Type.ID if kd["$UUID"].value.hex().startswith("9e29") else Type.D
    return hash_secret_raw(
        composite,
        salt,
        kd["I"].value,
        kd["M"].value // 1024,
        kd["P"].value,
        32,
        kind,
        version=kd["V"].value,
    )


def kdf(kp):
    return kp.kdbx.header.value.dynamic_header.kdf_parameters.data.dict


if a.mode == "make":
    tmp = a.path + ".tmp"
    kp = create_database(tmp, password="tmp")
    kp.add_entry(kp.root_group, "entry-a", "user-a", "pass-a")
    kp.save()
    if a.secret:
        kp = PyKeePass(tmp, password="tmp")
        kd = kdf(kp)
        # The file keeps the salt it was made with: pykeepass leaves it
        # alone when it is given the transformed key.
        kp.save(a.path, transformed_key=stretched(kd, a.passphrase))
    else:
        # The file is remade under the real credentials from its entries.
        kp = create_database(a.path, password=a.passphrase, keyfile=a.keyfile)
        kp.add_entry(kp.root_group, "entry-a", "user-a", "pass-a")
        kp.save()
    os.remove(tmp)
else:
    if a.secret:
        # The salt is in the clear header: read it without decrypting.
        kd = kdf(PyKeePass(a.path, password="x", decrypt=False))
        kp = PyKeePass(a.path, password=a.passphrase, transformed_key=stretched(kd, a.passphrase))
    else:
        kp = PyKeePass(a.path, password=a.passphrase, keyfile=a.keyfile)
    for e in kp.entries:
        print("\t".join([e.title or "", e.username or "", e.password or ""]))
