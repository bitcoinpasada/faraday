"""The Bitcoin building blocks the known answer of a MuSig2 public nonce
needs (`docs/OWN-RANDOMNESS.md` §8.5 test 3), written from the BIPs over
Python's standard library, never from Faraday's code or any library of
Bitcoin's. The one import is BIP-327's own reference implementation
(`tools/reference/bip327/reference.py`, vendored unchanged), for the
curve arithmetic, MuSig2's key aggregation and NonceGen.

- BIP-32 private and public key derivation, and the xprv/xpub text.
- BIP-39's seed from words (PBKDF2-HMAC-SHA512).
- BIP-174 reading of a PSBT: the unsigned transaction, each input's
  witness UTXO, sighash type, taproot internal key, taproot key origins
  and BIP-373's MuSig2 participants field.
- BIP-341's key-path signature message and sighash, every hash type.
- The MuSig2 output key of a wallet's address: BIP-327's key aggregation
  of the sorted participants, BIP-328's synthetic xpub over it (checked against its published vectors), the
  BIP-32 steps to the address as plain tweaks and BIP-341's tweak with
  no script tree as the x-only one.

`python3 -I -B tools/vectors/own-randomness/bitcoin.py --check` checks
each against the published vectors in `tools/vectors/` and fails loudly
on the first that differs.
"""
import hashlib
import hmac
import importlib.util
import json
import sys
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
VECTORS = ROOT / "tools/vectors"

_spec = importlib.util.spec_from_file_location(
    "bip327_reference", ROOT / "tools/reference/bip327/reference.py"
)
ref = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(ref)
N, P, G = ref.n, ref.p, ref.G

HARD = 0x80000000
# BIP-328: the chain code of the synthetic xpub over an aggregate key.
BIP328_CHAIN_CODE = bytes.fromhex(
    "868087ca02a6f974c4598924c36b57762d32cb45717167e300622c7167e38965"
)


def sha256(b):
    return hashlib.sha256(b).digest()


def dsha256(b):
    return sha256(sha256(b))


def hash160(b):
    return hashlib.new("ripemd160", sha256(b)).digest()


def tagged(tag, msg):
    return ref.tagged_hash(tag, msg)


# ---------------------------------------------------------------------
# BIP-32
# ---------------------------------------------------------------------

B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


def b58check(payload):
    data = payload + dsha256(payload)[:4]
    n = int.from_bytes(data, "big")
    out = ""
    while n:
        n, r = divmod(n, 58)
        out = B58[r] + out
    return "1" * (len(data) - len(data.lstrip(b"\0"))) + out


def point_of(k):
    return ref.cbytes(ref.point_mul(G, k))


class Node:
    """An extended key: `k` (int, or None for a public one), the public
    key `pub` (33 bytes), the chain code, depth, the parent's fingerprint
    and the child number."""

    def __init__(self, k, pub, chain, depth=0, parent=b"\0\0\0\0", index=0):
        self.k, self.pub, self.chain = k, pub, chain
        self.depth, self.parent, self.index = depth, parent, index

    def fingerprint(self):
        return hash160(self.pub)[:4]

    def secret(self):
        return self.k.to_bytes(32, "big")

    def serialize(self, private, mainnet=True):
        version = {
            (True, True): "0488ade4", (True, False): "0488b21e",
            (False, True): "04358394", (False, False): "043587cf",
        }[(mainnet, private)]
        key = b"\0" + self.secret() if private else self.pub
        return b58check(
            bytes.fromhex(version) + bytes([self.depth]) + self.parent
            + self.index.to_bytes(4, "big") + self.chain + key
        )


def master(seed):
    i = hmac.new(b"Bitcoin seed", seed, "sha512").digest()
    k = int.from_bytes(i[:32], "big")
    assert 0 < k < N, "an invalid master key"
    return Node(k, point_of(k), i[32:])


