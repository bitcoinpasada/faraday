"""The policy compiler (`faraday_bip::compile::compile`, over
rust-miniscript's) against Bitcoin Core v31.1 itself, python-bip380 and
btclib-wallet, and against the policy it was given.

rust-miniscript's compiler chooses a script by its own costs, and no
other compiler chooses the same one (sipa's C++ compiler differs by
design). So the script is not compared with a compiler's. What a person
who types a policy can rely on is that the descriptor out is valid, pays
a wallet, and is spent by exactly the keys and waits the policy names;
each is judged by something Faraday did not write:

- Bitcoin Core (`getdescriptorinfo`, `deriveaddresses` on regtest,
  `core_node.py`) reads the descriptor: valid, the checksum Faraday
  wrote (a descriptor with testnet keys is given to Core as written, one
  with mainnet keys with each key as a tpub), and the address at a random
  index of either chain, whose script is Faraday's.
- btclib-wallet's descriptors (`descriptor_ref.py`) read the same text:
  its kind (`wsh(miniscript)`, `tr(tree)` or `tr`), the checksum, the
  script at the index, and each key of the policy named in it.
- python-bip380 (`miniscript/`) and btclib's miniscript read the
  miniscript, or each leaf of the `tr()` tree, with each key written as
  the key it derives at the index: valid, sane (a signature in every
  spend, no malleability, no two timelocks of one kind in one spend), the
  same script in both, and for `wsh()` the script hashed into the address
  Core derived.
- The policy and the miniscript are the same boolean formula over the
  keys and the waits: for every set of keys that sign and every wait
  (the block height reached, the blocks since the coins were received,
  each taken at every value a policy names and at none), the policy is
  satisfied exactly when the miniscript is (a `tr()` is satisfied by its
  internal key or by a leaf). The keys of a policy are few (at most six),
  so every assignment is tried.
- Each way Faraday lists the wallet can be spent
  (`faraday_bip::spend::SpendPath`, when it lists any) satisfies the
  policy, and every set of keys that satisfies the policy with all waits
  passed holds one of the ways' keys.

What is ours: the lift of a miniscript to a formula (`lift`), written
here from the meaning of each fragment in the miniscript specification and
applied to python-bip380's parse; rust-miniscript's own `lift` is not
used. And the rules by which a policy is refused (a threshold of 0 or
above its parts, a lock of 0, a key twice, nothing typed), which no
outside compiler states.
"""

import hashlib
import itertools
import re

import bip32_ref
import core_node
import descriptor_ref as btclib
import miniscript_ref as ms
from area_addr import read_on
from bip32_ref import HARDENED
from common import Area, Mismatch, use_reference

use_reference("core")
from test_framework.descriptors import descsum_create  # noqa: E402

use_reference("miniscript")
from bip380.miniscript import fragments as fr  # noqa: E402

MAX_KEYS = 6
CHAINS = "/<0;1>/*"
CHECKSUM = re.compile(r"#[a-z0-9]{8}$")


# ----- formulas ------------------------------------------------------------------
#
# ("key", i), ("after", n), ("older", n), ("true",), ("false",),
# ("and", [f...]), ("or", [f...]), ("atleast", k, [f...]).

def holds(f, signers, height, age):
    """Whether the formula is satisfied by these signers, at this block
    height, with the coins this many blocks old."""
    kind = f[0]
    if kind == "key":
        return f[1] in signers
    if kind == "after":
        return height >= f[1]
    if kind == "older":
        return age >= f[1]
    if kind == "true":
        return True
    if kind == "false":
        return False
    if kind == "and":
        return all(holds(s, signers, height, age) for s in f[1])
    if kind == "or":
        return any(holds(s, signers, height, age) for s in f[1])
    if kind == "atleast":
        return sum(holds(s, signers, height, age) for s in f[2]) >= f[1]
    raise AssertionError(kind)


