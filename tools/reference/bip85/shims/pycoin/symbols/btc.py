"""Written here, not upstream: the few calls of `pycoin` that BIP-85's
reference implementation (`../../bip85/__init__.py`) makes, so that the
reference runs unchanged without vendoring all of pycoin.

Each call is handed to btclib (`tools/reference/btclib/`: btclib-wallet's
`bip32` and btclib's `b58`), which the cross-check unpacks and puts on
the import path first (`tools/crosscheck/bip32_ref.py`), on btclib's own
curve with libsecp256k1 turned off: the master key of a seed, BIP-32
derivation along a path, an xprv read and written, and a compressed WIF.
What is left here is the glue: pycoin's names, its path spelling (`p`
for hardened), and its reading of a key text as Bitcoin's (a mainnet
xprv, or no key).
"""

from btclib.b58 import prv_key_data_from_wif, wif_from_prv_key
from btclib.network import NETWORKS
from btclib_wallet.bip32 import bip32

XPRV = NETWORKS["mainnet"].bip32_prv
HARDENED = 0x80000000


class Node:
    """An extended private key, btclib's `BIP32KeyData`."""

    def __init__(self, data):
        if not data.is_private:
            raise ValueError("not a private key")
        self.data = data

    def secret_exponent(self):
        return int.from_bytes(self.data.key[1:], "big")

    def subkey_for_path(self, path):
        steps = []
        for step in path.split("/"):
            if not step:
                continue
            if step[-1] in "pH'":
                steps.append(int(step[:-1]) + HARDENED)
            else:
                steps.append(int(step))
        return Node(bip32.derive_(self.data, steps))

    def hwif(self, as_private=True):
        if not as_private:
            raise NotImplementedError("only a private key")
        return self.data.b58encode()


class Key:
    def __init__(self, k):
        self.k = k

    def wif(self):
        return wif_from_prv_key(self.k, "mainnet", compressed=True)

    def secret_exponent(self):
        return self.k


class Keys:
    @staticmethod
    def bip32_seed(seed):
        return Node(bip32.rootxprv_from_seed_(seed, XPRV))

    @staticmethod
    def private(secret_exponent):
        # btclib refuses a scalar of 0 or n and past it.
        wif_from_prv_key(secret_exponent, "mainnet", compressed=True)
        return Key(secret_exponent)

    @staticmethod
    def from_text(text):
        return Key(prv_key_data_from_wif(text, "mainnet", compressed=True).q)


class Network:
    keys = Keys()

    @staticmethod
    def parse(text):
        try:
            data = bip32.BIP32KeyData.b58decode(text)
        except ValueError:
            return None
        if data.version != XPRV:
            return None
        return Node(data)


network = Network()