def ckd_priv(node, index):
    if index >= HARD:
        data = b"\0" + node.secret() + index.to_bytes(4, "big")
    else:
        data = node.pub + index.to_bytes(4, "big")
    i = hmac.new(node.chain, data, "sha512").digest()
    il = int.from_bytes(i[:32], "big")
    k = (il + node.k) % N
    assert il < N and k != 0, "an invalid child"
    return Node(k, point_of(k), i[32:], node.depth + 1, node.fingerprint(), index)


def ckd_pub(node, index):
    """The child's public key, and the tweak (IL) that leads to it."""
    assert index < HARD, "a hardened step has no public derivation"
    i = hmac.new(node.chain, node.pub + index.to_bytes(4, "big"), "sha512").digest()
    il = int.from_bytes(i[:32], "big")
    assert il < N
    child = ref.point_add(ref.point_mul(G, il), ref.cpoint(node.pub))
    assert child is not None
    return (
        Node(None, ref.cbytes(child), i[32:], node.depth + 1, node.fingerprint(), index),
        il,
    )


def parse_path(path):
    steps = []
    for part in path.split("/")[1:]:
        hard = part[-1] in "h'H"
        steps.append(int(part.rstrip("h'H")) + (HARD if hard else 0))
    return steps


def derive(node, path):
    for step in parse_path(path):
        node = ckd_priv(node, step)
    return node


# ---------------------------------------------------------------------
# BIP-39
# ---------------------------------------------------------------------


def bip39_seed(words, passphrase=""):
    norm = lambda s: unicodedata.normalize("NFKD", s)
    return hashlib.pbkdf2_hmac(
        "sha512", norm(words).encode(), norm("mnemonic" + passphrase).encode(), 2048
    )


# ---------------------------------------------------------------------
# BIP-174 and the transaction
# ---------------------------------------------------------------------


def read_varint(d, i):
    b = d[i]
    if b < 0xFD:
        return b, i + 1
    size = {0xFD: 2, 0xFE: 4, 0xFF: 8}[b]
    return int.from_bytes(d[i + 1:i + 1 + size], "little"), i + 1 + size


def varint(n):
    if n < 0xFD:
        return bytes([n])
    if n <= 0xFFFF:
        return b"\xfd" + n.to_bytes(2, "little")
    return b"\xfe" + n.to_bytes(4, "little")


class Tx:
    """A transaction without witnesses: version, inputs (txid in the
    order the wire has it, vout, scriptSig, sequence), outputs (amount,
    script) and lock time."""

    def __init__(self, raw):
        self.version = int.from_bytes(raw[:4], "little")
        i = 4
        assert raw[i] != 0, "a segwit serialisation: the unsigned form has none"
        count, i = read_varint(raw, i)
        self.inputs = []
        for _ in range(count):
            prev, vout = raw[i:i + 32], int.from_bytes(raw[i + 32:i + 36], "little")
            n, i = read_varint(raw, i + 36)
            script = raw[i:i + n]
            seq = int.from_bytes(raw[i + n:i + n + 4], "little")
            self.inputs.append((prev, vout, script, seq))
            i += n + 4
        count, i = read_varint(raw, i)
        self.outputs = []
        for _ in range(count):
            amount = int.from_bytes(raw[i:i + 8], "little")
            n, i = read_varint(raw, i + 8)
            self.outputs.append((amount, raw[i:i + n]))
            i += n
        self.locktime = int.from_bytes(raw[i:i + 4], "little")
        assert i + 4 == len(raw), "bytes after the transaction"


def read_map(d, i):
    """One key-value map: a list of (key type, key data, value) and the
    index after it."""
    out = []
    while True:
        n, i = read_varint(d, i)
        if n == 0:
            return out, i
        key = d[i:i + n]
        i += n
        n, i = read_varint(d, i)
        out.append((key[0], key[1:], d[i:i + n]))
        i += n


