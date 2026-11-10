#!/usr/bin/env python3
"""Makes the vector for faraday/faraday-vault/tests/keyfile_write.rs
(docs/VAULT.md §3.2): a KeePass XML key file version 2.0 of fixed bytes,
laid out as KeePassXC 2.7.10 writes one, and a KDBX 4 file made with
pykeepass under a passphrase and that key file, holding one entry.

    keyfile_write.py OUT_DIR

writes OUT_DIR/faraday-key-file.keyx and OUT_DIR/pykeepass-key-file.kdbx.
The test checks that Faraday's writer gives the .keyx byte for byte and that
the vault opens. The key is bytes 3, 8, 13, ... (i * 5 + 3).
"""
import hashlib
import os
import sys

from pykeepass import create_database

PASSPHRASE = "key file writer passphrase"
key = bytes(i * 5 + 3 for i in range(32))
hexed = key.hex().upper()
lines = [" ".join(hexed[j : j + 8] for j in range(i, i + 32, 8)) for i in range(0, 64, 32)]
tag = hashlib.sha256(key).digest()[:4].hex().upper()
text = (
    '<?xml version="1.0" encoding="UTF-8"?>\n<KeyFile>\n    <Meta>\n'
    "        <Version>2.0</Version>\n    </Meta>\n    <Key>\n"
    f'        <Data Hash="{tag}">\n'
    + "".join(f"            {line}\n" for line in lines)
    + "        </Data>\n    </Key>\n</KeyFile>\n"
)
out = sys.argv[1]
keyx = os.path.join(out, "faraday-key-file.keyx")
with open(keyx, "w", newline="") as f:
    f.write(text)
db = os.path.join(out, "pykeepass-key-file.kdbx")
kp = create_database(db, password=PASSPHRASE, keyfile=keyx)
kp.add_entry(kp.root_group, "entry-a", "user-a", "pass-a")
kp.save()
