"""Miniscript: the scripts Faraday's descriptors pay to, against Bitcoin
Core v31.1 itself (`deriveaddresses` on regtest, `core_node.py`) for
random wallets, and against Core's own miniscript vectors with Antoine
Poinsot's python-bip380 (`tools/reference/miniscript/`, through
`miniscript_ref.py`) where a vector states no script.

- Bitcoin Core's miniscript unit tests (`tools/vectors/miniscript/`, 97
  miniscripts, each in P2WSH and in Tapscript): a miniscript Faraday
  reads as a descriptor (`faraday_bip::policy::parse_script_descriptor`)
  is read exactly when Core calls it valid there and sane (valid,
  non-malleable, needing a signature, no timelocks of two kinds, no key
  twice, within P2WSH's 201 operations), and its script is Core's where
  the test states one, and python-bip380's where it does not; each
  script read back as a PSBT input's witness script is read
  (`ExtParams::sane` with raw `pkh`, as `faraday_psbt` reads one) and
  written out again unchanged, and refused when not sane.
- Random wallets of the shapes Liana, Nunchuk, Sparrow and Faraday's
  recovery wallets write, over random accounts on mainnet and the test
  networks: `wsh()` and `sh(wsh())` miniscripts, `multi` and
  `sortedmulti` in `sh()`, `wsh()` and `sh(wsh())`, and `tr()` with
  trees of one to six leaves of `pk`, `pkh`, timelocked keys, `or_d`,
  `multi_a` and a lone `sortedmulti_a`, over an extended key or BIP-341's
  NUMS point, each cosigner's account key made by btclib (`bip32_ref.py`).
  `faraday_bip::policy::WalletPolicy` reads the descriptor and gives the
  script it pays to at a random index on both chains; Bitcoin Core is
  given the same wallet with each key written as btclib's account key, a
  `tpub`, derives it there (`sortedmulti`, `sortedmulti_a`,
  `sh()`, the taproot tree and BIP-341's tweak all Core's own) and its
  address's script must be Faraday's. The same wallets made not sane (a
  spend with no signature, a block and a time delay in one branch, a key
  twice) are refused by both.

- btclib-wallet's descriptors and miniscript (`descriptor_ref.py`, on btclib's
  own curve) judge beside Core and python-bip380: Core's vectors (each
  miniscript read exactly when Core calls it valid, its type properties,
  whether it is sane, its script); every random wallet's descriptor (read
  or refused, its kind, the text it writes for each chain, the script at the
  index Core and Faraday pay); and the ways the coins move.
  `faraday_bip::policy::WalletPolicy::spend_paths` says, for each wallet,
  the keys that sign and the waits that must have passed for each way;
  btclib's satisfaction (`Descriptor.satisfy` with a `SpendContext` of an
  nSequence and an nLockTime) must spend at each way's keys and waits,
  refuse a block or a height one short of them, and, with every wait passed,
  spend with a set of keys exactly when it holds a way's keys.
- Recovery wallets in both forms (`faraday_bip::recovery::RecoveryPolicy`:
  one to three recovery paths of one to three keys each, a primary path, delays
  of 1 to 65,535 blocks): the script at a random index of either chain is
  Core's and btclib's, the kind and text are btclib's, the parts read back out
  (`RecoveryPolicy::from_wallet_policy`) are the parts given, and btclib's
  satisfaction spends with the primary keys at no wait and with each recovery
  path's keys at its delay and not a block before, and not one signature short.

python-bip380 writes `v:` over a `multi_a` as `OP_NUMEQUAL OP_VERIFY`
where BIP 379 and Core write `OP_NUMEQUALVERIFY`, so the corpus has no
`v:multi_a`; Core's vector of one is checked against Core's script.

btclib's `script/solver.py` and `script/spendability.py` are ports of Core
and are not used (ESTABLISHED §9.2, rule 3).

Known differences, checked and counted: Faraday reads `sortedmulti_a`
only as a `tr()`'s one leaf (`faraday_bip::tapmulti`); BIP 387 also
allows it deeper in a tree, which Faraday refuses and Core reads. Two of Core's sane
miniscripts `miniscript` refuses, and so Faraday (`KNOWN_REFUSED`): one
with no satisfaction at all, and a `d:` used as a unit under Tapscript.
btclib says nothing of "needs a signature" for a malleable miniscript (BIP 379
does not), so that property is compared where Core calls the miniscript
non-malleable.
"""

