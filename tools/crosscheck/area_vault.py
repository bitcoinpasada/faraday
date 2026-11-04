"""What is left of the vault area (`docs/VAULT.md`): Argon2id and the
payload layouts of §7, against a reader written here from that document,
over Argon2id from argon2-cffi, the distribution's package (F79,
`docs/CROSSCHECK.md` §10.4).

The vault file is KDBX; it is judged by its own later area run, not yet
built for what the store writes. The checks of Faraday's earlier container
(HKDF, XChaCha20-Poly1305, the slot record, whole files) are gone with it.

Before anything else the area checks argon2-cffi against `tools/vectors/kdf/`'s
known answer (`tools/vectors/vault-primitives/`). Then:

- Argon2id (`faraday_backup::argon2id`) at random small costs, salts and
  passwords, with the costs and salts Argon2 refuses.
- The payload layouts of §7: words, seed and sheet, made and read, with
  every way a payload can be malformed, and a sheet past 16 KiB.
- A passphrase's NFC form (§3) against Python's `unicodedata`: NFD
  letters, Hangul jamo, the singletons NFC replaces, combining marks in
  either order, the compatibility characters NFC keeps, then random text.
  The area stops where Python's Unicode is newer than
  `unicode-normalization`'s.

The reference is the document alone.
"""

import json
import unicodedata

from common import ROOT, Area, HarnessError

try:
    from argon2.low_level import Type, hash_secret_raw
except ImportError as e:  # pragma: no cover - the machine's packages
    raise HarnessError(
        "the vault area needs the distribution's Argon2 binding, argon2-cffi "
        "(Debian and Ubuntu: python3-argon2; Fedora: python3-argon2-cffi; Arch: "
        f"python-argon2_cffi): {e}") from e

VECTORS = json.loads((ROOT / "tools/vectors/vault-primitives/vectors.json").read_text())

LANGUAGES = 10
MAX_SHEET = 16 * 1024


class Refused(Exception):
    """A refusal, named as `faraday_vault::Error` names its kinds."""

    def __init__(self, kind, detail=""):
        super().__init__(f"{kind}: {detail}" if detail else kind)
        self.kind = kind


def argon2id(password, salt, memory_kib, passes, lanes):
    return hash_secret_raw(secret=bytes(password), salt=bytes(salt), time_cost=passes,
                           memory_cost=memory_kib,
                           parallelism=lanes, hash_len=32, type=Type.ID, version=19)


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
        "`faraday_vault`'s payload layouts and a passphrase's NFC form with the Unicode version "
        "it is made under, and `faraday_backup::argon2id`"
    )
    reference = (
        '`docs/VAULT.md` §7 written out in `area_vault.py`, over argon2-cffi '
        "(the distribution's package) and Python's `unicodedata`"
    )
    reference_checked = (
        "`check_primitives` runs before anything else: argon2-cffi against "
        "`tools/vectors/kdf/`'s known answer; the version of argon2-cffi is written into the "
        "stamp; the area stops where Python's Unicode is newer than "
        "`unicode-normalization`'s"
    )
    vectors = (
        "the Argon2id known answer (`tools/vectors/vault-primitives/`)"
    )
    corpus = (
        'Argon2id at small random costs and the costs it refuses; payloads made, read '
        'and malformed; passphrases in NFC, by case and at random. The KDBX vault file is not '
        'in this area yet'
    )
    differences = []
    not_compared = (
        'the KDBX vault file, judged by its own later area run'
    )

    def check_primitives(self):
        """The area's own piece against its published answer."""
        a = VECTORS["argon2id"]
        if argon2id(bytes.fromhex(a["password"]), bytes.fromhex(a["salt"]), a["memory_kib"],
                    a["passes"], a["lanes"]).hex() != a["key"]:
            raise HarnessError("argon2-cffi fails the known Argon2id answer")

    # Each operation, both sides.

    def argon(self, password, salt, memory, passes, lanes, expected=None):
        return self.compare(
            "vault.argon2id",
            {"password": password.hex(), "salt": salt.hex(), "memory_kib": memory,
             "passes": passes, "lanes": lanes},
            lambda: {"key": argon2id(password, salt, memory, passes, lanes).hex()}, expected)

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

    LOOPS = (("argon", 40), ("payload", 1500), ("nfc", 1000))

    def prelude(self):
        self.check_unicode()
        self.check_primitives()
        a = VECTORS["argon2id"]
        self.argon(bytes.fromhex(a["password"]), bytes.fromhex(a["salt"]), a["memory_kib"],
                   a["passes"], a["lanes"], ("ok", {"key": a["key"]}))
        self.big_sheet()
        self.nfc_cases()

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
