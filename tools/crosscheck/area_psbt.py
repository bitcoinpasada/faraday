"""PSBTs against a reader written here from BIP-174, BIP-370 and
BIP-371's text, over Bitcoin Core's functional-test framework's
`CTransaction` (`core/test_framework/messages.py`) for the transactions
inside and the curve of `btc.py` for the keys.

- Every PSBT of BIP-174's, BIP-370's and BIP-371's Test Vectors sections
  (`tools/vectors/psbt-bips/`), read as version 0 by
  `faraday_psbt::Psbt` and as version 2 by `faraday_psbt::v2::PsbtV2`:
  refused where the BIP calls it invalid, read where valid, and for
  BIP-370's timelock cases the locktime it states.
- Random version 0 PSBTs with every field Faraday reads or carries
  (previous transactions and outputs, partial signatures, sighash types,
  scripts, BIP-32 and taproot key origins, final scripts, taproot key
  signatures, internal keys and merkle roots, global xpubs, unknown and
  proprietary pairs, taproot trees): written back by Faraday byte for
  byte as read, and the unsigned transaction's txid. Then the
  same PSBT broken one rule at a time: a key twice in a map, key data on
  a field that takes none, a key of the wrong length, a value of the
  wrong size, a scriptSig or witness in the unsigned transaction, a map
  too few or too many, bytes after the last map, a short stream, the
  magic changed.
- Random version 2 PSBTs: written back byte for byte, and the
  transaction and locktime BIP-370 gives, under every mix of required
  time and height locktimes.

A reading is compared as its maps (the key-value pairs of each map as a
set, since the BIPs leave the order of keys to the writer), and
Faraday's writing must be the bytes it read: a field it does not change
is written back as read (owner, 2026-10-23).

Where Faraday's version 2 reader takes what the BIPs refuse, the case is
a known difference (`LENIENT`), checked to still hold exactly, and
listed in the README.
"""

import io
import json

import btc
from common import ROOT, Area, Mismatch, use_reference

use_reference("core")
from test_framework.messages import (  # noqa: E402
    COutPoint, CTransaction, CTxIn, CTxOut, deser_vector)

MAGIC = b"psbt\xff"


class Refused(Exception):
    pass


# ----- the container ----------------------------------------------------------


