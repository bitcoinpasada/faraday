"""Sending silent payments (BIP-374 proofs, BIP-375 PSBTs) against the BIPs'
own runnable Python.

BIP-374's `reference.py` makes and checks the proofs; BIP-375's validator
(`test_runner.py`, `validator/`) checks a whole PSBT: its structure, the
shares' proofs, the inputs' eligibility and every output script. Both are
vendored unchanged at one commit of the BIPs repository (`bip374/`,
`bip375/`), over `secp256k1lab`. Nothing here is Faraday's own model: the
proofs, the shares and the scripts expected are the references' arithmetic
on the inputs' keys, and the verdict on a PSBT is the validator's.

- Every row of BIP-374's two vector files, generation and verification,
  and every one of BIP-375's 43 vectors, judged by the validator and by
  Faraday.
- Random proofs (a secret, a point, the auxiliary bytes, a message or none,
  secp256k1's generator or another) made by both and compared byte for byte,
  and random and damaged proofs verified by both.
- Random PSBTs of version 2: one to four inputs of every kind BIP-375 reads
  (P2PKH, P2SH-P2WPKH, P2WPKH, taproot key path) and of kinds it does not
  (P2WSH, P2SH of another script, a taproot output under the NUMS key),
  one to three silent payment outputs over one or two scan keys, and plain
  outputs. Faraday adds the shares, proofs and scripts, in one call or in
  two by different signers; each share, each proof (byte for byte) and each
  script is compared with the reference's, and the validator must accept
  the PSBT. A global share made by one holder of every key is accepted by
  both.
- Each finished PSBT then damaged in one of eighteen ways (a changed bit in
  a share, a proof or a script, a share or a proof removed, two scripts
  swapped, a sighash, the modifiable flags, a spend key, a field's length,
  a key the input's script does not commit to, scripts in sorted order, ...):
  Faraday and the validator must give the same verdict, but for the two
  differences listed.
"""

import hashlib
import importlib.util
import json

from common import ROOT, Area, Mismatch, as_verify, use_reference

use_reference("bip375")
use_reference("bip375", "deps", "secp256k1lab", "src")
use_reference("bip352")
import test_runner as T  # noqa: E402
from bitcoin_utils import hash160  # noqa: E402
from deps.bitcoin_test.messages import COutPoint  # noqa: E402
from deps.bitcoin_test.psbt import PSBT, PSBTMap  # noqa: E402
from secp256k1lab.secp256k1 import G, GE  # noqa: E402
from secp256k1lab.util import tagged_hash  # noqa: E402
from validator.bip352_crypto import compute_silent_payment_output_script  # noqa: E402
from validator.inputs import collect_input_ecdh_and_pubkey  # noqa: E402
from validator.psbt_bip375 import BIP375PSBT  # noqa: E402


