"""Transactions and signature hashes against btclib and Bitcoin Core: btclib's
`Tx.parse` and `sig_hash` (`legacy`, `segwit_v0`, `taproot`), Core's
`decoderawtransaction` on the run's regtest node (`core_node.py`), and, for
the signature hashes, Core's functional-test framework (`core/test_framework/`):
`script.py`'s `LegacySignatureHash`, `SegwitV0SignatureHash` and
`TaprootSignatureHash`.

- Published: Core's 500 legacy signature hashes
  (`tools/vectors/sighash/core-sighash.json`), BIP-143's thirteen
  examples, and BIP-341's key-path signature hashes, each checked against
  the framework first.
- Random transactions of 1 to 5 inputs and 0 to 5 outputs, with and
  without witnesses, read by Faraday (`faraday_psbt::transaction::read_raw`)
  to their txid, wtxid, weight and bytes, and by btclib and by Core's
  `decoderawtransaction`; cut or lengthened ones refused.
- Every input of each signed by every kind of hash Faraday asks for:
  legacy (the six standard hash types, a random 32-bit one, and the
  SIGHASH_SINGLE case with no output, which hashes to one), BIP-143 for
  P2WPKH and P2WSH (the six standard types), BIP-341 key path and script
  path (the seven types, and a type BIP-341 does not define refused). Each
  hash is btclib's and, where btclib answers, the framework's too.

A raw transaction with no inputs is refused by Faraday; Core's
`decoderawtransaction` reads such bytes (a legacy transaction of no
inputs, `00 01` read as a count of 0 and one output) and btclib's
`Tx.parse` reads `00 00`, so that refusal is the area's one rule over both
judges. Where btclib reads a marker and inputs of bytes Core reads as a
legacy transaction of no inputs, the area counts the case as the known
difference and takes btclib's answer.

btclib refuses SIGHASH_SINGLE without an output and any BIP-341 hash type
it does not list, as BIP-341 does, which `TaprootSignatureMsg` hashes
anyway: for such an input btclib's refusal stands and the framework is not
asked.

Known difference, checked as the cases come: `bitcoin`'s
`legacy_signature_hash` hashes the script code as given, while Core and
btclib remove every OP_CODESEPARATOR from it first. Faraday's legacy spends
are P2PKH and P2SH multisig, whose script codes hold none.
"""

import io
import json

import core_node
from common import ROOT, Area, Mismatch, reference, use_reference, use_wheels

use_wheels("btclib", [
    "typing_extensions-4.16.0-py3-none-any.whl",
    "ecc-2026.10.23-py3-none-any.whl",
    "btclib-2026.10.24-py3-none-any.whl",
    "btclib_wallet-2026.10.29-py3-none-any.whl",
])
from ecc import curves  # noqa: E402

# btclib computes on its own pure-Python curve, never libsecp256k1's.
curves.set_libsecp256k1_serving(serving=False)
from btclib.script import sig_hash as btclib_sig_hash  # noqa: E402
from btclib.script.taproot import leaf_hash  # noqa: E402
from btclib.tx import Tx as BtclibTx  # noqa: E402
from btclib.tx import TxOut as BtclibTxOut  # noqa: E402

use_reference("core")
import test_framework.script as S  # noqa: E402
from test_framework.messages import (  # noqa: E402
    COutPoint, CTransaction, CTxIn, CTxInWitness, CTxOut)

STANDARD = [1, 2, 3, 0x81, 0x82, 0x83]
TAPROOT = [0, 1, 2, 3, 0x81, 0x82, 0x83]
HASH_ONE = (1).to_bytes(32, "little")
NO_INPUTS = "a transaction with no inputs"


class Refused(Exception):
    pass


def load(raw):
    """The framework's reading of bytes known to be a transaction (a
    published vector, or one the corpus wrote)."""
    tx = CTransaction()
    tx.deserialize(io.BytesIO(raw))
    return tx


def agreed(named):
    """What the judges say together: their one answer, a refusal when every
    judge refuses, and a value that names the disagreement when they differ
    (which Faraday's answer then differs from, whichever it is)."""
    kinds = {answer[0] for _, answer in named}
    if kinds == {"err"}:
        raise Refused(named[0][1][1])
    first = named[0][1][1]
    if kinds == {"ok"} and all(answer[1] == first for _, answer in named):
        return first
    return {"judges_differ": {name: list(answer) for name, answer in named}}


def btclib_reads(raw):
    """btclib's reading of `raw`; a transaction with no inputs is refused
    as Faraday refuses it."""
    tx = BtclibTx.parse(raw, check_validity=False)
    if not tx.vin:
        raise Refused(NO_INPUTS)
    return {"txid": tx.id.hex(), "wtxid": tx.hash.hex(),
            "raw": tx.serialize(include_witness=True, check_validity=False).hex(),
            "weight": tx.weight}