import re

import bip32_ref
import core_node
import descriptor_ref as btclib
import miniscript_ref as ms
from area_addr import read_on
from bip32_ref import HARDENED
from common import Area, Mismatch, use_reference, vectors

use_reference("core")
from test_framework.descriptors import descsum_create  # noqa: E402

VALID, NONMAL, NEEDSIG, MIX, P2WSH_INVALID, TAP_INVALID = 1, 2, 4, 8, 16, 32
# Core's sane miniscripts that `miniscript` refuses, and so Faraday: one
# with no satisfaction at all, and a `d:` used as a unit under
# Tapscript, which Core types `u` there and `miniscript` does not.
KNOWN_REFUSED = {
    ("wsh", "or_b(0,a:0)"),
    ("tap", "thresh(2,dv:older(42),s:pk(025cbdf0646e5db4eaa398f365f2ea7a0e3d419b7e0330e39ce92bddedcac4f9bc),"
            "s:pk(03d30199d74fb5a22d47b6e054e2f378cedacffcb89904a61d75d0dbd407143e65))"),
}
NUMS = bytes.fromhex("50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0")

# Segwit v0 miniscripts, keys as {0}, {1}, ...; N a block delay, T an
# absolute height.
WSH = [
    ("or_d(pk({0}),and_v(v:pkh({1}),older({N})))", 2),
    ("or_i(and_v(v:pkh({1}),older({N})),and_v(v:pk({0}),pk({2})))", 3),
    ("or_d(multi(2,{0},{1},{2}),and_v(v:pkh({3}),older({N})))", 4),
    ("andor(pk({0}),older({N}),pk({1}))", 2),
    ("thresh(2,pk({0}),s:pk({1}),sln:older({N}))", 2),
    ("and_v(v:pk({0}),or_d(pk({1}),older({N})))", 2),
    ("or_d(pk({0}),and_v(v:multi(2,{1},{2}),after({T})))", 3),
    ("and_v(v:pk({0}),after({T}))", 1),
    ("or_b(pk({0}),s:pk({1}))", 2),
    ("or_i(pk({0}),pk({1}))", 2),
    ("thresh(3,pk({0}),s:pk({1}),s:pk({2}),sln:older({N}))", 3),
]
# The same shapes made not sane.
WSH_INSANE = [
    ("or_d(pk({0}),older({N}))", 1),
    ("or_d(pk({0}),and_v(v:pkh({1}),and_v(v:older({N}),older(4194305))))", 2),
    ("or_d(pk({0}),and_v(v:pkh({0}),older({N})))", 1),
    ("or_i(pk({0}),after({T}))", 1),
]
# Tapscript leaves.
LEAVES = [
    ("pk({0})", 1),
    ("and_v(v:pk({0}),older({N}))", 1),
    ("and_v(v:pkh({0}),older({N}))", 1),
    ("multi_a(2,{0},{1},{2})", 3),
    ("multi_a(1,{0},{1})", 2),
    ("or_d(pk({0}),and_v(v:pk({1}),older({N})))", 2),
]
LEAVES_INSANE = [
    ("or_d(pk({0}),older({N}))", 1),
    ("and_v(v:pk({0}),and_v(v:after({T}),after(1700000000)))", 1),
]


KEY_FRAGMENTS = re.compile(r"(?:pk_k|pk_h|pkh|pk)\(([0-9a-f]+)\)|multi(?:_a)?\(\d+,([0-9a-f,]+)\)")


def core_keys(text, tap):
    """The keys a Core vector's miniscript names, as its script holds
    them (x-only under Tapscript)."""
    keys = []
    for one, many in KEY_FRAGMENTS.findall(text):
        keys += [one] if one else many.split(",")
    return [k[-64:] for k in keys] if tap else keys


