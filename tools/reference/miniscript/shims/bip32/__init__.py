"""Written here, not upstream: python-bip380 imports `bip32` (a package
over libsecp256k1) to read and derive extended keys in a descriptor.
The cross-check derives every key itself (`tools/crosscheck/btc.py`,
checked against BIP-32's vectors) and gives python-bip380 raw public
keys only, so this stands in for the import and refuses every call that
would need an extended key rather than compute it some other way."""

HARDENED_INDEX = 0x80000000


class BIP32:
    @classmethod
    def from_xpub(cls, xpub):
        raise ValueError("the cross-check gives python-bip380 raw public keys only")
