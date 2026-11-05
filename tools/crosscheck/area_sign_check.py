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
- a final input of any script (a miniscript, a hash time lock, a multisig that
  names a key twice, bare, P2SH, P2WSH and P2SH-P2WSH, P2PKH, P2WPKH), built
  here from a table of rows, is run by btclib's script engine under
  consensus's rules alone (`CONSENSUS`), and Faraday's verdict on its final
  fields (`witness_mismatch`, else `final_invalid`) must be "refused" exactly
  when the engine fails it (a time lock included, with its sequence, lock time
  and transaction version), but for a high S, a sighash byte no standard
  defines, a junk element below the dummy of a legacy multisig and the other
  shapes stated under the area's differences, which
  Faraday refuses and consensus accepts, and for a script Faraday does not
  run (a hash time lock), which it answers "unjudged" whatever the engine
  decides;
- a final taproot input, built here from a second table of rows (an output
  key, a control block, a leaf, a witness stack and every spent output, with
  the signatures made here over btclib's signature hash), is run by btclib's
  script engine under consensus's rules with TAPROOT (`CONSENSUS_TAP`), and
  Faraday's verdict is judged as above. The rows are a key path (true,
  forged, SIGHASH_SINGLE with no output of its index, with an annex signed
  and with an annex the signature does not cover), a script path on a Tap
  miniscript leaf (a single key, a `multi_a`; true, forged, with an annex,
  with a control block that does not commit to the output key, with a
  65-byte element ending 0x00) and the leaves Faraday does not run (a leaf
  version other than 0xc0, a hash lock, a leaf with OP_CODESEPARATOR, a leaf
  with OP_DROP), which it answers "unjudged" whatever the engine decides;
- Core's `finalizepsbt` finalizes a PSBT exactly when every input has its
  signatures and they verify (an unfinalized input is finalized only when
  they do), and `testmempoolaccept` takes the extracted transaction when
  the run gave the area coins on the node (`Area.FUND`).

What is ours is the report of a reused nonce: Faraday's rule, failing
closed, is that two signatures that verify with the same `r` (ECDSA; `k`
and `-k` give one `r`) or the same `R` (Schnorr), differing in key or in
message, are reported, under one key or several. `reused()` finds them
from the signatures that verify, without Faraday's code, and Faraday's
report must be the same: the same keys, the same nonce, the same inputs.
Cases reuse a nonce on purpose, across inputs that share a key (legacy,
SegWit, multisig, taproot key and script path), under two keys (reported
as well), and the near miss of one key under two nonces.
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
# The rules of consensus alone (Core's block script flags: the soft forks
# through segwit, and BIP-147's empty dummy), which the final-input rows are
# judged under: a high S, a sighash byte no standard defines and the policy
# rules about stack shape and key form are not among them. A taproot row is
# judged under the same rules with TAPROOT (BIP-341 and BIP-342).
CONSENSUS = "P2SH,DERSIG,NULLDUMMY,CHECKLOCKTIMEVERIFY,CHECKSEQUENCEVERIFY,WITNESS"
CONSENSUS_TAP = CONSENSUS + ",TAPROOT"
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
    """Costs nothing: the nonces used twice, from Faraday's own rule, which
    fails closed. Signatures that verify and share one nonce (`r` of an
    ECDSA signature, `R`'s x of a Schnorr one) are reported when at least
    two of them differ in key or in message. With one key the private key
    is the difference of their s values over the difference of their
    messages; with keys whose difference is known it follows the same way;
    one nonce under unrelated keys is reported all the same, because an
    honest signer never does it. Each is (kind, key, nonce, message,
    input), only those that verify. The reports, sorted: (kind, the keys,
    sorted, in hex, the nonce, the inputs). The kind is the first's: the
    generator never shares a nonce between an ECDSA and a Schnorr
    signature."""
    groups = {}
    for kind, key, nonce, message, index in sigs:
        groups.setdefault(nonce.hex(), []).append((kind, key, message, index))
    out = []
    for nonce, members in sorted(groups.items()):
        if len({(k, m) for _, k, m, _ in members}) >= 2:
            out.append((members[0][0], sorted({k for _, k, _, _ in members}), nonce,
                        sorted({i for _, _, _, i in members})))
    return out


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


# ----- a final input of any script, judged by btclib's script engine --------------------------

W3 = ("p2wsh", "p2sh-p2wsh", "p2sh")
W4 = W3 + ("bare",)
JUNK = bytes([0xAA]) * 9


class Pd1(bytes):
    """An element a scriptSig pushes by PUSHDATA1 where a direct push fits."""


def push(data):
    """A data push, as a scriptSig carries it: an int is the opcode OP_n."""
    if isinstance(data, int):
        return bytes([0x50 + data])
    n = len(data)
    if isinstance(data, Pd1):
        return b"\x4c" + bytes([n]) + data
    if n <= 75:
        return bytes([n]) + data
    if n <= 255:
        return b"\x4c" + bytes([n]) + data
    return b"\x4d" + n.to_bytes(2, "little") + data


def cs(*items):
    return bytes(S.CScript(list(items)))


def ms(m, *keys):
    return cs(m, *keys, len(keys), S.OP_CHECKMULTISIG)


class Keys:
    """Twenty keys of a case, and the signatures of the row being built."""

    def __init__(self, rng):
        self.rng = rng
        self.ds = [rng.randrange(1, N) for _ in range(20)]

    def pub(self, i, comp=True):
        point = mul(self.ds[i])
        return compressed(point) if comp else b"\x04" + point.x().to_bytes(32, "big") + \
            point.y().to_bytes(32, "big")

    def sign(self, i, digest, ht=1, high=False):
        return ecdsa_sign(self.ds[i], digest, self.rng.randrange(1, N), high=high) + bytes([ht])


