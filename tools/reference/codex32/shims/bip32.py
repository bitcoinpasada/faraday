"""Written here, not upstream: python-codex32 imports `bip32` (a package
over libsecp256k1) only to compute a default identifier from a seed's
fingerprint in `Codex32String.from_seed`. The cross-check always passes
the identifier, so this stands in for the import and refuses the one
call that would need it rather than compute it some other way."""


class BIP32:
    @classmethod
    def from_seed(cls, seed):
        raise NotImplementedError("the cross-check passes every identifier explicitly")
