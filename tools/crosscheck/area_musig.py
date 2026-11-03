"""MuSig2 (BIP-327) against the BIP's reference implementation.

Every published vector file, then a random corpus of whole signing
sessions: keys, tweaks, nonces, partial signatures, their verification
(including tampered ones) and the aggregate signature, which must also
be a valid BIP-340 signature under the tweaked aggregate key.
"""

import base64
import functools
import hashlib
import re

import bip32_ref
import core_node
import lineage
from common import ROOT, Area, Mismatch, as_verify, reference, use_reference, vectors

use_reference("bip327")
import reference as R  # noqa: E402

use_reference("core")
from test_framework.address import address_to_scriptpubkey  # noqa: E402
from test_framework.descriptors import descsum_create  # noqa: E402

# The reference multiplies the same point by the same number many times
# over in a session (each signer's key, the aggregate key again for every
# partial signature and its check), and spends nearly all its time there.
# `point_mul` is a pure function of a point, a tuple of two integers or
# None, and an integer, and its answer a tuple or None, nothing the
# caller can change: kept answers are the answers it would compute again.
# The reference's own functions call it by its module name, so they see
# this one; its file is unchanged. About three times faster.
_point_mul = R.point_mul
_kept_point_mul = functools.lru_cache(maxsize=1 << 14)(_point_mul)


def point_mul(P, n):
    if (P is None or type(P) is tuple) and type(n) is int:
        return _kept_point_mul(P, n)
    return _point_mul(P, n)


R.point_mul = point_mul

# btclib's MuSig2 (`ecc.ecc.musig2`) on its own curve: `lineage` has
# switched its libsecp256k1 off, and its partial signature check, the one
# step it can hand to the bindings, then runs its Python.
from ecc.curves import secp256k1  # noqa: E402
from ecc.curves.sec_point import bytes_from_point  # noqa: E402
from ecc.ecc import musig2 as B  # noqa: E402

# btclib-wallet's BIP-373: the Updater, Signer and Finalizer over B.
from btclib_wallet.psbt import musig2 as BP  # noqa: E402
from btclib_wallet.psbt.psbt import Psbt  # noqa: E402


def h(b):
    return b.hex()


def tweak_list(tweaks, xonly):
    return [[h(t), x] for t, x in zip(tweaks, xonly)]


def keyagg_answer(ctx):
    return {"q": h(R.cbytes(ctx.Q)), "gacc": R.bytes_from_int(ctx.gacc).hex(),
            "tacc": R.bytes_from_int(ctx.tacc).hex()}


_PREFIXES = {b"\x04\x88\xb2\x1e": b"\x04\x35\x87\xcf", b"\x04\x88\xad\xe4": b"\x04\x35\x83\x94"}


def on_regtest(descriptor):
    """The descriptor with its mainnet keys written for regtest (xpub to
    tpub, xprv to tprv, WIF to its test form), which Core's regtest node
    reads; the scripts do not depend on the version bytes."""
    def one(m):
        raw = bip32_ref.b58check_decode(m.group(0))
        if raw[:4] in _PREFIXES:
            raw = _PREFIXES[raw[:4]] + raw[4:]
        elif raw[:1] == b"\x80":
            raw = b"\xef" + raw[1:]
        return bip32_ref.b58check_encode(raw)
    return re.sub(r"(?<![0-9A-Za-z])(?:[xX](?:pub|prv)[1-9A-HJ-NP-Za-km-z]{100,111}|[5KL][1-9A-HJ-NP-Za-km-z]{50,51})(?![0-9A-Za-z])",
                  one, descriptor)


def b_key_agg(pubkeys, tweaks, xonly):
    ctx = B.key_agg_and_tweak_var(pubkeys, tweaks, xonly)
    return {"q": h(bytes_from_point(ctx.Q, secp256k1)), "gacc": h(ctx.gacc.to_bytes(32, "big")),
            "tacc": h(ctx.tacc.to_bytes(32, "big"))}


