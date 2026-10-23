"""Written here, not upstream: the few calls of `pycoin` that BIP-85's
reference implementation (`../../bip85/__init__.py`) makes, so that the
reference runs unchanged without vendoring all of pycoin.

BIP-85 derives only along hardened paths, and a hardened BIP-32 child
needs no curve arithmetic: HMAC-SHA512 over the parent's private key,
and a sum modulo the group order. So this is BIP-32 for extended private
keys along hardened paths, base58check, and WIF, and it refuses
anything else (a non-hardened step, a public key) rather than guess.
It is checked by BIP-85's own published vectors, which the cross-check
runs before its random corpus.
"""

import hashlib
import hmac

import base58

N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141
XPRV = bytes.fromhex("0488ade4")
HARDENED = 0x80000000


class Node:
    """An extended private key."""

    def __init__(self, k, chain_code, depth=0, parent_fp=b"\0\0\0\0", child=0):
        if not 0 < k < N:
            raise ValueError("private key out of range")
        self.k = k
        self.chain_code = chain_code
        self.depth = depth
        self.parent_fp = parent_fp
        self.child = child

    def secret_exponent(self):
        return self.k

    def _child(self, i):
        if i < HARDENED:
            raise NotImplementedError("a non-hardened step needs the curve, which this shim does not do")
        data = b"\0" + self.k.to_bytes(32, "big") + i.to_bytes(4, "big")
        digest = hmac.new(self.chain_code, data, hashlib.sha512).digest()
        il = int.from_bytes(digest[:32], "big")
        if il >= N:
            raise ValueError("invalid child (I_L >= n)")
        # The parent fingerprint needs the parent's public key; nothing
        # here serializes a derived node, so it is left unknown.
        return Node((il + self.k) % N, digest[32:], self.depth + 1, None, i)

    def subkey_for_path(self, path):
        node = self
        for step in path.split("/"):
            if not step:
                continue
            if step[-1] in "pH'":
                node = node._child(int(step[:-1]) + HARDENED)
            else:
                node = node._child(int(step))
        return node

    def hwif(self, as_private=True):
        if not as_private or self.parent_fp is None:
            raise NotImplementedError("only a private key whose parent fingerprint is known")
        raw = (XPRV + bytes([self.depth]) + self.parent_fp + self.child.to_bytes(4, "big")
               + self.chain_code + b"\0" + self.k.to_bytes(32, "big"))
        return base58.b58encode_check(raw).decode()


class Key:
    def __init__(self, k):
        self.k = k

    def wif(self):
        return base58.b58encode_check(b"\x80" + self.k.to_bytes(32, "big") + b"\x01").decode()

    def secret_exponent(self):
        return self.k


class Keys:
    @staticmethod
    def bip32_seed(seed):
        digest = hmac.new(b"Bitcoin seed", seed, hashlib.sha512).digest()
        return Node(int.from_bytes(digest[:32], "big"), digest[32:])

    @staticmethod
    def private(secret_exponent):
        if not 0 < secret_exponent < N:
            raise ValueError("private key out of range")
        return Key(secret_exponent)

    @staticmethod
    def from_text(text):
        raw = base58.b58decode_check(text)
        if len(raw) != 34 or raw[0] != 0x80 or raw[33] != 1:
            raise ValueError("not a compressed mainnet WIF")
        return Key(int.from_bytes(raw[1:33], "big"))


class Network:
    keys = Keys()

    @staticmethod
    def parse(text):
        try:
            raw = base58.b58decode_check(text)
        except ValueError:
            return None
        if len(raw) != 78 or raw[:4] != XPRV or raw[45] != 0:
            return None
        k = int.from_bytes(raw[46:], "big")
        if not 0 < k < N:
            return None
        return Node(k, raw[13:45], raw[4], raw[5:9], int.from_bytes(raw[9:13], "big"))


network = Network()