def pkh_code(key):
    return cs(S.OP_DUP, S.OP_HASH160, hash160(key), S.OP_EQUALVERIFY, S.OP_CHECKSIG)


def thresh_script(k):
    return cs(k.pub(0), S.OP_CHECKSIG, S.OP_SWAP, k.pub(1), S.OP_CHECKSIG, S.OP_ADD, S.OP_SWAP,
              k.pub(2), S.OP_CHECKSIG, S.OP_ADD, 2, S.OP_EQUAL)


def recovery_script(k, n=10):
    """or_d(pk(A), and_v(v:pkh(B), older(n)))."""
    return cs(k.pub(0), S.OP_CHECKSIG, S.OP_IFDUP, S.OP_NOTIF, S.OP_DUP, S.OP_HASH160,
              hash160(k.pub(1)), S.OP_EQUALVERIFY, S.OP_CHECKSIGVERIFY, n,
              S.OP_CHECKSEQUENCEVERIFY, S.OP_ENDIF)


PREIMAGE = bytes([7]) * 32


def lock_script(k):
    """A hash lock on A, or a time lock of 144 on B."""
    return cs(S.OP_IF, S.OP_SHA256, hashlib.sha256(PREIMAGE).digest(), S.OP_EQUALVERIFY,
              k.pub(0), S.OP_CHECKSIG, S.OP_ELSE, 144, S.OP_CHECKSEQUENCEVERIFY, S.OP_DROP,
              k.pub(1), S.OP_CHECKSIG, S.OP_ENDIF)


def puzzle(length):
    return cs(b"\x07" * length, S.OP_DROP, S.OP_1)


def row(name, wrappers, script, stack, seq=0xFFFFFFFF, diff=None, unrun=False, **tx):
    """(name, wrappers, script(keys) -> bytes, stack(keys, sign) -> [elements],
    the input's sequence, the standard Faraday is stricter than consensus by,
    whether the script is one Faraday does not run: True, or the wrappings in which
    it is not, as an uncompressed key is not read in a segwit script), and the
    transaction's `version` and `lock` time where they are not 2 and 0, and
    `pd1`: the redeem script a p2sh scriptSig pushes by PUSHDATA1."""
    return (name, wrappers, script, stack, seq, diff, unrun, tx)


