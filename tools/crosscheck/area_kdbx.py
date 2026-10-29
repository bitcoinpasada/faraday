"""The KDBX 4 export (`faraday_backup::kdbx`) read by pykeepass, the
library KeePass tooling in Python uses, which knows nothing of this
repository. pykeepass is the distribution's package (Debian and Ubuntu:
`python3-pykeepass`), as the vault area's Argon2 and libsodium are; the
area stops and names it when it is missing, so a run without it fails
rather than passing without the check.

What the area expects a KeePass reader to find, from `docs/KDBX.md` and
the writer's statement of what it puts in a file: KDBX 4.0, ChaCha20,
Argon2id at the cost asked; one group named `Faraday`; one entry per
item in order, its Title and Notes as written but for control
characters other than tab, line feed and carriage return, which are
dropped, and its Password as written. Faraday writes the Password's
bytes as they are, encrypted (`Protected`); pykeepass, having decrypted
one, strips what XML 1.0 cannot hold (its `UnprotectedStream`), so a
Password with such a control character is expected as pykeepass reads
it and counted as a known difference. A database of no items or
more than eight, a field over 16 KiB, and a cost Argon2id refuses are
not written.

- The repository's own export (`tools/vectors/backup/`), opened with its
  passphrase.
- Random databases of one to eight items whose fields hold XML's markup
  characters, tabs, line ends, other control characters and text in
  several scripts, at random small costs: every entry read back, then
  the writer's own check against pykeepass's opening under the right
  passphrase, a wrong one, and with a byte changed.
"""

import os
import re
import tempfile

from common import ROOT, Area, HarnessError, Mismatch

try:
    from pykeepass import PyKeePass
except ImportError as e:  # pragma: no cover - the machine's packages
    raise HarnessError(
        "the kdbx area needs the distribution's pykeepass (Debian and Ubuntu: python3-pykeepass;"
        f" Fedora: python3-pykeepass; Arch: python-pykeepass): {e}") from e

SEED_LEN = 32 + 12 + 32 + 64 + 16 * (1 + 8)
MAX_ITEMS = 8
MAX_FIELD = 16 * 1024
VECTOR = ROOT / "tools/vectors/backup/backup-kdbx-words-12-english.kdbx"


def kept(text):
    """What a reader finds of a Title or Notes field."""
    return "".join(c for c in text if c in "\t\n\r" or not (ord(c) < 0x20 or ord(c) == 0x7F))


def open_kdbx(data, passphrase):
    """pykeepass's reading of `data`, or None when it will not open.

    pykeepass picks the header and body by the major version with a
    construct `Switch` that has no default: a major version other than
    3 or 4 parses as an empty header and no body, and `PyKeePass`
    returns without having derived a key or checked a hash or an HMAC.
    KeePass and KeePassXC refuse such a file (an unknown major version;
    the version is also inside the header's SHA-256 and HMAC), so a file
    is opened here only when pykeepass parsed a KDBX 4 body.

    pykeepass also runs the key derivation the header states before it
    checks the header's SHA-256, so a changed bit in the Argon2
    iterations (2^44 passes) holds it for good; KeePass and KeePassXC
    check the hash first and refuse at once. pykeepass is asked the same
    way: first with a key given and nothing decrypted, which checks the
    version and the header's hash and derives nothing, and only then
    with the passphrase."""
    fd, path = tempfile.mkstemp(suffix=".kdbx")
    try:
        with os.fdopen(fd, "wb") as f:
            f.write(data)
        try:
            head = PyKeePass(path, password=passphrase, transformed_key=bytes(32), decrypt=False)
            if head.version[0] != 4 or head.kdbx.body is None:
                return None
            kp = PyKeePass(path, password=passphrase)
        except Exception:  # noqa: BLE001 - every failure to open is a refusal
            return None
        if kp.version[0] != 4 or kp.kdbx.body is None:
            return None
        return kp
    finally:
        os.unlink(path)


XML_INVALID = re.compile("[^\u0020-\ud7ff\u0009\u000a\u000d\ue000-\ufffd\U00010000-\U0010ffff]+")


def contents(kp):
    return {
        "version": list(kp.version),
        "cipher": kp.encryption_algorithm,
        "kdf": kp.kdf_algorithm,
        "group": kp.root_group.name,
        "entries": [[e.title or "", e.notes or "", e.password or ""] for e in kp.entries],
    }


