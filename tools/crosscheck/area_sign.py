"""PSBT signing and finalizing end to end: `faraday_psbt::sign` and
`faraday_psbt::finalize` against Bitcoin Core itself, the pinned release
on regtest (`core_node.py`, F79): Core's signer, Core's finalizer and
extractor, and Core's script interpreter and mempool policy judging every
transaction Faraday finalizes.

- Published: BIP-174's signer, finalizer and extractor vectors
  (`tools/vectors/psbt/bip174-roles.txt`), and the wallets' spends in
  `tools/vectors/psbt/` that Bitcoin Core built (a 2-of-3 `sortedmulti`,
  the Liana-shaped miniscript down both paths, a taproot tree by a leaf
  and by the key path, BIP 387's taproot multisig), signed with every
  subset of their keys that matters; and the finalizer vectors there,
  each finalized as its final PSBT says where it has one.
- Random PSBTs of one to four inputs of every kind Faraday signs: P2PKH,
  P2SH-P2WPKH, P2WPKH, `multi` and `sortedmulti` in P2SH, P2WSH and
  P2SH-P2WSH, P2TR key path (BIP-86, and a key path beside a tree),
  P2TR script path (leaves of `pk`, `pkh` and timelocked keys, `or_d`,
  `multi_a`), and miniscript in P2WSH and P2SH-P2WSH (the Liana,
  recovery and threshold shapes), with keys this device holds, keys it
  does not, signatures cosigners already made (Core's), every standard
  sighash type, timelocks met and not met, under both RFC 6979 nonce
  rules. Each input spends an output of a funding transaction made on
  the node from a coin of the run's (`Area.FUND`), sent and buried
  before the run, so the spend is real.

Signing. Core's `descriptorprocesspsbt` signs the same PSBT with a
`pk()` descriptor for each key origin the PSBT names under a held
master, the master's extended private key and the origin's path, so Core
derives every key and decides which keys sign where: Faraday's added
pairs must be Core's. An ECDSA signature under Faraday's default nonce
rule (low R, ground as Core grinds) is Core's byte for byte. Under the
first RFC 6979 nonce, Core's signature is checked to verify under the
signature hash computed here (Core's `script.py`, the script code chosen
here), and Faraday's must be the one Core's `key.py` makes with that
nonce, and Core's when its R is low. Core's BIP-340 signatures are
randomised: each is checked to verify under the hash computed here, and
Faraday's must be the one BIP-340's reference (`secp256k1lab`) makes with
zero auxiliary randomness, by the tweaked key on a key path. Core signs a
taproot key path alone where it can; the leaves beside it are signed by a
second call without the internal key's descriptor.

Finalizing. Core's `finalizepsbt` finalizes the PSBT Faraday finalizes:
each final scriptSig and witness must be Core's byte for byte (Core keeps
the smallest witness among a taproot input's leaves, and a miniscript's
smallest non-malleable satisfaction), the extracted transaction Core's,
and the PSBT's other fields those BIP-174 keeps (the UTXOs and unknown
types). Then `testmempoolaccept` must accept the extracted transaction,
which spends the funding outputs on the node: consensus and standardness
are Core's verdict.

What is ours, in this file (F160): which inputs Faraday refuses or leaves
unsigned. Faraday refuses a SegWit v0 input that carries no previous
transaction in a transaction of more than one input, a finalized one
of those too unless taproot, a `NONE` or `SINGLE` sighash, a PSBT none of
the held keys takes part in (F136), and an origin under a held master
that derives another key or a key not in the script; a miniscript or a
taproot tree is signed only through a registered wallet that holds a
held key and gives its script, and an input of several keys is refused
without one (F141). Core signs all of these. The keys of the checks are
derived by `btc.py`. The signature hashes for the deterministic nonces
are computed with Core's `script.py` over the script code chosen here.

A witness script or leaf is read from the miniscript text it was
compiled from (`miniscript_ref.Scripts`, python-bip380): the corpus keeps
each text, and a fixture's are its wallet policy's.

Known differences, counted: the refusals above. Faraday signs a taproot
leaf with a `pkh` but leaves it unfinalized (`miniscript` 12 cannot
write that satisfaction), where Core finalizes it: the area takes the
signatures under such a leaf off before Core finalizes, and counts each
input where Core would have finalized otherwise.
"""

import base64
import io
import re

import btc
import core_node
import miniscript_ref as ms
from area_psbt import MAGIC, compact, read_map, read_unsigned, ser_compact, write_map
from btc import HARDENED
from common import ROOT, Area, HarnessError, Mismatch, use_reference

use_reference("core")
import test_framework.script as S  # noqa: E402
from test_framework.key import ECKey, ECPubKey, TaggedHash, tweak_add_privkey  # noqa: E402
from test_framework.messages import (  # noqa: E402
    COutPoint, CTransaction, CTxIn, CTxInWitness, CTxOut, ser_string)

use_reference("bip39")
from mnemonic import Mnemonic  # noqa: E402

use_reference("bip352", "secp256k1lab", "src")
from secp256k1lab.bip340 import schnorr_sign, schnorr_verify  # noqa: E402

import lineage  # noqa: E402

N = btc.N
NUMS_HEX = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0"
TAPSCRIPT = 0xC0
STANDARD = [1, 2, 3, 0x81, 0x82, 0x83]
TAPROOT = [0, 1, 2, 3, 0x81, 0x82, 0x83]
# The input fields BIP-174 and BIP-371 define, which a finalizer clears
# but for the UTXOs; any other type is unknown and kept.
DEFINED = {0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
           0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0xfc}
SIGS = (0x02, 0x13, 0x14)
# The chain heights a corpus timelock reaches: the funding is buried
# deeper than any relative one, and the chain is past every absolute one.
LOCKS = 1000


class Refused(Exception):
    pass


# Every miniscript this area builds or reads a fixture's wallet for,
# beside its script (`miniscript_ref.Scripts`).
SCRIPTS = ms.Scripts()
# The sighash types that leave outputs open to change after signing: a
# danger Faraday refuses with no way past it (F136), a known difference.
OPEN = [2, 3, 0x82, 0x83]


def split_tree(tree):
    """A `tr()` tree's leaves, left to right."""
    if not tree.startswith("{"):
        return [tree]
    body, depth = tree[1:-1], 0
    for j, ch in enumerate(body):
        depth += ch in "({"
        depth -= ch in ")}"
        if ch == "," and depth == 0:
            return split_tree(body[:j]) + split_tree(body[j + 1:])
    raise ValueError("not a tree")


def register_policy(text, raw):
    """The miniscripts of a BIP-388 wallet policy at the chain and index
    each input of `raw` names in its key origins, kept with their
    scripts: each key derived by `btc.py` from the policy's own key."""
    lines = [x.strip() for x in text.splitlines() if x.strip() and not x.startswith("#")]
    template, keys = lines[0], lines[1:]
    xpubs = [btc.decode(k.split("]", 1)[1], False) for k in keys]
    tap = template.startswith("tr(")
    for m in parse(raw)[1]:
        chains = [origin(v)[1][-2:] for _, v in of_type(m, 0x06)]
        chains += [tap_origin(v)[2][-2:] for _, v in of_type(m, 0x16)]
        if not chains:
            continue
        pubs = [x.derive(chains[0]).pub for x in xpubs]

        def fill(t):
            for i in reversed(range(len(pubs))):
                t = t.replace(f"@{i}/**", (pubs[i][1:] if tap else pubs[i]).hex())
            return t
        if template.startswith("wsh("):
            SCRIPTS.add(fill(template[4:-1]), False)
            continue
        tree = template[3:-1].split(",", 1)[1]
        if tree.startswith("sortedmulti_a("):
            k = int(tree[len("sortedmulti_a("):].split(",", 1)[0])
            ordered = sorted(p[1:] for p in pubs)
            SCRIPTS.add(f"multi_a({k},{','.join(x.hex() for x in ordered)})", True)
            continue
        for leaf in split_tree(tree):
            SCRIPTS.add(fill(leaf), True)


class Known(Refused):
    """A refusal Faraday makes where Core signs; `kind` is the warning
    Faraday's refusal must name, where it is one."""

    def __init__(self, why, kind=None):
        super().__init__(why)
        self.kind = kind


KEY_EXPR = re.compile(r"\[([0-9a-f]{8})((?:/[0-9]+[h']?)*)\]([xt]pub[1-9A-HJ-NP-Za-km-z]+)/<0;1>/\*")


def read_wallet(text):
    """A wallet's template, `@i/**` for each key, and its keys as
    (fingerprint, path, xpub): BIP-388's two-part form or a descriptor."""
    lines = [x.strip() for x in text.splitlines() if x.strip() and not x.startswith("#")]
    keys = []
    if "@" in lines[0]:
        exprs = [k.strip() for line in lines[1:] for k in line.split(",") if k.strip()]
        for e in exprs:
            fp, path = e[1:].split("]", 1)[0].split("/", 1)
            keys.append((bytes.fromhex(fp), btc.parse_path("m/" + path),
                         btc.decode(e.split("]", 1)[1], False)))
        return lines[0], keys

    def sub(m):
        keys.append((bytes.fromhex(m.group(1)), btc.parse_path("m" + m.group(2)),
                     btc.decode(m.group(3), False)))
        return f"@{len(keys) - 1}/**"
    return KEY_EXPR.sub(sub, " ".join(lines).split("#")[0]), keys


