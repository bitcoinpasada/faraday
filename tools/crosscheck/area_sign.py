"""PSBT signing and finalizing end to end: `faraday_psbt::sign` and
`faraday_psbt::finalize` against a BIP-174 signer and finalizer written
here from BIP-174's and BIP-371's text, over Bitcoin Core's
functional-test framework (`core/test_framework/`: `script.py`'s three
signature hashes and `taproot_construct`, `key.py`'s RFC 6979 ECDSA,
BIP-340 signing and taproot tweak, `messages.py`'s transactions), with
python-bip380 (`miniscript_ref.py`) for the satisfaction of a miniscript
and `btc.py` for BIP-32.

- Published: BIP-174's signer, finalizer and extractor vectors
  (`tools/vectors/psbt/bip174-roles.txt`), and the wallets' spends in
  `tools/vectors/psbt/` that Bitcoin Core built (a 2-of-3 `sortedmulti`,
  the Liana-shaped miniscript down both paths, a taproot tree by a leaf
  and by the key path, BIP 387's taproot multisig), signed with every
  subset of their keys that matters.
- Random PSBTs of one to four inputs of every kind Faraday signs: P2PKH,
  P2SH-P2WPKH, P2WPKH, `multi` and `sortedmulti` in P2SH, P2WSH and
  P2SH-P2WSH, P2TR key path (BIP-86, and a key path beside a tree),
  P2TR script path (leaves of `pk`, `pkh` and timelocked keys, `or_d`,
  `multi_a`), and miniscript in P2WSH and P2SH-P2WSH (the Liana,
  recovery and threshold shapes), with keys this device holds, keys it
  does not, signatures cosigners already made, every standard sighash
  type, timelocks met and not met, under both RFC 6979 nonce rules.

Each PSBT is signed by Faraday holding a random set of its masters; the
reference signs it the same way, and the two PSBTs must hold the same
key-value pairs: each ECDSA signature made again byte for byte (the
first RFC 6979 nonce from Core's `key.py`, Core's low-R grind as
libsecp256k1 states it from `area_msg.py`), checked low-S and, under
low-R, low-R; each BIP-340 signature made again byte for byte with zero
auxiliary randomness, by the tweaked key for a key path. Then Faraday
finalizes its own signed PSBT and the reference finalizes the same
bytes: each final scriptSig and witness as BIP-174 builds it for the
script (a miniscript's the smallest non-malleable satisfaction
python-bip380 finds; a taproot input's its key path when signed for,
otherwise the cheapest leaf's), the fields BIP-174 clears cleared, and
the extracted transaction byte for byte, its txid the unsigned
transaction's.

What the reference signs is BIP-174's signer with Faraday's rules where
the BIP leaves the choice: every input whose key origins name a held
master, the key derived from the origin's path and refused when it is
not the key the PSBT names or not in the script; a taproot leaf signed
by every held key the leaf names and whose origin lists the leaf; the
key path signed when the internal key is held and the output is that
key tweaked by the stated Merkle root.

python-bip380 writes a `multi_a`'s satisfaction with the first key's
signature deepest, which BIP 342's `OP_CHECKSIGADD` reads the other way;
a leaf that is one `multi_a` is satisfied here by BIP 379's rule, the
last key's signature or nothing first, the first key's last, exactly k
signatures, any k of those present.

A witness script or leaf is read from the miniscript text it was
compiled from (`miniscript_ref.Scripts`): the corpus keeps each text,
and a fixture's are its wallet policy's, the keys derived by `btc.py` at
the chain and index the input's origins name.

Known differences, counted: Faraday refuses to sign a SegWit v0 input
that carries no previous transaction in a transaction of more than one
input, which BIP-174's signer signs (`faraday_psbt::sign`); BIP-174's own
signer vector is such a PSBT. Faraday signs a taproot leaf with a `pkh`
but leaves it unfinalized (`faraday_psbt::finalize`: `miniscript` 12
cannot write that satisfaction); the reference does the same, and counts
each such leaf it could have satisfied.
"""

