#!/usr/bin/env python3
"""Open a Faraday vault, a file vault with pykeepass or a QR vault with sq, and print what it holds.

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
    python3 tools/vault/open.py vault.pgp
    python3 tools/vault/open.py vault.pgp --field note --out note.txt

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

A QR vault is an OpenPGP message (`docs/VAULT.md` §1.2). It is opened by
Sequoia's `sq decrypt` (on PATH; pykeepass is not needed for it). This
file is a recovery reader: it parses no OpenPGP and reads whatever `sq`
opens under the passphrase and whose text is the QR TOML. Refusing a message
that §1.2's packets or §2.1's rules exclude is Faraday's Rust
(`faraday_vault::qr::open`), not this file. The passphrase, in NFC, goes to
`sq` through a pipe, never as an argument or in a file. What is printed is the item as §2.1 names it, the TOML keys
`words`, `codex32`, `passphrase` or `note` and `for`, without the line
feeds that pad it to its length; an empty vault prints "QR vault: empty".
`--field KEY --out FILE` writes one key's value, as the bytes of the
text. A QR vault holds one item, so `--index` is not taken for it.
"""

from __future__ import annotations

import argparse
import getpass
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import unicodedata
from pathlib import Path


def pykeepass():
    """The distribution's pykeepass, imported only where a file vault is read."""
    try:
        from pykeepass import PyKeePass
    except ImportError as e:
        raise SystemExit(
            "open.py needs the distribution's pykeepass (Debian and Ubuntu: python3-pykeepass;"
            f" Fedora: python3-pykeepass; Arch: python-pykeepass): {e}") from e
    return PyKeePass

SIGNATURES = (0x9AA2D903, 0xB54BFB67)
KINDS = ("seed", "wallet", "note", "sheet", "gpg", "secure_boot", "wallet_share", "passphrase", "share")


QR_KEYS = ("words", "codex32", "passphrase", "note", "for")


def is_qr(path: str) -> bool:
    """A file that is not KDBX and begins like an OpenPGP packet (a byte
    whose top bit is set)."""
    data = Path(path).read_bytes()
    if len(data) >= 12 and struct.unpack_from("<2I", data, 0) == SIGNATURES:
        return False
    return bool(data[:1]) and bool(data[0] & 0x80)


def read_file(path: str) -> bytes:
    data = Path(path).read_bytes()
    if len(data) < 12 or struct.unpack_from("<2I", data, 0) != SIGNATURES:
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
    write_value(values[0].encode(), out)


def write_value(data: bytes, out: str) -> None:
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_BINARY", 0)
    try:
        fd = os.open(out, flags, 0o600)
    except FileExistsError:
        raise SystemExit(f"{out} exists; nothing was written")
    with os.fdopen(fd, "wb") as f:
        f.write(data)
    print(f"wrote {len(data)} bytes to {out}", file=sys.stderr)


def decrypt_qr(path: str, passphrase: str) -> str:
    """The QR TOML of a QR vault, as `sq decrypt` prints it, padding included."""
    sq = shutil.which("sq")
    if sq is None:
        raise SystemExit("open.py needs Sequoia's sq on PATH to read a QR vault (Debian and Ubuntu: sq; Fedora: sequoia-sq; Arch: sequoia-sq)")
    if os.name == "nt":
        raise SystemExit("open.py reads a QR vault on POSIX systems only; on Windows run sq decrypt by hand (docs/VAULT.md, section 10)")
    # The passphrase reaches sq through a pipe, so it is never in a file. It
    # is written before sq starts and the write end closed, which a pipe's
    # buffer holds at this length.
    secret = passphrase.encode()
    if len(secret) > 4096:
        raise SystemExit("a passphrase over 4096 bytes is not read here")
    r, w = os.pipe()
    try:
        os.write(w, secret)
        os.close(w)
        w = -1
        with tempfile.TemporaryDirectory(prefix="faraday-open-") as home:
            done = subprocess.run(
                [sq, "--home", home, "--cert-store=none", "--key-store=none", "decrypt",
                 "--password-file", f"/dev/fd/{r}", "--signatures", "0", path],
                # No terminal: sq asks for a password when none of the ones it was
                # given fits, and this file has given its one.
                capture_output=True, stdin=subprocess.DEVNULL, start_new_session=True,
                pass_fds=(r,))
    finally:
        os.close(r)
        if w >= 0:
            os.close(w)
    if done.returncode != 0:
        said = done.stderr.decode(errors="replace").strip().splitlines()
        raise SystemExit("sq did not open the QR vault (a wrong passphrase, or not a message sq opens): "
                         + (said[-1].strip() if said else f"exit {done.returncode}"))
    try:
        return done.stdout.decode()
    except UnicodeDecodeError:
        raise SystemExit("the QR vault does not hold text")


def qr_items(plain: str) -> list[tuple[str, str]]:
    """The keys of the QR TOML in order, with their values as text."""
    import tomllib
    try:
        table = tomllib.loads(plain)
    except tomllib.TOMLDecodeError as e:
        raise SystemExit(f"the QR vault does not hold TOML: {e}")
    for key, value in table.items():
        if key not in QR_KEYS or not isinstance(value, str):
            raise SystemExit(f"the QR vault holds a key it should not: {key}")
    return list(table.items())


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("vault")
    parser.add_argument("--passphrase")
    parser.add_argument("--field", help="a file vault's KIND.FIELD to write, as gpg.key; a QR vault's KEY, as note")
    parser.add_argument("--index", type=int, help="which item of that kind, from 1")
    parser.add_argument("--out", help="the new file --field writes")
    args = parser.parse_args()
    if (args.field is None) != (args.out is None) or (args.index is not None and args.field is None):
        raise SystemExit("--field and --out go together, and --index goes with them")
    if args.field is not None and args.passphrase is not None:
        raise SystemExit("--passphrase is refused with --field: an argument stays in the shell's history; type it at the prompt")
    qr = is_qr(args.vault)
    if qr and args.index is not None:
        raise SystemExit("--index is for file vaults: a QR vault holds one item")
    if not qr:
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
    if qr:
        pairs = qr_items(decrypt_qr(args.vault, nfc))
        if args.field is not None:
            values = [v for k, v in pairs if k == args.field]
            if not values:
                raise SystemExit(f"the QR vault has no {args.field}")
            return write_value(values[0].encode(), args.out)
        print("QR vault" if pairs else "QR vault: empty")
        for key, value in pairs:
            print(f"  {key}: {value}")
        return
    try:
        kp = pykeepass()(args.vault, password=nfc)
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