def input_origins(m):
    """Every key origin an input states, (fingerprint, path)."""
    return ([origin(v) for _, v in of_type(m, 0x06)]
            + [tap_origin(v)[1:] for _, v in of_type(m, 0x16)])


def holds(keys, masters):
    """Whether a wallet holds a held master's key, as Faraday's
    `Context::holds` asks: one of its keys is under a held fingerprint
    and is that master's extended key at its path, and none under a held
    fingerprint is another key."""
    mine = [masters[fp].derive(path).neuter().serialize() == x.serialize()
            for fp, path, x in keys if fp in masters]
    return bool(mine) and all(mine)


def split_top(text):
    """`a,b` split at its first comma outside brackets."""
    depth = 0
    for j, ch in enumerate(text):
        depth += ch in "({"
        depth -= ch in ")}"
        if ch == "," and depth == 0:
            return text[:j], text[j + 1:]
    raise ValueError("no comma")


def tap_items(tree, leaf):
    """A `tr()` tree as Core's `taproot_construct` takes it, each leaf
    made by `leaf`."""
    names = iter(range(1 << 20))

    def node(t):
        if t.startswith("{"):
            a, b = split_top(t[1:-1])
            return [node(a), node(b)]
        return (f"l{next(names)}", leaf(t))
    top = node(tree)
    return top if isinstance(top, list) else [top]


def wallet_script(template, keys, place):
    """The scriptPubKey the wallet `template` over `keys` pays at
    `place` (its chain and index), or None for a template this reference
    does not build."""
    pubs = [x.derive(place).pub for _, _, x in keys]

    def fill(t, tap):
        for i in reversed(range(len(pubs))):
            t = t.replace(f"@{i}/**", (pubs[i][1:] if tap else pubs[i]).hex())
        return t
    if template.startswith("tr("):
        internal, tree = split_top(template[3:-1])
        ik = fill(internal, True)
        if len(ik) != 64:
            return None

        def leaf(t):
            if t.startswith("sortedmulti_a("):
                k, rest = t[len("sortedmulti_a("):-1].split(",", 1)
                xs = sorted(bytes.fromhex(fill(x, True)) for x in rest.split(","))
                return SCRIPTS.add(f"multi_a({k},{','.join(x.hex() for x in xs)})", True)
            return SCRIPTS.add(fill(t, True), True)
        info = S.taproot_construct(bytes.fromhex(ik), tap_items(tree, leaf))
        return bytes(info.scriptPubKey)
    for head, wrap in (("sh(wsh(", lambda sc: p2sh(p2wsh(sc))), ("wsh(", p2wsh),
                       ("sh(", p2sh)):
        if template.startswith(head):
            inner = template[len(head):-head.count("(")]
            break
    else:
        return None
    multi = re.fullmatch(r"(sorted)?multi\((\d+),(.*)\)", inner)
    if multi:
        ks = [pubs[int(i)] for i in re.findall(r"@(\d+)/\*\*", multi.group(3))]
        script = multisig_script(int(multi.group(2)), sorted(ks) if multi.group(1) else ks)
    elif head == "sh(":
        return None
    else:
        script = SCRIPTS.add(fill(inner, False), False)
    return wrap(script)


def wallet_gives(text, inp, masters):
    """Whether the wallet `text` holds a held master's key and gives
    `inp`'s script at a chain and index one of the input's origins names
    under one of its keys: the wallet's script built here at that place
    (`multi`, `wsh` miniscript, or a `tr()` tree through Core's
    `taproot_construct`) and compared."""
    template, keys = read_wallet(text)
    if not holds(keys, masters):
        return False
    spk = bytes(inp.utxo.scriptPubKey)
    for fp, path in input_origins(inp.m):
        for kfp, kpath, _ in keys:
            if (fp == kfp and len(path) == len(kpath) + 2 and path[:-2] == kpath
                    and path[-2] in (0, 1) and path[-1] < HARDENED):
                if wallet_script(template, keys, path[-2:]) == spk:
                    return True
    return False


def tree_whole(inp):
    """The internal key of a taproot input whose carried leaves are its
    whole tree, else None: one internal key and one root from every
    control block, that key tweaked by that root the output spent, and
    every hash beside a leaf a carried leaf or a node built from carried
    leaves."""
    built, beside, top = set(), [], None
    for control, value in of_type(inp.m, 0x15):
        node = tap_leaf_hash(value[:-1], value[-1])
        built.add(node)
        internal = control[1:33]
        for j in range(33, len(control), 32):
            sibling = control[j:j + 32]
            beside.append(sibling)
            node = TaggedHash("TapBranch", min(node, sibling) + max(node, sibling))
            built.add(node)
        if top is None:
            top = (internal, node)
        elif top != (internal, node):
            return None
    if top is None or not all(h in built for h in beside):
        return None
    return top[0] if p2tr(*top)[0] == bytes(inp.utxo.scriptPubKey) else None


def several_keys(inp):
    """Whether an input spends a script of more than one key: a `multi`
    of two or more, a miniscript naming two or more, or a taproot script
    path carrying leaves which, with the internal key unless it is BIP
    341's unspendable point, name two or more between them, any of which
    is no miniscript known here, or which are not the output's whole
    tree. One that carries no leaf spends by its key path, one key."""
    if inp.kind in ("p2wsh-multi", "p2sh-p2wsh-multi", "p2sh-multi"):
        return len(inp.script_keys()) > 1
    if inp.kind in ("p2wsh-ms", "p2sh-p2wsh-ms"):
        node = SCRIPTS.node(inp.ws, False)
        return node is None or len({k.bytes() for k in node.keys}) > 1
    if inp.kind == "tr-script":
        if not of_type(inp.m, 0x15):
            return False
        internal = tree_whole(inp)
        if internal is None:
            return True
        keys = set() if internal.hex() == NUMS_HEX else {internal}
        for _, value in of_type(inp.m, 0x15):
            script, version = value[:-1], value[-1]
            node = SCRIPTS.node(script, True) if version == TAPSCRIPT else None
            if node is None:
                return True
            keys |= {k.bytes()[-32:] for k in node.keys}
        return len(keys) > 1
    return False


# ----- PSBT maps ----------------------------------------------------------------


def parse(raw):
    if raw[:5] != MAGIC:
        raise Refused("not a PSBT")
    g, at = read_map(raw, 5)
    tx = read_unsigned(dict(g)[b"\x00"])
    ins, outs = [], []
    for _ in tx.vin:
        m, at = read_map(raw, at)
        ins.append(m)
    for _ in tx.vout:
        m, at = read_map(raw, at)
        outs.append(m)
    if at != len(raw):
        raise Refused("bytes after the last map")
    return g, ins, outs, tx


def write(g, ins, outs):
    return MAGIC + write_map(g) + b"".join(write_map(m) for m in ins + outs)


def canonical(raw):
    """A PSBT as its maps, each a sorted list of pairs: BIP-174 leaves
    the order of keys to the writer."""
    g, ins, outs, _ = parse(raw)
    return [sorted((k.hex(), v.hex()) for k, v in m) for m in [g] + ins + outs]


def of_type(m, t):
    return [(k[1:], v) for k, v in m if k[0] == t]


def one(m, t):
    got = [v for k, v in m if k == bytes([t])]
    return got[0] if got else None


def origin(v):
    fp, path = v[:4], v[4:]
    return fp, [int.from_bytes(path[i:i + 4], "little") for i in range(0, len(path), 4)]


def tap_origin(v):
    n, at = compact(v, 0)
    leaves = [v[at + 32 * j:at + 32 * j + 32] for j in range(n)]
    fp, path = origin(v[at + 32 * n:])
    return leaves, fp, path


def ser_origin(fp, path):
    return fp + b"".join(i.to_bytes(4, "little") for i in path)


def read_txout(v):
    out = CTxOut()
    out.deserialize(io.BytesIO(v))
    return out


def read_prev(v):
    tx = CTransaction()
    tx.deserialize(io.BytesIO(v))
    return tx


def b64(raw):
    return base64.b64encode(raw).decode()


# ----- scripts ------------------------------------------------------------------


def p2pkh(h):
    return b"\x76\xa9\x14" + h + b"\x88\xac"


def p2wpkh(h):
    return b"\x00\x14" + h


def p2sh(script):
    return b"\xa9\x14" + btc.hash160(script) + b"\x87"


def p2wsh(script):
    return b"\x00\x20" + btc.sha256(script)


def multisig_script(m, keys):
    return bytes(S.CScript([m] + list(keys) + [len(keys), S.OP_CHECKMULTISIG]))


def parse_multisig(script):
    """`OP_m <key>… OP_n OP_CHECKMULTISIG`, or None."""
    try:
        ops = list(S.CScript(script))
    except S.CScriptInvalidError:
        return None
    if len(ops) < 4 or ops[-1] != S.OP_CHECKMULTISIG:
        return None
    # Core's CScript yields OP_1..OP_16 as plain integers.
    m, n, keys = ops[0], ops[-2], ops[1:-2]
    if not all(isinstance(k, bytes) and len(k) in (33, 65) for k in keys):
        return None
    if type(m) is not int or type(n) is not int:
        return None
    if not 1 <= m <= n or n != len(keys):
        return None
    return m, keys


