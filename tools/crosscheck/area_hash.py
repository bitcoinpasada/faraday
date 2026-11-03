"""The Hashes and Encodings calculators, and the hash of a file, against
Bitcoin Core's functional-test framework (`core/test_framework/`):
`crypto/ripemd160.py` (RIPEMD-160, for HASH160), `address.py`'s
Base58Check (`byte_to_base58`, `base58_to_byte`) and `segwit_addr.py`'s
bech32 and bech32m (`bech32_encode`, `bech32_decode`, `convertbits`),
with `hashlib` for SHA-256 and SHA-256d and `bytes.fromhex` for hex.

- Published answers first: FIPS 180-4's SHA-256 examples, RIPEMD-160 of
  "abc", BIP-173's public key and its HASH160, the first valid address of
  Bitcoin Core's `key_io_valid.json` and BIP-173's and BIP-350's valid
  and invalid strings.
- Random bytes of every length around SHA-256's and RIPEMD-160's 64-byte
  blocks and their padding (0 to 130, and random lengths to 300): the
  three hashes, and the SHA-256 of the same bytes as a file; files of
  4 KiB, 64 KiB and 1 MiB with one byte more and one less.
- Random bytes written in Base58Check (every version byte), Base58,
  bech32 and bech32m (real and random human-readable parts) and read back
  by Faraday, and the framework's own writing of the same bytes read by
  Faraday; a text changed (a character, its case, its end, spaces
  around it), truncated or made of random characters, read by both, and
  both must give the same encoding, bytes, version, human-readable part
  and checksum, or both refuse.
- Hex: Faraday's rule (an even number of hex digits, nothing else) in
  front of `bytes.fromhex`, which also accepts spaces.

What stays Faraday's own rule, written in the area in front of the
reference: the order `encodings::read` tries the spellings in (bech32,
Base58Check, hex, Base58), the trimming of the text around it, plain
Base58 (no checksum; Core's framework has only Base58Check, so those few
lines are the area's own, over Python integers), and hex being only an
even number of hex digits.
"""

import hashlib
import string

from common import Area, use_reference

use_reference("core")
from test_framework.address import base58_to_byte, byte_to_base58  # noqa: E402
from test_framework.crypto.ripemd160 import ripemd160  # noqa: E402
from test_framework.segwit_addr import (  # noqa: E402
    CHARSET, Encoding, bech32_decode, bech32_encode, bech32_verify_checksum, convertbits)

B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
BECH32_CHARS = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"
HRPS = ["bc", "tb", "bcrt", "lnbc", "lntb", "npub", "nsec", "age", "a", "abcdef"]
# Lengths around the 64-byte block of SHA-256 and RIPEMD-160 and their
# 9-byte padding, then a spread to 300.
EDGE_SIZES = list(range(0, 131))
# What Rust's `str::trim` takes off: Unicode's White_Space.
WHITESPACE = ("\t\n\x0b\x0c\r \x85\xa0\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007"
              "\u2008\u2009\u200a\u2028\u2029\u202f\u205f\u3000")


def sha256(b):
    return hashlib.sha256(b).digest()


def ref_hashes(b):
    once = sha256(b)
    return {"len": len(b), "sha256": once.hex(), "sha256d": sha256(once).hex(),
            "hash160": ripemd160(once).hex()}


