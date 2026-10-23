"""FROST threshold signing (BIP 445) against the BIP's reference
implementation, and Faraday's trusted dealer against the reference's
public-share interpolation.

The reference has no dealer. Faraday's `deal` picks every share and the
group key from `t` chosen shares; here each chosen share's public share
comes from the reference's curve, every other public share and the
group key from the reference's `derive_pubshare_at` and
`derive_thresh_pubkey` over those, and each secret share Faraday hands
out must be the discrete log of its public share.

Every vector file the BIP's repository publishes (less a tweak list
whose flags list has another length, which Faraday's types cannot
express: each tweak carries its own flag), then random groups
and whole signing sessions over random signer sets, tweaks and
messages, with tampered partial signatures.
"""

from common import Area, as_verify, use_reference, vectors

use_reference("bip445", "secp256k1lab")
use_reference("bip445")
from frost_ref import signing as F  # noqa: E402
from secp256k1lab.bip340 import schnorr_verify  # noqa: E402
from secp256k1lab.secp256k1 import G, GE, Scalar  # noqa: E402


def h(b):
    return None if b is None else bytes(b).hex()


def hl(items):
    return None if items is None else [h(x) for x in items]


def tweak_list(tweaks, xonly):
    assert len(tweaks) == len(xonly), "Faraday's tweak carries its own x-only flag"
    return [[h(t), x] for t, x in zip(tweaks, xonly)]


def without_shares(answer):
    if answer[0] == "ok":
        return ("ok", {k: v for k, v in answer[1].items() if k != "shares"})
    return answer