def waits(f):
    """(the heights, the ages) a formula names."""
    heights, ages = set(), set()
    stack = [f]
    while stack:
        g = stack.pop()
        if g[0] == "after":
            heights.add(g[1])
        elif g[0] == "older":
            ages.add(g[1])
        elif g[0] in ("and", "or"):
            stack.extend(g[1])
        elif g[0] == "atleast":
            stack.extend(g[2])
    return heights, ages


def assignments(count, *formulas):
    """Every set of signers among `count` keys, with every wait: the
    height and the age each taken at 0 and at every value a formula
    names, which are all the waits that change any answer."""
    heights, ages = {0}, {0}
    for f in formulas:
        h, a = waits(f)
        heights |= h
        ages |= a
    for n in range(1 << count):
        signers = {i for i in range(count) if n >> i & 1}
        for height in sorted(heights):
            for age in sorted(ages):
                yield signers, height, age


def same(count, a, b):
    """The first assignment where the formulas differ, or None."""
    for signers, height, age in assignments(count, a, b):
        if holds(a, signers, height, age) != holds(b, signers, height, age):
            return sorted(signers), height, age
    return None


# ----- the policy ------------------------------------------------------------------
#
# ("pk", i), ("after", n), ("older", n), ("and", a, b),
# ("or", [(weight, p), (weight, p)]), ("thresh", k, [p...]).

def policy_text(p):
    kind = p[0]
    if kind == "pk":
        return f"pk(k{p[1]})"
    if kind in ("after", "older"):
        return f"{kind}({p[1]})"
    if kind == "and":
        return f"and({policy_text(p[1])},{policy_text(p[2])})"
    if kind == "or":
        return "or(" + ",".join((f"{w}@" if w != 1 else "") + policy_text(s) for w, s in p[1]) + ")"
    return f"thresh({p[1]}," + ",".join(policy_text(s) for s in p[2]) + ")"


def policy_formula(p):
    """The policy as its words say: `and` needs both, `or` either,
    `thresh(k, ...)` k of them."""
    kind = p[0]
    if kind == "pk":
        return ("key", p[1])
    if kind in ("after", "older"):
        return (kind, p[1])
    if kind == "and":
        return ("and", [policy_formula(p[1]), policy_formula(p[2])])
    if kind == "or":
        return ("or", [policy_formula(s) for _, s in p[1]])
    return ("atleast", p[1], [policy_formula(s) for s in p[2]])


# ----- the lift of a miniscript, from the fragments' meanings -------------------------
#
# python-bip380 parses the text into its fragments; what each means is
# written here. A fragment that can be satisfied is satisfied by the
# conditions its sub-fragments need; where a sub-fragment may be left
# unsatisfied (the dissatisfiable side of an `or`, the parts of a
# `thresh` beyond its k) it can always be dissatisfied, by the
# fragment's type (`d` and `u`, `e`, `s`).

def lift(node, names, wrong=False):
    """The boolean formula a miniscript fragment is satisfied by.
    `names` maps a key's bytes to its number. `wrong` is the self-test's
    lift that reads every `and` as an `or`."""
    kind = type(node).__name__

    def sub(i):
        return lift(node.subs[i], names, wrong)

    if kind == "Just0":
        return ("false",)
    if kind == "Just1":
        return ("true",)
    if kind in ("Pk", "Pkh"):
        # pk_k pushes the key, pk_h checks the key its hash names: either
        # needs that key's signature.
        return ("key", names[node.pubkey.bytes()])
    if kind == "Older":
        return ("older", node.value)
    if kind == "After":
        return ("after", node.value)
    if kind in ("Multi", "MultiA"):
        return ("atleast", node.k, [("key", names[key.bytes()]) for key in node.pubkeys])
    if kind in ("AndV", "AndB", "WrapT"):
        return ("or" if wrong else "and", [sub(0), sub(1)])
    if kind in ("OrB", "OrC", "OrD", "OrI", "WrapL", "WrapU"):
        # l:X is or_i(0,X) and u:X is or_i(X,0); python-bip380 keeps the 0.
        return ("or", [sub(0), sub(1)])
    if kind == "AndN":
        # andn(X,Y) is andor(X,Y,0): X and Y, or X unsatisfied and 0.
        return ("or" if wrong else "and", [sub(0), sub(1)])
    if kind == "AndOr":
        # X and Y, or (X dissatisfied and) Z; X is `d` and `u`, so it can
        # be dissatisfied without anything else.
        return ("or", [("or" if wrong else "and", [sub(0), sub(1)]), sub(2)])
    if kind == "Thresh":
        return ("atleast", node.k, [sub(i) for i in range(len(node.subs))])
    if kind in ("WrapA", "WrapS", "WrapC", "WrapD", "WrapV", "WrapJ", "WrapN"):
        # a: and s: move the stack, c: checks the signature, d: tests
        # nonzero, v: verifies, j: skips a zero, n: nonzero: none changes
        # what satisfies the fragment.
        return sub(0)
    raise AssertionError(f"no lift for {kind}")


