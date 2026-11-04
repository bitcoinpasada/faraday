"""The Hashes and Encodings calculators, and the hash of a file, against
Bitcoin Core's functional-test framework (`core/test_framework/`):
`crypto/ripemd160.py` (RIPEMD-160, for HASH160), `address.py`'s
Base58Check (`byte_to_base58`, `base58_to_byte`) and `segwit_addr.py`'s
bech32 and bech32m (`bech32_encode`, `bech32_decode`, `convertbits`,
`encode_segwit_address`, `decode_segwit_address`),
with `hashlib` for SHA-256 and SHA-256d and `bytes.fromhex` for hex; and
PyCA cryptography's `HKDFExpand` and `HKDF` (OpenSSL) for the one HKDF;
and Python's standard library for HMAC (`hmac`), PBKDF2 (`hashlib.pbkdf2_hmac`)
and scrypt (`hashlib.scrypt`, OpenSSL).

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
- HKDF (RFC 5869 over SHA-256): RFC 5869's first test case, then random
  keys, salts (none, empty and up to 100 bytes), infos and output lengths
  from 0 to 255 blocks of 32 bytes (the edges 0, 1, 31, 32, 33, 8159,
  8160 among them), `hkdf_sha256_expand` against `HKDFExpand` and
  `hkdf_sha256` against `HKDF`; one byte past 255 blocks is refused by
  both.
- HMAC and PBKDF2, each over SHA-256 and SHA-512, and scrypt: RFC 4231's
  first case for each hash, RFC 7914 section 11's PBKDF2-SHA256 vector
  (rounds 1) and its two cheap scrypt vectors, then random HMAC keys of 0 to
  300 bytes (across the block sizes 64 and 128) over messages of 0 to 500
  bytes, PBKDF2 over passwords and salts of 0 to 100 bytes, rounds 1 to 50
  and lengths 1 to 200 (across 32 and 64), and scrypt at N from 2 to 4096,
  r 1 to 8, p 1 to 3 and lengths 1 to 100. scrypt with N not a power of two,
  N 0 or 1, r 0, p 0 or an empty output is refused by both. PBKDF2 is only
  asked within its callers' domain (rounds and length at least 1).
- Hex: Faraday's rule (an even number of hex digits, nothing else) in
  front of `bytes.fromhex`, which also accepts spaces.

What stays Faraday's own rule, written in the area in front of the
reference: the order `encodings::read` tries the spellings in (bech32,
Base58Check, hex, Base58), the trimming of the text around it, plain
Base58 (no checksum; Core's framework has only Base58Check, so those few
lines are the area's own, over Python integers), hex being only an even
number of hex digits, and which prefixes make a segwit address (bc, tb and
bcrt; Core's decoder is told the prefix).
"""

import hashlib
import hmac
import json
import pathlib
import string

from common import Area, use_reference

from cryptography.hazmat.primitives import hashes as pyca_hashes
from cryptography.hazmat.primitives.kdf.hkdf import HKDF, HKDFExpand

use_reference("core")
from test_framework.address import base58_to_byte, byte_to_base58  # noqa: E402
from test_framework.crypto.ripemd160 import ripemd160  # noqa: E402
from test_framework.segwit_addr import (  # noqa: E402
    CHARSET, Encoding, bech32_decode, bech32_encode, bech32_verify_checksum, convertbits,
    decode_segwit_address, encode_segwit_address)

B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
BECH32_CHARS = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"
ADDRESS_HRPS = ("bc", "tb", "bcrt")
ADDRESSES = json.loads((pathlib.Path(__file__).resolve().parents[1] / "vectors" / "addresses"
                        / "vectors.json").read_text())
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


def ref_hkdf_expand(prk, info, length):
    """PyCA's HKDFExpand: a refusal past 255 blocks, as RFC 5869 §2.3 says."""
    return HKDFExpand(algorithm=pyca_hashes.SHA256(), length=length,
                      info=bytes(info)).derive(bytes(prk)).hex()


