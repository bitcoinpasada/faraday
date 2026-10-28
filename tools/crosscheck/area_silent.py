"""Silent payments (BIP-352) against the BIP's reference implementation.

Every case of the BIP's send-and-receive vectors from both sides, then
random transactions: inputs of every kind the BIP reads (P2PKH, P2SH-
P2WPKH, P2WPKH, taproot key path, taproot script path with and without
the NUMS key, a future SegWit version), random receivers with labels,
the outputs a sender makes for them and what each receiver then finds
among them and decoys.

What a receiver finds is compared output by output, in the order found,
with the label each was paid to. The reference reports a found output's
key and spending tweak; the label is read back from the tweak with the
reference's own label and shared-secret arithmetic.

Faraday's sender arithmetic is test-only (Faraday makes no silent
payment) and has no per-group limit, so the vector that exceeds K_max
on the sending side is checked on the receiving side only.
"""

from common import Area, use_reference, vectors

use_reference("bip352")
use_reference("bip352", "secp256k1lab", "src")
import bitcoin_utils as U  # noqa: E402
import reference as R  # noqa: E402
from secp256k1lab.secp256k1 import G, GE, Scalar  # noqa: E402

NUMS = R.NUMS_H.to_bytes(32, "big")


def vin(outpoint, script_sig, witness, prevout, key=None):
    w = U.CTxInWitness()
    w.scriptWitness.stack = list(witness)
    return U.VinInfo(outpoint=outpoint, scriptSig=script_sig, txinwitness=w, prevout=prevout,
                     private_key=key)


def wire(op, script_sig, witness, prevout):
    return {"outpoint": op.serialize().hex(), "script_sig": script_sig.hex(),
            "witness": [w.hex() for w in witness], "prevout": prevout.hex()}


def future_segwit(spk):
    """BIP-141's witness program of version 2 to 16 (OP_2 to OP_16 and
    one push of 2 to 40 bytes)."""
    return 4 <= len(spk) <= 42 and 0x52 <= spk[0] <= 0x60 and spk[1] == len(spk) - 2


def ref_input_hash(inputs):
    # BIP-352 skips a transaction that spends a SegWit version above 1;
    # the reference's input reading leaves that rule to its caller.
    if any(future_segwit(i[3]) for i in inputs):
        raise ValueError("BIP-352: a transaction spending a SegWit version above 1 is skipped")
    keys = [k for k in (R.get_pubkey_from_input(vin(*i)) for i in inputs) if not k.infinity]
    if not keys:
        raise ValueError("no eligible input")
    total = GE.sum(*keys)
    if total.infinity:
        raise ValueError("the input keys sum to infinity")
    h = R.get_input_hash([i[0] for i in inputs], total)
    Scalar.from_bytes_checked(h)
    return total, h


def ref_scan(inputs, b_scan, b_spend_pk, outputs, labels):
    total, h = ref_input_hash(inputs)
    tweaks = {(R.generate_label(b_scan, m) * G).to_bytes_compressed().hex():
              R.generate_label(b_scan, m).to_bytes().hex() for m in labels}
    found = R.scanning(b_scan, b_spend_pk, total, h, list(outputs), tweaks)
    # The label each output was paid to, from its spending tweak.
    shared = Scalar.from_bytes_checked(h) * b_scan * total
    by_tweak = {int(R.generate_label(b_scan, m)): m for m in labels}
    out = []
    for k, f in enumerate(found):
        t_k = Scalar.from_bytes_checked(R.tagged_hash("BIP0352/SharedSecret",
                                                      shared.to_bytes_compressed() + U.ser_uint32(k)))
        tweak = Scalar.from_bytes_checked(bytes.fromhex(f["priv_key_tweak"]))
        label = None if tweak == t_k else by_tweak[int(tweak - t_k)]
        out.append({"index": outputs.index(bytes.fromhex(f["pub_key"])), "key": f["pub_key"],
                    "label": label})
    return {"found": out}


def sorted_outputs(answer):
    if answer[0] == "ok":
        return ("ok", {"outputs": sorted(answer[1]["outputs"])})
    return answer


