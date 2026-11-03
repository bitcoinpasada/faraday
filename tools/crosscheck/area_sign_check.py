"""The signature check of a PSBT (`faraday_psbt::verify`) against python-ecdsa,
btclib, btclib's script engine and Bitcoin Core v31.1.

A PSBT reaches the device carrying signatures: partial ones in
partial-signature, taproot key and script fields, and complete ones in a
final scriptSig or witness. Faraday checks each against its signature hash
and says whether it is the key's (`verify_signatures`), and reports one
nonce used in more than one place (`repeated_nonces`). Core's
`finalizepsbt` checks signatures with libsecp256k1, the library Faraday
checks them with, so Core alone is not a second opinion on a signature
(ESTABLISHED §9.3). Each signature is therefore judged by two
implementations from elsewhere, on signature hashes neither Faraday nor
Core computes:

- the signature hash is btclib's (`btclib/script/sig_hash.py`: legacy,
  SegWit v0 and taproot, key path and script path) over the unsigned
  transaction and every previous output;
- python-ecdsa (ECDSA over its own curve, and BIP-340 written here over
  its points: the equation `s*G = R + e*P` with an even `R.y`) and
  btclib (`ecc.ecc.dsa` and `ssa`, on its own curve) each verify the
  signature; they must agree, and an ECDSA signature is valid when it is
  strict DER with a defined sighash byte, a low S, and verifies;
- a signature that is not well formed (bad DER, a sighash byte no
  standard defines, a Schnorr signature of another length or with an
  explicit 0x00 byte) is judged "malformed": Faraday must not call it
  valid, and may refuse the PSBT or leave it out;
- a complete input, with its scriptSig and witness assembled here from the
  signatures given, is run by btclib's script engine (`verify_input`, with
  the standardness rules on: strict DER, low S, null fail, clean stack),
  and must pass exactly when every signature it needs is valid;
- Core's `finalizepsbt` finalizes a PSBT exactly when every input has its
  signatures and they verify (an unfinalized input is finalized only when
  they do), and `testmempoolaccept` takes the extracted transaction when
  the run gave the area coins on the node (`Area.FUND`).

What is ours is the report of a reused nonce: the definition is that two
signatures under one key, over different messages, with the same `r`
(ECDSA; `k` and `-k` give one `r`) or the same `R` (Schnorr) give the key
away. `reused()` finds them from the signatures that verify, without
Faraday's code, and Faraday's report must be the same: the same key, the
same nonce, the same inputs. Cases reuse a nonce on purpose, across inputs
that share a key (legacy, SegWit, multisig, taproot key and script
path), and the near misses: one nonce under two keys, one key under two
nonces.
"""

import base64
import hashlib

import core_node
import ecdsa
import lineage
from area_psbt import MAGIC, ser_compact, write_map
from area_sign import STANDARD, TAPROOT, p2pkh, p2wpkh, p2wsh
from bip32_ref import hash160
from common import Area, HarnessError, Mismatch, use_reference
from ecdsa.util import sigdecode_der, sigencode_der

use_reference("core")
import test_framework.script as S  # noqa: E402
from test_framework.messages import (  # noqa: E402
    COutPoint, CTransaction, CTxIn, CTxInWitness, CTxOut)

from btclib.script import sig_hash as btclib_sig_hash  # noqa: E402
from btclib.script.engine import verify_input  # noqa: E402
from btclib.script.taproot import leaf_hash  # noqa: E402
from btclib.tx import Tx as BtclibTx  # noqa: E402
from btclib.tx import TxOut as BtclibTxOut  # noqa: E402
from ecc.ecc import dsa as btclib_dsa  # noqa: E402
from ecc.ecc import ssa as btclib_ssa  # noqa: E402

CURVE = ecdsa.SECP256k1
N = CURVE.order
FIELD = CURVE.curve.p()
G = CURVE.generator
INFINITY = ecdsa.ellipticcurve.INFINITY
TAPSCRIPT = 0xC0
# The standardness rules the script engine is run under (Core's
# STANDARD_SCRIPT_VERIFY_FLAGS, less the ones about upgrades).
FLAGS = ("P2SH,DERSIG,NULLDUMMY,CHECKLOCKTIMEVERIFY,CHECKSEQUENCEVERIFY,WITNESS,TAPROOT,"
         "STRICTENC,LOW_S,NULLFAIL,CLEANSTACK,MINIMALDATA,WITNESS_PUBKEYTYPE,SIGPUSHONLY")
ECDSA_KINDS = ["p2pkh", "p2wpkh", "p2wsh-ms"]
SCHNORR_KINDS = ["tr-key", "tr-script"]
FLAWS = ["good"] * 12 + ["wrong-key"] * 2 + ["wrong-sighash"] * 2 + ["high-s"] + ["malformed"] * 3