def input_rows():
    out = []
    add = lambda *a, **kw: out.append(row(*a, **kw))  # noqa: E731
    two = lambda k: ms(2, k.pub(0), k.pub(1))  # noqa: E731
    pk = lambda k: cs(k.pub(0), S.OP_CHECKSIG)  # noqa: E731
    pkh = lambda k: pkh_code(k.pub(0))  # noqa: E731
    # and_v(v:pk(A),pk(B))
    and_v = lambda k: cs(k.pub(0), S.OP_CHECKSIGVERIFY, k.pub(1), S.OP_CHECKSIG)  # noqa: E731
    add("and_v, both", W3, and_v, lambda k, s: [s(1), s(0)])
    add("and_v, one signature", W3, and_v, lambda k, s: [s(0)])
    add("and_v, junk", W3, and_v, lambda k, s: [JUNK, s(0)])
    add("and_v, swapped", W3, and_v, lambda k, s: [s(0), s(1)])
    add("and_v, empty", W3, and_v, lambda k, s: [b"", s(0)])
    # thresh(2,pk(A),s:pk(B),s:pk(C))
    add("thresh, A and B", ("p2wsh",), thresh_script, lambda k, s: [b"", s(1), s(0)])
    add("thresh, A and C", ("p2wsh",), thresh_script, lambda k, s: [s(2), b"", s(0)])
    add("thresh, B and C", ("p2wsh",), thresh_script, lambda k, s: [s(2), s(1), b""])
    add("thresh, one", ("p2wsh",), thresh_script, lambda k, s: [b"", b"", s(0)])
    add("thresh, three", ("p2wsh",), thresh_script, lambda k, s: [s(2), s(1), s(0)])
    add("thresh, junk", ("p2wsh",), thresh_script, lambda k, s: [b"", JUNK, s(0)])
    # pk
    add("pk, true", W3, pk, lambda k, s: [s(0)])
    add("pk, junk", W3, pk, lambda k, s: [JUNK])
    add("pk, script alone", W3, pk, lambda k, s: [])
    add("pk, empty", W3, pk, lambda k, s: [b""])
    add("pk, wrong key", W3, pk, lambda k, s: [s(1)])
    # pkh
    add("pkh, true", W3, pkh, lambda k, s: [s(0), k.pub(0)])
    add("pkh, junk", W3, pkh, lambda k, s: [JUNK, k.pub(0)])
    add("pkh, wrong key", W3, pkh, lambda k, s: [s(2), k.pub(2)])
    add("pkh, forged", W3, pkh, lambda k, s: [s(2), k.pub(0)])
    # or_d with a recovery path
    add("recovery, B after the delay", ("p2wsh",), recovery_script,
        lambda k, s: [s(1), k.pub(1), b""], seq=10)
    add("recovery, B before the delay", ("p2wsh",), recovery_script,
        lambda k, s: [s(1), k.pub(1), b""], seq=9)
    add("recovery, A", ("p2wsh",), recovery_script, lambda k, s: [s(0)], seq=9)
    add("recovery, A forged", ("p2wsh",), recovery_script, lambda k, s: [s(2)], seq=10)
    add("recovery, junk", ("p2wsh",), recovery_script,
        lambda k, s: [JUNK, k.pub(1), b""], seq=10)
    add("recovery, wrong key", ("p2wsh", "p2sh"), recovery_script,
        lambda k, s: [s(2), k.pub(2), b""], seq=10)
    # multisig that names a key twice, in the order of its keys or not
    same = lambda k: ms(2, k.pub(0), k.pub(0))  # noqa: E731
    add("2-of-2 [A,A], one signature twice", W4, same, lambda k, s: [b"", (x := s(0)), x])
    add("2-of-2 [A,A], two signatures", W4, same, lambda k, s: [b"", s(0), s(0)])
    add("2-of-2 [A,A], one", W4, same, lambda k, s: [b"", s(0)])
    add("2-of-2 [A,A], ALL and ALL|ANYONECANPAY", W4, same,
        lambda k, s: [b"", s(0, 1), s(0, 0x81)])
    add("2-of-2 [A,A], ALL|ANYONECANPAY and ALL", W4, same,
        lambda k, s: [b"", s(0, 0x81), s(0, 1)])
    t3 = lambda k: ms(2, k.pub(0), k.pub(0), k.pub(1))  # noqa: E731
    add("2-of-3 [A,A,B], A A", W4, t3, lambda k, s: [b"", s(0), s(0)])
    add("2-of-3 [A,A,B], A B", W4, t3, lambda k, s: [b"", s(0), s(1)])
    add("2-of-3 [A,A,B], B A", W4, t3, lambda k, s: [b"", s(1), s(0)])
    aba = lambda k: ms(2, k.pub(0), k.pub(1), k.pub(0))  # noqa: E731
    add("2-of-3 [A,B,A], B A", W4, aba, lambda k, s: [b"", s(1), s(0)])
    add("2-of-3 [A,B,A], A B", W4, aba, lambda k, s: [b"", s(0), s(1)])
    add("2-of-3 [A,B,A], A A", W4, aba, lambda k, s: [b"", s(0), s(0)])
    # the dummy, padding, repeats and order in a 2-of-2 [A,B]
    add("2-of-2, empty dummy", W4, two, lambda k, s: [b"", s(0), s(1)])
    add("2-of-2, non-empty dummy", W4, two, lambda k, s: [b"\x01", s(0), s(1)])
    add("2-of-2, junk dummy", W4, two, lambda k, s: [JUNK, s(0), s(1)])
    # junk below the dummy: legacy consensus leaves it on the stack and accepts
    add("2-of-2, junk below the dummy", ("bare", "p2sh"), two,
        lambda k, s: [JUNK, b"", s(0), s(1)],
        diff="junk below the dummy of a legacy multisig")
    add("2-of-2, junk in place of a signature", W4, two, lambda k, s: [b"", JUNK, s(1)])
    add("2-of-2, duplicate", W4, two, lambda k, s: [b"", s(0), s(0)])
    add("2-of-2, swapped", W4, two, lambda k, s: [b"", s(1), s(0)])
    add("2-of-2, three signatures", W4, two, lambda k, s: [b"", s(0), s(1), s(1)])
    add("2-of-2, three signatures, the first twice", W4, two,
        lambda k, s: [b"", (x := s(0)), x, s(1)])
    add("2-of-2, one signature", W4, two, lambda k, s: [b"", s(0)])
    add("2-of-2, wrong key", W4, two, lambda k, s: [b"", s(0), s(2)])
    # many keys
    add("2-of-17", ("p2wsh",), lambda k: ms(2, *[k.pub(i) for i in range(17)]),
        lambda k, s: [b"", s(0), s(16)])
    add("2-of-17, out of order", ("p2wsh",), lambda k: ms(2, *[k.pub(i) for i in range(17)]),
        lambda k, s: [b"", s(16), s(0)])
    twenty = lambda k: ms(17, *[k.pub(i) for i in range(20)])  # noqa: E731
    add("17-of-20", ("p2wsh",), twenty, lambda k, s: [b""] + [s(i) for i in range(17)])
    add("17-of-20, the last three", ("p2wsh",), twenty,
        lambda k, s: [b""] + [s(i) for i in range(3, 20)])
    add("17-of-20, sixteen", ("p2wsh",), twenty, lambda k, s: [b""] + [s(i) for i in range(16)])
    add("17-of-20, one wrong", ("p2wsh",), twenty,
        lambda k, s: [b""] + [s(i if i != 5 else 19) for i in range(17)])
    # uncompressed keys
    unc = lambda k: ms(2, k.pub(0, False), k.pub(1, False))  # noqa: E731
    add("2-of-2 uncompressed", W4, unc, lambda k, s: [b"", s(0), s(1)])
    add("2-of-2 uncompressed, one", W4, unc, lambda k, s: [b"", s(0)])
    add("pk uncompressed", W3 + ("bare",), lambda k: cs(k.pub(0, False), S.OP_CHECKSIG),
        lambda k, s: [s(0)], unrun=("p2wsh", "p2sh-p2wsh"))
    # bare outputs
    add("p2pk", ("bare",), pk, lambda k, s: [s(0)])
    add("p2pk, junk", ("bare",), pk, lambda k, s: [JUNK])
    add("p2pk, wrong key", ("bare",), pk, lambda k, s: [s(1)])
    add("bare 2-of-3", ("bare",), lambda k: ms(2, k.pub(0), k.pub(1), k.pub(2)),
        lambda k, s: [b"", s(0), s(2)])
    add("bare 2-of-3, one", ("bare",), lambda k: ms(2, k.pub(0), k.pub(1), k.pub(2)),
        lambda k, s: [b"", s(0)])
    # p2pkh and p2wpkh
    add("single key, true", ("p2pkh", "p2wpkh"), pkh, lambda k, s: [s(0), k.pub(0)])
    add("single key, forged", ("p2pkh", "p2wpkh"), pkh, lambda k, s: [s(2), k.pub(0)])
    add("single key, wrong key", ("p2pkh", "p2wpkh"), pkh, lambda k, s: [s(2), k.pub(2)])
    add("single key, junk", ("p2pkh", "p2wpkh"), pkh, lambda k, s: [JUNK, k.pub(0)])
    add("single key, uncompressed", ("p2pkh", "p2wpkh"),
        lambda k: pkh_code(k.pub(0, False)), lambda k, s: [s(0), k.pub(0, False)])
    # a hash time lock, on both branches
    # Not a miniscript: Faraday does not run it, whatever the engine decides.
    add("lock, hash branch", W3, lock_script, lambda k, s: [s(0), PREIMAGE, b"\x01"],
        unrun=True)
    add("lock, time branch", W3, lock_script, lambda k, s: [s(1), b""], seq=144, unrun=True)
    add("lock, wrong preimage", W3, lock_script,
        lambda k, s: [s(0), bytes([8]) * 32, b"\x01"], unrun=True)
    add("lock, A's signature on the time branch", W3, lock_script,
        lambda k, s: [s(0), b""], seq=144, unrun=True)
    add("lock, sequence below the lock", W3, lock_script,
        lambda k, s: [s(1), b""], seq=143, unrun=True)
    # a redeem script over 520 bytes
    add("p2sh puzzle of 500", ("p2sh",), lambda k: puzzle(500), lambda k, s: [],
        unrun=True)
    add("p2sh puzzle of 517", ("p2sh",), lambda k: puzzle(517), lambda k, s: [])
    # standards Faraday keeps beyond consensus: a high S and a sighash byte no standard defines
    std = "a high S"
    add("single key, high S", ("p2pkh", "p2wpkh"), pkh,
        lambda k, s: [s(0, 1, True), k.pub(0)], diff=std)
    add("2-of-2, high S", W4, two, lambda k, s: [b"", s(0, 1, True), s(1)], diff=std)
    add("pk, high S", W3 + ("bare",), pk, lambda k, s: [s(0, 1, True)], diff=std)
    add("2-of-2, high S twice", W4, two, lambda k, s: [b"", s(0, 1, True), s(1, 1, True)],
        diff=std)
    odd = "a sighash byte no standard defines"
    for ht in (0x05, 0x00, 0x04, 0x84):
        add(f"single key, sighash byte {ht:#04x}", ("p2pkh", "p2wpkh"), pkh,
            lambda k, s, ht=ht: [s(0, ht), k.pub(0)], diff=odd)
        add(f"2-of-2, sighash byte {ht:#04x}", W4, two,
            lambda k, s, ht=ht: [b"", s(0, ht), s(1)], diff=odd)
    # time locks the interpreter reads in full: `after` needs this input's sequence not
    # final (BIP-65), `older` a transaction of version 2 or more (BIP-112)
    after = lambda k: cs(k.pub(0), S.OP_CHECKSIGVERIFY, 100, S.OP_CHECKLOCKTIMEVERIFY)  # noqa: E731
    older = lambda k: cs(k.pub(0), S.OP_CHECKSIGVERIFY, 10, S.OP_CHECKSEQUENCEVERIFY)  # noqa: E731
    wl = ("p2wsh", "p2sh")
    add("after, lock time met, sequence final", wl, after, lambda k, s: [s(0)],
        seq=0xFFFFFFFF, lock=100)
    add("after, lock time met, sequence not final", wl, after, lambda k, s: [s(0)],
        seq=0xFFFFFFFE, lock=100)
    add("after, lock time not met", wl, after, lambda k, s: [s(0)], seq=0xFFFFFFFE, lock=99)
    add("older, transaction version 1", wl, older, lambda k, s: [s(0)], seq=10, version=1)
    add("older, transaction version 2", wl, older, lambda k, s: [s(0)], seq=10, version=2)
    add("older, sequence below", wl, older, lambda k, s: [s(0)], seq=9, version=2)
    # the lock of a branch not taken is not counted
    ori_after = lambda k: cs(S.OP_IF, k.pub(0), S.OP_CHECKSIGVERIFY, 100,  # noqa: E731
                             S.OP_CHECKLOCKTIMEVERIFY, S.OP_ELSE, k.pub(1), S.OP_CHECKSIG,
                             S.OP_ENDIF)
    add("or_i, the branch without the lock, sequence final", wl, ori_after,
        lambda k, s: [s(1), b""], seq=0xFFFFFFFF)
    add("or_i, the branch with the lock, sequence final", wl, ori_after,
        lambda k, s: [s(0), b"\x01"], seq=0xFFFFFFFF, lock=100)
    # What no wallet writes and no node relays, which consensus accepts and Faraday refuses.
    pd1 = "a push by PUSHDATA1 where a direct push fits"
    add("p2sh 1-of-1, redeem script by PUSHDATA1", ("p2sh",), lambda k: ms(1, k.pub(0)),
        lambda k, s: [b"", s(0)], diff=pd1, pd1=True)
    add("p2sh pk, signature by PUSHDATA1", ("p2sh",), pk, lambda k, s: [Pd1(s(0))], diff=pd1)
    or_i = lambda k: cs(S.OP_IF, k.pub(0), S.OP_ELSE, k.pub(1), S.OP_ENDIF,  # noqa: E731
                        S.OP_CHECKSIG)
    add("p2sh or_i, selector OP_1", ("p2sh",), or_i, lambda k, s: [s(0), 1])
    add("p2sh or_i, selector 01 01", ("p2sh",), or_i, lambda k, s: [s(0), b"\x01"],
        diff="a selector that is not OP_1 or empty")
    add("p2wsh or_i, selector [1]", ("p2wsh",), or_i, lambda k, s: [s(0), b"\x01"])
    add("p2wsh or_i, selector [2]", ("p2wsh",), or_i, lambda k, s: [s(0), b"\x02"],
        diff="a selector that is not OP_1 or empty")
    or_d = lambda k: cs(k.pub(0), S.OP_CHECKSIG, S.OP_IFDUP, S.OP_NOTIF, k.pub(1),  # noqa: E731
                        S.OP_CHECKSIG, S.OP_ENDIF)
    add("or_d, dissatisfied by an empty element", ("p2wsh",), or_d,
        lambda k, s: [s(1), b""])
    add("or_d, dissatisfied by a wrong DER signature", ("p2wsh",), or_d,
        lambda k, s: [s(1), s(2)], diff="a failed signature check given a signature, not empty")
    add("p2sh pk, junk below", ("p2sh",), pk, lambda k, s: [JUNK, s(0)],
        diff="an element left on the stack")
    # a bare p2pk of a key off the curve is spent by nothing
    add("p2pk, key off the curve", ("bare",), lambda k: cs(b"\x02" + bytes(32), S.OP_CHECKSIG),
        lambda k, s: [s(0)], unrun=True)
    return out