def core_reads(raw):
    """Core's `decoderawtransaction`, which reads the bytes whole or not at
    all. Its answer holds no bytes: they are `raw`."""
    answer = core_node.answers([("decoderawtransaction", raw.hex())])[0]
    if answer[0] == "err":
        return answer
    tx = answer[1]
    if not tx["vin"]:
        return ("err", NO_INPUTS)
    return ("ok", {"txid": tx["txid"], "wtxid": tx["hash"], "raw": raw.hex(),
                   "weight": tx["weight"]})


def parses(script):
    try:
        list(S.CScript(script))
        return True
    except S.CScriptInvalidError:
        return False


def has_codeseparator(script):
    try:
        return any(op == S.OP_CODESEPARATOR for op in S.CScript(script))
    except S.CScriptInvalidError:
        return S.OP_CODESEPARATOR in bytes(script)


def legacy(tx, index, code, ht, keep=False):
    """Core's legacy hash; `keep` hashes the script code as given."""
    if keep:
        saved = S.FindAndDelete
        S.FindAndDelete = lambda script, sig: script
    try:
        h, err = S.LegacySignatureHash(S.CScript(code), tx, index, ht)
    finally:
        if keep:
            S.FindAndDelete = saved
    if err and "out of range" in err and index >= len(tx.vin):
        raise Refused(err)
    return h


def taproot(tx, prevouts, ht, index, leaf=None):
    # BIP-342: with no OP_CODESEPARATOR executed, codesep_pos is
    # 0xffffffff (the framework's default of -1 does not encode).
    return S.TaprootSignatureHash(tx, prevouts, ht, index, scriptpath=leaf is not None,
                                  leaf_script=S.CScript(leaf) if leaf is not None else None,
                                  codeseparator_pos=0xFFFFFFFF)


def btclib_sighash(mode, raw, index, ht, kw):
    """btclib's signature hash of the kind `mode` names, over the
    transaction `raw`."""
    tx = BtclibTx.parse(raw, check_validity=False)
    if mode == "legacy":
        return btclib_sig_hash.legacy(kw["script_code"], tx, index, ht)
    if mode in ("p2wpkh", "p2wsh"):
        code = kw["script_code"]
        if mode == "p2wpkh":
            code = b"\x76\xa9\x14" + code[2:22] + b"\x88\xac"
        return btclib_sig_hash.segwit_v0(code, tx, index, ht, kw["amount"])
    prevouts = [BtclibTxOut(p.nValue, bytes(p.scriptPubKey), check_validity=False)
                for p in kw["prevouts"]]
    leaf = kw.get("leaf")
    if leaf is None:
        return btclib_sig_hash.taproot(tx, index, prevouts, ht, 0, b"", b"")
    # BIP-342's extension: the leaf hash, key version 0, codesep_pos 0xffffffff
    return btclib_sig_hash.taproot(tx, index, prevouts, ht, 1, b"",
                                   leaf_hash(0xC0, leaf) + b"\x00" + b"\xff\xff\xff\xff")


def core_sighash(mode, tx, index, ht, kw):
    """The framework's signature hash, as a hex string."""
    if mode == "legacy":
        h = legacy(tx, index, kw["script_code"], ht)
    elif mode in ("p2wpkh", "p2wsh"):
        code = kw["script_code"]
        if mode == "p2wpkh":
            code = b"\x76\xa9\x14" + code[2:22] + b"\x88\xac"
        h = S.SegwitV0SignatureHash(S.CScript(code), tx, index, ht, kw["amount"])
    else:
        h = taproot(tx, kw["prevouts"], ht, index, kw.get("leaf"))
    return h