def tap_leaf_hash(script, version):
    return TaggedHash("TapLeaf", bytes([version]) + ser_string(script))


def p2tr(internal, merkle):
    tweak = TaggedHash("TapTweak", internal + (merkle or b""))
    q = S.tweak_add_pubkey(internal, tweak)
    return b"\x51\x20" + q[0], tweak


# ----- the inputs, and Faraday's refusals (ours) --------------------------------


class Input:
    """One input as a signer reads it."""

    def __init__(self, m, tx, i):
        self.m, self.i = m, i
        self.final = one(m, 0x07) is not None or one(m, 0x08) is not None
        self.utxo = None
        prev = one(m, 0x00)
        if prev is not None:
            p = read_prev(prev)
            if p.txid_int != tx.vin[i].prevout.hash:
                raise Refused("non_witness_utxo is not the transaction spent")
            if tx.vin[i].prevout.n >= len(p.vout):
                raise Refused("non_witness_utxo has no such output")
            self.utxo = p.vout[tx.vin[i].prevout.n]
        w = one(m, 0x01)
        if w is not None:
            wo = read_txout(w)
            if self.utxo is not None and wo.serialize() != self.utxo.serialize():
                raise Refused("witness_utxo and non_witness_utxo disagree")
            if self.utxo is None:
                self.utxo = wo
        self.has_prev = prev is not None
        self.redeem = one(m, 0x04)
        self.ws = one(m, 0x05)
        self.kind = self.classify() if self.utxo is not None and not self.final else None

    def classify(self):
        spk = bytes(self.utxo.scriptPubKey)
        if len(spk) == 25 and spk[:3] == b"\x76\xa9\x14" and spk[23:] == b"\x88\xac":
            return "p2pkh"
        if len(spk) == 22 and spk[:2] == b"\x00\x14":
            return "p2wpkh"
        if len(spk) == 34 and spk[:2] == b"\x00\x20":
            return self.witness_kind(spk, "p2wsh")
        if len(spk) == 23 and spk[:2] == b"\xa9\x14" and spk[22] == 0x87:
            if self.redeem is None or p2sh(self.redeem) != spk:
                raise Refused("p2sh without its redeem script")
            r = self.redeem
            if len(r) == 22 and r[:2] == b"\x00\x14":
                return "p2sh-p2wpkh"
            if len(r) == 34 and r[:2] == b"\x00\x20":
                return self.witness_kind(r, "p2sh-p2wsh")
            if parse_multisig(r):
                return "p2sh-multi"
            raise Refused("an unknown redeem script")
        if len(spk) == 34 and spk[:2] == b"\x51\x20":
            script_path = (one(self.m, 0x18) is not None or of_type(self.m, 0x15)
                           or of_type(self.m, 0x14)
                           or any(tap_origin(v)[0] for _, v in of_type(self.m, 0x16)))
            return "tr-script" if script_path else "tr-key"
        raise Refused("an unknown script")

    def witness_kind(self, program, wrapper):
        if self.ws is None or p2wsh(self.ws) != program:
            raise Refused("p2wsh without its witness script")
        if parse_multisig(self.ws):
            return wrapper + "-multi"
        return wrapper + "-ms"

    def node(self):
        node = SCRIPTS.node(self.ws, False)
        if node is None:
            raise Refused("a witness script whose miniscript is not known")
        return node

    def script_keys(self):
        if self.kind in ("p2wsh-multi", "p2sh-p2wsh-multi"):
            return parse_multisig(self.ws)[1]
        if self.kind == "p2sh-multi":
            return parse_multisig(self.redeem)[1]
        if self.kind in ("p2wsh-ms", "p2sh-p2wsh-ms"):
            return [k.bytes() for k in self.node().keys]
        return []

    def key_in_script(self, pk):
        spk = bytes(self.utxo.scriptPubKey)
        if self.kind == "p2pkh":
            return spk == p2pkh(btc.hash160(pk))
        if self.kind == "p2wpkh":
            return len(pk) == 33 and spk == p2wpkh(btc.hash160(pk))
        if self.kind == "p2sh-p2wpkh":
            return len(pk) == 33 and self.redeem == p2wpkh(btc.hash160(pk))
        return pk in self.script_keys()

    def ecdsa_sighash(self, tx, ht):
        """The ECDSA signature hash, Core's `script.py` over the script
        code chosen here: for the first RFC 6979 nonce's check."""
        amount = self.utxo.nValue
        if self.kind == "p2pkh":
            h, _ = S.LegacySignatureHash(S.CScript(bytes(self.utxo.scriptPubKey)), tx, self.i, ht)
            return h
        if self.kind == "p2sh-multi":
            h, _ = S.LegacySignatureHash(S.CScript(self.redeem), tx, self.i, ht)
            return h
        if self.kind in ("p2wpkh", "p2sh-p2wpkh"):
            program = bytes(self.utxo.scriptPubKey) if self.kind == "p2wpkh" else self.redeem
            code = p2pkh(program[2:22])
            return S.SegwitV0SignatureHash(S.CScript(code), tx, self.i, ht, amount)
        return S.SegwitV0SignatureHash(S.CScript(self.ws), tx, self.i, ht, amount)


def prevouts(inputs):
    outs = []
    for inp in inputs:
        if inp.utxo is None:
            raise Refused("an input with no previous output")
        outs.append(inp.utxo)
    return outs


def tap_sighash(tx, inputs, i, ht, leaf=None, version=TAPSCRIPT):
    if ht not in TAPROOT:
        raise Refused("not a taproot sighash type")
    if ht & 3 == 3 and i >= len(tx.vout):
        raise Refused("SIGHASH_SINGLE without an output")
    return S.TaprootSignatureHash(tx, prevouts(inputs), ht, i, scriptpath=leaf is not None,
                                  leaf_script=S.CScript(leaf) if leaf is not None else None,
                                  codeseparator_pos=0xFFFFFFFF, leaf_ver=version)


def leaf_keys(script, version):
    """The keys a leaf checks signatures against, read through
    python-bip380; nothing when it is no miniscript known here."""
    node = SCRIPTS.node(script, True) if version == TAPSCRIPT else None
    return [] if node is None else [k.bytes() for k in node.keys]


def takes_part(inp, masters, wallets):
    """Whether a held master takes part in `inp` as Faraday's inspection
    counts it: a single-key or multisig input whose origins name one, a
    key-path input whose key origin does, and a miniscript or a taproot
    tree only through a wallet in use whose script it is, holding a held
    key (F135, F136's `NoParticipatingKey`): another wallet registered
    beside it does not make it the person's."""
    if inp.kind is None:
        return False
    ecdsa = any(origin(v)[0] in masters for _, v in of_type(inp.m, 0x06))
    tap = [tap_origin(v) for _, v in of_type(inp.m, 0x16)]
    if inp.kind in ("p2pkh", "p2wpkh", "p2sh-p2wpkh", "p2sh-multi", "p2wsh-multi",
                    "p2sh-p2wsh-multi"):
        return ecdsa
    if inp.kind == "tr-key":
        return any(fp in masters for ls, fp, _ in tap if not ls)
    return ((ecdsa or any(fp in masters for _, fp, _ in tap))
            and any(wallet_gives(w, inp, masters) for w in wallets))


# The kinds signed only through a registered wallet whose script the input
# is, whatever the number of their keys (F135, F141).
THROUGH_WALLET = ("p2wsh-ms", "p2sh-p2wsh-ms", "tr-script")


def policy(raw, masters, wallets=()):
    """Costs a BIP-32 derivation for each held key origin: Faraday's
    refusals and the inputs it leaves unsigned, ours; Core signs every
    one of these. Raises Known (counted, its kind named by
    Faraday's refusal) or Refused; otherwise the inputs, and the indexes
    of those a miniscript or a taproot tree leaves unsigned without its
    wallet (F141)."""
    g, ins, outs, tx = parse(raw)
    inputs = [Input(m, tx, i) for i, m in enumerate(ins)]
    for inp in inputs:
        if inp.final or inp.kind is None or not several_keys(inp):
            continue
        named = any(fp in masters for fp, _ in input_origins(inp.m))
        if named and not any(wallet_gives(w, inp, masters) for w in wallets):
            raise Known("an input of several keys whose wallet is not registered",
                        kind="WalletNotRegistered")
    if len(inputs) > 1:
        for inp in inputs:
            spk = bytes(inp.utxo.scriptPubKey) if inp.utxo is not None else b""
            taproot = len(spk) == 34 and spk[:2] == b"\x51\x20"
            if inp.final and inp.utxo is not None and not inp.has_prev and not taproot:
                raise Known("a final input with no previous transaction beside another input")
    skip = set()
    for inp in inputs:
        if not inp.final and not input_checks(inp, inputs, tx, masters, wallets):
            skip.add(inp.i)
    # Where Core signs, the dangers no hold accepts (F136), each named by
    # the kind Faraday's refusal must give.
    for inp in inputs:
        ht_field = one(inp.m, 0x03)
        if ht_field and int.from_bytes(ht_field, "little") in OPEN:
            raise Known("a NONE or SINGLE sighash, which no hold accepts", "UnusualSighash")
    if not any(takes_part(inp, masters, wallets) for inp in inputs):
        raise Known("none of the held keys can sign", "NoParticipatingKey")
    return inputs, tx, skip


