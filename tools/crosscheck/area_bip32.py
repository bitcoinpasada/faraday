"""BIP-32 against btclib (`tools/reference/btclib/`: btclib-wallet's
`bip32`, on btclib's own curve with libsecp256k1 turned off, through
`bip32_ref.py`), which is first checked against the BIP's test vectors
1 to 5 (`tools/vectors/bip32/`).

- The master key of a seed (16 to 64 bytes), mainnet and test.
- Private derivation along random paths of hardened and normal steps,
  every index from 0 to 2³² - 1 reachable, with the child's xprv, xpub,
  fingerprint and the master's fingerprint.
- Public derivation from an xpub along normal steps, which must give the
  xpub private derivation gives; a hardened step from an xpub refused.
- Extended keys read as text by the strict decoders (`faraday_bip::xkey`)
  and the master-key import (`MasterKey::decode`): every valid key read
  back to itself, and keys broken one rule at a time (the checksum, the
  length, the version, a depth-0 key with a parent or an index, the pad
  byte, a scalar of 0 or past n, a public key's prefix or a point off
  the curve) refused by both.

What is ours: which versions a strict reader takes. btclib reads every
version of its network table, SLIP-132's included; the area keeps
BIP-32's own four (xprv and tprv for a private key, xpub and tpub for a
public one, from btclib's table) and refuses the rest, as Faraday does.
"""

import json

import bip32_ref
from bip32_ref import HARDENED, N, Invalid
from common import ROOT, Area

NETWORKS = ["mainnet", "testnet", "signet", "regtest"]


def is_test(network):
    return network != "mainnet"


