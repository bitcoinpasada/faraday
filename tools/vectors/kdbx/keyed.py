#!/usr/bin/env python3
"""Makes and reads a KDBX 4 file that needs a key file, with pykeepass, for
faraday/faraday-vault/tests/keyed.rs (docs/VAULT.md §3.2).

    keyed.py make OUT PASSPHRASE [--keyfile PATH]
    keyed.py read FILE PASSPHRASE [--keyfile PATH]

`read` prints `title<TAB>username<TAB>password` for each entry.
"""
import argparse

from pykeepass import PyKeePass, create_database

ap = argparse.ArgumentParser()
ap.add_argument("mode")
ap.add_argument("path")
ap.add_argument("passphrase")
ap.add_argument("--keyfile")
a = ap.parse_args()

if a.mode == "make":
    kp = create_database(a.path, password=a.passphrase, keyfile=a.keyfile)
    kp.add_entry(kp.root_group, "entry-a", "user-a", "pass-a")
    kp.save()
else:
    kp = PyKeePass(a.path, password=a.passphrase, keyfile=a.keyfile)
    for e in kp.entries:
        print("\t".join([e.title or "", e.username or "", e.password or ""]))