def split_top(text):
    """`text` split at the commas outside every bracket."""
    out, depth, start = [], 0, 0
    for i, ch in enumerate(text):
        if ch in "({":
            depth += 1
        elif ch in ")}":
            depth -= 1
        elif ch == "," and depth == 0:
            out.append(text[start:i])
            start = i + 1
    out.append(text[start:])
    return out


def tree_leaves(tree):
    """The miniscripts of a `tr()` tree of `{left,right}` branches."""
    if tree.startswith("{"):
        left, right = split_top(tree[1:-1])
        return tree_leaves(left) + tree_leaves(right)
    return [tree]


def lift_descriptor(text, names, wrong=False):
    """(the formula, the miniscripts read, whether taproot) of a
    descriptor `wsh(miniscript)` or `tr(key[,tree])` written with raw
    keys and no checksum."""
    if text.startswith("wsh(") and text.endswith(")"):
        node = fr.Node.from_str(text[4:-1], False)
        return lift(node, names, wrong), [(text[4:-1], node)], False
    if text.startswith("tr(") and text.endswith(")"):
        parts = split_top(text[3:-1])
        internal = ("key", names[bytes.fromhex(parts[0])])
        nodes = []
        if len(parts) > 1:
            for leaf in tree_leaves(parts[1]):
                nodes.append((leaf, fr.Node.from_str(leaf, True)))
        branches = [internal] + [lift(n, names, wrong) for _, n in nodes]
        return ("or", branches), nodes, True
    raise AssertionError(f"neither a wsh nor a tr: {text}")


# ----- the area -------------------------------------------------------------------------