class Miniscript(Area):
    name = "miniscript"
    CORE = True
    faraday = (
        "`faraday_bip::policy::WalletPolicy` (a wallet's descriptor and the script it "
        'pays at an index, its kind, text and spend paths), '
        '`faraday_bip::policy::parse_script_descriptor`, a witness '
        'script read back as `faraday_psbt` reads one, '
        "`faraday_bip::recovery::RecoveryPolicy` (a recovery wallet built from its parts and "
        'read back)'
    )
    reference = (
        "Bitcoin Core v31.1's `bitcoind` on regtest (`deriveaddresses`) for random "
        "wallets, their cosigners' keys from btclib (`btclib/`, on btclib's own curve); "
        "btclib-wallet's descriptors and miniscript (`btclib/`) for each descriptor's "
        "kind, text and script, Core's vectors' types and scripts, and the satisfaction "
        "of each way a wallet's coins move under its lock times; "
        "Antoine Poinsot's python-bip380 (`miniscript/`) for Core's vectors that state "
        'no script'
    )
    reference_checked = (
        'its vendored files are hashed against `references.sha256` before any run, and '
        "a published vector's answer is required of the reference as well as of "
        "Faraday; btclib reads Core's 97 miniscripts, in both contexts, before any "
        "random wallet (valid exactly when Core says, its properties, sane, script); "
        "Core's release tarball is checked against its pinned SHA-256 "
        "(`tools/reference/core/release.pin`) before every run, and the node's version is "
        'asked'
    )
    vectors = (
        "Bitcoin Core's miniscript unit tests, 97 miniscripts in P2WSH and Tapscript "
        '(`tools/vectors/miniscript/`)'
    )
    corpus = (
        'wallets of the Liana, Nunchuk, Sparrow and recovery shapes in `wsh()` and '
        '`sh(wsh())`, `multi`/`sortedmulti` in `sh()`, `wsh()`, `sh(wsh())`, `tr()` '
        'with trees of one to six leaves or a lone `sortedmulti_a`, over random '
        'accounts on every network, at random indices of both chains; the same made not '
        'sane, refused; recovery wallets of both forms with one to three recovery paths, '
        'and the ways each wallet\'s coins move spent at their lock times'
    )
    differences = [
        (
            "Miniscript: Faraday reads `sortedmulti_a` only as a `tr()`'s one leaf; BIP 387"
            " allows it deeper in a tree, and Core reads it there. Two of Bitcoin Core's "
            'sane miniscripts are '
            'refused, as `miniscript` refuses them: `or_b(0,a:0)`, which has no '
            'satisfaction, and a `d:` used as a unit under Tapscript. Each is checked and '
            'counted; btclib is not asked the deeper `sortedmulti_a`. btclib says nothing '
            'of "needs a signature" for a malleable miniscript (BIP 379 does not), so '
            "that property is compared where Core calls the miniscript non-malleable."
        ),
    ]
    not_compared = (
        "The witness bytes of a satisfaction (btclib assembles one and area `sign` "
        "compares Faraday's finalizer); a wallet with more than 64 ways to spend, for "
        "which Faraday states none; hash locks and locks in seconds, which the corpus "
        "does not generate; and a wallet btclib cannot read (none yet)."
    )

    def __init__(self, *a):
        super().__init__(*a)
        self.known = 0

    # ----- Bitcoin Core's vectors ------------------------------------------

    def core_vectors(self):
        data = vectors("miniscript", "core-miniscript.json")
        for case in data["cases"]:
            for ctx in ("wsh", "tap"):
                self.core_case(case, ctx)

    def core_case(self, case, ctx):
        mode, text = case["mode"], case["ms"]
        valid = bool(mode & VALID) and not (ctx == "wsh" and mode & P2WSH_INVALID) \
            and not (ctx == "tap" and mode & TAP_INVALID)
        keys = core_keys(text, ctx == "tap")
        sane = (valid and bool(mode & NONMAL) and bool(mode & NEEDSIG) and not mode & MIX
                and len(set(keys)) == len(keys)
                and not (ctx == "wsh" and case["ops"] is not None and case["ops"] > 201)
                and not (ctx == "wsh" and case["stack"] is not None and case["stack"] > 100))
        published = case[ctx]
        inputs = {"ms": text, "ctx": ctx}
        rust = self.rust.call("miniscript.script", **inputs)
        self.compared += 1
        if not sane:
            if rust[0] != "err":
                raise Mismatch(self.name, "miniscript.script", inputs, rust, ("err", "not sane"),
                               f"Core's mode {mode}: not valid and sane under {ctx}")
            self.btclib_case(mode, text, ctx, sane, None, rust)
            return
        if published != "?":
            want = published
        else:
            try:
                want = ms.compile_ms(text, ctx == "tap").hex()
            except Exception:  # noqa: BLE001 - python-bip380 cannot read it
                want = None
        if rust[0] != "ok" and (ctx, text) in KNOWN_REFUSED:
            self.known += 1
            self.btclib_case(mode, text, ctx, sane, want, rust)
            return
        if rust[0] != "ok" or (want is not None and rust[1]["script"] != want):
            raise Mismatch(self.name, "miniscript.script", inputs, rust, ("ok", {"script": want}),
                           f"Core's mode {mode}: valid and sane under {ctx}")
        script = rust[1]["script"]
        self.compare("miniscript.from_script", {"script": script, "ctx": ctx},
                     lambda: {"script": script})
        self.btclib_case(mode, text, ctx, sane, want, rust)

    def btclib_case(self, mode, text, ctx, sane, want, rust):
        """btclib's reading of a vector's miniscript: read exactly when
        Core calls it valid, its type properties Core's (a signature
        required is compared where the miniscript is non-malleable: btclib
        says nothing of it for a malleable one, as BIP 379 does not), sane
        exactly when Core says so, and its script Core's or
        python-bip380's. Faraday's script, where Faraday reads the
        miniscript, is btclib's."""
        tap = ctx == "tap"
        raw = re.sub(r"\b0[23]([0-9a-f]{64})\b", r"\1", text) if tap else text
        inputs = {"ms": text, "ctx": ctx}
        valid = bool(mode & VALID) and not (ctx == "wsh" and mode & P2WSH_INVALID) \
            and not (ctx == "tap" and mode & TAP_INVALID)
        try:
            got = btclib.miniscript(raw, tap)
        except Exception as e:  # noqa: BLE001 - any refusal of btclib
            got = None
            why = str(e)[:80]
        self.compared += 1
        if not valid:
            if got is not None:
                raise Mismatch(self.name, "miniscript.script", inputs, ("err", "not valid"),
                               ("ok", got), f"Core's mode {mode}: not valid under {ctx}, btclib reads it")
            return
        if got is None:
            raise Mismatch(self.name, "miniscript.script", inputs, ("ok", rust[1]),
                           ("err", why), f"Core's mode {mode}: valid under {ctx}, btclib refuses it")
        flags = (bool(mode & NONMAL), bool(mode & MIX))
        if (got["non_malleable"], got["mixes"]) != flags \
                or (flags[0] and got["signature"] != bool(mode & NEEDSIG)) \
                or got["sane"] != sane:
            raise Mismatch(self.name, "miniscript.script", inputs,
                           ("ok", {"non_malleable": flags[0], "mixes": flags[1],
                                   "signature": bool(mode & NEEDSIG), "sane": sane}),
                           ("ok", {k: got[k] for k in ("non_malleable", "mixes", "signature", "sane")}),
                           f"Core's mode {mode} under {ctx}: the type properties")
        if want is not None and got["script"] != want:
            raise Mismatch(self.name, "miniscript.script", inputs, ("ok", {"script": want}),
                           ("ok", {"script": got["script"]}), f"btclib's script under {ctx}")
        if rust[0] == "ok" and rust[1]["script"] != got["script"]:
            raise Mismatch(self.name, "miniscript.script", inputs, rust,
                           ("ok", {"script": got["script"]}), f"btclib's script under {ctx}")

    def core_insane_scripts(self):
        """Scripts of Core's valid miniscripts that are not sane, read
        back as a witness script: refused."""
        data = vectors("miniscript", "core-miniscript.json")
        for case in data["cases"]:
            mode = case["mode"]
            if not mode & VALID or (mode & NONMAL and mode & NEEDSIG and not mode & MIX):
                continue
            for ctx in ("wsh", "tap"):
                hexs = case[ctx]
                if hexs == "?" or (ctx == "wsh" and mode & P2WSH_INVALID) or \
                        (ctx == "tap" and mode & TAP_INVALID):
                    continue
                self.compare("miniscript.from_script", {"script": hexs, "ctx": ctx},
                             lambda: (_ for _ in ()).throw(ValueError("not sane")))

    # ----- random wallets ----------------------------------------------------

    def account(self, test):
        """A cosigner's account at m/48'/coin'/account'/2', its key as the
        wallet writes it (btclib's xpub with its origin) and as Core is
        given it (the same account key as a tpub, a chain to fill). Core
        is not given the master's tprv and the whole path: its miniscript
        tells keys apart by their key at index 0, which it cannot derive
        through a hardened step at parse time, so every such key reads to
        it as a duplicate."""
        seed = self.randbytes(32)
        m = bip32_ref.master(seed, test=test)
        path = [48 + HARDENED, (1 if test else 0) + HARDENED,
                self.rng.randrange(0, 4) + HARDENED, 2 + HARDENED]
        acct = m.derive(path).neuter()
        origin = bip32_ref.path_text(path).replace("m", m.fingerprint().hex(), 1)
        text = f"[{origin}]{acct.serialize()}/<0;1>/*"
        core = acct.with_test(True).serialize() + "/{chain}/*"
        return text, core

    def timelocks(self):
        return {"N": self.rng.randrange(1, 65536), "T": self.rng.randrange(1, 500_000_000)}

    def wallet(self, descriptor, core, test, known=False):
        """Faraday's script at random indices of both chains, against the
        script Core's `deriveaddresses` pays at the same place of `core`,
        the same wallet with each key written as the account's tpub: one
        RPC request for both. Where Core refuses the
        wallet, Faraday must too. `known`: a wallet Faraday refuses and
        BIP 387 allows, counted when Core reads it."""
        places = [(change, self.rng.choice([0, 1, self.rng.randrange(0, 1 << 31)]))
                  for change in (False, True)]
        answers = core_node.answers([
            ("deriveaddresses", descsum_create(core.replace("{chain}", str(int(change)))),
             [index, index]) for change, index in places])
        for (change, index), said in zip(places, answers):
            inputs = {"descriptor": descriptor, "change": change, "index": index}
            if known:
                rust = self.rust.call("miniscript.wallet", **inputs)
                self.compared += 1
                if rust[0] == "ok":
                    raise Mismatch(self.name, "miniscript.wallet", inputs, rust,
                                   ("err", "Faraday reads sortedmulti_a only as a tr()'s leaf"))
                if said[0] == "ok":
                    self.known += 1
                return

            def by_core(said=said):
                if said[0] == "err":
                    raise ValueError(f"Core: {said[1]}")
                return {"script_pubkey": read_on(said[1][0], "regtest").hex()}
            self.compare("miniscript.wallet", inputs, by_core)
            self.compare("miniscript.wallet", inputs, lambda: {
                "script_pubkey": btclib.script_at(descriptor, network_of(test), change, index)})
        if not known:
            self.inspect(descriptor, test, *places[0])

    def inspect(self, descriptor, test, change, index):
        """Faraday's reading of the wallet against btclib's: the kind of
        descriptor, the text it writes back (btclib's, with a hardened
        step written `h`), and the ways its coins move, each judged by
        btclib's satisfaction (`spend_check`)."""
        network = network_of(test)
        inputs = {"descriptor": descriptor}

        def same(answer):
            if answer[0] != "ok":
                return answer
            said = {k: answer[1][k] for k in ("type", "descriptor")}
            if isinstance(said["descriptor"], str):
                said["descriptor"] = btclib.split(said["descriptor"])
            return ("ok", said)

        def by_btclib():
            one = btclib.chains(descriptor, network)
            return {"type": btclib.shape(one[0]), "descriptor": [btclib.text_of(d) for d in one]}
        got = self.compare("miniscript.inspect", inputs, by_btclib, normalize=same)
        if got[0] == "ok":
            self.spend_check("miniscript.inspect", inputs, got[1], descriptor, network,
                             change, index)

    def spend_check(self, op, inputs, wallet, descriptor, network, change, index):
        """btclib's satisfaction of the wallet at `index` of one chain
        against the ways Faraday says its coins move (`wallet["paths"]`:
        the keys that sign, the waits that have passed). A way's keys at
        the way's own waits spend; a wait one short of it does not,
        unless another way is open to the same keys; with every wait
        passed, a set of keys spends exactly when it holds a way's keys;
        with none passed, the keys of all spend exactly when a way has no
        wait. The unspendable internal key of a taproot wallet stands in
        the list of keys and never signs: nobody holds it, and btclib, told
        it had signed, would spend by the key path. So no way may name it."""
        paths = [(set(p["keys"]), p["locks"]) for p in wallet["paths"]]
        if not paths:
            return
        if any(over_nums(wallet["keys"][i]) for have, _ in paths for i in have):
            raise Mismatch(self.name, op, inputs, ("ok", {"paths": wallet["paths"]}),
                           ("ok", {"keys": wallet["keys"]}), "a way names the unspendable key")
        d = btclib.chains(descriptor, network)[int(change)]
        tap = wallet["type"].startswith("tr")
        keys = btclib.keys_of(d, index, network, wallet["keys"])
        if len(keys) != len(wallet["keys"]):
            raise Mismatch(self.name, op, inputs, ("ok", {"keys": len(wallet["keys"])}),
                           ("ok", {"keys": len(keys)}), "btclib does not name every key Faraday does")

        def waits(locks):
            return (max([n for kind, n in locks if kind == "blocks"], default=0),
                    max([n for kind, n in locks if kind == "height"], default=0))

        def ask(signing, wait, expect, what):
            self.compared += 1
            got = btclib.spends(d, index, network, keys, sorted(signing), wait[0], wait[1], tap)
            if got != expect:
                raise Mismatch(
                    self.name, op, inputs,
                    ("ok", {"spends": expect, "keys": sorted(signing), "sequence": wait[0],
                            "locktime": wait[1]}),
                    ("ok", {"spends": got}), f"btclib's satisfaction, {what}")

        every = {i for i, text in enumerate(wallet["keys"]) if not over_nums(text)}
        top = waits([lock for _, locks in paths for lock in locks])
        for have, locks in paths:
            ask(have, waits(locks), True, "a way at its own waits")
            if any(other <= have for other, _ in paths if other is not have):
                continue
            sequence, height = waits(locks)
            if sequence:
                ask(have, (sequence - 1, height), False, "a block wait one short")
            if height:
                ask(have, (sequence, height - 1), False, "a height one short")
        ask(every, top, True, "every key, every wait passed")
        ask(every, (0, 0), any(not locks for _, locks in paths), "every key, no wait passed")
        for _ in range(4):
            signing = {i for i in every if self.rng.random() < 0.5}
            ask(signing, top, any(have <= signing for have, _ in paths),
                "some keys, every wait passed")

    def segwit_case(self, test):
        template, n = self.rng.choice(WSH + [("multi({k},{keys})", 0), ("sortedmulti({k},{keys})", 0)])
        if self.rng.random() < 0.15:
            template, n = self.rng.choice(WSH_INSANE)
        locks = self.timelocks()
        wrap = self.rng.choice(["wsh", "shwsh"] + (["sh"] if n == 0 else []))
        multi = n == 0
        if multi:
            count = self.rng.randrange(1, 6)
            k = self.rng.randrange(1, count + 1)
            n = count
        texts, cores = zip(*[self.account(test) for _ in range(n)])

        def body(keys):
            if multi:
                return template.format(k=k, keys=",".join(keys))
            return template.format(*keys[:n], **locks)

        def wrapped(keys):
            inner = body(list(keys))
            return {"wsh": f"wsh({inner})", "shwsh": f"sh(wsh({inner}))",
                    "sh": f"sh({inner})"}[wrap]
        desc = wrapped(texts)
        if self.rng.random() < 0.5:
            desc = descsum_create(desc)
        self.wallet(desc, wrapped(cores), test)

    def leaf(self, test, insane=False):
        template, n = self.rng.choice(LEAVES_INSANE if insane else LEAVES)
        texts, cores = zip(*[self.account(test) for _ in range(n)])
        return template, n, list(texts), list(cores)

    def taproot_case(self, test):
        locks = self.timelocks()
        sorted_lone = self.rng.random() < 0.15
        insane = not sorted_lone and self.rng.random() < 0.1
        count = 1 if sorted_lone else self.rng.randrange(1, 7)
        leaves = [self.leaf(test, insane and i == 0) for i in range(count)]
        if sorted_lone:
            m = self.rng.randrange(2, 5)
            texts, cores = zip(*[self.account(test) for _ in range(m)])
            leaves = [("sortedmulti_a({k},{keys})", m, list(texts), list(cores))]
            k = self.rng.randrange(1, m + 1)
        internal = self.account(test) if self.rng.random() < 0.5 else (NUMS.hex(), NUMS.hex())
        shape = self.shape(count)

        def text_of(i, keys):
            template, n, _, _ = leaves[i]
            if template.startswith("sortedmulti_a"):
                return template.format(k=k, keys=",".join(keys))
            return template.format(*keys[:n], **locks)

        def tree_text(node, side):
            if isinstance(node, int):
                return text_of(node, leaves[node][side])
            return "{" + tree_text(node[0], side) + "," + tree_text(node[1], side) + "}"

        desc = f"tr({internal[0]}," + tree_text(shape, 2) + ")"
        core = f"tr({internal[1]}," + tree_text(shape, 3) + ")"
        if not sorted_lone and count > 1 and self.rng.random() < 0.05:
            # A sortedmulti_a below a tree's root: BIP 387 allows it,
            # Faraday does not read it.
            (a, ca), (b, cb), (c, cc) = (self.account(test) for _ in range(3))
            desc = f"tr({internal[0]},{{sortedmulti_a(1,{a},{b}),pk({c})}})"
            core = f"tr({internal[1]},{{sortedmulti_a(1,{ca},{cb}),pk({cc})}})"
            self.wallet(descriptor_sum(desc, self.rng), core, test, known=True)
            return
        self.wallet(descriptor_sum(desc, self.rng), core, test)

    def shape(self, count):
        """A random binary tree over leaves 0..count-1, in order."""
        def build(lo, hi):
            if hi - lo == 1:
                return lo
            mid = self.rng.randrange(lo + 1, hi)
            return [build(lo, mid), build(mid, hi)]
        return build(0, count)

    # ----- recovery wallets ----------------------------------------------------

    def recovery_case(self, test):
        """A recovery wallet built from its parts by `RecoveryPolicy`, in
        both forms: its script at a random index against Core's and
        btclib's, its kind and text against btclib's, the parts read back
        out of it, the ways its coins move by btclib's satisfaction, and
        what the parts say (the primary keys spend with no wait, each
        recovery path's keys spend once its delay has passed and not a
        block before) by btclib's satisfaction too."""
        rng = self.rng
        network = network_of(test)
        count = rng.randint(1, 3)
        delays = rng.sample(range(1, 65536), count)
        if rng.random() < 0.2:
            delays[0] = rng.choice([1, 65535])
        sizes = [rng.randint(1, 3) for _ in range(count + 1)]
        accounts = [self.account(test) for _ in range(sum(sizes))]
        bare = [text[:-len(CHAINS)] for text, _ in accounts]
        parts, at = [], 0
        for size in sizes:
            parts.append({"threshold": rng.randint(1, size), "keys": bare[at:at + size]})
            at += size
        recovery = [dict(part, delay=delay) for part, delay in zip(parts[1:], delays)]
        change = rng.random() < 0.5
        index = rng.choice([0, 1, rng.randrange(0, 1 << 31)])
        form = rng.choice(["segwit", "taproot"])
        inputs = {"form": form, "primary": parts[0], "recovery": recovery,
                  "change": change, "index": index}
        got = self.rust.call("miniscript.recovery", **inputs)
        self.compared += 1
        if got[0] != "ok":
            raise AssertionError(f"recovery: refused a valid wallet: {got[1]}")
        wallet = got[1]

        def agree(what, faraday, reference):
            if faraday != reference:
                raise Mismatch(self.name, "miniscript.recovery", inputs, ("ok", {what: faraday}),
                               ("ok", {what: reference}), what)
        both = btclib.chains(wallet["descriptor"], network)
        agree("type", wallet["type"], btclib.shape(both[0]))
        agree("descriptor", btclib.split(wallet["descriptor"]), [btclib.text_of(d) for d in both])
        agree("script_pubkey", wallet["script_pubkey"],
              btclib.script_at(wallet["descriptor"], network, change, index))
        chain = int(change)
        core = core_text(wallet["descriptor"], accounts, chain)
        (said,) = core_node.answers([("deriveaddresses", descsum_create(core), [index, index])])
        if said[0] == "err":
            raise Mismatch(self.name, "miniscript.recovery", inputs, ("ok", wallet),
                           ("err", f"Core: {said[1]}"), "Core refuses the wallet")
        agree("script_pubkey", wallet["script_pubkey"], read_on(said[1][0], "regtest").hex())
        want = {"primary": {"threshold": parts[0]["threshold"], "keys": sorted(parts[0]["keys"])},
                "recovery": sorted(({"delay": r["delay"], "threshold": r["threshold"],
                                     "keys": sorted(r["keys"])} for r in recovery),
                                   key=lambda r: r["delay"])}
        back = wallet["back"] and {
            "primary": dict(wallet["back"]["primary"], keys=sorted(wallet["back"]["primary"]["keys"])),
            "recovery": [dict(r, keys=sorted(r["keys"])) for r in wallet["back"]["recovery"]]}
        agree("back", back, want)
        self.spend_check("miniscript.recovery", inputs, wallet, wallet["descriptor"], network,
                         change, index)
        self.parts_check(inputs, wallet, network, index, bare, parts, recovery, form == "taproot")

    def parts_check(self, inputs, wallet, network, index, bare, parts, recovery, tap):
        """btclib's satisfaction of the wallet against the parts it was
        built from."""
        one = btclib.chains(wallet["descriptor"], network)[int(inputs["change"])]
        keys = btclib.keys_of(one, index, network, bare)
        self.compared += 1
        if len(keys) != len(bare):
            raise Mismatch(self.name, "miniscript.recovery", inputs, ("ok", {"keys": len(bare)}),
                           ("ok", {"keys": len(keys)}), "btclib does not name every key")
        top = max(r["delay"] for r in recovery)
        position = {key: i for i, key in enumerate(bare)}

        def ask(signing, sequence, expect, what):
            self.compared += 1
            got = btclib.spends(one, index, network, keys, signing, sequence, 0, tap)
            if got != expect:
                raise Mismatch(self.name, "miniscript.recovery", inputs,
                               ("ok", {"spends": expect, "keys": signing, "sequence": sequence}),
                               ("ok", {"spends": got}), f"btclib's satisfaction, {what}")
        for path, delay, name in [(parts[0], 0, "the primary keys")] + \
                [(r, r["delay"], "a recovery path") for r in recovery]:
            have = [position[k] for k in path["keys"]][:path["threshold"]]
            ask(have, delay, True, f"{name} once their wait has passed")
            if delay:
                ask(have, delay - 1, False, f"{name} a block short of their wait")
            if path["threshold"] > 1:
                ask(have[:-1], top, False, f"{name} one signature short")

    LOOPS = (("corpus", 250),)

    def prelude(self):
        self.core_vectors()
        self.core_insane_scripts()

    def case_corpus(self):
        test = self.rng.random() < 0.6
        pick = self.rng.random()
        if pick < 0.5:
            self.segwit_case(test)
        elif pick < 0.85:
            self.taproot_case(test)
        else:
            self.recovery_case(test)