class Frost(Area):
    name = "frost"

    def nonce_gen(self, rand, secshare, pubshare, xonly, msg, extra, expected=None):
        def ref():
            sec, pub = F.nonce_gen_internal(rand, secshare, pubshare, xonly, msg, extra)
            return {"secnonce": h(sec), "pubnonce": h(pub)}
        return self.compare("frost.nonce_gen", {
            "rand": h(rand), "secshare": h(secshare), "pubshare": h(pubshare),
            "thresh_pk": h(xonly), "msg": h(msg), "extra_in": h(extra)}, ref, expected)

    def nonce_agg(self, pubnonces, expected=None):
        return self.compare("frost.nonce_agg", {"pubnonces": hl(pubnonces)},
                            lambda: {"aggnonce": h(F.nonce_agg(pubnonces))}, expected)

    def ctx_args(self, n, t, ids, pubshares, thresh_pk, aggnonce, tweaks, xonly, msg):
        return {"n": n, "t": t, "ids": ids, "pubshares": hl(pubshares), "thresh_pk": h(thresh_pk),
                "aggnonce": h(aggnonce), "tweaks": tweak_list(tweaks, xonly), "msg": h(msg)}

    def sign(self, secnonce, secshare, my_id, n, t, ids, pubshares, thresh_pk, aggnonce,
             tweaks, xonly, msg, expected=None):
        def ref():
            ctx = F.SessionContext(n, t, ids, pubshares, thresh_pk, aggnonce, tweaks, xonly, msg)
            return {"psig": h(F.sign(bytearray(secnonce), secshare, my_id, ctx))}
        args = self.ctx_args(n, t, ids, pubshares, thresh_pk, aggnonce, tweaks, xonly, msg)
        args.update(secnonce=h(secnonce), secshare=h(secshare), my_id=my_id)
        return self.compare("frost.sign", args, ref, expected)

    def verify(self, psig, pubnonces, n, t, ids, pubshares, thresh_pk, tweaks, xonly, msg, i,
               expected=None):
        args = {"psig": h(psig), "pubnonces": hl(pubnonces), "n": n, "t": t, "ids": ids,
                "pubshares": hl(pubshares), "thresh_pk": h(thresh_pk),
                "tweaks": tweak_list(tweaks, xonly), "msg": h(msg), "i": i}
        return self.compare("frost.verify", args, lambda: {"valid": F.partial_sig_verify(
            psig, pubnonces, n, t, ids, pubshares, thresh_pk, tweaks, xonly, msg, i)},
            expected, normalize=as_verify)

    def agg(self, psigs, n, t, ids, pubshares, thresh_pk, aggnonce, tweaks, xonly, msg,
            expected=None):
        def ref():
            ctx = F.SessionContext(n, t, ids, pubshares, thresh_pk, aggnonce, tweaks, xonly, msg)
            return {"sig": h(F.partial_sig_agg(psigs, ctx))}
        args = self.ctx_args(n, t, ids, pubshares, thresh_pk, aggnonce, tweaks, xonly, msg)
        args.update(psigs=hl(psigs))
        return self.compare("frost.agg", args, ref, expected)

    def det_sign(self, secshare, my_id, other, n, t, ids, pubshares, thresh_pk, tweaks, xonly,
                 msg, rand, expected=None):
        def ref():
            pub, psig = F.deterministic_sign(secshare, my_id, other, n, t, ids, pubshares,
                                             thresh_pk, tweaks, xonly, msg, rand)
            return {"pubnonce": h(pub), "psig": h(psig)}
        args = {"secshare": h(secshare), "my_id": my_id, "aggothernonce": h(other), "n": n,
                "t": t, "ids": ids, "pubshares": hl(pubshares), "thresh_pk": h(thresh_pk),
                "tweaks": tweak_list(tweaks, xonly), "msg": h(msg), "rand": h(rand)}
        return self.compare("frost.det_sign", args, ref, expected)

    def deal(self, n, t, chosen_ids, chosen):
        def ref():
            if not 1 <= t <= n or len(chosen_ids) != t or len(set(chosen_ids)) != t \
                    or any(not 0 <= i < n for i in chosen_ids):
                raise ValueError("t shares at distinct identifiers below n")
            points = [int.from_bytes(s, "big") * G for s in chosen]
            pubshares = [F.derive_pubshare_at(chosen_ids, points, x) for x in range(n)]
            if any(p.infinity for p in pubshares):
                raise ValueError("a share is zero")
            return {"pubshares": [p.to_bytes_compressed().hex() for p in pubshares],
                    "thresh_pk": F.derive_thresh_pubkey(chosen_ids, points).hex()}
        ans = self.compare("frost.deal", {"n": n, "t": t, "chosen_ids": chosen_ids,
                                          "chosen_shares": hl(chosen)}, ref,
                           normalize=without_shares)
        if ans[0] == "ok":
            for s, p in zip(ans[1]["shares"], ans[1]["pubshares"]):
                if (int(s, 16) * G).to_bytes_compressed().hex() != p:
                    raise AssertionError("a dealt share is not its public share's discrete log")
        return ans

    # -- the published vectors ----------------------------------------

    def published(self):
        fx = lambda l: [bytes.fromhex(x) for x in l]  # noqa: E731
        ok = lambda v: ("ok", v)  # noqa: E731
        refused = ("err", None)
        opt = lambda c, k: None if c.get(k) is None else bytes.fromhex(c[k])  # noqa: E731

        for c in vectors("bip445", "nonce_gen_vectors.json")["valid_tests"]:
            want = ok({"secnonce": c["expected"][0].lower(), "pubnonce": c["expected"][1].lower()}) \
                if isinstance(c["expected"], list) else None
            self.nonce_gen(opt(c, "rand"), opt(c, "secshare"), opt(c, "pubshare"),
                           opt(c, "thresh_pk_xonly"), opt(c, "msg"), opt(c, "extra_in"), want)
        v = vectors("bip445", "nonce_agg_vectors.json")
        P = fx(v["pubnonces"])
        for c in v["valid_tests"]:
            self.nonce_agg([P[i] for i in c["pubnonce_indices"]], ok({"aggnonce": c["expected"].lower()}))
        for c in v["error_tests"]:
            self.nonce_agg([P[i] for i in c["pubnonce_indices"]], refused)

        for g in vectors("bip445", "sign_verify_vectors.json")["test_groups"]:
            self.published_sign_verify(g, fx, ok, refused)
        for g in vectors("bip445", "tweak_vectors.json")["test_groups"]:
            self.published_tweak(g, fx, ok, refused)
        for g in vectors("bip445", "det_sign_vectors.json")["test_groups"]:
            self.published_det(g, fx, ok, refused)
        for g in vectors("bip445", "sig_agg_vectors.json")["test_groups"]:
            self.published_agg(g, fx, ok, refused)

    def group(self, g, fx):
        return (g["n"], g["t"], bytes.fromhex(g["thresh_pk"]), fx(g["pubshares"]),
                fx(g.get("secshares", [])), fx(g.get("secnonces", [])), fx(g.get("pubnonces", [])))

    def published_sign_verify(self, g, fx, ok, refused):
        n, t, tpk, PS, SS, SN, PN = self.group(g, fx)
        for c in g["valid_tests"] + g["sign_error_tests"]:
            pubshares = [PS[i] for i in c["pubshare_indices"]] if c.get("pubshare_indices") is not None else None
            want = ok({"psig": c["expected"].lower()}) if "expected" in c else refused
            self.sign(SN[c["secnonce_index"]], SS[c["secshare_index"]], c["my_id"], n, t, c["ids"],
                      pubshares, tpk, bytes.fromhex(c["aggnonce"]), [], [], bytes.fromhex(c["msg"]), want)
            if "expected" in c and pubshares is not None:
                self.verify(bytes.fromhex(c["expected"]), [PN[i] for i in c["pubnonce_indices"]], n, t,
                            c["ids"], pubshares, tpk, [], [], bytes.fromhex(c["msg"]),
                            c["ids"].index(c["my_id"]), ok({"valid": True}))
        for c in g["verify_fail_tests"] + g["verify_error_tests"]:
            self.verify(bytes.fromhex(c["psig"]), [PN[i] for i in c["pubnonce_indices"]], n, t, c["ids"],
                        [PS[i] for i in c["pubshare_indices"]], tpk, [], [], bytes.fromhex(c["msg"]),
                        c["signer_index"], ok({"valid": False}))

    def published_tweak(self, g, fx, ok, refused):
        n, t, tpk, PS, SS, SN, PN = self.group(g, fx)
        T = fx(g["tweaks"])
        for c in g["valid_tests"] + g["error_tests"]:
            tw = [T[i] for i in c["tweak_indices"]]
            if len(tw) != len(c["is_xonly"]):
                continue  # a list-length error Faraday's types cannot express
            want = ok({"psig": c["expected"].lower()}) if "expected" in c else refused
            self.sign(SN[c["secnonce_index"]], SS[c["secshare_index"]], c["my_id"], n, t, c["ids"],
                      [PS[i] for i in c["pubshare_indices"]], tpk, bytes.fromhex(c["aggnonce"]), tw,
                      c["is_xonly"], bytes.fromhex(c["msg"]), want)

    def published_det(self, g, fx, ok, refused):
        n, t, tpk, PS, SS, _, _ = self.group(g, fx)
        for c in g["valid_tests"] + g["error_tests"]:
            pubshares = [PS[i] for i in c["pubshare_indices"]] if c.get("pubshare_indices") is not None else None
            want = ok({"pubnonce": c["expected"][0].lower(), "psig": c["expected"][1].lower()}) \
                if "expected" in c else refused
            self.det_sign(SS[c["secshare_index"]], c["my_id"], opt_hex(c.get("aggothernonce")), n, t,
                          c["ids"], pubshares, tpk, fx(c["tweaks"]), c["is_xonly"],
                          bytes.fromhex(c["msg"]), opt_hex(c.get("aux_rand")), want)

    def published_agg(self, g, fx, ok, refused):
        n, t, tpk, PS, _, _, _ = self.group(g, fx)
        T = fx(g["tweaks"])
        for c in g["valid_tests"] + g["error_tests"]:
            want = ok({"sig": c["expected"].lower()}) if "expected" in c else refused
            pubshares = [PS[i] for i in c["pubshare_indices"]] if c.get("pubshare_indices") is not None else None
            self.agg(fx(c["psigs"]), n, t, c["ids"], pubshares, tpk,
                     bytes.fromhex(c["aggnonce"]), [T[i] for i in c["tweak_indices"]], c["is_xonly"],
                     bytes.fromhex(c["msg"]), want)

    # -- the random corpus --------------------------------------------

    def scalar(self):
        while True:
            k = int.from_bytes(self.randbytes(32), "big")
            if 0 < k < GE.ORDER:
                return k.to_bytes(32, "big")

    def session(self):
        rng = self.rng
        n = rng.choice([1, 2, 3, 3, 5])
        t = rng.randint(1, n)
        chosen_ids = rng.sample(range(n), t)
        dealt = self.need(self.deal(n, t, chosen_ids, [self.scalar() for _ in range(t)]), "deal")
        shares = [bytes.fromhex(s) for s in dealt["shares"]]
        pubshares_all = [bytes.fromhex(p) for p in dealt["pubshares"]]
        tpk = bytes.fromhex(dealt["thresh_pk"])
        ids = rng.sample(range(n), rng.randint(t, n))
        pubshares = [pubshares_all[i] for i in ids]
        tweaks = [self.scalar() for _ in range(rng.choice([0, 1, 2]))]
        xonly = [rng.random() < 0.5 for _ in tweaks]
        msg = self.randbytes(rng.choice([32, 32, 0, rng.randint(0, 80)]))
        tweaked = F.thresh_pubkey_and_tweak(tpk, tweaks, xonly)
        xpk = F.get_xonly_pk(tweaked)
        pick = lambda v: v if rng.random() < 0.5 else None  # noqa: E731
        secs, pubs = [], []
        for i in ids:
            ans = self.need(self.nonce_gen(self.randbytes(32), pick(shares[i]), pick(pubshares_all[i]),
                                           pick(xpk), pick(msg), pick(self.randbytes(rng.randint(0, 40)))),
                            "nonce_gen")
            secs.append(bytes.fromhex(ans["secnonce"]))
            pubs.append(bytes.fromhex(ans["pubnonce"]))
        aggnonce = bytes.fromhex(self.need(self.nonce_agg(pubs), "nonce_agg")["aggnonce"])
        psigs = []
        for k, i in enumerate(ids):
            known = pubshares if rng.random() < 0.7 else None
            psig = self.need(self.sign(secs[k], shares[i], i, n, t, ids, known, tpk, aggnonce, tweaks,
                                       xonly, msg), "sign")["psig"]
            psigs.append(bytes.fromhex(psig))
            self.verify(psigs[-1], pubs, n, t, ids, pubshares, tpk, tweaks, xonly, msg, k,
                        ("ok", {"valid": True}))
        k = rng.randrange(len(ids))
        bad = bytearray(psigs[k])
        bad[rng.randrange(32)] ^= 1 << rng.randrange(8)
        self.verify(bytes(bad), pubs, n, t, ids, pubshares, tpk, tweaks, xonly, msg, k)
        sig = bytes.fromhex(self.need(self.agg(psigs, n, t, ids, pick(pubshares), tpk, aggnonce,
                                               tweaks, xonly, msg), "agg")["sig"])
        if not schnorr_verify(msg, xpk, sig):
            raise AssertionError("the aggregate signature does not verify under BIP-340")
        k = rng.randrange(len(ids))
        others = F.nonce_agg(pubs[:k] + pubs[k + 1:]) if len(ids) > 1 else None
        self.det_sign(shares[ids[k]], ids[k], others, n, t, ids, pick(pubshares), tpk, tweaks,
                      xonly, msg, pick(self.randbytes(32)))
        # A signer set below the threshold, or naming an identifier twice.
        if t > 1:
            self.sign(secs[0], shares[ids[0]], ids[0], n, t, ids[:t - 1], None, tpk, aggnonce,
                      tweaks, xonly, msg)

    def run(self):
        self.published()
        for _ in range(self.cases(40)):
            self.session()


def opt_hex(x):
    return None if x is None else bytes.fromhex(x)