import base64
import io

import area_msg
import btc
import miniscript_ref as ms
from area_psbt import MAGIC, compact, read_map, read_unsigned, ser_compact, write_map
from btc import HARDENED
from common import ROOT, Area, Mismatch, use_reference

use_reference("core")
import test_framework.script as S  # noqa: E402
from test_framework.key import (  # noqa: E402
    ECKey, ECPubKey, TaggedHash, sign_schnorr, tweak_add_privkey, verify_schnorr)
from test_framework.messages import (  # noqa: E402
    COutPoint, CTransaction, CTxIn, CTxInWitness, CTxOut, ser_string)

use_reference("bip39")
from mnemonic import Mnemonic  # noqa: E402

N = btc.N
TAPSCRIPT = 0xC0
STANDARD = [1, 2, 3, 0x81, 0x82, 0x83]
TAPROOT = [0, 1, 2, 3, 0x81, 0x82, 0x83]
# The input fields BIP-174 and BIP-371 define, which a finalizer clears
# but for the UTXOs; any other type is unknown and kept.
DEFINED = {0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
           0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0xfc}


class Refused(Exception):
    pass


# Every miniscript this area builds or reads a fixture's wallet for,
# beside its script (`miniscript_ref.Scripts`).
SCRIPTS = ms.Scripts()
# Satisfiable taproot leaves with a `pkh` the reference left unfinalized,
# as Faraday does: a known difference, counted.
PKH_LEFT = [0]


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
    """A refusal Faraday makes where BIP-174 signs."""


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


def ser_witness(stack):
    return ser_compact(len(stack)) + b"".join(ser_compact(len(e)) + e for e in stack)


def witness_weight(stack):
    return len(ser_witness(stack))


def read_txout(v):
    out = CTxOut()
    out.deserialize(io.BytesIO(v))
    return out


def read_prev(v):
    tx = CTransaction()
    tx.deserialize(io.BytesIO(v))
    return tx


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


def is_plain_multi_a(script):
    """A leaf that is one `multi_a` and nothing else: its threshold and
    keys, or None."""
    try:
        ops = list(S.CScript(script))
    except S.CScriptInvalidError:
        return None
    if len(ops) < 4 or ops[-1] != S.OP_NUMEQUAL or ops[1] != S.OP_CHECKSIG:
        return None
    keys = ops[0:-2:2]
    rest = ops[1:-2:2]
    if not all(isinstance(k, bytes) and len(k) == 32 for k in keys):
        return None
    if rest[0] != S.OP_CHECKSIG or any(op != S.OP_CHECKSIGADD for op in rest[1:]):
        return None
    k = ops[-2]
    if type(k) is not int:
        return None
    return k, keys


def tap_leaf_hash(script, version):
    return TaggedHash("TapLeaf", bytes([version]) + ser_string(script))


def p2tr(internal, merkle):
    tweak = TaggedHash("TapTweak", internal + (merkle or b""))
    q = S.tweak_add_pubkey(internal, tweak)
    return b"\x51\x20" + q[0], tweak


# ----- the reference signer -----------------------------------------------------


class Input:
    """One input as BIP-174's signer reads it."""

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


def ecdsa(d, h, nonce, ht, pk):
    """The DER signature and its sighash byte, as libsecp256k1 makes it
    under the nonce rule: Core's `key.py` makes the first RFC 6979 one,
    `area_msg.py` grinds for low R. Verified by Core's `key.py` against
    `pk`."""
    if nonce == "first":
        key = ECKey()
        key.set(d.to_bytes(32, "big"), True)
        sig = key.sign_ecdsa(h, low_s=True, rfc6979=True)
        r, s, _ = area_msg.ecdsa_sign(d, h)
        if area_msg.der(r, s) != sig:
            raise AssertionError("Core's RFC 6979 and libsecp256k1's as written differ")
    else:
        r, s, _ = area_msg.sign_ground(d, h, True)
        if r >= 1 << 255:
            raise AssertionError("a low-R signature with a high R")
        sig = area_msg.der(r, s)
    if s > N // 2:
        raise AssertionError("a high-S signature")
    pub = ECPubKey()
    pub.set(pk)
    if not pub.verify_ecdsa(sig, h):
        raise AssertionError("the reference's ECDSA signature does not verify")
    return sig + bytes([ht])


