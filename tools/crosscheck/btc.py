"""Bitcoin's building blocks for the cross-check, written here from the
BIPs' text over the standard library and two vendored modules of the
BIPs repository: the curve (`secp256k1lab`) and RIPEMD-160
(`ripemd160.py`), both from `tools/reference/bip352/`.

- Base58Check (as Bitcoin Core's `base58.cpp` defines it).
- BIP-32: the master key, private and public child derivation, the
  78-byte serialisation, the fingerprint, and a strict decoder by the
  rules BIP-32's test vector 5 states.

Each area that uses this checks it first against the BIPs' own vectors
(`tools/vectors/bip32/`), so an error here is caught there rather than
taken as Faraday's.
"""

import hashlib
import hmac

from common import use_reference

use_reference("bip352", "secp256k1lab", "src")
use_reference("bip352")
from ripemd160 import ripemd160  # noqa: E402
from secp256k1lab.secp256k1 import FAST_G, GE  # noqa: E402

N = GE.ORDER

B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"

XPRV = bytes.fromhex("0488ade4")
XPUB = bytes.fromhex("0488b21e")
TPRV = bytes.fromhex("04358394")
TPUB = bytes.fromhex("043587cf")
HARDENED = 1 << 31


class Invalid(Exception):
    """An input the BIP says to refuse."""


def sha256(b):
    return hashlib.sha256(b).digest()


def hash256(b):
    return sha256(sha256(b))


def hash160(b):
    return ripemd160(sha256(b))


def b58encode(b):
    n = int.from_bytes(b, "big")
    out = ""
    while n:
        n, r = divmod(n, 58)
        out = B58[r] + out
    return "1" * (len(b) - len(b.lstrip(b"\0"))) + out


def b58decode(s):
    n = 0
    for ch in s:
        if ch not in B58:
            raise Invalid(f"{ch!r} is not base58")
        n = n * 58 + B58.index(ch)
    body = n.to_bytes((n.bit_length() + 7) // 8, "big") if n else b""
    return b"\0" * (len(s) - len(s.lstrip("1"))) + body


def b58check_encode(payload):
    return b58encode(payload + hash256(payload)[:4])


def b58check_decode(s):
    raw = b58decode(s)
    if len(raw) < 4 or hash256(raw[:-4])[:4] != raw[-4:]:
        raise Invalid("base58 checksum")
    return raw[:-4]


def point(k):
    """k·G, compressed."""
    return FAST_G.mul(k).to_bytes_compressed()


def fingerprint(pub):
    return hash160(pub)[:4]


class Key:
    """An extended key: private when `k` is set, else public."""

    def __init__(self, chain, k=None, pub=None, depth=0, parent=b"\0\0\0\0", child=0,
                 test=False):
        self.chain, self.k, self.depth, self._parent, self.child, self.test = (
            chain, k, depth, parent, child, test)
        self._pub = pub

    @property
    def parent(self):
        """Costs an elliptic-curve multiplication when first read: the
        parent's fingerprint, worked out from the parent key then, as a
        private key's hardened child needs no public key but this."""
        if isinstance(self._parent, Key):
            self._parent = self._parent.fingerprint()
        return self._parent

    @property
    def pub(self):
        if self._pub is None:
            self._pub = point(self.k)
        return self._pub

    def neuter(self):
        return Key(self.chain, None, self.pub, self.depth, self.parent, self.child, self.test)

    def ckd(self, i):
        """BIP-32's CKDpriv for a private key, CKDpub for a public one."""
        if self.depth == 255:
            raise Invalid("depth past 255")
        if i >= HARDENED:
            if self.k is None:
                raise Invalid("a hardened child of a public key")
            data = b"\0" + self.k.to_bytes(32, "big")
        else:
            data = self.pub
        digest = hmac.new(self.chain, data + i.to_bytes(4, "big"), hashlib.sha512).digest()
        il, ir = int.from_bytes(digest[:32], "big"), digest[32:]
        if il >= N:
            raise Invalid("I_L is not below n")
        common = dict(depth=self.depth + 1, parent=self, child=i, test=self.test)
        if self.k is not None:
            k = (il + self.k) % N
            if k == 0:
                raise Invalid("a zero child key")
            return Key(ir, k, **common)
        p = FAST_G.mul(il) + GE.from_bytes_compressed(self.pub)
        if p.infinity:
            raise Invalid("the child is the point at infinity")
        return Key(ir, None, p.to_bytes_compressed(), **common)

    def derive(self, path):
        key = self
        for i in path:
            key = key.ckd(i)
        return key

    def serialize(self):
        if self.k is not None:
            version, data = (TPRV if self.test else XPRV), b"\0" + self.k.to_bytes(32, "big")
        else:
            version, data = (TPUB if self.test else XPUB), self.pub
        return b58check_encode(version + bytes([self.depth]) + self.parent
                               + self.child.to_bytes(4, "big") + self.chain + data)

    def fingerprint(self):
        return fingerprint(self.pub)


def master(seed, test=False):
    """BIP-32's master key generation."""
    digest = hmac.new(b"Bitcoin seed", seed, hashlib.sha512).digest()
    k = int.from_bytes(digest[:32], "big")
    if k == 0 or k >= N:
        raise Invalid("the master key is not below n")
    return Key(digest[32:], k, test=test)


def decode(text, private):
    """An extended key read by BIP-32's rules, test vector 5's included:
    the checksum, 78 bytes, a version of the kind asked for, at depth 0
    no parent fingerprint and child number 0, a private key's pad byte
    zero and its scalar in 1..n-1, a public key on the curve with prefix
    02 or 03."""
    raw = b58check_decode(text)
    if len(raw) != 78:
        raise Invalid(f"{len(raw)} bytes")
    version = raw[:4]
    depth, parent, child, chain, data = raw[4], raw[5:9], raw[9:13], raw[13:45], raw[45:]
    if version not in ((XPRV, TPRV) if private else (XPUB, TPUB)):
        raise Invalid(f"version {version.hex()}")
    if depth == 0 and (parent != bytes(4) or child != bytes(4)):
        raise Invalid("a depth-0 key with a parent or an index")
    common = dict(depth=depth, parent=parent, child=int.from_bytes(child, "big"),
                  test=version in (TPRV, TPUB))
    if private:
        if data[0] != 0:
            raise Invalid("a private key's pad byte is not zero")
        k = int.from_bytes(data[1:], "big")
        if not 0 < k < N:
            raise Invalid("a private key out of range")
        return Key(chain, k, **common)
    if data[0] not in (2, 3):
        raise Invalid("a public key's prefix")
    try:
        GE.from_bytes_compressed(data)
    except ValueError as e:
        raise Invalid("a public key off the curve") from e
    return Key(chain, None, data, **common)


def parse_path(text):
    """`m/0h/1/2'` as child numbers."""
    parts = text.split("/")
    if parts[0] == "m":
        parts = parts[1:]
    out = []
    for p in parts:
        hard = p[-1:] in ("h", "'", "H")
        out.append(int(p[:-1] if hard else p) + (HARDENED if hard else 0))
    return out


def path_text(path):
    return "/".join(["m"] + [f"{i - HARDENED}h" if i >= HARDENED else str(i) for i in path])