def input_checks(inp, inputs, tx, masters, wallets):
    """Costs a BIP-32 derivation for each held key origin: Faraday's
    refusals of one input, ours; False when the input is left unsigned
    (F141)."""
    m, i = inp.m, inp.i
    ecdsa_claims = [(k, origin(v)) for k, v in of_type(m, 0x06) if origin(v)[0] in masters]
    tap = [(k, tap_origin(v)) for k, v in of_type(m, 0x16)]
    key_claims = [(k, fp, p) for k, (ls, fp, p) in tap if not ls and fp in masters]
    leaf_claims = [(k, ls, fp, p) for k, (ls, fp, p) in tap if ls and fp in masters]
    if not ecdsa_claims and not key_claims and not leaf_claims:
        return True
    if inp.utxo is None:
        raise Refused("no previous output")
    if inp.kind in ("p2wpkh", "p2sh-p2wpkh", "p2wsh-multi", "p2sh-p2wsh-multi", "p2wsh-ms",
                    "p2sh-p2wsh-ms") and not inp.has_prev and len(inputs) > 1:
        raise Known("a SegWit v0 input with no previous transaction beside another input")
    if inp.kind in THROUGH_WALLET and not any(wallet_gives(w, inp, masters) for w in wallets):
        return False
    ht_field = one(m, 0x03)
    if not inp.kind.startswith("tr-"):
        ht = int.from_bytes(ht_field, "little") if ht_field else 1
        if ht not in STANDARD:
            raise Refused("not a standard sighash type")
        for pk, (fp, path) in ecdsa_claims:
            if masters[fp].derive(path).pub != pk or not inp.key_in_script(pk):
                raise Refused("the key the origin derives is not the one named, or not in the script")
            if ht & 3 == 3 and i >= len(tx.vout):
                raise Refused("SIGHASH_SINGLE without an output")
        return True
    ht = int.from_bytes(ht_field, "little") if ht_field else 0
    internal = one(m, 0x17)
    spk = bytes(inp.utxo.scriptPubKey)
    if inp.kind == "tr-key":
        for x, fp, path in key_claims:
            if internal is None:
                raise Refused("taproot input without an internal key")
            if internal != x or p2tr(internal, None)[0] != spk:
                raise Refused("the key path is not this key's")
            if masters[fp].derive(path).pub[1:] != x:
                raise Refused("the key the origin derives is not the one named")
            tap_sighash(tx, inputs, i, ht)
        return True
    signs = False
    for control, value in of_type(m, 0x15):
        script, version = value[:-1], value[-1]
        lh = tap_leaf_hash(script, version)
        names = leaf_keys(script, version)
        for x, leaves, fp, path in leaf_claims:
            if lh not in leaves or x not in names:
                continue
            if masters[fp].derive(path).pub[1:] != x:
                raise Refused("the key the origin derives is not the one named")
            tap_sighash(tx, inputs, i, ht, script, version)
            signs = True
    merkle = one(m, 0x18)
    for x, fp, path in key_claims:
        if internal != x or p2tr(internal, merkle)[0] != spk:
            continue
        if masters[fp].derive(path).pub[1:] != x:
            raise Refused("the key the origin derives is not the one named")
        tap_sighash(tx, inputs, i, ht)
        signs = True
    if not signs:
        raise Refused("no leaf or key path of this taproot input is ours")
    return True


# ----- Bitcoin Core: the signer -------------------------------------------------


def core_key(key):
    """An extended private key as Core on regtest reads it (tprv)."""
    return btc.Key(key.chain, key.k, None, key.depth, key.parent, key.child, test=True).serialize()


def descriptors(ins, masters, leave_out=()):
    """A `pk()` descriptor for each key origin the inputs name under a
    held master, Core to derive the key from the master: but those in
    `leave_out`, (fingerprint, path)."""
    out = set()
    for m in ins:
        for fp, path in input_origins(m):
            if fp in masters and (fp, tuple(path)) not in leave_out:
                out.add(f"pk({core_key(masters[fp])}{btc.path_text(path)[1:]})")
    return sorted(out)


def added_signatures(before, after):
    """The signature pairs `after`'s inputs hold that `before`'s do not."""
    _, b_ins, _, _ = parse(before)
    _, a_ins, _, _ = parse(after)
    out = []
    for bm, am in zip(b_ins, a_ins):
        have = set(bm)
        out.append([(k, v) for k, v in am if k[0] in SIGS and (k, v) not in have])
    return out


SIGHASH_NAMES = {0: "DEFAULT", 1: "ALL", 2: "NONE", 3: "SINGLE", 0x81: "ALL|ANYONECANPAY",
                  0x82: "NONE|ANYONECANPAY", 0x83: "SINGLE|ANYONECANPAY"}


def core_sign(raw, masters):
    """Costs a request to Bitcoin Core (descriptorprocesspsbt, once for
    each sighash type the inputs ask for, and again for each when a
    taproot input's internal key is held beside its leaves): the
    signature pairs Core adds to each input with the held masters' keys.

    Core signs every input of a PSBT under the one sighash type it is
    asked for, and refuses a PSBT any input of which says another, by
    its field or by a signature already on it: each call asks for one
    type, with the other inputs' fields and every signature already made
    taken off, and keeps the signatures of the inputs that asked for it
    (no field is Core's DEFAULT: ALL for ECDSA, DEFAULT for taproot).
    Core signs a taproot key path alone where it can, so the leaves
    beside one are taken from a call without the internal key."""
    g, ins, outs, _ = parse(raw)
    internal = set()
    beside = set()
    for j, m in enumerate(ins):
        x = one(m, 0x17)
        if x is None or not of_type(m, 0x15):
            continue
        for k, v in of_type(m, 0x16):
            leaves, fp, path = tap_origin(v)
            if k == x and not leaves and fp in masters:
                internal.add((fp, tuple(path)))
                beside.add(j)
    asks = []
    for m in ins:
        field = one(m, 0x03)
        asks.append(int.from_bytes(field, "little") if field is not None else 0)
    calls, which = [], []
    for ht in sorted(set(asks)):
        if ht not in SIGHASH_NAMES:
            continue
        variant = write(g, [[(k, v) for k, v in m
                             if k[0] not in SIGS and (a == ht or k != b"\x03")]
                            for m, a in zip(ins, asks)], outs)
        for without in ((), internal) if beside else ((),):
            calls.append(("descriptorprocesspsbt", b64(variant),
                          descriptors(ins, masters, without), SIGHASH_NAMES[ht], False, False))
            which.append((ht, bool(without), variant))
    answers = core_node.batch(calls)
    added = [[] for _ in ins]
    for (ht, leaves_only, variant), answer in zip(which, answers):
        got = added_signatures(variant, base64.b64decode(answer["psbt"]))
        for j, pairs in enumerate(got):
            if asks[j] != ht:
                continue
            if j in beside:
                pairs = [p for p in pairs if (p[0][0] == 0x14) == leaves_only]
            elif leaves_only:
                continue
            added[j] += pairs
    return added


def held_key(m, x, masters, xonly):
    """Costs a BIP-32 derivation: the held master's private key the
    input's origin for `x` names."""
    for k, v in of_type(m, 0x16 if xonly else 0x06):
        if k == x:
            fp, path = tap_origin(v)[1:] if xonly else origin(v)
            if fp in masters:
                return masters[fp].derive(path)
    raise AssertionError(f"Core signed for {x.hex()}, which no held origin names")


