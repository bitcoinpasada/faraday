"""The vault area (`docs/VAULT.md`): Argon2id and the payload layouts of §7
against a reader written here from that document, over Argon2id from
argon2-cffi, the distribution's package (F79, `docs/CROSSCHECK.md` §10.4), and PyCA
cryptography's (OpenSSL's) beside it as a second lineage;
and the vault file, KDBX 4 (§1.1, §7, §7.1), against two outside programs
that know nothing of this repository, pykeepass and `keepassxc-cli`, both
ways.

Before anything else the area checks argon2-cffi, and PyCA (44 or later over OpenSSL 3.2
or later), against `tools/vectors/kdf/`'s known answer (`tools/vectors/vault-primitives/`). Then:

- Argon2id (`faraday_backup::argon2id`) at random small costs, salts and
  passwords, with the costs and salts Argon2 refuses; PyCA's answer or refusal must be
  argon2-cffi's.
- The payload layouts of §7: words, seed and sheet, made and read, with
  every way a payload can be malformed, and a sheet past 16 KiB.
- A passphrase's NFC form (§3) against Python's `unicodedata`: NFD
  letters, Hangul jamo, the singletons NFC replaces, combining marks in
  either order, the compatibility characters NFC keeps, then random text.
  The area stops where Python's Unicode is newer than
  `unicode-normalization`'s.
- Every file in `tools/vectors/vault/`: Faraday opens the ones it opens and
  refuses the others; pykeepass and `keepassxc-cli` open the first and
  show the same entries, and refuse the second except where a reader's own
  rules are wider than Faraday's (`WIDER`, said in the report).
- Faraday writes, each outside reader reads (`faraday` loop): a vault with
  items of every kind of §7, at small costs, then Faraday's own next seal
  of it: pykeepass reads every group, entry, field, protected mark and
  custom data item as Faraday read it and as it was put in;
  `keepassxc-cli` lists the groups and entries and shows every field. The
  passphrase is NFC (§3); the NFD form opens in Faraday and in neither
  outside reader. A wrong passphrase and a changed byte are refused by all.
- An export, made by `file::create_holding` (`export` loop, the function the app's export
  runs): entries whose Title, Notes and Password hold control characters, read as in the
  `faraday` loop by pykeepass and `keepassxc-cli` and keepass-rs, a wrong passphrase, the NFD form and a changed byte refused. The export vector of
  `tools/vectors/backup/` goes the same way with a byte changed, and with each bit of the
  version field and bit 44 of the Argon2 iterations changed in turn: all refused.
- They write, Faraday reads (`foreign` loop, `kpxc` loop): files made at
  random by pykeepass (cipher, KDF, compression, groups, entries, tags,
  icons, custom properties, attachments, history, custom data) and by
  `keepassxc-cli` (from a KDBX 4 file of Faraday's, because its `db-create`
  writes 3.1 under AES-KDF: groups, entries, notes, attachments) open in
  Faraday with the same fields; Faraday's next seal over each keeps
  everything pykeepass read (attachments, tags, icons, history and the
  other program's custom data, §7.1).

- keepass-rs, the Rust KeePass library (`keepass_rs.py`, the program of
  `tools/crosscheck/keepass-rs/`), judges the file format only, both ways
  (F193 point 1; it runs RustCrypto, as the app does, so it is not counted
  for the primitives): it reads vaults Faraday writes and seals (`rsread`
  loop) and Faraday reads files it writes (`rswrite` loop: AES-256 or
  ChaCha20, Argon2d or Argon2id, with or without compression, groups,
  entries, tags, icons, custom properties, attachments, custom data), then
  keepass-rs reads Faraday's seal over them. Files it writes with Twofish,
  AES-KDF or a Salsa20 inner stream, which Faraday refuses (§4), are a
  listed known difference.

`keepassxc-cli` runs per case, slowly, so a share of the files go to it.
The area stops and names it when it is missing. The reference is the
document alone for the payloads, and the outside programs for the file.
"""

import base64
import datetime
import hashlib
import json
import os
import unicodedata

import keepass_rs as rs
import readers
import vault_files as vf
from common import ROOT, Area, HarnessError, Mismatch, reference
from vault_files import word

try:
    from argon2.low_level import Type, hash_secret_raw
except ImportError as e:  # pragma: no cover - the machine's packages
    raise HarnessError(
        "the vault area needs the distribution's Argon2 binding, argon2-cffi "
        "(Debian and Ubuntu: python3-argon2; Fedora: python3-argon2-cffi; Arch: "
        f"python-argon2_cffi): {e}") from e

try:
    import cryptography
    from cryptography.hazmat.backends.openssl import backend as openssl_backend
    from cryptography.hazmat.primitives.kdf.argon2 import Argon2id
except ImportError as e:  # pragma: no cover - the machine's packages
    raise HarnessError(
        "the vault area needs the distribution's PyCA cryptography, 44 or later, over OpenSSL "
        "3.2 or later, for its Argon2id (Debian and Ubuntu: python3-cryptography; Fedora: "
        f"python3-cryptography; Arch: python-cryptography): {e}") from e

VECTORS = json.loads((ROOT / "tools/vectors/vault-primitives/vectors.json").read_text())

LANGUAGES = 10
MAX_SHEET = 16 * 1024


class Refused(Exception):
    """A refusal the area compares, with the reason `faraday_vault`'s layouts or NFC step give."""

    def __init__(self, kind, detail=""):
        super().__init__(f"{kind}: {detail}" if detail else kind)
        self.kind = kind


def argon2id(password, salt, memory_kib, passes, lanes):
    return hash_secret_raw(secret=bytes(password), salt=bytes(salt), time_cost=passes,
                           memory_cost=memory_kib,
                           parallelism=lanes, hash_len=32, type=Type.ID, version=19)


def pyca_argon2id(password, salt, memory_kib, passes, lanes):
    """PyCA cryptography's Argon2id (OpenSSL's), version 1.3, 32 bytes: the second lineage
    beside argon2-cffi's reference C."""
    return Argon2id(salt=bytes(salt), length=32, iterations=passes, lanes=lanes,
                    memory_cost=memory_kib).derive(bytes(password))


# ----- the payload layouts (§7) ---------------------------------------------



def words_payload_of(p):
    if len(p) != 34:
        return None
    count, lang = p[0], p[1]
    n = count * 4 // 3
    if count not in (12, 15, 18, 21, 24) or lang >= LANGUAGES or any(p[2 + n:]):
        return None
    return {"words": {"language": lang, "entropy": p[2:2 + n].hex()}}


def seed_payload_of(p):
    if len(p) != 65 or not 16 <= p[0] <= 64 or any(p[1 + p[0]:]):
        return None
    return {"seed": p[1:1 + p[0]].hex()}


def sheet_payload_of(p):
    """Three u16-length UTF-8 parts and nothing after; at most 16 KiB,
    read as written."""
    if len(p) > MAX_SHEET:
        return None
    parts, at = [], 0
    for _ in range(3):
        if at + 2 > len(p):
            return None
        n = int.from_bytes(p[at:at + 2], "little")
        at += 2
        if at + n > len(p):
            return None
        try:
            p[at:at + n].decode("utf-8")
        except UnicodeDecodeError:
            return None
        parts.append(p[at:at + n].hex())
        at += n
    return {"sheet": parts} if at == len(p) else None