class Silent(Area):
    name = "sp"
    faraday = (
        '`faraday_bip::silent`'
    )
    reference = (
        "BIP-352's `reference.py` (`bip352/`)"
    )
    reference_checked = (
        'its vendored files are hashed against `references.sha256` before any run, and '
        "a published vector's answer is required of the reference as well as of Faraday"
    )
    vectors = (
        'every case of `tools/vectors/bip352/`, both sides'
    )
    corpus = (
        "inputs of every kind BIP-352 reads, receivers with labels, a sender's outputs "
        'and what each receiver finds among them and decoys'
    )
    differences = [
        (
            "BIP-352: Faraday's sender arithmetic is test-only and has no K_max, so the "
            'vector that exceeds it is checked on the receiving side only.'
        ),
        (
            "Where a reference leaves a rule to its caller, the area writes the BIP's own "
            "sentence and says so in its docstring: BIP-352's skip of a transaction "
            'spending SegWit version 2 or above.'
        ),
    ]
    not_compared = (
        'none'
    )

    def address(self, b_scan, b_spend, mainnet, label, expected=None):
        hrp = "sp" if mainnet else "tsp"
        def ref():
            if label is None:
                return {"address": R.encode_silent_payment_address(b_scan * G, b_spend * G, hrp)}
            return {"address": R.create_labeled_silent_payment_address(b_scan, b_spend * G, label, hrp)}
        return self.compare("sp.address", {"scan_sk": b_scan.to_bytes().hex(),
                                           "spend_pk": (b_spend * G).to_bytes_compressed().hex(),
                                           "mainnet": mainnet, "label": label}, ref, expected)

    def decode(self, address):
        def ref():
            hrp = "sp" if address.lower().startswith("sp") else "tsp"
            scan, spend = R.decode_silent_payment_address(address.lower(), hrp)
            return {"mainnet": hrp == "sp", "scan": scan.to_bytes_compressed().hex(),
                    "spend": spend.to_bytes_compressed().hex()}
        return self.compare("sp.decode", {"address": address}, ref)

    def input_hash(self, inputs):
        def ref():
            total, h = ref_input_hash(inputs)
            return {"sum": total.to_bytes_compressed().hex(), "hash": h.hex()}
        return self.compare("sp.input_hash", {"inputs": [wire(*i) for i in inputs]}, ref)

    def sender(self, keys, outpoints, scan, spends):
        def ref():
            recipients = [{"address": R.encode_silent_payment_address(scan, s, "sp"),
                           "scan_pub_key": scan.to_bytes_compressed().hex(),
                           "spend_pub_key": s.to_bytes_compressed().hex()} for s in spends]
            out = R.create_outputs([(Scalar.from_bytes_checked(k), t) for k, t in keys], outpoints,
                                   recipients, hrp="sp")
            if not out:
                raise ValueError("the reference makes no outputs")
            return {"outputs": out}
        return self.compare("sp.sender", {
            "keys": [[k.hex(), t] for k, t in keys], "outpoints": [o.serialize().hex() for o in outpoints],
            "scan": scan.to_bytes_compressed().hex(),
            "spends": [s.to_bytes_compressed().hex() for s in spends]}, ref, normalize=sorted_outputs)

    def scan(self, inputs, b_scan, b_spend_pk, outputs, labels, expected=None):
        return self.compare("sp.scan", {
            "inputs": [wire(*i) for i in inputs], "scan_sk": b_scan.to_bytes().hex(),
            "spend_pk": b_spend_pk.to_bytes_compressed().hex(), "outputs": [o.hex() for o in outputs],
            "labels": labels}, lambda: ref_scan(inputs, b_scan, b_spend_pk, outputs, labels), expected)

    # -- the published vectors ----------------------------------------

    def given_inputs(self, given):
        out = []
        for i in given["vin"]:
            witness = U.CTxInWitness().deserialize(U.from_hex(i["txinwitness"])).scriptWitness.stack
            out.append((U.COutPoint(U.deser_txid(i["txid"]), i["vout"]), bytes.fromhex(i["scriptSig"]),
                        witness, bytes.fromhex(i["prevout"]["scriptPubKey"]["hex"]),
                        bytes.fromhex(i["private_key"]) if "private_key" in i else None))
        return out

    def published(self):
        for case in vectors("bip352", "send_and_receive_test_vectors.json"):
            for test in case["sending"]:
                self.published_send(test)
            for test in case["receiving"]:
                self.published_receive(test)

    def published_send(self, test):
        given, expected = test["given"], test["expected"]
        inputs = self.given_inputs(given)
        plain = [i[:4] for i in inputs]
        self.input_hash(plain)
        keys = [(i[4], U.is_p2tr(i[3])) for i in inputs
                if not R.get_pubkey_from_input(vin(*i[:4])).infinity]
        if not keys or any(r.get("count", 1) > R.K_max for r in given["recipients"]):
            return
        groups = {}
        for r in given["recipients"]:
            scan, spend = R.decode_silent_payment_address(r["address"], "sp")
            groups.setdefault(scan.to_bytes_compressed(), []).extend([spend] * r.get("count", 1))
        made = []
        for scan, spends in groups.items():
            ans = self.sender(keys, [i[0] for i in inputs], GE.from_bytes_compressed(scan), spends)
            if ans[0] == "ok":
                made.extend(ans[1]["outputs"])
        if not any(sorted(made) == sorted(e) for e in expected["outputs"]):
            raise AssertionError(f"sending vector: {sorted(made)} is none of {expected['outputs']}")

    def published_receive(self, test):
        given, expected = test["given"], test["expected"]
        inputs = [i[:4] for i in self.given_inputs(given)]
        b_scan = Scalar.from_bytes_checked(bytes.fromhex(given["key_material"]["scan_priv_key"]))
        b_spend = Scalar.from_bytes_checked(bytes.fromhex(given["key_material"]["spend_priv_key"]))
        addresses = [self.address(b_scan, b_spend, True, None)[1]["address"]]
        for m in given["labels"]:
            addresses.append(self.address(b_scan, b_spend, True, m)[1]["address"])
        if addresses != expected["addresses"]:
            raise AssertionError(f"receiving vector addresses {addresses}")
        outputs = [bytes.fromhex(o) for o in given["outputs"]]
        ans = self.scan(inputs, b_scan, b_spend * G, outputs, given["labels"])
        found = ans[1]["found"] if ans[0] == "ok" else []
        if "outputs" in expected:
            if sorted(f["key"] for f in found) != sorted(o["pub_key"] for o in expected["outputs"]):
                raise AssertionError(f"receiving vector: found {found}")
        elif len(found) != expected["n_outputs"]:
            raise AssertionError(f"receiving vector: found {len(found)}, not {expected['n_outputs']}")

    # -- the random corpus --------------------------------------------

    def scalar(self):
        while True:
            k = int.from_bytes(self.randbytes(32), "big")
            if 0 < k < GE.ORDER:
                return Scalar(k)

    def random_input(self, key):
        """An input spent by `key`, of a random kind; whether its key
        counts is the reference's business."""
        rng = self.rng
        op = U.COutPoint(self.randbytes(32), rng.choice([0, 1, rng.randrange(2**32)]))
        pub = key * G
        pc = pub.to_bytes_compressed()
        sig = self.randbytes(rng.choice([71, 72]))
        kind = rng.choice(["p2pkh", "p2sh-p2wpkh", "p2wpkh", "p2tr", "p2tr", "p2tr-script", "nums"])
        if kind == "p2pkh":
            return (op, bytes([len(sig)]) + sig + b"\x21" + pc, [], b"\x76\xa9\x14" + U.hash160(pc) + b"\x88\xac")
        if kind == "p2sh-p2wpkh":
            redeem = b"\x00\x14" + U.hash160(pc)
            return (op, bytes([len(redeem)]) + redeem, [sig, pc], b"\xa9\x14" + U.hash160(redeem) + b"\x87")
        if kind == "p2wpkh":
            return (op, b"", [sig, pc], b"\x00\x14" + U.hash160(pc))
        spk = b"\x51\x20" + pub.to_bytes_xonly()
        if kind == "p2tr":
            witness = [self.randbytes(64)] + ([b"\x50" + self.randbytes(8)] if rng.random() < 0.2 else [])
            return (op, b"", witness, spk)
        internal = NUMS if kind == "nums" else self.randbytes(32)
        return (op, b"", [self.randbytes(64), b"\x51", b"\xc0" + internal], spk)

    def corpus_case(self):
        rng = self.rng
        keys = [self.scalar() for _ in range(rng.randint(1, 4))]
        inputs = [self.random_input(k) for k in keys]
        if rng.random() < 0.03:
            inputs.append((U.COutPoint(self.randbytes(32), 0), b"", [self.randbytes(64)],
                           b"\x52\x20" + self.randbytes(32)))
        self.input_hash(inputs)
        eligible = [(k, i) for k, i in zip(keys, inputs) if not R.get_pubkey_from_input(vin(*i)).infinity]
        receivers = []
        for _ in range(rng.randint(1, 3)):
            b_scan, b_spend = self.scalar(), self.scalar()
            labels = sorted({0} | {rng.randrange(1, 50) for _ in range(rng.randint(0, 2))}) \
                if rng.random() < 0.6 else []
            mainnet = rng.random() < 0.5
            receivers.append((b_scan, b_spend, labels))
            addr = self.address(b_scan, b_spend, mainnet, None)
            self.decode(addr[1]["address"])
            if labels:
                self.address(b_scan, b_spend, mainnet, rng.choice(labels))
        outputs = []
        if eligible:
            for b_scan, b_spend, labels in receivers:
                spends = []
                for _ in range(rng.randint(1, 3)):
                    m = rng.choice(labels + [None]) if labels else None
                    spends.append(b_spend * G if m is None else b_spend * G + R.generate_label(b_scan, m) * G)
                ans = self.sender([(k.to_bytes(), U.is_p2tr(i[3])) for k, i in eligible],
                                  [i[0] for i in inputs], b_scan * G, spends)
                if ans[0] == "ok":
                    outputs.extend(bytes.fromhex(o) for o in ans[1]["outputs"])
        paid = len(outputs)
        outputs.extend(o for o in (self.randbytes(32) for _ in range(rng.randint(0, 3))) if _on_curve(o))
        rng.shuffle(outputs)
        found = 0
        for b_scan, b_spend, labels in receivers:
            ans = self.scan(inputs, b_scan, b_spend * G, outputs, labels)
            found += len(ans[1]["found"]) if ans[0] == "ok" else 0
        # Every payment made is found by the receiver it was made for,
        # unless the transaction is one BIP-352 skips.
        if any(future_segwit(i[3]) for i in inputs):
            paid = 0
        if found != paid:
            raise AssertionError(f"{paid} outputs paid, {found} found")

    LOOPS = (("corpus", 100),)

    def prelude(self):
        self.published()

    def case_corpus(self):
        self.corpus_case()


def _on_curve(x):
    try:
        GE.from_bytes_xonly(x)
        return True
    except ValueError:
        return False