class Psbt:
    """A PSBT as BIP-174 lays it out: the global map's unsigned
    transaction and, per input, the witness UTXO (amount and script),
    sighash type, taproot internal key and key origins (BIP-371), and
    the MuSig2 participants (BIP-373)."""

    def __init__(self, raw):
        assert raw[:5] == b"psbt\xff", "not a PSBT"
        glob, i = read_map(raw, 5)
        unsigned = [v for t, k, v in glob if t == 0x00 and not k]
        assert len(unsigned) == 1, "no unsigned transaction"
        self.tx = Tx(unsigned[0])
        self.inputs = []
        for _ in self.tx.inputs:
            m, i = read_map(raw, i)
            self.inputs.append(self._input(m))

    @staticmethod
    def _input(m):
        out = {"origins": {}, "participants": []}
        for t, k, v in m:
            if t == 0x01:  # witness UTXO: amount (8, LE), script (compact size)
                n, j = read_varint(v, 8)
                out["utxo"] = (int.from_bytes(v[:8], "little"), v[j:j + n])
            elif t == 0x03:
                out["sighash"] = int.from_bytes(v, "little")
            elif t == 0x16:  # taproot key origin: leaf hashes, fingerprint, path
                leaves, j = read_varint(v, 0)
                j += 32 * leaves
                fp, rest = v[j:j + 4], v[j + 4:]
                path = [int.from_bytes(rest[o:o + 4], "little") for o in range(0, len(rest), 4)]
                out["origins"][k] = (leaves, fp, path)
            elif t == 0x17:
                out["internal"] = v
            elif t == 0x1A:  # MuSig2 participants: aggregate -> keys
                assert len(k) == 33 and len(v) % 33 == 0
                out["participants"].append(
                    (k, [v[o:o + 33] for o in range(0, len(v), 33)])
                )
        return out


# ---------------------------------------------------------------------
# BIP-341
# ---------------------------------------------------------------------

DEFAULT, ALL, NONE, SINGLE, ACP = 0, 1, 2, 3, 0x80


def sig_msg(tx, utxos, index, hash_type, annex=None, ext=b"", ext_flag=0):
    """BIP-341's signature message (without the epoch byte): `utxos` is
    the (amount, script) of every input the transaction spends."""
    assert hash_type in (0, 1, 2, 3, 0x81, 0x82, 0x83), "an invalid hash type"
    out_type = ALL if hash_type == DEFAULT else hash_type & 3
    acp = hash_type & ACP
    s = bytes([hash_type])
    s += tx.version.to_bytes(4, "little") + tx.locktime.to_bytes(4, "little")
    if not acp:
        s += sha256(b"".join(p + v.to_bytes(4, "little") for p, v, _, _ in tx.inputs))
        s += sha256(b"".join(a.to_bytes(8, "little") for a, _ in utxos))
        s += sha256(b"".join(varint(len(sc)) + sc for _, sc in utxos))
        s += sha256(b"".join(q.to_bytes(4, "little") for _, _, _, q in tx.inputs))
    if out_type == ALL:
        s += sha256(b"".join(a.to_bytes(8, "little") + varint(len(sc)) + sc for a, sc in tx.outputs))
    s += bytes([ext_flag * 2 + (1 if annex is not None else 0)])
    if acp:
        prev, vout, _, seq = tx.inputs[index]
        amount, script = utxos[index]
        s += prev + vout.to_bytes(4, "little") + amount.to_bytes(8, "little")
        s += varint(len(script)) + script + seq.to_bytes(4, "little")
    else:
        s += index.to_bytes(4, "little")
    if annex is not None:
        s += sha256(varint(len(annex)) + annex)
    if out_type == SINGLE:
        assert index < len(tx.outputs), "SIGHASH_SINGLE with no output"
        a, sc = tx.outputs[index]
        s += sha256(a.to_bytes(8, "little") + varint(len(sc)) + sc)
    return s + ext


def sighash(tx, utxos, index, hash_type):
    """BIP-341's key-path sighash: the tagged hash of the epoch 0 and
    the signature message."""
    return tagged("TapSighash", b"\0" + sig_msg(tx, utxos, index, hash_type))


def tap_tweak(internal_xonly, merkle_root=b""):
    return tagged("TapTweak", internal_xonly + merkle_root)


