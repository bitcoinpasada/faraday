"""Lightning node keys and the aezeed (`faraday_bip::aezeed`) against
LND's own code (the vendored Go tree of `tools/reference/lnd/`, one
process for the run: `main.go` there) and ldk's published answers
(`tools/vectors/aezeed/`), with ldk's derivation written here from the
sources `tools/reference/aezeed/README.md` quotes, over btclib's BIP-32
(`bip32_ref.py`, btclib's own curve) and python-mnemonic (`bip39/`).

- LND (README §3), answered by LND's code: the aezeed read by LND's
  `aezeed` package (its wordlist, version byte, CRC-32C, scrypt and the AEZ
  of Yawning Angel's `aez`, unchanged), and the node key at
  `m/1017'/coin'/6'/0/0` of the aezeed's sixteen bytes of entropy taken as
  the BIP-32 seed over btcd's `hdkeychain`, coin 0 on mainnet and 1
  elsewhere. The path's numbers are `keychain/derivation.go`'s, written in
  `main.go`; LND derives them through btcwallet, which is not vendored.
- ldk-node (README §6), still ours as a composition: a master key from
  the 64-byte BIP-39 seed, its private key taken as LDK's 32-byte seed, a
  second master key from that, and `m/0'` of the second; the network
  changes nothing. Each master key, the derivation and the public key are
  btclib's.

- Every vector of `tools/vectors/aezeed/`.
- Random BIP-39 seeds on every network (ldk) and random entropies on
  every network (LND).
- Random word lists (the checksum fails), lists with the checksum put
  right and a version other than 0, and the published seeds under a
  changed passphrase, with an enciphered byte or the salt changed and the
  checksum put right, and with a word changed: each of them LND refuses.
- Every aezeed case above is asked twice. The second time Faraday's command
  is the one built with the aes crate's software round (`--cfg
  aes_backend="soft"`, the fixsliced round the Pi runs) and LND's program
  the one built without assembly (tags `noasm`, `purego`: Yawning Angel's AEZ
  then runs its own bitsliced round), both made by `tools/crosscheck/run.sh`
  and checked by `soft.py` before the run; on a processor with AES-NI the
  first pass compares the processor's round on both sides, so the software
  round is compared only here.
"""

import json
import os
import re
import subprocess

import bip32_ref
import bolt11_enc
from common import ROOT, Area, HarnessError, Mismatch, Rust, reference, use_reference, use_wheels

use_reference("bip39")
from mnemonic import Mnemonic  # noqa: E402

use_wheels("bolt11", [
    "bech32-1.2.0-py3-none-any.whl",
    "base58-2.1.1-py3-none-any.whl",
    "bitstring-3.1.9-py3-none-any.whl",
    "bolt11-2.2.0-py3-none-any.whl",
])
use_reference("bolt11", "shims")
import bolt11  # noqa: E402
from bolt11.bit_utils import scid_to_int  # noqa: E402
from bolt11.models.features import FeatureState  # noqa: E402

H = bip32_ref.HARDENED
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


def ldk(seed):
    """Costs an elliptic-curve multiplication or two, in btclib."""
    outer = bip32_ref.master(seed)
    inner = bip32_ref.master(outer.k.to_bytes(32, "big"), True)
    child = inner.derive([H])
    return {"secret": child.k.to_bytes(32, "big").hex(), "node_id": child.pub.hex()}


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


def soft_exes():
    """The software-round pass's two programs, which `run.sh` builds and
    names: Faraday's command with the aes crate's software backend and LND's
    program without assembly."""
    rust, lnd = os.environ.get("FARADAY_SOFT_RUST"), os.environ.get("FARADAY_LND_SOFT_REF")
    for path in (rust, lnd):
        if not path or not os.access(path, os.X_OK):
            raise HarnessError(
                "the lightning area's software-round pass needs the two programs "
                "tools/crosscheck/run.sh builds (FARADAY_SOFT_RUST, FARADAY_LND_SOFT_REF); "
                f"{path or 'one'} is missing")
    return rust, lnd


# Faraday's refusals of what the `bolt11` package accepts. BOLT #11 makes a
# wrong-length p, h, s or n, a repeated p or s, and both or neither of d and
# h a refusal too, as do mixed case, an amount finer than a millisatoshi
# and a leading zero value in x, c or 9 (the package lags the specification
# there); it leaves a repeated x, c, m, 9, d, h or n, a text over 1023
# characters and an amount with a leading zero open, which Faraday refuses
# (docs/STANDARDS.md, BOLT 11).
STRICTER = {"TooLong", "MixedCase", "Amount", "Field", "Duplicate", "UnknownRequiredFeature",
            "Description"}
NETWORK_OF = {"bc": "bc", "tb": "tb", "tbs": "tbs", "bcrt": "bcrt"}