# ----- a final taproot input, judged by btclib's script engine with TAPROOT ----------------

def xonly(k, i):
    return mul(k.ds[i]).x().to_bytes(32, "big")


def tweaked(d, merkle=b""):
    """(internal key, the secret of the output key, the output key's x, its
    y parity) of internal secret `d` under `merkle`."""
    internal = mul(d).x().to_bytes(32, "big")
    even = d if mul(d).y() % 2 == 0 else N - d
    t = int.from_bytes(tagged("TapTweak", internal + merkle), "big")
    out = mul((even + t) % N)
    return internal, (even + t) % N, out.x().to_bytes(32, "big"), out.y() & 1


def key_path(ht=0, forged=False, annex=None, annex_signed=True, single_missing=False):
    def build(k, digest):
        internal, dd, out, _ = tweaked(k.ds[0])
        spk = b"\x51\x20" + out
        if single_missing:
            # BIP-341 hashes no output the input has no index for: no digest exists
            msg = k.rng.randbytes(32)
        else:
            msg = digest(spk, ht, 0, annex if annex_signed and annex else b"", b"")
        d = k.ds[2] if forged else dd
        sig = schnorr_sign(d, msg, k.rng.randrange(1, N)) + (bytes([ht]) if ht else b"")
        return spk, [sig] + ([annex] if annex else [])
    return build