def expected_value(inp, inputs, tx, key, value, nonce, masters):
    """Costs up to six elliptic-curve multiplications: the signature
    Faraday must have made where Core signed `key` with `value`. ECDSA:
    Core's own under the low-R rule; under the first RFC 6979 nonce,
    Core's `key.py`'s, once Core's signature verifies under the hash
    computed here; either one is also checked by python-ecdsa (the first
    nonce byte for byte, the other verified). BIP-340: BIP-340's
    reference with zero auxiliary randomness, once Core's signature
    verifies under the hash computed here; btclib verifies it and makes it
    again byte for byte."""
    m, i = inp.m, inp.i
    ht_field = one(m, 0x03)
    if key[0] == 0x02:
        pk = key[1:]
        ht = int.from_bytes(ht_field, "little") if ht_field else 1
        if value[-1] != ht:
            raise AssertionError("Core's ECDSA signature carries another sighash byte")
        h = inp.ecdsa_sighash(tx, ht)
        if nonce == "low_r":
            lineage.check_ecdsa(held_key(m, pk, masters, False).k, pk, h, value[:-1], nonce)
            return value
        pub = ECPubKey()
        pub.set(pk)
        if not pub.verify_ecdsa(value[:-1], h):
            raise AssertionError("Core's ECDSA signature does not verify under the hash here")
        d = held_key(m, pk, masters, False).k
        ec = ECKey()
        ec.set(d.to_bytes(32, "big"), True)
        sig = ec.sign_ecdsa(h, low_s=True, rfc6979=True) + bytes([ht])
        r_len = sig[3]
        if r_len < 33 and sig != value:
            raise AssertionError("the first RFC 6979 nonce gave a low R, and not Core's signature")
        lineage.check_ecdsa(d, pk, h, sig[:-1], nonce)
        return sig
    ht = int.from_bytes(ht_field, "little") if ht_field else 0
    suffix = bytes([ht]) if ht else b""
    if value[64:] != suffix:
        raise AssertionError("Core's BIP-340 signature carries another sighash byte")
    spk = bytes(inp.utxo.scriptPubKey)
    if key[0] == 0x13:
        x = one(m, 0x17)
        tweak = p2tr(x, one(m, 0x18))[1]
        seckey = tweak_add_privkey(held_key(m, x, masters, True).k.to_bytes(32, "big"), tweak)
        msg = tap_sighash(tx, inputs, i, ht)
        signer = spk[2:]
    else:
        x, lh = key[1:33], key[33:]
        leaf = [v for _, v in of_type(m, 0x15) if tap_leaf_hash(v[:-1], v[-1]) == lh]
        if not leaf:
            raise AssertionError("Core signed under a leaf the input does not carry")
        seckey = held_key(m, x, masters, True).k.to_bytes(32, "big")
        msg = tap_sighash(tx, inputs, i, ht, leaf[0][:-1], leaf[0][-1])
        signer = x
    if not schnorr_verify(msg, signer, value[:64]):
        raise AssertionError("Core's BIP-340 signature does not verify under the hash here")
    mine = schnorr_sign(msg, seckey, bytes(32))
    lineage.check_schnorr(seckey, msg, signer, mine)
    return mine + suffix


# ----- Bitcoin Core: the finalizer and the mempool -------------------------------


def pkh_leaves(m):
    """The leaf hashes of an input's carried leaves that check a key by
    its hash (python-bip380), which Faraday leaves unfinalized."""
    out = set()
    for _, value in of_type(m, 0x15):
        script, version = value[:-1], value[-1]
        node = SCRIPTS.node(script, True) if version == TAPSCRIPT else None
        if node is not None and ms.has_pkh(node):
            out.add(tap_leaf_hash(script, version))
    return out


def without_pkh(raw):
    """The PSBT with the signatures under each `pkh` leaf taken off."""
    g, ins, outs, _ = parse(raw)
    kept = []
    for m in ins:
        drop = pkh_leaves(m)
        kept.append([(k, v) for k, v in m if not (k[0] == 0x14 and k[33:] in drop)])
    return write(g, kept, outs)


def core_finalize(raw):
    """Costs a request to Bitcoin Core (finalizepsbt, twice or three
    times): what Faraday's finalizer must answer for `raw`, its PSBT and
    its transaction, with whether Core would have finalized more but for
    the signatures under a `pkh` leaf taken off. An input Core finalizes
    keeps the fields BIP-174 keeps (the UTXOs and unknown types) and
    Core's final scriptSig and witness; any other keeps its pairs."""
    stripped = without_pkh(raw)
    calls = [("finalizepsbt", b64(stripped), False), ("finalizepsbt", b64(stripped), True)]
    if stripped != raw:
        calls.append(("finalizepsbt", b64(raw), False))
    answers = core_node.batch(calls)
    g, ins, outs, _ = parse(raw)
    _, core_ins, _, _ = parse(base64.b64decode(answers[0]["psbt"]))
    final = []
    for m, cm in zip(ins, core_ins):
        got = [(k, v) for k, v in cm if k[0] in (0x07, 0x08)]
        if got and not any(k[0] in (0x07, 0x08) for k, _ in m):
            final.append([(k, v) for k, v in m if k[0] in (0x00, 0x01) or k[0] not in DEFINED]
                         + got)
        else:
            final.append(m)
    more = False
    if len(answers) == 3:
        _, whole, _, _ = parse(base64.b64decode(answers[2]["psbt"]))
        more = [[k for k, _ in x if k[0] in (0x07, 0x08)] for x in whole] != \
            [[k for k, _ in x if k[0] in (0x07, 0x08)] for x in core_ins]
    complete = answers[1]["complete"]
    return {"psbt": write(g, final, outs).hex(), "tx": answers[1]["hex"] if complete else None,
            "complete": complete, "more": more}


def core_accepts(tx_hex):
    """Costs a request to Bitcoin Core (testmempoolaccept): None when Core
    would take the transaction into its mempool, else its reason."""
    got = core_node.batch([("testmempoolaccept", [tx_hex])])[0][0]
    return None if got.get("allowed") else got.get("reject-reason", "refused")


# ----- the area -----------------------------------------------------------------