class Kdbx(Area):
    name = "kdbx"
    faraday = (
        '`faraday_backup::kdbx` (the KDBX 4 export and its own check)'
    )
    reference = (
        "pykeepass, the distribution's package (`python3-pykeepass`)"
    )
    reference_checked = (
        "pykeepass is the distribution's package and the area stops and names it when "
        'it is missing, so a run without it fails; the versions are written into the '
        'stamp'
    )
    vectors = (
        "the repository's export (`tools/vectors/backup/`), written again byte for byte "
        'and opened'
    )
    corpus = (
        'databases of one to eight items with markup characters, control characters, '
        'line ends and several scripts, at small Argon2id costs, every entry read back; '
        "the writer's check against pykeepass under the right passphrase, a wrong one "
        'and a changed byte'
    )
    differences = [
        (
            'KDBX: pykeepass strips the characters XML 1.0 cannot hold from a protected '
            "value after decrypting it; Faraday writes a Password's bytes as they are. The "
            "area expects pykeepass's reading and counts the items that differ. pykeepass "
            'also opens a file whose major version is neither 3 nor 4 without reading past '
            'it, deriving a key or checking a hash; KeePass refuses such a file, and the '
            'area counts a file as opened only when pykeepass read a KDBX 4 body.'
        ),
        (
            'pykeepass runs the key derivation the header states before it checks the '
            "header's SHA-256, so a changed bit in the Argon2 iterations (2^44 passes) "
            'holds it for good; KeePass and KeePassXC check the hash first and refuse at '
            'once. The area asks pykeepass first with a key given and nothing decrypted, '
            "which checks the version and the header's hash and derives nothing, and only "
            'then with the passphrase.'
        ),
    ]
    not_compared = (
        'none'
    )

    def __init__(self, *a):
        super().__init__(*a)
        self.known = 0

    def write(self, items, passphrase, cost, seed):
        req = {"items": [{"title": t.encode().hex(), "notes": n.encode().hex(),
                          "secret": s.encode().hex()} for t, n, s in items],
               "passphrase": passphrase.encode().hex(), "memory_kib": cost[0],
               "passes": cost[1], "lanes": cost[2], "seed": seed.hex()}
        got = self.rust.call("kdbx.write", **req)
        self.compared += 1
        refuse = (not 1 <= len(items) <= MAX_ITEMS
                  or any(len(f.encode()) > MAX_FIELD for i in items for f in i)
                  or cost[2] < 1 or cost[1] < 1 or cost[0] < 8 * cost[2])
        if refuse or got[0] != "ok":
            if refuse != (got[0] != "ok"):
                raise Mismatch(self.name, "kdbx.write", req, got,
                               ("err", None) if refuse else ("ok", "a file"))
            return None
        data = bytes.fromhex(got[1]["file"])
        kp = open_kdbx(data, passphrase)
        want = {"version": [4, 0], "cipher": "chacha20", "kdf": "argon2id", "group": "Faraday",
                "entries": [[kept(t), kept(n), XML_INVALID.sub("", s)] for t, n, s in items]}
        self.known += sum(XML_INVALID.search(s) is not None for _, _, s in items)
        found = contents(kp) if kp else None
        if found != want:
            raise Mismatch(self.name, "kdbx.write", req, found, want,
                           "pykeepass does not read what Faraday wrote")
        return data

    def verify(self, data, passphrase):
        return self.compare("kdbx.verify", {"file": data.hex(),
                                            "passphrase": passphrase.encode().hex()},
                            lambda: {"valid": open_kdbx(data, passphrase) is not None})

    def published(self):
        """The vector as its README states it: written again from the
        same inputs, byte for byte, and read by pykeepass."""
        words = " ".join(["abandon"] * 11 + ["about"])
        passphrase = "correct horse battery staple"
        item = [("73c5da0a", "12 words, English", words)]
        data = self.write(item, passphrase, (65536, 3, 1),
                          bytes(i % 256 for i in range(SEED_LEN)))
        if data != VECTOR.read_bytes():
            raise AssertionError("Faraday no longer writes tools/vectors/backup's KDBX file")
        self.verify(data, passphrase)
        # The case `--fresh` found (seed 62966c39701c90bed4224b29ed8cccdc):
        # the major version's high bit set, 4 read as 132. Neither side
        # may open it, nor the file with any other bit of the version
        # field changed.
        for at in range(8, 12):
            for bit in range(8):
                changed = data[:at] + bytes([data[at] ^ (1 << bit)]) + data[at + 1:]
                self.compare("kdbx.verify", {"file": changed.hex(),
                                             "passphrase": passphrase.encode().hex()},
                             lambda changed=changed: {
                                 "valid": open_kdbx(changed, passphrase) is not None},
                             expected=("ok", {"valid": False}))
        # Argon2's iterations with bit 44 set: refused from the header's
        # hash, before any key derivation, by both.
        at = data.index(b"\x05\x01\x00\x00\x00I\x08\x00\x00\x00") + 10 + 5
        changed = data[:at] + bytes([data[at] ^ 0x10]) + data[at + 1:]
        self.compare("kdbx.verify", {"file": changed.hex(),
                                     "passphrase": passphrase.encode().hex()},
                     lambda: {"valid": open_kdbx(changed, passphrase) is not None},
                     expected=("ok", {"valid": False}))

    def text(self, n):
        rng = self.rng
        pool = ("abcxyz ABC 0189 <>&\"' \t\n\r \x01\x07\x1f\x7f é ß ñ 日本語 ключ"
                " 🔑 abandon about zoo")
        return "".join(rng.choice(pool) for _ in range(n))

    def corpus_case(self):
        rng = self.rng
        count = rng.choice([1, 1, 2, rng.randint(1, MAX_ITEMS), 0, MAX_ITEMS + 1])
        items = [(self.text(rng.randint(0, 40)), self.text(rng.choice([0, 10, 300])),
                  self.text(rng.choice([0, 20, 200]))) for _ in range(count)]
        if rng.random() < 0.03 and items:
            items[0] = (items[0][0], "n" * (MAX_FIELD + 1), items[0][2])
        lanes = rng.choice([1, 1, 2])
        cost = (rng.choice([8 * lanes, 16, 64, rng.randint(8 * lanes, 256)]),
                rng.randint(1, 3), lanes)
        if rng.random() < 0.03:
            cost = (rng.choice([0, 7]), cost[1], 1)
        passphrase = self.text(rng.randint(1, 30))
        data = self.write(items, passphrase, cost, self.randbytes(SEED_LEN))
        if data is None:
            return
        self.verify(data, passphrase)
        self.verify(data, passphrase + "x")
        i = rng.randrange(len(data))
        self.verify(data[:i] + bytes([data[i] ^ (1 << rng.randrange(8))]) + data[i + 1:],
                    passphrase)

    LOOPS = (("corpus", 120),)

    def prelude(self):
        self.published()

    def case_corpus(self):
        self.corpus_case()