def schnorr(seckey, h, ht, signer):
    """A BIP-340 signature with zero auxiliary randomness, verified
    against the x-only key `signer` (the output key for a key path)."""
    sig = sign_schnorr(seckey, h, aux=bytes(32))
    if not verify_schnorr(signer, sig, h):
        raise AssertionError("the reference's BIP-340 signature does not verify")
    return sig + (bytes([ht]) if ht else b"")


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


def ref_sign(raw, masters, nonce):
    """BIP-174's signer over `raw` with every master in `masters`
    (fingerprint → btc.Key): the signed PSBT, the number of signatures,
    and whether every input can now be finalized."""
    g, ins, outs, tx = parse(raw)
    inputs = [Input(m, tx, i) for i, m in enumerate(ins)]
    out_ins = []
    count = 0
    for inp, m in zip(inputs, ins):
        added = []
        if not inp.final:
            added = sign_input(inp, inputs, tx, masters, nonce)
        count += len(added)
        out_ins.append(list(m) + added)
    signed = write(g, out_ins, outs)
    complete = ref_finalize(signed, check_only=True)
    return {"psbt": signed.hex(), "signatures": count, "complete": complete}


def sign_input(inp, inputs, tx, masters, nonce):
    m, i = inp.m, inp.i
    ecdsa_claims = [(k, origin(v)) for k, v in of_type(m, 0x06) if origin(v)[0] in masters]
    tap = [(k, tap_origin(v)) for k, v in of_type(m, 0x16)]
    key_claims = [(k, fp, p) for k, (ls, fp, p) in tap if not ls and fp in masters]
    leaf_claims = [(k, ls, fp, p) for k, (ls, fp, p) in tap if ls and fp in masters]
    if not ecdsa_claims and not key_claims and not leaf_claims:
        return []
    if inp.utxo is None:
        raise Refused("no previous output")
    if inp.kind in ("p2wpkh", "p2sh-p2wpkh", "p2wsh-multi", "p2sh-p2wsh-multi", "p2wsh-ms",
                    "p2sh-p2wsh-ms") and not inp.has_prev and len(inputs) > 1:
        raise Known("a SegWit v0 input with no previous transaction beside another input")
    ht_field = one(m, 0x03)
    added = []
    if not inp.kind.startswith("tr-"):
        ht = int.from_bytes(ht_field, "little") if ht_field else 1
        if ht not in STANDARD:
            raise Refused("not a standard sighash type")
        for pk, (fp, path) in ecdsa_claims:
            key = masters[fp].derive(path)
            if key.pub != pk or not inp.key_in_script(pk):
                raise Refused("the key the origin derives is not the one named, or not in the script")
            if ht & 3 == 3 and i >= len(tx.vout):
                raise Refused("SIGHASH_SINGLE without an output")
            h = inp.ecdsa_sighash(tx, ht)
            added.append((b"\x02" + pk, ecdsa(key.k, h, nonce, ht, pk)))
        return added
    ht = int.from_bytes(ht_field, "little") if ht_field else 0
    internal = one(m, 0x17)
    spk = bytes(inp.utxo.scriptPubKey)
    if inp.kind == "tr-key":
        for x, fp, path in key_claims:
            if internal is None:
                raise Refused("taproot input without an internal key")
            if internal != x or p2tr(internal, None)[0] != spk:
                raise Refused("the key path is not this key's")
            key = masters[fp].derive(path)
            if key.pub[1:] != x:
                raise Refused("the key the origin derives is not the one named")
            tweaked = tweak_add_privkey(key.k.to_bytes(32, "big"), p2tr(internal, None)[1])
            h = tap_sighash(tx, inputs, i, ht)
            added.append((b"\x13", schnorr(tweaked, h, ht, spk[2:])))
        return added
    for control, value in of_type(m, 0x15):
        script, version = value[:-1], value[-1]
        lh = tap_leaf_hash(script, version)
        names = leaf_keys(script, version)
        for x, leaves, fp, path in leaf_claims:
            if lh not in leaves or x not in names:
                continue
            key = masters[fp].derive(path)
            if key.pub[1:] != x:
                raise Refused("the key the origin derives is not the one named")
            h = tap_sighash(tx, inputs, i, ht, script, version)
            added.append((b"\x14" + x + lh, schnorr(key.k.to_bytes(32, "big"), h, ht, x)))
    merkle = one(m, 0x18)
    for x, fp, path in key_claims:
        if internal != x or p2tr(internal, merkle)[0] != spk:
            continue
        key = masters[fp].derive(path)
        if key.pub[1:] != x:
            raise Refused("the key the origin derives is not the one named")
        tweaked = tweak_add_privkey(key.k.to_bytes(32, "big"), p2tr(internal, merkle)[1])
        h = tap_sighash(tx, inputs, i, ht)
        added.append((b"\x13", schnorr(tweaked, h, ht, spk[2:])))
    if not added:
        raise Refused("no leaf or key path of this taproot input is ours")
    return added