def output_key(internal_xonly, merkle_root=b""):
    """BIP-341's output key (x-only) of an internal key."""
    t = int.from_bytes(tap_tweak(internal_xonly, merkle_root), "big")
    assert t < N
    q = ref.point_add(ref.lift_x(internal_xonly), ref.point_mul(G, t))
    return ref.xbytes(q)


# ---------------------------------------------------------------------
# MuSig2 over an address of a wallet
# ---------------------------------------------------------------------


def musig_aggregate(pubkeys):
    """BIP-327's aggregate (a point) of the plain keys as given."""
    return ref.key_agg(list(pubkeys)).Q


def musig_address(participants, steps):
    """The x-only output key of the address `steps` (two unhardened
    steps) under the aggregate of `participants` (compressed keys, in
    any order): BIP-327 over the sorted keys, BIP-328's xpub over the
    aggregate, a plain tweak for each step and BIP-341's x-only tweak.
    Returns the output key and the internal key (x-only)."""
    q = musig_aggregate(sorted(participants))
    ctx = ref.key_agg(sorted(participants))
    node = Node(None, ref.cbytes(q), BIP328_CHAIN_CODE)
    for step in steps:
        node, il = ckd_pub(node, step)
        ctx = ref.apply_tweak(ctx, il.to_bytes(32, "big"), False)
    assert ref.cbytes(ctx.Q) == node.pub, "the plain tweaks and BIP-32 disagree"
    internal = node.pub[1:]
    ctx = ref.apply_tweak(ctx, tap_tweak(internal), True)
    return ref.xbytes(ctx.Q), internal


# ---------------------------------------------------------------------
# Checks against the published vectors
# ---------------------------------------------------------------------


def load(name):
    return json.loads((VECTORS / name).read_text())


def check_bip32():
    n = 0
    for case in load("bip32/vectors.json"):
        root = master(bytes.fromhex(case["seed"]))
        for chain in case["chains"]:
            node = derive(root, chain["path"])
            assert node.serialize(True) == chain["xprv"], ("BIP-32 xprv", chain["path"])
            assert node.serialize(False) == chain["xpub"], ("BIP-32 xpub", chain["path"])
            n += 1
    # Public derivation agrees with the private one at every unhardened
    # step of BIP-32's own first vector.
    root = master(bytes.fromhex("000102030405060708090a0b0c0d0e0f"))
    for path in ("m/0h/1", "m/0h/1/2h/2/1000000000"):
        priv = derive(root, path)
        parent = derive(root, path.rsplit("/", 1)[0])
        step = parse_path(path)[-1]
        if step < HARD:
            child, _ = ckd_pub(Node(None, parent.pub, parent.chain), step)
            assert child.pub == priv.pub and child.chain == priv.chain, ("CKDpub", path)
            n += 1
    return n


def check_bip39():
    n = 0
    for words in load("bip39/vectors.json")["english"]:
        _, mnemonic, seed, xprv = words
        got = bip39_seed(mnemonic, "TREZOR")
        assert got.hex() == seed, ("BIP-39 seed", mnemonic)
        assert master(got).serialize(True) == xprv, ("BIP-39 to BIP-32", mnemonic)
        n += 1
    jp = json.loads((VECTORS / "bip39/test_JP_BIP39.json").read_text())
    for case in jp:
        got = bip39_seed(case["mnemonic"], case["passphrase"])
        assert got.hex() == case["seed"], ("BIP-39 seed (Japanese)", case["mnemonic"])
        assert master(got).serialize(True) == case["bip32_xprv"]
        n += 1
    return n


