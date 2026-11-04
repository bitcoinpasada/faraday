#!/usr/bin/env python3
"""Open a Faraday file vault with pykeepass and print what it holds.

A file vault is a KDBX 4 database (`docs/VAULT.md` §1.1, §7). It is not
Faraday's secret: this file and a published library read it, so a person
with the file and a passphrase does not need Faraday to get their keys,
wallets, notes and entries back. pykeepass is the distribution's package
(Debian and Ubuntu: `python3-pykeepass`; Fedora: `python3-pykeepass`;
Arch: `python-pykeepass`).

Usage:

    python3 tools/vault/open.py vault.kdbx
    python3 tools/vault/open.py vault.kdbx --passphrase PASSPHRASE
    python3 tools/vault/open.py vault.kdbx --field gpg.key --out gpg-key.asc

With no `--passphrase` it is read from the terminal without echo, or from
one line of standard input where that is not a terminal. Type it at the
prompt: an argument stays in the shell's history, so `--field` refuses
`--passphrase`. Everything the vault holds is printed on standard output,
secrets included; nothing is written. The passphrase is put in Unicode
NFC first, as Faraday does (§3); KeePass apps do not.

Each item is printed under §7's names: a line with its kind (`seed`,
`wallet`, `note`, `sheet`, `gpg`, `secure_boot`, `wallet_share`,
`passphrase`, `share`, or `entry` for a person's own entry), its group and
its UUID, then its fields and custom data as the file holds them, in the
order the file has them. The vault's own values come first: its label, its
identifier (the root group's UUID), `faraday.format`, `faraday.seals`,
`faraday.sealed`, and each `faraday.signed.{tx}` and `faraday.plan.{n}`.

`--field KIND.FIELD --out FILE` prints nothing of the vault and writes one
value's bytes, as they are, to a new file created with mode 0600 (an
existing file is refused). It is how a secret value (a GPG key's armored
secret key, `gpg.key`) reaches another tool without Faraday
(`docs/VAULT.md` §10). Where the vault holds several items of that kind,
`--index N` takes the N-th in the order they are printed, counting from 1;
without it the command refuses and says how many there are.

A QR vault is an OpenPGP message and is not read here: the QR vault is not
built.
"""

from __future__ import annotations

import argparse
import getpass
import os
import struct
import sys
import unicodedata
from pathlib import Path

try:
    from pykeepass import PyKeePass
except ImportError as e:
    raise SystemExit(
        "open.py needs the distribution's pykeepass (Debian and Ubuntu: python3-pykeepass;"
        f" Fedora: python3-pykeepass; Arch: python-pykeepass): {e}") from e

SIGNATURES = (0x9AA2D903, 0xB54BFB67)
KINDS = ("seed", "wallet", "note", "sheet", "gpg", "secure_boot", "wallet_share", "passphrase", "share")


def read_file(path: str) -> bytes:
    data = Path(path).read_bytes()
    if len(data) < 12 or struct.unpack_from("<2I", data, 0) != SIGNATURES:
        # An OpenPGP packet begins with a byte whose top bit is set.
        if data[:1] and data[0] & 0x80:
            raise SystemExit("A QR vault is not read here: the QR vault is not built.")
        raise SystemExit("not a KeePass file")
    minor, major = struct.unpack_from("<2H", data, 8)
    if major < 4:
        raise SystemExit("A KeePass file of an older version: KDBX 4 is the least this reads")
    return data


def text(element) -> str:
    return element.text or ""


def fields_of(entry) -> list[tuple[str, str]]:
    """An entry's fields (pykeepass has decrypted the protected ones) and
    custom data, in file order."""
    out = []
    for s in entry._element.xpath("String"):
        out.append((text(s.find("Key")), text(s.find("Value"))))
    for i in entry._element.xpath("CustomData/Item"):
        out.append((text(i.find("Key")), text(i.find("Value"))))
    return out


def kind_of(fields: list[tuple[str, str]]) -> str:
    for key, value in fields:
        if key == "faraday.kind":
            return value
    return "entry"