class Bip32(Area):
    name = "bip32"
    faraday = (
        '`faraday_bip::keys` (master, private derivation), `faraday_bip::xkey` (strict '
        "decoding), `bitcoin`'s `Xpub::derive_pub`"
    )
    reference = (
        "btclib-wallet's `bip32` over btclib and `ecc` (`btclib/`), on btclib's own "
        'curve with libsecp256k1 turned off, through `bip32_ref.py`; which versions a '
        "strict reader takes (BIP-32's four, from btclib's network table) is ours"
    )
    reference_checked = (
        'the wheels are hashed against `references.sha256` before any run; '
        "`check_driver` runs btclib against BIP-32's vectors 1 to 5 and the invalid keys "
        'of vector 5 before anything else'
    )
    vectors = (
        "BIP-32's vectors 1 to 5 (`tools/vectors/bip32/`), python-mnemonic's master "
        'keys'
    )
    corpus = (
        'seeds of 16 to 64 bytes on every network, random paths with every index, '
        'public derivation against private, keys broken one rule at a time and the '
        'master-key import'
    )
    differences = [
        (
            'none'
        ),
    ]
    not_compared = (
        'none'
    )

    def check_driver(self):
        """btclib against BIP-32's own vectors before anything else."""
        for v in json.loads((ROOT / "tools/vectors/bip32/vectors.json").read_text()):
            m = bip32_ref.master(bytes.fromhex(v["seed"]))
            for chain in v["chains"]:
                k = m.derive(bip32_ref.parse_path(chain["path"]))
                if (k.serialize(), k.neuter().serialize()) != (chain["xprv"], chain["xpub"]):
                    raise AssertionError(f"btclib fails BIP-32 at {chain['path']}")
                if bip32_ref.decode(chain["xpub"], False).serialize() != chain["xpub"]:
                    raise AssertionError("btclib does not read back a BIP-32 xpub")
        for line in (ROOT / "tools/vectors/bip32/invalid-keys.txt").read_text().splitlines():
            key = line.split()[0] if line.strip() else ""
            if not key or key.startswith("#"):
                continue
            for private in (True, False):
                try:
                    bip32_ref.decode(key, private)
                except Invalid:
                    continue
                raise AssertionError(f"btclib reads {key}, which BIP-32 vector 5 rejects")

    def master(self, seed, network, expected=None):
        def ref():
            m = bip32_ref.master(seed, is_test(network))
            return {"xprv": m.serialize(), "xpub": m.neuter().serialize(),
                    "fingerprint": m.fingerprint().hex()}
        return self.compare("bip32.master", {"seed": seed.hex(), "network": network}, ref,
                            expected)

    def derive(self, seed, network, path, expected=None):
        def ref():
            m = bip32_ref.master(seed, is_test(network))
            k = m.derive(path)
            return {"xprv": k.serialize(), "xpub": k.neuter().serialize(),
                    "fingerprint": k.fingerprint().hex(),
                    "master_fingerprint": m.fingerprint().hex()}
        return self.compare("bip32.derive", {"seed": seed.hex(), "network": network,
                                             "path": bip32_ref.path_text(path)}, ref, expected)

    def derive_pub(self, xpub, path):
        return self.compare("bip32.derive_pub", {"xpub": xpub, "path": bip32_ref.path_text(path)},
                            lambda: {"xpub": bip32_ref.decode(xpub, False).derive(path).serialize()})

    def decode(self, key, private):
        op = "bip32.decode_xprv" if private else "bip32.decode_xpub"
        return self.compare(op, {"key": key},
                            lambda: {"key": bip32_ref.decode(key, private).serialize()})

    def import_master(self, key):
        def ref():
            k = bip32_ref.decode(key, True)
            if not k.data.is_root:
                raise Invalid("not a master key")
            return {"xprv": k.serialize(), "fingerprint": k.fingerprint().hex()}
        return self.compare("bip32.import", {"key": key}, ref)

    def published(self):
        self.check_driver()
        for v in json.loads((ROOT / "tools/vectors/bip32/vectors.json").read_text()):
            seed = bytes.fromhex(v["seed"])
            for chain in v["chains"]:
                path = bip32_ref.parse_path(chain["path"])
                got = self.need(self.derive(seed, "mainnet", path), "derive")
                if (got["xprv"], got["xpub"]) != (chain["xprv"], chain["xpub"]):
                    raise AssertionError(f"BIP-32 vector at {chain['path']}")
                self.decode(chain["xprv"], True)
                self.decode(chain["xpub"], False)
                self.import_master(chain["xprv"])
        for line in (ROOT / "tools/vectors/bip32/invalid-keys.txt").read_text().splitlines():
            key = line.split()[0] if line.strip() else ""
            if key and not key.startswith("#"):
                self.decode(key, True)
                self.decode(key, False)
                self.import_master(key)
        vectors = json.loads((ROOT / "tools/vectors/bip39/vectors.json").read_text())
        for _, _, seed, xprv in vectors["english"]:
            got = self.need(self.master(bytes.fromhex(seed), "mainnet"), "master")
            if got["xprv"] != xprv:
                raise AssertionError("python-mnemonic's master key vector")

    def random_path(self):
        rng = self.rng
        path = []
        for _ in range(rng.choice([0, 1, 2, 3, 4, 5, rng.randint(0, 8)])):
            i = rng.choice([0, 1, 2, rng.randrange(100), rng.randrange(HARDENED),
                            HARDENED - 1])
            if rng.random() < 0.5:
                i += HARDENED
            path.append(i)
        return path

    def broken(self, key):
        """`key` (a valid extended key's text) with one rule broken."""
        rng = self.rng
        raw = bytearray(bip32_ref.b58check_decode(key))
        how = rng.randrange(10)
        if how == 0:
            # A character changed: the checksum fails, or the text is
            # not base58.
            i = rng.randrange(len(key))
            return key[:i] + rng.choice("1z0OIl") + key[i + 1:]
        if how == 1:
            raw = raw + b"\0" if rng.random() < 0.5 else raw[:-1]
        elif how == 2:
            raw[:4] = rng.choice([*bip32_ref.PRV, *bip32_ref.PUB,
                                  bytes.fromhex("049d7cb2"), bytes.fromhex("04b2430c"),
                                  self.randbytes(4)])
        elif how == 3:
            raw[4] = 0
            raw[5:9] = self.randbytes(4) if rng.random() < 0.5 else raw[5:9]
            raw[9:13] = self.randbytes(4) if rng.random() < 0.5 else raw[9:13]
        elif how == 4:
            raw[45] = rng.choice([1, 2, 3, 4, 0xff])
        elif how == 5:
            raw[45:] = b"\0" + rng.choice([0, N, N + 1, 2**256 - 1, N - 1, 1]).to_bytes(32, "big")
        elif how == 6:
            raw[45] = rng.choice([0, 4, 5, 6, 7])
        elif how == 7:
            # A public key whose x is not on the curve, or is past p.
            x = rng.choice([5, 2**256 - 1, rng.randrange(2**256)])
            raw[45:] = bytes([rng.choice([2, 3])]) + x.to_bytes(32, "big")
        elif how == 8:
            raw[4] = rng.choice([1, 255, raw[4] + 1 & 0xFF])
        else:
            raw[13:45] = self.randbytes(32)
        return bip32_ref.b58check_encode(bytes(raw))

    def corpus_case(self):
        rng = self.rng
        seed = self.randbytes(rng.choice([16, 32, 64, rng.randint(16, 64)]))
        network = rng.choice(NETWORKS)
        self.master(seed, network)
        path = self.random_path()
        got = self.need(self.derive(seed, network, path), "derive")
        for key, private in ((got["xprv"], True), (got["xpub"], False)):
            self.decode(key, private)
            self.decode(key, not private)
            bad = self.broken(key)
            self.decode(bad, private)
            self.decode(bad, not private)
            if private and rng.random() < 0.3:
                self.import_master(rng.choice([key, bad]))
        # Public derivation along normal steps from this key, and a
        # hardened step refused.
        suffix = [rng.choice([0, 1, rng.randrange(HARDENED)])
                  for _ in range(rng.randint(1, 3))]
        pub = self.need(self.derive_pub(got["xpub"], suffix), "derive_pub")["xpub"]
        priv = self.need(self.derive(seed, network, path + suffix), "derive")["xpub"]
        if pub != priv:
            raise AssertionError("public and private derivation disagree on both sides")
        if rng.random() < 0.2:
            self.derive_pub(got["xpub"], suffix + [HARDENED + rng.randrange(HARDENED)])

    LOOPS = (("corpus", 400),)

    def prelude(self):
        self.published()

    def case_corpus(self):
        self.corpus_case()
