"""btclib's BIP-32 (`tools/reference/btclib/`: btclib-wallet's `bip32`
over btclib and `ecc`), set up for the areas that need extended keys:
`bip32`, `addr`, `miniscript`, `lightning`, `sign` and BIP-85's pycoin
shim. The wheels are unpacked as `area_msg.py` unpacks them
(`common.use_wheels`), and btclib computes on its own pure-Python curve,
libsecp256k1 turned off, so its keys are a lineage of their own (F160).

`Key` holds btclib's `BIP32KeyData` and gives it the few names the areas
read (its chain code, private scalar, public key, fingerprint); every
derivation, serialisation, decoding and fingerprint is btclib's. What
`decode` adds is the one rule btclib leaves to its caller: which versions
a reader takes. btclib reads every version its network table knows, the
SLIP-132 ones included; BIP-32's strict reading takes xprv and tprv for
a private key, xpub and tpub for a public one, and refuses the rest.
"""

from common import HarnessError, use_wheels

use_wheels("btclib", [
    "typing_extensions-4.16.0-py3-none-any.whl",
    "ecc-2026.10.23-py3-none-any.whl",
    "btclib-2026.10.24-py3-none-any.whl",
    "btclib_wallet-2026.10.29-py3-none-any.whl",
])
from ecc import curves  # noqa: E402

curves.set_libsecp256k1_serving(serving=False)
if curves.is_libsecp256k1_serving():
    raise HarnessError("btclib would derive keys through libsecp256k1")

from btclib import base58  # noqa: E402
from btclib.hashes import hash160  # noqa: E402,F401
from btclib.network import NETWORKS  # noqa: E402
from btclib_wallet.bip32 import bip32  # noqa: E402
from btclib_wallet.bip32.der_path import indexes_from_der_path, str_from_der_path  # noqa: E402
from ecc.curves import bytes_from_prv_key_int, secp256k1  # noqa: E402

HARDENED = 1 << 31
N = secp256k1.n
MAIN, TEST = NETWORKS["mainnet"], NETWORKS["testnet"]
# BIP-32's own versions; testnet, signet and regtest share tprv and tpub.
PRV = {MAIN.bip32_prv: False, TEST.bip32_prv: True}
PUB = {MAIN.bip32_pub: False, TEST.bip32_pub: True}


class Invalid(Exception):
    """An input btclib refuses, or a version BIP-32's reader does not take."""


def point(k):
    """Costs an elliptic-curve multiplication: k·G compressed, by btclib's
    curve."""
    return bytes_from_prv_key_int(k)


def b58check_encode(payload):
    return base58.encode(payload).decode("ascii")


def b58check_decode(text):
    try:
        return base58.decode(text)
    except Exception as e:  # noqa: BLE001 - every refusal of btclib's
        raise Invalid(str(e)) from e


class Key:
    """An extended key, btclib's `BIP32KeyData`: private or public. Each
    key keeps what has been worked out of it (its public key, its
    fingerprint, the children asked of it), as btclib's own keys are
    immutable."""

    def __init__(self, data):
        self.data = data
        self._pub = None
        self._fingerprint = None
        self._children = {}

    @property
    def chain(self):
        return self.data.chain_code

    @property
    def k(self):
        """The private scalar, or None for a public key."""
        if not self.data.is_private:
            return None
        return int.from_bytes(self.data.key[1:], "big")

    @property
    def pub(self):
        """Costs an elliptic-curve multiplication the first time, for a
        private key: its compressed public key."""
        if self._pub is None:
            self._pub = (bip32.xpub_from_xprv_(self.data).key if self.data.is_private
                         else self.data.key)
        return self._pub

    @property
    def depth(self):
        return self.data.depth

    @property
    def child(self):
        return self.data.index

    @property
    def parent(self):
        return self.data.parent_fingerprint

    @property
    def test(self):
        return self.data.version in (TEST.bip32_prv, TEST.bip32_pub)

    def derive(self, path):
        """Costs btclib's derivation along `path` (child numbers), once a
        path: the key there, private from a private key, public from a
        public one."""
        path = tuple(path)
        if path not in self._children:
            try:
                self._children[path] = Key(bip32.derive_(self.data, list(path)))
            except Exception as e:  # noqa: BLE001 - every refusal of btclib's
                raise Invalid(str(e)) from e
        return self._children[path]

    def ckd(self, i):
        return self.derive([i])

    def neuter(self):
        """Costs an elliptic-curve multiplication: the public key."""
        if not self.data.is_private:
            return self
        return Key(bip32.xpub_from_xprv_(self.data))

    def with_test(self, test):
        """The same key under BIP-32's mainnet or test version."""
        table = PRV if self.data.is_private else PUB
        version = next(v for v, t in table.items() if t == test)
        d = self.data
        return Key(bip32.BIP32KeyData(version, d.depth, d.parent_fingerprint, d.index,
                                      d.chain_code, d.key))

    def serialize(self):
        return self.data.b58encode()

    def fingerprint(self):
        """Costs an elliptic-curve multiplication the first time, for a
        private key: btclib's fingerprint."""
        if self._fingerprint is None:
            self._fingerprint = bip32.fingerprint(self.data)
        return self._fingerprint


def master(seed, test=False):
    """Costs nothing but HMAC: btclib's master key of a seed."""
    try:
        return Key(bip32.rootxprv_from_seed_(seed, (TEST if test else MAIN).bip32_prv))
    except Exception as e:  # noqa: BLE001 - every refusal of btclib's
        raise Invalid(str(e)) from e


def decode(text, private):
    """An extended key read by btclib (its checksum, length, depth, index
    and parent, pad byte, scalar range and point), of BIP-32's own
    versions only, private or public as asked."""
    try:
        data = bip32.BIP32KeyData.b58decode(text)
    except Exception as e:  # noqa: BLE001 - every refusal of btclib's
        raise Invalid(str(e)) from e
    if data.version not in (PRV if private else PUB):
        raise Invalid(f"version {data.version.hex()}")
    return Key(data)


def parse_path(text):
    """`m/0h/1/2'` as child numbers, read by btclib."""
    return indexes_from_der_path(text)


def path_text(path):
    """Child numbers as `m/0h/1`, written by btclib."""
    return str_from_der_path(list(path))
