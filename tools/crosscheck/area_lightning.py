"""Lightning node keys and the aezeed (`faraday_bip::aezeed`) against
LND's and ldk's published answers (`tools/vectors/aezeed/`) and a
reference written here from the sources `tools/reference/aezeed/README.md`
quotes, over BIP-32 from `btc.py` and python-mnemonic (`bip39/`).

- ldk-node (README §6): a master key from the 64-byte BIP-39 seed, its
  private key taken as LDK's 32-byte seed, a second master key from that,
  and `m/0'` of the second; the network changes nothing.
- LND (README §3): `m/1017'/coin'/6'/0/0` of the aezeed's sixteen bytes
  of entropy taken as the BIP-32 seed, coin 0 on mainnet and 1 elsewhere.
- The aezeed's outer layer (README §1): 24 words of 11 bits are 33
  bytes, a version byte (0), 23 enciphered bytes, a 5-byte salt and a
  CRC-32C (Castagnoli) of the first 29 bytes, big-endian. A seed whose
  version is not 0 or whose checksum fails is refused before scrypt runs.
  AEZ itself has no second implementation here: what comes out of the
  cipher is checked against LND's five published seeds, and a seed whose
  checksum holds but whose passphrase or enciphered bytes were changed
  must be refused (AEZ's 32-bit tag fails, but for one time in 2³²).

- Every vector of `tools/vectors/aezeed/`.
- Random BIP-39 seeds on every network (ldk) and random entropies on
  every network (LND).
- Random word lists (the checksum fails), lists with the checksum put
  right and a version other than 0, and the published seeds under a
  changed passphrase, with an enciphered byte or the salt changed and the
  checksum put right, and with a word changed.
"""

import json

import btc
from common import ROOT, Area, use_reference

use_reference("bip39")
from mnemonic import Mnemonic  # noqa: E402

H = btc.HARDENED
NETWORKS = ["mainnet", "testnet", "signet", "regtest"]
ENGLISH = Mnemonic("english")


def crc32c(data):
    crc = 0xFFFFFFFF
    for b in data:
        crc ^= b
        for _ in range(8):
            crc = (crc >> 1) ^ (0x82F63B78 if crc & 1 else 0)
    return crc ^ 0xFFFFFFFF


def to_bytes(words):
    n = 0
    for w in words:
        n = (n << 11) | w
    return n.to_bytes(33, "big")


def to_words(data):
    n = int.from_bytes(data, "big")
    return [(n >> (11 * (23 - i))) & 2047 for i in range(24)]


def with_checksum(data):
    return data[:29] + crc32c(data[:29]).to_bytes(4, "big")


def outer_refusal(words):
    """Why the outer layer refuses, or None when it holds."""
    data = to_bytes(words)
    if data[0] != 0:
        return "a version other than 0"
    if crc32c(data[:29]) != int.from_bytes(data[29:], "big"):
        return "the checksum fails"
    return None


def node(k):
    return {"secret": k.to_bytes(32, "big").hex(), "node_id": btc.point(k).hex()}


def ldk(seed):
    outer = btc.master(seed)
    inner = btc.master(outer.k.to_bytes(32, "big"), True)
    return node(inner.derive([H]).k)


def lnd(entropy, network):
    coin = 0 if network == "mainnet" else 1
    return node(btc.master(entropy).derive([1017 + H, coin + H, 6 + H, 0, 0]).k)


class Lightning(Area):
    name = "lightning"

    def ldk(self, seed, network, expected=None):
        return self.compare("lightning.ldk", {"seed": seed.hex(), "network": network},
                            lambda: ldk(seed), expected)

    def lnd(self, entropy, network, expected=None):
        return self.compare("lightning.lnd", {"entropy": entropy.hex(), "network": network},
                            lambda: lnd(entropy, network), expected)

    def aezeed(self, words, passphrase, cost, expected):
        """Faraday's reading against the outer layer's rule and, where
        that holds, the expected answer."""
        def ref():
            why = outer_refusal(words)
            if why:
                raise ValueError(why)
            if expected[0] == "err":
                raise ValueError("AEZ's tag fails")
            return expected[1]
        return self.compare("lightning.aezeed", {"words": words, "passphrase": passphrase.hex(),
                                                 "cost": cost}, ref)

    def published(self):
        v = json.loads((ROOT / "tools/vectors/aezeed/vectors.json").read_text())
        self.vectors = v
        entropy = bytes.fromhex(v["entropy"])
        for a in v["aezeed"]:
            words = [ENGLISH.wordlist.index(w) for w in a["words"].split()]
            self.aezeed(words, a["passphrase"].encode(), a["cost"], ("ok", {
                "internal_version": a["internal_version"], "birthday": a["birthday"],
                "entropy": v["entropy"]}))
        for n in v["lnd_node"]:
            want = {"secret": n["secret"], "node_id": n["node_id"]}
            self.lnd(entropy, n["network"], ("ok", want))
        for n in v["ldk_node"]:
            seed = ENGLISH.to_seed(n["words"], n["passphrase"])
            for network in NETWORKS:
                self.ldk(seed, network, ("ok", {"secret": n["secret"], "node_id": n["node_id"]}))

    def corpus_case(self):
        rng = self.rng
        network = rng.choice(NETWORKS)
        words = ENGLISH.to_mnemonic(self.randbytes(rng.choice([16, 20, 24, 28, 32])))
        passphrase = "".join(rng.choice("abcXYZ019 !~") for _ in range(rng.choice([0, 0, 8])))
        self.ldk(ENGLISH.to_seed(words, passphrase), network)
        self.lnd(self.randbytes(16), network)
        # The outer layer: random words, a version other than 0 with the
        # checksum right.
        self.aezeed([rng.randrange(2048) for _ in range(24)], b"", 16, ("err", None))
        data = bytearray(self.randbytes(33))
        data[0] = rng.randint(1, 255)
        self.aezeed(to_words(with_checksum(bytes(data))), b"", 16, ("err", None))
        # A published seed (at the low cost) changed: the passphrase, an
        # enciphered or salt byte with the checksum put right, a word.
        a = rng.choice([x for x in self.vectors["aezeed"] if x["cost"] == 16])
        good = [ENGLISH.wordlist.index(w) for w in a["words"].split()]
        r = rng.random()
        if r < 0.4:
            self.aezeed(good, (a["passphrase"] + rng.choice(["x", "!", " "])).encode(), 16,
                        ("err", None))
        elif r < 0.8:
            data = bytearray(to_bytes(good))
            data[rng.randint(1, 28)] ^= 1 << rng.randrange(8)
            self.aezeed(to_words(with_checksum(bytes(data))), a["passphrase"].encode(), 16,
                        ("err", None))
        else:
            bad = good[:]
            i = rng.randrange(24)
            bad[i] = (bad[i] + rng.randint(1, 2047)) % 2048
            self.aezeed(bad, a["passphrase"].encode(), 16, ("err", None))

    def run(self):
        self.published()
        for _ in range(self.cases(400)):
            self.corpus_case()
