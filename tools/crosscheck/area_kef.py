"""KEF, Krux's encrypted code (`faraday_kef`, `faraday_codec::kef`,
`docs/KEF.md` §14), against Krux's own `src/krux/kef.py` (`tools/reference/krux/`,
vendored unchanged), run on the shims that stand in for the firmware's three
modules: `ucryptolib` (AES-256 in ECB, CBC, CTR and GCM, over PyCA
cryptography), `uhashlib_hw` (SHA-256 and PBKDF2 over `hashlib`) and `deflate`
(raw deflate at `wbits` -10).

- Each of the twelve versions, both ways: what Faraday seals under a key the
  area chose (`seal_with_key`) is Krux's `Cipher.encrypt` and `wrap` of the
  same plaintext, id, count and IV, byte for byte; what Krux seals is opened
  by Faraday (`open_with_key`) and what Faraday seals is opened by Krux's
  `Cipher.decrypt`. The versions that compress are not equal byte for byte
  (two compressors), so each opens the other's, and the plaintext is the
  one that went in; Krux's decryption inflates under zlib at `wbits` -10, so
  that is also where Faraday's written streams are read.
- The plaintexts are both sizes of entropy, short and long text, text of up
  to 1,024 bytes (the longest a compressed version is written for), text that
  ends in NUL, text whose hidden check ends in 0x00, repeated blocks and a
  single byte; ids of 0, 8 and 252 bytes and random ones; counts at every
  edge of `wrap` (10,000, a multiple of 10,000 up to 100,000,000, any count
  above 10,000 and under 2^24, and the ones either side).
- A code under a passphrase (`seal` and `open`, a key worked out from the
  passphrase, the id and the count): Krux's `Cipher(passphrase, id, count)`
  at counts from 10,000 to 30,000, and Krux's own vectors
  (`tools/vectors/kef/`), opened by both.
- Codes from a writer that skipped §4.1 (Krux's `encrypt` with
  `fail_unsafe=False`), a plaintext ending in NUL or a hidden check ending
  in 0x00, opened by both; envelopes cut, a byte changed, a count or a
  version changed, opened by both.
- `suggest_versions`: every mode at every length from 1 to 1,024 (one text of
  each), and drawn texts of the lengths where the rules change.

Known differences, each checked to still hold and counted:

- Faraday refuses what Krux writes or reads and the specification allows:
  an empty plaintext, a compressed version over 1,024 bytes, an id of 253
  bytes or more when reading, a count over 550,000 and a passphrase that is
  not printable ASCII when sealing under a passphrase; it reads a stream of
  exactly 102,400 bytes (Krux's firmware raises there: its read stops at the
  limit before it sees the end of the stream) and a stream whose matches reach
  back more than 1,024 bytes (Krux's window), which Krux refuses; and it
  refuses at once an envelope whose payload holds nothing but the IV and the
  checks, and a version 7 code is refused by one side for a repeated block when
  the other compressor's stream has none.
"""

import hashlib
import json
import zlib

from common import ROOT, Area, Mismatch, reference, use_reference

use_reference("krux", "shims")
use_reference("krux", "src")
from krux import kef as krux_kef  # noqa: E402

VERSIONS = [0, 1, 5, 6, 7, 10, 11, 12, 15, 16, 20, 21]
COMPRESSED = [7, 12, 16, 21]
NUL_PADDED = [0, 1, 5, 10]
MODE_NAMES = {"ecb": "AES-ECB", "cbc": "AES-CBC", "ctr": "AES-CTR", "gcm": "AES-GCM"}
VECTORS = ROOT / "tools" / "vectors" / "kef" / "vectors.json"
COMPRESS_MAX = 1024
INFLATE_LIMIT = 102_400
PASS_COUNTS = [10_000, 12_345, 20_000, 30_000]
# Every edge of `wrap`'s count: a multiple of 10,000 is stated as the
# quotient, 1 to 10,000; any other count above 10,000 and under 2^24 as
# itself.
EDGE_COUNTS = [
    0, 1, 9_999, 10_000, 10_001, 19_999, 20_000, 100_000, 123_456, 999_999, 1_000_000,
    16_777_215, 16_777_216, 16_780_000, 50_000_000, 99_990_000, 100_000_000,
    100_000_001, 100_010_000,
]