# ----- the curve, from python-ecdsa's points -------------------------------------------

def jac(point):
    return ecdsa.ellipticcurve.PointJacobi(CURVE.curve, point.x(), point.y(), 1, N)


def mul(k):
    return G * (k % N)


def lift_x(x):
    """The point of x-only key `x` (an even y), or None."""
    y2 = (pow(x, 3, FIELD) + 7) % FIELD
    y = pow(y2, (FIELD + 1) // 4, FIELD)
    if x >= FIELD or y * y % FIELD != y2:
        return None
    return jac(ecdsa.ellipticcurve.Point(CURVE.curve, x, y if y % 2 == 0 else FIELD - y))


def compressed(point):
    return bytes([2 + (point.y() & 1)]) + point.x().to_bytes(32, "big")


def tagged(tag, data):
    h = hashlib.sha256(tag.encode()).digest()
    return hashlib.sha256(h + h + data).digest()


def ecdsa_sign(d, digest, k, high=False):
    """ECDSA from its definition, with the nonce `k`: strict DER, a low S
    (a high one when asked)."""
    r = mul(k).x() % N
    s = pow(k, -1, N) * (int.from_bytes(digest, "big") + r * d) % N
    s = N - min(s, N - s) if high else min(s, N - s)
    return sigencode_der(r, s, N)


def schnorr_sign(d, msg, k):
    """BIP-340's signature with the nonce `k`, from its definition."""
    if mul(d).y() & 1:
        d = N - d
    if mul(k).y() & 1:
        k = N - k
    r = mul(k).x().to_bytes(32, "big")
    e = int.from_bytes(tagged("BIP0340/challenge", r + mul(d).x().to_bytes(32, "big") + msg), "big") % N
    return r + ((k + e * d) % N).to_bytes(32, "big")


def schnorr_verifies(xonly, msg, sig):
    """BIP-340's verification over python-ecdsa's points: `s*G - e*P` is a
    point with an even y and the x of `r`."""
    r, s = int.from_bytes(sig[:32], "big"), int.from_bytes(sig[32:], "big")
    point = lift_x(int.from_bytes(xonly, "big"))
    if point is None or r >= FIELD or s >= N:
        return False
    e = int.from_bytes(tagged("BIP0340/challenge", sig[:32] + xonly + msg), "big") % N
    got = mul(s) + point * ((N - e) % N)
    return got != INFINITY and got.y() % 2 == 0 and got.x() == r


# ----- what is a good signature, from the specifications ----------------------------------

def judge_ecdsa(sig, pub, digest_of):
    """(status, r, message): "malformed" (not strict DER with a defined sighash
    byte), "invalid" (a high S, or it does not verify) or "valid"; python-ecdsa
    and btclib must agree."""
    if len(sig) < 9 or sig[-1] not in STANDARD:
        return "malformed", None, None
    try:
        r, s = sigdecode_der(sig[:-1], N)
    except Exception:  # noqa: BLE001 - python-ecdsa refuses the encoding
        return "malformed", None, None
    digest = digest_of(sig[-1])
    theirs = lineage.ecdsa_verifies(pub, digest, sig[:-1])
    btclib = btclib_dsa.verify_(digest, pub, sig[:-1]) and s <= N // 2
    if theirs != btclib:
        raise HarnessError(f"python-ecdsa says {theirs} and btclib {btclib} of {sig.hex()}")
    return ("valid" if theirs else "invalid"), r.to_bytes(32, "big"), digest


def judge_schnorr(sig, xonly, digest_of):
    """As `judge_ecdsa`, for BIP-340: 64 bytes, or 65 with a sighash byte
    that is not 0x00; the nonce is `R`'s x."""
    if len(sig) == 64:
        ht = 0
    elif len(sig) == 65 and sig[64] in TAPROOT and sig[64] != 0:
        ht = sig[64]
    else:
        return "malformed", None, None
    digest = digest_of(ht)
    theirs = schnorr_verifies(xonly, digest, sig[:64])
    btclib = btclib_ssa.verify_(digest, xonly, sig[:64])
    if theirs != btclib:
        raise HarnessError(f"python-ecdsa says {theirs} and btclib {btclib} of {sig.hex()}")
    return ("valid" if theirs else "invalid"), sig[:32], digest


def reused(sigs):
    """Costs nothing: the nonces used twice, from the definition. Two
    signatures that verify, under one key, over different messages, with one
    nonce (`r` of an ECDSA signature, `R`'s x of a Schnorr one): the
    key is the difference of their s values over the difference of their
    messages. Each is (kind, key, nonce, message, input), only those that
    verify. The reports, sorted: (kind, key, nonce, the inputs)."""
    groups = {}
    for kind, key, nonce, message, index in sigs:
        groups.setdefault((kind, key, nonce.hex()), []).append((message, index))
    out = []
    for (kind, key, nonce), members in sorted(groups.items()):
        if len({m for m, _ in members}) >= 2:
            out.append((kind, key, nonce, sorted(i for _, i in members)))
    return out


# ----- a known failure of Faraday ---------------------------------------------------------

def known_bug_taproot_sighash_zero(state, rec, verdict):
    """KNOWN_BUG_TAPROOT_SIGHASH_ZERO, the one place the area holds a bug of
    Faraday pending its fix: a finalized taproot signature (key path or script
    path) of 65 bytes whose last byte is 0x00 is malformed (BIP 341 spells
    the default type by leaving the byte out; Core and the judges refuse it),
    and `verify_signatures` calls it valid. Remove this function and its two
    callers when Faraday refuses such a signature; until then a verdict of
    valid for exactly these signatures is counted, and every other
    disagreement fails. When Faraday refuses them the rule matches none."""
    return (state == "final" and rec["kind"] == "schnorr" and rec["status"] == "malformed"
            and len(rec["bytes"]) == 65 and rec["bytes"][-1] == 0 and verdict == "valid")


# ----- inputs -------------------------------------------------------------------------

def witness_bytes(stack):
    return ser_compact(len(stack)) + b"".join(ser_compact(len(x)) + x for x in stack)


class Inp:
    """One input: what it spends, the keys of its script and what its
    signature hashes are."""

    def __init__(self, kind, value, rng, share=None):
        self.kind, self.value = kind, value
        scalar = lambda: rng.randrange(1, N)  # noqa: E731
        self.script = self.control = self.leaf = self.internal = None
        self.shared_slot = None
        if kind in ("p2pkh", "p2wpkh"):
            self.ds = [share or scalar()]
            self.m = 1
            self.shared_slot = 0 if share else None
            self.pubs = [compressed(mul(d)) for d in self.ds]
            code = p2pkh(hash160(self.pubs[0]))
            self.spk = code if kind == "p2pkh" else p2wpkh(hash160(self.pubs[0]))
            self.code = code
        elif kind == "p2wsh-ms":
            n = rng.randint(2, 3)
            self.m = rng.randint(1, n)
            self.ds = [scalar() for _ in range(n)]
            if share:
                self.shared_slot = rng.randrange(n)
                self.ds[self.shared_slot] = share
            self.pubs = [compressed(mul(d)) for d in self.ds]
            self.script = bytes(S.CScript([self.m] + self.pubs + [n, S.OP_CHECKMULTISIG]))
            self.spk = p2wsh(self.script)
        elif kind == "tr-key":
            d = share or scalar()
            point = mul(d)
            even = d if point.y() % 2 == 0 else N - d
            self.internal = point.x().to_bytes(32, "big")
            t = int.from_bytes(tagged("TapTweak", self.internal), "big")
            dd = (even + t) % N
            self.ds, self.m = [dd], 1
            self.shared_slot = 0 if share else None
            self.pubs = [mul(dd).x().to_bytes(32, "big")]
            self.spk = b"\x51\x20" + self.pubs[0]
        else:
            d = share or scalar()
            self.ds, self.m = [d], 1
            self.shared_slot = 0 if share else None
            self.pubs = [mul(d).x().to_bytes(32, "big")]
            self.script = b"\x20" + self.pubs[0] + b"\xac"
            self.leaf = leaf_hash(TAPSCRIPT, self.script)
            inner = scalar()
            self.internal = mul(inner).x().to_bytes(32, "big")
            t = int.from_bytes(tagged("TapTweak", self.internal + self.leaf), "big")
            top = lift_x(int.from_bytes(self.internal, "big")) + mul(t)
            self.control = bytes([TAPSCRIPT | (top.y() & 1)]) + self.internal
            self.spk = b"\x51\x20" + top.x().to_bytes(32, "big")
        self.taproot = kind.startswith("tr-")
        self.prev = None
        self.at = 0

    def twin(self):
        """A 2 of 2 multisig that names one key twice, so a signature and its
        high-S twin (the same `r`, the same message) both sit in one input."""
        self.m = 2
        self.ds = [self.ds[0]] * 2
        self.pubs = [self.pubs[0]] * 2
        self.script = bytes(S.CScript([2] + self.pubs + [2, S.OP_CHECKMULTISIG]))
        self.spk = p2wsh(self.script)

    def digest(self, tx, outs, index, ht):
        """btclib's signature hash for hash type `ht`."""
        if self.kind == "p2pkh":
            return btclib_sig_hash.legacy(self.code, tx, index, ht)
        if self.kind == "p2wpkh":
            return btclib_sig_hash.segwit_v0(self.code, tx, index, ht, self.value)
        if self.kind == "p2wsh-ms":
            return btclib_sig_hash.segwit_v0(self.script, tx, index, ht, self.value)
        if self.kind == "tr-key":
            return btclib_sig_hash.taproot(tx, index, outs, ht, 0, b"", b"")
        return btclib_sig_hash.taproot(tx, index, outs, ht, 1, b"",
                                       self.leaf + b"\x00" + b"\xff\xff\xff\xff")


def malform(rng, sig, ecdsa_kind):
    """A signature that is not well formed, from a good one."""
    if ecdsa_kind:
        body, ht = sig[:-1], sig[-1]
        return rng.choice([
            body[:-1] + bytes([ht]),
            bytes([body[0] ^ 1]) + body[1:] + bytes([ht]),
            body + bytes([rng.choice([0x00, 0x04, 0x05, 0x84])]),
            b"\x30\x00" + bytes([ht]),
        ])
    return rng.choice([sig[:-1], sig[:64] + b"\x00", sig[:64] + b"\x04", sig + b"\x01"])


class SignCheck(Area):
    name = "sign-check"
    faraday = (
        "`faraday_psbt::verify::verify_signatures` (every signature of a PSBT, in a partial "
        "field or a final scriptSig or witness, checked against its signature hash) and "
        "`faraday_psbt::verify::repeated_nonces` (one `r` or `R` under one key in more than "
        "one place)"
    )
    reference = (
        "python-ecdsa (the distribution's package, its own curve) and btclib (`btclib/`, "
        "libsecp256k1 off) verify every signature over btclib's signature hashes "
        "(`sig_hash.py`); btclib's script engine (`verify_input`, standardness rules on) runs "
        "every complete input; Bitcoin Core v31.1's `finalizepsbt` and `testmempoolaccept` "
        "judge the complete inputs and the transaction, and are named as sharing "
        "libsecp256k1 with Faraday; the reuse of a nonce is ours (`reused`, in "
        "`area_sign_check.py`)"
    )
    reference_checked = (
        "its vendored files and the btclib wheels are hashed against `references.sha256` "
        "before any run; Core's release tarball is checked against its pinned SHA-256 "
        "(`tools/reference/core/release.pin`) before every run, and the node's version is "
        "asked; the self-test makes Faraday's verdict wrong and its report of a nonce wrong, "
        "and each is caught"
    )
    vectors = (
        "none: BIP-340's and ECDSA's published vectors are in the areas that sign (`sign`, "
        "`msg`); the signatures here are made for the PSBT in the case, with verdicts "
        "given by two verifiers"
    )
    corpus = (
        "transactions of one to three inputs of legacy P2PKH, P2WPKH, P2WSH multisig "
        "(1 of 2 to 3 of 3), taproot key path and taproot script path, each with its "
        "signatures partial (in the PSBT's fields) or complete (in a final scriptSig or "
        "witness), good, under the wrong key, over the wrong sighash type, with a high S, or "
        "malformed, with every sighash type; nonces reused on purpose across inputs that "
        "share a key (ECDSA with `k` and `-k` too, Schnorr), one nonce under two keys, and one "
        "key under fresh nonces"
    )
    differences = [
        (
            "A partial signature that is not a strict DER signature with a defined sighash "
            "byte (or a taproot signature of 64 bytes, or 65 with a sighash byte that is "
            "defined and not 0x00): Bitcoin Core reads the PSBT and leaves the input "
            "unfinalized, Faraday refuses the PSBT or does not call the signature valid. "
            "Counted."
        ),
        (
            "A bug in Faraday, held until it is fixed (KNOWN_BUG_TAPROOT_SIGHASH_ZERO): a "
            "finalized taproot signature, key path or script path, of 65 bytes ending in the "
            "sighash byte 0x00. BIP 341 leaves the byte out for the default type, and Bitcoin "
            "Core, python-ecdsa and btclib call the signature malformed; `verify_signatures` "
            "calls it valid. Counted for exactly these signatures; every other disagreement "
            "fails, and the run passes when Faraday refuses them."
        ),
    ]
    not_compared = (
        "A PSBT with no previous output for an input (Faraday leaves the signature "
        "unchecked); signatures under a `pkh` leaf or a script that names no key; MuSig2 "
        "partial signatures (area `musig`); a finalized P2SH input; the order of the "
        "signatures within a reuse report"
    )
    CORE = True
    FUND = {"corpus": 3}
    FUND_DEPTH = 0
    LOOPS = (("corpus", 400),)

    def __init__(self, *a):
        super().__init__(*a)
        self.collecting = False

    # ----- the case -------------------------------------------------------------------

    def funding(self, spk, value, j):
        """The transaction the input spends: from the case's coin `j` on the
        node when the run gave coins (sent before the run, while
        `collecting`), else a made-up one."""
        coin = self.fund_coin(j)
        prev = CTransaction()
        prev.version = 2
        if coin is None:
            prev.vin = [CTxIn(COutPoint(int.from_bytes(self.randbytes(32), "big"),
                                        self.rng.randrange(4)), b"", 0xFFFFFFFF)]
        else:
            txid, vout, _ = coin
            prev.vin = [CTxIn(COutPoint(int(txid, 16), vout), b"", 0xFFFFFFFF)]
            w = CTxInWitness()
            w.scriptWitness.stack = [core_node.OP_TRUE]
            prev.wit.vtxinwit = [w]
        n = self.rng.randrange(1, 3)
        at = self.rng.randrange(n)
        prev.vout = [CTxOut(self.rng.randrange(1000, 500_000), p2wpkh(self.randbytes(20)))
                     for _ in range(n)]
        prev.vout[at] = CTxOut(value, spk)
        if coin is not None and sum(o.nValue for o in prev.vout) > coin[2]:
            raise HarnessError("a funding transaction pays more than its coin")
        if coin is not None and self.collecting:
            self.funding_txs.append(prev.serialize().hex())
        return prev, at

    def fund_corpus(self):
        self.collecting = True
        try:
            self.case_corpus(fund_only=True)
        finally:
            self.collecting = False


    def bad(self, op, inputs, faraday, reference, why):
        raise Mismatch(self.name, op, inputs, ("ok", faraday), ("ok", reference), why)

    def case_corpus(self, fund_only=False):
        rng = self.rng
        n_in = rng.randint(1, 3)
        mode = rng.choice(["plain"] * 4 + ["reuse"] * 3 + ["negated", "other-key", "fresh", "twin"])
        scheme = rng.choice(["ecdsa", "schnorr"])
        if mode == "twin":
            n_in = 1
        elif mode != "plain":
            n_in = rng.randint(2, 3)
        shared, k0 = rng.randrange(1, N), rng.randrange(1, N)
        inputs = []
        one_kind = rng.choice(SCHNORR_KINDS)
        for i in range(n_in):
            if mode != "plain" and scheme == "schnorr":
                # a taproot key path and a script path are not under one key
                kind = one_kind
            elif mode == "plain":
                kind = rng.choice(ECDSA_KINDS + SCHNORR_KINDS)
            else:
                kind = rng.choice(ECDSA_KINDS if scheme == "ecdsa" else SCHNORR_KINDS)
            same_key = mode in ("reuse", "negated", "fresh")
            if mode == "twin":
                kind = "p2wsh-ms"
            inp = Inp(kind, rng.randrange(20_000, 3_000_000), rng, shared if same_key else None)
            if mode == "twin":
                inp.twin()
            inp.prev, inp.at = self.funding(inp.spk, inp.value, i)
            inputs.append(inp)
        if fund_only:
            return
        n_out = rng.randint(n_in, n_in + 1)
        tx = CTransaction()
        tx.version = 2
        tx.nLockTime = 0
        for inp in inputs:
            tx.vin.append(CTxIn(COutPoint(inp.prev.txid_int, inp.at), b"", 0xFFFFFFFE))
        spare = sum(i.value for i in inputs) - rng.randrange(2_000, 20_000)
        each = spare // n_out
        for j in range(n_out):
            value = each if j < n_out - 1 else spare - each * (n_out - 1)
            tx.vout.append(CTxOut(value, p2wpkh(self.randbytes(20))))
        raw = tx.serialize_without_witness()
        unsigned = BtclibTx.parse(raw, check_validity=False)
        outs = [BtclibTxOut(i.value, i.spk, check_validity=False) for i in inputs]
        sigs = []
        finals = []
        for index, inp in enumerate(inputs):
            sigs += self.sign(inp, index, unsigned, outs, mode, k0, finals)
        self.judge(tx, inputs, unsigned, outs, sigs, finals, mode)

    def sign(self, inp, index, unsigned, outs, mode, k0, finals):
        """The signatures an input carries, as records of what each is and
        how it is stored; `finals` gets (how, records) for the input."""
        rng = self.rng
        ecdsa_kind = not inp.taproot
        if mode == "twin":
            return self.sign_twin(inp, index, unsigned, outs, finals)
        state = rng.choice(["partial", "final", "partial"])
        low = 1 if state == "final" or mode != "plain" else 0
        count = inp.m if rng.random() < 0.85 else rng.randint(low, inp.m)
        slots = sorted(rng.sample(range(len(inp.ds)), count))
        if mode in ("reuse", "negated", "fresh") and inp.shared_slot not in slots:
            slots = sorted(slots[:-1] + [inp.shared_slot])
        records = []
        for j in slots:
            sharing = (mode in ("reuse", "negated", "fresh") and j == inp.shared_slot) or \
                (mode == "other-key" and j == slots[0])
            flaw = "good" if sharing else rng.choice(FLAWS)
            if flaw == "high-s" and not ecdsa_kind:
                flaw = "good"
            kinds = STANDARD if ecdsa_kind else TAPROOT
            ht = 1 if ecdsa_kind and rng.random() < 0.6 else rng.choice(kinds)
            if not ecdsa_kind and rng.random() < 0.5:
                ht = 0
            k = rng.randrange(1, N)
            if sharing and mode in ("reuse", "other-key"):
                k = k0
            elif sharing and mode == "negated":
                k = k0 if index % 2 == 0 else N - k0
            hash_ht = rng.choice([x for x in kinds if x != ht]) if flaw == "wrong-sighash" else ht
            digest = inp.digest(unsigned, outs, index, hash_ht)
            secret = rng.randrange(1, N) if flaw == "wrong-key" else inp.ds[j]
            if ecdsa_kind:
                sig = ecdsa_sign(secret, digest, k, high=flaw == "high-s") + bytes([ht])
            else:
                sig = schnorr_sign(secret, digest, k) + (bytes([ht]) if ht else b"")
            if flaw == "malformed":
                sig = malform(rng, sig, ecdsa_kind)
            key = inp.pubs[j]
            status, nonce, message = (judge_ecdsa if ecdsa_kind else judge_schnorr)(
                sig, key, lambda h: inp.digest(unsigned, outs, index, h))
            records.append({"input": index, "kind": "ecdsa" if ecdsa_kind else "schnorr",
                            "key": key, "bytes": sig, "status": status, "nonce": nonce,
                            "message": message, "state": state})
        finals.append((state, records))
        return records

    def sign_twin(self, inp, index, unsigned, outs, finals):
        """A signature and its high-S twin under one key, over one message:
        one nonce, and no key is given away, so no reuse is to be reported."""
        rng = self.rng
        ht = rng.choice(STANDARD)
        digest = inp.digest(unsigned, outs, index, ht)
        k = rng.randrange(1, N)
        records = []
        for high in (False, True):
            sig = ecdsa_sign(inp.ds[0], digest, k, high=high) + bytes([ht])
            status, nonce, message = judge_ecdsa(
                sig, inp.pubs[0], lambda h: inp.digest(unsigned, outs, index, h))
            records.append({"input": index, "kind": "ecdsa", "key": inp.pubs[0], "bytes": sig,
                            "status": status, "nonce": nonce, "message": message,
                            "state": "final"})
        finals.append(("final", records))
        return records

    # ----- the PSBT, and the judges ---------------------------------------------------------

    def psbt(self, tx, inputs, finals):
        g = [(b"\x00", tx.serialize_without_witness())]
        maps = []
        assembled = []
        for index, (inp, (state, recs)) in enumerate(zip(inputs, finals)):
            m = [(b"\x00", inp.prev.serialize_without_witness())] if inp.kind == "p2pkh" else \
                [(b"\x01", CTxOut(inp.value, inp.spk).serialize())]
            sigs = [r["bytes"] for r in recs]
            if state != "final" and recs and len({r["bytes"][-1] for r in recs}) == 1 and len(recs[0]["bytes"]) in (65, 72, 71, 70, 73, 74):
                m.append((b"\x03", int(recs[0]["bytes"][-1]).to_bytes(4, "little")))
            script_sig, stack = self.assemble(inp, recs)
            assembled.append((script_sig, stack))
            if state == "final":
                if script_sig:
                    m.append((b"\x07", script_sig))
                if stack:
                    m.append((b"\x08", witness_bytes(stack)))
            else:
                if inp.kind in ("p2pkh", "p2wpkh", "p2wsh-ms"):
                    if inp.script:
                        m.append((b"\x05", inp.script))
                    m += [(b"\x02" + r["key"], r["bytes"]) for r in recs]
                else:
                    m.append((b"\x17", inp.internal))
                    if inp.kind == "tr-script":
                        m.append((b"\x15" + inp.control, inp.script + bytes([TAPSCRIPT])))
                    for sig in sigs:
                        if inp.kind == "tr-key":
                            m.append((b"\x13", sig))
                        else:
                            m.append((b"\x14" + inp.pubs[0] + inp.leaf, sig))
            maps.append(m)
        raw = MAGIC + write_map(g) + b"".join(write_map(m) for m in maps) + \
            b"\0" * len(tx.vout)
        return raw, assembled

    def assemble(self, inp, recs):
        """The final scriptSig and witness stack the signatures given make."""
        sigs = [r["bytes"] for r in recs]
        if inp.kind == "p2pkh":
            return (bytes(S.CScript([sigs[0], inp.pubs[0]])) if sigs else b""), []
        if inp.kind == "p2wpkh":
            return b"", ([sigs[0], inp.pubs[0]] if sigs else [])
        if inp.kind == "p2wsh-ms":
            return b"", [b""] + sigs + [inp.script]
        if inp.kind == "tr-key":
            return b"", sigs[:1]
        return b"", (sigs[:1] + [inp.script, inp.control]) if sigs else []

    def judge(self, tx, inputs, unsigned, outs, sigs, finals, mode):
        needs_malformed_refusal = any(r["status"] == "malformed" and r["state"] == "partial"
                                      for r in sigs)
        raw, assembled = self.psbt(tx, inputs, finals)
        complete = []
        for inp, (state, recs) in zip(inputs, finals):
            complete.append(len(recs) == inp.m and all(r["status"] == "valid" for r in recs))
        given = {"psbt": raw.hex(), "network": "mainnet"}
        # The engine runs every input with all its signatures given.
        final_tx = CTransaction()
        final_tx.version, final_tx.nLockTime = tx.version, tx.nLockTime
        final_tx.vin = [CTxIn(v.prevout, script_sig, v.nSequence)
                        for v, (script_sig, _) in zip(tx.vin, assembled)]
        final_tx.vout = tx.vout
        final_tx.wit.vtxinwit = []
        for _, stack in assembled:
            w = CTxInWitness()
            w.scriptWitness.stack = stack
            final_tx.wit.vtxinwit.append(w)
        engine_tx = BtclibTx.parse(final_tx.serialize(), check_validity=False)
        for index, (inp, (state, recs)) in enumerate(zip(inputs, finals)):
            if state != "final" and len(recs) != inp.m:
                continue
            try:
                verify_input(outs, engine_tx, index, FLAGS)
                passes = True
            except Exception:  # noqa: BLE001 - every refusal of the script engine
                passes = False
            self.compared += 1
            if passes != complete[index]:
                self.bad("sign-check.verify", given, {"input": index, "engine passes": passes},
                         {"input": index, "all signatures valid": complete[index]},
                         "btclib's script engine and the signature judges differ")
        self.core(raw, inputs, finals, complete, given)
        # Faraday.
        got = self.rust.call("sign-check.verify", **given)
        self.compared += 1
        if got[0] == "err":
            if needs_malformed_refusal:
                self.known += 1
                return
            raise Mismatch(self.name, "sign-check.verify", given, got, ("ok", "a verdict per signature"),
                           "Faraday refuses a PSBT whose signatures are all well formed")
        self.verdicts(given, got[1]["inputs"], inputs, finals)
        counted = self.rust.call("sign-check.reuse", **given)
        wanted = reused([(r["kind"], r["key"].hex(), r["nonce"], r["message"], r["input"])
                         for r in sigs if r["status"] == "valid"])
        if counted[0] != "ok":
            raise Mismatch(self.name, "sign-check.reuse", given, counted, ("ok", wanted), "refused")
        report = sorted((x["kind"], x["key"], x["nonce"], sorted(x["inputs"])) for x in counted[1]["reused"])
        wanted = [tuple(x) for x in wanted]
        self.compared += 1
        if counted[1]["count"] != len(wanted) or report != wanted:
            raise Mismatch(self.name, "sign-check.reuse", given, counted,
                           ("ok", {"count": len(wanted), "reused": wanted}),
                           "the report of a reused nonce differs")
        if mode in ("reuse", "negated") and not wanted:
            raise HarnessError("a case made to reuse a nonce reuses none")

    def verdicts(self, given, answer, inputs, finals):
        if len(answer) != len(inputs):
            self.bad("sign-check.verify", given, {"inputs": len(answer)}, {"inputs": len(inputs)},
                     "the number of inputs")
        for index, ((state, recs), listed) in enumerate(zip(finals, answer)):
            entries = list(zip(listed["checked"], listed["signatures"]))
            by_bytes = {}
            for verdict, e in entries:
                by_bytes.setdefault(e["signature"], []).append((verdict, e))
            wanted = {r["bytes"].hex() for r in recs}
            for stray in set(by_bytes) - wanted:
                self.bad("sign-check.verify", given, {"input": index, "signature": stray},
                         {"input": index, "signatures": sorted(wanted)},
                         "Faraday lists a signature that is not on the input")
            for r in recs:
                found = by_bytes.get(r["bytes"].hex(), [])
                self.compared += 1
                if r["status"] == "malformed":
                    if any(v == "valid" and known_bug_taproot_sighash_zero(state, r, v)
                           for v, _ in found):
                        self.known += 1
                        continue
                    if any(v == "valid" for v, _ in found):
                        self.bad("sign-check.verify", given, {"verdict": "valid"},
                                 {"verdict": "malformed"}, f"input {index}: a malformed signature")
                    continue
                if len(found) != 1:
                    self.bad("sign-check.verify", given, {"input": index, "listed": len(found)},
                             {"input": index, "listed": 1}, "a signature is not listed once")
                verdict, e = found[0]
                if verdict != r["status"]:
                    self.bad("sign-check.verify", given, {"input": index, "verdict": verdict},
                             {"input": index, "verdict": r["status"]},
                             f"{r['kind']} signature {r['bytes'].hex()}")
                if e["finalized"] != (state == "final") or e["kind"] != r["kind"]:
                    self.bad("sign-check.verify", given, {"kind": e["kind"], "finalized": e["finalized"]},
                             {"kind": r["kind"], "finalized": state == "final"}, "how it is stored")
                if (r["status"] == "valid" or state == "partial") and e["key"] != self.key_of(
                        inputs[index], r):
                    self.bad("sign-check.verify", given, {"key": e["key"]},
                             {"key": self.key_of(inputs[index], r)}, "the key it is under")
                leaf = inputs[index].leaf.hex() if inputs[index].kind == "tr-script" else None
                if e["leaf"] != leaf:
                    self.bad("sign-check.verify", given, {"leaf": e["leaf"]}, {"leaf": leaf}, "the leaf")

    @staticmethod
    def key_of(inp, r):
        return r["key"].hex()

    @staticmethod
    def describe(inputs, finals, complete):
        return "; ".join(
            f"{i.kind} {st} complete={c} " + ",".join(
                f"{r['status']}/{r['bytes'][-1]:x}/{len(r['bytes'])}" for r in recs)
            for i, (st, recs), c in zip(inputs, finals, complete))

    def core(self, raw, inputs, finals, complete, given):
        """Core finalizes exactly the inputs whose signatures are good, and
        takes the whole transaction when every input is complete."""
        # A PSBT input names one sighash type, so Core is not asked of a
        # partial input whose signatures carry different ones.
        if any(st != "final" and len({r["bytes"][-1] for r in recs}) > 1 for st, recs in finals):
            return
        b64 = base64.b64encode(raw).decode()
        done, whole = core_node.answers([("finalizepsbt", b64, True), ("finalizepsbt", b64, False)])
        self.compared += 1
        sound = all(complete)
        anyfinal = any(st == "final" for st, _ in finals)
        if done[0] == "err":
            if sound:
                self.bad("sign-check.verify", given, {"Core": done[1]}, {"complete": True},
                         "Core refuses a PSBT whose signatures are all good")
            return
        # Core takes an input that arrives final as it is, so its "complete"
        # says nothing of such an input's signatures; the transaction check
        # below judges those.
        if not anyfinal and done[1]["complete"] != sound:
            self.bad("sign-check.verify", given, {"Core complete": done[1]["complete"]},
                     {"complete": sound}, "Core's finalizepsbt and the signature judges differ: " + self.describe(inputs, finals, complete))
        g = whole[1]["psbt"]
        # Each input that was not final when it arrived is final in Core's
        # answer exactly when it was complete.
        answered = core_node.answers([("decodepsbt", g)])[0]
        if answered[0] == "ok":
            for index, (state, recs) in enumerate(finals):
                if state == "final":
                    continue
                entry = answered[1]["inputs"][index]
                has = "final_scriptwitness" in entry or "final_scriptSig" in entry
                self.compared += 1
                if has != complete[index]:
                    self.bad("sign-check.verify", given, {"input": index, "Core finalized": has},
                             {"input": index, "complete": complete[index]},
                             "Core finalizes an input whose signatures are not all good")
        if done[1].get("hex") and self.fund_coins is not None:
            tx_hex = done[1].get("hex")
            allowed = core_node.answers([("testmempoolaccept", [tx_hex])])[0]
            self.compared += 1
            took = allowed[0] == "ok" and bool(allowed[1][0].get("allowed"))
            if took != sound:
                self.bad("sign-check.verify", given, {"testmempoolaccept": allowed},
                         {"allowed": sound}, "Core and the signature judges differ on the transaction")