# ----- the reference finalizer ----------------------------------------------------


def timelock_material(tx, i):
    """What BIP-68 and BIP-65 let this input's satisfaction use, in
    blocks: the relative height its sequence makes, the absolute height
    the locktime makes (both zero when disabled or counted in time)."""
    seq = tx.vin[i].nSequence
    rel = 0
    if tx.version >= 2 and not seq & (1 << 31) and not seq & (1 << 22):
        rel = seq & 0xFFFF
    absolute = 0
    if seq != 0xFFFFFFFF and tx.nLockTime < 500_000_000:
        absolute = tx.nLockTime
    return rel, absolute


def final_for(inp, tx):
    """(scriptSig, witness) choices for one input, or None while it
    cannot be finalized. Each choice is a list of acceptable
    (scriptSig, witness) pairs, or a predicate over one."""
    m = inp.m
    sigs = {k: v for k, v in of_type(m, 0x02)}
    kind = inp.kind
    if kind in ("p2pkh", "p2wpkh", "p2sh-p2wpkh"):
        found = [(pk, s) for pk, s in sorted(sigs.items()) if inp.key_in_script(pk)]
        if not found:
            return None
        pk, s = found[0]
        if kind == "p2pkh":
            return [(bytes(S.CScript([s, pk])), None)]
        script_sig = bytes(S.CScript([inp.redeem])) if kind == "p2sh-p2wpkh" else b""
        return [(script_sig, [s, pk])]
    if kind.endswith("-multi"):
        script = inp.redeem if kind == "p2sh-multi" else inp.ws
        need, keys = parse_multisig(script)
        chosen = [sigs[k] for k in keys if k in sigs][:need]
        if len(chosen) < need:
            return None
        if kind == "p2sh-multi":
            return [(bytes(S.CScript([S.OP_0] + chosen + [script])), None)]
        script_sig = bytes(S.CScript([inp.redeem])) if kind == "p2sh-p2wsh-multi" else b""
        return [(script_sig, [b""] + chosen + [script])]
    if kind.endswith("-ms"):
        rel, absolute = timelock_material(tx, inp.i)
        wit = ms.satisfy(inp.node(), sigs, rel, absolute)
        if wit is None:
            return None
        script_sig = bytes(S.CScript([inp.redeem])) if kind == "p2sh-p2wsh-ms" else b""
        return [(script_sig, list(wit) + [inp.ws])]
    if kind == "tr-key" or (kind == "tr-script" and one(m, 0x13) is not None):
        key_sig = one(m, 0x13)
        return None if key_sig is None else [(b"", [key_sig])]
    # A script path: every leaf the PSBT carries, the cheapest.
    rel, absolute = timelock_material(tx, inp.i)
    options = []
    for control, value in of_type(m, 0x15):
        script, version = value[:-1], value[-1]
        lh = tap_leaf_hash(script, version)
        here = {k[:32]: v for k, v in of_type(m, 0x14) if k[32:] == lh}
        multi = is_plain_multi_a(script)
        if multi:
            need, keys = multi
            if sum(1 for x in keys if x in here) < need:
                continue
            size = witness_weight([b""] * (len(keys) - need)
                                  + [here[x] for x in keys if x in here][:need]
                                  + [script, control])
            options.append((size, ("multi_a", need, keys, here, script, control)))
            continue
        node = SCRIPTS.node(script, True) if version == TAPSCRIPT else None
        if node is None:
            continue
        wit = ms.satisfy(node, here, rel, absolute)
        if wit is None:
            continue
        if ms.has_pkh(node):
            # Faraday leaves a taproot leaf with a `pkh` to the
            # coordinator (`faraday_psbt::finalize`).
            PKH_LEFT[0] += 1
            continue
        stack = list(wit) + [script, control]
        options.append((witness_weight(stack), ("exact", stack)))
    if not options:
        return None
    best = min(size for size, _ in options)
    return [(b"", choice) for size, choice in options if size == best]


