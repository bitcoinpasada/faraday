"""Transactions and signature hashes against Bitcoin Core's
functional-test framework (`core/test_framework/`): `messages.py`'s
`CTransaction` (serialisation, txid, wtxid, weight) and `script.py`'s
`LegacySignatureHash`, `SegwitV0SignatureHash` and
`TaprootSignatureHash`.

- Published: Core's 500 legacy signature hashes
  (`tools/vectors/sighash/core-sighash.json`), BIP-143's thirteen
  examples, and BIP-341's key-path signature hashes, each checked against
  the reference first.
- Random transactions of 1 to 5 inputs and 0 to 5 outputs, with and
  without witnesses, read by Faraday (`faraday_psbt::transaction::read_raw`)
  to their txid, wtxid, weight and bytes; cut or lengthened ones refused.
- Every input of each signed by every kind of hash Faraday asks for:
  legacy (the six standard hash types and the SIGHASH_SINGLE case with no
  output, which hashes to one), BIP-143 for P2WPKH and P2WSH (the six
  standard types), BIP-341 key path and script path (the seven types,
  and a type BIP-341 does not define refused).

The reference is read as the BIPs read it where the test framework is
looser: BIP-341 fails SIGHASH_SINGLE without an output and any hash type
it does not list, which `TaprootSignatureMsg` hashes anyway, and a raw
transaction is read only when its bytes are exactly the serialisation of
what was read (the framework reads past a short input without saying so).

Known difference, checked as the cases come: `bitcoin`'s
`legacy_signature_hash` hashes the script code as given, while Core
removes every OP_CODESEPARATOR from it first. Faraday's legacy spends
are P2PKH and P2SH multisig, whose script codes hold none.
"""

import io
import json

from common import ROOT, Area, Mismatch, use_reference

use_reference("core")
import test_framework.messages as M  # noqa: E402
import test_framework.script as S  # noqa: E402
from test_framework.messages import (  # noqa: E402
    COutPoint, CTransaction, CTxIn, CTxInWitness, CTxOut)

STANDARD = [1, 2, 3, 0x81, 0x82, 0x83]
TAPROOT = [0, 1, 2, 3, 0x81, 0x82, 0x83]
HASH_ONE = (1).to_bytes(32, "little")


class Refused(Exception):
    pass


NOT_EXACT = "not exactly a transaction"


def read(raw):
    """Core's reading, refused unless `raw` is exactly the serialisation
    of what was read.

    The framework reads past the end of its input as if zeros followed,
    so a witness stack's count read from the last bytes can be in the
    millions, and its `ser_string_vector` writes such a stack back in
    time quadratic in its length (hours for one case). Every item of a
    stack is written back as at least one byte, and a stack of items is
    never left out of the serialisation, so a count larger than `raw`
    itself means the serialisation is longer than `raw`: the refusal
    below is the answer the full reading would reach, given at once.
    """
    tx = CTransaction()
    f = io.BytesIO(raw)
    whole = M.deser_string_vector

    def counted(stream):
        at = stream.tell()
        if M.deser_compact_size(stream) > len(raw):
            raise Refused(NOT_EXACT)
        stream.seek(at)
        return whole(stream)
    M.deser_string_vector = counted
    try:
        tx.deserialize(f)
    finally:
        M.deser_string_vector = whole
    if f.read() or tx.serialize() != raw:
        raise Refused(NOT_EXACT)
    return tx


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
    if ht not in TAPROOT:
        raise Refused("a hash type BIP-341 does not define")
    if ht & 3 == 3 and index >= len(tx.vout):
        raise Refused("SIGHASH_SINGLE without an output")
    # BIP-342: with no OP_CODESEPARATOR executed, codesep_pos is
    # 0xffffffff (the framework's default of -1 does not encode).
    return S.TaprootSignatureHash(tx, prevouts, ht, index, scriptpath=leaf is not None,
                                  leaf_script=S.CScript(leaf) if leaf is not None else None,
                                  codeseparator_pos=0xFFFFFFFF)


class Tx(Area):
    name = "tx"

    def read_tx(self, raw):
        def ref():
            tx = read(raw)
            return {"txid": tx.txid_hex, "wtxid": tx.wtxid_hex, "raw": tx.serialize().hex(),
                    "weight": tx.get_weight()}
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
            if mode == "legacy":
                h = legacy(tx, index, kw["script_code"], ht)
            elif mode in ("p2wpkh", "p2wsh"):
                if ht not in STANDARD:
                    raise Refused("not a standard hash type")
                code = kw["script_code"]
                if mode == "p2wpkh":
                    code = b"\x76\xa9\x14" + code[2:22] + b"\x88\xac"
                h = S.SegwitV0SignatureHash(S.CScript(code), tx, index, ht, kw["amount"])
            else:
                h = taproot(tx, kw["prevouts"], ht, index, kw.get("leaf"))
            return {"sighash": h.hex()}
        if mode == "legacy" and has_codeseparator(kw["script_code"]):
            keep = legacy(tx, index, kw["script_code"], ht, keep=True)
            if keep != legacy(tx, index, kw["script_code"], ht):
                rust = self.rust.call("tx.sighash", **inputs)
                self.compared += 1
                self.known += 1
                if rust != ("ok", {"sighash": keep.hex()}):
                    raise Mismatch(self.name, "tx.sighash", inputs, rust, ("ok", ref()),
                                   "the known difference (OP_CODESEPARATOR in a legacy "
                                   "script code) no longer holds")
                return rust
        return self.compare("tx.sighash", inputs, ref, expected)

    def published(self):
        vectors = json.loads((ROOT / "tools/vectors/sighash/core-sighash.json").read_text())
        for raw, script, index, ht, want in vectors[1:]:
            tx = read(bytes.fromhex(raw))
            ht &= 0xFFFFFFFF
            got = legacy(tx, index, bytes.fromhex(script), ht)
            if got[::-1].hex() != want:
                raise AssertionError("Core's test framework fails Core's sighash.json")
            self.sighash("legacy", tx, index, ht, script_code=bytes.fromhex(script))
        for case in json.loads((ROOT / "tools/vectors/sighash/bip143.json").read_text()):
            tx = read(bytes.fromhex(case["tx"]))
            self.sighash("p2wsh", tx, case["index"], case["hash_type"],
                         ("ok", {"sighash": case["sighash"]}),
                         script_code=bytes.fromhex(case["script_code"]), amount=case["amount"])
        bip341 = json.loads((ROOT / "tools/vectors/sighash/bip341-wallet-test-vectors.json")
                            .read_text())
        for spend in bip341["keyPathSpending"]:
            tx = read(bytes.fromhex(spend["given"]["rawUnsignedTx"]))
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