CHAINS = "/<0;1>/*"


def core_text(descriptor, accounts, chain):
    """Faraday's descriptor as Core is given it: each account's key
    written as its tpub with the one chain, and any other extended key
    (the unspendable internal key of a taproot recovery wallet) a tpub
    with that chain."""
    for text, core in accounts:
        xpub = text[:-len(CHAINS)].rsplit("]", 1)[-1]
        descriptor = re.sub(r"\[[^\]]*\]" + re.escape(xpub) + re.escape(CHAINS),
                            lambda _, core=core: core.replace("{chain}", str(chain)), descriptor)
    descriptor = re.sub(r"xpub[1-9A-HJ-NP-Za-km-z]+",
                        lambda m: bip32_ref.decode(m.group(0), False).with_test(True).serialize(),
                        descriptor)
    return descriptor.replace(CHAINS, f"/{chain}/*")


def over_nums(text):
    """Whether a key `[origin]xpub` of Faraday's is an extended key whose
    public key is BIP 341's NUMS point, which nobody can sign for."""
    return bip32_ref.decode(text.rsplit("]", 1)[-1], False).pub == b"\x02" + NUMS


def network_of(test):
    return "testnet" if test else "mainnet"


def descriptor_sum(desc, rng):
    return descsum_create(desc) if rng.random() < 0.5 else desc