def compact(b, at):
    if at >= len(b):
        raise Refused("short")
    n = b[at]
    if n < 0xfd:
        return n, at + 1
    size = {0xfd: 2, 0xfe: 4, 0xff: 8}[n]
    if at + 1 + size > len(b):
        raise Refused("short")
    v = int.from_bytes(b[at + 1:at + 1 + size], "little")
    if v < (0xfd if size == 2 else 1 << (8 * size // 2)):
        raise Refused("a size not written minimally")
    return v, at + 1 + size


def ser_compact(n):
    if n < 0xfd:
        return bytes([n])
    if n <= 0xFFFF:
        return b"\xfd" + n.to_bytes(2, "little")
    if n <= 0xFFFFFFFF:
        return b"\xfe" + n.to_bytes(4, "little")
    return b"\xff" + n.to_bytes(8, "little")


def read_map(b, at):
    """One map: its pairs in order, and where it ends. A key twice is
    refused (BIP-174: keys within a map are unique)."""
    pairs = []
    while True:
        n, at = compact(b, at)
        if n == 0:
            break
        if at + n > len(b):
            raise Refused("short")
        key, at = b[at:at + n], at + n
        m, at = compact(b, at)
        if at + m > len(b):
            raise Refused("short")
        pairs.append((bytes(key), bytes(b[at:at + m])))
        at += m
    if len({k for k, _ in pairs}) != len(pairs):
        raise Refused("a key twice in a map")
    return pairs, at


def write_map(pairs):
    return b"".join(ser_compact(len(k)) + k + ser_compact(len(v)) + v for k, v in pairs) + b"\0"


def read_tx(raw):
    tx = CTransaction()
    f = io.BytesIO(raw)
    tx.deserialize(f)
    if f.read() or tx.serialize() != raw:
        raise Refused("not exactly a transaction")
    return tx


def read_unsigned(raw):
    """The unsigned transaction, which BIP-174 serialises without the
    witness format: a transaction of no inputs is read as one."""
    tx = CTransaction()
    f = io.BytesIO(raw)
    tx.version = int.from_bytes(f.read(4), "little")
    tx.vin = deser_vector(f, CTxIn)
    tx.vout = deser_vector(f, CTxOut)
    tx.nLockTime = int.from_bytes(f.read(4), "little")
    if f.read() or tx.serialize_without_witness() != raw:
        raise Refused("not exactly a transaction")
    return tx


def read_txout(raw):
    if len(raw) < 9:
        raise Refused("short output")
    n, at = compact(raw, 8)
    if at + n != len(raw):
        raise Refused("an output of the wrong size")


def read_witness(raw):
    n, at = compact(raw, 0)
    for _ in range(n):
        m, at = compact(raw, at)
        at += m
        if at > len(raw):
            raise Refused("short witness")
    if at != len(raw):
        raise Refused("a witness of the wrong size")


def pubkey(b):
    """A key BIP-174 takes: 33 bytes compressed or 65 uncompressed, on
    the curve."""
    try:
        if len(b) == 33:
            btc.GE.from_bytes_compressed(b)
            return
        if len(b) == 65:
            btc.GE.from_bytes_uncompressed(b)
            return
    except (ValueError, AssertionError):
        pass
    raise Refused("not a public key")


def xonly(b):
    try:
        if len(b) == 32:
            btc.GE.from_bytes_xonly(b)
            return
    except (ValueError, AssertionError):
        pass
    raise Refused("not an x-only key")


def der_sig(b):
    """A DER ECDSA signature and its sighash byte (BIP-174's partial
    signature: the signature as a scriptSig pushes it), strict DER."""
    sig, ht = b[:-1], b[-1:] if b else b""
    if not ht or ht[0] not in (1, 2, 3, 0x81, 0x82, 0x83):
        raise Refused("not a standard sighash type")
    if len(sig) < 8 or sig[0] != 0x30 or sig[1] != len(sig) - 2:
        raise Refused("not DER")
    at, nums = 2, []
    for _ in range(2):
        if at + 2 > len(sig) or sig[at] != 0x02:
            raise Refused("not DER")
        n = sig[at + 1]
        v = sig[at + 2:at + 2 + n]
        if n == 0 or len(v) != n or v[0] & 0x80 or (n > 1 and v[0] == 0 and not v[1] & 0x80):
            raise Refused("not DER")
        nums.append(int.from_bytes(v, "big"))
        at += 2 + n
    if at != len(sig):
        raise Refused("not DER")


def schnorr_sig(b):
    """A BIP-371 signature: 64 bytes, or 65 with a sighash byte that is
    a BIP-341 type other than the default."""
    if len(b) == 64:
        return
    if len(b) == 65 and b[64] in (1, 2, 3, 0x81, 0x82, 0x83):
        return
    raise Refused("not a taproot signature")


def origin(v, at=0):
    if (len(v) - at) % 4 or len(v) - at < 4:
        raise Refused("not a key origin")


def tap_origin(v):
    n, at = compact(v, 0)
    at += 32 * n
    if at > len(v):
        raise Refused("short leaf hashes")
    origin(v, at)


def global_xpub(k, v):
    """BIP-174's PSBT_GLOBAL_XPUB: the 78-byte key, and an origin of as
    many indices as the key's depth."""
    if len(k) != 79:
        raise Refused("an xpub key of the wrong length")
    key = btc.decode(btc.b58check_encode(k[1:]), False)
    origin(v)
    if (len(v) - 4) // 4 != key.depth:
        raise Refused("an xpub whose origin is not as long as its depth")


def single(k):
    if len(k) != 1:
        raise Refused(f"key data on field {k[0]:#x}")


# Each known field: (version 0 allows it, version 2 allows it, check(key,
# value)). BIP-174, BIP-370 and BIP-371.
GLOBAL = {
    0x00: (True, False, lambda k, v: (single(k), read_unsigned(v))),
    0x01: (True, True, lambda k, v: global_xpub(k, v)),
    0x02: (False, True, lambda k, v: (single(k), _exact(v, 4))),
    0x03: (False, True, lambda k, v: (single(k), _exact(v, 4))),
    0x04: (False, True, lambda k, v: (single(k), _count(v))),
    0x05: (False, True, lambda k, v: (single(k), _count(v))),
    0x06: (False, True, lambda k, v: (single(k), _exact(v, 1))),
    0xfb: (True, True, lambda k, v: (single(k), _exact(v, 4))),
}
INPUT = {
    0x00: (True, True, lambda k, v: (single(k), read_tx(v))),
    0x01: (True, True, lambda k, v: (single(k), read_txout(v))),
    0x02: (True, True, lambda k, v: (pubkey(k[1:]), der_sig(v))),
    0x03: (True, True, lambda k, v: (single(k), _exact(v, 4))),
    0x04: (True, True, lambda k, v: single(k)),
    0x05: (True, True, lambda k, v: single(k)),
    0x06: (True, True, lambda k, v: (pubkey(k[1:]), origin(v))),
    0x07: (True, True, lambda k, v: single(k)),
    0x08: (True, True, lambda k, v: (single(k), read_witness(v))),
    0x0e: (False, True, lambda k, v: (single(k), _exact(v, 32))),
    0x0f: (False, True, lambda k, v: (single(k), _exact(v, 4))),
    0x10: (False, True, lambda k, v: (single(k), _exact(v, 4))),
    0x11: (False, True, lambda k, v: (single(k), _exact(v, 4),
                                      _at_least(v, 500_000_000))),
    0x12: (False, True, lambda k, v: (single(k), _exact(v, 4), _height(v))),
    0x13: (True, True, lambda k, v: (single(k), schnorr_sig(v))),
    0x14: (True, True, lambda k, v: (_exact(k, 65), xonly(k[1:33]), schnorr_sig(v))),
    0x15: (True, True, lambda k, v: (control_block(k[1:]), _leaf(v))),
    0x16: (True, True, lambda k, v: (xonly(k[1:]), tap_origin(v))),
    0x17: (True, True, lambda k, v: (single(k), xonly(v))),
    0x18: (True, True, lambda k, v: (single(k), _exact(v, 32))),
}
OUTPUT = {
    0x00: (True, True, lambda k, v: single(k)),
    0x01: (True, True, lambda k, v: single(k)),
    0x02: (True, True, lambda k, v: (pubkey(k[1:]), origin(v))),
    0x03: (False, True, lambda k, v: (single(k), _exact(v, 8))),
    0x04: (False, True, lambda k, v: single(k)),
    0x05: (True, True, lambda k, v: (single(k), xonly(v))),
    0x06: (True, True, lambda k, v: (single(k), tap_tree(v))),
    0x07: (True, True, lambda k, v: (xonly(k[1:]), tap_origin(v))),
}


def control_block(c):
    """BIP-341's control block: 33 + 32m bytes, m at most 128, its
    internal key on the curve."""
    if len(c) < 33 or (len(c) - 33) % 32 or (len(c) - 33) // 32 > 128:
        raise Refused("not a control block")
    xonly(c[1:33])


def _leaf(v):
    """A leaf script and its leaf version (BIP-371)."""
    if not v:
        raise Refused("no leaf version")


def tap_tree(v):
    """BIP-371's PSBT_OUT_TAP_TREE: leaves of depth, leaf version and
    script, depth-first, at least one."""
    at = 0
    if not v:
        raise Refused("an empty tree")
    while at < len(v):
        if at + 2 > len(v) or v[at] > 128:
            raise Refused("not a tree")
        n, at = compact(v, at + 2)
        at += n
        if at > len(v):
            raise Refused("short tree")


def _exact(v, n):
    if len(v) != n:
        raise Refused(f"a value of {len(v)} bytes, not {n}")


def _count(v):
    n, at = compact(v, 0)
    if at != len(v):
        raise Refused("not a count")


# Set while the reference reads as Faraday does (above).
LENIENT = [False]


def _at_least(v, low):
    if int.from_bytes(v, "little") < low:
        raise Refused("a time locktime below 500000000")


def _height(v):
    if not 0 < int.from_bytes(v, "little") < 500_000_000:
        raise Refused("a height locktime of 0 or past 499999999")


def check_map(pairs, table, version):
    """Each pair against its field's rule. Read as Faraday reads a
    version 2 PSBT (`LENIENT`), BIP-370's fields are checked, and of
    BIP-174's and BIP-371's only that a taproot signature's sighash byte
    is not 0; the others are checked when it is bridged to version 0."""
    for k, v in pairs:
        rule = table.get(k[0])
        if rule is None:
            continue
        if not rule[version == 2]:
            raise Refused(f"field {k[0]:#x} in a version {version} PSBT")
        if LENIENT[0] and version == 2 and (rule[0] or len(k) != 1):
            # Faraday's version 2 reader takes a BIP-370 key with key
            # data for another, unknown, key, and reads BIP-174's fields
            # when it is bridged to version 0, but for BIP-341's zero
            # sighash byte.
            if table is INPUT and (k == b"\x13" or k[0] == 0x14) and len(v) == 65 and v[64] == 0:
                raise Refused("a taproot signature with a sighash byte of 0")
            continue
        rule[2](k, v)


def get(pairs, ty):
    found = [v for k, v in pairs if k == bytes([ty])]
    return found[0] if found else None


def ref_v0(raw):
    """A version 0 PSBT read by BIP-174: its maps."""
    if raw[:5] != MAGIC:
        raise Refused("magic")
    glob, at = read_map(raw, 5)
    check_map(glob, GLOBAL, 0)
    version = get(glob, 0xfb)
    if version is not None and int.from_bytes(version, "little") != 0:
        raise Refused("not version 0")
    txraw = get(glob, 0x00)
    if txraw is None:
        raise Refused("no unsigned transaction")
    tx = read_unsigned(txraw)
    if any(i.scriptSig for i in tx.vin):
        raise Refused("the unsigned transaction carries a scriptSig or witness")
    maps = []
    for table, n in ((INPUT, len(tx.vin)), (OUTPUT, len(tx.vout))):
        for _ in range(n):
            m, at = read_map(raw, at)
            check_map(m, table, 0)
            maps.append(m)
    if at != len(raw):
        raise Refused("bytes after the last map")
    return {"maps": canonical([glob] + maps), "txid": tx.txid_hex}


def locktime(inputs, glob):
    """BIP-370's locktime determination."""
    stated = [(get(m, 0x12), get(m, 0x11)) for m in inputs]
    stated = [(h, t) for h, t in stated if h is not None or t is not None]
    if not stated:
        f = get(glob, 0x03)
        return int.from_bytes(f, "little") if f is not None else 0
    if all(h is not None for h, _ in stated):
        return max(int.from_bytes(h, "little") for h, _ in stated)
    if all(t is not None for _, t in stated):
        return max(int.from_bytes(t, "little") for _, t in stated)
    return None


def ref_v2(raw):
    """A version 2 PSBT read by BIP-370: its maps, its locktime and its
    transaction (none while an output has no script)."""
    if raw[:5] != MAGIC:
        raise Refused("magic")
    glob, at = read_map(raw, 5)
    check_map(glob, GLOBAL, 2)
    version = get(glob, 0xfb)
    if version is None or int.from_bytes(version, "little") != 2:
        raise Refused("not version 2")
    for ty in (0x02, 0x04, 0x05):
        if get(glob, ty) is None:
            raise Refused(f"no global field {ty:#x}")
    n_in = compact(get(glob, 0x04), 0)[0]
    n_out = compact(get(glob, 0x05), 0)[0]
    inputs, outputs = [], []
    for _ in range(n_in):
        m, at = read_map(raw, at)
        check_map(m, INPUT, 2)
        if get(m, 0x0e) is None or get(m, 0x0f) is None:
            raise Refused("an input without its previous output")
        inputs.append(m)
    for _ in range(n_out):
        m, at = read_map(raw, at)
        check_map(m, OUTPUT, 2)
        # BIP-375: a silent payment output (PSBT_OUT_SP_V0_INFO) has no
        # script until the shares give it.
        if get(m, 0x03) is None or (get(m, 0x04) is None and get(m, 0x09) is None):
            raise Refused("an output without its amount or script")
        outputs.append(m)
    if at != len(raw):
        raise Refused("bytes after the last map")
    lock = locktime(inputs, glob)
    tx = None
    if lock is not None and all(get(m, 0x04) for m in outputs):
        t = CTransaction()
        t.version = int.from_bytes(get(glob, 0x02), "little")
        t.nLockTime = lock
        for m in inputs:
            seq = get(m, 0x10)
            t.vin.append(CTxIn(COutPoint(int.from_bytes(get(m, 0x0e), "little"),
                                         int.from_bytes(get(m, 0x0f), "little")),
                               b"", int.from_bytes(seq, "little") if seq else 0xFFFFFFFF))
        for m in outputs:
            t.vout.append(CTxOut(int.from_bytes(get(m, 0x03), "little"), get(m, 0x04)))
        tx = t.serialize().hex()
    return {"maps": canonical([glob] + inputs + outputs), "locktime": lock, "tx": tx}


def canonical(maps):
    return [sorted([k.hex(), v.hex()] for k, v in m) for m in maps]


def answer_maps(answer, version, read):
    """Faraday's answer with its written bytes read back into maps, and
    whether they are the bytes it read."""
    if answer[0] != "ok":
        return ("err", None)
    out = dict(answer[1])
    raw = bytes.fromhex(out.pop("bytes"))
    out["as_read"] = raw == read
    try:
        glob, at = read_map(raw, 5)
        n_in = n_out = 0
        if version == 0:
            tx = read_unsigned(get(glob, 0x00))
            n_in, n_out = len(tx.vin), len(tx.vout)
        else:
            n_in, n_out = compact(get(glob, 0x04), 0)[0], compact(get(glob, 0x05), 0)[0]
        maps = [glob]
        for _ in range(n_in + n_out):
            m, at = read_map(raw, at)
            maps.append(m)
        out["maps"] = canonical(maps)
        out["trailing"] = raw[at:].hex()
    except Exception as e:  # noqa: BLE001 - Faraday wrote something unreadable
        out["maps"] = f"unreadable: {e}"
    return ("ok", out)


def ref_answer(answer):
    if answer[0] != "ok":
        return answer
    return ("ok", dict(answer[1], trailing="", as_read=True))


# Where Faraday reads a PSBT otherwise than the BIPs (README, "Known
# differences"), `LENIENT` reads it as Faraday does: a version 2 PSBT
# (Faraday's own reader) takes a BIP-370 key with key data for an
# unknown key, and checks BIP-174's and BIP-371's fields inside only when
# it is bridged to version 0 (but for a taproot signature whose sighash
# byte is 0, which it refuses on reading).


class Psbt(Area):
    name = "psbt"

    def read(self, raw, version, expected=None):
        """Compares with the BIPs' reading, or, where Faraday reads the
        PSBT otherwise (`LENIENT`), with Faraday's reading."""
        op = f"psbt.v{version}"
        inputs = {"psbt": raw.hex()}
        rust = answer_maps(self.rust.call(op, **inputs), version, raw)
        ref, known = (ref_answer(_try(lambda: (ref_v0 if version == 0 else ref_v2)(raw), lenient))
                      for lenient in (False, True))
        self.compared += 1
        if ref != known and _okerr(ref) != _okerr(known):
            self.known += 1
            if rust != known:
                raise Mismatch(self.name, op, inputs, rust, ref,
                               f"a known difference no longer holds; Faraday's reading is {known}")
            return rust
        if _okerr(rust) != _okerr(ref) or (rust[0] == "ok" and rust[1] != ref[1]):
            raise Mismatch(self.name, op, inputs, rust, ref)
        if expected is not None and _okerr(ref)[0] != expected:
            raise Mismatch(self.name, op, inputs, rust, ref,
                           f"both differ from the published vector ({expected})")
        return rust

    def published(self):
        for case in json.loads((ROOT / "tools/vectors/psbt-bips/vectors.json").read_text()):
            raw = bytes.fromhex(case["hex"])
            expect = case["expect"]
            v2 = raw[:5] == MAGIC and _says_v2(raw)
            if expect == "invalid":
                self.read(raw, 0, "err")
                self.read(raw, 2, "err")
            elif expect in ("valid", "signer-fails"):
                self.read(raw, 2 if v2 else 0, "ok")
            else:
                got = self.read(raw, 2, "ok")
                want = None if expect == "locktime none" else int(expect.split()[1])
                if got[0] == "ok" and got[1].get("locktime") != want:
                    raise AssertionError(f"{name}: locktime {got[1].get('locktime')}, "
                                         f"the BIP says {want}")

    # Random PSBTs.

    def key(self):
        return btc.point(self.rng.randrange(1, btc.N))

    def der(self):
        rng = self.rng

        def num(x):
            b = x.to_bytes((x.bit_length() + 7) // 8, "big")
            return b"\0" + b if b[0] & 0x80 else b
        r, s = (num(rng.randrange(1, btc.N)) for _ in range(2))
        body = b"\x02" + bytes([len(r)]) + r + b"\x02" + bytes([len(s)]) + s
        return b"\x30" + bytes([len(body)]) + body + bytes([rng.choice([1, 2, 3, 0x81, 0x82, 0x83])])

    def origin(self, n=None):
        n = self.rng.randint(0, 5) if n is None else n
        return self.randbytes(4) + b"".join(self.rng.randrange(2**32).to_bytes(4, "little")
                                            for _ in range(n))

    def script(self):
        return self.randbytes(self.rng.choice([0, 22, 25, 34, self.rng.randint(0, 80)]))

    def prev_tx(self, vout):
        rng = self.rng
        t = CTransaction()
        t.version = 2
        t.vin.append(CTxIn(COutPoint(rng.randrange(2**256), rng.randrange(4)), b"", 0xFFFFFFFF))
        for _ in range(vout + 1):
            t.vout.append(CTxOut(rng.randrange(10**10), self.script()))
        return t

    def unknown(self, kind):
        """An unknown or proprietary pair, which a reader keeps."""
        rng = self.rng
        if rng.random() < 0.5:
            ty = {"global": rng.choice([0x07, 0x08, 0x40, 0xfa]), "in": rng.choice([0x40, 0x7f, 0xf0]),
                  "out": rng.choice([0x40, 0x7f, 0xf0])}[kind]
            return bytes([ty]) + self.randbytes(rng.randint(0, 40)), self.randbytes(rng.randint(0, 40))
        ident = self.randbytes(rng.randint(0, 8))
        return (b"\xfc" + ser_compact(len(ident)) + ident + ser_compact(rng.randint(0, 300))
                + self.randbytes(rng.randint(0, 8)), self.randbytes(rng.randint(0, 40)))

    def tap_tree(self):
        """A BIP-371 PSBT_OUT_TAP_TREE: the leaves of a random complete
        binary tree of 1 to 6 leaves, depth first, each its depth, leaf
        version 0xc0 and a script."""
        rng = self.rng

        def leaves(n, depth):
            if n == 1:
                return [depth]
            left = rng.randint(1, n - 1)
            return leaves(left, depth + 1) + leaves(n - left, depth + 1)
        out = b""
        for depth in leaves(rng.randint(1, 6), 0):
            script = self.randbytes(rng.randint(1, 40))
            out += bytes([depth, 0xc0]) + ser_compact(len(script)) + script
        return out

    def random_v0(self):
        rng = self.rng
        tx = CTransaction()
        tx.version = rng.choice([1, 2])
        tx.nLockTime = rng.choice([0, rng.randrange(2**32)])
        inputs = []
        for _ in range(rng.randint(1, 4)):
            m = []
            if rng.random() < 0.4:
                vout = rng.randint(0, 2)
                prev = self.prev_tx(vout)
                tx.vin.append(CTxIn(COutPoint(prev.txid_int, vout), b"", rng.choice([0xFFFFFFFF, 0xFFFFFFFD])))
                m.append((b"\x00", prev.serialize_without_witness()))
            else:
                tx.vin.append(CTxIn(COutPoint(rng.randrange(2**256), rng.randrange(4)), b"",
                                    rng.choice([0xFFFFFFFF, 0xFFFFFFFD])))
            if rng.random() < 0.7:
                m.append((b"\x01", CTxOut(rng.randrange(10**10), self.script()).serialize()))
            for _ in range(rng.choice([0, 0, 1, 2])):
                m.append((b"\x02" + self.key(), self.der()))
            if rng.random() < 0.3:
                m.append((b"\x03", rng.choice([1, 2, 3, 0x81, 0x83, 0x41]).to_bytes(4, "little")))
            for ty in (0x04, 0x05, 0x07):
                if rng.random() < 0.2:
                    m.append((bytes([ty]), self.script()))
            for _ in range(rng.choice([0, 1, 2])):
                m.append((b"\x06" + self.key(), self.origin()))
            if rng.random() < 0.2:
                stack = [self.randbytes(rng.choice([0, 33, 72])) for _ in range(rng.randint(0, 3))]
                m.append((b"\x08", ser_compact(len(stack)) + b"".join(ser_compact(len(s)) + s
                                                                      for s in stack)))
            if rng.random() < 0.2:
                m.append((b"\x13", self.randbytes(64) + (bytes([rng.choice([1, 0x81])])
                                                         if rng.random() < 0.3 else b"")))
            if rng.random() < 0.3:
                x = self.key()[1:]
                m.append((b"\x17", x))
                hashes = [self.randbytes(32) for _ in range(rng.randint(0, 2))]
                m.append((b"\x16" + x, ser_compact(len(hashes)) + b"".join(hashes) + self.origin()))
            if rng.random() < 0.15:
                m.append((b"\x18", self.randbytes(32)))
            if rng.random() < 0.2:
                m.append(self.unknown("in"))
            rng.shuffle(m)
            inputs.append(m)
        outputs = []
        for _ in range(rng.randint(0, 4)):
            tx.vout.append(CTxOut(rng.randrange(10**10), self.script()))
            m = []
            for ty in (0x00, 0x01):
                if rng.random() < 0.2:
                    m.append((bytes([ty]), self.script()))
            for _ in range(rng.choice([0, 1])):
                m.append((b"\x02" + self.key(), self.origin()))
            if rng.random() < 0.2:
                m.append((b"\x05", self.key()[1:]))
            if rng.random() < 0.2:
                m.append((b"\x06", self.tap_tree()))
            if rng.random() < 0.1:
                m.append(self.unknown("out"))
            rng.shuffle(m)
            outputs.append(m)
        glob = [(b"\x00", tx.serialize_without_witness())]
        if rng.random() < 0.2:
            k = btc.Key(self.randbytes(32), None, self.key(), depth=rng.randint(1, 5),
                        parent=self.randbytes(4), child=rng.randrange(2**32))
            raw = btc.b58check_decode(k.serialize())
            glob.append((b"\x01" + raw, self.origin(k.depth)))
        if rng.random() < 0.2:
            glob.append((b"\xfb", (0).to_bytes(4, "little")))
        if rng.random() < 0.2:
            glob.append(self.unknown("global"))
        return glob, inputs, outputs

    def random_v2(self):
        rng = self.rng
        inputs, outputs = [], []
        for _ in range(rng.randint(1, 4)):
            m = [(b"\x0e", self.randbytes(32)), (b"\x0f", rng.randrange(4).to_bytes(4, "little"))]
            if rng.random() < 0.5:
                m.append((b"\x10", rng.choice([0xFFFFFFFF, 0xFFFFFFFD, 0]).to_bytes(4, "little")))
            kind = rng.randrange(4)
            if kind in (1, 3):
                m.append((b"\x12", rng.randint(1, 499_999_999).to_bytes(4, "little")))
            if kind in (2, 3):
                m.append((b"\x11", rng.randint(500_000_000, 2**32 - 1).to_bytes(4, "little")))
            if rng.random() < 0.5:
                m.append((b"\x01", CTxOut(rng.randrange(10**10), self.script()).serialize()))
            if rng.random() < 0.2:
                m.append((b"\x06" + self.key(), self.origin()))
            rng.shuffle(m)
            inputs.append(m)
        for _ in range(rng.randint(1, 4)):
            m = [(b"\x03", rng.randrange(10**10).to_bytes(8, "little")),
                 (b"\x04", self.randbytes(rng.choice([22, 34, 25])))]
            if rng.random() < 0.2:
                m.append((b"\x02" + self.key(), self.origin()))
            outputs.append(m)
        glob = [(b"\x02", rng.choice([2, 3]).to_bytes(4, "little")),
                (b"\x04", ser_compact(len(inputs))), (b"\x05", ser_compact(len(outputs))),
                (b"\xfb", (2).to_bytes(4, "little"))]
        if rng.random() < 0.5:
            glob.append((b"\x03", rng.randrange(2**32).to_bytes(4, "little")))
        if rng.random() < 0.3:
            glob.append((b"\x06", bytes([rng.randrange(8)])))
        return glob, inputs, outputs

    def broken(self, glob, inputs, outputs, version):
        """The PSBT with one BIP rule broken, as bytes."""
        rng = self.rng
        maps = [list(glob)] + [list(m) for m in inputs] + [list(m) for m in outputs]
        how = rng.randrange(10)
        target = rng.randrange(len(maps))
        if how == 0 and maps[target]:
            maps[target].append(rng.choice(maps[target]))
        elif how == 1:
            # Key data on a field that takes none.
            m = maps[target]
            singles = [i for i, (k, _) in enumerate(m) if len(k) == 1]
            if singles:
                i = rng.choice(singles)
                m[i] = (m[i][0] + b"\x00", m[i][1])
        elif how == 2:
            # A key of the wrong length.
            m = maps[target]
            keyed = [i for i, (k, _) in enumerate(m) if k[0] in (0x02, 0x06, 0x16) and len(k) > 1]
            if keyed:
                i = rng.choice(keyed)
                k, v = m[i]
                m[i] = (k[:-1] if rng.random() < 0.5 else k + b"\x00", v)
        elif how == 3:
            # A value of the wrong size.
            m = maps[target]
            sized = [i for i, (k, _) in enumerate(m) if k[0] in (0x01, 0x03, 0x0e, 0x0f, 0x10, 0x11,
                                                                0x12, 0x13, 0x17, 0x18)
                     and len(k) == 1]
            if sized:
                i = rng.choice(sized)
                k, v = m[i]
                m[i] = (k, v[:-1] if rng.random() < 0.5 else v + b"\x00")
        elif how == 4 and version == 0:
            # A scriptSig in the unsigned transaction.
            tx = read_unsigned(dict(maps[0])[b"\x00"])
            tx.vin[0].scriptSig = b"\x00"
            maps[0] = [(k, tx.serialize_without_witness() if k == b"\x00" else v) for k, v in maps[0]]
        elif how == 5:
            maps = maps[:-1] if len(maps) > 1 else maps
        elif how == 6:
            maps.append([])
        elif how == 7:
            return MAGIC + b"".join(write_map(m) for m in maps) + bytes([rng.randrange(256)])
        elif how == 8:
            raw = MAGIC + b"".join(write_map(m) for m in maps)
            return raw[:rng.randrange(5, len(raw))]
        else:
            return b"psbt\xfe" + b"".join(write_map(m) for m in maps)
        return MAGIC + b"".join(write_map(m) for m in maps)

    def corpus_case(self):
        rng = self.rng
        version = 0 if rng.random() < 0.7 else 2
        glob, inputs, outputs = self.random_v0() if version == 0 else self.random_v2()
        raw = MAGIC + write_map(glob) + b"".join(write_map(m) for m in inputs + outputs)
        self.read(raw, version)
        self.read(self.broken(glob, inputs, outputs, version), version)
        if rng.random() < 0.1:
            self.read(raw, 2 - version)

    LOOPS = (("corpus", 1500),)

    def prelude(self):
        self.published()

    def case_corpus(self):
        self.corpus_case()


def _says_v2(raw):
    try:
        glob, _ = read_map(raw, 5)
    except Refused:
        return False
    v = get(glob, 0xfb)
    return v is not None and int.from_bytes(v, "little") == 2


def _try(fn, lenient):
    LENIENT[0] = lenient
    try:
        return ("ok", fn())
    except Exception as e:  # noqa: BLE001 - every refusal is a refusal
        return ("err", f"{type(e).__name__}: {e}")
    finally:
        LENIENT[0] = False


def _okerr(answer):
    return (answer[0], None) if answer[0] == "err" else ("ok", None)