def _bip374():
    spec = importlib.util.spec_from_file_location(
        "bip374_reference", ROOT / "tools/reference/bip374/reference.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


R374 = _bip374()
N = GE.ORDER
NUMS = bytes.fromhex("50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0")
INFINITY = GE()
GEN = G.to_bytes_compressed().hex()

# PSBT field types (BIP-174, 370, 371, 375).
IN_NON_WITNESS, IN_WITNESS, IN_SIGHASH, IN_REDEEM, IN_BIP32 = 0x00, 0x01, 0x03, 0x04, 0x06
IN_PREV_TXID, IN_PREV_INDEX, IN_SEQUENCE = 0x0E, 0x0F, 0x10
IN_TAP_BIP32, IN_TAP_INTERNAL = 0x16, 0x17
IN_SHARE, IN_DLEQ = 0x1D, 0x1E
G_MODIFIABLE, G_SHARE, G_DLEQ = 0x06, 0x07, 0x08
OUT_AMOUNT, OUT_SCRIPT, OUT_SP_INFO, OUT_SP_LABEL = 0x03, 0x04, 0x09, 0x0A


def point(h):
    return INFINITY if h == "INFINITY" else GE.from_bytes(bytes.fromhex(h))


def le32(n):
    return n.to_bytes(4, "little")


def key_of(t, data=b""):
    """A map's key: an integer for a bare type, bytes with key data."""
    return t if not data else bytes([t]) + data


def fields(psbt_map, t):
    """Every (key data, value) of a type in a map, by the BIP's own reader."""
    return psbt_map.get_all_by_type(t)


def verdict(b64, checks=None):
    """The BIP-375 validator's verdict: True when its four checks pass. A
    PSBT it cannot read, or that makes it raise, is one it does not accept."""
    try:
        return T.validate_bip375_psbt(b64, checks)[0]
    except Exception:  # noqa: BLE001 - every failure to validate is a refusal
        return False


class SilentSend(Area):
    name = "sps"
    faraday = (
        '`faraday_psbt::silent`, `faraday_bip::dleq`'
    )
    reference = (
        "BIP-374's `reference.py` (`bip374/`) and BIP-375's validator and `test_runner.py` "
        '(`bip375/`), over `secp256k1lab`'
    )
    reference_checked = (
        'its vendored files are hashed against `references.sha256` before any run, the '
        "validator is required to give each of BIP-375's 43 vectors its published verdict "
        "and the reference to give each of BIP-374's rows its published answer"
    )
    vectors = (
        'both CSV files of `tools/vectors/bip374/` and `tools/vectors/bip375/bip375_test_vectors.json`'
    )
    corpus = (
        'proofs made and verified, damaged or not, under two generators; version 2 PSBTs of '
        'one to four inputs of every kind BIP-375 reads and some it does not, one to three '
        'silent payment outputs, shares added by one signer or two, a global share, and each '
        "finished PSBT damaged in one of eighteen ways, judged by the BIP's validator"
    )
    differences = [
        (
            "BIP-375: Faraday takes a key a PSBT states for a P2PKH, P2SH-P2WPKH or P2WPKH input "
            "only where the input's script commits to it (F89); the validator takes the first "
            "key stated, so a PSBT it accepts with an unbound key is refused by Faraday."
        ),
        (
            "BIP-375: Faraday accepts the scripts of a scan key's outputs numbered in sorted "
            "order of their spend keys, as the BIP's text says, and in output order, as its "
            "vectors do; the validator accepts output order only."
        ),
        (
            "BIP-375: Faraday refuses an input whose redeem script is not the one its P2SH "
            "script hashes, and one whose previous transaction is not the one spent; the "
            "validator reads both fields without checking them."
        ),
    ]
    not_compared = (
        'none'
    )

    # -- what each side says ------------------------------------------

    def judge(self, b64, expected=None, checks=None):
        """Faraday's and the validator's verdict on a PSBT, which must be
        the same, unless one of the two known differences explains it.
        `expected` is a published vector's verdict."""
        inputs = {"psbt": b64}
        only = bool(checks)
        rust = self.rust.call("sps.check", psbt=b64, inputs_only=only)
        valid = verdict(b64, checks)
        self.compared += 1
        if expected is not None and valid != expected:
            raise Mismatch(self.name, "sps.check", inputs, rust, {"valid": valid},
                           f"the validator differs from the published vector: {expected}")
        if (rust[0] == "ok") != valid:
            if not self.known_difference(rust, valid, b64):
                raise Mismatch(self.name, "sps.check", inputs, rust, {"valid": valid})
            self.known += 1
        elif valid and not only:
            self.check_report(b64, rust)
        return rust, valid

    def check_report(self, b64, rust):
        """What Faraday says of a PSBT both accept: which outputs are silent
        payments, whether each states its script, and the script it gives
        once the shares are all there, which must be the validator's own
        arithmetic's (`k` counted in output order)."""
        psbt = BIP375PSBT.from_base64(b64)
        outpoints = [COutPoint(int.from_bytes(m[IN_PREV_TXID], "little"),
                               int.from_bytes(m[IN_PREV_INDEX], "little")) for m in psbt.i]
        want, ks = [], {}
        for j, m in enumerate(psbt.o):
            if OUT_SP_INFO not in m:
                continue
            scan, spend = m[OUT_SP_INFO][:33], m[OUT_SP_INFO][33:]
            k = ks[scan] = ks.get(scan, -1) + 1
            want.append((j, k, OUT_SCRIPT in m, spend, scan))
        got = rust[1]["outputs"]
        # `k` is set only where the shares give the script.
        ok = [(o["index"], o["stated"]) for o in got] == [(w[0], w[2]) for w in want]
        ok = ok and all(o["k"] == w[1] for o, w in zip(got, want) if o["script"])
        ok = ok and rust[1]["complete"] == all(o["script"] for o in got)
        for o, (j, k, stated, spend, scan) in zip(got, want):
            if not ok or not o["script"]:
                continue
            share, summed = collect_input_ecdh_and_pubkey(psbt, scan)
            computed = compute_silent_payment_output_script(outpoints, summed, share, spend, k)
            ok = ok and o["script"] == computed.hex() and (
                not stated or psbt.o[j][OUT_SCRIPT] == computed)
        if not ok:
            raise Mismatch(self.name, "sps.check", {"psbt": b64}, rust,
                           {"outputs": [[w[0], w[2]] for w in want]},
                           "the report of a PSBT both accept is not what the validator's arithmetic gives")

    def known_difference(self, rust, valid, b64):
        if rust[0] == "err":
            return valid and any(why in rust[1] for why in (
                "no public key the PSBT states is the one the input's script",
                "the redeem script is not the one the input spends",
                "the previous transaction is not the one spent"))
        if valid:
            return False
        return self.in_sorted_order(b64)

    def in_sorted_order(self, b64):
        """Whether every stated silent payment script is the one BIP-352
        gives with `k` counted in the sorted order of the spend keys."""
        psbt = BIP375PSBT.from_base64(b64)
        outpoints = [COutPoint(int.from_bytes(m[IN_PREV_TXID], "little"),
                               int.from_bytes(m[IN_PREV_INDEX], "little")) for m in psbt.i]
        by_scan = {}
        for j, m in enumerate(psbt.o):
            if OUT_SP_INFO in m:
                by_scan.setdefault(m[OUT_SP_INFO][:33], []).append((m[OUT_SP_INFO][33:], j))
        stated = 0
        for scan, outs in by_scan.items():
            share, summed = collect_input_ecdh_and_pubkey(psbt, scan)
            if not share:
                return False
            for k, (spend, j) in enumerate(sorted(outs)):
                script = psbt.o[j].get(OUT_SCRIPT)
                if script:
                    stated += 1
                    if script != compute_silent_payment_output_script(
                            outpoints, summed, share, spend, k):
                        return False
        return stated > 0

    # -- BIP-374 --------------------------------------------------------

    def dleq_generate(self, g, a, b, r, m, expected=None):
        def ref():
            proof = R374.dleq_generate_proof(a, point(b), r, G=point(g), m=m)
            if proof is None:
                raise ValueError("BIP-374 gives no proof")
            return {"proof": proof.hex()}
        req = {"a": a.to_bytes(32, "big").hex(), "b": b, "r": r.hex(), "g": g}
        if m is not None:
            req["m"] = m.hex()
        return self.compare("sps.dleq_generate", req, ref, expected)

    def dleq_verify(self, g, a, b, c, proof, m, expected=None):
        def ref():
            return {"valid": R374.dleq_verify_proof(point(a), point(b), point(c), proof,
                                                    G=point(g), m=m)}
        req = {"g": g, "a": a, "b": b, "c": c, "proof": proof.hex()}
        if m is not None:
            req["m"] = m.hex()
        return self.compare("sps.dleq_verify", req, ref, expected, normalize=as_verify)

    def published_dleq(self):
        base = ROOT / "tools/vectors/bip374"
        rows = lambda f: [l.rstrip("\r").split(",") for l in
                          (base / f).read_text().splitlines()[1:] if l.strip()]
        for index, g, a, b, r, m, proof, comment in rows("test_vectors_generate_proof.csv"):
            expected = ("err", None) if proof == "INVALID" else ("ok", {"proof": proof.lower()})
            try:
                self.dleq_generate(g, int(a, 16), b, bytes.fromhex(r),
                                   bytes.fromhex(m) if m else None, expected)
            except OverflowError:  # a secret too large for 32 bytes: not a request
                pass
        for index, g, a, b, c, proof, m, result, comment in rows("test_vectors_verify_proof.csv"):
            self.dleq_verify(g, a, b, c, bytes.fromhex(proof), bytes.fromhex(m) if m else None,
                             ("ok", {"valid": result == "TRUE"}))

    def scalar(self):
        while True:
            k = int.from_bytes(self.randbytes(32), "big")
            if 0 < k < N:
                return k

    def random_point(self):
        return (self.scalar() * G).to_bytes_compressed().hex()

    def case_dleq(self):
        rng = self.rng
        a = self.scalar() if rng.random() < 0.95 else rng.choice([0, N])
        b = self.random_point() if rng.random() < 0.97 else "INFINITY"
        g = GEN if rng.random() < 0.7 else self.random_point()
        m = self.randbytes(32) if rng.random() < 0.5 else None
        r = self.randbytes(32)
        made = self.dleq_generate(g, a, b, r, m)
        if made[0] != "ok":
            return
        proof = bytes.fromhex(made[1]["proof"])
        gp, bp = point(g), point(b)
        a_pt, c_pt = a * gp, a * bp
        A, C = a_pt.to_bytes_compressed().hex(), c_pt.to_bytes_compressed().hex()
        shown = self.dleq_verify(g, A, b, C, proof, m)
        assert shown == ("ok", {"valid": True}), "a proof made does not verify"
        other = self.random_point()
        flipped = bytearray(proof)
        flipped[rng.randrange(64)] ^= 1 << rng.randrange(8)
        tries = [
            (g, A, b, C, bytes(flipped), m),
            (g, A, b, other, proof, m),
            (g, other, b, C, proof, m),
            (g, A, other, C, proof, m),
            (g, A, b, C, proof, self.randbytes(32) if m is None or rng.random() < 0.5 else None),
            (self.random_point() if g == GEN else GEN, A, b, C, proof, m),
            (g, C, b, A, proof, m),
            (g, A, b, C, N.to_bytes(32, "big") + proof[32:], m),
            (g, A, b, C, proof[:32] + N.to_bytes(32, "big"), m),
            (g, A, b, C, bytes(64), m),
        ]
        for t in rng.sample(tries, 5):
            self.dleq_verify(*t)

    # -- BIP-375 vectors ------------------------------------------------

    def published_psbts(self):
        data = ROOT.joinpath("tools/vectors/bip375/bip375_test_vectors.json").read_text()
        vectors = json.loads(data)
        for kind in ("invalid", "valid"):
            for v in vectors[kind]:
                self.judge(v["psbt"], kind == "valid", v.get("checks"))

    # -- random PSBTs ---------------------------------------------------

    def origin(self):
        return self.randbytes(4) + b"".join(
            le32(self.rng.randrange(2**32)) for _ in range(self.rng.randint(1, 4)))

    def input_of(self, kind):
        """An input of a kind, as a PSBT map, with what the reference needs to
        know of it: whether it counts, the key it gives, and the secret
        Faraday is given for it."""
        rng = self.rng
        a = self.scalar()
        pub = a * G
        pc = pub.to_bytes_compressed()
        amount = rng.randrange(1000, 10**8).to_bytes(8, "little")
        txid, index = self.randbytes(32), rng.randrange(4)
        out = {"kind": kind, "eligible": True, "secret": a, "key": a, "map": {
            IN_SEQUENCE: b"\xfe\xff\xff\xff"}}
        m = out["map"]
        if kind == "p2wpkh":
            m[IN_WITNESS] = amount + b"\x16\x00\x14" + hash160(pc)
            m[key_of(IN_BIP32, pc)] = self.origin()
        elif kind == "p2sh-p2wpkh":
            redeem = b"\x00\x14" + hash160(pc)
            m[IN_WITNESS] = amount + b"\x17\xa9\x14" + hash160(redeem) + b"\x87"
            m[IN_REDEEM] = redeem
            m[key_of(IN_BIP32, pc)] = self.origin()
        elif kind == "p2pkh":
            spk = b"\x76\xa9\x14" + hash160(pc) + b"\x88\xac"
            prev = (le32(2) + b"\x01" + self.randbytes(32) + le32(0) + b"\x00" + b"\xff" * 4
                    + b"\x01" + amount + bytes([len(spk)]) + spk + le32(0))
            m[IN_NON_WITNESS] = prev
            m[key_of(IN_BIP32, pc)] = self.origin()
            txid, index = hashlib.sha256(hashlib.sha256(prev).digest()).digest(), 0
        elif kind == "p2tr":
            d = a if pub.has_even_y() else N - a
            x = (d * G).to_bytes_xonly()
            sk = (d + int.from_bytes(tagged_hash("TapTweak", x), "big")) % N
            q = sk * G
            out["secret"], out["key"] = sk, sk if q.has_even_y() else N - sk
            m[IN_WITNESS] = amount + b"\x22\x51\x20" + q.to_bytes_xonly()
            m[IN_TAP_INTERNAL] = x
            if rng.random() < 0.5:
                m[key_of(IN_TAP_BIP32, x)] = b"\x00" + self.origin()
        elif kind == "nums":
            out["eligible"] = False
            m[IN_WITNESS] = amount + b"\x22\x51\x20" + pub.to_bytes_xonly()
            m[IN_TAP_INTERNAL] = NUMS
        elif kind == "p2wsh":
            out["eligible"] = False
            m[IN_WITNESS] = amount + b"\x22\x00\x20" + self.randbytes(32)
        else:  # p2sh of another script
            out["eligible"] = False
            m[IN_WITNESS] = amount + b"\x17\xa9\x14" + hash160(b"\x51") + b"\x87"
            m[IN_REDEEM] = b"\x51"
        m[IN_PREV_TXID], m[IN_PREV_INDEX] = txid, le32(index)
        out["outpoint"] = COutPoint(int.from_bytes(txid, "little"), index)
        return out

    def random_psbt(self):
        """A PSBT before any signer: its inputs (with their secrets), the
        outputs, and the scan keys the outputs name."""
        rng = self.rng
        kinds = ["p2wpkh", "p2sh-p2wpkh", "p2pkh", "p2tr", "p2tr", "p2wpkh"]
        while True:
            inputs = [self.input_of(rng.choice(kinds if rng.random() < 0.85 else
                                               ["nums", "p2wsh", "p2sh-other"]))
                      for _ in range(rng.randint(1, 4))]
            if any(i["eligible"] for i in inputs):
                break
        scans = [self.scalar() * G for _ in range(rng.randint(1, 2))]
        outs = []
        for _ in range(rng.randint(1, 3)):
            scan = rng.choice(scans)
            spend = self.scalar() * G
            m = {OUT_AMOUNT: rng.randrange(1000, 10**7).to_bytes(8, "little"),
                 key_of(OUT_SP_INFO): scan.to_bytes_compressed() + spend.to_bytes_compressed()}
            if rng.random() < 0.3:
                m[OUT_SP_LABEL] = le32(rng.randrange(2**32))
            outs.append(m)
        for _ in range(rng.randint(0, 2)):
            outs.append({OUT_AMOUNT: rng.randrange(1000, 10**7).to_bytes(8, "little"),
                         OUT_SCRIPT: b"\x00\x14" + self.randbytes(20)})
        rng.shuffle(outs)
        psbt = PSBT(g=PSBTMap({0xFB: le32(2), 2: le32(2), 4: bytes([len(inputs)]),
                               5: bytes([len(outs)]), G_MODIFIABLE: b"\x03"}),
                    i=[PSBTMap(dict(i["map"])) for i in inputs], o=[PSBTMap(o) for o in outs])
        psbt.version = 2
        return psbt, inputs

    def shares(self, b64, inputs, which, aux):
        keys = [[i, inputs[i]["secret"].to_bytes(32, "big").hex()] for i in which]
        return self.rust.call("sps.shares", psbt=b64, keys=keys, aux=aux.hex())

    def expect_signed(self, b64, answer, inputs, aux):
        """What the references make of the signed PSBT: each share, each proof
        and each script, which must be the ones in the PSBT and the ones
        Faraday reports."""
        psbt = BIP375PSBT.from_base64(b64)
        eligible = [(i, x) for i, x in enumerate(inputs) if x["eligible"]]
        by_scan, outpoints = {}, [x["outpoint"] for x in inputs]
        for m in psbt.o:
            if OUT_SP_INFO in m:
                by_scan.setdefault(m[OUT_SP_INFO][:33], [])
        for scan in by_scan:
            S = GE.from_bytes(scan)
            for i, x in eligible:
                share, proof = psbt.i[i].get_by_key(IN_SHARE, scan), psbt.i[i].get_by_key(IN_DLEQ, scan)
                want_share = (x["key"] * S).to_bytes_compressed()
                want_proof = R374.dleq_generate_proof(x["key"], S, aux)
                if share != want_share or proof != want_proof:
                    raise Mismatch(self.name, "sps.shares",
                                   {"input": i, "scan": scan.hex(), "aux": aux.hex()},
                                   {"share": share and share.hex(), "proof": proof and proof.hex()},
                                   {"share": want_share.hex(), "proof": want_proof.hex()})
        summed = {}
        for scan in by_scan:
            S = GE.from_bytes(scan)
            summed[scan] = (GE.sum(*[x["key"] * G for _, x in eligible]).to_bytes_compressed(),
                            GE.sum(*[x["key"] * S for _, x in eligible]).to_bytes_compressed())
        ks, reported = {}, {o["index"]: o for o in answer["outputs"]}
        for j, m in enumerate(psbt.o):
            if OUT_SP_INFO not in m:
                continue
            scan, spend = m[OUT_SP_INFO][:33], m[OUT_SP_INFO][33:]
            k = ks[scan] = ks.get(scan, -1) + 1
            want = compute_silent_payment_output_script(
                outpoints, summed[scan][0], summed[scan][1], spend, k)
            got = m.get(OUT_SCRIPT)
            if got != want or reported[j]["script"] != want.hex():
                raise Mismatch(self.name, "sps.shares",
                               {"output": j, "k": k, "scan": scan.hex()},
                               {"script": got and got.hex(), "reported": reported[j]["script"]},
                               {"script": want.hex()})
        return psbt

    def global_share(self, psbt, inputs, aux):
        """The same PSBT with one global share and proof per scan key, as one
        holder of every eligible key makes them, in place of the inputs'."""
        eligible = [x for x in inputs if x["eligible"]]
        total = sum(x["key"] for x in eligible) % N
        if total == 0:
            return None
        for m in psbt.i:
            for t in (IN_SHARE, IN_DLEQ):
                for kd, _ in fields(m, t):
                    del m.map[key_of(t, kd)]
        for scan in {m[OUT_SP_INFO][:33] for m in psbt.o if OUT_SP_INFO in m}:
            S = GE.from_bytes(scan)
            psbt.g.map[key_of(G_SHARE, scan)] = (total * S).to_bytes_compressed()
            psbt.g.map[key_of(G_DLEQ, scan)] = R374.dleq_generate_proof(total, S, aux)
        return psbt

    # -- the damage -----------------------------------------------------

    def mutations(self, psbt, inputs):
        """The ways a finished PSBT is damaged: each a function of the PSBT
        (parsed afresh each time) that returns whether it applied."""
        rng = self.rng

        def pick(maps, t):
            found = [(m, kd) for m in maps for kd, _ in fields(m, t)]
            return rng.choice(found) if found else None

        def flip(maps, t):
            def f(p):
                hit = pick(maps(p), t)
                if not hit:
                    return False
                m, kd = hit
                v = bytearray(m.map[key_of(t, kd)])
                v[rng.randrange(len(v))] ^= 1 << rng.randrange(8)
                m.map[key_of(t, kd)] = bytes(v)
                return True
            return f

        def drop(maps, t):
            def f(p):
                hit = pick(maps(p), t)
                if not hit:
                    return False
                del hit[0].map[key_of(t, hit[1])]
                return True
            return f

        def resize(maps, t):
            def f(p):
                hit = pick(maps(p), t)
                if not hit:
                    return False
                v = hit[0].map[key_of(t, hit[1])]
                hit[0].map[key_of(t, hit[1])] = v[:-1] if rng.random() < 0.5 else v + b"\x00"
                return True
            return f

        def sp_outputs(p):
            return [m for m in p.o if OUT_SP_INFO in m]

        def swap_scripts(p):
            same = [(m, n) for a, m in enumerate(sp_outputs(p)) for n in sp_outputs(p)[a + 1:]
                    if m[OUT_SP_INFO][:33] == n[OUT_SP_INFO][:33] and OUT_SCRIPT in m and OUT_SCRIPT in n]
            if not same:
                return False
            m, n = rng.choice(same)
            m.map[OUT_SCRIPT], n.map[OUT_SCRIPT] = n.map[OUT_SCRIPT], m.map[OUT_SCRIPT]
            return True

        def sighash(value):
            def f(p):
                m = rng.choice(p.i)
                m.map[IN_SIGHASH] = le32(value if value is not None else rng.choice([0, 2, 3, 0x81]))
                return True
            return f

        def modifiable(p):
            p.g.map[G_MODIFIABLE] = bytes([rng.choice([1, 2, 3])])
            return True

        def spend_key(p):
            outs = sp_outputs(p)
            if not outs:
                return False
            m = rng.choice(outs)
            m.map[OUT_SP_INFO] = m.map[OUT_SP_INFO][:33] + self.random_point_bytes()
            return True

        def unbind(p):
            """The input's script commits to another key than the one stated."""
            idx = [i for i, x in enumerate(inputs) if x["kind"] == "p2wpkh"]
            if not idx:
                return False
            m = p.i[rng.choice(idx)]
            amount = m.map[IN_WITNESS][:8]
            m.map[IN_WITNESS] = amount + b"\x16\x00\x14" + self.randbytes(20)
            return True

        def wrong_redeem(p):
            idx = [i for i, x in enumerate(inputs) if x["kind"] == "p2sh-p2wpkh"]
            if not idx:
                return False
            p.i[rng.choice(idx)].map[IN_REDEEM] = b"\x00\x14" + self.randbytes(20)
            return True

        def wrong_previous(p):
            idx = [i for i, x in enumerate(inputs) if x["kind"] == "p2pkh"]
            if not idx:
                return False
            m = p.i[rng.choice(idx)]
            prev = m.map[IN_NON_WITNESS]
            m.map[IN_NON_WITNESS] = prev[:5] + self.randbytes(32) + prev[37:]
            return True

        def sorted_scripts(p):
            outpoints = [x["outpoint"] for x in inputs]
            outs = [(j, m) for j, m in enumerate(p.o) if OUT_SP_INFO in m]
            for scan in {m[OUT_SP_INFO][:33] for _, m in outs}:
                share, summed = collect_input_ecdh_and_pubkey(p, scan)
                mine = sorted((m[OUT_SP_INFO][33:], j) for j, m in outs if m[OUT_SP_INFO][:33] == scan)
                for k, (spend, j) in enumerate(mine):
                    p.o[j].map[OUT_SCRIPT] = compute_silent_payment_output_script(
                        outpoints, summed, share, spend, k)
            return bool(outs)

        def unscript(p):
            outs = [m for m in sp_outputs(p) if OUT_SCRIPT in m]
            if not outs:
                return False
            del rng.choice(outs).map[OUT_SCRIPT]
            return True

        def nums_key(p):
            idx = [i for i, x in enumerate(inputs) if x["kind"] == "p2tr"]
            if not idx:
                return False
            p.i[rng.choice(idx)].map[IN_TAP_INTERNAL] = NUMS
            return True

        def future_segwit(p):
            m = p.i[rng.randrange(len(p.i))]
            m.map[IN_WITNESS] = m.map.get(IN_WITNESS, bytes(8) + b"\x00")[:8] \
                + b"\x22\x52\x20" + self.randbytes(32)
            return True

        def all_inputs(p):
            return p.i

        def globals_(p):
            return [p.g]

        return [
            flip(all_inputs, IN_SHARE), flip(all_inputs, IN_DLEQ),
            flip(lambda p: sp_outputs(p), OUT_SCRIPT), drop(all_inputs, IN_SHARE),
            drop(all_inputs, IN_DLEQ), swap_scripts, sighash(None), sighash(1), modifiable,
            spend_key, resize(all_inputs, IN_SHARE), resize(all_inputs, IN_DLEQ), unbind,
            sorted_scripts, unscript, nums_key, future_segwit, flip(globals_, G_SHARE),
            wrong_redeem, wrong_previous,
            resize(lambda p: sp_outputs(p), OUT_SP_INFO), resize(lambda p: sp_outputs(p), OUT_SP_LABEL),
        ]

    def random_point_bytes(self):
        return (self.scalar() * G).to_bytes_compressed()

    def case_psbt(self):
        rng = self.rng
        aux = self.randbytes(32)
        psbt, inputs = self.random_psbt()
        b64 = psbt.to_base64()
        self.judge(b64)  # before any signer: nothing is set, nothing is refused
        eligible = [i for i, x in enumerate(inputs) if x["eligible"]]
        two = len(eligible) > 1 and rng.random() < 0.4
        first = eligible if not two else rng.sample(eligible, rng.randint(1, len(eligible) - 1))
        step = self.need(self.shares(b64, inputs, first, aux), "the first signer")
        if two:
            self.judge(step["psbt"])  # partly signed: shares, and no script yet
            step = self.need(self.shares(step["psbt"], inputs,
                                         [i for i in eligible if i not in first], aux), "the second signer")
        if not step["complete"]:
            raise Mismatch(self.name, "sps.shares", {"eligible": eligible, "aux": aux.hex()},
                           {"complete": False}, {"complete": True},
                           "every eligible input is signed, so every script is set")
        signed = step["psbt"]
        parsed = self.expect_signed(signed, step, inputs, aux)
        rust, valid = self.judge(signed)
        if rust[0] != "ok" or not valid:
            raise AssertionError(f"a PSBT signed by Faraday is refused: {rust} {valid}")
        # The same PSBT with a global share, as one holder of every key makes it.
        glob = self.global_share(BIP375PSBT.from_base64(signed), inputs, aux)
        if glob is not None and rng.random() < 0.5:
            rust, valid = self.judge(glob.to_base64())
            if rust[0] != "ok" or not valid:
                raise AssertionError(f"a global share is refused: {rust} {valid}")
        damage = self.mutations(parsed, inputs)
        for f in rng.sample(damage, 6):
            damaged = BIP375PSBT.from_base64(signed)
            if f(damaged):
                self.judge(damaged.to_base64())

    LOOPS = (("psbt", 40), ("dleq", 40))

    def prelude(self):
        self.published_dleq()
        self.published_psbts()