def items(kp) -> list[tuple[str, str, str, list[tuple[str, str]]]]:
    """Each entry as (kind, group path, UUID, fields)."""
    out = []
    for entry in kp.entries:
        fields = fields_of(entry)
        path = "/".join(g for g in (entry.path[:-1] if entry.path else []) if g)
        out.append((kind_of(fields), path, entry.uuid.hex, fields))
    return out


def database_values(kp) -> list[tuple[str, str]]:
    return [(text(i.find("Key")), text(i.find("Value")))
            for i in kp.tree.xpath("/KeePassFile/Meta/CustomData/Item")]


def print_vault(kp) -> None:
    print(f"Vault: {kp.database_name or ''}")
    print(f"  id: {kp.root_group.uuid.hex}")
    for key, value in database_values(kp):
        print(f"  {key}: {value}")
    for kind, group, uuid, fields in items(kp):
        print(f"{kind}" + (f" ({group})" if group else "") + f" {uuid}")
        for key, value in fields:
            if key == "faraday.kind":
                continue
            print(f"  {key}: {value}")


def write_field(kp, spec: str, index: int | None, out: str) -> None:
    """One value of one item, as bytes, to a new file only its owner reads."""
    kind, dot, name = spec.partition(".")
    if not dot or not name or (kind != "entry" and kind not in KINDS):
        raise SystemExit("--field wants KIND.FIELD, as gpg.key (a kind: entry, " + ", ".join(KINDS) + ")")
    same = [fields for k, _, _, fields in items(kp) if k == kind]
    if not same:
        raise SystemExit(f"the vault holds no {kind}")
    if index is None:
        if len(same) > 1:
            raise SystemExit(f"the vault holds {len(same)} of kind {kind}; --index N (from 1) takes one")
        index = 1
    if not 1 <= index <= len(same):
        raise SystemExit(f"the vault holds {len(same)} of kind {kind}, not {index}")
    values = [v for k, v in same[index - 1] if k == name]
    if len(values) != 1:
        raise SystemExit(f"that {kind} has no field {name}" if not values else f"that {kind} repeats field {name}")
    data = values[0].encode()
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_BINARY", 0)
    try:
        fd = os.open(out, flags, 0o600)
    except FileExistsError:
        raise SystemExit(f"{out} exists; nothing was written")
    with os.fdopen(fd, "wb") as f:
        f.write(data)
    print(f"wrote {len(data)} bytes to {out}", file=sys.stderr)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("vault")
    parser.add_argument("--passphrase")
    parser.add_argument("--field", help="KIND.FIELD to write, as gpg.key")
    parser.add_argument("--index", type=int, help="which item of that kind, from 1")
    parser.add_argument("--out", help="the new file --field writes")
    args = parser.parse_args()
    if (args.field is None) != (args.out is None) or (args.index is not None and args.field is None):
        raise SystemExit("--field and --out go together, and --index goes with them")
    if args.field is not None and args.passphrase is not None:
        raise SystemExit("--passphrase is refused with --field: an argument stays in the shell's history; type it at the prompt")
    read_file(args.vault)
    if args.passphrase is not None:
        passphrase = args.passphrase
    elif sys.stdin.isatty():
        passphrase = getpass.getpass("Passphrase: ")
    else:
        # A passphrase piped in (a test's): one line, without its newline.
        passphrase = sys.stdin.readline().rstrip("\r\n")
    nfc = unicodedata.normalize("NFC", passphrase)
    if not nfc:
        raise SystemExit("an empty passphrase opens nothing")
    try:
        kp = PyKeePass(args.vault, password=nfc)
    except Exception as e:
        name = type(e).__name__
        if name == "CredentialsError":
            raise SystemExit("Wrong passphrase. A file that also needs a key file or a security key cannot be opened here.")
        raise SystemExit(f"the file does not open ({name}): it is damaged or not one Faraday writes")
    if args.field is not None:
        return write_field(kp, args.field, args.index, args.out)
    print_vault(kp)


if __name__ == "__main__":
    sys.exit(main())