def bolt11_ref(text):
    """What the `bolt11` package reads of an invoice, in the shape of
    Faraday's answer (the fields it reads: not the fallback address, which
    it writes in bech32 for every witness version, nor unknown fields)."""
    inv = bolt11.decode(text)
    first_f = inv.tags.get(bolt11.models.tags.TagChar.fallback)
    fallback = None
    if first_f is not None and first_f.data is not None:
        data = first_f.data.data
        program = data[5:]
        fallback = {"version": data[0:5].uint,
                    "data": program[: len(program) - len(program) % 8].tobytes().hex()}
    features = []
    if inv.features is not None:
        for feature, state in inv.features.feature_list.items():
            features.append(feature.value * 2 + (1 if state == FeatureState.supported else 0))
    return {
        "network": inv.currency,
        "amount_msat": None if inv.amount_msat is None else int(inv.amount_msat),
        "timestamp": inv.date,
        "payment_hash": inv.payment_hash,
        "description": inv.description,
        "description_hash": inv.description_hash,
        "payee": inv.payee,
        "expiry": inv.expiry,
        "min_final_cltv_expiry": inv.min_final_cltv_expiry,
        "fallback": fallback,
        "route_hints": [[{
            "node_id": r.public_key, "short_channel_id": scid_to_int(r.short_channel_id),
            "fee_base_msat": r.base_fee % 2**32,
            "fee_proportional_millionths": r.ppm_fee % 2**32,
            "cltv_expiry_delta": r.cltv_expiry_delta % 2**16} for r in hint.routes]
            for hint in inv.route_hints],
        "features": sorted(features),
        "payment_secret": inv.payment_secret,
        "metadata": inv.metadata,
    }


def bolt11_view(got):
    """Faraday's answer cut to what the package reads: its first fallback
    (the others are compared with the writer's list, `invoice`)."""
    out = {k: v for k, v in got.items()
           if k not in ("fallbacks", "unknown", "payee_from_field")}
    first = got["fallbacks"][0] if got["fallbacks"] else None
    out["fallback"] = first
    return out