class Compile(Area):
    name = "compile"
    CORE = True
    faraday = (
        "`faraday_bip::compile::compile` (a concrete policy to a `wsh()` or `tr()` "
        "descriptor, the keys it names, the ways it can be spent and the script it pays at "
        "an index), over rust-miniscript's policy compiler"
    )
    reference = (
        "Bitcoin Core v31.1's `bitcoind` on regtest (`getdescriptorinfo` and "
        "`deriveaddresses`), with its `descriptors.py` (BIP-380's checksum); "
        "btclib-wallet's descriptors and miniscript (`btclib/`, on btclib's own curve) for "
        "the kind, checksum, script at an index and keys; Antoine Poinsot's python-bip380 "
        "(`miniscript/`) for the miniscript's validity, sanity and script, and its fragments "
        "for the lift to a formula, which is ours (`lift`, in `area_compile.py`)"
    )
    reference_checked = (
        'its vendored files are hashed against `references.sha256` before any run; '
        "Core's release tarball is checked against its pinned SHA-256 "
        "(`tools/reference/core/release.pin`) before every run, and the node's version is "
        'asked; the self-test makes Faraday\'s descriptor wrong and the lift wrong, and '
        'each is caught'
    )
    vectors = (
        'none: no compiler states a script for a policy (rust-miniscript, sipa\'s and '
        'the Go and Python ones choose differently), so the answers are judged by what '
        'the descriptor is and means'
    )
    corpus = (
        'policies of `and`, `or` (with weights), `thresh`, `pk`, `after` and `older` of one to '
        'six keys, each key written as an account key with its origin, compiled for '
        '`wsh()` and for `tr()` (a key beside the rest, which the compiler takes as the '
        'internal key), on mainnet and testnet keys, read at random indices of both '
        'chains; refusals: a threshold of 0 or above its parts, a lock of 0, a key twice, '
        'nothing typed'
    )
    differences = [
        (
            "A single key as a `tr()` policy: rust-miniscript's `compile_tr` takes the key "
            "as the internal key, is left with no leaf and refuses it (`Empty Miniscript "
            "compilation`); Faraday refuses it too, in its own words, and the area counts the refusal. "
            "python-bip380 writes `v:multi_a` as `OP_NUMEQUAL OP_VERIFY` where BIP 379 and "
            "Core write `OP_NUMEQUALVERIFY`, so a leaf with one is not compared to python-bip380's "
            "script (counted); the output key, which commits to the leaf, is Core's and btclib's."
        ),
    ]
    not_compared = (
        "The script itself: no other compiler makes rust-miniscript's choice, so only "
        "that it is valid, sane and means the policy is judged; hash locks, locks in "
        "seconds and the time flag of `older`, which the corpus does not generate; a "
        "policy that names a key by a fingerprint token of a loaded key; policies of more "
        "than six keys."
    )

    def __init__(self, exe, seed, scale, sabotage=None):
        super().__init__(exe, seed, scale, sabotage)
        self.wrong_lift = sabotage == "compile.lift"
        self.keys_made = 0

    # ----- policies --------------------------------------------------------------------

    def sig_policy(self, budget):
        """A policy every spend of which needs a signature, using at most
        `budget` keys; (policy, keys used)."""
        rng = self.rng
        if budget == 1 or rng.random() < 0.3:
            return ("pk", self.next_key()), 1
        choice = rng.random()
        if choice < 0.4:
            a, used = self.sig_policy(budget - 1)
            b, more = self.any_policy(budget - used)
            return (("and", a, b) if rng.random() < 0.5 else ("and", b, a)), used + more
        if choice < 0.75:
            a, used = self.sig_policy(budget // 2)
            b, more = self.sig_policy(budget - used)
            weights = [rng.choice([1, 1, 2, 5, 99]), rng.choice([1, 1, 3, 7])]
            return ("or", list(zip(weights, [a, b]))), used + more
        count = rng.randint(2, min(4, budget))
        subs, total = [], 0
        for i in range(count):
            s, used = self.sig_policy(max(1, (budget - total) - (count - i - 1)))
            subs.append(s)
            total += used
        return ("thresh", rng.randint(1, count), subs), total

    def any_policy(self, budget):
        """A policy that may need no signature: a lock, or a signing one."""
        rng = self.rng
        if budget <= 0 or rng.random() < 0.5:
            return self.lock(), 0
        return self.sig_policy(budget)

    def lock(self):
        rng = self.rng
        if rng.random() < 0.5:
            return ("older", rng.choice([1, 144, 1008, rng.randrange(1, 65536)]))
        return ("after", rng.choice([1, 800_000, rng.randrange(1, 499_999_999)]))

    def next_key(self):
        n = self.keys_made
        self.keys_made += 1
        return n

    def account(self, test):
        """A cosigner's account key as a policy writes it: the account's
        key with its origin and both chains."""
        m = bip32_ref.master(self.randbytes(32), test=test)
        path = [48 + HARDENED, (1 if test else 0) + HARDENED,
                self.rng.randrange(0, 4) + HARDENED, 2 + HARDENED]
        acct = m.derive(path).neuter()
        origin = bip32_ref.path_text(path).replace("m", m.fingerprint().hex(), 1)
        return f"[{origin}]{acct.serialize()}{CHAINS}", acct.serialize()

    # ----- one case ----------------------------------------------------------------------

    def case(self, loop, i):
        self.keys_made = 0
        super().case(loop, i)

    def case_corpus(self):
        rng = self.rng
        test = rng.random() < 0.6
        tap = rng.random() < 0.4
        budget = rng.randint(1, MAX_KEYS)
        if tap:
            # The compiler takes the internal key out of the policy's
            # first level: a key beside the rest.
            key = self.next_key()
            if budget == 1 or rng.random() < 0.1:
                self.lone_key()
                return
            else:
                rest, _ = self.sig_policy(budget - 1)
                weights = [rng.choice([1, 3, 10, 99]), 1]
                policy = ("or", list(zip(weights, [("pk", key), rest])))
        else:
            policy, _ = self.sig_policy(budget)
        if rng.random() < 0.1:
            self.refusal(policy)
        self.accepted(policy, tap, test)

    def lone_key(self):
        """A single key under `tr()`: rust-miniscript's `compile_tr` takes
        the key as the internal key and is left with no leaf, and
        refuses (`Empty Miniscript compilation`). Faraday refuses it too,
        in its own words. Known and counted."""
        test = self.rng.random() < 0.5
        expression, _ = self.account(test)
        self.known += 1
        self.compare("compile.compile",
                     {"policy": "pk(k0)", "script": "taproot", "keys": [["k0", expression]],
                      "change": False, "index": 0},
                     lambda: (_ for _ in ()).throw(ValueError("a key alone leaves no leaf")))

    def refusal(self, policy):
        """A policy made wrong in one way, which both sides refuse: the
        compiler must, and the rules are ours."""
        rng = self.rng
        bad = rng.choice(["empty", "thresh0", "thresh_over", "after0", "older0", "twice", "word"])
        keys = self.keys_made + 2
        tokens = [(f"k{i}", f"k{i}") for i in range(keys)]
        text = {
            "empty": "",
            "thresh0": "thresh(0,pk(k0),pk(k1))",
            "thresh_over": "thresh(3,pk(k0),pk(k1))",
            "after0": "and(pk(k0),after(0))",
            "older0": "and(pk(k0),older(0))",
            "twice": "and(pk(k0),pk(k0))",
            "word": "both(pk(k0),pk(k1))",
        }[bad]
        for script in ("segwit", "taproot"):
            self.compare("compile.compile",
                         {"policy": text, "script": script, "keys": tokens, "change": False, "index": 0},
                         lambda bad=bad: (_ for _ in ()).throw(ValueError(f"{bad}")))

    def accepted(self, policy, tap, test):
        rng = self.rng
        network = "testnet" if test else "mainnet"
        count = self.keys_made
        accounts = [self.account(test) for _ in range(count)]
        text = policy_text(policy)
        change = rng.random() < 0.5
        index = rng.choice([0, 1, rng.randrange(0, 1 << 31)])
        inputs = {"policy": text, "script": "taproot" if tap else "segwit",
                  "keys": [[f"k{i}", accounts[i][0]] for i in range(count)],
                  "change": change, "index": index}
        got = self.rust.call("compile.compile", **inputs)
        self.compared += 1
        if got[0] != "ok":
            raise Mismatch(self.name, "compile.compile", inputs, got,
                           ("ok", "a descriptor"), "Faraday refuses a policy that compiles")
        out = got[1]
        descriptor = out["descriptor"]

        def disagree(what, faraday, reference):
            raise Mismatch(self.name, "compile.compile", inputs, ("ok", {what: faraday}),
                           ("ok", {what: reference}), what)

        # The checksum: BIP-380's code, Core's framework and btclib's.
        if not CHECKSUM.search(descriptor):
            disagree("descriptor", descriptor, "a descriptor with a checksum")
        base = CHECKSUM.sub("", descriptor)
        for who, said in (("Core's descriptors.py", descsum_create(base)),
                          ("btclib", btclib.checksum_of(base))):
            if said != descriptor:
                disagree("checksum", descriptor, who + ": " + said)
        if not btclib.verify(descriptor):
            disagree("checksum", descriptor, "btclib: not its own")
        # Core's node reads the descriptor and derives the address.
        core_base = base
        if not test:
            core_base = re.sub(r"xpub[1-9A-HJ-NP-Za-km-z]+",
                               lambda m: bip32_ref.decode(m.group(0), False).with_test(True).serialize(),
                               base)
        chain = int(change)
        derive = descsum_create(core_base.replace(CHAINS, f"/{chain}/*"))
        info, derived = core_node.answers([("getdescriptorinfo", core_base),
                                           ("deriveaddresses", derive, [index, index])])
        if info[0] == "err":
            disagree("getdescriptorinfo", descriptor, "Core: " + info[1])
        if info[1]["checksum"] != descsum_create(core_base)[-8:]:
            disagree("checksum", descsum_create(core_base)[-8:], "Core: " + info[1]["checksum"])
        if core_base == base and info[1]["checksum"] != descriptor[-8:]:
            disagree("checksum", descriptor[-8:], "Core: " + info[1]["checksum"])
        if not (info[1]["isrange"] and info[1]["issolvable"]):
            disagree("descriptor", descriptor, f"Core: {info[1]}")
        if derived[0] == "err":
            disagree("deriveaddresses", descriptor, "Core: " + derived[1])
        core_script = read_on(derived[1][0], "regtest")
        if out["script_pubkey"] is None:
            disagree("script_pubkey", None, core_script.hex())
        if out["script_pubkey"] != core_script.hex():
            disagree("script_pubkey", out["script_pubkey"], "Core: " + core_script.hex())
        # btclib reads the text.
        both = btclib.chains(descriptor, network)
        kind = btclib.shape(both[0])
        # btclib names the script in a `wsh()` by its own class (a lone key,
        # a `multi`, or a miniscript), and a `tr()` with a leaf a tree.
        if not (kind.startswith("wsh(") if not tap else kind == "tr(tree)"):
            disagree("kind", "wsh(...)" if not tap else "tr(tree)", kind)
        if btclib.script_at(descriptor, network, change, index) != core_script.hex():
            disagree("script_pubkey", core_script.hex(), "btclib: another script")
        xpubs = [acct[1] for acct in accounts]
        one = both[chain]
        found = btclib.keys_of(one, index, network, xpubs)
        if len(found) != count:
            disagree("keys", count, f"btclib names {len(found)}")
        raw = {i: (found[i][1:] if tap else found[i]).hex() for i in range(count)}
        names = {bytes.fromhex(raw[i]): i for i in range(count)}
        # The miniscript with each key as the key it derives, read by
        # python-bip380 and btclib.
        written = base
        for i in range(count):
            written = written.replace(accounts[i][0], raw[i])
        formula, nodes, is_tap = lift_descriptor(written, names, self.wrong_lift)
        for leaf, node in nodes:
            self.miniscript_check(inputs, leaf, node, tap)
        if not tap:
            program = hashlib.sha256(bytes(nodes[0][1].script)).digest()
            if core_script != b"\x00\x20" + program:
                disagree("script_pubkey", core_script.hex(), "the miniscript's script hashed: "
                         + (b"\x00\x20" + program).hex())
        # The policy and the miniscript mean the same.
        wanted = policy_formula(policy)
        differs = same(count, wanted, formula)
        if differs is not None:
            signers, height, age = differs
            # Named for the lift: it is the self-test's second round.
            raise Mismatch(self.name, "compile.lift", inputs, ("ok", {"descriptor": descriptor}),
                           ("ok", {"policy": text}),
                           f"the policy and the miniscript differ with keys {signers} signing, "
                           f"height {height}, age {age}")
        self.paths_check(inputs, out, wanted, count, accounts)

    def miniscript_check(self, inputs, leaf, node, tap):
        """python-bip380 and btclib read a miniscript (a `wsh()`'s, or one
        leaf of a tree): valid, sane, the same script."""
        def disagree(what, faraday, reference):
            raise Mismatch(self.name, "compile.compile", inputs, ("ok", {what: faraday}),
                           ("ok", {what: reference}), f"{what}: {leaf}")
        self.compared += 1
        if not (node.needs_sig and node.is_nonmalleable and node.no_timelock_mix):
            disagree("sane", True, "python-bip380 does not call it sane")
        if not node.p.has_all("B"):
            disagree("type", "B", str(node.p))
        try:
            theirs = btclib.miniscript(leaf, tap)
        except Exception as e:  # noqa: BLE001 - any refusal of btclib
            disagree("read", True, f"btclib refuses it: {str(e)[:80]}")
        if not (theirs["valid"] and theirs["sane"]):
            disagree("sane", True, f"btclib: {theirs}")
        if tap and "v:multi_a(" in leaf:
            # python-bip380 writes OP_NUMEQUAL OP_VERIFY where BIP 379 and
            # Core write OP_NUMEQUALVERIFY; the output key, which commits
            # to the leaf, is Core's and btclib's.
            self.known += 1
        elif theirs["script"] != bytes(node.script).hex():
            disagree("python-bip380's script", bytes(node.script).hex(), "btclib: " + theirs["script"])

    def paths_check(self, inputs, out, wanted, count, accounts):
        """Each way Faraday lists satisfies the policy, and every set of
        keys that satisfies it with every wait passed holds a way's."""
        if not out["paths"]:
            return
        place = {}
        for position, expression in enumerate(out["keys"]):
            place[position] = next(i for i in range(count) if accounts[i][0] == expression)
        ways = []
        for way in out["paths"]:
            have = {place[k] for k in way["keys"]}
            height = max([n for kind, n in way["locks"] if kind == "height"], default=0)
            age = max([n for kind, n in way["locks"] if kind == "blocks"], default=0)
            other = [kind for kind, _ in way["locks"] if kind not in ("height", "blocks")]
            self.compared += 1
            if other or not holds(wanted, have, height, age):
                raise Mismatch(self.name, "compile.compile", inputs, ("ok", {"path": way}),
                               ("ok", {"policy satisfied": False}),
                               "a way Faraday lists does not satisfy the policy")
            ways.append(have)
        heights, ages = waits(wanted)
        top, oldest = max(heights, default=0), max(ages, default=0)
        for signers in (set(s) for r in range(count + 1)
                        for s in itertools.combinations(range(count), r)):
            if holds(wanted, signers, top, oldest) and not any(w <= signers for w in ways):
                raise Mismatch(self.name, "compile.compile", inputs, ("ok", {"paths": out["paths"]}),
                               ("ok", {"keys that spend": sorted(signers)}),
                               "keys that spend the policy hold none of the ways Faraday lists")

    LOOPS = (("corpus", 300),)

    def prelude(self):
        self.keys_made = 0
        for policy, tap in FIXED:
            self.keys_made = 0
            self.rng_fixed(policy)
            self.accepted(policy, tap, tap)

    def rng_fixed(self, policy):
        n = 0
        stack = [policy]
        while stack:
            p = stack.pop()
            if p[0] == "pk":
                n = max(n, p[1] + 1)
            elif p[0] == "and":
                stack += [p[1], p[2]]
            elif p[0] == "or":
                stack += [s for _, s in p[1]]
            elif p[0] == "thresh":
                stack += p[2]
        self.keys_made = n


# The shapes Faraday's screen shows as examples, and the ones Liana,
# Nunchuk and Sparrow write.
FIXED = [
    (("pk", 0), False),
    (("and", ("pk", 0), ("older", 144)), False),
    (("or", [(1, ("pk", 0)), (1, ("and", ("pk", 1), ("older", 1008)))]), False),
    (("or", [(99, ("pk", 0)), (1, ("and", ("pk", 1), ("older", 1008)))]), True),
    (("thresh", 2, [("pk", 0), ("pk", 1), ("pk", 2)]), False),
    (("or", [(1, ("thresh", 2, [("pk", 0), ("pk", 1), ("pk", 2)])),
             (1, ("and", ("pk", 3), ("after", 800000)))]), False),
    (("or", [(10, ("pk", 0)), (1, ("thresh", 2, [("pk", 1), ("pk", 2), ("and", ("pk", 3), ("older", 52560))]))]), True),
]