ANNEX = b"\x50" + bytes([0xA5]) * 7


def script_path(script_of, stack_of, version=0xC0, annex=None, annex_signed=True, fake=False):
    """A script path: the output key commits to the one leaf (or, `fake`, to another
    leaf than the one the witness shows); `stack_of(k, sign, script)` gives the
    elements below the script, `sign(i, ht=0, forged=False, pos=0xffffffff)` signing
    with key `i` of the case."""
    def build(k, digest):
        script = script_of(k)
        committed = cs(xonly(k, 7), S.OP_CHECKSIG) if fake else script
        internal, _, out, parity = tweaked(k.ds[19], leaf_hash(version, committed))
        spk = b"\x51\x20" + out
        control = bytes([version | parity]) + internal
        own = leaf_hash(version, script)

        def sign(i, ht=0, forged=False, pos=0xFFFFFFFF):
            msg = digest(spk, ht, 1, annex if annex_signed and annex else b"",
                         own + b"\x00" + pos.to_bytes(4, "little"))
            d = k.ds[18] if forged else k.ds[i]
            return schnorr_sign(d, msg, k.rng.randrange(1, N)) + (bytes([ht]) if ht else b"")

        stack = stack_of(k, sign, script)
        return spk, stack + [script, control] + ([annex] if annex else [])
    return build