def check_bip341():
    v = load("sighash/bip341-wallet-test-vectors.json")
    keys = sighashes = 0
    for s in v["scriptPubKey"]:
        if s["given"]["scriptTree"] is None:
            internal = bytes.fromhex(s["given"]["internalPubkey"])
            want = s["expected"]["scriptPubKey"]
            assert (b"\x51\x20" + output_key(internal)).hex() == want, "BIP-341 output key"
            assert tap_tweak(internal).hex() == s["intermediary"]["tweak"]
            keys += 1
    for k in v["keyPathSpending"]:
        tx = Tx(bytes.fromhex(k["given"]["rawUnsignedTx"]))
        utxos = [(u["amountSats"], bytes.fromhex(u["scriptPubKey"])) for u in k["given"]["utxosSpent"]]
        for case in k["inputSpending"]:
            g, mid = case["given"], case["intermediary"]
            i, ht = g["txinIndex"], g["hashType"]
            msg = b"\0" + sig_msg(tx, utxos, i, ht)
            assert msg.hex() == mid["sigMsg"], ("BIP-341 signature message", i, ht)
            assert sighash(tx, utxos, i, ht).hex() == mid["sigHash"], ("BIP-341 sighash", i, ht)
            if g["merkleRoot"] is None:
                internal = bytes.fromhex(mid["internalPubkey"])
                assert tap_tweak(internal).hex() == mid["tweak"]
                assert (b"\x51\x20" + output_key(internal)) == utxos[i][1], "BIP-341 key path output key"
            sighashes += 1
    return keys, sighashes


def check_bip86():
    """BIP-86's own first address, from its text: the abandon mnemonic's
    m/86'/0'/0'/0/0 key and the output key it tweaks to."""
    words = " ".join(["abandon"] * 11 + ["about"])
    node = derive(master(bip39_seed(words)), "m/86h/0h/0h/0/0")
    assert node.pub[1:].hex() == "cc8a4bc64d897bddc5fbc2f670f7a8ba0b386779106cf1223c6fc5d7cd6fc115"
    assert output_key(node.pub[1:]).hex() == "a60869f0dbcf1dc659c9cecbaf8050135ea9e8cdc487053f1dc6880949dc684c"
    return 1


def check_bip327():
    n = 0
    v = load("bip327/key_agg_vectors.json")
    keys = [bytes.fromhex(k) for k in v["pubkeys"]]
    for case in v["valid_test_cases"]:
        got = ref.xbytes(musig_aggregate([keys[i] for i in case["key_indices"]]))
        assert got.hex().upper() == case["expected"], ("BIP-327 key aggregation", case)
        n += 1
    s = load("bip327/key_sort_vectors.json")
    got = sorted(bytes.fromhex(k) for k in s["pubkeys"])
    assert [k.hex().upper() for k in got] == s["sorted_pubkeys"], "BIP-327 key sort"
    n += 1
    return n


def check_tweaks():
    """BIP-327's tweak vectors: the reference's tweaked aggregate, the
    one `musig_address` stacks its tweaks on (`apply_tweak`), verifies the
    partial signature each vector holds."""
    v = load("bip327/tweak_vectors.json")
    keys = [bytes.fromhex(k) for k in v["pubkeys"]]
    tweaks = [bytes.fromhex(t) for t in v["tweaks"]]
    n = 0
    for case in v["valid_test_cases"]:
        ctx = ref.key_agg_and_tweak(
            [keys[i] for i in case["key_indices"]],
            [tweaks[i] for i in case["tweak_indices"]],
            case["is_xonly"],
        )
        # The signature the vector holds verifies under this key.
        msg = bytes.fromhex(v["msg"])
        pubnonces = [bytes.fromhex(v["pnonces"][i]) for i in case["nonce_indices"]]
        sess = ref.SessionContext(
            ref.nonce_agg(pubnonces),
            [keys[i] for i in case["key_indices"]],
            [tweaks[i] for i in case["tweak_indices"]],
            case["is_xonly"], msg,
        )
        psig = bytes.fromhex(case["expected"])
        assert ref.partial_sig_verify_internal(
            psig, pubnonces[case["signer_index"]], keys[case["key_indices"][case["signer_index"]]], sess
        ), ("BIP-327 tweak vector", case)
        n += 1
    return n


MUSIG_PSBT = ROOT / "tools/vectors/own-randomness/musig-unsigned.psbt"