def iv_len(version):
    return {"ecb": 0, "cbc": 16, "ctr": 12, "gcm": 12}[mode_of(version)]


def mode_of(version):
    return {krux_kef.MODE_ECB: "ecb", krux_kef.MODE_CBC: "cbc",
            krux_kef.MODE_CTR: "ctr", krux_kef.MODE_GCM: "gcm"}[krux_kef.VERSIONS[version]["mode"]]


def cipher(key):
    """Krux's `Cipher` holding a key the area chose: the key is worked out
    from a passphrase (one round of the shim's PBKDF2 here), then replaced."""
    c = krux_kef.Cipher(b"x", b"y", 1)
    c._key = bytes(key)
    return c


def krux_seal(key, plaintext, version, id_, count, iv, fail_unsafe=True):
    payload = cipher(key).encrypt(plaintext, version, iv, fail_unsafe=fail_unsafe)
    return krux_kef.wrap(id_, version, count, payload)


def krux_open(key, envelope):
    """What Krux's `unwrap` and `decrypt` give; nothing, or an empty result,
    is the failure Faraday reports for both."""
    _id, version, _count, payload = krux_kef.unwrap(envelope)
    out = cipher(key).decrypt(payload, version)
    if not out:
        raise ValueError("did not open")
    return {"plaintext": bytes(out).hex()}


def krux_open_pass(passphrase, envelope):
    id_, version, count, payload = krux_kef.unwrap(envelope)
    out = krux_kef.Cipher(passphrase, id_, count).decrypt(payload, version)
    if not out:
        raise ValueError("did not open")
    return {"plaintext": bytes(out).hex()}


def krux_seal_pass(passphrase, plaintext, version, id_, count, iv):
    payload = krux_kef.Cipher(passphrase, id_, count).encrypt(plaintext, version, iv)
    return krux_kef.wrap(id_, version, count, payload)


def krux_suggest(plaintext, mode):
    return {"versions": krux_kef.suggest_versions(plaintext, MODE_NAMES[mode])}