def acceptable(choice, script_sig, witness):
    want_sig, want = choice
    if want_sig != script_sig:
        return False
    if isinstance(want, tuple) and want[0] == "exact":
        return want[1] == witness
    if isinstance(want, tuple) and want[0] == "multi_a":
        _, need, keys, here, script, control = want
        if witness is None or len(witness) != len(keys) + 2:
            return False
        if witness[-2:] != [script, control]:
            return False
        # BIP 379: the last key's signature (or nothing) deepest.
        body = witness[:-2]
        used = 0
        for x, element in zip(reversed(keys), body):
            if element == b"":
                continue
            if here.get(x) != element:
                return False
            used += 1
        return used == need
    return want == witness


def ref_finalize(raw, check_only=False, theirs=None):
    """BIP-174's finalizer and extractor over `raw`. With `theirs`
    (Faraday's finalized PSBT), each input's final scriptSig and witness
    is checked against every choice the reference allows and the
    reference answers with Faraday's choice where it is one of them."""
    g, ins, outs, tx = parse(raw)
    inputs = [Input(m, tx, i) for i, m in enumerate(ins)]
    their_ins = parse(theirs)[1] if theirs is not None else None
    out_ins = []
    finals = []
    all_final = True
    for j, (inp, m) in enumerate(zip(inputs, ins)):
        if inp.final:
            out_ins.append(m)
            finals.append((one(m, 0x07) or b"", read_witness_value(one(m, 0x08))))
            continue
        if inp.utxo is None:
            all_final = False
            out_ins.append(m)
            finals.append(None)
            continue
        choices = final_for(inp, tx)
        if choices is None:
            all_final = False
            out_ins.append(m)
            finals.append(None)
            continue
        pick = choices[0]
        if their_ins is not None:
            got_sig = one(their_ins[j], 0x07) or b""
            got_wit = read_witness_value(one(their_ins[j], 0x08))
            if any(acceptable(c, got_sig, got_wit) for c in choices):
                pick = (got_sig, got_wit)
            else:
                pick = concrete(pick)
        else:
            pick = concrete(pick)
        script_sig, witness = pick
        kept = [(k, v) for k, v in m if k[0] in (0x00, 0x01) or k[0] not in DEFINED]
        if script_sig:
            kept.append((b"\x07", script_sig))
        if witness:
            kept.append((b"\x08", ser_witness(witness)))
        out_ins.append(kept)
        finals.append((script_sig, witness))
    if check_only:
        return all_final
    final_psbt = write(g, out_ins, outs)
    if not all_final:
        return {"psbt": final_psbt.hex(), "tx": None}
    done = CTransaction(tx)
    done.wit.vtxinwit = []
    for k, (script_sig, witness) in enumerate(finals):
        done.vin[k].scriptSig = script_sig
        w = CTxInWitness()
        w.scriptWitness.stack = list(witness or [])
        done.wit.vtxinwit.append(w)
    # With no scriptSig, the extracted transaction's txid is the unsigned
    # one's: only the witness was added.
    if all(not v.scriptSig for v in done.vin) and done.txid_hex != tx.txid_hex:
        raise AssertionError("the extracted transaction's txid is not the unsigned one's")
    return {"psbt": final_psbt.hex(), "tx": done.serialize().hex()}