def musig_input_of(psbt, index=0):
    """What a MuSig2 input says, read as BIP-373 and BIP-371 lay it out:
    the aggregate, the participants, the internal key, and the two steps
    of the address below the internal key's origin."""
    inp = psbt.inputs[index]
    assert len(inp["participants"]) == 1, "one aggregate expected"
    aggregate, keys = inp["participants"][0]
    _, _, path = inp["origins"][inp["internal"]]
    assert len(path) == 2 and all(s < HARD for s in path), "chain and index"
    return aggregate, keys, inp["internal"], path


def check_musig_psbt():
    """The test stick's MuSig2 spend (`musig-unsigned.psbt`, made by
    Faraday's test kit: an input, not an answer), read as BIP-174 says:
    test key 1's participant key is what its seed's m/86'/1'/0' gives,
    and the output key the aggregate leads to by BIP-327, BIP-328, BIP-32
    and BIP-341 is the key of the UTXO's script and the internal key the
    file names."""
    psbt = Psbt(MUSIG_PSBT.read_bytes())
    aggregate, keys, internal, steps = musig_input_of(psbt)
    root = master(bip39_seed(" ".join(["bacon"] * 24)))
    account = derive(root, "m/86h/1h/0h")
    assert account.pub in keys, "test key 1 is not a participant"
    leaves, fp, path = psbt.inputs[0]["origins"][account.pub[1:]]
    assert (leaves, fp, path) == (0, root.fingerprint(), parse_path("m/86h/1h/0h"))
    # BIP-373: the internal key's origin names the BIP-328 xpub's fingerprint.
    synthetic = Node(None, ref.cbytes(musig_aggregate(sorted(keys))), BIP328_CHAIN_CODE)
    _, ifp, ipath = psbt.inputs[0]["origins"][internal]
    assert ifp == synthetic.fingerprint(), "the internal key origin's fingerprint"
    assert ipath == steps, "the internal key origin's steps"
    assert ref.cbytes(musig_aggregate(sorted(keys))) == aggregate, "the field's aggregate"
    out, got_internal = musig_address(keys, steps)
    assert got_internal == internal, "the internal key the file names"
    assert psbt.inputs[0]["utxo"][1] == b"\x51\x20" + out, "the UTXO's script"
    return 1


def check_bip328():
    """BIP-328's published cases (`tools/vectors/bip328/vectors.txt`):
    the aggregate of the keys as listed and the synthetic xpub over it."""
    n = 0
    for line in (VECTORS / "bip328/vectors.txt").read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        keys, aggregate, xpub = line.split()
        q = musig_aggregate([bytes.fromhex(k) for k in keys.split(",")])
        assert ref.cbytes(q).hex() == aggregate.lower(), ("BIP-328 aggregate", line)
        node = Node(None, ref.cbytes(q), BIP328_CHAIN_CODE)
        assert node.serialize(False) == xpub, ("BIP-328 xpub", line)
        n += 1
    assert n == 3, "BIP-328's three cases"
    return n


def check_all():
    r = {}
    r["BIP-32 chain keys (xprv and xpub) and public derivations"] = check_bip32()
    r["BIP-39 seeds (English with TREZOR, Japanese with a passphrase)"] = check_bip39()
    keys, hashes = check_bip341()
    r["BIP-341 output keys"] = keys
    r["BIP-341 signature messages and sighashes"] = hashes
    r["BIP-86 first address"] = check_bip86()
    r["BIP-327 key aggregation and sort vectors"] = check_bip327()
    r["BIP-327 tweak vectors"] = check_tweaks()
    r["BIP-328 aggregate keys and synthetic xpubs"] = check_bip328()
    r["MuSig2 spend of the test stick: keys, aggregate, address, script"] = check_musig_psbt()
    return r


if __name__ == "__main__":
    if sys.argv[1:] != ["--check"]:
        sys.exit("usage: python3 -I -B tools/vectors/own-randomness/bitcoin.py --check")
    for what, count in check_all().items():
        print(f"{count:4d}  {what}")
    print("all agree")