class Kef(Area):
    name = "kef"
    faraday = (
        "`faraday_kef` (`seal`, `seal_with_key`, `open`, `open_with_key`, "
        "`suggest_versions`), `faraday_codec::kef::unwrap`"
    )
    reference = (
        "krux's `kef.py` (`krux/`, unchanged), by the krux developers, on three small "
        "shims for the firmware's AES, SHA-256 and PBKDF2, and deflate modules (PyCA "
        "cryptography's AES, `hashlib`, zlib at `wbits` -10)"
    )
    reference_checked = (
        "the vendored files are hashed against `references.sha256` before any run; "
        "krux's own vectors (`tools/vectors/kef/`) are required of the reference as well "
        "as of Faraday; every plaintext Faraday seals is opened by krux's `decrypt`, and "
        "every plaintext krux seals by Faraday, to the plaintext that went in"
    )
    vectors = (
        "krux's test data for versions 5, 15 and 20 (`tools/vectors/kef/`): a CTR code "
        "under `key`, a GCM code under `test key` at 100,000 rounds, and the version 5 "
        "code opened by `abc` and by a wrong passphrase that passes its 3-byte check"
    )
    corpus = (
        "each of the twelve versions sealed by Faraday and by krux, byte for byte for "
        "the ten that do not compress and opened by the other for all twelve, over "
        "entropy of 16 and 32 bytes, short and long text, text up to 1,024 bytes, "
        "text ending in NUL, text whose hidden check ends in 0x00, repeated blocks, "
        "ids of 0, 8, 252 and random lengths, every edge of the count, and wrong IV "
        "lengths; codes under a passphrase at 10,000 to 30,000 rounds, one of 200 "
        "characters with an id of 252 bytes, and keys of raw bytes (to 200 bytes) "
        "krux seals and Faraday opens; codes from a writer that skipped the NUL "
        "rules, with 1 to 8 NULs at the end; envelopes cut or changed; the suggested "
        "versions for each mode at every length to 1,024"
    )
    differences = [
        (
            "KEF: Faraday refuses an empty plaintext, a compressed version over 1,024 "
            "bytes, an id of 253 bytes or more when reading (krux's `unwrap` does not "
            "check it), a count over 550,000 and a passphrase that is not printable ASCII "
            "when it works out the key, and a stream that inflates to over 102,400 bytes; "
            "krux does none of these. Each is asked of krux and counted. Faraday does "
            "not suggest a compressed version over 1,024 bytes."
        ),
        (
            "KEF: a stream that inflates to exactly 102,400 bytes is read by Faraday "
            "and refused by krux's firmware (its read is capped at the limit and raises "
            "when it is asked for more before the stream has ended); one of 102,401 "
            "bytes or more is refused by both. A stream whose matches reach back more "
            "than 1,024 bytes is read by Faraday (any window to 32 KiB) and refused by "
            "krux's 1,024-byte window; Faraday writes none."
        ),
        (
            "KEF: Faraday's `unwrap` refuses at once a payload that holds nothing but "
            "the IV, the exposed check and the hidden check; krux's `unwrap` takes it "
            "apart and its decryption then fails. Both end in a code that does not open."
        ),
        (
            "KEF: version 7 (ECB with deflate) is refused for a repeated block by the "
            "side whose compressor makes a stream with one; the stream of the other "
            "has none, and that side's code is opened by both."
        ),
        (
            "KEF: the compressed versions are not equal byte for byte, as Faraday's "
            "deflate and zlib's write different streams. Each opens the other's; krux's "
            "decryption inflates under zlib at `wbits` -10."
        ),
    ]
    not_compared = (
        "the text forms and the classifier (krux's `kef.py` holds neither), the "
        "Reveal variant and the worker; whether Faraday notes that the check passed "
        "only after putting NULs back; a key entered as hex that Faraday writes under "
        "(it refuses a passphrase that is not printable ASCII), so keys of raw bytes "
        "are compared only when Faraday opens"
    )

    LOOPS = (("kef", 700), ("suggest", 120))

    # ----- one comparison ----------------------------------------------------

    def open_both(self, key, envelope, plaintext):
        """Faraday and krux open `envelope` alike; an envelope made as
        `plaintext` opens to it."""
        got = self.compare("kef.open", {"key": key.hex(), "envelope": envelope.hex()},
                           lambda: krux_open(key, envelope))
        if plaintext is not None and self.need(got, "kef.open")["plaintext"] != plaintext.hex():
            raise AssertionError("an envelope made from a plaintext opens to another")
        return got

    def cross(self, key, args, plaintext):
        """Both sides seal `plaintext`. A version that does not compress is
        equal byte for byte; one that does is opened by the other side.
        Returns Faraday's envelope, or None when both refused."""
        version = args["version"]
        id_, count, iv = bytes.fromhex(args["ident"]), args["count"], bytes.fromhex(args["iv"])
        inputs = dict(args, key=key.hex())
        if version not in COMPRESSED:
            got = self.compare("kef.seal", inputs, lambda: {
                "envelope": krux_seal(key, plaintext, version, id_, count, iv).hex()})
            if got[0] != "ok":
                return None
            envelope = bytes.fromhex(got[1]["envelope"])
            self.open_both(key, envelope, plaintext)
            return envelope
        rust = self.rust.call("kef.seal", **inputs)
        ref = reference(lambda: krux_seal(key, plaintext, version, id_, count, iv))
        self.compared += 1
        if rust[0] == "err" and ref[0] == "err":
            return None
        if rust[0] == "err" or ref[0] == "err":
            # Two compressors can make a duplicate block in one and not in the
            # other (version 7 only), and the one that refused says so; a
            # plaintext of up to 1,024 bytes is otherwise sealed by both.
            said = rust[1] if rust[0] == "err" else ref[1]
            sealed = ref[1] if rust[0] == "err" else bytes.fromhex(rust[1]["envelope"])
            if version == 7 and "Duplicate" in str(said):
                if len(set(_blocks(_payload(sealed)))) != len(_blocks(_payload(sealed))):
                    raise AssertionError("a version 7 code that was sealed repeats a block")
                self.known += 1
                self.open_both(key, sealed, plaintext)
                return None
            raise Mismatch(self.name, "kef.seal", inputs, rust, ref)
        envelope = bytes.fromhex(rust[1]["envelope"])
        if version == 7 and len(set(_blocks(_payload(envelope)))) != len(_blocks(_payload(envelope))):
            raise AssertionError("a version 7 code that was sealed repeats a block")
        self.open_both(key, envelope, plaintext)
        self.open_both(key, ref[1], plaintext)
        return envelope

    # ----- the corpus --------------------------------------------------------

    def plaintext(self, version):
        rng = self.rng
        shape = rng.choice(["entropy16", "entropy32", "short", "long", "up_to_max", "nul_end",
                            "repeated", "one", "check_zero", "words"])
        if shape == "entropy16":
            return self.randbytes(16)
        if shape == "entropy32":
            return self.randbytes(32)
        if shape == "short":
            return self.text(rng.randint(1, 31))
        if shape == "long":
            return self.text(rng.randint(33, 400))
        if shape == "up_to_max":
            return self.text(rng.randint(400, COMPRESS_MAX))
        if shape == "nul_end":
            return self.randbytes(rng.randint(1, 40)) + b"\x00" * rng.randint(1, 3)
        if shape == "repeated":
            block = self.randbytes(16)
            return block * rng.randint(2, 6) + self.randbytes(rng.randint(0, 20))
        if shape == "one":
            return bytes([rng.choice([0, 1, 65, 255])])
        if shape == "words":
            return " ".join(rng.choice(["abandon", "ability", "able", "about", "above", "zoo"])
                            for _ in range(rng.choice([12, 24]))).encode()
        # A plaintext whose hidden check of version 0 or 1 ends in 0x00.
        for _ in range(100_000):
            p = self.randbytes(rng.randint(8, 40))
            if hashlib.sha256(p).digest()[15] == 0:
                return p
        return self.randbytes(16)

    def text(self, n):
        rng = self.rng
        if rng.random() < 0.5:
            return bytes(rng.choice(b"abcdefghijklmnopqrstuvwxyz ") for _ in range(n))
        return self.randbytes(n)

    def an_id(self):
        rng = self.rng
        n = rng.choice([0, 8, 8, 252, rng.randint(1, 252), rng.randint(1, 20), 253, 255])
        return self.randbytes(n)

    def case_kef(self):
        rng = self.rng
        version = rng.choice(VERSIONS)
        plaintext = self.plaintext(version)
        key = self.randbytes(32)
        iv = self.randbytes(iv_len(version))
        r = rng.random()
        if r < 0.05:
            iv = self.randbytes(rng.choice([n for n in (0, 8, 12, 16, 17) if n != iv_len(version)]))
        count = rng.choice(EDGE_COUNTS + [10_000, 100_000, 100_000])
        args = {"plaintext": plaintext.hex(), "version": version, "ident": self.an_id().hex(),
                "count": count, "iv": iv.hex()}
        envelope = self.cross(key, args, plaintext)
        if envelope is None:
            return
        # Changed envelopes: cut, a byte changed, a count or a version changed
        # (never the first byte: krux's `unwrap` does not bound the id).
        for _ in range(3):
            self.open_both(key, self.damage(envelope), None)
        # A code under a passphrase, at a small count.
        if rng.random() < 0.2:
            self.passphrase_case(plaintext, version, iv)
        if rng.random() < 0.05:
            self.passphrase_case(plaintext, version, iv, long=True)
        if rng.random() < 0.1:
            self.raw_key_case(plaintext, version, iv)
        # A code from a writer that skipped §4.1.
        if rng.random() < 0.3:
            self.skipped_case(key)
        # Krux's writing of a plaintext beyond what Faraday writes, read by Faraday.
        if rng.random() < 0.05:
            self.krux_long_case(key, version)

    def damage(self, envelope):
        rng = self.rng
        e = bytearray(envelope)
        len_id = e[0]
        r = rng.random()
        if r < 0.3:
            return bytes(e[:rng.randint(1, len(e))])
        if r < 0.6:
            i = rng.randrange(1 + len_id + 1, len(e))  # the count, the IV, the ciphertext, the check
            e[i] ^= 1 << rng.randrange(8)
            return bytes(e)
        if r < 0.75:
            e[1 + len_id] = rng.choice(VERSIONS + [2, 3, 4, 8, 9, 13, 14, 17, 22, 255])
            return bytes(e)
        if r < 0.9:
            e[2 + len_id:5 + len_id] = rng.choice([0, 1, 9_999, 10_000, 10_001, 0xFFFFFF]) \
                .to_bytes(3, "big")
            return bytes(e)
        return bytes(e) + self.randbytes(rng.randint(1, 20))

    def passphrase_case(self, plaintext, version, iv, long=False):
        rng = self.rng
        chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 !#$%&()*+,-./:;<=>?@[]^_{|}~"
        passphrase = "".join(rng.choice(chars) for _ in range(200 if long else rng.randint(1, 40)))
        id_ = self.randbytes(252 if long else rng.choice([0, 4, 8, 16]))
        count = 10_000 if long else rng.choice(PASS_COUNTS)
        args = {"passphrase": passphrase, "plaintext": plaintext.hex(), "version": version,
                "ident": id_.hex(), "count": count, "iv": iv.hex()}
        if version in COMPRESSED:
            rust = self.rust.call("kef.seal_pass", **args)
            ref = reference(lambda: krux_seal_pass(passphrase, plaintext, version, id_, count, iv))
            self.compared += 1
            if rust[0] != "ok" or ref[0] != "ok":
                if rust[0] == ref[0]:
                    return
                said = rust[1] if rust[0] == "err" else ref[1]
                if version != 7 or "Duplicate" not in str(said):
                    raise Mismatch(self.name, "kef.seal_pass", args, rust, ref)
                self.known += 1  # (a repeated block in one compressor's stream)
                envelopes = [ref[1] if rust[0] == "err" else bytes.fromhex(rust[1]["envelope"])]
            else:
                envelopes = [bytes.fromhex(rust[1]["envelope"]), ref[1]]
        else:
            rust = self.compare("kef.seal_pass", args, lambda: {
                "envelope": krux_seal_pass(passphrase, plaintext, version, id_, count, iv).hex()})
            if rust[0] != "ok":
                return
            envelopes = [bytes.fromhex(rust[1]["envelope"])]
        for env in envelopes:
            got = self.compare("kef.open_pass", {"passphrase": passphrase, "envelope": env.hex()},
                               lambda env=env: krux_open_pass(passphrase, env))
            if self.need(got, "kef.open_pass")["plaintext"] != plaintext.hex():
                raise AssertionError("a code under a passphrase opens to another plaintext")
            self.compare("kef.open_pass", {"passphrase": passphrase + "x", "envelope": env.hex()},
                         lambda env=env: krux_open_pass(passphrase + "x", env))

    def raw_key_case(self, plaintext, version, iv):
        """A key of raw bytes (1 to 200, over HMAC's 64-byte block too), as
        Enter as hex gives it: Krux seals under it, Faraday opens."""
        rng = self.rng
        key = self.randbytes(rng.choice([1, 16, 32, 63, 64, 65, rng.randint(1, 200), 200]))
        id_ = self.randbytes(rng.choice([0, 8, 252]))
        made = reference(lambda: krux_seal_pass(key, plaintext, version, id_, 10_000, iv))
        if made[0] != "ok":
            return
        got = self.compare("kef.open_pass", {"passphrase_hex": key.hex(), "envelope": made[1].hex()},
                           lambda: krux_open_pass(key, made[1]))
        if self.need(got, "kef.open_pass")["plaintext"] != plaintext.hex():
            raise AssertionError("a code under a raw key opens to another plaintext")

    def skipped_case(self, key):
        """Krux's writer with `fail_unsafe` off, on a plaintext the strict
        writer refuses (a NUL end, or a hidden check ending in 0x00), opened
        by Faraday and by Krux."""
        rng = self.rng
        version = rng.choice(NUL_PADDED)
        plain = self.randbytes(rng.randint(1, 40))
        if version in (5, 10) or rng.random() < 0.5:
            # (1 to 8 NULs: version 5 opens with 4 and fails with 5, version 10
            # opens with 5 and fails with 6, and the others reach theirs)
            plain = plain.rstrip(b"\x00") + b"\x01" + b"\x00" * rng.randint(1, 8)
        else:
            for _ in range(20_000):
                if hashlib.sha256(plain).digest()[15] == 0:
                    break
                plain = self.randbytes(rng.randint(8, 40))
        iv = self.randbytes(iv_len(version))
        env = krux_seal(key, plain, version, b"skipped", 10_000, iv, fail_unsafe=False)
        got = self.open_both(key, env, None)
        # What opens is the plaintext that went in, NULs put back included.
        if got[0] == "ok" and got[1]["plaintext"] != plain.hex():
            raise Mismatch(self.name, "kef.open", {"key": key.hex(), "envelope": env.hex()},
                           got, ("ok", {"plaintext": plain.hex()}),
                           "a code from a writer that skipped §4.1 opens to another plaintext")

    def krux_long_case(self, key, version):
        """A plaintext over 1,024 bytes that Krux seals and Faraday opens;
        Faraday refuses to write it."""
        if version not in COMPRESSED:
            return
        rng = self.rng
        plain = self.text(rng.randint(COMPRESS_MAX + 1, 4000))
        args = {"plaintext": plain.hex(), "version": version, "ident": "00",
                "count": 10_000, "iv": self.randbytes(iv_len(version)).hex()}
        got = self.rust.call("kef.seal", key=key.hex(), **args)
        env = reference(lambda: krux_seal(key, plain, version, b"\x00", 10_000,
                                          bytes.fromhex(args["iv"])))
        self.compared += 1
        if got[0] != "err" or env[0] != "ok":
            raise Mismatch(self.name, "kef.seal", args, got, env, "a known difference no longer holds")
        self.known += 1
        self.open_both(key, env[1], plain)

    def case_suggest(self):
        rng = self.rng
        mode = rng.choice(list(MODE_NAMES))
        n = rng.choice([1, 2, 16, 17, 31, 32, 33, 119, 120, 121, rng.randint(1, COMPRESS_MAX),
                        COMPRESS_MAX])
        p = self.text(n)
        r = rng.random()
        if r < 0.3:
            p = p[:1] + (bytes(16) if n > 32 else b"") + p[1:]  # (a block of zeros)
            p = p[:COMPRESS_MAX]
        elif r < 0.5:
            block = self.randbytes(16)
            p = (block * (n // 16 + 1))[:n]
        elif r < 0.65:
            p = p[:-1] + b"\x00"
        self.compare("kef.suggest", {"plaintext": p.hex(), "mode": mode},
                     lambda: krux_suggest(p, mode))

    # ----- the prelude -------------------------------------------------------

    def published(self):
        """Krux's own vectors, opened by both under their passphrases."""
        d = json.loads(VECTORS.read_text())
        cases = [(d["ctr"]["envelope"], d["ctr"]["passphrase"], d["ctr"]["plaintext"]),
                 (d["gcm"]["envelope"], d["gcm"]["passphrase"], d["gcm"]["plaintext"]),
                 (d["grind"]["envelope"], d["grind"]["right"]["passphrase"],
                  d["grind"]["right"]["plaintext"]),
                 (d["grind"]["envelope"], d["grind"]["wrong_that_passes"]["passphrase"],
                  d["grind"]["wrong_that_passes"]["plaintext"])]
        for envelope, passphrase, plaintext in cases:
            env = bytes.fromhex(envelope)
            got = self.compare("kef.open_pass", {"passphrase": passphrase, "envelope": envelope},
                               lambda env=env, p=passphrase: krux_open_pass(p, env),
                               expected=("ok", {"plaintext": plaintext}))
            self.need(got, "kef.open_pass")
            self.compare("kef.unwrap", {"envelope": envelope},
                         lambda env=env: _unwrapped(env))

    def refusals(self):
        """Envelopes both refuse or both take apart: `i` of 0, an unknown
        version, a payload of the wrong size, an empty ciphertext; and what
        both refuse to write at the edges of the id and the count."""
        for version in VERSIONS:
            for payload_len in range(0, 70):
                for i in (0, 1, 9_999, 10_000, 10_001, 0xFFFFFF):
                    env = bytes([2]) + b"id" + bytes([version]) + i.to_bytes(3, "big") \
                        + self.randbytes(payload_len)
                    self.unwrap_both(env)
        for version in [2, 3, 4, 8, 9, 13, 14, 17, 18, 19, 22, 100, 255]:
            env = bytes([2]) + b"id" + bytes([version]) + (1).to_bytes(3, "big") + self.randbytes(48)
            self.compare("kef.unwrap", {"envelope": env.hex()}, lambda env=env: _unwrapped(env))
        for env in (b"", b"\x00", b"\x05abc", b"\x00\x05"):
            self.compare("kef.unwrap", {"envelope": env.hex()}, lambda env=env: _unwrapped(env))
        key = self.randbytes(32)
        for version in VERSIONS:
            for count in EDGE_COUNTS:
                for id_len in (0, 8, 252, 253):
                    plain = self.randbytes(20)
                    args = {"plaintext": plain.hex(), "version": version,
                            "ident": self.randbytes(id_len).hex(), "count": count,
                            "iv": self.randbytes(iv_len(version)).hex()}
                    self.cross(key, args, plain)

    def unwrap_both(self, env):
        """`unwrap` on both sides. Faraday refuses at once a payload with
        nothing but the IV, an exposed check and a hidden check, which Krux
        takes apart and fails to decrypt: that is a known difference, asked
        of both and counted."""
        kept = reference(lambda: _unwrapped(env))
        bare = kept[0] == "ok" and _bare(kept[1])
        if not bare:
            self.compare("kef.unwrap", {"envelope": env.hex()}, lambda: _unwrapped(env))
            return
        got = self.rust.call("kef.unwrap", envelope=env.hex())
        self._known(got[0] == "err", "a payload with no ciphertext", {"envelope": env.hex()}, got, kept)

    def known_differences(self):
        key = self.randbytes(32)
        # Empty: Krux seals it (versions that need no last byte); Faraday refuses.
        args = {"plaintext": "", "version": 6, "ident": "00", "count": 10_000, "iv": ""}
        got = self.rust.call("kef.seal", key=key.hex(), **args)
        ref = reference(lambda: krux_seal(key, b"", 6, b"\x00", 10_000, b""))
        self._known(got[0] == "err" and ref[0] == "ok", "an empty plaintext", args, got, ref)
        # An id of 253 bytes is read by Krux's `unwrap`; Faraday refuses.
        env = bytes([253]) + b"i" * 253 + bytes([15]) + (1).to_bytes(3, "big") + self.randbytes(40)
        got = self.rust.call("kef.unwrap", envelope=env.hex())
        ref = reference(lambda: _unwrapped(env))
        self._known(got[0] == "err" and ref[0] == "ok", "an id of 253 bytes", {"envelope": env.hex()},
                    got, ref)
        # Streams at the inflate limit. Krux's firmware raises at 102,400 and
        # over (its read stops at the limit before the stream's end): one
        # byte under, both open; at the limit, Faraday opens and Krux does not;
        # over it, both refuse.
        key = self.randbytes(32)
        for version in COMPRESSED:
            iv = self.randbytes(iv_len(version))
            for size in (INFLATE_LIMIT - 1, INFLATE_LIMIT, INFLATE_LIMIT + 1):
                made = reference(lambda: krux_seal(key, bytes(size), version, b"bomb", 10_000, iv))
                if made[0] != "ok":
                    continue  # (version 7: a duplicate block in zlib's stream)
                env = made[1]
                if size == INFLATE_LIMIT:
                    got = self.rust.call("kef.open", key=key.hex(), envelope=env.hex())
                    ref = reference(lambda env=env: krux_open(key, env))
                    self._known(got == ("ok", {"plaintext": bytes(size).hex()}) and ref[0] == "err",
                                f"v{version} inflating to the limit", {"envelope": env.hex()}, got, ref)
                else:
                    self.open_both(key, env, bytes(size) if size < INFLATE_LIMIT else None)
        # A stream whose matches reach back more than 1,024 bytes (as the
        # firmware's window does not): Faraday reads it, Krux refuses.
        block = self.randbytes(1100)
        plain = block + block
        for version in COMPRESSED:
            iv = self.randbytes(iv_len(version))
            env = _wide_window_seal(key, plain, version, iv)
            if env is None:
                continue  # (version 7: a duplicate block)
            got = self.rust.call("kef.open", key=key.hex(), envelope=env.hex())
            ref = reference(lambda env=env: krux_open(key, env))
            self._known(got == ("ok", {"plaintext": plain.hex()}) and ref[0] == "err",
                        f"v{version} with a match back over 1,024 bytes", {"envelope": env.hex()},
                        got, ref)
        # Under a passphrase Faraday writes: a count over 550,000 and a passphrase that is
        # not printable ASCII are refused before any work; Krux has no such rule.
        base = {"plaintext": "00" * 16, "version": 20, "ident": "00", "iv": "00" * 12}
        for passphrase, count in (("key", 560_000), ("kéy", 10_000), ("", 10_000),
                                  ("k" * 201, 10_000), ("tab\t", 10_000)):
            args = dict(base, passphrase=passphrase, count=count)
            got = self.rust.call("kef.seal_pass", **args)
            ref = reference(lambda: krux_seal_pass(passphrase, bytes(16), 20, b"\x00", count,
                                                   bytes(12)))
            self._known(got[0] == "err" and ref[0] == "ok",
                        f"the passphrase {passphrase[:6]!r} at {count}", args, got, ref)
        # No compressed version is suggested over 1,024 bytes; krux suggests one.
        for mode, versions in (("ecb", [6]), ("cbc", [11]), ("ctr", [15]), ("gcm", [20])):
            p = bytes(self.rng.choice(b"abcdefghijklmnopqrstuvwxyz ") for _ in range(COMPRESS_MAX + 1))
            got = self.rust.call("kef.suggest", plaintext=p.hex(), mode=mode)
            ref = reference(lambda p=p, mode=mode: krux_suggest(p, mode))
            self._known(got == ("ok", {"versions": versions}) and ref[0] == "ok"
                        and any(v in COMPRESSED for v in ref[1]["versions"]),
                        f"suggestions for {mode} over 1,024", {"mode": mode}, got, ref)

    def _known(self, holds, what, inputs, rust, ref):
        self.compared += 1
        if not holds:
            raise Mismatch(self.name, "kef", inputs, rust, ref,
                           f"the known difference no longer holds: {what}")
        self.known += 1

    def every_length(self):
        """Each mode's suggestion at every length from 1 to 1,024."""
        for mode in MODE_NAMES:
            for n in range(1, COMPRESS_MAX + 1):
                p = self.text(n)
                self.compare("kef.suggest", {"plaintext": p.hex(), "mode": mode},
                             lambda p=p, mode=mode: krux_suggest(p, mode))

    def retries(self):
        """Versions 0 and 1 with a hidden check ending in one and in two
        0x00 bytes, at each length up to 40, sealed by Krux with
        `fail_unsafe` off and opened by both: the retry that moves bytes from
        the check back to the plaintext, at its deeper counts."""
        key = self.randbytes(32)
        for version in (0, 1):
            for zeros in (1, 2):
                for n in range(1, 41):
                    mask = b"\x00" * zeros
                    for _ in range(2_000_000):
                        plain = self.randbytes(n)
                        if hashlib.sha256(plain).digest()[16 - zeros:16] == mask:
                            break
                    else:
                        continue
                    env = krux_seal(key, plain, version, b"retry", 10_000,
                                    self.randbytes(iv_len(version)), fail_unsafe=False)
                    self.open_both(key, env, plain)

    def long_passphrase(self):
        """A passphrase of 200 characters with an id of 252 bytes, sealed and
        opened both ways at every kind of mode, so PBKDF2 hashes a long key
        and a long salt."""
        for version in (5, 10, 15, 20):
            self.passphrase_case(self.randbytes(24), version, self.randbytes(iv_len(version)),
                                 long=True)

    def prelude(self):
        self.published()
        self.every_length()
        self.retries()
        self.long_passphrase()
        self.refusals()
        self.known_differences()


def _payload(envelope):
    return envelope[1 + envelope[0] + 4:]


def _blocks(payload):
    return [payload[i:i + 16] for i in range(0, len(payload), 16)]


def _unwrapped(env):
    id_, version, count, payload = krux_kef.unwrap(env)
    return {"id": bytes(id_).hex(), "version": version, "count": count,
            "payload": bytes(payload).hex()}


def _wide_window_seal(key, plaintext, version, iv):
    """Krux's `encrypt` with its compression at the full 32 KiB window, as
    a writer other than Krux's firmware would write; None when it refuses."""
    saved = krux_kef._deflate

    def deflate(data):
        c = zlib.compressobj(9, zlib.DEFLATED, -15)
        return c.compress(data) + c.flush()

    krux_kef._deflate = deflate
    try:
        made = reference(lambda: krux_seal(key, plaintext, version, b"wide", 10_000, iv))
    finally:
        krux_kef._deflate = saved
    return made[1] if made[0] == "ok" else None


def _bare(out):
    """A payload with nothing but the IV, the exposed check and the hidden
    check: `_unwrapped_strict`'s bound."""
    spec = krux_kef.VERSIONS[out["version"]]
    extra = krux_kef.MODE_IVS.get(spec["mode"], 0) + max(0, spec.get("auth", 0))
    hidden = -min(0, spec.get("auth", 0))
    return len(bytes.fromhex(out["payload"])) - extra <= hidden