def ref_hkdf(salt, ikm, info, length):
    """PyCA's HKDF; salt None is no salt and b"" an empty one, each 32 zero
    bytes (§2.2): the two are passed as themselves."""
    return HKDF(algorithm=pyca_hashes.SHA256(), length=length, salt=salt,
                info=bytes(info)).derive(bytes(ikm)).hex()


def ref_hmac(hash_name, key, msg):
    """The standard library's HMAC: any key length."""
    return hmac.new(bytes(key), bytes(msg), hash_name).hexdigest()


def ref_pbkdf2(hash_name, password, salt, rounds, length):
    return hashlib.pbkdf2_hmac(hash_name, bytes(password), bytes(salt), rounds, length).hex()


def ref_scrypt(password, salt, n, r, p, length):
    """OpenSSL's scrypt; it refuses N below 2 or not a power of two, r or p
    of 0 and an empty output (Python raises for each)."""
    if length < 1:
        raise ValueError("empty output")
    return hashlib.scrypt(bytes(password), salt=bytes(salt), n=n, r=r, p=p, dklen=length,
                          maxmem=1 << 30).hex()


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
    """Core's `decode_segwit_address` for an address of bc, tb or bcrt;
    any other text is `bech32_decode`'s data read as bytes."""
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
    name = "bech32" if enc == Encoding.BECH32 else "bech32m"
    if hrp in ADDRESS_HRPS:
        version, program = decode_segwit_address(hrp, text)
        if version is not None:
            return {"encoding": name, "bytes": bytes(program).hex(), "version": version,
                    "hrp": hrp, "checksum": True}
    raw = convertbits(data, 5, 8, False)
    if raw is None:
        # Any other text: BIP-173's padding rule is a segwit address's, and
        # `encodings::read` turns all the groups into bytes and drops the
        # bits that do not fill one. The reference reads it the same way.
        raw = convertbits(data, 5, 8, True)[:len(data) * 5 // 8]
    return {"encoding": name,
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
        '`faraday_core::file_menu::sha256`, and `faraday_crypto::hkdf_sha256` and `hkdf_sha256_expand`, '
        '`hmac_sha256`, `hmac_sha512`, `pbkdf2_hmac_sha256`, `pbkdf2_hmac_sha512` and `scrypt`'
    )
    reference = (
        "Bitcoin Core v31.1's functional-test framework (`core/`): `ripemd160.py`, `address.py` and "
        '`segwit_addr.py`, with `hashlib` for SHA-256 and `bytes.fromhex` for hex; PyCA cryptography\'s `HKDF` and '
        '`HKDFExpand` (OpenSSL) for the one HKDF; `hmac`, `hashlib.pbkdf2_hmac` and `hashlib.scrypt` (OpenSSL) '
        'for HMAC, PBKDF2 and scrypt'
    )
    reference_checked = (
        "its vendored files are hashed against `references.sha256` before any run, and a "
        "published vector's answer is required of the reference as well as of Faraday; the "
        "order `read` tries the spellings in, the trimming of the text, hex as an even number of "
        "hex digits, plain Base58 (which Core's framework does not read; the area's own few "
        "lines) and which prefixes make a segwit address are Faraday's rules, written in the area in front of the reference"
    )
    vectors = (
        "FIPS 180-4's SHA-256 examples, RIPEMD-160 of `abc`, BIP-173's key and its HASH160, "
        "Bitcoin Core's first `key_io_valid.json` address, BIP-173's and BIP-350's bech32 strings "
        "and their valid and invalid segwit addresses (`tools/vectors/addresses`)"
    )
    corpus = (
        'random bytes of every length from 0 to 130 and to 300 hashed (SHA-256, SHA-256d, HASH160) '
        'and as a file, files of 4 KiB, 64 KiB and 1 MiB with a byte more and less; random bytes '
        'written and read as Base58Check, Base58, bech32 and bech32m, Core\'s writing read by '
        'Faraday, segwit addresses of every version Core\'s framework writes, and texts changed, cut or made of random characters read by both'
    )
    differences = [
        (
            'bech32 groups: a string that is not a segwit address of bc, tb or bcrt is read as '
            'bytes, all its data groups together, and the bits that do not fill a byte are '
            'dropped. BIP-173 has a padding rule only for a segwit address, so Core\'s '
            '`convertbits`, which applies it to any data, refuses some of these strings, and '
            'the area reads them as Faraday does.'
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

    def hkdf_expand(self, prk, info, length, expected=None):
        return self.compare("hash.hkdf_expand",
                            {"prk": prk.hex(), "info": info.hex(), "len": length},
                            lambda: ref_hkdf_expand(prk, info, length), expected)

    def hkdf(self, salt, ikm, info, length, expected=None):
        return self.compare("hash.hkdf",
                            {"salt": (salt or b"").hex(), "ikm": ikm.hex(), "info": info.hex(), "len": length},
                            lambda: ref_hkdf(salt, ikm, info, length), expected)

    def hmac(self, hash_name, key, msg, expected=None):
        return self.compare("hash.hmac", {"hash": hash_name, "key": key.hex(), "msg": msg.hex()},
                            lambda: ref_hmac(hash_name, key, msg), expected)

    def pbkdf2(self, hash_name, password, salt, rounds, length, expected=None):
        return self.compare("hash.pbkdf2",
                            {"hash": hash_name, "password": password.hex(), "salt": salt.hex(),
                             "rounds": rounds, "len": length},
                            lambda: ref_pbkdf2(hash_name, password, salt, rounds, length), expected)

    def scrypt(self, password, salt, n, r, p, length, expected=None):
        return self.compare("hash.scrypt",
                            {"password": password.hex(), "salt": salt.hex(), "n": n, "r": r,
                             "p": p, "len": length},
                            lambda: ref_scrypt(password, salt, n, r, p, length), expected)

    def mac_case(self):
        rng = self.rng
        for h in ("sha256", "sha512"):
            key = self.randbytes(rng.choice([0, 1, 20, 63, 64, 65, 127, 128, 129, rng.randint(0, 300)]))
            self.hmac(h, key, self.randbytes(rng.choice([0, 1, 111, 112, 128, rng.randint(0, 500)])))
            self.pbkdf2(h, self.randbytes(rng.randint(0, 100)), self.randbytes(rng.randint(0, 100)),
                        rng.choice([1, 2, rng.randint(1, 50)]),
                        rng.choice([1, 31, 32, 33, 63, 64, 65, rng.randint(1, 200)]))

    def scrypt_case(self):
        rng = self.rng
        # A fraction of the other cases' count: every case does real work.
        if rng.randrange(8):
            return
        n = 1 << rng.randint(1, 12)
        self.scrypt(self.randbytes(rng.randint(0, 40)), self.randbytes(rng.randint(0, 40)), n,
                    rng.randint(1, 8) if n <= 1024 else rng.randint(1, 2), rng.randint(1, 3),
                    rng.randint(1, 100))
        pw, salt = self.randbytes(rng.randint(0, 10)), self.randbytes(rng.randint(0, 10))
        n, r, p, length = 16, 1, 1, 16
        bad = rng.choice(["n_odd", "n0", "n1", "r0", "p0", "empty"])
        if bad == "n_odd":
            n = rng.choice([3, 5, 6, 7, 12, 15, 17, 24, 1000, 4095, 4097])
        elif bad == "n0":
            n = 0
        elif bad == "n1":
            n = 1
        elif bad == "r0":
            r = 0
        elif bad == "p0":
            p = 0
        else:
            length = 0
        self.scrypt(pw, salt, n, r, p, length)

    def hkdf_case(self):
        rng = self.rng
        length = rng.choice([0, 1, 31, 32, 33, 8159, 8160, 8161, rng.randint(0, 8160),
                             rng.randint(1, 200)])
        info = self.randbytes(rng.choice([0, 1, 4, rng.randint(0, 100)]))
        self.hkdf_expand(self.randbytes(rng.choice([32, 32, 33, 64, rng.randint(32, 100)])),
                         info, length)
        n = rng.choice([None, 0, 0, 13, 32, 64, rng.randint(0, 100)])
        salt = None if n is None else self.randbytes(n)
        self.hkdf(salt, self.randbytes(rng.choice([0, 1, 22, 32, rng.randint(0, 100)])),
                  info, length)

    def published(self):
        for data, sha in [
            (b"", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
            (b"abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
            (b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
             "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"),
        ]:
            if self.hashes(data)["sha256"] != sha:
                raise AssertionError("SHA-256 differs from FIPS 180-4's example")
        # RFC 5869 Appendix A, test case 1.
        okm = ("3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf"
               "34007208d5b887185865")
        self.hkdf(bytes(range(13)), bytes([0x0b] * 22), bytes(range(0xf0, 0xfa)), 42,
                  ("ok", okm))
        self.hkdf_expand(bytes.fromhex("077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844a"
                                       "d7c2b3e5"), bytes(range(0xf0, 0xfa)), 42, ("ok", okm))
        # RFC 4231 test case 1 (key 0x0b * 20, "Hi There"), per hash.
        key, msg = bytes([0x0b] * 20), b"Hi There"
        self.hmac("sha256", key, msg, ("ok", "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"))
        self.hmac("sha512", key, msg, ("ok", "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cde"
                                              "daa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854"))
        # RFC 7914 section 11: PBKDF2-HMAC-SHA-256 ("passwd", "salt", c = 1; c = 80000 is too slow here).
        self.pbkdf2("sha256", b"passwd", b"salt", 1, 64,
                    ("ok", "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc"
                           "49ca9cccf179b645991664b39d77ef317c71b845b1e30bd509112041d3a19783"))
        # RFC 7914 section 12: scrypt N = 16, r = 1, p = 1, and N = 1024, r = 8, p = 16.
        self.scrypt(b"", b"", 16, 1, 1, 64,
                    ("ok", "77d6576238657b203b19ca42c18a0497f16b4844e3074ae8dfdffa3fede21442"
                           "fcd0069ded0948f8326a753a0fc81f17e8d3e0fb2e0d3628cf35e20c38d18906"))
        self.scrypt(b"password", b"NaCl", 1024, 8, 16, 64,
                    ("ok", "fdbabe1c9d3472007856e7190d01e9fe7c6ad7cbc8237830e77376634b373162"
                           "2eaf30d92e22a3886ff109279d9830dac727afb94a83ee6d8360cbdfa2cc0640"))
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
        # BIP-173's and BIP-350's addresses: witness version and program.
        for case in ADDRESSES["segwit_valid"]:
            script = bytes.fromhex(case["script"])
            version = script[0] - 0x50 if script[0] else 0
            self.read(case["address"], ("ok", {
                "encoding": "bech32m" if version else "bech32", "bytes": script[2:].hex(),
                "version": version, "hrp": case["address"].lower().rsplit("1", 1)[0],
                "checksum": True}))
        # The invalid ones: read alike, and never as an address by the reference.
        for case in ADDRESSES["segwit_invalid"]:
            self.read(case["address"])
            try:
                if ref_read(case["address"])["version"] is not None:
                    raise AssertionError(f"the reference takes {case['address']} as an address")
            except ValueError:
                pass
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
        self.hkdf_case()
        self.mac_case()
        self.scrypt_case()
        # A segwit address Core's framework writes, of any version and of
        # every length that version allows.
        version = rng.choice([0, 0, 1, 1, rng.randint(2, 16)])
        size = rng.choice([20, 32]) if version == 0 else rng.choice([2, 32, 40, rng.randint(2, 40)])
        self.read(encode_segwit_address(rng.choice(ADDRESS_HRPS), version, self.randbytes(size)))
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