def b58_decode_plain(text):
    """Base58 with nothing appended: the area's own lines (Core's framework
    reads only Base58Check). Leading '1's are zero bytes."""
    n = 0
    for c in text:
        n = n * 58 + B58.index(c)
    body = n.to_bytes((n.bit_length() + 7) // 8, "big")
    zeros = len(text) - len(text.lstrip(B58[0]))
    return b"\0" * zeros + body


def b58_encode_plain(b):
    n = int.from_bytes(b, "big")
    out = ""
    while n:
        n, r = divmod(n, 58)
        out = B58[r] + out
    return B58[0] * (len(b) - len(b.lstrip(b"\0"))) + out


def from_hex(text):
    """Faraday's rule: hex digits only, an even number of them."""
    if not text or len(text) % 2 or not all(c in string.hexdigits for c in text):
        raise ValueError("not hex")
    return bytes.fromhex(text)


def bech32_decode_long(bech):
    """`bech32_decode` of Core's framework without its length limit: the
    same steps, with the checks the framework makes before the length."""
    if (any(ord(x) < 33 or ord(x) > 126 for x in bech) or
            (bech.lower() != bech and bech.upper() != bech)):
        return (None, None, None)
    bech = bech.lower()
    pos = bech.rfind("1")
    if pos < 1 or pos + 7 > len(bech) or not all(x in CHARSET for x in bech[pos + 1:]):
        return (None, None, None)
    data = [CHARSET.find(x) for x in bech[pos + 1:]]
    enc = bech32_verify_checksum(bech[:pos], data)
    return (None, None, None) if enc is None else (enc, bech[:pos], data[:-6])


def ref_bech32(text):
    """Core's `bech32_decode`, the data read as bytes (no padding left)."""
    if len(text) > 90:
        # Known difference (length): `bech32_decode` refuses more than 90
        # characters, BIP-173's limit for a segwit address; Faraday reads
        # any length. The same decoding, shortened to pass the limit.
        enc, hrp, data = bech32_decode_long(text)
    else:
        enc, hrp, data = bech32_decode(text)
    if enc is None or len(hrp) > 83:
        # BIP-173: a human-readable part of 1 to 83 characters.
        raise ValueError("not bech32")
    raw = convertbits(data, 5, 8, False)
    if raw is None:
        # Known difference: `encodings::read` turns all the data groups into
        # bytes and drops the bits that do not fill one. BIP-173's rule of at
        # most 4 padding bits, all zero, is a segwit address's, over the
        # program after the witness version; `convertbits` here applies it
        # to all the groups, and so refuses every P2WPKH address. The
        # reference reads it the same way as Faraday.
        raw = convertbits(data, 5, 8, True)[:len(data) * 5 // 8]
    return {"encoding": "bech32" if enc == Encoding.BECH32 else "bech32m",
            "bytes": bytes(raw).hex(), "version": None, "hrp": hrp, "checksum": True}


def ref_read(text):
    t = text.strip(WHITESPACE)
    if not t:
        raise ValueError("empty")
    try:
        return ref_bech32(t)
    except (ValueError, AssertionError):
        pass
    try:
        res = base58_to_byte(t)
        if isinstance(res, tuple):
            data, version = res
            return {"encoding": "base58check", "bytes": data.hex(), "version": version,
                    "hrp": None, "checksum": True}
    except (ValueError, AssertionError, IndexError):
        pass
    try:
        return {"encoding": "hex", "bytes": from_hex(t).hex(), "version": None, "hrp": None,
                "checksum": None}
    except ValueError:
        pass
    if all(c in B58 for c in t):
        return {"encoding": "base58", "bytes": b58_decode_plain(t).hex(), "version": None,
                "hrp": None, "checksum": None}
    raise ValueError("none")


def ref_bech32_write(enc, hrp, b):
    """Core writes whatever it is given; a string its own decoder refuses
    (no or bad human-readable part, longer than 90) is no bech32 string."""
    text = bech32_encode(enc, hrp, convertbits(b, 8, 5))
    if bech32_decode(text)[0] is None:
        raise ValueError("not a bech32 string")
    return text


def ref_write(kind, version, hrp, b):
    if kind == "base58check":
        return byte_to_base58(b, version)
    if kind == "base58":
        return b58_encode_plain(b)
    return ref_bech32_write(Encoding.BECH32 if kind == "bech32" else Encoding.BECH32M, hrp, b)


class Hash(Area):
    name = "hash"
    faraday = (
        '`faraday_bip::hashes` and `faraday_codec::encodings` (`read`, the writers and `from_hex`), '
        'and `faraday_core::file_menu::sha256`'
    )
    reference = (
        "Bitcoin Core v31.1's functional-test framework (`core/`): `ripemd160.py`, `address.py` and "
        '`segwit_addr.py`, with `hashlib` for SHA-256 and `bytes.fromhex` for hex'
    )
    reference_checked = (
        "its vendored files are hashed against `references.sha256` before any run, and a "
        "published vector's answer is required of the reference as well as of Faraday; the "
        "order `read` tries the spellings in, the trimming of the text, hex as an even number of "
        "hex digits and plain Base58 (which Core's framework does not read; the area's own few "
        "lines) are Faraday's rules, written in the area in front of the reference"
    )
    vectors = (
        "FIPS 180-4's SHA-256 examples, RIPEMD-160 of `abc`, BIP-173's key and its HASH160, "
        "Bitcoin Core's first `key_io_valid.json` address and BIP-173's and BIP-350's bech32 strings"
    )
    corpus = (
        'random bytes of every length from 0 to 130 and to 300 hashed (SHA-256, SHA-256d, HASH160) '
        'and as a file, files of 4 KiB, 64 KiB and 1 MiB with a byte more and less; random bytes '
        'written and read as Base58Check, Base58, bech32 and bech32m, Core\'s writing read by '
        'Faraday, and texts changed, cut or made of random characters read by both'
    )
    differences = [
        (
            'bech32 groups: Faraday reads any string with a valid checksum as bytes, all its '
            'data groups together, and drops the bits that do not fill a byte. BIP-173 has a '
            'padding rule only for a segwit address, over the program after the witness version, '
            'and Faraday does not take the version off. So Core\'s `convertbits` over all the '
            'groups refuses every P2WPKH address, and the area reads those strings as Faraday '
            'does. The bytes shown for a segwit address are therefore its version and program '
            'run together, not its program: this area does not judge them as a program.'
        ),
        (
            'bech32 length: Core\'s decoder refuses more than 90 characters (BIP-173); '
            'Faraday accepts any length the checksum covers. The area reads longer strings the same way.'
        ),
    ]
    not_compared = (
        "the file hash is `file_menu::sha256` over a whole byte slice (it reads in no buffer, so "
        "no read size is probed); the Files screen's reading of a file from a stick is the "
        "files areas'"
    )

    def hashes(self, b):
        ans = self.need(self.compare("hash.hashes", {"bytes": b.hex()}, lambda: ref_hashes(b)),
                        "hashes")
        self.compare("hash.file", {"bytes": b.hex()}, lambda: sha256(b).hex())
        return ans

    def read(self, text, expected=None):
        return self.compare("hash.read", {"text": text}, lambda: ref_read(text), expected)

    def write(self, kind, b, version=0, hrp=""):
        return self.compare("hash.write",
                            {"kind": kind, "bytes": b.hex(), "version": version, "hrp": hrp},
                            lambda: ref_write(kind, version, hrp, b))

    def from_hex(self, text):
        return self.compare("hash.from_hex", {"text": text}, lambda: from_hex(text).hex())

    def published(self):
        for data, sha in [
            (b"", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
            (b"abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
            (b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
             "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"),
        ]:
            if self.hashes(data)["sha256"] != sha:
                raise AssertionError("SHA-256 differs from FIPS 180-4's example")
        if ripemd160(b"abc").hex() != "8eb208f7e05d987a9b044a8e98c6b087f15a0bfc":
            raise AssertionError("the reference's RIPEMD-160 differs from the published answer")
        # BIP-173's public key and its HASH160.
        key = bytes.fromhex("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798")
        if self.hashes(key)["hash160"] != "751e76e8199196d454941c45d1b3a323f1433bd6":
            raise AssertionError("HASH160 differs from BIP-173's")
        # Bitcoin Core's key_io_valid.json (v31.1), first entry.
        h = bytes.fromhex("a31c06bd463e3923bc1aadbde48b16976c080717")
        text = "1FsSia9rv4NeEwvJ2GvXrX7LyxYspbN2mo"
        self.read(text, ("ok", {"encoding": "base58check", "bytes": h.hex(), "version": 0,
                                "hrp": None, "checksum": True}))
        if self.need(self.write("base58check", h, 0), "write") != text:
            raise AssertionError("Base58Check differs from Bitcoin Core's address")
        # BIP-173 and BIP-350: strings of no data (read with their bytes),
        # other valid strings (both must read them alike), and invalid ones.
        for s, enc in [("a12uel5l", "bech32"), ("A12UEL5L", "bech32"), ("a1lqfn3a", "bech32m")]:
            self.read(s, ("ok", {"encoding": enc, "bytes": "", "version": None, "hrp": "a",
                                 "checksum": True}))
        for s in ["abcdef1l7aum6echk45nj3s0wdvt2fg8x9yrzpqzd3ryx",
                  "abcdef1qpzry9x8gf2tvdw0s3jn54khce6mua7lmqqqxw",
                  "an83characterslonghumanreadablepartthatcontainsthenumber1andtheexcludedcharactersbio1tt5tgs",
                  "an84characterslonghumanreadablepartthatcontainsthenumber1andtheexcludedcharactersbio1569pvx",
                  "split1checkupstagehandshakeupstreamerranterredcaperred2y9e3w",
                  "A12uEL5L", "1nwldj5", "\x7f1axkwrx", "pzry9x0s0muk",
                  "an84characterslonghumanreadablepartthatcontainsthenumber1andtheexcludedcharactersbio1569pvz"]:
            self.read(s)
        # Longer than BIP-173's 90 characters, both checksums.
        for enc in (Encoding.BECH32, Encoding.BECH32M):
            self.read(bech32_encode(enc, "a", [i % 32 for i in range(120)]))
        for s in ["0", "O", "l", "I", "1111111111", "1", "0x00"]:
            self.read(s)

    def sizes(self):
        rng = self.rng
        return rng.choice(EDGE_SIZES) if rng.random() < 0.5 else rng.randint(131, 300)

    def corpus_case(self):
        rng = self.rng
        b = self.randbytes(self.sizes())
        self.hashes(b)
        # Writing and reading, in every spelling.
        small = b[:rng.randint(0, 40)]
        version = rng.randrange(256)
        texts = {
            "base58check": self.need(self.write("base58check", small, version), "base58check"),
            "base58": self.need(self.write("base58", small), "base58"),
        }
        hrp = rng.choice(HRPS) if rng.random() < 0.6 else "".join(
            rng.choice(string.ascii_lowercase + string.digits) for _ in range(rng.randint(1, 12)))
        for kind in ("bech32", "bech32m"):
            made = self.write(kind, small, hrp=hrp)
            if made[0] == "ok":
                texts[kind] = made[1]
        for kind, t in texts.items():
            got = self.read(t) if not t else self.need(self.read(t), "read " + kind)
            if kind == "base58check" and got["encoding"] == "base58check":
                if got["bytes"] != small.hex() or got["version"] != version:
                    raise AssertionError("Base58Check written is not read back")
            self.read(t.upper())
            self.read(rng.choice(WHITESPACE) + t + rng.choice(WHITESPACE))
        # Core's own writing, read by Faraday.
        self.read(byte_to_base58(small, version))
        for enc in (Encoding.BECH32, Encoding.BECH32M):
            data5 = [rng.randrange(32) for _ in range(rng.randint(0, 50))]
            self.read(bech32_encode(enc, hrp, data5))
            self.read(bech32_encode(enc, hrp, convertbits(small, 8, 5)))
        # Texts changed, cut or random.
        for t in list(texts.values()):
            self.read(self.damage(t))
        alphabet = rng.choice([string.hexdigits, B58, BECH32_CHARS + "1", B58 + "0OIl"])
        self.read("".join(rng.choice(alphabet) for _ in range(rng.randint(1, 80))))
        # Hex.
        digits = small.hex()
        self.from_hex(digits)
        self.from_hex(digits.upper())
        self.from_hex(self.damage(digits))
        self.from_hex(" ".join(digits[i:i + 2] for i in range(0, len(digits), 2)))

    def damage(self, text):
        rng = self.rng
        kind = rng.randrange(6)
        if not text:
            return text + "0"
        i = rng.randrange(len(text))
        if kind == 0:
            return text[:i] + rng.choice(B58 + BECH32_CHARS + "0OIl1") + text[i + 1:]
        if kind == 1:
            return text[:i] + text[i].swapcase() + text[i + 1:]
        if kind == 2:
            return text[:i]
        if kind == 3:
            return text + rng.choice(B58)
        if kind == 4:
            return " " + text + "  "
        return text[:i] + text[i + 1:]

    def files(self):
        for size in (4096, 65536, 1 << 20):
            for n in (size - 1, size, size + 1):
                b = self.randbytes(n)
                self.compare("hash.file", {"bytes": b.hex()}, lambda b=b: sha256(b).hex())

    LOOPS = (("corpus", 3000),)

    def prelude(self):
        self.published()
        self.files()

    def case_corpus(self):
        self.corpus_case()