class Tx(Area):
    name = "tx"
    CORE = True
    faraday = (
        "`faraday_psbt::transaction::read_raw`, `bitcoin`'s `SighashCache` as "
        '`faraday_psbt::sign` calls it'
    )
    reference = (
        "btclib's `Tx.parse` and `sig_hash` (`btclib/`); Bitcoin Core v31.1's "
        "`decoderawtransaction` on regtest (`core_node.py`) for the transactions; Core's "
        "`messages.py` and `script.py` (`core/`) for the signature hashes; the refusal of a "
        'transaction with no inputs is ours'
    )
    reference_checked = (
        'its vendored files and the btclib wheels are hashed against `references.sha256` '
        "before any run, and a published vector's answer is required of the reference as "
        'well as of Faraday; each published signature hash is checked against the '
        'framework first'
    )
    vectors = (
        "Core's 500 legacy sighashes, BIP-143's thirteen, BIP-341's key-path sighashes "
        '(`tools/vectors/sighash/`)'
    )
    corpus = (
        'random transactions with and without witnesses (txid, wtxid, weight, bytes) read by '
        "btclib and Core's `decoderawtransaction`, cut or lengthened; every input hashed as "
        'legacy, BIP-143 P2WPKH and P2WSH, BIP-341 key and script path under every hash '
        'type, by btclib and the framework'
    )
    differences = [
        (
            'Legacy sighash: `bitcoin` hashes the script code as given, Core and btclib '
            "remove every OP_CODESEPARATOR first. Faraday's legacy spends (P2PKH, P2SH "
            'multisig) have none.'
        ),
        (
            "Core's `decoderawtransaction` reads bytes whose marker `00 01` it takes as a "
            'count of no inputs and one output, a legacy transaction of no inputs; Faraday '
            'refuses a transaction with no inputs, and the area counts a case where btclib '
            'reads inputs of such bytes as this difference.'
        ),
        (
            "BIP-341's refusals (SIGHASH_SINGLE without an output, a hash type it does not "
            "list) are btclib's: the framework hashes those anyway and is not asked."
        ),
    ]
    not_compared = (
        'none'
    )

    def read_tx(self, raw):
        def ref():
            ours = reference(lambda: btclib_reads(raw))
            core = core_reads(raw)
            if core == ("err", NO_INPUTS) and ours[0] == "ok":
                self.known += 1
                return ours[1]
            return agreed([("btclib", ours), ("core", core)])
        return self.compare("tx.read", {"raw": raw.hex()}, ref)

    def sighash(self, mode, tx, index, ht, expected=None, **kw):
        raw = tx.serialize()
        inputs = {"mode": mode, "tx": raw.hex(), "index": index, "hash_type": ht}
        if "script_code" in kw:
            inputs["script_code"] = kw["script_code"].hex()
        if "amount" in kw:
            inputs["amount"] = kw["amount"]
        if "prevouts" in kw:
            inputs["prevouts"] = [[p.nValue, p.scriptPubKey.hex()] for p in kw["prevouts"]]
        if "leaf" in kw:
            inputs["leaf"] = kw["leaf"].hex()

        def ref():
            named = [("btclib", reference(lambda: btclib_sighash(mode, raw, index, ht, kw)))]
            if mode.startswith("taproot") and named[0][1][0] == "err":
                return agreed(named)
            named.append(("core", reference(lambda: core_sighash(mode, tx, index, ht, kw))))
            return {"sighash": agreed([(n, (k, v.hex() if k == "ok" else v))
                                       for n, (k, v) in named])}
        if mode == "legacy" and has_codeseparator(kw["script_code"]):
            keep = legacy(tx, index, kw["script_code"], ht, keep=True)
            plain = legacy(tx, index, kw["script_code"], ht)
            if keep != plain:
                rust = self.rust.call("tx.sighash", **inputs)
                judged = reference(ref)
                self.compared += 1
                self.known += 1
                if judged != ("ok", {"sighash": plain.hex()}):
                    raise Mismatch(self.name, "tx.sighash", inputs, rust, judged,
                                   "btclib and Core no longer agree on the hash that "
                                   "removes the OP_CODESEPARATORs")
                if rust != ("ok", {"sighash": keep.hex()}):
                    raise Mismatch(self.name, "tx.sighash", inputs, rust, ("ok", {"sighash": keep.hex()}),
                                   "the known difference (OP_CODESEPARATOR in a legacy "
                                   "script code) no longer holds")
                return rust
        return self.compare("tx.sighash", inputs, ref, expected)

    def published(self):
        vectors = json.loads((ROOT / "tools/vectors/sighash/core-sighash.json").read_text())
        for raw, script, index, ht, want in vectors[1:]:
            tx = load(bytes.fromhex(raw))
            ht &= 0xFFFFFFFF
            got = legacy(tx, index, bytes.fromhex(script), ht)
            if got[::-1].hex() != want:
                raise AssertionError("Core's test framework fails Core's sighash.json")
            self.sighash("legacy", tx, index, ht, script_code=bytes.fromhex(script))
        for case in json.loads((ROOT / "tools/vectors/sighash/bip143.json").read_text()):
            tx = load(bytes.fromhex(case["tx"]))
            self.sighash("p2wsh", tx, case["index"], case["hash_type"],
                         ("ok", {"sighash": case["sighash"]}),
                         script_code=bytes.fromhex(case["script_code"]), amount=case["amount"])
        bip341 = json.loads((ROOT / "tools/vectors/sighash/bip341-wallet-test-vectors.json")
                            .read_text())
        for spend in bip341["keyPathSpending"]:
            tx = load(bytes.fromhex(spend["given"]["rawUnsignedTx"]))
            prevouts = [CTxOut(u["amountSats"], bytes.fromhex(u["scriptPubKey"]))
                        for u in spend["given"]["utxosSpent"]]
            for inp in spend["inputSpending"]:
                self.sighash("taproot_key", tx, inp["given"]["txinIndex"],
                             inp["given"]["hashType"],
                             ("ok", {"sighash": inp["intermediary"]["sigHash"]}),
                             prevouts=prevouts)
            self.read_tx(tx.serialize())

    def random_script(self):
        rng = self.rng
        kind = rng.randrange(6)
        if kind == 0:
            return b"\x76\xa9\x14" + self.randbytes(20) + b"\x88\xac"
        if kind == 1:
            n = rng.randint(1, 3)
            keys = b"".join(b"\x21\x02" + self.randbytes(32) for _ in range(n))
            return bytes([0x50 + rng.randint(1, n)]) + keys + bytes([0x50 + n, 0xae])
        if kind == 2:
            return b"\x00\x14" + self.randbytes(20)
        if kind == 3:
            return b"\x51\x20" + self.randbytes(32)
        if kind == 4:
            return b"\x20" + self.randbytes(32) + b"\xac"
        return self.randbytes(rng.randint(0, 60))

    def random_tx(self):
        rng = self.rng
        tx = CTransaction()
        tx.version = rng.choice([1, 2, 3, rng.randrange(2**32)])
        tx.nLockTime = rng.choice([0, rng.randrange(500_000_000), rng.randrange(2**32)])
        for _ in range(rng.randint(1, 5)):
            tx.vin.append(CTxIn(COutPoint(rng.randrange(2**256), rng.randrange(2**32)),
                                self.randbytes(rng.choice([0, 0, rng.randint(1, 120)])),
                                rng.choice([0xFFFFFFFF, 0xFFFFFFFD, rng.randrange(2**32)])))
        for _ in range(rng.randint(0, 5)):
            tx.vout.append(CTxOut(rng.choice([0, 546, rng.randrange(21 * 10**14)]),
                                  self.random_script()))
        if rng.random() < 0.5:
            tx.wit.vtxinwit = []
            for _ in tx.vin:
                w = CTxInWitness()
                w.scriptWitness.stack = [self.randbytes(rng.choice([0, 33, 72, rng.randint(0, 200)]))
                                         for _ in range(rng.randint(0, 4))]
                tx.wit.vtxinwit.append(w)
        return tx

    def corpus_case(self):
        rng = self.rng
        tx = self.random_tx()
        raw = tx.serialize()
        self.read_tx(raw)
        if rng.random() < 0.3:
            bad = rng.choice([raw[:-rng.randint(1, min(10, len(raw)))], raw + b"\x00",
                              raw[:4] + b"\x00\x01" + raw[4:]])
            self.read_tx(bad)
        prevouts = [CTxOut(rng.randrange(21 * 10**14), self.random_script()) for _ in tx.vin]
        for index in range(len(tx.vin)):
            mode = rng.choice(["legacy", "p2wpkh", "p2wsh", "taproot_key", "taproot_script"])
            if mode == "legacy":
                ht = rng.choice(STANDARD + [rng.randrange(2**32)])
                # A script code Core's framework can read op by op, as
                # every script code Faraday hashes is.
                code = self.random_script()
                while not parses(code):
                    code = self.random_script()
                self.sighash(mode, tx, index, ht, script_code=code)
            elif mode == "p2wpkh":
                self.sighash(mode, tx, index, rng.choice(STANDARD),
                             script_code=b"\x00\x14" + self.randbytes(20),
                             amount=rng.randrange(21 * 10**14))
            elif mode == "p2wsh":
                self.sighash(mode, tx, index, rng.choice(STANDARD),
                             script_code=self.random_script(), amount=rng.randrange(21 * 10**14))
            else:
                ht = rng.choice(TAPROOT + ([4, 0x84, 0x80] if rng.random() < 0.1 else []))
                # A taproot input spends a P2TR output (the framework's
                # ANYONECANPAY message assumes one).
                spent = list(prevouts)
                spent[index] = CTxOut(prevouts[index].nValue, b"\x51\x20" + self.randbytes(32))
                kw = {"prevouts": spent}
                if mode == "taproot_script":
                    kw["leaf"] = self.random_script()
                self.sighash(mode, tx, index, ht, **kw)

    LOOPS = (("corpus", 3000),)

    def prelude(self):
        self.published()

    def case_corpus(self):
        self.corpus_case()