class Lnd:
    """The Go program answering with LND's code, a request a line."""

    def __init__(self, exe=None):
        self.proc = subprocess.Popen([exe or lnd_exe()], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
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
        "`faraday_bip::aezeed` (ldk-node's and LND's node keys, the aezeed read), "
        "`faraday_codec::bolt11::decode` (a BOLT #11 invoice read)"
    )
    reference = (
        "LND's own code, vendored in Go (`tools/reference/lnd/`: its `aezeed` package "
        "over Yawning Angel's `aez`, and the node key over btcd's `hdkeychain`), one "
        "process answering the run's requests (the aezeed also asked of a second build "
        "of each side, Faraday's with the aes crate's software round and LND's without "
        "assembly); ldk-node's derivation as "
        "`tools/reference/aezeed/README.md` quotes it, written out in `area_lightning.py` "
        "over btclib's BIP-32 (`btclib/`, on btclib's own curve); for invoices, the "
        "`bolt11` package by lnbits (2.2.0, `tools/reference/bolt11/`: its wheels over "
        "the bech32, base58 and bitstring packages, with a stand-in for its libsecp256k1 "
        "binding `coincurve` written over btclib's curve), and invoices written in "
        "`bolt11_enc.py` from BOLT #11 and signed with btclib"
    )
    reference_checked = (
        "LND's code reads the aezeed (wordlist, version, CRC-32C, scrypt, AEZ) and "
        "derives the node key, and its published answers (`tools/vectors/aezeed/`) are "
        "required of it as well as of Faraday; the Go program is built offline from the "
        "vendored tree (`GOFLAGS=-mod=vendor`, `GOTOOLCHAIN=local`) and its version is "
        "written into the stamp; the area stops when the program is missing; "
        "the software-round pass runs only on programs `soft.py` has checked to be soft "
        "(the tags `noasm` and `purego`; no AES-NI instruction in Faraday's command) and "
        "the command says it was built soft; "
        "ldk-node's derivation is written in the area from the sources the README "
        "quotes, and ldk's published answers are required of it as well as of Faraday; "
        "the `bolt11` package's wheels and its stand-in are hashed against "
        "`references.sha256` before any run, and each of BOLT #11's 31 published "
        "examples is required to be read by Faraday as the BOLT says (a valid one read, "
        "an invalid one refused), the package reading the valid ones to the same fields"
    )
    vectors = (
        "LND's five cipher seeds, LND's two node keys, ldk's five node ids "
        '(`tools/vectors/aezeed/`); BOLT #11\'s 31 examples, valid and invalid '
        '(`tools/vectors/lightning-bolt11/`)'
    )
    corpus = (
        'random BIP-39 seeds and entropies on every network; every aezeed case again with '
        'the aes crate\'s software round against LND\'s AEZ without assembly; random words, a wrong '
        'version with a right checksum, published seeds under a changed passphrase, a '
        'changed enciphered byte or salt with the checksum put right, a changed word, '
        "each refused by LND's code; 150 invoices written with every kind of field "
        "(amounts in every multiplier, description or its hash, payee key, expiry, "
        "final cltv, fallbacks of every version, route hints, features, metadata, "
        "fields BOLT #11 does not define) and signed, read by Faraday and by the "
        "package field by field, then each with one Bech32 character changed, with one "
        "letter in upper case, and with two of these spoiled: a repeated p or x, a "
        "wrong-length p, s, h or n, no p, no s, both or neither of d and h, an "
        "unknown required feature, a leading zero value in a longer x, c or 9, a leading zero or a fraction of a millisatoshi in "
        "the amount, a bad multiplier, an unknown network, a high-S signature, a "
        "field cut short, a text over 1023 characters, a description that is not UTF-8"
    )
    differences = [
        (
            "BOLT #11 invoices: the package skips a p, h, s or n field of the wrong "
            "length, reads the first of a repeated p or s, and accepts both or neither "
            "of a description and its hash, mixed case, an amount finer than a "
            "millisatoshi and a leading zero value in x, c or 9, all of which BOLT #11 "
            "makes a refusal or says \"should\" of and Faraday refuses; it also accepts "
            "what BOLT #11 leaves open, which Faraday is stricter on: a repeated x, c, m, "
            "9, d, h or n, a text over 1023 characters, an amount with a leading zero, and a "
            "required feature bit Faraday does not know; the area counts each "
            "refusal as known, naming its reason"
        ),
        (
            "BOLT #11 route hints: the package reads a hop's fees and delta as signed "
            "integers (a fee above 2^31 reads negative); the area reads them modulo "
            "2^32 and 2^16 before comparing, and keeps a hint to three hops, as the "
            "package drops the last hop of a hint of a multiple of five"
        ),
        (
            "BOLT #11 fallbacks: the package reads only the first fallback address and "
            "writes the others in bech32 for every witness version, no bech32m; the "
            "area compares Faraday's whole list of fallbacks with the writer's f fields "
            "of versions 0 to 18, in order, and its fields of an unknown version by "
            "the writer's list"
        ),
        (
            "BOLT #11 features: the package keeps one state per feature, so a feature "
            "with both its required and its supported bit set reads as supported only; "
            "the writer sets one bit of each pair"
        ),
    ]
    not_compared = (
        "ldk-node's node key is compared with a derivation written here, not with "
        "ldk's code (Rust, not vendored): only ldk's five published answers are outside "
        "it; LND's key path numbers are written in the Go program from "
        "`keychain/derivation.go`, as LND derives through btcwallet, which is not vendored; "
        "the `bolt11` package is not asked the fields it does not read (fields BOLT #11 "
        "does not define, a fallback of a witness version above 16, a second or later "
        "fallback), which are compared with the writer's own list; its signature check runs on "
        "btclib's curve, not libsecp256k1's, and refuses a high-S signature as "
        "libsecp256k1 does"
    )

    lnd_proc = None
    soft_rust = None
    soft_lnd = None

    def __init__(self, exe, seed, scale, sabotage=None):
        # The self-test makes the software pass's Faraday wrong on its own
        # (`lightning.aezeed-soft`), the first pass's with `lightning.aezeed`.
        soft_sabotage = None
        if sabotage == "lightning.aezeed-soft":
            sabotage, soft_sabotage = None, "lightning.aezeed"
        super().__init__(exe, seed, scale, sabotage)
        rust_exe, lnd_exe_soft = soft_exes()
        self.soft_rust = Rust(rust_exe, self.name, soft_sabotage)
        self.soft_lnd = Lnd(lnd_exe_soft)
        backend = self.soft_rust.call("lightning.backend")
        if backend != ("ok", {"aes": "soft"}):
            raise HarnessError(
                f"the software-round pass's Faraday command is not built with the software "
                f"AES round: {backend}")

    def lnd_ref(self):
        if self.lnd_proc is None:
            self.lnd_proc = Lnd()
        return self.lnd_proc

    def close(self):
        if self.lnd_proc is not None:
            self.lnd_proc.close()
        if self.soft_lnd is not None:
            self.soft_lnd.close()
        if self.soft_rust is not None:
            self.soft_rust.close()
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
        inputs = {"words": words, "passphrase": passphrase.hex(), "cost": cost}
        got = self.compare("lightning.aezeed", inputs, ref, expected)
        self.aezeed_soft(inputs, expected)
        return got

    def aezeed_soft(self, inputs, expected):
        """The same case with the aes crate's software round against LND's
        AEZ without assembly."""
        rust = self.soft_rust.call("lightning.aezeed", **inputs)
        lnd = reference(lambda: self.soft_lnd.call(
            "aezeed", words=[ENGLISH.wordlist[w] for w in inputs["words"]],
            passphrase=inputs["passphrase"], cost=inputs["cost"]))
        self.compared += 1
        op = "lightning.aezeed-soft"
        if rust[0] != lnd[0] or (rust[0] == "ok" and rust[1] != lnd[1]):
            raise Mismatch(self.name, op, inputs, rust, lnd)
        if expected is not None and (expected[0] != lnd[0]
                                     or (expected[0] == "ok" and expected[1] != lnd[1])):
            raise Mismatch(self.name, op, inputs, rust, lnd,
                           f"both differ from the published vector {expected}")

    def invoice(self, text, expect, tagged=None):
        """A BOLT #11 invoice: Faraday's reading against the `bolt11`
        package's, field by field where both accept. `expect` is what
        Faraday must do ("ok" or "err": the invoice was built valid, or
        spoiled, or is a published vector). The package is laxer, so
        where Faraday refuses and the package accepts, the refusal must be
        one of Faraday's stricter rules (`STRICTER`) and counts as a known
        difference; Faraday accepting what the package refuses is a
        disagreement. `tagged` is the writer's list of fields, which Faraday's
        whole list of fallbacks (versions 0 to 18) and its unknown fields are
        also compared with."""
        inputs = {"invoice": text}
        rust = self.rust.call("lightning.bolt11", **inputs)
        ref = reference(lambda: bolt11_ref(text))
        self.compared += 1
        if rust[0] != expect:
            raise Mismatch(self.name, "lightning.bolt11", inputs, rust, ref,
                           f"Faraday must answer {expect}")
        if rust[0] == "err":
            if ref[0] == "ok":
                reason = re.match(r"\w+", rust[1]).group(0)
                if reason not in STRICTER:
                    raise Mismatch(self.name, "lightning.bolt11", inputs, rust, ref,
                                   "Faraday refuses what the package accepts, not by a stricter rule")
                self.known += 1
            return
        if ref[0] != "ok":
            raise Mismatch(self.name, "lightning.bolt11", inputs, rust, ref,
                           "Faraday accepts what the package refuses")
        got, want = bolt11_view(rust[1]), ref[1]
        if want["fallback"] is not None and want["fallback"]["version"] > 18:
            got["fallback"] = want["fallback"] = None
        if got != want:
            raise Mismatch(self.name, "lightning.bolt11", inputs, got, want)
        if tagged is not None:
            unknown = [{"tag": t, "data": v} for t, v in tagged
                       if t in bolt11_enc.UNDEFINED_TAGS or (t == "f" and v[0] > 18)]
            known = [{"version": v[0], "data": bolt11_enc.from5_bytes(v[1:]).hex()}
                     for t, v in tagged if t == "f" and v[0] <= 18]
            if rust[1]["fallbacks"] != known:
                raise Mismatch(self.name, "lightning.bolt11", inputs, rust[1]["fallbacks"], known,
                               "the fallbacks, as the writer put them")
            if rust[1]["unknown"] != unknown:
                raise Mismatch(self.name, "lightning.bolt11", inputs, rust[1]["unknown"], unknown,
                               "the fields BOLT #11 does not define, as the writer put them")

    def invoices_corpus(self):
        rng = self.rng
        inv = bolt11_enc.random_invoice(rng)
        text = bolt11_enc.text_of(inv)
        # Faraday refuses a text over 1023 characters (BOLT #11 says to read it).
        self.invoice(text, "ok" if len(text) <= 1023 else "err", inv["tagged"])
        self.invoice(bolt11_enc.damaged(text, rng), "err")
        self.invoice(bolt11_enc.mixed_case(text, rng), "err")
        for kind in rng.sample(bolt11_enc.KINDS, 2):
            bad = bolt11_enc.spoil(inv, kind, rng)
            # A high-S signature fails the check against an `n` field; the key
            # recovered from one is the same key.
            high_s_ok = kind == "high_s" and not any(t == "n" for t, _ in inv["tagged"])
            bad_text = bolt11_enc.text_of(bad)
            self.invoice(bad_text, "ok" if high_s_ok and len(bad_text) <= 1023 else "err")

    def published(self):
        for v in json.loads((ROOT / "tools/vectors/lightning-bolt11/invoice-test.json").read_text()):
            self.invoice(v["invoice"], "ok" if v["valid"] else "err")
        self.published_aezeed()

    def published_aezeed(self):
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

    LOOPS = (("corpus", 400), ("invoices", 150))

    def prelude(self):
        self.published()

    def case_corpus(self):
        self.corpus_case()

    def case_invoices(self):
        self.invoices_corpus()
