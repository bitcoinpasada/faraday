"""Lightning node keys and the aezeed (`faraday_bip::aezeed`) against
LND's own code (the vendored Go tree of `tools/reference/lnd/`, one
process for the run: `main.go` there) and ldk's published answers
(`tools/vectors/aezeed/`), with ldk's derivation written here from the
sources `tools/reference/aezeed/README.md` quotes, over BIP-32 from
`btc.py` and python-mnemonic (`bip39/`).

- LND (README §3), answered by LND's code: the aezeed read by LND's
  `aezeed` package (its wordlist, version byte, CRC-32C, scrypt and the AEZ
  of Yawning Angel's `aez`, unchanged), and the node key at
  `m/1017'/coin'/6'/0/0` of the aezeed's sixteen bytes of entropy taken as
  the BIP-32 seed over btcd's `hdkeychain`, coin 0 on mainnet and 1
  elsewhere. The path's numbers are `keychain/derivation.go`'s, written in
  `main.go`; LND derives them through btcwallet, which is not vendored.
- ldk-node (README §6), still ours: a master key from the 64-byte BIP-39
  seed, its private key taken as LDK's 32-byte seed, a second master key
  from that, and `m/0'` of the second; the network changes nothing.

- Every vector of `tools/vectors/aezeed/`.
- Random BIP-39 seeds on every network (ldk) and random entropies on
  every network (LND).
- Random word lists (the checksum fails), lists with the checksum put
  right and a version other than 0, and the published seeds under a
  changed passphrase, with an enciphered byte or the salt changed and the
  checksum put right, and with a word changed: each of them LND refuses.
"""

import json
import os
import subprocess

import btc
from common import ROOT, Area, HarnessError, use_reference

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


def node(k):
    return {"secret": k.to_bytes(32, "big").hex(), "node_id": btc.point(k).hex()}


def ldk(seed):
    outer = btc.master(seed)
    inner = btc.master(outer.k.to_bytes(32, "big"), True)
    return node(inner.derive([H]).k)


def lnd_exe():
    """LND's answers as a program: built by `tools/crosscheck/run.sh` from
    the vendored Go tree (`FARADAY_LND_REF` names another place)."""
    path = os.environ.get("FARADAY_LND_REF") or str(ROOT / "out/crosscheck/lnd-ref")
    if not os.access(path, os.X_OK):
        raise HarnessError(
            f"the lightning area needs LND's answer program, {path}, which "
            "tools/crosscheck/run.sh builds from tools/reference/lnd/ with the Go toolchain "
            "(docs/DEVELOPING.md: the version go.mod names, installed from go.dev/dl)")
    return path


class Lnd:
    """The Go program answering with LND's code, a request a line."""

    def __init__(self):
        self.proc = subprocess.Popen([lnd_exe()], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     text=True, bufsize=1)
        self.next_id = 0

    def call(self, op, **args):
        self.next_id += 1
        self.proc.stdin.write(json.dumps(dict(args, id=self.next_id, op=op)) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            raise HarnessError(f"{op}: LND's answer program exited")
        out = json.loads(line)
        if out.get("id") != self.next_id:
            raise HarnessError(f"{op}: LND's answer out of step: {line.strip()}")
        if "ok" in out:
            return out["ok"]
        raise ValueError(out["err"])

    def close(self):
        self.proc.stdin.close()
        self.proc.wait()


class Lightning(Area):
    # The published vectors, which corpus cases also draw from.
    aezeed_vectors = json.loads((ROOT / "tools/vectors/aezeed/vectors.json").read_text())
    name = "lightning"
    faraday = (
        "`faraday_bip::aezeed` (ldk-node's and LND's node keys, the aezeed read)"
    )
    reference = (
        "LND's own code, vendored in Go (`tools/reference/lnd/`: its `aezeed` package "
        "over Yawning Angel's `aez`, and the node key over btcd's `hdkeychain`), one "
        "process answering the run's requests; ldk-node's derivation as "
        "`tools/reference/aezeed/README.md` quotes it, written out in `area_lightning.py` "
        'over `btc.py`'
    )
    reference_checked = (
        "LND's code reads the aezeed (wordlist, version, CRC-32C, scrypt, AEZ) and "
        "derives the node key, and its published answers (`tools/vectors/aezeed/`) are "
        "required of it as well as of Faraday; the Go program is built offline from the "
        "vendored tree (`GOFLAGS=-mod=vendor`, `GOTOOLCHAIN=local`) and its version is "
        "written into the stamp; the area stops when the program is missing; "
        "ldk-node's derivation is written in the area from the sources the README "
        "quotes, and ldk's published answers are required of it as well as of Faraday"
    )
    vectors = (
        "LND's five cipher seeds, LND's two node keys, ldk's five node ids "
        '(`tools/vectors/aezeed/`)'
    )
    corpus = (
        'random BIP-39 seeds and entropies on every network; random words, a wrong '
        'version with a right checksum, published seeds under a changed passphrase, a '
        'changed enciphered byte or salt with the checksum put right, a changed word, '
        "each refused by LND's code"
    )
    differences = [
        (
            'none'
        ),
    ]
    not_compared = (
        "ldk-node's node key is compared with a derivation written here, not with "
        "ldk's code (Rust, not vendored): only ldk's five published answers are outside "
        "it; LND's key path numbers are written in the Go program from "
        "`keychain/derivation.go`, as LND derives through btcwallet, which is not vendored"
    )

    lnd_proc = None

    def lnd_ref(self):
        if self.lnd_proc is None:
            self.lnd_proc = Lnd()
        return self.lnd_proc

    def close(self):
        if self.lnd_proc is not None:
            self.lnd_proc.close()
        super().close()

    def ldk(self, seed, network, expected=None):
        return self.compare("lightning.ldk", {"seed": seed.hex(), "network": network},
                            lambda: ldk(seed), expected)

    def lnd(self, entropy, network, expected=None):
        return self.compare("lightning.lnd", {"entropy": entropy.hex(), "network": network},
                            lambda: self.lnd_ref().call("node", entropy=entropy.hex(),
                                                        network=network), expected)

    def aezeed(self, words, passphrase, cost, expected):
        """Faraday's reading against LND's, and the expected answer: a
        published seed's, or a refusal."""
        def ref():
            return self.lnd_ref().call("aezeed", words=[ENGLISH.wordlist[w] for w in words],
                                       passphrase=passphrase.hex(), cost=cost)
        return self.compare("lightning.aezeed", {"words": words, "passphrase": passphrase.hex(),
                                                 "cost": cost}, ref, expected)

    def published(self):
        v = self.aezeed_vectors
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
        a = rng.choice([x for x in self.aezeed_vectors["aezeed"] if x["cost"] == 16])
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

    LOOPS = (("corpus", 400),)

    def prelude(self):
        self.published()

    def case_corpus(self):
        self.corpus_case()