def tap_rows():
    out = []
    add = lambda name, build, **kw: out.append(  # noqa: E731
        row(name, ("p2tr",), build, None, **kw))
    pk = lambda k: cs(xonly(k, 0), S.OP_CHECKSIG)  # noqa: E731
    multi_a = lambda k: cs(xonly(k, 0), S.OP_CHECKSIG, xonly(k, 1), S.OP_CHECKSIGADD,  # noqa: E731
                           xonly(k, 2), S.OP_CHECKSIGADD, 2, S.OP_NUMEQUAL)
    ab = lambda k, s, c: [b"", s(1), s(0)]  # noqa: E731
    # key path
    add("taproot key path, true", key_path())
    add("taproot key path, true, SIGHASH_ALL", key_path(ht=1))
    add("taproot key path, true, ALL|ANYONECANPAY", key_path(ht=0x81))
    add("taproot key path, forged", key_path(forged=True))
    add("taproot key path, forged, SIGHASH_ALL", key_path(ht=1, forged=True))
    add("taproot key path, SIGHASH_SINGLE with no output of its index",
        key_path(ht=3, single_missing=True))
    add("taproot key path, annex signed", key_path(annex=ANNEX))
    add("taproot key path, annex not signed", key_path(annex=ANNEX, annex_signed=False))
    add("taproot key path, annex signed, forged", key_path(annex=ANNEX, forged=True))
    # script path on a Tap miniscript leaf
    one = lambda k, s, c: [s(0)]  # noqa: E731
    add("taproot script path, pk, true", script_path(pk, one))
    add("taproot script path, pk, true, SIGHASH_ALL", script_path(pk, lambda k, s, c: [s(0, 1)]))
    add("taproot script path, pk, forged", script_path(pk, lambda k, s, c: [s(0, forged=True)]))
    add("taproot script path, pk, empty signature", script_path(pk, lambda k, s, c: [b""]))
    add("taproot script path, pk, a control block that does not commit to the output key",
        script_path(pk, one, fake=True))
    add("taproot script path, pk, annex signed", script_path(pk, one, annex=ANNEX))
    add("taproot script path, pk, annex not signed",
        script_path(pk, one, annex=ANNEX, annex_signed=False))
    add("taproot script path, multi_a, A and B", script_path(multi_a, ab))
    add("taproot script path, multi_a, B and C",
        script_path(multi_a, lambda k, s, c: [s(2), s(1), b""]))
    add("taproot script path, multi_a, one",
        script_path(multi_a, lambda k, s, c: [b"", b"", s(0)]))
    add("taproot script path, multi_a, one forged",
        script_path(multi_a, lambda k, s, c: [b"", s(1), s(0, forged=True)]))
    add("taproot script path, multi_a, annex signed", script_path(multi_a, ab, annex=ANNEX))
    add("taproot script path, multi_a, annex not signed",
        script_path(multi_a, ab, annex=ANNEX, annex_signed=False))
    # a 65-byte element ending 0x00 is no signature BIP-341 takes
    add("taproot script path, pk, 65 bytes ending 00",
        script_path(pk, lambda k, s, c: [s(0) + b"\x00"]))
    add("taproot script path, multi_a, 65 bytes ending 00 in a slot",
        script_path(multi_a, lambda k, s, c: [s(2) + b"\x00", s(1), s(0)]))
    # leaves Faraday does not run, whatever the engine decides
    add("taproot future leaf version, junk",
        script_path(pk, lambda k, s, c: [JUNK], version=0xC2), unrun=True)
    add("taproot future leaf version, a signature", script_path(pk, one, version=0xC2),
        unrun=True)
    hashlock = lambda k: cs(S.OP_SHA256, hashlib.sha256(PREIMAGE).digest(),  # noqa: E731
                            S.OP_EQUALVERIFY, xonly(k, 0), S.OP_CHECKSIG)
    add("taproot hash lock, true", script_path(hashlock, lambda k, s, c: [s(0), PREIMAGE]),
        unrun=True)
    add("taproot hash lock, wrong preimage",
        script_path(hashlock, lambda k, s, c: [s(0), bytes([8]) * 32]), unrun=True)
    add("taproot hash lock, forged",
        script_path(hashlock, lambda k, s, c: [s(0, forged=True), PREIMAGE]), unrun=True)
    # OP_CODESEPARATOR is the first opcode: a signature commits to position 0
    sep = lambda k: cs(S.OP_CODESEPARATOR, xonly(k, 0), S.OP_CHECKSIG)  # noqa: E731
    add("taproot OP_CODESEPARATOR, true", script_path(sep, lambda k, s, c: [s(0, pos=0)]),
        unrun=True)
    add("taproot OP_CODESEPARATOR, signed as if none ran",
        script_path(sep, lambda k, s, c: [s(0)]), unrun=True)
    add("taproot OP_CODESEPARATOR, forged",
        script_path(sep, lambda k, s, c: [s(0, forged=True, pos=0)]), unrun=True)
    drop = lambda k: cs(S.OP_DROP, xonly(k, 0), S.OP_CHECKSIG)  # noqa: E731
    add("taproot OP_DROP, true", script_path(drop, lambda k, s, c: [s(0), JUNK]), unrun=True)
    add("taproot OP_DROP, forged",
        script_path(drop, lambda k, s, c: [s(0, forged=True), JUNK]), unrun=True)
    return out