class Sign(Area):
    name = "sign"
    faraday = (
        '`faraday_psbt::sign` (every held key selected, deterministic Schnorr, both RFC '
        '6979 nonce rules) and `faraday_psbt::finalize`'
    )
    reference = (
        "Bitcoin Core v31.1 on regtest (`core_node.py`): `descriptorprocesspsbt` signs from "
        "the held masters' keys, `finalizepsbt` finalizes and extracts, `testmempoolaccept` "
        "judges each finalized transaction over its funding on the node; BIP-340's "
        "reference (`secp256k1lab`) and Core's `key.py` for the deterministic nonces, over "
        "Core's `script.py` sighashes; python-ecdsa (the distribution's package) and btclib "
        "(`btclib/`, its own curve) as a second curve lineage, beside them: every ECDSA "
        "signature again under plain RFC 6979 or verified, every BIP-340 signature verified "
        "and made again; python-bip380 (`miniscript/`) reads the leaves. "
        "Faraday's refusals and the inputs a wallet's rule leaves unsigned are ours, in "
        "`area_sign.py`"
    )
    reference_checked = (
        'Core is the pinned release, hashed against its pin before every run; the vendored '
        "files are hashed against `references.sha256`; a published vector's answer is "
        "required of Core as well as of Faraday; every ECDSA and BIP-340 signature Core "
        'makes is checked to verify under the hash the area computes before that hash is '
        "used for Faraday's deterministic nonces"
    )
    vectors = (
        "BIP-174's signer, finalizer and extractor vectors; the Core-built spends of "
        '`tools/vectors/psbt/` (2-of-3 `sortedmulti`, the Liana wallet down both paths, '
        "a taproot tree by a leaf and by the key path, BIP 387's taproot multisig), "
        "signed by every subset of their keys; the finalizer vectors there (`or_b`'s smaller "
        'signature, the taproot leaves that tie by count, the taproot multisig and MuSig2 '
        'spends)'
    )
    corpus = (
        'PSBTs of one to four inputs, each spending a funding output on the node: P2PKH, '
        'P2SH-P2WPKH, P2WPKH, `multi`/`sortedmulti` in P2SH, P2WSH and P2SH-P2WSH, P2TR key '
        'path, P2TR script path (`pk`, `pkh`, timelocked keys, `or_d`, `multi_a`) with the '
        "key path beside it, miniscript in P2WSH and P2SH-P2WSH; held and foreign keys, "
        "cosigners' signatures already on (Core's), every standard sighash type, timelocks "
        "met and not; Core's signatures (ECDSA byte for byte, BIP-340 by verification and the "
        "deterministic nonce), each final scriptSig and witness and the extracted transaction "
        "Core's, the cleared fields, and every finalized transaction accepted by Core's "
        'mempool; an origin that derives another key refused'
    )
    differences = [
        (
            'PSBT signing: Faraday refuses to sign a SegWit v0 input with no previous '
            "transaction in a transaction of more than one input; Core signs it, and "
            "BIP-174's own signer vector is such a PSBT. The refusal is ours, checked and"
            ' counted. Faraday holds a finalized input to the same rule and refuses the '
            'PSBT when such an input is final, except a taproot one, whose final witness is'
            " the coordinator's."
        ),
        (
            'PSBT signing, wallets: Faraday signs an input spent from a script of several '
            'keys (a `multi` of two or more, a miniscript naming two or more, a taproot '
            'script path whose carried leaves name two or more with its internal key, or '
            'are not its whole tree) only for a registered wallet that holds one of its '
            "keys and whose script it is; Core signs any. The area refuses, by a rule of "
            'its own, where no wallet the request registers holds a held key and gives the '
            "input, and counts it only where Faraday's refusal is that the wallet is not "
            'registered. A miniscript or a taproot tree of one key is signed only through '
            'such a wallet, and is otherwise left unsigned while the rest is signed (F141);'
            " the area takes Core's signatures off it. Half the corpus's `multi`, "
            'miniscript and taproot-tree inputs come with their wallet registered, so their'
            " signatures are still compared. A registered tree's every leaf has its wait "
            'met, since Faraday refuses a path that can never be valid, which the area does'
            ' not work out.'
        ),
        (
            'PSBT signing: Faraday refuses a PSBT any input of which asks for a `NONE` or '
            '`SINGLE` sighash, and one that none of the held keys can sign: dangers no hold'
            " accepts (F136). Core signs the first and answers the second with nothing "
            'signed. The area expects both refusals by a rule of its own, counting a held '
            "key as taking part as Faraday's inspection does (a miniscript or a taproot tree "
            "only through a wallet in use), checks that Faraday's refusal names the danger, "
            'and counts them.'
        ),
        (
            'Taproot `pkh` leaves: Faraday signs a taproot leaf that checks its key by hash'
            " but does not finalize it: `miniscript` 12's finalizer looks the key up among "
            'the ECDSA origins and panics, and would push 33 bytes where the leaf hashed '
            "32. Core finalizes such a leaf; the area takes the signatures under it off "
            'before Core finalizes, and counts each input Core would otherwise have '
            'finalized another way.'
        ),
    ]
    not_compared = (
        'none'
    )
    CORE = True
    FUND = {"corpus": 4}
    # Past the longest relative timelock the corpus spends with (`LOCKS`
    # blocks and up to 50 more).
    FUND_DEPTH = LOCKS + 100

    def __init__(self, *a):
        super().__init__(*a)
        self.known = 0
        self.real = False
        self.collecting = False
        self.finals = {}

    def sign(self, raw, seeds, xprvs, network, nonce, masters, wallets=()):
        inputs = {"psbt": raw.hex(), "seeds": [s.hex() for s in seeds], "xprvs": xprvs,
                  "network": network, "nonce": nonce, "wallets": list(wallets)}
        answer = self.rust.call("sign.sign", **inputs)
        self.compared += 1
        named = None
        try:
            ref = ("ok", self.ref_sign(raw, masters, nonce, wallets))
        except Known as e:
            self.known += 1
            named = e.kind
            ref = ("err", f"Known: {e}")
        except Refused as e:
            ref = ("err", f"Refused: {e}")

        def norm(a):
            if a[0] != "ok":
                return a
            return ("ok", dict(a[1], psbt=canonical(bytes.fromhex(a[1]["psbt"]))))
        got, want = norm(answer), norm(ref)
        if got[0] != want[0] or (got[0] == "ok" and got[1] != want[1]):
            raise Mismatch(self.name, "sign.sign", inputs, answer, ref)
        # A known difference with a warning of its own is Faraday's refusal
        # only where Faraday's refusal names that warning.
        if named and named not in str(answer[1]):
            raise Mismatch(self.name, "sign.sign", inputs, answer, ("err", named),
                           f"the refusal does not name {named}")
        return answer

    def ref_sign(self, raw, masters, nonce, wallets):
        """What Faraday's signer must answer: Faraday's refusals and the
        inputs it leaves unsigned (ours, `policy`), and on the rest the
        signatures Core makes, each as Faraday must have made it."""
        inputs, tx, skip = policy(raw, masters, wallets)
        g, ins, outs, _ = parse(raw)
        added = core_sign(raw, masters)
        out_ins, count = [], 0
        for inp, m, pairs in zip(inputs, ins, added):
            if inp.i in skip or inp.final:
                pairs = []
            mine = [(k, expected_value(inp, inputs, tx, k, v, nonce, masters)) for k, v in pairs]
            count += len(mine)
            out_ins.append(list(m) + mine)
        signed = write(g, out_ins, outs)
        return {"psbt": signed.hex(), "signatures": count,
                "complete": self.core_final(signed)["complete"]}

    def core_final(self, raw):
        """Core's finalizing of `raw`, asked once a case."""
        if raw not in self.finals:
            self.finals[raw] = core_finalize(raw)
        return self.finals[raw]

    def finalize(self, raw):
        theirs = self.rust.call("sign.finalize", psbt=raw.hex())
        self.compared += 1
        try:
            parse(raw)
            core = self.core_final(raw)
        except Refused as e:
            core = None
            ref = ("err", str(e))
        if core is not None:
            ref = ("ok", {"psbt": core["psbt"], "tx": core["tx"]})
            self.known += core["more"]

        def norm(answer):
            if answer[0] != "ok":
                return answer
            return ("ok", {"psbt": canonical(bytes.fromhex(answer[1]["psbt"])),
                           "tx": answer[1]["tx"]})
        a, b = norm(theirs), norm(ref)
        if a[0] != b[0] or (a[0] == "ok" and a != b):
            raise Mismatch(self.name, "sign.finalize", {"psbt": raw.hex()}, theirs, ref)
        if self.real and a[0] == "ok" and theirs[1]["tx"]:
            reason = core_accepts(theirs[1]["tx"])
            self.compared += 1
            if reason:
                raise Mismatch(self.name, "sign.finalize", {"psbt": raw.hex()}, theirs,
                               ("err", reason), "Core's mempool refuses the transaction")
        return theirs

    def round(self, raw, seeds, xprvs, network, masters, nonce=None, wallets=()):
        nonce = nonce or self.rng.choice(["low_r", "first"])
        self.finals = {}
        signed = self.sign(raw, seeds, xprvs, network, nonce, masters, wallets)
        if signed[0] == "ok":
            self.finalize(bytes.fromhex(signed[1]["psbt"]))
        return signed

    # ----- published -------------------------------------------------------------

    def bip174(self):
        recs, cur = [], {}
        for line in (ROOT / "tools/vectors/psbt/bip174-roles.txt").read_text().splitlines():
            if not line.strip():
                if cur:
                    recs.append(cur)
                cur = {}
                continue
            if line.startswith("#"):
                continue
            k, _, v = line.partition(": ")
            cur.setdefault(k, []).append(v)
        if cur:
            recs.append(cur)
        head = recs[0]
        roles = {r["case"][0]: r for r in recs if "case" in r}

        def psbt(name):
            r = roles[f"role {name}"]
            return bytes.fromhex(r["hex"][0]) if "hex" in r else base64.b64decode(r["base64"][0])
        tprv = head["master-tprv"][0]
        master = btc.decode(tprv, True)
        masters = {master.fingerprint(): master}
        # As published: the P2SH-P2WSH input carries no previous
        # transaction, so Faraday refuses the PSBT.
        self.sign(psbt("updater-sighash-all"), [], [tprv], "testnet", "first", masters)
        # With that input final, as the finalizer leaves it, Faraday refuses
        # it too: the final input still carries no previous transaction.
        g, ins, outs, tx = parse(psbt("updater-sighash-all"))
        fin = parse(psbt("finalizer"))
        ins[1] = fin[1][1]
        raw = write(g, ins, outs)
        self.sign(raw, [], [tprv], "testnet", "first", masters)
        # The rest of the chain from the combiner vector, which holds both
        # signatures Bitcoin Core produced.
        self.finals = {}
        done = self.finalize(psbt("combiner"))
        if canonical(bytes.fromhex(done[1]["psbt"])) != canonical(psbt("finalizer")):
            raise Mismatch(self.name, "sign.finalize", {}, done, "BIP-174's finalizer PSBT")
        extracted = roles["role extractor"]["tx"][0]
        if done[1]["tx"] != extracted:
            raise Mismatch(self.name, "sign.finalize", {}, done, extracted,
                           "not BIP-174's extracted transaction")
        self.round(raw, [], [tprv], "testnet", masters, nonce="low_r")

    def finalized(self):
        """The finalizer vectors of `tools/vectors/psbt/`, each finalized by
        Faraday as Core finalizes it, and as the vector's final PSBT
        says where it has one."""
        vectors = ROOT / "tools/vectors/psbt"
        for name in ("finalize-or-b", "finalize-tap-leaf-tie", "wallet-tapmulti-signed",
                     "wallet-musig-device-first-signed"):
            raw = base64.b64decode((vectors / f"{name}.psbt").read_text().strip())
            self.finals = {}
            done = self.finalize(raw)
            final = vectors / f"{name}-final.psbt"
            if final.is_file():
                want = base64.b64decode(final.read_text().strip())
                if canonical(bytes.fromhex(done[1]["psbt"])) != canonical(want):
                    raise Mismatch(self.name, "sign.finalize", {"psbt": raw.hex()}, done,
                                   ("ok", {"psbt": want.hex()}), f"not {final.name}")

    def fixtures(self):
        words = {
            "abandon": " ".join(["abandon"] * 11 + ["about"]),
            "zoo": " ".join(["zoo"] * 11 + ["wrong"]),
            "legal": "legal winner thank year wave sausage worth useful legal winner thank yellow",
        }
        seeds = {n: Mnemonic.to_seed(w, "") for n, w in words.items()}
        keys = {n: btc.master(s, test=True) for n, s in seeds.items()}
        subsets = [["abandon"], ["zoo"], ["legal"], ["abandon", "zoo"], ["zoo", "legal"],
                   ["abandon", "zoo", "legal"]]
        policies = {"wallet-liana": "wallet-liana", "wallet-liana-recovery": "wallet-liana",
                    "wallet-tree": "wallet-tree", "wallet-tree-keypath": "wallet-tree",
                    "wallet-tapmulti-first": "wallet-tapmulti", "wallet-2of3": "wallet-2of3"}
        for name in ["wallet-2of3", "wallet-liana", "wallet-liana-recovery", "wallet-tree",
                     "wallet-tree-keypath", "wallet-tapmulti-first", "demo-regtest"]:
            text = (ROOT / f"tools/vectors/psbt/{name}.psbt").read_text().strip()
            raw = base64.b64decode(text)
            wallets = []
            if name in policies:
                policy_text = (ROOT / f"tools/vectors/psbt/{policies[name]}.policy").read_text()
                wallets = [policy_text]
                if name != "wallet-2of3":
                    register_policy(policy_text, raw)
            for subset in subsets:
                masters = {keys[n].fingerprint(): keys[n] for n in subset}
                for nonce in ("low_r", "first"):
                    self.round(raw, [seeds[n] for n in subset], [], "regtest", masters, nonce,
                               wallets)

    # ----- the random corpus -------------------------------------------------------

    def new_master(self):
        seed = self.randbytes(self.rng.choice([16, 32, 64]))
        return seed, btc.master(seed, test=self.test)

    def key(self, purpose, master=None, script=None):
        """A key at a BIP-44-style path below `master` (a new foreign
        master when None): (pub, fingerprint, path, master)."""
        if master is None:
            master = self.new_master()[1]
        coin = HARDENED + (1 if self.test else 0)
        path = [purpose + HARDENED, coin, self.rng.randrange(0, 3) + HARDENED]
        if script is not None:
            path.append(script + HARDENED)
        path += [self.rng.randrange(0, 2), self.rng.randrange(0, 1 << 20)]
        k = master.derive(path)
        return k.pub, master.fingerprint(), path, master

    def funding(self, spk, value, j=None):
        """The transaction the input spends: from the case's coin `j` on
        the node when the run gave coins (sent before the run, while
        `collecting`), else a made-up one."""
        coin = self.fund_coin(j) if j is not None else None
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
        n = self.rng.randrange(1, 4)
        at = self.rng.randrange(n)
        prev.vout = [CTxOut(self.rng.randrange(1000, 500_000), p2wpkh(self.randbytes(20)))
                     for _ in range(n)]
        prev.vout[at] = CTxOut(value, spk)
        if coin is not None and sum(o.nValue for o in prev.vout) > coin[2]:
            raise HarnessError("a funding transaction pays more than its coin")
        if coin is not None and self.collecting:
            self.funding_txs.append(prev.serialize().hex())
        return prev, at

    def holder(self):
        """Whose key a slot is: one of ours (index into self.ours) or a
        foreign master."""
        if self.rng.random() < 0.6:
            return self.rng.choice(self.ours)[1]
        return None

    def fund_corpus(self):
        self.collecting = True
        try:
            self.corpus_case(fund_only=True)
        finally:
            self.collecting = False

    def corpus_case(self, fund_only=False):
        self.test = self.rng.random() < 0.7
        network = self.rng.choice(["testnet", "regtest", "signet"]) if self.test else "mainnet"
        self.ours = [self.new_master() for _ in range(self.rng.randrange(1, 4))]
        masters = {m.fingerprint(): m for _, m in self.ours}
        tx = CTransaction()
        tx.version = 2
        tx.nLockTime = self.rng.choice([0, self.rng.randrange(1, LOCKS)])
        n_in = self.rng.randrange(1, 5)
        n_out = self.rng.randrange(1, 4)
        ins = []
        total = 0
        presign = {}
        wallets = []
        for i in range(n_in):
            m, spk, value, seq, extra = self.input_spec(i, n_out)
            if "wallet" in extra:
                wallets.append(extra["wallet"])
            prev, at = self.funding(spk, value, i)
            total += value
            tx.vin.append(CTxIn(COutPoint(prev.txid_int, at), b"", seq))
            fields = [(b"\x00", prev.serialize_without_witness())] if extra.get("prev", True) else []
            if extra.get("witness_utxo", True):
                fields.append((b"\x01", CTxOut(value, spk).serialize()))
            ins.append(fields + m)
            presign.update(extra.get("cosigners", {}))
        if fund_only:
            return
        self.real = self.fund_coins is not None
        fee = self.rng.randrange(500, min(20_000, total - n_out * 1000))
        spare = total - fee - n_out * 546
        cuts = sorted(self.rng.randrange(0, spare + 1) for _ in range(n_out - 1))
        amounts = [546 + b - a for a, b in zip([0] + cuts, cuts + [spare])]
        for a in amounts:
            spk = (p2wpkh(self.randbytes(20)) if self.rng.random() < 0.5
                   else b"\x51\x20" + btc.point(self.rng.randrange(1, N))[1:])
            tx.vout.append(CTxOut(a, spk))
        g = [(b"\x00", tx.serialize_without_witness())]
        outs = [[] for _ in tx.vout]
        raw = write(g, ins, outs)
        # Cosigners that already signed, by Core: their signatures are on
        # the PSBT Faraday receives.
        if presign:
            for j, pairs in enumerate(core_sign(raw, presign)):
                ins[j] = ins[j] + pairs
            raw = write(g, ins, outs)
        held = [s for s, _ in self.ours]
        self.round(raw, held, [], network, masters, wallets=wallets)

    def sighash(self, i, n_out, taproot):
        choices = (TAPROOT if taproot else STANDARD)
        r = self.rng.random()
        if r < 0.6:
            return None
        ht = self.rng.choice(choices)
        if ht & 3 == 3 and i >= n_out:
            return None
        return ht

    def ht_field(self, ht):
        return [] if ht is None else [(b"\x03", ht.to_bytes(4, "little"))]

    def input_spec(self, i, n_out):
        kind = self.rng.choice(["p2pkh", "p2wpkh", "p2sh-p2wpkh", "multi", "multi", "tr-key",
                                "tr-tree", "tr-tree", "ms", "ms"])
        value = self.rng.randrange(20_000, 3_000_000)
        seq = self.rng.choice([0xFFFFFFFF, 0xFFFFFFFE, 0xFFFFFFFD])
        extra = {}
        fields = []
        if kind in ("p2pkh", "p2wpkh", "p2sh-p2wpkh"):
            purpose = {"p2pkh": 44, "p2wpkh": 84, "p2sh-p2wpkh": 49}[kind]
            pk, fp, path, _ = self.key(purpose, self.rng.choice(self.ours)[1])
            h = btc.hash160(pk)
            if kind == "p2pkh":
                spk = p2pkh(h)
                extra["witness_utxo"] = False
            elif kind == "p2wpkh":
                spk = p2wpkh(h)
            else:
                fields.append((b"\x04", p2wpkh(h)))
                spk = p2sh(p2wpkh(h))
            fields.append((b"\x06" + pk, ser_origin(fp, path)))
            fields += self.ht_field(self.sighash(i, n_out, False))
            return fields, spk, value, seq, extra
        if kind == "multi":
            n = self.rng.randrange(1, 6)
            need = self.rng.randrange(1, n + 1)
            registered = n > 1 and self.rng.random() < 0.5
            if registered:
                slots, exprs = self.wallet_keys(n)
            else:
                slots = [self.key(48, self.holder(), 2) for _ in range(n)]
            keys = [s[0] for s in slots]
            ordered = self.rng.random() < 0.5
            if ordered:
                keys = sorted(keys)
            script = multisig_script(need, keys)
            wrap = self.rng.choice(["p2sh", "p2wsh", "p2sh-p2wsh"])
            if registered:
                body = f"{'sortedmulti' if ordered else 'multi'}({need},{','.join(exprs)})"
                extra["wallet"] = {"p2sh": f"sh({body})", "p2wsh": f"wsh({body})",
                                   "p2sh-p2wsh": f"sh(wsh({body}))"}[wrap]
            if wrap == "p2sh":
                spk = p2sh(script)
                fields.append((b"\x04", script))
                extra["witness_utxo"] = False
            elif wrap == "p2wsh":
                spk = p2wsh(script)
                fields.append((b"\x05", script))
            else:
                spk = p2sh(p2wsh(script))
                fields += [(b"\x04", p2wsh(script)), (b"\x05", script)]
            for pk, fp, path, _ in slots:
                fields.append((b"\x06" + pk, ser_origin(fp, path)))
            extra["cosigners"] = self.cosigners(slots)
            fields += self.ht_field(self.sighash(i, n_out, False))
            return fields, spk, value, seq, extra
        if kind == "ms":
            return self.ms_input(i, n_out, value, extra)
        if kind == "tr-key":
            pk, fp, path, _ = self.key(86, self.rng.choice(self.ours)[1])
            x = pk[1:]
            spk = p2tr(x, None)[0]
            fields += [(b"\x17", x), (b"\x16" + x, ser_compact(0) + ser_origin(fp, path))]
            fields += self.ht_field(self.sighash(i, n_out, True))
            return fields, spk, value, seq, extra
        return self.tree_input(i, n_out, value, extra)

    def wallet_keys(self, n):
        """`n` keys of a registered wallet at one chain and index, each at
        its own BIP-48 account of a held or a foreign master: the slots
        (pub, fingerprint, path, master) and each key's descriptor
        expression."""
        coin = HARDENED + (1 if self.test else 0)
        chain, index = self.rng.randrange(0, 2), self.rng.randrange(0, 1 << 20)
        slots, exprs, accounts = [], [], set()
        while len(slots) < n:
            master = self.holder() or self.new_master()[1]
            account = self.rng.randrange(0, 1 << 20)
            if (master.fingerprint(), account) in accounts:
                continue
            accounts.add((master.fingerprint(), account))
            head = [48 + HARDENED, coin, account + HARDENED, 2 + HARDENED]
            path = head + [chain, index]
            slots.append((master.derive(path).pub, master.fingerprint(), path, master))
            xpub = master.derive(head).neuter().serialize()
            exprs.append(f"[{master.fingerprint().hex()}/{btc.path_text(head)[2:]}]{xpub}/<0;1>/*")
        return slots, exprs

    def cosigners(self, slots):
        foreign = {fp: m for _, fp, _, m in slots
                   if fp not in {mm.fingerprint() for _, mm in self.ours}}
        chosen = {fp: m for fp, m in foreign.items() if self.rng.random() < 0.5}
        return chosen

    def locks(self, i):
        """A block delay and a height for a timelocked template, both
        within the chain the run's funding is buried in."""
        older = self.rng.randrange(1, LOCKS)
        after = self.rng.randrange(1, LOCKS)
        return older, after

    MS = [
        ("or_d(pk({0}),and_v(v:pkh({1}),older({N})))", 2),
        ("or_i(and_v(v:pkh({1}),older({N})),and_v(v:pk({0}),pk({2})))", 3),
        ("or_d(multi(2,{0},{1},{2}),and_v(v:pkh({3}),older({N})))", 4),
        ("andor(pk({0}),older({N}),pk({1}))", 2),
        ("thresh(2,pk({0}),s:pk({1}),sln:older({N}))", 2),
        ("or_d(pk({0}),and_v(v:multi(2,{1},{2}),after({T})))", 3),
        ("and_v(v:pk({0}),after({T}))", 1),
        ("or_b(pk({0}),s:pk({1}))", 2),
    ]
    LEAVES = [
        ("pk({0})", 1),
        ("and_v(v:pk({0}),older({N}))", 1),
        ("and_v(v:pkh({0}),older({N}))", 1),
        ("or_d(pk({0}),and_v(v:pk({1}),older({N})))", 2),
        ("multi_a({k},{keys})", 0),
    ]

    def timelocked(self, i, older, after, tx_hint=None):
        """A sequence for this input: BIP-68 blocks at, above or below
        `older`, or the final ones."""
        r = self.rng.random()
        if r < 0.4:
            return older + self.rng.randrange(0, 50)
        if r < 0.6:
            return max(0, older - self.rng.randrange(1, older + 1))
        return self.rng.choice([0xFFFFFFFE, 0xFFFFFFFD])

    def ms_input(self, i, n_out, value, extra):
        template, n = self.rng.choice(self.MS)
        registered = n > 1 and self.rng.random() < 0.5
        if registered:
            slots, exprs = self.wallet_keys(n)
        else:
            slots = [self.key(48, self.holder(), 2) for _ in range(n)]
        older, after = self.locks(i)
        text = template.format(*[s[0].hex() for s in slots], N=older, T=after)
        script = SCRIPTS.add(text, False)
        fields = []
        native = self.rng.random() < 0.5
        if native:
            spk = p2wsh(script)
            fields.append((b"\x05", script))
        else:
            spk = p2sh(p2wsh(script))
            fields += [(b"\x04", p2wsh(script)), (b"\x05", script)]
        if registered:
            body = template.format(*exprs, N=older, T=after)
            extra["wallet"] = f"wsh({body})" if native else f"sh(wsh({body}))"
        for pk, fp, path, _ in slots:
            fields.append((b"\x06" + pk, ser_origin(fp, path)))
        extra["cosigners"] = self.cosigners(slots)
        fields += self.ht_field(self.sighash(i, n_out, False))
        seq = self.timelocked(i, older, after)
        return fields, spk, value, seq, extra

    def tree_input(self, i, n_out, value, extra):
        count = self.rng.randrange(1, 5)
        # Half the trees come with their wallet registered (F141): every
        # key of it, the internal key too unless that is NUMS, a held or a
        # foreign account's key at one chain and index, so the wallet
        # gives the input.
        registered = self.rng.random() < 0.5
        shapes = []
        for _ in range(count):
            template, n = self.rng.choice(self.LEAVES)
            older, after = self.locks(i)
            need = None
            if n == 0:
                n = self.rng.randrange(1, 4)
                need = self.rng.randrange(1, n + 1)
            shapes.append((template, n, need, older))
        r = self.rng.random()
        keyed = registered and r < 0.7
        total = sum(n for _, n, _, _ in shapes) + (1 if keyed else 0)
        if registered:
            pool, exprs = self.wallet_keys(total)
        else:
            pool, exprs = [self.key(87, self.holder()) for _ in range(total)], [None] * total
        at = 1 if keyed else 0
        leaves, bodies = [], []
        for template, n, need, older in shapes:
            slots, names = pool[at:at + n], exprs[at:at + n]
            at += n
            hexes = [s[0][1:].hex() for s in slots]
            if need is not None:
                text = template.format(k=need, keys=",".join(hexes))
                body = template.format(k=need, keys=",".join(names)) if registered else None
            else:
                text = template.format(*hexes, N=older)
                body = template.format(*names, N=older) if registered else None
            leaves.append((SCRIPTS.add(text, True), slots, older))
            bodies.append(body)
        # The internal key: one of ours, a foreign key, or BIP-341's NUMS;
        # a registered tree's is one of its wallet's keys, or NUMS.
        nums = (b"\x02" + bytes.fromhex(NUMS_HEX), None, None, None)
        if keyed:
            ik, ik_expr = pool[0], exprs[0]
        elif registered:
            ik, ik_expr = nums, NUMS_HEX
        elif r < 0.4:
            ik = self.key(86, self.rng.choice(self.ours)[1])
        elif r < 0.7:
            ik = self.key(86, None)
        else:
            ik = nums
        internal = ik[0][1:]
        items = self.items(len(leaves), [(f"l{j}", s) for j, (s, _, _) in enumerate(leaves)])
        if registered:
            def text_of(item):
                if isinstance(item, list):
                    return "{" + ",".join(text_of(x) for x in item) + "}"
                return bodies[int(item[0][1:])]
            tree = text_of(items[0]) if len(items) == 1 else text_of(items)
            extra["wallet"] = f"tr({ik_expr},{tree})"
        info = taproot_construct_safe(internal, items)
        fields = [(b"\x17", internal), (b"\x18", info.merkle_root)]
        origins = {}
        for j, (script, slots, _) in enumerate(leaves):
            leaf = info.leaves[f"l{j}"]
            control = bytes([TAPSCRIPT | info.negflag]) + internal + leaf.merklebranch
            fields.append((b"\x15" + control, script + bytes([TAPSCRIPT])))
            for pk, fp, path, _ in slots:
                entry = origins.setdefault(pk[1:], [[], fp, path])
                entry[0].append(leaf.leaf_hash)
        if ik[1] is not None:
            origins.setdefault(internal, [[], ik[1], ik[2]])
        for x, (lhs, fp, path) in origins.items():
            fields.append((b"\x16" + x, ser_compact(len(lhs)) + b"".join(lhs) + ser_origin(fp, path)))
        fields += self.ht_field(self.sighash(i, n_out, True))
        slots_all = [s for _, slots, _ in leaves for s in slots]
        extra["cosigners"] = self.cosigners(slots_all)
        if registered:
            # Every leaf's wait met: with its wallet in use Faraday refuses
            # a path that can never be valid (TimelockNotMet, F136), which
            # this reference does not work out.
            seq = max(o for _, _, o in leaves) + self.rng.randrange(0, 50)
        else:
            older = min(o for _, _, o in leaves)
            seq = self.timelocked(i, older, 0)
        return fields, bytes(info.scriptPubKey), value, seq, extra

    def items(self, count, leaves):
        def build(lo, hi):
            if hi - lo == 1:
                return leaves[lo]
            mid = self.rng.randrange(lo + 1, hi)
            return [build(lo, mid), build(mid, hi)]
        top = build(0, count)
        return [top] if count == 1 else top

    def mismatched(self):
        """An origin naming a held master at a path that derives another
        key: refused by both."""
        self.test = True
        seed, master = self.new_master()
        self.ours = [(seed, master)]
        pk, fp, path, _ = self.key(84, master)
        wrong = list(path)
        wrong[-1] ^= 1
        spk = p2wpkh(btc.hash160(pk))
        prev, at = self.funding(spk, 50_000)
        tx = CTransaction()
        tx.vin = [CTxIn(COutPoint(prev.txid_int, at), b"", 0xFFFFFFFD)]
        tx.vout = [CTxOut(40_000, p2wpkh(self.randbytes(20)))]
        ins = [[(b"\x00", prev.serialize_without_witness()), (b"\x01", CTxOut(50_000, spk).serialize()),
                (b"\x06" + pk, ser_origin(fp, wrong))]]
        raw = write([(b"\x00", tx.serialize_without_witness())], ins, [[]])
        self.finals = {}
        self.sign(raw, [seed], [], "testnet", "low_r", {fp: master})

    LOOPS = (("mismatched", 8), ("corpus", 120))

    def prelude(self):
        self.bip174()
        self.finalized()
        self.fixtures()

    def case_mismatched(self):
        self.real = False
        self.mismatched()

    def case_corpus(self):
        self.real = False
        self.corpus_case()


def taproot_construct_safe(internal, items):
    return S.taproot_construct(internal, items)
