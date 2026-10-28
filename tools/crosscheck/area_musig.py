"""MuSig2 (BIP-327) against the BIP's reference implementation.

Every published vector file, then a random corpus of whole signing
sessions: keys, tweaks, nonces, partial signatures, their verification
(including tampered ones) and the aggregate signature, which must also
be a valid BIP-340 signature under the tweaked aggregate key.
"""

import functools

from common import Area, as_verify, use_reference, vectors

use_reference("bip327")
import reference as R  # noqa: E402

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


def h(b):
    return b.hex()


def tweak_list(tweaks, xonly):
    return [[h(t), x] for t, x in zip(tweaks, xonly)]


def keyagg_answer(ctx):
    return {"q": h(R.cbytes(ctx.Q)), "gacc": R.bytes_from_int(ctx.gacc).hex(),
            "tacc": R.bytes_from_int(ctx.tacc).hex()}


class Musig(Area):
    name = "musig"
    faraday = (
        '`faraday_bip::musig`'
    )
    reference = (
        "BIP-327's `reference.py` (`bip327/`)"
    )
    reference_checked = (
        'its vendored files are hashed against `references.sha256` before any run, and '
        "a published vector's answer is required of the reference as well as of Faraday"
    )
    vectors = (
        'all eight files of `tools/vectors/bip327/`'
    )
    corpus = (
        'sessions of 1 to 3 signers: key sort and aggregation, tweaks, nonces, partial '
        'signatures, their verification (and a tampered one), aggregation checked as '
        'BIP-340, deterministic signing'
    )
    differences = [
        (
            'none'
        ),
    ]
    not_compared = (
        'none'
    )

    # -- the published vectors --------------------------------------

    def key_sort(self, pubkeys, expected=None):
        self.compare("musig.key_sort", {"pubkeys": [h(k) for k in pubkeys]},
                     lambda: {"pubkeys": [h(k) for k in R.key_sort(list(pubkeys))]}, expected)

    def key_agg(self, pubkeys, tweaks, xonly, expected=None):
        return self.compare(
            "musig.key_agg",
            {"pubkeys": [h(k) for k in pubkeys], "tweaks": tweak_list(tweaks, xonly)},
            lambda: keyagg_answer(R.key_agg_and_tweak(pubkeys, tweaks, xonly)), expected)

    def nonce_gen(self, rand, sk, pk, aggpk, msg, extra):
        def ref():
            sec, pub = R.nonce_gen_internal(rand, sk, pk, aggpk, msg, extra)
            return {"secnonce": h(bytes(sec)), "pubnonce": h(pub)}
        opt = lambda b: None if b is None else h(b)  # noqa: E731
        return self.compare("musig.nonce_gen", {
            "rand": h(rand), "sk": opt(sk), "pk": h(pk), "aggpk": opt(aggpk),
            "msg": opt(msg), "extra_in": opt(extra)}, ref)

    def nonce_agg(self, pubnonces, expected=None):
        return self.compare("musig.nonce_agg", {"pubnonces": [h(p) for p in pubnonces]},
                            lambda: {"aggnonce": h(R.nonce_agg(pubnonces))}, expected)

    def sign(self, secnonce, sk, pubkeys, tweaks, xonly, aggnonce, msg, expected=None):
        def ref():
            ctx = R.SessionContext(aggnonce, pubkeys, tweaks, xonly, msg)
            return {"psig": h(R.sign(bytearray(secnonce), sk, ctx))}
        return self.compare("musig.sign", {
            "secnonce": h(secnonce), "sk": h(sk), "pubkeys": [h(k) for k in pubkeys],
            "tweaks": tweak_list(tweaks, xonly), "aggnonce": h(aggnonce), "msg": h(msg)},
            ref, expected)

    def verify(self, psig, pubnonces, pubkeys, tweaks, xonly, msg, i, expected=None):
        return self.compare("musig.verify", {
            "psig": h(psig), "pubnonces": [h(p) for p in pubnonces],
            "pubkeys": [h(k) for k in pubkeys], "tweaks": tweak_list(tweaks, xonly),
            "msg": h(msg), "i": i},
            lambda: {"valid": R.partial_sig_verify(psig, pubnonces, pubkeys, tweaks, xonly, msg, i)},
            expected, normalize=as_verify)

    def agg(self, psigs, pubkeys, tweaks, xonly, aggnonce, msg, expected=None):
        def ref():
            ctx = R.SessionContext(aggnonce, pubkeys, tweaks, xonly, msg)
            return {"sig": h(R.partial_sig_agg(psigs, ctx))}
        return self.compare("musig.agg", {
            "psigs": [h(p) for p in psigs], "pubkeys": [h(k) for k in pubkeys],
            "tweaks": tweak_list(tweaks, xonly), "aggnonce": h(aggnonce), "msg": h(msg)},
            ref, expected)

    def det_sign(self, sk, other, pubkeys, tweaks, xonly, msg, rand, expected=None):
        def ref():
            pub, psig = R.deterministic_sign(sk, other, pubkeys, tweaks, xonly, msg, rand)
            return {"pubnonce": h(pub), "psig": h(psig)}
        return self.compare("musig.det_sign", {
            "sk": h(sk), "aggothernonce": h(other), "pubkeys": [h(k) for k in pubkeys],
            "tweaks": tweak_list(tweaks, xonly), "msg": h(msg),
            "rand": None if rand is None else h(rand)}, ref, expected)

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
        if n > 1:
            others = R.nonce_agg(pubs[1:])
            rand = self.randbytes(32) if rng.random() < 0.5 else None
            self.det_sign(sks[0], others, pks, tweaks, xonly, msg, rand)

    LOOPS = (("session", 20),)

    def prelude(self):
        self.published()

    def case_session(self):
        self.session()