ROWS = input_rows()
ALL_ROWS = ROWS + tap_rows()


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
        "every complete input and, under consensus's rules alone, every final input of the "
        "table (`CONSENSUS`); Bitcoin Core v31.1's `finalizepsbt` and `testmempoolaccept` "
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
        "key under fresh nonces; and, for the verdict on a final input, a table of "
        "about ninety rows, each in every wrapping its script allows: `and_v`, `thresh`, "
        "`pk`, `pkh`, an `or_d` recovery path with `older`, 2 of 2 over [A,A], 2 of 3 over "
        "[A,A,B] and [A,B,A], the dummy empty and not, junk, repeated, swapped and extra "
        "signatures, 2 of 17 and 17 of 20, uncompressed keys, bare P2PK and multisig, P2PKH "
        "and P2WPKH true and forged, a hash time lock on both branches and three false spends of it, a redeem script of "
        "500 and of 517 bytes, a high S and sighash bytes no standard defines, with keys "
        "drawn for each case; and about thirty taproot rows, run under TAPROOT: a key path "
        "and a script path on a Tap miniscript leaf, true and forged, an annex signed and "
        "not, SIGHASH_SINGLE with no output of its index, a control block that does not "
        "commit to the output key, a 65-byte element ending 0x00, and the leaves not run "
        "(another leaf version, a hash lock, OP_CODESEPARATOR, OP_DROP)"
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
            "A final input whose script Faraday does not run (a hash time lock, which is no "
            "miniscript: a wrong preimage, the key of the other branch, a sequence below "
            "the lock, and its true spends; a P2SH puzzle; a segwit `pk` of an "
            "uncompressed key, which Faraday's segwit reading refuses, though a segwit "
            "multisig of such keys is read by hand and answers `ok`; a bare P2PK of a key "
            "off the curve; a taproot leaf of a version other than 0xc0, a taproot hash lock, a "
            "taproot leaf with OP_CODESEPARATOR or OP_DROP): Faraday answers "
            "`unjudged`, whatever the engine decides. Such a script reads not sound in the "
            "nonce check and is not refused, and only these rows may answer so: a "
            "miniscript or a multisig answering `unjudged` fails the area. Counted."
        ),
        (
            "A legacy (bare or P2SH) multisig with a junk element below the dummy: "
            "consensus leaves it on the stack and accepts, Faraday refuses it. No wallet "
            "writes one, and the refusal stays. Counted."
        ),
        (
            "What no wallet writes and no node relays, which consensus accepts: a P2SH "
            "redeem script or signature pushed by PUSHDATA1 where a direct push fits; a "
            "P2SH `or_i` selector `01 01` (a push of one byte, not OP_1); a P2WSH `or_i` "
            "selector `[2]` (not `[1]` or empty); an `or_d` dissatisfied by a wrong DER "
            "signature, not an empty element; an element left below a P2SH `pk`. Faraday's "
            "reading of the final fields refuses each, and the refusal stays. Counted."
        ),
        (
            "A final input with a high S, or with a sighash byte no standard defines "
            "(0x00, 0x04, 0x05, 0x84), in a signature the script engine accepts under "
            "consensus's rules: Faraday refuses it (a mismatch, or an invalid signature) "
            "and consensus takes it. Faraday keeps to the standard here on purpose: a "
            "node relays neither, so a transaction carrying one does not reach a miner. "
            "Counted."
        ),
    ]
    not_compared = (
        "A PSBT with no previous output for an input (Faraday leaves the signature "
        "unchecked); signatures under a `pkh` leaf or a script that names no key; MuSig2 "
        "partial signatures (area `musig`); the order of the "
        "signatures within a reuse report"
    )
    CORE = True
    FUND = {"corpus": 3}
    FUND_DEPTH = 0
    LOOPS = (("corpus", 400), ("input", 300))

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

    # ----- a final input of any script ---------------------------------------------------------

    def case_input(self):
        """Row `i` of the table, in each wrapping it names, with keys drawn for the
        case: btclib's script engine, under consensus's rules, says whether the
        input spends its output, and Faraday's verdict on the final fields
        (`witness_mismatch`, else `final_invalid`) must refuse exactly then not."""
        name, wrappers, script_of, stack_of, seq, diff, unrun, tx = ALL_ROWS[self.case_at[1] % len(ALL_ROWS)]
        keys = Keys(self.rng)
        if wrappers == ("p2tr",):
            fail = self.tap_row(name, script_of, keys, seq, diff, unrun)
            if fail:
                self.bad("sign-check.input", fail[0], fail[2], fail[3], fail[1])
            return
        script = script_of(keys)
        fails = [self.input_row(f"{name} in {wrapper}", wrapper, script, keys, stack_of, seq, diff,
                                unrun is True or (unrun and wrapper in unrun), tx)
                 for wrapper in wrappers]
        fails = [f for f in fails if f]
        if fails:
            given, text = fails[0][0], "; ".join(f[1] for f in fails)
            self.bad("sign-check.input", given, fails[0][2], fails[0][3], text)

    def input_row(self, what, wrapper, script, keys, stack_of, seq, diff, unrun, tx_opts):
        value = 10_000
        segwit = wrapper in ("p2wpkh", "p2wsh", "p2sh-p2wsh")
        wsh = p2wsh(script)
        spk = {"bare": script, "p2sh": cs(S.OP_HASH160, hash160(script), S.OP_EQUAL),
               "p2wsh": wsh, "p2sh-p2wsh": cs(S.OP_HASH160, hash160(wsh), S.OP_EQUAL),
               # the script of a single key is its p2pkh code, which holds the key's hash
               "p2pkh": p2pkh(script[3:23]), "p2wpkh": p2wpkh(script[3:23])}[wrapper]
        txid = int.from_bytes(self.randbytes(32), "big")
        version, lock = tx_opts.get("version", 2), tx_opts.get("lock", 0)
        tx = CTransaction()
        tx.version, tx.nLockTime = version, lock
        tx.vin = [CTxIn(COutPoint(txid, 0), b"", seq)]
        tx.vout = [CTxOut(value - 1000, p2wpkh(self.randbytes(20)))]
        unsigned = BtclibTx.parse(tx.serialize_without_witness(), check_validity=False)

        def digest(ht):
            if segwit:
                return btclib_sig_hash.segwit_v0(script, unsigned, 0, ht, value)
            return btclib_sig_hash.legacy(script, unsigned, 0, ht)

        stack = stack_of(keys, lambda i, ht=1, high=False: keys.sign(i, digest(ht), ht, high))
        script_sig = witness = None
        if wrapper in ("bare", "p2pkh"):
            script_sig, stack = b"".join(push(x) for x in stack), []
        elif wrapper == "p2sh":
            redeem = Pd1(script) if tx_opts.get("pd1") else script
            script_sig, stack = b"".join(push(x) for x in stack + [redeem]), []
        elif wrapper == "p2sh-p2wsh":
            script_sig, stack = push(wsh), stack + [script]
        elif wrapper == "p2wsh":
            script_sig, stack = b"", stack + [script]
        else:
            script_sig = b""
        final = CTransaction()
        final.version, final.nLockTime = version, lock
        final.vin = [CTxIn(COutPoint(txid, 0), script_sig, seq)]
        final.vout = tx.vout
        w = CTxInWitness()
        w.scriptWitness.stack = stack
        final.wit.vtxinwit = [w]
        engine_tx = BtclibTx.parse(final.serialize(), check_validity=False)
        try:
            verify_input([BtclibTxOut(value, spk, check_validity=False)], engine_tx, 0, CONSENSUS)
            passes = True
        except Exception:  # noqa: BLE001 - every refusal of the script engine
            passes = False
        fields = [(b"\x01", CTxOut(value, spk).serialize())]
        if script_sig:
            fields.append((b"\x07", script_sig))
        if stack:
            fields.append((b"\x08", witness_bytes(stack)))
        raw = MAGIC + write_map([(b"\x00", tx.serialize_without_witness())]) + write_map(fields) + \
            b"\0" * len(tx.vout)
        given = {"psbt": raw.hex(), "network": "mainnet", "index": 0}
        return self.verdict_of(what, given, passes, diff, unrun,
                               f"spent output {spk.hex()}, scriptSig {script_sig.hex()}, "
                               f"witness {[x.hex() for x in stack]}")

    def verdict_of(self, what, given, passes, diff, unrun, shown):
        """Faraday's verdict on the final fields in `given` against whether the
        engine spends them: None when they agree or differ as stated."""
        got = self.rust.call("sign-check.input", **given)
        self.compared += 1
        if got[0] != "ok":
            return (given, f"{what}: Faraday refuses to read it: {got[1]}", got[1], passes)
        verdict = got[1]["verdict"]
        if verdict == "unjudged":
            # A script Faraday does not run reads not sound and is not refused, whatever
            # the engine decides: only the rows of such scripts may answer so.
            if unrun:
                self.known += 1
                return None
            return (given, f"{what}: Faraday answers unjudged for a script it should run",
                    verdict, "spends" if passes else "refused by btclib")
        if unrun:
            return (given, f"{what}: Faraday ({verdict}) reads a script it does not run",
                    verdict, "spends" if passes else "refused by btclib")
        if (verdict == "ok") == passes:
            return None
        if verdict != "ok" and diff:
            self.known += 1
            return None
        say = "takes an input the script engine refuses" if passes is False else \
            "refuses an input the script engine accepts"
        return (given, f"{what}: Faraday ({verdict}) {say}: {shown}",
                verdict, "spends" if passes else "refused by btclib")

    def tap_row(self, what, build, keys, seq, diff, unrun):
        """A taproot input: two inputs, the second spent, so a SIGHASH_SINGLE has
        no output of its index; `build(keys, digest)` gives the spent output
        script and the witness stack, signed over btclib's signature hash."""
        value0, value = 7_000, 10_000
        spk0 = p2wpkh(self.randbytes(20))
        tx = CTransaction()
        tx.version, tx.nLockTime = 2, 0
        tx.vin = [CTxIn(COutPoint(int.from_bytes(self.randbytes(32), "big"), 0), b"", seq)
                  for _ in range(2)]
        tx.vout = [CTxOut(value - 1000, p2wpkh(self.randbytes(20)))]
        unsigned = BtclibTx.parse(tx.serialize_without_witness(), check_validity=False)

        def spent(spk):
            return [BtclibTxOut(value0, spk0, check_validity=False),
                    BtclibTxOut(value, spk, check_validity=False)]

        def digest(spk, ht, ext_flag, annex, ext):
            return btclib_sig_hash.taproot(unsigned, 1, spent(spk), ht, ext_flag, annex, ext)

        spk, stack = build(keys, digest)
        final = CTransaction()
        final.version, final.nLockTime = 2, 0
        final.vin = tx.vin
        final.vout = tx.vout
        w = CTxInWitness()
        w.scriptWitness.stack = stack
        final.wit.vtxinwit = [CTxInWitness(), w]
        engine_tx = BtclibTx.parse(final.serialize(), check_validity=False)
        try:
            verify_input(spent(spk), engine_tx, 1, CONSENSUS_TAP)
            passes = True
        except Exception:  # noqa: BLE001 - every refusal of the script engine
            passes = False
        raw = MAGIC + write_map([(b"\x00", tx.serialize_without_witness())]) + \
            write_map([(b"\x01", CTxOut(value0, spk0).serialize())]) + \
            write_map([(b"\x01", CTxOut(value, spk).serialize()),
                       (b"\x08", witness_bytes(stack))]) + b"\0" * len(tx.vout)
        given = {"psbt": raw.hex(), "network": "mainnet", "index": 1}
        return self.verdict_of(what, given, passes, diff, unrun,
                               f"spent output {spk.hex()}, witness {[x.hex() for x in stack]}")

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
        report = sorted((x["kind"], sorted(x["keys"]), x["nonce"], sorted(x["inputs"]))
                        for x in counted[1]["reused"])
        wanted = [tuple(x) for x in wanted]
        self.compared += 1
        if counted[1]["count"] != len(wanted) or report != wanted:
            raise Mismatch(self.name, "sign-check.reuse", given, counted,
                           ("ok", {"count": len(wanted), "reused": wanted}),
                           "the report of a reused nonce differs")
        if mode in ("reuse", "negated", "other-key") and not wanted:
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