def payload_of(kind, p):
    if kind == 1:
        return words_payload_of(p)
    if kind == 2:
        return seed_payload_of(p)
    if kind == 4:
        return sheet_payload_of(p)
    return None


# ----- the area --------------------------------------------------------------


class Vault(Area):
    name = "vault"
    faraday = (
        "`faraday_vault`'s file (`file::create`, `open` and `seal`, the document model and the "
        "items of §7), its payload layouts and a passphrase's NFC form with the Unicode version "
        "it is made under, `file::create_holding` (an export), and `faraday_backup::argon2id`"
    )
    reference = (
        "pykeepass and `keepassxc-cli` (the distribution's packages) for the vault file, both "
        "ways, and keepass-rs for the file format only; `docs/VAULT.md` §7 written out in `area_vault.py`, over argon2-cffi and PyCA "
        "cryptography's Argon2id (OpenSSL's) (the distribution's packages) and Python's "
        "`unicodedata`, for the payloads"
    )
    reference_checked = (
        "`check_primitives` runs before anything else: argon2-cffi and PyCA (44 or later over "
        "OpenSSL 3.2 or later, or the area stops) against `tools/vectors/kdf/`'s known answer; pykeepass and `keepassxc-cli` are the "
        "distribution's packages and the area stops and names a missing one, so a run "
        "without it fails; their versions are written into the stamp; the area stops where "
        "Python's Unicode is newer than `unicode-normalization`'s"
    )
    vectors = (
        "the Argon2id known answer (`tools/vectors/vault-primitives/`); the export vector of "
        "`tools/vectors/backup/`, read with a byte changed refused, and with each bit of the "
        "version field and bit 44 of the Argon2 iterations changed refused; every file of "
        "`tools/vectors/vault/`, read by Faraday and by both outside readers"
    )
    corpus = (
        'Argon2id at small random costs and the costs it refuses; payloads made, read '
        'and malformed; passphrases in NFC, by case and at random; vaults Faraday writes '
        'at small costs with items of every kind of §7 and seals once more, read by '
        'pykeepass (every group, entry, field, protected mark and custom data item) and '
        '`keepassxc-cli` (listing and every field), under the passphrase, its NFD form, a '
        'wrong one and a changed byte; exports (`file::create_holding`) of one to eight '
        'entries whose Title, Notes and Password hold control characters, read the same '
        'way;  files pykeepass makes at random (ciphers, KDFs, '
        'compression, tags, icons, custom properties, attachments, history, custom data) '
        'and `keepassxc-cli` makes (groups, entries, notes, attachments), opened in '
        "Faraday and sealed once more with nothing of the other program's lost; keepass-rs "
        "(format only, not counted for the primitives) reads the vaults Faraday writes and "
        "seals, with a wrong passphrase, the NFD form and a changed byte refused, and writes "
        "files (AES-256 or ChaCha20, Argon2d or Argon2id, compression or not, groups, "
        "entries, tags, icons, properties, attachments, custom data) that Faraday opens and "
        "seals once more"
    )
    differences = [
        (
            "pykeepass strips the characters XML 1.0 cannot hold from a protected value after "
            "decrypting it; Faraday keeps a Password's characters as they are put in (and "
            "leaves them out of a Title or Notes when it writes). The `export` loop expects "
            "pykeepass's reading of a Password and Faraday's own as put in, and counts the "
            "passwords that differ."
        ),
        (
            "pykeepass opens a file whose major version is neither 3 nor 4 without reading "
            "past it, deriving a key or checking a hash; KeePass refuses such a file, and the "
            "area counts a file as opened only when pykeepass read a KDBX 4 body "
            "(`readers.open_kdbx`). It also runs the key derivation the header states before "
            "it checks the header's SHA-256, so a changed bit in the Argon2 iterations (2^44 "
            "passes) holds it for good; KeePass and `keepassxc-cli` refuse at once, and the "
            "area asks pykeepass first with a key given and nothing decrypted."
        ),
        (
            "The vault's passphrase is put in NFC (§3); KeePass apps do not. A vault under "
            "an NFC passphrase opens in Faraday under its NFD form and in neither outside "
            "reader. The area counts the NFD tries and expects exactly that."
        ),
        (
            "Faraday's reader is stricter than the outside readers' in what it takes of a "
            "file (§4, §7.1): a block stream that ends before its empty last block "
            "(`keepassxc-cli` reads it), an XML document with an element after the root "
            "(`keepassxc-cli`), a protected value not canonical Base64, nesting past 64, "
            "another root element, a DOCTYPE and another encoding (pykeepass reads these). "
            "`WIDER` names each and the reader that opens it; the area expects exactly that "
            "set and counts those opens as known differences."
        ),
        (
            "keepass-rs writes files Faraday refuses (§4): Twofish, AES-KDF and a Salsa20 "
            "inner stream (`keepass_rs.REFUSED`). The area makes one of each, expects "
            "Faraday to refuse it and counts the refusals as known differences."
        ),
        (
            "keepass-rs decodes a custom data item's value as Base64 when it can, and the "
            "program encodes the bytes again (`keepass-rs/src/main.rs`). A value that is "
            "valid Base64 but not canonical (padding or trailing bits the encoder would not "
            "write) comes back as another text, so keepass-rs's reading of such a value "
            "differs from the file's and from pykeepass's. The area compares custom data as they "
            "are, so such a value is a known difference of keepass-rs, not a disagreement "
            "about the format."
        ),
    ]
    not_compared = (
        "keepass-rs does not count for the primitives and takes no part in the NFD or "
        "wrong-passphrase cases of files it did not read; the QR vault and OpenPGP are other "
        "areas"
    )

    def __init__(self, *a):
        super().__init__(*a)
        self.second = {"judge": "keepassxc-cli", "judged": 0, "uncovered": 0}
        self.has_kpxc = None
        # keepass-rs judges the file format only (F193 point 1); it is counted apart from
        # `second`, which a primitive check needs.
        self.format_judge = {"judge": "keepass-rs", "judged": 0}
        self.rs_case = False

    def check_primitives(self):
        """The area's own piece against its published answer."""
        a = VECTORS["argon2id"]
        if argon2id(bytes.fromhex(a["password"]), bytes.fromhex(a["salt"]), a["memory_kib"],
                    a["passes"], a["lanes"]).hex() != a["key"]:
            raise HarnessError("argon2-cffi fails the known Argon2id answer")
        major = int(cryptography.__version__.split(".")[0])
        ssl = openssl_backend.openssl_version_number()
        if major < 44 or ssl < 0x30200000:
            raise HarnessError(
                "the vault area needs PyCA cryptography 44 or later over OpenSSL 3.2 or later, "
                f"found {cryptography.__version__} over {openssl_backend.openssl_version_text()}")
        if pyca_argon2id(bytes.fromhex(a["password"]), bytes.fromhex(a["salt"]), a["memory_kib"],
                         a["passes"], a["lanes"]).hex() != a["key"]:
            raise HarnessError("PyCA's Argon2id fails the known Argon2id answer")

    # Each operation, both sides.

    def argon(self, password, salt, memory, passes, lanes, expected=None):
        """Faraday's Argon2id against argon2-cffi's, then PyCA's against argon2-cffi's: the
        same key, or a refusal by both."""
        inputs = {"password": password.hex(), "salt": salt.hex(), "memory_kib": memory,
                  "passes": passes, "lanes": lanes}
        raw = self.compare(
            "vault.argon2id", inputs,
            lambda: {"key": argon2id(password, salt, memory, passes, lanes).hex()}, expected)
        other = reference(lambda: {"key": pyca_argon2id(password, salt, memory, passes,
                                                        lanes).hex()})
        if other[0] != raw[0] or (raw[0] == "ok" and other[1] != raw[1]):
            raise Mismatch(self.name, "vault.argon2id", inputs, raw, other,
                           "PyCA's Argon2id differs from argon2-cffi's and Faraday's")
        return raw

    def payload(self, kind, p):
        def ref():
            out = payload_of(kind, p)
            if out is None:
                raise Refused("not that layout")
            return out
        return self.compare("vault.payload_of", {"kind": kind, "payload": p.hex()}, ref)

    # Random inputs.

    def text(self, limit=40):
        rng = self.rng
        alphabet = rng.choice(["abcdefghijklmnopqrstuvwxyz ", "éüßçøñ ", "日本語漢字 ", "🙂🚀 ",
                               "abc\n\t-/"])
        out = b""
        for _ in range(rng.randint(0 if rng.random() < 0.2 or not limit else 1, limit)):
            c = rng.choice(alphabet).encode()
            if len(out) + len(c) > limit:
                break
            out += c
        return out

    def payload_case(self):
        rng = self.rng
        kind = rng.choice([1, 2, 4])
        if kind == 1:
            lang = rng.randrange(LANGUAGES)
            count = rng.choice([12, 15, 18, 21, 24])
            entropy = self.randbytes(count * 4 // 3)
            p = self.need(self.compare(
                "vault.words_payload", {"language": lang, "entropy": entropy.hex()},
                lambda: {"payload": (bytes([count, lang]) + entropy
                                     + bytes(32 - len(entropy))).hex()}), "words")["payload"]
        elif kind == 2:
            seed = self.randbytes(rng.choice([rng.randint(16, 64), rng.randint(0, 80)]))

            def ref():
                if not 16 <= len(seed) <= 64:
                    raise Refused("not 16 to 64 bytes")
                return {"payload": (bytes([len(seed)]) + seed + bytes(64 - len(seed))).hex()}
            ans = self.compare("vault.seed_payload", {"seed": seed.hex()}, ref)
            if ans[0] != "ok":
                return
            p = ans[1]["payload"]
        else:
            parts = [self.text(rng.choice([0, 20, 400])) for _ in range(3)]
            if rng.random() < 0.05:
                parts[0] = self.randbytes(rng.choice([16380, 16384, 20000, 65536]))

            def ref():
                total = 6 + sum(len(x) for x in parts)
                if total > MAX_SHEET or any(len(x) > 0xFFFF for x in parts):
                    raise Refused("too long")
                return {"payload": b"".join(len(x).to_bytes(2, "little") + x
                                            for x in parts).hex()}
            ans = self.compare("vault.sheet_payload", {"descriptor": parts[0].hex(),
                                                       "name": parts[1].hex(),
                                                       "note": parts[2].hex()}, ref)
            if ans[0] != "ok":
                return
            p = ans[1]["payload"]
        p = bytearray(bytes.fromhex(p))
        self.payload(kind, bytes(p))
        # The same bytes read as another layout, and malformed.
        self.payload(rng.choice([k for k in (0, 1, 2, 3, 4, 5) if k != kind]), bytes(p))
        how = rng.randrange(5)
        if how == 0 and p:
            p[rng.randrange(len(p))] ^= 1 << rng.randrange(8)
        elif how == 1:
            p += bytes([rng.randrange(256)])
        elif how == 2 and p:
            del p[-1]
        elif how == 3 and kind in (1, 2):
            p[0] = rng.choice([0, 11, 13, 25, 15, 16, 64, 65, 255])
        elif how == 4 and kind == 1:
            p[1] = rng.choice([10, 11, 255])
        self.payload(kind, bytes(p))

    def big_sheet(self):
        """A sheet payload past 16 KiB, which §7 caps: refused by both."""
        p = (20000).to_bytes(2, "little") + b"d" * 20000 + bytes(4)
        self.payload(4, p)

    def nfc(self, passphrase):
        def ref():
            out = unicodedata.normalize("NFC", passphrase).encode()
            if not out:
                raise Refused("empty")
            return {"nfc": out.hex()}
        return self.compare("vault.passphrase_nfc", {"passphrase": passphrase}, ref)

    def nfc_cases(self):
        """§3's forms: those NFC makes one, and the compatibility characters
        it keeps as other passphrases."""
        for p in ("", "e\u0301", "\u00e9", "A\u030a", "\u00c5",
                  "\u1112\u1161\u11ab\u1100\u1173\u11af", "\ud55c\uae00",
                  "\u212b", "\u2126", "a\u0323\u0302", "a\u0302\u0323", "\u1ead",
                  "\ufb01", "fi", "\uff21", "A", "a\u00a0b", "a b"):
            self.nfc(p)

    def unicode_text(self, most=12):
        """Random text: code points assigned in Python's Unicode version,
        drawn often from the combining marks, the Hangul jamo and syllables,
        and the letters that compose."""
        rng = self.rng
        pools = [(0x20, 0x7E), (0xC0, 0x24F), (0x300, 0x36F), (0x1100, 0x11FF),
                 (0xAC00, 0xD7A3), (0x1E00, 0x1EFF), (0x2100, 0x214F), (0xFB00, 0xFB4F),
                 (0xFF00, 0xFFEF), (0x0900, 0x097F), (0x3040, 0x30FF), (0x1F300, 0x1F64F),
                 (0x20, 0x10FFFF)]
        out = ""
        for _ in range(rng.randint(1, most)):
            lo, hi = rng.choice(pools)
            while True:
                ch = chr(rng.randint(lo, hi))
                if unicodedata.category(ch) not in ("Cn", "Cs"):
                    break
            out += ch
        return out

    def check_unicode(self):
        """The area stops where Python's Unicode is newer than the crate's:
        a code point assigned only in the newer could compose on one side
        alone."""
        ans = self.rust.call("vault.unicode_version")
        if ans[0] != "ok":
            raise HarnessError(f"vault.unicode_version: {ans[1]}")
        theirs = ans[1]["version"]
        version = lambda v: tuple(int(x) for x in v.split("."))  # noqa: E731
        if version(unicodedata.unidata_version) > version(theirs):
            raise HarnessError(
                f"Python's Unicode {unicodedata.unidata_version} is newer than "
                f"unicode-normalization's {theirs}: a code point only one side composes could "
                "differ")

    LOOPS = (("argon", 60), ("payload", 1500), ("nfc", 1000), ("faraday", 16), ("export", 16), ("foreign", 16),
             ("kpxc", 6), ("rsfixed", 1), ("rsread", 6), ("rswrite", 8))

    def prelude(self):
        self.check_unicode()
        self.check_primitives()
        self.kpxc_on()
        a = VECTORS["argon2id"]
        self.argon(bytes.fromhex(a["password"]), bytes.fromhex(a["salt"]), a["memory_kib"],
                   a["passes"], a["lanes"], ("ok", {"key": a["key"]}))
        self.big_sheet()
        self.nfc_cases()
        self.vectors()
        # One vault with an item of every kind, whatever the corpus draws.
        self.faraday_case(kinds=list(vf.KINDS), kpxc=self.kpxc_on())

    def case_nfc(self):
        self.nfc(self.unicode_text())

    def case_argon(self):
        rng = self.rng
        lanes = rng.choice([1, 1, 2, 4])
        self.argon(self.randbytes(rng.randint(0, 64)),
                   self.randbytes(rng.choice([16, 32, rng.randint(0, 40)])),
                   rng.choice([8 * lanes, rng.randint(8 * lanes - 2, 300)]),
                   rng.choice([1, 2, 3, rng.randint(0, 4)]), lanes if rng.random() < 0.9 else 0)

    def case_payload(self):
        self.payload_case()

    # ----- the vault file, against the outside readers ------------------------

    def kpxc_on(self):
        """Whether `keepassxc-cli` judges; the area stops and names it when it is missing."""
        if self.has_kpxc is None:
            readers.need_keepassxc_cli()
            self.has_kpxc = True
        return self.has_kpxc

    def fail(self, op, inputs, got, want, note, data=None):
        """A disagreement; the file is kept in `out/crosscheck/` for a look."""
        if data is not None:
            folder = ROOT / "out" / "crosscheck"
            if folder.is_dir():
                name = f"vault-{hashlib.sha256(data).hexdigest()[:12]}.kdbx"
                (folder / name).write_bytes(data)
                note += f" (the file: out/crosscheck/{name})"
        raise Mismatch(self.name, op, inputs, got, want, note)

    @staticmethod
    def tag(data):
        return f"{len(data)} bytes, sha256 {hashlib.sha256(data).hexdigest()[:16]}"

    @staticmethod
    def differ(got, want):
        """Where two views first differ, in words."""
        for k in ("label", "id", "custom"):
            if got.get(k) != want.get(k):
                return f"{k}: {got.get(k)!r} against {want.get(k)!r}"
        a = {e["uuid"]: e for e in got["entries"]}
        b = {e["uuid"]: e for e in want["entries"]}
        if a.keys() != b.keys():
            return f"entries: {sorted(a.keys() ^ b.keys())} in one only"
        for u in a:
            for k in ("path", "fields", "custom"):
                if a[u][k] != b[u][k]:
                    return f"entry {u} {k}: {a[u][k]!r} against {b[u][k]!r}"
        return "no difference found"

    def open_faraday(self, data, passphrase, want):
        """Faraday opens `data`: with a view as `want` it must read just that, with None
        it must refuse. Gives its answer."""
        inputs = {"file": self.tag(data), "passphrase": passphrase}
        got = self.rust.call("vault.open", file=data.hex(), passphrase=passphrase)
        self.compared += 1
        if want is None:
            if got[0] == "ok":
                self.fail("vault.open", inputs, "opened", "refused",
                          "Faraday opens a file the outside readers refuse", data)
            return None
        if got[0] != "ok":
            self.fail("vault.open", inputs, got, "opened", "Faraday refuses what pykeepass reads",
                      data)
        mine = vf.faraday_view(got[1])
        if mine != vf.plain(want):
            self.fail("vault.open", inputs, "a view", "a view", self.differ(mine, vf.plain(want)),
                      data)
        return got[1]

    def kpxc_checks(self, data, passphrase, v, wrong=True, flip=None, nfd=None):
        """`keepassxc-cli` over a file whose view pykeepass gave: the same listing and fields,
        a refusal of a wrong passphrase, a changed byte and the NFD form."""
        with readers.Kpxc(data) as k:
            diff = vf.kpxc_differs(k, passphrase, v)
            if diff:
                self.fail("keepassxc-cli", {"file": self.tag(data)}, diff[1], diff[2],
                          f"{diff[0]} does not show what pykeepass reads", data)
            self.second["judged"] += 1
            self.compared += 1
            if wrong and k.entries(passphrase + "x") is not None:
                self.fail("keepassxc-cli", {"file": self.tag(data)}, "opened", "refused",
                          "keepassxc-cli opens under a wrong passphrase", data)
            if nfd and k.entries(nfd) is not None:
                self.fail("keepassxc-cli", {"file": self.tag(data)}, "opened", "refused",
                          "keepassxc-cli opens under the NFD form of an NFC passphrase", data)
        if flip is not None:
            with readers.Kpxc(flip) as k:
                if k.entries(passphrase) is not None:
                    self.fail("keepassxc-cli", {"file": self.tag(flip)}, "opened", "refused",
                              "keepassxc-cli opens a file with a byte changed", flip)
        self.compared += 1

    @staticmethod
    def unstripped(v, raw):
        """pykeepass's view with each Password as the export put it: pykeepass strips what XML
        1.0 cannot hold from a protected value after decrypting it, Faraday reads it as it is
        (a known difference). `raw` maps an entry's UUID to the Password put in."""
        return v | {"entries": [
            e | {"fields": [[k, raw.get(e["uuid"], val) if k == "Password" else val, prot]
                            for k, val, prot in e["fields"]]} for e in v["entries"]]}

    def read_all(self, data, passphrase, kpxc, flips=True, raw=None):
        """The file under every reader: pykeepass's view, Faraday's reading of it under the
        passphrase and its NFD form, the refusals (a wrong passphrase, the NFD form where
        it differs, a byte changed) by all, and `keepassxc-cli`'s listing when `kpxc`.
        Gives pykeepass's deep view."""
        rng = self.rng
        kp = readers.open_kdbx(data, passphrase)
        if kp is None:
            self.fail("pykeepass", {"file": self.tag(data)}, "refused", "opened",
                      "pykeepass does not open what Faraday wrote", data)
        v = vf.view(kp, deep=True)
        mine = self.unstripped(v, raw) if raw else v
        self.open_faraday(data, passphrase, mine)
        nfd = unicodedata.normalize("NFD", passphrase)
        if nfd != passphrase:
            # Faraday puts the passphrase in NFC (§3); no outside reader does.
            self.open_faraday(data, nfd, mine)
            self.known += 1
            if readers.open_kdbx(data, nfd) is not None:
                self.fail("pykeepass", {"file": self.tag(data)}, "opened", "refused",
                          "pykeepass opens under the NFD form", data)
        else:
            nfd = None
        self.open_faraday(data, passphrase + "x", None)
        if readers.open_kdbx(data, passphrase + "x") is not None:
            self.fail("pykeepass", {"file": self.tag(data)}, "opened", "refused",
                      "pykeepass opens under a wrong passphrase", data)
        flip = None
        if flips:
            i = rng.randrange(len(data))
            flip = data[:i] + bytes([data[i] ^ (1 << rng.randrange(8))]) + data[i + 1:]
            self.open_faraday(flip, passphrase, None)
            if readers.open_kdbx(flip, passphrase) is not None:
                self.fail("pykeepass", {"file": self.tag(flip)}, "opened", "refused",
                          "pykeepass opens a file with a byte changed", flip)
        if kpxc:
            self.kpxc_checks(data, passphrase, v, flip=flip, nfd=nfd)
        elif not self.rs_case:
            self.second["uncovered"] += 1
        return v

    # What Faraday is asked to put in.

    def item(self, kind, n):
        """An item of a request, the n-th of it (titles differ by n)."""
        rng = self.rng

        def label(k=14):
            return f"{word(rng, 1, k)} {n}"
        if kind == "entry":
            it = {"kind": kind, "title": label(), "user": word(rng, 0, 12),
                  "password": word(rng, 0, 20),
                  "url": rng.choice(["", "https://" + word(rng, 1, 10)]),
                  "notes": word(rng, 0, 60, lines=True)}
            if rng.random() < 0.5:
                key = base64.b32encode(rng.randbytes(rng.choice([10, 20]))).decode()
                it["totp"] = f"otpauth://totp/Example:you?secret={key}&issuer=Example"
            return it
        if kind == "note":
            return {"kind": kind, "text": word(rng, 1, 80, lines=True)}
        if kind == "seed_words":
            it = {"kind": kind, "language": rng.randrange(LANGUAGES),
                  "entropy": rng.randbytes(rng.choice([16, 20, 24, 28, 32])).hex(),
                  "label": label(), "no_load": rng.random() < 0.3, "held": rng.random() < 0.3,
                  "codex32": rng.random() < 0.3, "child": rng.random() < 0.3}
            if it["child"]:  # a child seed is twelve words (records.rs)
                it["entropy"] = rng.randbytes(16).hex()
            if rng.random() < 0.5:
                it["passphrase"] = word(rng, 1, 16)
            return it
        if kind == "seed_hex":
            return {"kind": kind, "seed": rng.randbytes(rng.randint(16, 64)).hex(),
                    "title": label()}
        if kind in ("wallet_policy", "wallet_descriptor"):
            return {"kind": kind, "name": label()}
        if kind == "sheet":
            return {"kind": kind, "descriptor": word(rng, 1, 60), "name": label(),
                    "note": word(rng, 0, 40, lines=True)}
        if kind == "gpg":
            def armor(what):
                body = base64.b64encode(rng.randbytes(rng.randint(30, 300))).decode()
                return (f"-----BEGIN PGP {what} KEY BLOCK-----\n\n{body}\n"
                        f"-----END PGP {what} KEY BLOCK-----\n")
            it = {"kind": kind, "title": f"{word(rng, 1, 10)} <{word(rng, 1, 8)}@example.com> {n}",
                  "key": armor("PRIVATE")}
            if rng.random() < 0.7:
                it["public"] = armor("PUBLIC")
            if rng.random() < 0.4:
                it["revocation"] = armor("PUBLIC")
                it["revoked"] = "2026-10-28T14:02:00Z"
            return it
        if kind == "secure_boot":
            return {"kind": kind, "owner": rng.randbytes(16).hex()}
        if kind == "wallet_share":
            it = {"kind": kind, "text": word(rng, 1, 30), "name": label(),
                  "index": rng.randint(1, 3), "of": 3}
            if rng.random() < 0.5:
                it["wallet"] = word(rng, 1, 40)
            return it
        if kind == "passphrase":
            return {"kind": kind, "key": rng.randbytes(4).hex(), "text": word(rng, 1, 30)}
        if kind == "plan":
            return {"kind": kind, "place": "".join(rng.choice("abcdefXYZ019 ") for _ in range(8))
                    .strip() or "Home"}
        if kind == "signed":
            return {"kind": kind, "tx": rng.randbytes(32).hex(), "first": rng.randrange(10**9),
                    "second": rng.randrange(10**9)}
        return {"kind": kind}

    def items(self, kinds, start=0, entries=0):
        """Items of the kinds (and `entries` more of a person's), numbered from `start`."""
        out = [self.item(k, start + i) for i, k in enumerate(list(kinds) + ["entry"] * entries)]
        return out

    def pick(self, lo=1, hi=None):
        """Kinds of items for a vault: one of each at most (several of a kind share a title)."""
        rng = self.rng
        kinds = list(vf.KINDS)
        return rng.sample(kinds, rng.randint(lo, hi or len(kinds)))

    def put_check(self, v, items, start, plans, data):
        """pykeepass's view holds every item as put: its group, its fields, the vault's
        custom data for a plan and the signed amounts."""
        by = {e["uuid"]: e for e in v["entries"]}
        for n, it in enumerate(items, start):
            want = vf.expected(n, it)
            if want is None:
                continue
            path, fields = want
            e = by.get(vf.uuid_of(n))
            if e is None:
                self.fail("vault.create", {"item": it}, "missing", "an entry",
                          f"pykeepass finds no entry for {it['kind']}", data)
            have = vf.fields_of(e)
            for k, val in fields.items():
                if have.get(k, "") != val:
                    self.fail("vault.create", {"item": it}, {k: have.get(k)}, {k: val},
                              f"{it['kind']}'s field {k} is not what was put in", data)
            if e["path"] != path:
                self.fail("vault.create", {"item": it}, e["path"], path,
                          f"{it['kind']} is in another group", data)
        meta = dict(v["custom"])
        for it in items:
            if it["kind"] == "plan":
                plans += 1
            for k, val in vf.custom_expected(it, plans):
                if val not in meta.get(k, ""):
                    self.fail("vault.create", {"item": it}, meta.get(k), val,
                              f"the vault's custom data {k} is not what was put in", data)
        return plans

    # Faraday writes.

    def create(self, items, size, cost, passphrase, label, now):
        rng = self.rng
        ans = self.rust.call(
            "vault.create", vault_id=rng.randbytes(16).hex(), label=label, passphrase=passphrase,
            memory_kib=cost[0], passes=cost[1], lanes=cost[2], size=size,
            random=rng.randbytes(140).hex(), now=now, items=items)
        self.compared += 1
        if ans[0] != "ok":
            alone = []
            for it in items:
                one = self.rust.call(
                    "vault.create", vault_id="00" * 16, label=label, passphrase=passphrase,
                    memory_kib=cost[0], passes=cost[1], lanes=cost[2], size=size,
                    random=rng.randbytes(140).hex(), now=now, items=[it])
                if one[0] != "ok":
                    alone.append((it, one[1]))
            raise AssertionError(f"vault.create refused a valid vault: {ans[1]}; alone: {alone}")
        return bytes.fromhex(ans[1]["file"])

    def export(self, items, cost, passphrase, label, now):
        """`file::create_holding`, the function an export is made by, over entries."""
        rng = self.rng
        ans = self.rust.call(
            "vault.export", vault_id=rng.randbytes(16).hex(), label=label, passphrase=passphrase,
            memory_kib=cost[0], passes=cost[1], lanes=cost[2], random=rng.randbytes(140).hex(),
            now=now, items=items)
        self.compared += 1
        if ans[0] != "ok":
            raise AssertionError(f"vault.export refused a valid export: {ans[1]}")
        return bytes.fromhex(ans[1]["file"])

    def seal(self, data, passphrase, items, label=None, now=None):
        rng = self.rng
        req = {"file": data.hex(), "passphrase": passphrase, "items": items,
               "random": rng.randbytes(140).hex(), "now": now}
        if label is not None:
            req["label"] = label
        ans = self.rust.call("vault.seal", **req)
        self.compared += 1
        if ans[0] != "ok":
            raise AssertionError(f"vault.seal refused a valid change: {ans[1]}")
        return bytes.fromhex(ans[1]["file"])

    def seal_kept(self, data, passphrase, before, items, kpxc, label=None):
        """Faraday's next seal over `data` (its deep view `before`): every group, entry,
        field, tag, icon, attachment, history and custom data of the file is still there as
        pykeepass reads it, with the new items and the seal's own custom data added."""
        rng = self.rng
        now = rng.randrange(1_500_000_000, 2_200_000_000)
        out = self.seal(data, passphrase, items, label, now)
        after = self.read_all(out, unicodedata.normalize("NFC", passphrase), kpxc, flips=False)
        if after["label"] != (before["label"] if label is None else label):
            self.fail("vault.seal", {"label": label}, after["label"], label,
                      "the vault's name is not what the seal was given", out)
        if after["id"] != before["id"]:
            self.fail("vault.seal", {}, after["id"], before["id"], "the vault's id changed", out)
        kept = {e["uuid"]: e for e in after["entries"]}
        for e in before["entries"]:
            if kept.get(e["uuid"]) != e:
                self.fail("vault.seal", {"uuid": e["uuid"]}, kept.get(e["uuid"]), e,
                          "an entry of the file changed in Faraday's seal (§7.1)", out)
        if not set(map(tuple, before["groups"])) <= set(map(tuple, after["groups"])):
            self.fail("vault.seal", {}, after["groups"], before["groups"],
                      "a group of the file is gone after Faraday's seal", out)
        meta = dict(map(tuple, after["custom"]))
        for k, val in before["custom"]:
            if k.startswith("faraday."):
                continue
            if meta.get(k) != val:
                self.fail("vault.seal", {"key": k}, meta.get(k), val,
                          "another program's custom data is not kept by Faraday's seal", out)
        want = str(int(dict(map(tuple, before["custom"])).get("faraday.seals", "0")) + 1)
        if meta.get("faraday.seals") != want:
            self.fail("vault.seal", {}, meta.get("faraday.seals"), want,
                      "the seal count is not one higher", out)
        if meta.get("faraday.sealed") != datetime.datetime.fromtimestamp(
                now, datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"):
            self.fail("vault.seal", {}, meta.get("faraday.sealed"), now,
                      "the seal's time is not in the file", out)
        self.put_check(after, items, 100, int(self.plans_in(before)), out)
        return out, after

    @staticmethod
    def plans_in(v):
        return sum(1 for k, _ in v["custom"] if k.startswith("faraday.plan."))

    def wants_kpxc(self, rare=0.35):
        on = self.kpxc_on()
        return on and self.rng.random() < rare

    def faraday_case(self, kinds=None, kpxc=None):
        """A vault Faraday writes at a small cost, read by all, then sealed once more."""
        rng = self.rng
        kinds = kinds if kinds is not None else self.pick()
        entries = rng.choice([0, 0, 1, 2])
        items = self.items(kinds, entries=entries)
        p = vf.passphrase(self.rng)
        size = rng.choice([262_144, 262_144, 1_048_576]) if len(items) > 3 else \
            rng.choice([65_536, 262_144])
        cost = (8192, rng.choice([1, 2]), rng.choice([1, 2]))
        now = rng.choice([None, rng.randrange(1_500_000_000, 2_200_000_000)])
        label = word(rng, 1, 20)
        data = self.create(items, size, cost, p, label, now)
        kpxc = self.wants_kpxc() if kpxc is None else kpxc
        v = self.read_all(data, p, kpxc)
        if v["label"] != label:
            self.fail("vault.create", {}, v["label"], label, "the vault's name is not in the file",
                      data)
        plans = self.put_check(v, items, 0, 0, data)
        more = self.items([k for k in self.pick(0, 3) if k not in ("share", "secure_boot", "note")]
                          , start=100)
        # A seal takes the 64 MiB step and a fresh Argon2id run: once for the file.
        self.seal_kept(data, unicodedata.normalize("NFD", p) if rng.random() < 0.3 else p, v, more, kpxc,
                       label=rng.choice([None, word(rng, 1, 20)]))
        return plans

    def case_faraday(self):
        self.faraday_case()

    # They write.

    def export_case(self):
        """An export as the app makes one: entries whose Title, Notes and Password hold control
        characters, through `file::create_holding`, read by all. A Title and Notes are read
        without the characters XML 1.0 cannot hold, as the writer leaves them out; a Password
        is read by pykeepass without them and by Faraday as it was put in (a known
        difference, counted)."""
        rng = self.rng
        items = self.items([], entries=rng.randint(1, 8))
        for n, it in enumerate(items):
            it["title"] = f"{word(rng, 1, 14, controls=True)} {n}"
            it["notes"] = word(rng, 0, 60, lines=True, controls=True)
            it["password"] = word(rng, 0, 20, controls=True)
        raw = {vf.uuid_of(n): it["password"] for n, it in enumerate(items)}
        self.known += sum(vf.xml_kept(p) != p for p in raw.values())
        shown = [it | {k: vf.xml_kept(it[k]) for k in ("title", "notes", "password")}
                 for it in items]
        p = vf.passphrase(rng)
        cost = (8192, rng.choice([1, 2]), rng.choice([1, 2]))
        now = rng.choice([None, rng.randrange(1_500_000_000, 2_200_000_000)])
        label = word(rng, 1, 20)
        data = self.export(items, cost, p, label, now)
        v = self.read_all(data, p, self.wants_kpxc(), raw=raw)
        if v["label"] != label:
            self.fail("vault.export", {}, v["label"], label,
                      "the vault's name is not in the file", data)
        self.put_check(v, shown, 0, 0, data)
        self.rs_checks(data, p, self.unstripped(v, raw))

    def case_export(self):
        self.export_case()

    def foreign_case(self):
        """A file pykeepass makes: opens in Faraday with the same fields, and Faraday's seal
        keeps everything."""
        rng = self.rng
        p = vf.passphrase(rng)
        data, model = vf.py_file(rng, p)
        kpxc = self.wants_kpxc(0.25)
        v = self.read_all(data, p, kpxc)
        by = {(tuple(e["path"]), vf.fields_of(e)["Title"]): vf.fields_of(e) for e in v["entries"]}
        if by.keys() != model.keys():
            self.fail("pykeepass", {}, sorted(by), sorted(model), "pykeepass made other entries",
                      data)
        for key, f in model.items():
            for k, val in f.items():
                if by[key].get(k, "") != val:
                    self.fail("pykeepass", {"entry": key}, by[key].get(k), val,
                              f"field {k} is not what pykeepass was given", data)
        more = self.items([k for k in self.pick(1, 3) if k not in ("share", "secure_boot", "note")],
                          start=100, entries=rng.choice([0, 1]))
        self.seal_kept(data, p, v, more, kpxc)

    def case_foreign(self):
        self.foreign_case()

    def kpxc_case(self):
        """A file `keepassxc-cli` edits (groups, entries, notes, attachments), from a KDBX 4
        file of Faraday's: opens in Faraday, and Faraday's seal keeps it."""
        rng = self.rng
        self.kpxc_on()
        if not self.has_kpxc:
            self.second["uncovered"] += 1
            return
        p = vf.passphrase(rng)
        empty = self.create([], rng.choice([65_536, 262_144]), (8192, 1, 1), p, word(rng, 1, 10),
                            rng.choice([None, 1_700_000_000]))
        with readers.Kpxc(empty) as k:
            model = vf.kpxc_edit(rng, k, p)
            data = k.data()
        v = self.read_all(data, p, True, flips=rng.random() < 0.3)
        by = {(tuple(e["path"]), vf.fields_of(e)["Title"]): vf.fields_of(e) for e in v["entries"]}
        if by.keys() != model.keys():
            self.fail("keepassxc-cli", {}, sorted(by), sorted(model),
                      "keepassxc-cli made other entries", data)
        for key, f in model.items():
            for k, val in f.items():
                if k != "attachments" and by[key].get(k, "") != val:
                    self.fail("keepassxc-cli", {"entry": key}, by[key].get(k), val,
                              f"field {k} is not what keepassxc-cli was given", data)
        # Attachments: the names and bytes, as pykeepass and keepassxc-cli read them.
        for e in v["entries"]:
            key = (tuple(e["path"]), vf.fields_of(e)["Title"])
            got = dict(map(tuple, e["attachments"]))
            want = {n: b.hex() for n, b in model[key]["attachments"].items()}
            if got != want:
                self.fail("keepassxc-cli", {"entry": key}, got, want,
                          "pykeepass reads other attachments than keepassxc-cli put in", data)
        more = self.items(["wallet_policy", "passphrase"], start=100)
        out, after = self.seal_kept(data, p, v, more, True)
        with readers.Kpxc(out) as k:
            for key, f in model.items():
                path = "/".join(list(key[0]) + [key[1]])
                for name, b in f["attachments"].items():
                    if k.attachment(p, path, name) != b:
                        self.fail("keepassxc-cli", {"entry": key}, "other bytes", name,
                                  "an attachment is not kept by Faraday's seal", out)
                if sorted(k.attachments(p, path)) != sorted(f["attachments"]):
                    self.fail("keepassxc-cli", {"entry": key}, k.attachments(p, path),
                              sorted(f["attachments"]), "attachments differ after the seal", out)

    def case_kpxc(self):
        self.kpxc_case()

    # The files of tools/vectors/vault/.

    # Files Faraday refuses that an outside reader opens: its reader is stricter (§4, §7.1).
    # {file: (readers that open it, why Faraday refuses it)}
    WIDER = {
        "cut-stream.kdbx": ({"keepassxc-cli"}, "the block stream ends before its empty last block"),
        "refused-xml-after-root.kdbx": ({"keepassxc-cli"}, "an element after the root"),
        "refused-xml-base64.kdbx": ({"pykeepass", "keepassxc-cli"},
                                    "a protected value not canonical Base64"),
        "refused-xml-doctype.kdbx": ({"pykeepass"}, "a DOCTYPE"),
        "refused-xml-encoding.kdbx": ({"pykeepass"}, "another encoding"),
        "refused-xml-root.kdbx": ({"pykeepass"}, "a root that is not KeePassFile"),
        "refused-xml-too-deep.kdbx": ({"pykeepass", "keepassxc-cli"}, "nesting past 64"),
    }

    def refused_changed(self, data, at, mask, passphrase):
        """The file with `mask` flipped at byte `at`: Faraday and pykeepass refuse it."""
        changed = data[:at] + bytes([data[at] ^ mask]) + data[at + 1:]
        self.open_faraday(changed, passphrase, None)
        if readers.open_kdbx(changed, passphrase) is not None:
            self.fail("pykeepass", {"file": self.tag(changed)}, "opened", "refused",
                      "pykeepass opens a file with a version or cost bit changed", changed)

    def vectors(self):
        folder = ROOT / "tools" / "vectors" / "vault"
        kpxc = self.kpxc_on()
        pass_ = "correct horse battery staple"
        # The export vector (`tools/vectors/backup/`, the file an export is: a one-entry
        # vault), read by pykeepass and `keepassxc-cli` as Faraday reads it, with a byte
        # changed refused by all.
        vector = (ROOT / "tools/vectors/backup/backup-kdbx-words-12-english.kdbx").read_bytes()
        self.read_all(vector, pass_, kpxc)
        # The case `--fresh` once found (seed 62966c39701c90bed4224b29ed8cccdc): the major
        # version's high bit set, 4 read as 132. No reader may open it, nor the file with any
        # other bit of the version field changed.
        for at in range(8, 12):
            for bit in range(8):
                self.refused_changed(vector, at, 1 << bit, pass_)
        # Argon2's iterations with bit 44 set: refused from the header's hash, before any key
        # derivation, by all.
        at = vector.index(b"\x05\x01\x00\x00\x00I\x08\x00\x00\x00") + 10 + 5
        self.refused_changed(vector, at, 0x10, pass_)
        for path in sorted(folder.glob("*.kdbx")):
            data = path.read_bytes()
            name = path.name
            p = "caf\u00e9" if name == "vault-nfd.kdbx" else pass_
            opens = not name.startswith("refused-") and name != "cut-stream.kdbx"
            if opens:
                self.read_all(data, p, kpxc, flips=False)
                continue
            self.open_faraday(data, p, None)
            wider = self.WIDER.get(name, (set(), ""))[0]
            mine = set()
            if readers.open_kdbx(data, p) is not None:
                mine.add("pykeepass")
            if kpxc:
                with readers.Kpxc(data) as k:
                    if k.entries(p) is not None:
                        mine.add("keepassxc-cli")
                self.second["judged"] += 1
            else:
                mine |= wider & {"keepassxc-cli"}
                self.second["uncovered"] += 1
            if mine != wider:
                self.fail("vault.open", {"file": name}, sorted(mine), sorted(wider),
                          "the outside readers open other files Faraday refuses than the area "
                          "expects (WIDER)", data)
            self.known += len(mine)
            self.compared += 1


    # ----- keepass-rs, the file format only (F193 point 1) ----------------------

    def case(self, loop, i):
        """The cases of keepass-rs's loops are not the ones `keepassxc-cli` does not cover."""
        self.rs_case = loop.startswith("rs")
        super().case(loop, i)

    def rs_checks(self, data, passphrase, v, flip=True):
        """keepass-rs reads `data` as pykeepass's deep view `v` says, and refuses a wrong
        passphrase, the NFD form where it differs and a byte changed."""
        got = rs.read(data, passphrase)
        if got is None:
            self.fail("keepass-rs", {"file": self.tag(data)}, "refused", "opened",
                      "keepass-rs does not open what Faraday wrote", data)
        diff = rs.differs(got, v)
        if diff:
            self.fail("keepass-rs", {"file": self.tag(data)}, diff[1], diff[2],
                      f"{diff[0]} is not what pykeepass reads", data)
        self.format_judge["judged"] += 1
        self.compared += 1
        if rs.read(data, passphrase + "x") is not None:
            self.fail("keepass-rs", {"file": self.tag(data)}, "opened", "refused",
                      "keepass-rs opens under a wrong passphrase", data)
        nfd = unicodedata.normalize("NFD", passphrase)
        if nfd != passphrase and rs.read(data, nfd) is not None:
            self.fail("keepass-rs", {"file": self.tag(data)}, "opened", "refused",
                      "keepass-rs opens under the NFD form of an NFC passphrase", data)
        if flip:
            i = self.rng.randrange(len(data))
            bad = data[:i] + bytes([data[i] ^ (1 << self.rng.randrange(8))]) + data[i + 1:]
            if rs.read(bad, passphrase) is not None:
                self.fail("keepass-rs", {"file": self.tag(bad)}, "opened", "refused",
                          "keepass-rs opens a file with a byte changed", bad)
        self.compared += 1

    def rs_description(self):
        """What keepass-rs is asked to write, and the model of its entries
        ({(group path, title): fields}) as the foreign cases keep it."""
        rng = self.rng
        desc = {"label": word(rng, 1, 20), "cipher": rng.choice(["aes256", "chacha20"]),
                "kdf": rng.choice(["argon2d", "argon2id"]), "memory_kib": 8192,
                "iterations": rng.choice([1, 2, 3]), "parallelism": rng.choice([1, 2]),
                "compression": rng.random() < 0.5, "custom": [], "groups": [], "entries": []}
        if rng.random() < 0.5:
            desc["custom"].append(["other.vault", word(rng, 1, 20)])
        groups = [[]]
        for i in range(rng.choice([0, 1, 2, 3])):
            path = rng.choice(groups) + [f"{word(rng, 1, 12)} g{i}"]
            groups.append(path)
            desc["groups"].append(path)
        model = {}
        for i in range(rng.randint(1, 5)):
            path = rng.choice(groups)
            title = f"{word(rng, 1, 14)} e{i}"
            f = {"UserName": word(rng, 0, 10), "Password": word(rng, 0, 20),
                 "URL": rng.choice(["", "https://example.org/" + word(rng, 1, 5)]),
                 "Notes": word(rng, 0, 80, lines=True)}
            entry = {"path": path, "title": title, "user": f["UserName"],
                     "password": f["Password"], "url": f["URL"], "notes": f["Notes"],
                     "tags": rng.choice([[], ["one"], ["alpha", "beta"]]),
                     "icon": rng.choice([None, 0, 12, 30]),
                     "props": [[f"prop{j}", word(rng, 0, 30, lines=True), rng.random() < 0.5]
                               for j in range(rng.choice([0, 0, 1, 2]))],
                     "attachments": [[f"file{j}{rng.choice(['.bin', '.txt', ' \u00e9'])}",
                                      rng.randbytes(rng.choice([0, 5, 300, 3000])).hex()]
                                     for j in range(rng.choice([0, 0, 1, 2]))],
                     "custom": [["other.program", word(rng, 1, 20)]] if rng.random() < 0.3 else []}
            for k, val, _ in entry["props"]:
                f[k] = val
            desc["entries"].append(entry)
            model[(tuple(path), title)] = f
        return desc, model

    def case_rsread(self):
        """A vault Faraday writes and seals, read by keepass-rs."""
        rng = self.rng
        items = self.items(self.pick(), entries=rng.choice([0, 1, 2]))
        p = vf.passphrase(rng)
        size = rng.choice([262_144, 1_048_576]) if len(items) > 3 else rng.choice([65_536, 262_144])
        data = self.create(items, size, (8192, rng.choice([1, 2]), rng.choice([1, 2])), p,
                           word(rng, 1, 20), rng.choice([None, rng.randrange(1_500_000_000, 2_200_000_000)]))
        v = self.read_all(data, p, False)
        self.rs_checks(data, p, v)
        more = self.items([k for k in self.pick(0, 3) if k not in ("share", "secure_boot", "note")],
                          start=100)
        out, after = self.seal_kept(data, p, v, more, False)
        self.rs_checks(out, p, after, flip=False)

    def case_rswrite(self):
        """A file keepass-rs writes: Faraday opens it with the same fields, keepass-rs reads
        it as pykeepass does, and Faraday's seal keeps everything."""
        rng = self.rng
        p = vf.passphrase(rng)
        desc, model = self.rs_description()
        data = rs.write(desc, p)
        if data is None:
            self.fail("keepass-rs", {"description": desc}, "refused", "a file",
                      "keepass-rs does not write what it is asked")
        v = self.read_all(data, p, False)
        by = {(tuple(e["path"]), vf.fields_of(e)["Title"]): vf.fields_of(e) for e in v["entries"]}
        if by.keys() != model.keys():
            self.fail("keepass-rs", {}, sorted(by), sorted(model), "keepass-rs made other entries",
                      data)
        for key, f in model.items():
            for k, val in f.items():
                if by[key].get(k, "") != val:
                    self.fail("keepass-rs", {"entry": key}, by[key].get(k), val,
                              f"field {k} is not what keepass-rs was given", data)
        self.rs_checks(data, p, v)
        more = self.items([k for k in self.pick(1, 3) if k not in ("share", "secure_boot", "note")],
                          start=100, entries=rng.choice([0, 1]))
        out, after = self.seal_kept(data, p, v, more, False)
        self.rs_checks(out, p, after, flip=False)

    def case_rsfixed(self):
        """A loop of one case, so that `--case rsfixed:0` reaches it without the prelude,
        which stops where `keepassxc-cli` is missing."""
        self.rs_prelude()

    def rs_prelude(self):
        """One vault with an item of every kind read by keepass-rs, and the files keepass-rs
        writes that Faraday refuses (§4), each a known difference."""
        self.rs_case = True
        rng = self.rng
        try:
            p = vf.passphrase(rng)
            data = self.create(self.items(list(vf.KINDS), entries=1), 262_144, (8192, 1, 1), p,
                               word(rng, 1, 20), None)
            self.rs_checks(data, p, self.read_all(data, p, False))
            for name, (setting, why) in rs.REFUSED.items():
                desc = {"label": "refused", "memory_kib": 8192, "iterations": 1,
                        "parallelism": 1, "entries": [], "groups": []} | setting
                bad = rs.write(desc, p)
                if bad is None:
                    self.fail("keepass-rs", {"setting": name}, "refused", "a file",
                              f"keepass-rs does not write {why}")
                if rs.read(bad, p) is None:
                    self.fail("keepass-rs", {"setting": name}, "refused", "opened",
                              f"keepass-rs does not open its own file with {why}", bad)
                self.open_faraday(bad, p, None)
                self.format_judge["judged"] += 1
                self.known += 1
        finally:
            self.rs_case = False
