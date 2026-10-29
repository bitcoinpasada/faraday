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

python-bip380 writes `v:` over a `multi_a` as `OP_NUMEQUAL OP_VERIFY`
where BIP 379 and Core write `OP_NUMEQUALVERIFY`, so the corpus has no
`v:multi_a`; Core's vector of one is checked against Core's script.

Known differences, checked and counted: Faraday reads `sortedmulti_a`
only as a `tr()`'s one leaf (`faraday_bip::tapmulti`); BIP 387 also
allows it deeper in a tree, which Faraday refuses and Core reads. Two of Core's sane
miniscripts `miniscript` refuses, and so Faraday (`KNOWN_REFUSED`): one
with no satisfaction at all, and a `d:` used as a unit under Tapscript.
"""

import re

import bip32_ref
import core_node
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
        'pays at an index), `faraday_bip::policy::parse_script_descriptor`, a witness '
        'script read back as `faraday_psbt` reads one'
    )
    reference = (
        "Bitcoin Core v31.1's `bitcoind` on regtest (`deriveaddresses`) for random "
        "wallets, their cosigners' keys from btclib (`btclib/`, on btclib's own curve); "
        "Antoine Poinsot's python-bip380 (`miniscript/`) for Core's vectors that state "
        'no script'
    )
    reference_checked = (
        'its vendored files are hashed against `references.sha256` before any run, and '
        "a published vector's answer is required of the reference as well as of "
        "Faraday; Core's release tarball is checked against its pinned SHA-256 "
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
        'sane, refused'
    )
    differences = [
        (
            "Miniscript: Faraday reads `sortedmulti_a` only as a `tr()`'s one leaf; BIP 387"
            " allows it deeper in a tree, and Core reads it there. Two of Bitcoin Core's "
            'sane miniscripts are '
            'refused, as `miniscript` refuses them: `or_b(0,a:0)`, which has no '
            'satisfaction, and a `d:` used as a unit under Tapscript. Each is checked and '
            'counted.'
        ),
    ]
    not_compared = (
        'none'
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
            return
        if rust[0] != "ok" or (want is not None and rust[1]["script"] != want):
            raise Mismatch(self.name, "miniscript.script", inputs, rust, ("ok", {"script": want}),
                           f"Core's mode {mode}: valid and sane under {ctx}")
        script = rust[1]["script"]
        self.compare("miniscript.from_script", {"script": script, "ctx": ctx},
                     lambda: {"script": script})

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

    def wallet(self, descriptor, core, known=False):
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
        self.wallet(desc, wrapped(cores))

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
            self.wallet(descriptor_sum(desc, self.rng), core, known=True)
            return
        self.wallet(descriptor_sum(desc, self.rng), core)

    def shape(self, count):
        """A random binary tree over leaves 0..count-1, in order."""
        def build(lo, hi):
            if hi - lo == 1:
                return lo
            mid = self.rng.randrange(lo + 1, hi)
            return [build(lo, mid), build(mid, hi)]
        return build(0, count)

    LOOPS = (("corpus", 250),)

    def prelude(self):
        self.core_vectors()
        self.core_insane_scripts()

    def case_corpus(self):
        test = self.rng.random() < 0.6
        if self.rng.random() < 0.55:
            self.segwit_case(test)
        else:
            self.taproot_case(test)


def descriptor_sum(desc, rng):
    return descsum_create(desc) if rng.random() < 0.5 else desc