def concrete(choice):
    """One witness out of a choice, for the answer when Faraday's was not
    acceptable: the exact one, or a `multi_a`'s first k signatures."""
    script_sig, want = choice
    if isinstance(want, tuple) and want[0] == "exact":
        return script_sig, want[1]
    if isinstance(want, tuple) and want[0] == "multi_a":
        _, need, keys, here, script, control = want
        body, used = [], 0
        for x in keys:
            if x in here and used < need:
                body.append(here[x])
                used += 1
            else:
                body.append(b"")
        return script_sig, list(reversed(body)) + [script, control]
    return script_sig, want


def read_witness_value(v):
    if v is None:
        return None
    n, at = compact(v, 0)
    out = []
    for _ in range(n):
        size, at = compact(v, at)
        out.append(v[at:at + size])
        at += size
    return out


# ----- the area -----------------------------------------------------------------


class Sign(Area):
    name = "sign"

    def __init__(self, *a):
        super().__init__(*a)
        self.known = 0

    def sign(self, raw, seeds, xprvs, network, nonce, masters, wallets=()):
        inputs = {"psbt": raw.hex(), "seeds": [s.hex() for s in seeds], "xprvs": xprvs,
                  "network": network, "nonce": nonce, "wallets": list(wallets)}

        def ref():
            try:
                return ref_sign(raw, masters, nonce)
            except Known:
                self.known += 1
                raise

        def norm(answer):
            if answer[0] != "ok":
                return answer
            return ("ok", dict(answer[1], psbt=canonical(bytes.fromhex(answer[1]["psbt"]))))
        return self.compare("sign.sign", inputs, ref, normalize=norm)

    def finalize(self, raw):
        theirs = self.rust.call("sign.finalize", psbt=raw.hex())
        self.compared += 1
        theirs_psbt = bytes.fromhex(theirs[1]["psbt"]) if theirs[0] == "ok" else None
        before = PKH_LEFT[0]
        try:
            ref = ("ok", ref_finalize(raw, theirs=theirs_psbt))
        except Refused as e:
            ref = ("err", str(e))
        self.known += PKH_LEFT[0] - before

        def norm(answer):
            if answer[0] != "ok":
                return answer
            return ("ok", {"psbt": canonical(bytes.fromhex(answer[1]["psbt"])),
                           "tx": answer[1]["tx"]})
        a, b = norm(theirs), norm(ref)
        if a[0] != b[0] or (a[0] == "ok" and a != b):
            raise Mismatch(self.name, "sign.finalize", {"psbt": raw.hex()}, theirs, ref)
        return theirs

    def round(self, raw, seeds, xprvs, network, masters, nonce=None, wallets=()):
        nonce = nonce or self.rng.choice(["low_r", "first"])
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
        # With that input final, as the finalizer leaves it, the legacy
        # input is signed: its signatures are the combiner's.
        g, ins, outs, tx = parse(psbt("updater-sighash-all"))
        fin = parse(psbt("finalizer"))
        ins[1] = fin[1][1]
        raw = write(g, ins, outs)
        signed = self.sign(raw, [], [tprv], "testnet", "first", masters)
        theirs = parse(bytes.fromhex(signed[1]["psbt"]))[1][0]
        combiner = parse(psbt("combiner"))[1][0]
        if sorted(of_type(theirs, 0x02)) != sorted(of_type(combiner, 0x02)):
            raise Mismatch(self.name, "sign.sign", {"psbt": raw.hex()}, signed, "the combiner's",
                           "the signatures are not BIP-174's")
        done = self.finalize(bytes.fromhex(signed[1]["psbt"]))
        if canonical(bytes.fromhex(done[1]["psbt"])) != canonical(psbt("finalizer")):
            raise Mismatch(self.name, "sign.finalize", {}, done, "BIP-174's finalizer PSBT")
        extracted = roles["role extractor"]["tx"][0]
        if done[1]["tx"] != extracted:
            raise Mismatch(self.name, "sign.finalize", {}, done, extracted,
                           "not BIP-174's extracted transaction")
        self.round(raw, [], [tprv], "testnet", masters, nonce="low_r")

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
                policy = (ROOT / f"tools/vectors/psbt/{policies[name]}.policy").read_text()
                wallets = [policy]
                if name != "wallet-2of3":
                    register_policy(policy, raw)
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

    def funding(self, spk, value):
        prev = CTransaction()
        prev.version = 2
        prev.vin = [CTxIn(COutPoint(int.from_bytes(self.randbytes(32), "big"), self.rng.randrange(4)),
                          b"", 0xFFFFFFFF)]
        n = self.rng.randrange(1, 4)
        at = self.rng.randrange(n)
        prev.vout = [CTxOut(self.rng.randrange(1000, 10**7), p2wpkh(self.randbytes(20)))
                     for _ in range(n)]
        prev.vout[at] = CTxOut(value, spk)
        return prev, at

    def holder(self):
        """Whose key a slot is: one of ours (index into self.ours) or a
        foreign master."""
        if self.rng.random() < 0.6:
            return self.rng.choice(self.ours)[1]
        return None

    def corpus_case(self):
        self.test = self.rng.random() < 0.7
        network = self.rng.choice(["testnet", "regtest", "signet"]) if self.test else "mainnet"
        self.ours = [self.new_master() for _ in range(self.rng.randrange(1, 4))]
        masters = {m.fingerprint(): m for _, m in self.ours}
        tx = CTransaction()
        tx.version = 2
        tx.nLockTime = self.rng.choice([0, self.rng.randrange(1, 900_000)])
        n_in = self.rng.randrange(1, 5)
        n_out = self.rng.randrange(1, 4)
        ins = []
        total = 0
        presign = []
        for i in range(n_in):
            m, spk, value, seq, extra = self.input_spec(i, n_out)
            prev, at = self.funding(spk, value)
            total += value
            tx.vin.append(CTxIn(COutPoint(prev.txid_int, at), b"", seq))
            fields = [(b"\x00", prev.serialize_without_witness())] if extra.get("prev", True) else []
            if extra.get("witness_utxo", True):
                fields.append((b"\x01", CTxOut(value, spk).serialize()))
            ins.append(fields + m)
            presign.append(extra.get("cosigners", {}))
        fee = self.rng.randrange(500, 20_000)
        left = max(total - fee, n_out * 1000)
        cuts = sorted(self.rng.randrange(1, left) for _ in range(n_out - 1))
        amounts = [b - a for a, b in zip([0] + cuts, cuts + [left])]
        for a in amounts:
            spk = (p2wpkh(self.randbytes(20)) if self.rng.random() < 0.5
                   else b"\x51\x20" + btc.point(self.rng.randrange(1, N))[1:])
            tx.vout.append(CTxOut(max(a, 546), spk))
        g = [(b"\x00", tx.serialize_without_witness())]
        outs = [[] for _ in tx.vout]
        raw = write(g, ins, outs)
        # Cosigners that already signed: their signatures are on the PSBT
        # Faraday receives.
        for i, cos in enumerate(presign):
            if cos:
                try:
                    _, pins, _, _ = parse(raw)
                    inputs = [Input(mm, tx, j) for j, mm in enumerate(pins)]
                    added = sign_input(inputs[i], inputs, tx, cos, "low_r")
                except Refused:
                    added = []
                ins[i] = ins[i] + added
        raw = write(g, ins, outs)
        held = [s for s, _ in self.ours]
        self.round(raw, held, [], network, masters)

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
        value = self.rng.randrange(20_000, 10**8)
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
            slots = [self.key(48, self.holder(), 2) for _ in range(n)]
            keys = [s[0] for s in slots]
            if self.rng.random() < 0.5:
                keys = sorted(keys)
            script = multisig_script(need, keys)
            wrap = self.rng.choice(["p2sh", "p2wsh", "p2sh-p2wsh"])
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

    def cosigners(self, slots):
        foreign = {fp: m for _, fp, _, m in slots
                   if fp not in {mm.fingerprint() for _, mm in self.ours}}
        chosen = {fp: m for fp, m in foreign.items() if self.rng.random() < 0.5}
        return chosen

    def locks(self, i):
        """A block delay and a height for a timelocked template, and the
        sequence this input spends with: sometimes enough, sometimes not."""
        older = self.rng.randrange(1, 1000)
        after = self.rng.randrange(1, 800_000)
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
        slots = [self.key(48, self.holder(), 2) for _ in range(n)]
        older, after = self.locks(i)
        text = template.format(*[s[0].hex() for s in slots], N=older, T=after)
        script = SCRIPTS.add(text, False)
        fields = []
        if self.rng.random() < 0.5:
            spk = p2wsh(script)
            fields.append((b"\x05", script))
        else:
            spk = p2sh(p2wsh(script))
            fields += [(b"\x04", p2wsh(script)), (b"\x05", script)]
        for pk, fp, path, _ in slots:
            fields.append((b"\x06" + pk, ser_origin(fp, path)))
        extra["cosigners"] = self.cosigners(slots)
        fields += self.ht_field(self.sighash(i, n_out, False))
        seq = self.timelocked(i, older, after)
        return fields, spk, value, seq, extra

    def tree_input(self, i, n_out, value, extra):
        count = self.rng.randrange(1, 5)
        leaves = []
        names = []
        for _ in range(count):
            template, n = self.rng.choice(self.LEAVES)
            older, after = self.locks(i)
            if n == 0:
                n = self.rng.randrange(1, 4)
                need = self.rng.randrange(1, n + 1)
                slots = [self.key(87, self.holder()) for _ in range(n)]
                text = template.format(k=need, keys=",".join(s[0][1:].hex() for s in slots))
            else:
                slots = [self.key(87, self.holder()) for _ in range(n)]
                text = template.format(*[s[0][1:].hex() for s in slots], N=older)
            leaves.append((SCRIPTS.add(text, True), slots, older))
        # The internal key: one of ours, a foreign key, or BIP-341's NUMS.
        r = self.rng.random()
        if r < 0.4:
            ik = self.key(86, self.rng.choice(self.ours)[1])
        elif r < 0.7:
            ik = self.key(86, None)
        else:
            ik = (b"\x02" + bytes.fromhex(
                "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0"), None, None, None)
        internal = ik[0][1:]
        items = self.items(len(leaves), [(f"l{j}", s) for j, (s, _, _) in enumerate(leaves)])
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
        self.sign(raw, [seed], [], "testnet", "low_r", {fp: master})

    def run(self):
        self.bip174()
        self.fixtures()
        for _ in range(self.cases(8)):
            self.mismatched()
        for _ in range(self.cases(120)):
            self.corpus_case()


def taproot_construct_safe(internal, items):
    return S.taproot_construct(internal, items)