class Musig(Area):
    name = "musig"
    CORE = True
    faraday = (
        '`faraday_bip::musig`'
    )
    reference = (
        "BIP-327's `reference.py` (`bip327/`); btclib's MuSig2 (`btclib/`, its own curve) in full, "
        "answering every operation beside it and verifying the aggregate signature as BIP-340; "
        "btclib-wallet's BIP-373 roles over the Core round trip's PSBTs; Bitcoin Core v31.1 "
        "(`deriveaddresses`) for `musig()` descriptors"
    )
    reference_checked = (
        'its vendored files are hashed against `references.sha256` before any run, and '
        "a published vector's answer is required of the reference as well as of Faraday"
    )
    vectors = (
        'all eight files of `tools/vectors/bip327/`; the valid and invalid descriptors of '
        '`tools/vectors/bip390/` (Core)'
    )
    corpus = (
        'sessions of 1 to 3 signers: key sort and aggregation, tweaks, nonces, partial '
        'signatures, their verification (and a tampered one), aggregation checked as '
        'BIP-340, deterministic signing; `tr(musig(…))` wallets of 2 to 4 keys, as xpubs on '
        'both chains or as plain keys, whose scripts Core pays too'
    )
    differences = [
        (
            'none'
        ),
    ]
    not_compared = (
        'none'
    )

    def compare(self, op, inputs, ref_fn, expected=None, normalize=None, btc_fn=None):
        """The two-way comparison of `Area.compare`, and btclib's answer
        to the same inputs, which must be Faraday's too."""
        raw = super().compare(op, inputs, ref_fn, expected, normalize)
        if btc_fn is not None:
            btc = reference(btc_fn)
            rust = normalize(raw) if normalize else raw
            btc = normalize(btc) if normalize else btc
            if rust[0] != btc[0] or (rust[0] == "ok" and rust[1] != btc[1]):
                raise Mismatch(self.name, op, inputs, rust, btc, "btclib's MuSig2 differs")
        return raw

    # -- the published vectors --------------------------------------

    def key_sort(self, pubkeys, expected=None):
        self.compare("musig.key_sort", {"pubkeys": [h(k) for k in pubkeys]},
                     lambda: {"pubkeys": [h(k) for k in R.key_sort(list(pubkeys))]}, expected,
                     btc_fn=lambda: {"pubkeys": [h(k) for k in B.key_sort(pubkeys)]})

    def key_agg(self, pubkeys, tweaks, xonly, expected=None):
        return self.compare(
            "musig.key_agg",
            {"pubkeys": [h(k) for k in pubkeys], "tweaks": tweak_list(tweaks, xonly)},
            lambda: keyagg_answer(R.key_agg_and_tweak(pubkeys, tweaks, xonly)), expected,
            btc_fn=lambda: b_key_agg(pubkeys, tweaks, xonly))

    def nonce_gen(self, rand, sk, pk, aggpk, msg, extra):
        def ref():
            sec, pub = R.nonce_gen_internal(rand, sk, pk, aggpk, msg, extra)
            return {"secnonce": h(bytes(sec)), "pubnonce": h(pub)}

        def btc():
            sec, pub = B.nonce_gen_(rand, sk, pk, aggpk, msg, extra)
            return {"secnonce": h(bytes(sec)), "pubnonce": h(pub)}
        opt = lambda b: None if b is None else h(b)  # noqa: E731
        return self.compare("musig.nonce_gen", {
            "rand": h(rand), "sk": opt(sk), "pk": h(pk), "aggpk": opt(aggpk),
            "msg": opt(msg), "extra_in": opt(extra)}, ref, btc_fn=btc)

    def nonce_agg(self, pubnonces, expected=None):
        return self.compare("musig.nonce_agg", {"pubnonces": [h(p) for p in pubnonces]},
                            lambda: {"aggnonce": h(R.nonce_agg(pubnonces))}, expected,
                            btc_fn=lambda: {"aggnonce": h(B.nonce_agg_var(pubnonces))})

    def sign(self, secnonce, sk, pubkeys, tweaks, xonly, aggnonce, msg, expected=None):
        def ref():
            ctx = R.SessionContext(aggnonce, pubkeys, tweaks, xonly, msg)
            return {"psig": h(R.sign(bytearray(secnonce), sk, ctx))}

        def btc():
            ctx = B.SessionContext(aggnonce, pubkeys, tweaks, xonly, msg)
            return {"psig": h(B.sign(bytearray(secnonce), sk, ctx))}
        return self.compare("musig.sign", {
            "secnonce": h(secnonce), "sk": h(sk), "pubkeys": [h(k) for k in pubkeys],
            "tweaks": tweak_list(tweaks, xonly), "aggnonce": h(aggnonce), "msg": h(msg)},
            ref, expected, btc_fn=btc)

    def verify(self, psig, pubnonces, pubkeys, tweaks, xonly, msg, i, expected=None):
        return self.compare("musig.verify", {
            "psig": h(psig), "pubnonces": [h(p) for p in pubnonces],
            "pubkeys": [h(k) for k in pubkeys], "tweaks": tweak_list(tweaks, xonly),
            "msg": h(msg), "i": i},
            lambda: {"valid": R.partial_sig_verify(psig, pubnonces, pubkeys, tweaks, xonly, msg, i)},
            expected, normalize=as_verify,
            btc_fn=lambda: {"valid": B.partial_sig_verify(psig, pubnonces, pubkeys, tweaks, xonly, msg, i)})

    def agg(self, psigs, pubkeys, tweaks, xonly, aggnonce, msg, expected=None):
        def ref():
            ctx = R.SessionContext(aggnonce, pubkeys, tweaks, xonly, msg)
            return {"sig": h(R.partial_sig_agg(psigs, ctx))}

        def btc():
            ctx = B.SessionContext(aggnonce, pubkeys, tweaks, xonly, msg)
            return {"sig": h(B.partial_sig_agg(psigs, ctx).serialize())}
        return self.compare("musig.agg", {
            "psigs": [h(p) for p in psigs], "pubkeys": [h(k) for k in pubkeys],
            "tweaks": tweak_list(tweaks, xonly), "aggnonce": h(aggnonce), "msg": h(msg)},
            ref, expected, btc_fn=btc)

    def det_sign(self, sk, other, pubkeys, tweaks, xonly, msg, rand, expected=None):
        def ref():
            pub, psig = R.deterministic_sign(sk, other, pubkeys, tweaks, xonly, msg, rand)
            return {"pubnonce": h(pub), "psig": h(psig)}

        def btc():
            pub, psig = B.deterministic_sign(sk, other, pubkeys, tweaks, xonly, msg, rand)
            return {"pubnonce": h(pub), "psig": h(psig)}
        return self.compare("musig.det_sign", {
            "sk": h(sk), "aggothernonce": h(other), "pubkeys": [h(k) for k in pubkeys],
            "tweaks": tweak_list(tweaks, xonly), "msg": h(msg),
            "rand": None if rand is None else h(rand)}, ref, expected, btc_fn=btc)

    def published(self):
        fx = lambda l: [bytes.fromhex(x) for x in l]  # noqa: E731
        ok = lambda v: ("ok", v)  # noqa: E731
        refused = ("err", None)

        v = vectors("bip327", "key_sort_vectors.json")
        self.key_sort(fx(v["pubkeys"]), ok({"pubkeys": [k.lower() for k in v["sorted_pubkeys"]]}))

        v = vectors("bip327", "key_agg_vectors.json")
        X, T = fx(v["pubkeys"]), fx(v["tweaks"])
        for c in v["valid_test_cases"]:
            ans = self.key_agg([X[i] for i in c["key_indices"]], [], [])
            if ans[1]["q"][2:] != c["expected"].lower():
                raise AssertionError(f"key_agg vector {c}: {ans}")
        for c in v["error_test_cases"]:
            self.key_agg([X[i] for i in c["key_indices"]], [T[i] for i in c["tweak_indices"]],
                         c["is_xonly"], refused)

        v = vectors("bip327", "nonce_gen_vectors.json")
        for c in v["test_cases"]:
            g = lambda k: None if c[k] is None else bytes.fromhex(c[k])  # noqa: E731
            ans = self.nonce_gen(g("rand_"), g("sk"), g("pk"), g("aggpk"), g("msg"), g("extra_in"))
            want = {"secnonce": c["expected_secnonce"].lower(), "pubnonce": c["expected_pubnonce"].lower()}
            if ans[1] != want:
                raise AssertionError(f"nonce_gen vector: {ans} != {want}")

        v = vectors("bip327", "nonce_agg_vectors.json")
        P = fx(v["pnonces"])
        for c in v["valid_test_cases"]:
            self.nonce_agg([P[i] for i in c["pnonce_indices"]], ok({"aggnonce": c["expected"].lower()}))
        for c in v["error_test_cases"]:
            self.nonce_agg([P[i] for i in c["pnonce_indices"]], refused)

        self.published_sign_verify(fx, ok, refused)
        self.published_tweak_det_agg(fx, ok, refused)

    def published_sign_verify(self, fx, ok, refused):
        v = vectors("bip327", "sign_verify_vectors.json")
        sk, X, S, P, A, M = (bytes.fromhex(v["sk"]), fx(v["pubkeys"]), fx(v["secnonces"]),
                             fx(v["pnonces"]), fx(v["aggnonces"]), fx(v["msgs"]))
        for c in v["valid_test_cases"]:
            keys = [X[i] for i in c["key_indices"]]
            nonces = [P[i] for i in c["nonce_indices"]]
            msg = M[c["msg_index"]]
            want = c["expected"].lower()
            self.sign(S[0], sk, keys, [], [], A[c["aggnonce_index"]], msg, ok({"psig": want}))
            self.verify(bytes.fromhex(want), nonces, keys, [], [], msg, c["signer_index"],
                        ok({"valid": True}))
        for c in v["sign_error_test_cases"]:
            self.sign(S[c["secnonce_index"]], sk, [X[i] for i in c["key_indices"]], [], [],
                      A[c["aggnonce_index"]], M[c["msg_index"]], refused)
        for c in v["verify_fail_test_cases"] + v["verify_error_test_cases"]:
            self.verify(bytes.fromhex(c["sig"]), [P[i] for i in c["nonce_indices"]],
                        [X[i] for i in c["key_indices"]], [], [], M[c["msg_index"]],
                        c["signer_index"], ok({"valid": False}))

    def published_tweak_det_agg(self, fx, ok, refused):
        v = vectors("bip327", "tweak_vectors.json")
        sk, X, P, T = bytes.fromhex(v["sk"]), fx(v["pubkeys"]), fx(v["pnonces"]), fx(v["tweaks"])
        secnonce, aggnonce, msg = bytes.fromhex(v["secnonce"]), bytes.fromhex(v["aggnonce"]), bytes.fromhex(v["msg"])
        for c in v["valid_test_cases"] + v["error_test_cases"]:
            keys = [X[i] for i in c["key_indices"]]
            tw = [T[i] for i in c["tweak_indices"]]
            want = ok({"psig": c["expected"].lower()}) if "expected" in c else refused
            self.sign(secnonce, sk, keys, tw, c["is_xonly"], aggnonce, msg, want)
            if "expected" in c:
                self.verify(bytes.fromhex(c["expected"]), [P[i] for i in c["nonce_indices"]], keys,
                            tw, c["is_xonly"], msg, c["signer_index"], ok({"valid": True}))

        v = vectors("bip327", "det_sign_vectors.json")
        sk, X, M = bytes.fromhex(v["sk"]), fx(v["pubkeys"]), fx(v["msgs"])
        for c in v["valid_test_cases"] + v["error_test_cases"]:
            rand = None if c["rand"] is None else bytes.fromhex(c["rand"])
            want = (ok({"pubnonce": c["expected"][0].lower(), "psig": c["expected"][1].lower()})
                    if "expected" in c else refused)
            self.det_sign(sk, bytes.fromhex(c["aggothernonce"]), [X[i] for i in c["key_indices"]],
                          fx(c["tweaks"]), c["is_xonly"], M[c["msg_index"]], rand, want)

        v = vectors("bip327", "sig_agg_vectors.json")
        X, P, T, S = fx(v["pubkeys"]), fx(v["pnonces"]), fx(v["tweaks"]), fx(v["psigs"])
        msg = bytes.fromhex(v["msg"])
        for c in v["valid_test_cases"] + v["error_test_cases"]:
            nonces = [P[i] for i in c["nonce_indices"]]
            aggnonce = bytes.fromhex(c["aggnonce"]) if "aggnonce" in c else R.nonce_agg(nonces)
            want = ok({"sig": c["expected"].lower()}) if "expected" in c else refused
            self.agg([S[i] for i in c["psig_indices"]], [X[i] for i in c["key_indices"]],
                     [T[i] for i in c["tweak_indices"]], c["is_xonly"], aggnonce, msg, want)

    # -- the random corpus --------------------------------------------

    def random_scalar(self):
        while True:
            k = int.from_bytes(self.randbytes(32), "big")
            if 0 < k < R.n:
                return R.bytes_from_int(k)

    def session(self):
        rng = self.rng
        n = rng.choice([1, 2, 2, 3])
        sks = [self.random_scalar() for _ in range(n)]
        pks = [R.individual_pk(sk) for sk in sks]
        if rng.random() < 0.2 and n > 1:
            pks[1], sks[1] = pks[0], sks[0]  # a key listed twice
        if rng.random() < 0.5:
            self.key_sort(pks)
        tweaks = [self.random_scalar() for _ in range(rng.choice([0, 1, 1, 2]))]
        xonly = [rng.random() < 0.5 for _ in tweaks]
        self.key_agg(pks, tweaks, xonly)
        msg = self.randbytes(rng.choice([32, 32, 0, 1, rng.randint(0, 100)]))
        ctx = R.key_agg_and_tweak(pks, tweaks, xonly)
        aggpk = R.get_xonly_pk(ctx)
        secs, pubs = [], []
        for sk, pk in zip(sks, pks):
            pick = lambda v: v if rng.random() < 0.5 else None  # noqa: E731
            ans = self.nonce_gen(self.randbytes(32), pick(sk), pk, pick(aggpk), pick(msg),
                                 pick(self.randbytes(rng.randint(0, 40))))
            secs.append(bytes.fromhex(ans[1]["secnonce"]))
            pubs.append(bytes.fromhex(ans[1]["pubnonce"]))
        aggnonce = bytes.fromhex(self.nonce_agg(pubs)[1]["aggnonce"])
        psigs = []
        for i, sk in enumerate(sks):
            psig = bytes.fromhex(self.need(self.sign(secs[i], sk, pks, tweaks, xonly, aggnonce, msg), "sign")["psig"])
            psigs.append(psig)
            self.verify(psig, pubs, pks, tweaks, xonly, msg, i, ("ok", {"valid": True}))
        j = rng.randrange(n)
        bad = bytearray(psigs[j])
        bad[rng.randrange(32)] ^= 1 << rng.randrange(8)
        self.verify(bytes(bad), pubs, pks, tweaks, xonly, msg, j)
        sig = bytes.fromhex(self.agg(psigs, pks, tweaks, xonly, aggnonce, msg)[1]["sig"])
        # The reference's BIP-340 check takes 32-byte messages only.
        if len(msg) == 32 and not R.schnorr_verify(msg, aggpk, sig):
            raise AssertionError("the aggregate signature does not verify under BIP-340")
        # btclib, its own curve: BIP-340 verifies the aggregate too.
        lineage.check_schnorr_verify(msg, aggpk, sig)
        if n > 1:
            others = R.nonce_agg(pubs[1:])
            rand = self.randbytes(32) if rng.random() < 0.5 else None
            self.det_sign(sks[0], others, pks, tweaks, xonly, msg, rand)

    LOOPS = (("session", 20), ("descriptor", 12))

    # -- BIP-390's musig() descriptors, judged by Bitcoin Core ---------

    def core_scripts(self, descriptor, span=None):
        """Costs one RPC: the scripts Core's `deriveaddresses` pays at
        `span` (first, last) of the descriptor, or at its one place; for a
        multipath descriptor, the receive chain's and then the change
        chain's."""
        call = ("deriveaddresses", descsum_create(descriptor)) + (() if span is None else (span,))
        (got,) = core_node.answers([call])
        if got[0] == "err":
            return got
        chains = got[1] if got[1] and isinstance(got[1][0], list) else [got[1]]
        return ("ok", [[address_to_scriptpubkey(a).hex() for a in chain] for chain in chains])

    def bip390(self):
        """Core against the BIP's own vectors: every valid descriptor pays
        the scripts the BIP lists, every invalid one is refused."""
        text = (ROOT / "tools/vectors/bip390/descriptors.txt").read_text()
        cases = []
        for block in text.split("\n\n"):
            lines = [l for l in block.splitlines() if l and not l.startswith("#")]
            if not lines:
                continue
            if lines[0].startswith("invalid "):
                # Each `invalid` line is a case of its own, whatever block it sits in.
                cases += [(l.partition(" ")[2], "invalid", []) for l in lines if l.startswith("invalid ")]
            else:
                kind, _, descriptor = lines[0].partition(" ")
                cases.append((descriptor, kind, lines[1:]))
        refused = 0
        for descriptor, kind, scripts in cases:
            ranged = descriptor.endswith("*)") or "/*" in descriptor
            test = on_regtest(descriptor)
            # A refusal must be the BIP's reason, not a mainnet key on the regtest node.
            if re.search(r"[xX](?:pub|prv)[1-9A-HJ-NP-Za-km-z]{100,111}", test):
                raise AssertionError(f"a key of {descriptor!r} was not rewritten for regtest")
            got = self.core_scripts(test, (0, 2) if ranged else None)
            self.compared += 1
            if kind == "invalid":
                if got[0] != "err":
                    raise Mismatch(self.name, "musig.bip390", {"descriptor": descriptor},
                                   ("err", None), got, "Core accepts a descriptor the BIP refuses")
                refused += 1
            elif got != ("ok", [scripts]):
                raise Mismatch(self.name, "musig.bip390", {"descriptor": descriptor},
                               ("ok", scripts), got, "Core pays other scripts than the BIP's")
        if refused != sum(1 for l in text.splitlines() if l.startswith("invalid ")):
            raise AssertionError("not every invalid descriptor of the BIP was sent to Core")

    def descriptor_case(self):
        """A random `tr(musig(…))` wallet: Faraday's script and Core's
        `deriveaddresses` at the same place, on both chains."""
        rng = self.rng
        n = rng.choice([2, 2, 3, 4])
        if rng.random() < 0.25:
            keys = [h(R.individual_pk(self.random_scalar())) for _ in range(n)]
            descriptor, span = f"tr(musig({','.join(keys)}))", None
        else:
            keys = [bip32_ref.master(self.randbytes(32), test=True).derive(
                [bip32_ref.HARDENED + rng.randrange(0, 3)]).neuter().serialize()
                for _ in range(n)]
            descriptor, span = f"tr(musig({','.join(keys)})/<0;1>/*)", (idx := rng.randrange(0, 4), idx)
        got = self.core_scripts(descriptor, span)
        for change in (False, True) if span else (False,):
            ours = self.rust.call("musig.descriptor", text=descriptor, change=change,
                                  index=span[0] if span else 0)
            theirs = ("err", got[1]) if got[0] == "err" else ("ok", {"script": got[1][int(change)][0]})
            self.compared += 1
            if ours[0] != theirs[0] or (ours[0] == "ok" and ours[1] != theirs[1]):
                raise Mismatch(self.name, "musig.descriptor", {"descriptor": descriptor,
                               "change": change}, ours, theirs, "Bitcoin Core pays another script")

    # -- BIP-373's roles, judged by btclib-wallet ----------------------

    def bip373(self):
        """The MuSig2 PSBTs of the Bitcoin Core round trip
        (`tools/vectors/psbt/wallet-musig-*.psbt`, the device's side
        written by Faraday), read by btclib-wallet's BIP-373 roles:
        its Updater files each fixture's participants under the same
        aggregate key; its Signer verifies every partial signature
        against the nonce filed beside it and, for the device that signs
        last, makes the same public nonce and partial signature again;
        its Finalizer adds the device's and Core's partial signatures
        into the key-path signature the fixture carries."""
        def load(name):
            text = (ROOT / "tools/vectors/psbt" / f"{name}.psbt").read_text()
            return Psbt.b64decode(text)

        def need(ok, name, what, ours, theirs):
            self.compared += 1
            if not ok:
                raise Mismatch(self.name, "musig.bip373", {"fixture": name, "check": what},
                               ("ok", ours), ("ok", theirs), "btclib-wallet's BIP-373 differs")

        seed = hashlib.pbkdf2_hmac("sha512", " ".join(["abandon"] * 11 + ["about"]).encode(),
                                   b"mnemonic", 2048)
        account = bip32_ref.master(seed, test=True).derive(
            [86 + bip32_ref.HARDENED, 1 + bip32_ref.HARDENED, bip32_ref.HARDENED])
        sk = account.k.to_bytes(32, "big")
        mine = B.individual_pub_key(sk)
        names = ("wallet-musig-device-first", "wallet-musig-core-first",
                 "wallet-musig-core-first-signed", "wallet-musig-device-first-nonce",
                 "wallet-musig-device-first-core", "wallet-musig-device-first-signed")
        for name in names:
            psbt = load(name)
            psbt_in = psbt.inputs[0]
            (agg, keys), = psbt_in.musig2_participant_pub_keys.items()
            # Updater
            psbt_in.musig2_participant_pub_keys = {}
            made = BP.add_participant_pub_keys(psbt_in, keys)
            need(made == agg and psbt_in.musig2_participant_pub_keys == {agg: keys},
                 name, "updater", made.hex(), agg.hex())
            # Signer: every partial signature holds against the nonce beside it
            for key_data, psig in psbt_in.musig2_partial_sigs.items():
                ok = BP.partial_sig_verify(psbt, 0, key_data[:33], agg)
                need(ok, name, f"partial signature of {key_data[:4].hex()}", ok, True)

        # The device signs last: its public nonce and partial signature again.
        name = "wallet-musig-core-first-signed"
        psbt = load(name)
        psbt_in = psbt.inputs[0]
        (agg, _), = psbt_in.musig2_participant_pub_keys.items()
        session = BP.session_context(psbt, 0, agg).context
        core_nonce, = (v for k, v in psbt_in.musig2_pub_nonces.items() if k[:33] != mine)
        (mine_key, device_nonce), = ((k, v) for k, v in psbt_in.musig2_pub_nonces.items()
                                     if k[:33] == mine)
        nonce, psig = B.deterministic_sign(
            sk, B.nonce_agg_var([core_nonce]), session.pub_keys, session.tweaks,
            session.is_xonly, session.msg)
        need(nonce == device_nonce, name, "public nonce", nonce.hex(), device_nonce.hex())
        need(psig == psbt_in.musig2_partial_sigs[mine_key], name, "partial signature",
             psig.hex(), psbt_in.musig2_partial_sigs[mine_key].hex())

        # The Finalizer: both partial signatures into the key-path signature.
        name = "wallet-musig-device-first-signed"
        psbt = load(name)
        psbt_in = psbt.inputs[0]
        (agg, _), = psbt_in.musig2_participant_pub_keys.items()
        want = psbt_in.taproot_key_spend_signature
        sig = BP.partial_sigs_agg(psbt, 0, agg).serialize()
        need(sig == want, name, "finalizer", sig.hex(), want.hex())

    def prelude(self):
        self.published()
        self.bip373()
        self.bip390()

    def case_session(self):
        self.session()

    def case_descriptor(self):
        self.descriptor_case()
