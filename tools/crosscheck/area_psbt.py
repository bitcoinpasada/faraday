"""PSBTs against Bitcoin Core v31.1 itself for version 0, against HWI's
`psbt.py` (`tools/reference/hwi/`) for version 2, which Core refuses, and
against btclib-wallet's `Psbt` (btclib's own code, `tools/reference/btclib/`)
for both.

- Version 0, through `bitcoind` on regtest (`core_node.py`), one
  JSON-RPC batch a case. `decodepsbt` must accept what Faraday's
  `faraday_psbt::Psbt` accepts and refuse what it refuses, and give the
  txid Faraday gives. Core's `decodepsbt` of Faraday's own writing of
  what it read (`bitcoin`'s, which `to_bytes` uses for any field that
  changed) must be its `decodepsbt` of the PSBT, field for field, an
  output's taproot tree compared by the Merkle root Core's
  `taproot_tree_helper` gives it (BIP-371 lets a writer list the leaves
  from either side). Faraday must write back the bytes it read, and must
  read Core's own writing of the PSBT and write that back byte for byte.
  Core's writing is `combinepsbt` of the one PSBT: `CombinePSBTs` copies
  the first PSBT and merges nothing into it, so it is Core's reader then
  Core's writer. That writer is byte-faithful to a PSBT already in
  Core's form (all of BIP-174's and BIP-371's valid vectors but two,
  whose previous transactions carry a witness), not to every PSBT: it
  writes fields in its own order, sorts keyed fields and leaf hashes,
  writes a previous transaction without its witness, and leaves out a
  `PSBT_GLOBAL_VERSION` of 0 and an input's unfinished fields once it has
  final scripts. `utxoupdatepsbt` consults the node's UTXO set and
  `joinpsbts` needs two PSBTs, so neither is used.
- Version 2, through HWI's `PSBT` class (`hwilib/psbt.py` at a pinned
  commit), run as published: its reader must accept what
  `faraday_psbt::v2::PsbtV2` accepts and refuse what it refuses,
  Faraday's locktime must be HWI's `compute_lock_time`, its transaction
  HWI's `get_unsigned_tx`, and the version 0 PSBT it makes for the
  signer (`to_v0`) must read in Core as HWI's `convert_to_v0` written
  out by HWI reads in Core.
- Both versions, through btclib-wallet's `Psbt.parse` (`btclib_read`,
  with its `assert_valid`): it must read what Faraday reads and
  the other judge reads, and refuse what they refuse, of the version it
  is asked for. A version 2 PSBT's locktime and transaction must be its
  `lock_time` and `tx`, and the version 0 PSBT Faraday makes (`to_v0`)
  its `to_v0`; a version 0 PSBT made version 2 (`from_v0`) must be its
  `to_v2`, each map as a set of pairs with a field's leaf hashes sorted
  (btclib sorts them).
- Version 0 to version 2: every version 0 PSBT Faraday reads, made
  version 2 by Faraday (`PsbtV2::from_v0`), by HWI (`convert_to_v2`) and
  by btclib (`to_v2`), compared map by map as sets of pairs, leaving out
  the pairs HWI rewrites even without converting (its own writing of a
  field); and Faraday's made version 0 again (`to_v0`), which must be the
  bytes read where the unsigned transaction leads the global map, no
  `PSBT_GLOBAL_VERSION` is written and the transaction has an input.

Every file of the fuzz corpora's `psbt_diff`, `psbt_parse` and `psbt_v2`
folders (`fuzz/corpus/`), read as bytes or, where it is base64 text, as the
PSBT it spells, is a case of its own (`fuzz` loop), read as the version it
says and as the other and judged as every case above is, btclib included.
A file with no PSBT magic is counted and left out; the same bytes twice are
read once. A plain run reads those
committed seeds only, so it gives the same answer on every machine.
`FARADAY_FUZZ_CORPUS` (folders separated by `:`, one folder per target, as
`tools/fuzz-release.sh` reads it) and `FARADAY_FUZZ_STORE` (a fuzz store's
folders, separated by `:`, one folder per target) add the corpus a fuzz run
grew, when they name one: at most `FUZZ_GROWN_CAP` files of them, the
first by sorted name, the rest counted as left out. The run prints what it
read. The cases of a corpus are run at `--scale` 1 at most.

For a version 0 PSBT Faraday and Core both read, Core's `analyzepsbt` fee
(given only when every input's UTXO is known) must be the fee Faraday's
`inspect` shows, and Faraday must show none where Core gives none.

Every PSBT of BIP-174's, BIP-370's and BIP-371's Test Vectors sections
(`tools/vectors/psbt-bips/`) is read all three ways, and the side that
decides must give the BIP's answer; then random version 0 and 2 PSBTs with
every field Faraday reads or carries, in random order, and the same PSBTs
broken one rule at a time.

Still Faraday's own, and counted as known differences:

- A version 0 PSBT with an input whose UTXO script is provably unspendable:
  Core's `analyzepsbt` refuses it ("spends unspendable output") and gives no
  fee; Faraday shows the fee.
- A version 0 PSBT whose outputs are worth more than its inputs (or that has
  no inputs): Core's `analyzepsbt` gives a negative fee; Faraday shows none.
- Core reads a version 0 PSBT with version 2 fields as valid, the fields
  as unknown; HWI refuses it, as BIP-370 does, and decides it (btclib
  refuses it too).
- A taproot signature of 65 bytes whose sighash byte is 0, which neither
  Core nor HWI refuses, and bytes after a version 2 PSBT's last map or a
  stream that ends inside one, which HWI reads: the BIPs refuse them, as
  Faraday does, and btclib refuses each, which decides them. The area
  holds no rule of its own for them.
- Faraday's version 2 reader leaves BIP-174's and BIP-371's fields to the
  bridge to version 0 and refuses a BIP-370 key with key data, as HWI
  does. Where HWI refuses a PSBT Faraday reads, HWI must read it once those
  pairs are taken out (`faraday_leaves`), to the same locktime and
  transaction, and so must btclib. Where HWI takes a field's value
  shorter than its length says and writes it out repaired, and Faraday
  makes no version 0 PSBT, Core must refuse the version 0 PSBT of the
  PSBT's own pairs (`v0_of_pairs`).
- btclib asks more of a field than the BIPs: an ECDSA signature in
  strict DER with a low S and an r that is a point's x, a sighash type
  of the seven it defines, no BIP-375 field in a version 0 global map,
  a previous transaction and a witness UTXO that agree. A small share
  of the corpus's PSBTs (`exotic`) carries fields of those kinds, and
  btclib is not asked of them or of what is made from them. It also
  reads some PSBTs Core refuses (a key with key data on a field that
  takes none), so it is asked to refuse only what Core and HWI read and
  Faraday refuses.
  Faraday refuses.

The fuzz corpora's PSBTs add these. Each is a rule of its own in the judge
(`core_rule`, `hwi_rule`, `btclib_rule`), applied only where the judge's error
text and the property of the input both hold; a refusal no rule states fails
the area. Every one is a PSBT Faraday reads and then blocks at use, or only
carries; none is signed.

- Core's `decodepsbt` refuses "Non-witness UTXO does not match outpoint hash"
  (an input's previous transaction is not the spent output's txid) and "Input
  specifies output index that does not exist" (it has no output at the spent
  index); Faraday reads the PSBT, leaves that input's amount and the fee
  unknown and blocks the input, so it is never signed.
- Core refuses a key type, or a proprietary key's subtype, that is a compact
  size over its limit ("ReadCompactSize(): size too large") or cut short by the
  key's end ("SpanReader::read(): end of data"); Faraday, as `bitcoin` does,
  reads the type as one byte and only carries the pair, which it never reads.
  For a version 2 PSBT Core refuses both version 0 forms alike.
- Core ("... musig2 ...") and HWI ("Musig2") refuse a MuSig2 field (BIP-373:
  input 0x1a, 0x1b, 0x1c, output 0x08) whose key data is of a size BIP-373
  forbids, and so does Faraday where it uses the field (`musig.rs`): a hold on
  the input, or an ordinary payment for an output, not at reading.
- HWI refuses a key whose type is a compact size cut short ("unpack requires a
  buffer of N bytes"), a transaction version of 2^31 or more ("'i' format
  requires ..."; `struct.pack` of a signed version), and an output script that
  is present and empty ("PSBT_OUTPUT_SCRIPT is required in PSBTv2"; BIP-370's
  script is the scriptPubKey, which is empty); Faraday reads all three. HWI
  reads a version 2 PSBT in which Faraday's reader refuses a compact size not
  written in its shortest form, as Core does, and btclib decides it.
- btclib refuses an amount over 21,000,000 BTC ("satoshi amount"), a witness
  UTXO that is not the previous transaction's output at the spent index
  ("mismatched witness utxo"), a hash preimage that does not hash to its key,
  and a BIP-375 field (any map) of a version 0 PSBT; Faraday reads them and
  blocks the amount, the disagreement and the preimage at use.
- A version 2 PSBT HWI makes a version 0 PSBT of, Core reads, and Faraday makes
  none of: a key origin of more than 16 steps (Faraday's own limit, no wallet's
  path is longer) or a hash preimage that does not hash to its key (the bridge
  to version 0 checks it).
- Faraday refuses; Core, HWI and btclib read: an x-only key that is not a point's
  x (an input's taproot internal key, 0x17, or key origin, 0x16, an output's 0x05
  or 0x07; "invalid xonly public key"), which `bitcoin` checks as a key.
- Faraday refuses; HWI reads: a version 2 transaction with no input (with or
  without outputs), for which HWI makes a version 0 PSBT and Core reads it and Faraday's
  bridge to version 0 fails ("Bridge").
- HWI refuses; Faraday reads: an output with a silent payment's info (BIP-375,
  0x09) and no script ("PSBT_OUTPUT_SCRIPT is required"); BIP-375 leaves the
  script to the signer.
- A version 2 transaction with no input: `bitcoin` writes it with the segwit
  marker `0001` after the version, HWI and BIP-174 without; the two are
  compared with the marker taken out, also in the version 0 form Faraday makes.
"""

import base64
import hashlib
import io
import json
import os
import pathlib
import signal
import sys
from decimal import Decimal

import core_node
from common import ROOT, Area, Mismatch, use_reference

use_reference("core")
from test_framework.crypto.ripemd160 import ripemd160  # noqa: E402
from test_framework.crypto.secp256k1 import FAST_G, GE  # noqa: E402
from test_framework.messages import (  # noqa: E402
    COutPoint, CTransaction, CTxIn, CTxOut, deser_vector, ser_uint256)
from test_framework.script import taproot_tree_helper  # noqa: E402

use_reference("hwi")
# btclib's wheels, with libsecp256k1 turned off (`lineage.py`).
import lineage  # noqa: E402,F401
from btclib.exceptions import BTClibValueError  # noqa: E402
from btclib_wallet.psbt.psbt import Psbt as BtclibPsbt  # noqa: E402
# Last on the path: HWI imports only `typing_extensions.Protocol`, for
# type hints, and a worker's other areas keep the real package where the
# machine has it.
sys.path.append(str(ROOT / "tools" / "reference" / "hwi" / "shims"))
from hwilib.errors import PSBTSerializationError  # noqa: E402
from hwilib.psbt import PSBT as HwiPsbt  # noqa: E402

MAGIC = b"psbt\xff"
N = GE.ORDER


class Refused(Exception):
    pass


# ----- the fuzz corpora's PSBTs ------------------------------------------------

FUZZ_TARGETS = ("psbt_diff", "psbt_parse", "psbt_v2")
# At most this many files of the grown corpora (`FARADAY_FUZZ_CORPUS` and the
# kept store) are read: their names sorted, the first ones, so the same run
# reads the same files. The committed seeds are always all read.
FUZZ_GROWN_CAP = 3000


def _fuzz_folders():
    """(kind, folder) of every folder read: the committed seeds, then each
    `FARADAY_FUZZ_CORPUS` folder and each `FARADAY_FUZZ_STORE` folder
    (separated by `:`), each as it holds a folder per target. With neither
    variable set only the committed seeds are read."""
    out = [("seeds", ROOT / "fuzz" / "corpus" / t) for t in FUZZ_TARGETS]
    grown = [pathlib.Path(d) for d in os.environ.get("FARADAY_FUZZ_CORPUS", "").split(":") if d]
    grown += [pathlib.Path(d) for d in os.environ.get("FARADAY_FUZZ_STORE", "").split(":") if d]
    for d in grown:
        out += [("grown", d / t) for t in FUZZ_TARGETS]
    return [(k, d) for k, d in out if d.is_dir()]


def fuzz_psbts():
    """The corpus PSBTs, in a fixed order, as (name, bytes): each file read
    as bytes, or as base64 text where it is that; a file that is no PSBT
    (no magic either way) is counted and left out. Returns the list and a
    dict of counts: files of each kind, PSBTs, not PSBTs, grown files left
    out by the cap, and PSBTs already read (the same bytes)."""
    counts = {"seeds": 0, "grown": 0, "psbts": 0, "not_psbts": 0, "left_out": 0,
              "duplicates": 0}
    found, seen = [], set()
    grown_read = 0
    for kind, folder in _fuzz_folders():
        files = sorted(f for f in folder.iterdir() if f.is_file())
        if kind == "grown":
            room = max(0, FUZZ_GROWN_CAP - grown_read)
            counts["left_out"] += max(0, len(files) - room)
            files = files[:room]
            grown_read += len(files)
        for f in files:
            counts[kind] += 1
            raw = f.read_bytes()
            if not raw.startswith(MAGIC):
                try:
                    raw = base64.b64decode(b"".join(raw.split()), validate=True)
                except ValueError:
                    raw = b""
            if not raw.startswith(MAGIC):
                counts["not_psbts"] += 1
                continue
            h = hashlib.sha256(raw).hexdigest()
            if h in seen:
                counts["duplicates"] += 1
                continue
            seen.add(h)
            found.append((f"{folder.parent.name}/{folder.name}/{f.name}", raw))
    counts["psbts"] = len(found)
    return found, counts


FUZZ = fuzz_psbts()


# ----- the container: written here, to build PSBTs and to take them apart
# for the comparisons below ---------------------------------------------------


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
    """One map's pairs in order, and where it ends. A key twice is
    refused (BIP-174: keys within a map are unique)."""
    pairs = []
    while True:
        n, at = compact(b, at)
        if n == 0:
            if len({k for k, _ in pairs}) != len(pairs):
                raise Refused("a key twice in a map")
            return pairs, at
        if at + n > len(b):
            raise Refused("short")
        key, at = b[at:at + n], at + n
        m, at = compact(b, at)
        if at + m > len(b):
            raise Refused("short")
        pairs.append((bytes(key), bytes(b[at:at + m])))
        at += m


def write_map(pairs):
    return b"".join(ser_compact(len(k)) + k + ser_compact(len(v)) + v for k, v in pairs) + b"\0"


def read_unsigned(raw):
    """The unsigned transaction, which BIP-174 serialises without the
    witness format (`area_sign.py` reads its PSBTs with this too)."""
    tx = CTransaction()
    f = io.BytesIO(raw)
    tx.version = int.from_bytes(f.read(4), "little")
    tx.vin = deser_vector(f, CTxIn)
    tx.vout = deser_vector(f, CTxOut)
    tx.nLockTime = int.from_bytes(f.read(4), "little")
    if f.read() or tx.serialize_without_witness() != raw:
        raise Refused("not exactly a transaction")
    return tx


def get(pairs, ty):
    found = [v for k, v in pairs if k == bytes([ty])]
    return found[0] if found else None


def split(raw, n_in=None, n_out=None):
    """The maps, global first; a version 2 PSBT's counts are read from
    its global map. Refused when the stream ends inside a map or bytes
    follow the last (the area builds and takes apart PSBTs with it; it
    is not a judge)."""
    glob, at = read_map(raw, len(MAGIC))
    if n_in is None:
        n_in = compact(get(glob, 0x04) or b"\0", 0)[0]
        n_out = compact(get(glob, 0x05) or b"\0", 0)[0]
    maps = [glob]
    for _ in range(n_in + n_out):
        m, at = read_map(raw, at)
        maps.append(m)
    if at != len(raw):
        raise Refused("bytes after the last map")
    return maps, n_in


# Version 2 only (BIP-375): global, input and output field types, which
# `to_v0` leaves out.
BIP375_V2 = ({0x07, 0x08}, {0x1d, 0x1e}, {0x09, 0x0a})
# Version 2 only (BIP-370): global, input and output field types.
GLOBAL_V2 = {0x02, 0x03, 0x04, 0x05, 0x06}
INPUT_V2 = {0x0e, 0x0f, 0x10, 0x11, 0x12}
OUTPUT_V2 = {0x03, 0x04}
# The fields version 0 also has (BIP-174 and BIP-371), which Faraday's
# version 2 reader leaves to the bridge to version 0.
GLOBAL_SHARED = {0x00, 0x01}
INPUT_SHARED = set(range(0x00, 0x09)) | set(range(0x13, 0x19))
OUTPUT_SHARED = {0x00, 0x01, 0x02, 0x05, 0x06, 0x07}


def faraday_leaves(raw):
    """The version 2 PSBT without the pairs Faraday's reader does not
    read: BIP-174's and BIP-371's fields (a BIP-370 key with key data is
    left out too, though Faraday refuses a PSBT that holds one). None
    when it cannot be split."""
    try:
        maps, n_in = split(raw)
    except Refused:
        return None
    out = []
    for i, m in enumerate(maps):
        shared, v2 = ((GLOBAL_SHARED, GLOBAL_V2) if i == 0 else
                      (INPUT_SHARED, INPUT_V2) if i <= n_in else (OUTPUT_SHARED, OUTPUT_V2))
        out.append([(k, v) for k, v in m
                    if k[0] not in shared and not (k[0] in v2 and len(k) != 1)])
    return MAGIC + b"".join(write_map(m) for m in out)


def v0_of_pairs(raw, tx_hex):
    """The version 0 PSBT of a version 2 PSBT's own pairs, as BIP-370
    bridges one: its version 2 fields and `PSBT_GLOBAL_VERSION` left
    out, the unsigned transaction put in first."""
    maps, n_in = split(raw)
    out = []
    for i, m in enumerate(maps):
        drop = GLOBAL_V2 | {0xfb} if i == 0 else INPUT_V2 if i <= n_in else OUTPUT_V2
        out.append([(k, v) for k, v in m if k[0] not in drop])
    out[0].insert(0, (b"\x00", bytes.fromhex(tx_hex)))
    return MAGIC + b"".join(write_map(m) for m in out)


# ----- the outside readers --------------------------------------------------


def core_batch(calls):
    """Bitcoin Core's answers to [(method, params)], in one JSON-RPC
    batch: ("ok", result) or ("err", message) each."""
    node = core_node.rpc()
    reqs = [getattr(node, method).get_request(*params) for method, params in calls]
    by_id = {r["id"]: r for r in node.batch(reqs)}
    out = []
    for req in reqs:
        r = by_id[req["id"]]
        out.append(("err", r["error"]["message"]) if r.get("error") else ("ok", r["result"]))
    return out


def tree_root(leaves):
    """The Merkle root of a `PSBT_OUT_TAP_TREE` as `decodepsbt` lists
    it (leaves depth first), hashed by Core's `taproot_tree_helper`. The
    order of leaves is the writer's (BIP-371: depth first, from either
    side), so two writings of one tree are compared by this."""
    def nest(i, depth):
        leaf = leaves[i]
        if leaf["depth"] == depth:
            return ("leaf", bytes.fromhex(leaf["script"]), leaf["leaf_ver"]), i + 1
        left, i = nest(i, depth + 1)
        right, i = nest(i, depth + 1)
        return [left, right], i
    tree, end = nest(0, 0)
    if end != len(leaves):
        raise Refused("leaves after the tree")
    return taproot_tree_helper([tree])[1].hex()


def core_view(answer):
    """Core's `decodepsbt`, as compared: amounts as text, and each
    output's taproot tree as its Merkle root."""
    if answer[0] != "ok":
        return answer
    view = json.loads(json.dumps(answer[1], default=str))
    for out in view.get("outputs", []):
        if "taproot_tree" in out:
            out["taproot_tree"] = tree_root(out["taproot_tree"])
    return ("ok", view)


def b64(raw):
    return base64.b64encode(raw).decode()


def hwi_read(raw):
    """HWI's `PSBT`, read from the bytes."""
    p = HwiPsbt()
    p.deserialize(b64(raw))
    return p


def hwi_v2(raw):
    """HWI's reading of a version 2 PSBT: its locktime, its unsigned
    transaction and the version 0 PSBT `convert_to_v0` makes of it, all
    None where `compute_lock_time` finds the inputs' required locktimes
    cannot be met together."""
    p = hwi_read(raw)
    if p.version != 2:
        raise Refused(f"HWI reads version {p.version}")
    try:
        tx = p.get_unsigned_tx()
    except PSBTSerializationError:
        return {"locktime": None, "tx": None, "v0": None}
    p.convert_to_v0()
    return {"locktime": tx.nLockTime, "tx": tx.serialize_without_witness().hex(),
            "v0": base64.b64decode(p.serialize()).hex()}


# HWI's refusal of BIP-370's version 2 fields in a version 0 PSBT, which
# Core reads as unknown fields.
HWI_V2_IN_V0 = "is not allowed in PSBTv0"


def btclib_read(raw, version):
    """btclib-wallet's `Psbt.parse` of the bytes, with its `assert_valid`,
    of the version asked for: a PSBT of the other version is refused, as
    the readers of the other versions refuse it. A version 2 PSBT whose
    inputs' required locktimes cannot be met together is a PSBT (BIP-370
    has no transaction for it): btclib's validity check refuses it, so it
    is read again without the check."""
    try:
        p = BtclibPsbt.parse(raw)
    except BTClibValueError as e:
        if "no lock time satisfies every input" not in str(e):
            raise
        p = BtclibPsbt.parse(raw, check_validity=False)
    if p.version != version:
        raise Refused(f"btclib reads version {p.version}")
    return p


def btclib_lock(p):
    """The locktime btclib computes, None where the inputs' required
    locktimes cannot be met together."""
    try:
        return p.lock_time
    except Exception as e:  # noqa: BLE001
        if "no lock time satisfies every input" in str(e):
            return None
        raise


def sorted_leaves(pairs):
    """A map as a set of pairs, a `PSBT_IN_TAP_BIP32_DERIVATION`'s leaf
    hashes in order (btclib writes them sorted, BIP-371 lets a writer
    list them as it likes)."""
    out = set()
    for k, v in pairs:
        if k[:1] == b"\x16":
            n, at = compact(v, 0)
            if n > len(v) // 32:
                out.add((k, v))  # a count the value cannot hold: nothing to sort
                continue
            hashes = sorted(v[at + 32 * i:at + 32 * i + 32] for i in range(n))
            v = ser_compact(n) + b"".join(hashes) + v[at + 32 * n:]
        out.add((k, v))
    return out


def btclib_maps(p, n_in, n_out, version):
    """btclib's PSBT, serialised and taken apart: its maps as sets of
    pairs."""
    raw = p.serialize()
    maps = split(raw, n_in, n_out)[0] if version == 0 else split(raw)[0]
    return [sorted_leaves(m) for m in maps]


def unrewritten(read, back, ours, theirs):
    """The pairs of `ours` and `theirs`, map by map, that differ between
    them once the pairs btclib does not write back as read (its own
    writing of a field, an empty final witness among them) and what it
    writes in their place are left out of both sides: `read` the maps as
    read, `back` btclib's writing of them, each a list of sets."""
    return [(o - (a - b)) ^ (t - (b - a)) for a, b, o, t in zip(read, back, ours, theirs)]


def seen(answer):
    """An `outcome` of btclib's, as a report shows it."""
    return ("ok", "read") if answer[0] == "ok" else answer


def outcome(fn):
    try:
        return ("ok", fn())
    except Exception as e:  # noqa: BLE001 - every refusal is a refusal
        return ("err", f"{type(e).__name__}: {e}")


# ----- the known differences, as rules --------------------------------------
#
# A rule names a judge, the judge's answer (its error text, and a property of
# the input) and Faraday's answer; it applies only when both hold. Each
# function takes the judge's error text and the PSBT's bytes and gives the
# rule's name, or None: a refusal no rule states is a disagreement.

# Seconds one corpus case may take, every judge and helper together.
CASE_SECONDS = 120
# Faraday's text where `to_v0` makes nothing of a transaction with no input.
V0_NONE = "Bridge"
MAX_MONEY = 21_000_000 * 10**8
# Bitcoin Core's limit on a compact size it reads (`MAX_SIZE`).
CORE_MAX_SIZE = 0x02000000


def maps_of(raw):
    """The PSBT's maps, global first, and its input count; None when it
    cannot be taken apart."""
    try:
        if _says_v2(raw):
            return split(raw)
        glob, _ = read_map(raw, len(MAGIC))
        tx = read_unsigned(get(glob, 0))
        return split(raw, len(tx.vin), len(tx.vout))
    except Exception:  # noqa: BLE001 - a PSBT that is not taken apart has no property
        return None


def pairs_of(raw):
    """Every pair of every map as (kind, index, key, value), kind
    "global", "input" or "output"; empty when the PSBT is not taken apart."""
    got = maps_of(raw)
    if got is None:
        return []
    maps, n_in = got
    return [("global" if i == 0 else "input" if i <= n_in else "output", i, k, v)
            for i, m in enumerate(maps) for k, v in m]


def _size_at(b, at):
    """The compact size at `at` as Core reads it: "cut" when the bytes end
    inside it, else its value."""
    if at >= len(b):
        return "cut"
    n = b[at]
    if n < 0xfd:
        return n
    size = {0xfd: 2, 0xfe: 4, 0xff: 8}[n]
    if at + 1 + size > len(b):
        return "cut"
    return int.from_bytes(b[at + 1:at + 1 + size], "little")


def _type_sizes(key):
    """The compact sizes Core reads from a key as its type: the type, and
    for a proprietary key (0xfc) the prefix's length and the subtype."""
    out = [_size_at(key, 0)]
    if key[:1] == b"\xfc":
        at = 1
        n = _size_at(key, at)
        out.append(n)
        if n != "cut":
            at += (1 if n < 0xfd else 1 + {0xfd: 2, 0xfe: 4, 0xff: 8}[key[at]]) + n
            out.append(_size_at(key, at))
    return out


def core_wide_type(raw, text):
    """Core reads a key's type, and a proprietary key's subtype, as a
    compact size: it refuses one over its limit ("size too large") or cut
    short by the key's end ("end of data"). Faraday's reader (and
    `bitcoin`'s) reads the type as one byte and only carries the pair."""
    want = ("size too large" if "ReadCompactSize(): size too large" in text else
            "end of data" if "SpanReader::read(): end of data" in text else None)
    if want is None:
        return False
    for _, _, key, _ in pairs_of(raw):
        for n in _type_sizes(key):
            if (n == "cut") if want == "end of data" else (n != "cut" and n > CORE_MAX_SIZE):
                return True
    return False


def musig_key_wrong(raw):
    """A pair of a MuSig2 type (BIP-373) whose key data is not of a size it
    allows: an aggregate key of 33 bytes for the participants fields
    (input 0x1a, output 0x08), 33 and 33 or 33 more bytes for a public nonce
    or a partial signature (input 0x1b, 0x1c; the key is the type byte and
    its data)."""
    for kind, _, key, value in pairs_of(raw):
        if kind == "input" and key[:1] == b"\x1a" and len(key) != 34:
            return True
        if kind == "input" and key[:1] in (b"\x1b", b"\x1c") and len(key) not in (67, 99):
            return True
        if kind == "output" and key[:1] == b"\x08" and len(key) != 34:
            return True
        # the values: participants are 33-byte keys, a nonce 66 bytes, a
        # partial signature 32
        if key[:1] in ((b"\x1a",) if kind == "input" else (b"\x08",) if kind == "output" else ()):
            if not value or len(value) % 33:
                return True
        if kind == "input" and key[:1] == b"\x1b" and len(value) != 66:
            return True
        if kind == "input" and key[:1] == b"\x1c" and len(value) != 32:
            return True
    return False


def _sum_over(value):
    return value < 0 or value > MAX_MONEY


def _prev_tx(value):
    tx = CTransaction()
    tx.deserialize(io.BytesIO(value))
    return tx


def amount_over(raw):
    """An amount over 21,000,000 BTC (or one that reads negative): an
    output of the unsigned transaction, a version 2 output's amount, a
    witness UTXO's, or an output of a previous transaction."""
    got = maps_of(raw)
    if got is None:
        return False
    maps, n_in = got
    glob = get(maps[0], 0)
    if glob is not None:
        try:
            if any(_sum_over(o.nValue) for o in read_unsigned(glob).vout):
                return True
        except Exception:  # noqa: BLE001
            pass
    for i, m in enumerate(maps[1:], 1):
        for k, v in m:
            if i > n_in and k == b"\x03" and len(v) == 8:
                if _sum_over(int.from_bytes(v, "little", signed=True)):
                    return True
            if i <= n_in and k == b"\x01" and len(v) >= 8:
                if _sum_over(int.from_bytes(v[:8], "little", signed=True)):
                    return True
            if i <= n_in and k == b"\x00":
                try:
                    if any(_sum_over(o.nValue) for o in _prev_tx(v).vout):
                        return True
                except Exception:  # noqa: BLE001
                    pass
    return False


def utxos_disagree(raw):
    """An input with a previous transaction and a witness UTXO, the
    witness UTXO not the previous transaction's output at the spent index."""
    got = maps_of(raw)
    if got is None:
        return False
    maps, n_in = got
    v2 = _says_v2(raw)
    spent = None
    if not v2:
        try:
            spent = [i.prevout.n for i in read_unsigned(get(maps[0], 0)).vin]
        except Exception:  # noqa: BLE001
            return False
    for i, m in enumerate(maps[1:n_in + 1]):
        prev, wit = get(m, 0x00), get(m, 0x01)
        if prev is None or wit is None:
            continue
        n = spent[i] if spent is not None else int.from_bytes(get(m, 0x0f) or b"", "little")
        try:
            outs = _prev_tx(prev).vout
            if n >= len(outs) or outs[n].serialize() != wit:
                return True
        except Exception:  # noqa: BLE001
            continue
    return False


def preimage_wrong(raw):
    """An input's hash preimage (types 0x0a RIPEMD160, 0x0b SHA256, 0x0c
    HASH160, 0x0d HASH256) that does not hash to its key's data."""
    for kind, _, key, value in pairs_of(raw):
        if kind != "input" or key[:1] not in (b"\x0a", b"\x0b", b"\x0c", b"\x0d"):
            continue
        sha = hashlib.sha256(value).digest()
        want = {b"\x0a": ripemd160(value), b"\x0b": sha, b"\x0c": ripemd160(sha),
                b"\x0d": hashlib.sha256(sha).digest()}[key[:1]]
        if key[1:] != want:
            return True
    return False


def bip375_in_v0(raw):
    """A pair of a type BIP-375 defines (`BIP375_V2`) in a version 0 PSBT, in
    any map."""
    if _says_v2(raw):
        return False
    return any(key[:1] and key[0] in BIP375_V2[0 if kind == "global" else 1 if kind == "input" else 2]
               for kind, _, key, _ in pairs_of(raw))


def bip375_pair(raw):
    """A pair of a type BIP-375 defines, in a PSBT of either version (the
    global 0x09 included)."""
    types = {"global": {0x07, 0x08, 0x09}, "input": {0x1d, 0x1e}, "output": {0x09, 0x0a}}
    return any(key[0] in types[kind] for kind, _, key, _ in pairs_of(raw))


def long_origin(raw):
    """A key origin (a global xpub 0x01, an input's 0x06 or taproot 0x16, an
    output's 0x02 or taproot 0x07) of more than 16 steps."""
    origins = {("global", 0x01), ("input", 0x06), ("input", 0x16), ("output", 0x02),
               ("output", 0x07)}
    for kind, _, key, value in pairs_of(raw):
        ty = key[:1]
        if (kind, key[0] if key else None) not in origins:
            continue
        if ty in (b"\x16", b"\x07"):  # leaf hashes first
            n = _size_at(value, 0)
            if n == "cut":
                continue
            value = value[(1 if n < 0xfd else 1 + {0xfd: 2, 0xfe: 4, 0xff: 8}[value[0]]) + 32 * n:]
        if len(value) >= 4 and (len(value) - 4) % 4 == 0 and (len(value) - 4) // 4 > 16:
            return True
    return False


def tx_version_high(raw):
    """A version 2 PSBT's transaction version (global 0x02) of 2^31 or more."""
    got = maps_of(raw)
    v = get(got[0][0], 0x02) if got else None
    return v is not None and len(v) == 4 and int.from_bytes(v, "little") >= 2**31


def empty_output_script(raw):
    """A version 2 output's PSBT_OUT_SCRIPT (0x04) that is present and empty."""
    return any(kind == "output" and key == b"\x04" and value == b""
               for kind, _, key, value in pairs_of(raw))


def key_type_cut(raw):
    """A key whose type is a compact size cut short by the key's end (0xfd,
    0xfe or 0xff and fewer bytes than that form takes)."""
    return any(_size_at(key, 0) == "cut" for _, _, key, _ in pairs_of(raw) if key)


def no_inputs(raw):
    """True for a version 2 PSBT of a transaction with no input."""
    got = maps_of(raw)
    return got is not None and got[1] == 0


def unmarked_tx(tx):
    """A transaction (hex) of no input as HWI and BIP-174 write it, with no
    segwit marker: `bitcoin` writes one of no input with the marker `0001`
    after the version, which is the same transaction."""
    return tx[:8] + tx[12:] if tx[8:12] == "0001" else tx


def unmarked_v0(raw):
    """The version 0 PSBT (bytes) with its unsigned transaction of no input
    written without the marker."""
    glob, at = read_map(raw, len(MAGIC))
    tx = get(glob, 0)
    if tx is None or tx[4:6] != b"\x00\x01":
        return raw
    glob = [(k, tx[:4] + tx[6:] if k == b"\x00" else v) for k, v in glob]
    return MAGIC + write_map(glob) + raw[at:]


def core_rule(raw, text):
    """Bitcoin Core refuses a version 0 PSBT (`decodepsbt`'s `text`) that
    Faraday reads."""
    if "Non-witness UTXO does not match outpoint hash" in text:
        # Faraday reads it, shows that input's amount and the fee as unknown
        # and blocks it; it never signs it.
        return "core: a previous transaction that is not the spent output's"
    if "Input specifies output index that does not exist" in text:
        return "core: a previous transaction with no output at the spent index"
    if core_wide_type(raw, text):
        return "core: a key type or proprietary subtype as a compact size over its limit"
    if "musig2" in text.lower() and musig_key_wrong(raw):
        return "core: a MuSig2 field's key data of a size BIP-373 forbids"
    return None


def _is_x(b):
    """Whether 32 bytes are the x of a point on secp256k1."""
    p = 2**256 - 2**32 - 977
    x = int.from_bytes(b, "big")
    if len(b) != 32 or x >= p:
        return False
    rhs = (pow(x, 3, p) + 7) % p
    return pow(pow(rhs, (p + 1) // 4, p), 2, p) == rhs


def xonly_invalid(raw):
    """An x-only key that is not a point's x, where `bitcoin` reads one as a
    key: an input's taproot internal key (value of 0x17) or key origin (key
    data of 0x16), an output's (value of 0x05, key data of 0x07)."""
    for kind, _, key, value in pairs_of(raw):
        ty = key[0] if key else None
        if kind == "input" and ty == 0x17 and len(value) == 32 and not _is_x(value):
            return True
        if kind == "output" and ty == 0x05 and len(value) == 32 and not _is_x(value):
            return True
        if (kind, ty) in (("input", 0x16), ("output", 0x07)) and len(key) == 33 \
                and not _is_x(key[1:]):
            return True
    return False


def silent_output_without_script(raw):
    """A version 2 PSBT with an output that has a silent payment's info
    (BIP-375, `PSBT_OUT_SP_V0_INFO`, 0x09) and no script (0x04)."""
    got = maps_of(raw)
    if got is None:
        return False
    maps, n_in = got
    return any(any(k[:1] == b"\x09" for k, _ in m) and not any(k == b"\x04" for k, _ in m)
               for m in maps[1 + n_in:])


def origins_repeat(raw):
    """A map with two BIP-32 derivation pairs (an input's 0x06, an output's
    0x02) of the same value, whatever their keys."""
    seen_values = set()
    for kind, i, key, value in pairs_of(raw):
        if (kind, key[:1]) in (("input", b"\x06"), ("output", b"\x02")):
            if (i, value) in seen_values:
                return True
            seen_values.add((i, value))
    return False


def spent_index_missing(raw):
    """An input with a previous transaction that has no output at the spent
    index."""
    got = maps_of(raw)
    if got is None:
        return False
    maps, n_in = got
    v2 = _says_v2(raw)
    spent = None
    if not v2:
        try:
            spent = [i.prevout.n for i in read_unsigned(get(maps[0], 0)).vin]
        except Exception:  # noqa: BLE001
            return False
    for i, m in enumerate(maps[1:n_in + 1]):
        prev = get(m, 0x00)
        if prev is None:
            continue
        n = spent[i] if spent is not None else int.from_bytes(get(m, 0x0f) or b"", "little")
        try:
            if n >= len(_prev_tx(prev).vout):
                return True
        except Exception:  # noqa: BLE001
            continue
    return False


def hwi_rule(raw, text):
    """HWI refuses a version 2 PSBT (once the pairs Faraday leaves to the
    bridge are taken out) that Faraday reads."""
    if "unpack requires a buffer of" in text and key_type_cut(raw):
        return "hwi: a key type cut short"
    if "'i' format requires" in text and tx_version_high(raw):
        return "hwi: a transaction version of 2^31 or more"
    if "PSBT_OUTPUT_SCRIPT is required" in text and empty_output_script(raw):
        return "hwi: an empty output script"
    if "PSBT_OUTPUT_SCRIPT is required" in text and silent_output_without_script(raw):
        return "hwi: a silent payment output with no script"
    if "Musig2" in text and musig_key_wrong(raw):
        return "hwi: a MuSig2 field's key data of a size BIP-373 forbids"
    return None


def btclib_rule(raw, version, text):
    """btclib refuses a PSBT that Faraday reads."""
    if "satoshi amount" in text and amount_over(raw):
        return "btclib: an amount over 21,000,000 BTC"
    if "mismatched witness utxo" in text and utxos_disagree(raw):
        return "btclib: a witness UTXO that is not the previous transaction's output"
    if "preimage" in text and preimage_wrong(raw):
        return "btclib: a preimage that does not hash to its key"
    if ("silent payment" in text or "signed message" in text) and bip375_pair(raw):
        return "btclib: a BIP-375 field whose key data is of a size it forbids"
    if version == 0 and bip375_in_v0(raw):
        return "btclib: a BIP-375 field in a version 0 PSBT"
    if "Duplicated key origin values" in text and origins_repeat(raw):
        return "btclib: two key origins of one map with the same value"
    if "vout out of range for the non-witness utxo" in text and spent_index_missing(raw):
        return "btclib: a previous transaction with no output at the spent index"
    return None


class Psbt(Area):
    name = "psbt"
    # A corpus PSBT with fields btclib refuses and the BIPs do not (see the
    # module's last known difference): btclib is not asked of it.
    exotic = False
    CORE = True
    faraday = (
        '`faraday_psbt::Psbt` (version 0, over `bitcoin`), `faraday_psbt::v2::PsbtV2` '
        "(version 2, Faraday's own)"
    )
    reference = (
        "Bitcoin Core v31.1's `bitcoind` on regtest for version 0 (`decodepsbt`, "
        "`combinepsbt` of one PSBT as its writer, the test framework's "
        "`taproot_tree_helper`); HWI's `psbt.py` (`hwi/`) for version 2 and the version 2 "
        "fields refused in version 0, its version 0 form read by Core; btclib-wallet's "
        "`Psbt` (`btclib/`, on btclib's own curve) for both versions: `parse`, `lock_time`, "
        "`tx`, `to_v0` and `to_v2`, which also decides the refusals of a taproot signature "
        "whose sighash byte is 0, of bytes after the last map and of a stream that ends "
        "inside a map"
    )
    reference_checked = (
        "Core's release tarball is checked against its pinned SHA-256 "
        "(`tools/reference/core/release.pin`) before every run, and the node's version "
        "is asked; HWI's vendored files and btclib's wheels are hashed against "
        "`references.sha256` before any run; a published vector's answer is required of "
        "the side that decides"
    )
    vectors = (
        "every PSBT of BIP-174's, 370's and 371's test vectors "
        "(`tools/vectors/psbt-bips/`), with BIP-370's locktimes, read by btclib too"
    )
    corpus = (
        'random version 0 and 2 PSBTs with every field Faraday reads, taproot trees '
        'among them, in random order, read field for field as Core reads them and '
        'written back byte for byte as read and as Core writes them, then broken one '
        "rule at a time; each version 0 PSBT made version 2 as HWI's `convert_to_v2` "
        "and btclib's `to_v2` make it, and back to its own bytes; BIP-370's locktime, "
        'transaction and version 0 form under every mix of required locktimes, also '
        "btclib's `lock_time`, `tx` and `to_v0`; one version 0 PSBT in eight carries "
        'fields btclib refuses and btclib is not asked of it; every PSBT of the fuzz '
        "corpora (`fuzz/corpus/psbt_diff`, `psbt_parse`, `psbt_v2`, and a grown corpus "
        "when `FARADAY_FUZZ_CORPUS` or `FARADAY_FUZZ_STORE` names one) judged the same ways"
    )
    differences = [
        (
            "Version 0 made version 2: HWI's `convert_to_v2` leaves out the "
            '`PSBT_OUT_SCRIPT` of an output whose script is empty, which BIP-370 '
            'requires; Faraday and btclib write it, and the rest of the PSBT must be the same.'
        ),
        (
            'Bitcoin Core reads a version 0 PSBT with a version 2 field as valid, the field '
            'as unknown; Faraday refuses it, as BIP-370 does, and so do HWI, which '
            'decides it, and btclib.'
        ),
        (
            'A taproot signature of 65 bytes whose sighash byte is 0, which neither Core '
            "nor HWI refuses, and bytes after a version 2 PSBT's last map or a stream "
            'that ends inside one, which HWI reads: Faraday refuses each, as BIP-341 and '
            "BIP-174 do, and so does btclib, which decides them (the stream ending inside "
            'a map checked, as the specification marked it not checked).'
        ),
        (
            'PSBT version 2 (`faraday_psbt::v2`): a BIP-370 key with key data is refused,'
            " as HWI refuses it; BIP-174's and BIP-371's fields inside are checked only when the PSBT is "
            'bridged to version 0, except a taproot signature whose sighash byte is 0, '
            'refused on reading. HWI checks them on reading; the area requires HWI and '
            "btclib to read the PSBT without those pairs, to Faraday's locktime and "
            'transaction. Where HWI reads a value shorter than its length says and makes a '
            'version 0 PSBT of it, which Faraday does not, Core must refuse the version 0 '
            "PSBT of the PSBT's own pairs, and btclib refuses the PSBT."
        ),
        (
            "btclib reads some PSBTs that Core and Faraday refuse (a key with key data on a "
            'field that takes none, for one): btclib is not asked to refuse what Core refuses, '
            'only what Core and HWI read and Faraday refuses.'
        ),
        (
            'btclib refuses what the BIPs allow: an ECDSA signature that is not strict DER '
            "with a low S and an r that is a point's x, a sighash type of the seven it "
            'defines, a BIP-375 field in a version 0 global map, a previous transaction and a '
            'witness UTXO that disagree. About one version 0 PSBT in eight carries such a '
            'field, and btclib is not asked of it or of what is made from it. Pairs btclib '
            'does not write back as read (an empty final witness) and what it writes in their '
            'place are left out of the conversions it is compared on, and leaf hashes are '
            'compared sorted, as it writes them.'
        ),
        (
            "Bitcoin Core refuses a previous transaction that is not the spent output's, or has "
            'no output at the spent index, a key type or proprietary subtype that is a compact '
            "size over its limit or cut short, and a MuSig2 field whose key data is of a size "
            'BIP-373 forbids; Faraday reads the PSBT, carries the pair or blocks the input at '
            'use, and never signs it.'
        ),
        (
            'HWI refuses a key type cut short, a transaction version of 2^31 or more, an '
            'empty output script and a MuSig2 field whose key data is of a size BIP-373 '
            "forbids; Faraday reads them. Faraday's bridge to version 0 refuses a key origin of "
            'more than 16 steps and a preimage that does not hash to its key, which HWI and '
            'Core read.'
        ),
        (
            'btclib refuses an amount over 21,000,000 BTC, a witness UTXO that is not the '
            'previous transaction\'s output, a preimage that does not hash to its key and a '
            'BIP-375 field in a version 0 PSBT; Faraday reads them and blocks the first two '
            'at use.'
        ),
        (
            'btclib refuses a PSBT with two BIP-32 derivation pairs of one map of the same '
            "value (a version 0 PSBT, in the cases met) and a version 2 PSBT whose input's "
            'previous transaction has no output at the spent index (Core refuses the version 0 '
            'form of that, above); Faraday reads both.'
        ),
        (
            'A version 2 transaction with no input is written by `bitcoin` with the segwit '
            'marker and by HWI without; it is compared without it.'
        ),
    ]
    not_compared = (
        'whether a `PSBT_GLOBAL_VERSION` of 0 is written: Core reads a version 0 PSBT '
        'alike with it or without it, and Faraday writes it back as read'
    )

    # ----- one batch of readings

    def read_all(self, reads):
        """Each (raw, version, expected) read by Faraday, then by Core or
        HWI, Core's in one batch; `expected` is a published vector's
        "ok" or "err", or None. Faraday's answers, in order."""
        rust = [self.rust.call(f"psbt.v{v}", psbt=raw.hex()) for raw, v, _ in reads]
        hwi = [outcome(lambda: hwi_v2(raw)) if v == 2 else None for raw, v, _ in reads]
        calls, at = [], {}
        for i, ((raw, version, _), r, h) in enumerate(zip(reads, rust, hwi)):
            if version == 0:
                at[i] = len(calls)
                calls += [("decodepsbt", [b64(raw)]), ("combinepsbt", [[b64(raw)]])]
                if r[0] == "ok":
                    calls.append(("decodepsbt", [b64(bytes.fromhex(r[1]["canonical"]))]))
                    calls.append(("analyzepsbt", [b64(raw)]))
            elif r[0] == "ok" and h[0] == "ok" and r[1]["v0"] and h[1]["v0"]:
                # Both version 0 forms, read by Core.
                at[i] = len(calls)
                v0s = [bytes.fromhex(x[1]["v0"]) for x in (r, h)]
                if no_inputs(raw):
                    v0s[0] = unmarked_v0(v0s[0])
                calls += [("decodepsbt", [b64(v)]) for v in v0s]
        core = core_batch(calls) if calls else []
        for i, ((raw, version, expected), r, h) in enumerate(zip(reads, rust, hwi)):
            if version == 0:
                self.judge_v0(raw, r, core[at[i]:at[i] + 4], expected)
            else:
                self.judge_v2(raw, r, h, core[at[i]:at[i] + 2] if i in at else None,
                              expected)
        return rust

    def judge_v0(self, raw, rust, core, expected):
        inputs = {"psbt": raw.hex()}
        decoded, written = core[0], core[1]
        shown = decoded
        if decoded[0] == "ok":
            tx = decoded[1]["tx"]
            shown = ("ok", {"txid": tx["txid"], "inputs": len(tx["vin"]),
                            "outputs": len(tx["vout"])})
        decided = shown
        self.compared += 1
        if rust[0] == "err" and decoded[0] == "ok" and "invalid xonly public key" in rust[1] \
                and xonly_invalid(raw):
            # Faraday refuses; Core, HWI and btclib read.
            self.rule("faraday: an x-only key that is not a point's x")
        elif rust[0] == "err" and decoded[0] == "ok":
            # Core reads it; HWI must refuse it as BIP-370 does, or
            # btclib as BIP-341 does (a taproot signature with a sighash
            # byte of 0).
            hwi = outcome(lambda: hwi_read(raw))
            bt = outcome(lambda: btclib_read(raw, 0))
            if hwi[0] == "err" and HWI_V2_IN_V0 in hwi[1]:
                decided = hwi
            elif bt[0] == "err" and not self.exotic:
                decided = bt
            else:
                raise Mismatch(self.name, "psbt.v0", inputs, rust,
                               {"core": shown, "hwi": hwi, "btclib": seen(bt)},
                               "Faraday refuses what Core reads, and neither HWI nor "
                               "btclib refuses it")
            self.known += 1
        elif rust[0] == "ok" and decoded[0] == "err" and core_rule(raw, decoded[1]):
            # Core refuses what Faraday reads, by a rule of the list.
            self.rule(core_rule(raw, decoded[1]))
            if bytes.fromhex(rust[1]["bytes"]) != raw:
                raise Mismatch(self.name, "psbt.v0", inputs, rust, shown,
                               "Faraday does not write back the bytes it read")
        elif rust[0] != decoded[0]:
            raise Mismatch(self.name, "psbt.v0", inputs, rust, shown)
        elif rust[0] == "ok":
            got = rust[1]
            if bytes.fromhex(got["bytes"]) != raw:
                raise Mismatch(self.name, "psbt.v0", inputs, rust, shown,
                               "Faraday does not write back the bytes it read")
            if got["txid"] != decoded[1]["tx"]["txid"]:
                raise Mismatch(self.name, "psbt.v0", inputs, rust, shown,
                               "the unsigned transaction's txid differs")
            mine, theirs = core_view(decoded), core_view(core[2])
            if mine != theirs:
                raise Mismatch(self.name, "psbt.v0", inputs, rust,
                               {"core_reads_it": mine, "core_reads_faradays": theirs},
                               "Core reads Faraday's writing of the PSBT otherwise than "
                               "the PSBT itself")
            self.judge_fee(inputs, rust, core[3])
            self.judge_to_v2(raw, got, len(decoded[1]["tx"]["vin"]),
                             len(decoded[1]["tx"]["vout"]))
            # Core's own writing of the PSBT, read and written back by
            # Faraday byte for byte.
            if written[0] != "ok":
                raise Mismatch(self.name, "psbt.v0", inputs, rust, written,
                               "Core reads the PSBT but does not write it")
            core_bytes = base64.b64decode(written[1])
            again = self.rust.call("psbt.v0", psbt=core_bytes.hex())
            self.compared += 1
            if again[0] != "ok" or bytes.fromhex(again[1]["bytes"]) != core_bytes:
                raise Mismatch(self.name, "psbt.v0", {"psbt": core_bytes.hex()}, again,
                               ("ok", {"bytes": core_bytes.hex()}),
                               "Faraday does not write back Core's writing of the PSBT")
        self.judge_btclib_v0(raw, rust, decided)
        if expected is not None and decided[0] != expected:
            raise Mismatch(self.name, "psbt.v0", inputs, rust, decided,
                           f"differs from the published vector ({expected})")

    def rule(self, name):
        """A known difference of the list applied."""
        self.known += 1

    def judge_fee(self, inputs, rust, analyzed):
        """Core's `analyzepsbt` fee, in satoshis, against Faraday's
        `inspect` fee: both give one and it is the same, or neither does. Known
        difference: outputs worth more than the inputs: Core's fee is the
        negative difference and Faraday shows none."""
        theirs = None
        if analyzed[0] == "ok" and analyzed[1].get("fee") is not None:
            theirs = int((Decimal(str(analyzed[1]["fee"])) * 10**8).to_integral_value())
        ours = rust[1]["fee"]
        self.compared += 1
        if (analyzed[0] == "ok" and "unspendable" in str(analyzed[1].get("error", ""))
                and theirs is None):
            # analyzepsbt refuses a PSBT that spends an output whose script
            # is provably unspendable; inspect has no such rule.
            self.known += 1
            return
        if theirs is not None and theirs < 0 and ours is None:
            # Outputs worth more than the inputs (or no inputs at all):
            # analyzepsbt gives the negative difference as its fee; Faraday
            # gives none.
            self.known += 1
            return
        if ours != theirs:
            raise Mismatch(self.name, "psbt.v0", inputs, rust, {"analyzepsbt": [analyzed[0], str(analyzed[1])]},
                           f"Faraday's fee {ours} is not analyzepsbt's {theirs}")

    def judge_btclib_v0(self, raw, rust, decided):
        """btclib reads the version 0 PSBT when the others do and refuses
        it when they do, except for what it asks more of (`exotic`)."""
        if self.exotic:
            return
        bt = outcome(lambda: btclib_read(raw, 0))
        self.compared += 1
        if bt[0] == "ok" and decided[0] == "err" and rust[0] == "err":
            # Core and Faraday refuse it and btclib reads it (a key with
            # key data on a field that takes none, for one): not a
            # refusal btclib is asked to make.
            self.known += 1
            return
        if bt[0] == "err" and rust[0] == "ok":
            rule = btclib_rule(raw, 0, bt[1])
            if rule:
                self.rule(rule)
                return
            if decided[0] == "err":
                return  # Core refuses it too, by a rule of the list
        if rust[0] == "ok" and decided[0] == "err":
            # Core refuses it by a rule of the list (`core_rule`): btclib's
            # reading of it is not compared.
            return
        if bt[0] != decided[0]:
            raise Mismatch(self.name, "psbt.v0", {"psbt": raw.hex()}, rust,
                           {"btclib": seen(bt), "others": decided},
                           "btclib's reading of the PSBT differs from Core's and Faraday's")
        if bt[0] == "ok" and rust[0] == "ok":
            tx = bt[1].tx
            if tx.id.hex() != rust[1]["txid"]:
                raise Mismatch(self.name, "psbt.v0", {"psbt": raw.hex()}, rust,
                               {"btclib_txid": tx.id.hex()},
                               "the unsigned transaction's txid differs from btclib's")

    def judge_to_v2(self, raw, got, n_in, n_out):
        """Faraday's version 2 PSBT of a version 0 one (`PsbtV2::from_v0`)
        against HWI's `convert_to_v2`, map by map as sets of pairs, and
        Faraday's version 0 of that (`to_v0`) against the bytes read.
        Where HWI does not write a pair of the PSBT back as read even
        without converting it (its own reading of a field), that pair
        and HWI's writing of it are left out of both sides; where HWI
        does not read the PSBT, nothing is compared. HWI leaves out the
        `PSBT_OUT_SCRIPT` of an output whose script is empty, which
        BIP-370 requires; Faraday writes it, a known difference, and
        turns it back to version 0 as the empty script. A PSBT with a
        pair of a type BIP-375 defines for version 2 alone (`BIP375_V2`)
        is not turned back: version 2 reads it as that field and `to_v0`
        leaves it out."""
        inputs = {"psbt": raw.hex()}
        self.judge_btclib_to_v2(raw, got, n_in, n_out)
        try:
            p = hwi_read(raw)
            h0 = split(base64.b64decode(p.serialize()), n_in, n_out)[0]
            p.convert_to_v2()
            h2 = split(base64.b64decode(p.serialize()))[0]
        except Exception:  # noqa: BLE001 - HWI not reading it is no answer
            return
        ours = split(bytes.fromhex(got["v2"]))[0]
        r0 = split(raw, n_in, n_out)[0]
        self.compared += 1
        empty_script = (b"\x04", b"")
        for i, (o, h, a, b) in enumerate(zip(ours, h2, r0, h0)):
            not_back, hwis = set(a) - set(b), set(b) - set(a)
            left = (set(o) - not_back) ^ (set(h) - hwis)
            if left == {empty_script} and i > n_in and empty_script in o:
                self.known += 1
                continue
            if left:
                raise Mismatch(self.name, "psbt.v0", inputs, ("ok", got["v2"]),
                               ("ok", base64.b64decode(p.serialize()).hex()),
                               f"the version 2 PSBT of it differs from HWI's in map {i}")
        # Back to version 0, where the unsigned transaction leads and no
        # version is written; not for a transaction of no input, whose
        # serialisation reads as a SegWit marker and is not signed.
        glob = r0[0]
        bip375 = any(k[:1] and k[0] in BIP375_V2[min(i, 1) if i <= n_in else 2]
                     for i, m in enumerate(r0) for k, _ in m)
        if (n_in and not bip375 and glob and glob[0][0] == b"\x00"
                and get(glob, 0xfb) is None):
            if got["back"] != raw.hex():
                raise Mismatch(self.name, "psbt.v0", inputs, ("ok", got["back"]),
                               ("ok", raw.hex()),
                               "its version 2 PSBT turned back to version 0 is not "
                               "the bytes read")

    def judge_btclib_to_v2(self, raw, got, n_in, n_out):
        """Faraday's version 2 PSBT of a version 0 one against btclib's
        `to_v2`, map by map as sets of pairs, with no pair left out: btclib
        writes the empty `PSBT_OUT_SCRIPT` as Faraday does. A PSBT with a
        pair of a type BIP-375 defines for version 2 alone is not turned
        (`BIP375_V2`), nor one of `exotic`'s."""
        if self.exotic:
            return
        r0 = split(raw, n_in, n_out)[0]
        if any(k[:1] and k[0] in BIP375_V2[min(i, 1) if i <= n_in else 2]
               for i, m in enumerate(r0) for k, _ in m):
            return
        inputs = {"psbt": raw.hex()}
        read = outcome(lambda: btclib_read(raw, 0))
        if read[0] != "ok":
            return  # judge_btclib_v0 says so
        p = read[1]
        made = outcome(lambda: btclib_maps(p.to_v2(), n_in, n_out, 2))
        self.compared += 1
        if made[0] != "ok":
            raise Mismatch(self.name, "psbt.v0", inputs, ("ok", got["v2"]), made,
                           "btclib does not make a version 2 PSBT of it")
        ours = [sorted_leaves(m) for m in split(bytes.fromhex(got["v2"]))[0]]
        differ = unrewritten([sorted_leaves(m) for m in r0], btclib_maps(p, n_in, n_out, 0),
                             ours, made[1])
        for i, left in enumerate(differ):
            if left:
                raise Mismatch(self.name, "psbt.v0", inputs, ("ok", got["v2"]),
                               ("ok", sorted(map(repr, left))[:4]),
                               f"the version 2 PSBT of it differs from btclib's in map {i}")

    def judge_v2(self, raw, rust, hwi, core, expected):
        """`core`: Core's readings of Faraday's and HWI's version 0
        forms, where both made one."""
        inputs = {"psbt": raw.hex()}
        decided = hwi
        leaves = False
        read = raw
        self.compared += 1
        if rust[0] == "err" and hwi[0] == "ok":
            # HWI reads it; the BIPs refuse a stream ending inside a map,
            # bytes after the last map and a taproot signature whose
            # sighash byte is 0, as btclib does.
            bt = outcome(lambda: btclib_read(raw, 2))
            if bt[0] != "err" or self.exotic:
                raise Mismatch(self.name, "psbt.v2", inputs, rust, {"hwi": hwi, "btclib": seen(bt)},
                               "Faraday refuses what HWI reads, and btclib does not refuse it")
            self.known += 1
            decided = ("err", bt[1])
        elif rust[0] == "ok" and hwi[0] == "err":
            left = faraday_leaves(raw)
            hwi = outcome(lambda: hwi_v2(left)) if left is not None else hwi
            if hwi[0] == "err":
                rule = hwi_rule(raw, hwi[1])
                if not rule:
                    raise Mismatch(self.name, "psbt.v2", inputs, rust, hwi,
                                   "Faraday reads what HWI refuses, even without the pairs "
                                   "Faraday leaves to the bridge")
                self.rule(rule)
            else:
                self.known += 1
            decided = hwi
            leaves = True
            read = left
        elif rust[0] != hwi[0]:
            raise Mismatch(self.name, "psbt.v2", inputs, rust, hwi)
        if rust[0] == "ok":
            got = rust[1]
            if bytes.fromhex(got["bytes"]) != raw:
                raise Mismatch(self.name, "psbt.v2", inputs, rust, hwi,
                               "Faraday does not write back the bytes it read")
            tx = unmarked_tx(got["tx"]) if no_inputs(raw) and got["tx"] else got["tx"]
            if hwi[0] == "ok" and (got["locktime"], tx) != (hwi[1]["locktime"], hwi[1]["tx"]):
                raise Mismatch(self.name, "psbt.v2", inputs, rust, hwi,
                               "the locktime or the transaction differs")
            # The version 0 form: HWI's `convert_to_v0`, written by HWI,
            # and Faraday's `to_v0`, both read by Core. Not where HWI read
            # the PSBT without the pairs Faraday leaves to the bridge.
            if not leaves and got["v0"] is None and hwi[1]["v0"] is not None:
                # HWI takes a value shorter than its length says without
                # noticing, and writes it out repaired; Core must refuse
                # the version 0 PSBT of the PSBT's own pairs.
                core_v0 = core_batch([("decodepsbt", [b64(v0_of_pairs(raw, hwi[1]["tx"]))])])[0]
                if core_v0[0] != "err":
                    # Faraday's bridge refuses what no wallet has (a key
                    # origin of over 16 steps) and a preimage that does not
                    # hash to its key; HWI and Core read them.
                    rule = ("faraday: a transaction with no input"
                            if no_inputs(raw) and V0_NONE in str(got["v0_error"])
                            else "faraday: a key origin of more than 16 steps" if long_origin(raw)
                            else "faraday: a preimage that does not hash to its key"
                            if preimage_wrong(raw) else None)
                    if not rule:
                        raise Mismatch(self.name, "psbt.v2", inputs, rust, hwi,
                                       "HWI makes a version 0 PSBT of it, Core reads one, "
                                       "Faraday makes none")
                    self.rule(rule)
                else:
                    self.known += 1
            elif not leaves and (got["v0"] is None) != (hwi[1]["v0"] is None):
                raise Mismatch(self.name, "psbt.v2", inputs, rust, hwi,
                               "one makes a version 0 PSBT of it and the other does not")
            if core is not None:
                mine, theirs = core_view(core[0]), core_view(core[1])
                if (mine[0] == theirs[0] == "err" and core_rule(raw, mine[1])
                        and core_rule(raw, theirs[1])):
                    # Core refuses both version 0 forms by a rule of the list.
                    self.rule(core_rule(raw, mine[1]))
                elif mine[0] != "ok" or mine != theirs:
                    raise Mismatch(self.name, "psbt.v2", inputs, rust,
                                   {"core_reads_faradays": mine, "core_reads_hwis": theirs},
                                   "Core reads Faraday's version 0 form otherwise than HWI's")
        self.judge_btclib_v2(raw, read, rust, leaves, decided)
        if expected is not None and decided[0] != expected:
            raise Mismatch(self.name, "psbt.v2", inputs, rust, decided,
                           f"differs from the published vector ({expected})")

    def judge_btclib_v2(self, raw, read, rust, leaves, decided):
        """btclib reads the version 2 PSBT (`read`: the PSBT, or without
        the pairs Faraday leaves to the bridge where HWI refused it) when
        the others do and refuses it when they do; its locktime and
        transaction are Faraday's, and its version 0 PSBT is Faraday's
        map by map as sets of pairs, where Faraday makes one from the
        PSBT as it is."""
        if self.exotic:
            return
        inputs = {"psbt": raw.hex()}
        bt = outcome(lambda: btclib_read(read, 2))
        self.compared += 1
        if bt[0] == "ok" and decided[0] == "err" and rust[0] == "err":
            self.known += 1  # as in version 0: btclib reads what both refuse
            return
        if bt[0] == "err" and rust[0] == "ok" and rust[1]["v0"] is None and not leaves:
            # A BIP-174 field Faraday's reader leaves to the bridge is
            # malformed (a value shorter than its length says, which HWI
            # reads): btclib refuses the PSBT where the bridge makes no
            # version 0 PSBT, and Core refuses the one of its own pairs.
            self.known += 1
            return
        if bt[0] == "err" and rust[0] == "ok":
            rule = btclib_rule(raw, 2, bt[1])
            if rule:
                self.rule(rule)
                return
            if decided[0] == "err":
                return  # HWI refuses it too, by a rule of the list
        if decided[0] == "err" and rust[0] == "ok" and bt[0] == "ok":
            pass  # HWI refuses it by a rule of the list; btclib reads it
        elif bt[0] != decided[0]:
            raise Mismatch(self.name, "psbt.v2", inputs, rust, {"btclib": seen(bt), "others": decided},
                           "btclib's reading of the PSBT differs from HWI's and Faraday's")
        if bt[0] != "ok" or rust[0] != "ok":
            return
        p, got = bt[1], rust[1]
        lock = btclib_lock(p)
        tx = None if lock is None else p.tx.serialize(include_witness=False,
                                                     check_validity=False).hex()
        ours_tx = unmarked_tx(got["tx"]) if no_inputs(read) and got["tx"] else got["tx"]
        if (lock, tx) != (got["locktime"], ours_tx):
            raise Mismatch(self.name, "psbt.v2", inputs, rust, {"locktime": lock, "tx": tx},
                           "the locktime or the transaction differs from btclib's")
        if leaves or got["v0"] is None or lock is None:
            return
        n_in, n_out = len(p.inputs), len(p.outputs)
        made = outcome(lambda: btclib_maps(p.to_v0(), n_in, n_out, 0))
        if made[0] != "ok":
            raise Mismatch(self.name, "psbt.v2", inputs, rust, made,
                           "btclib does not make a version 0 PSBT of it")
        v0 = bytes.fromhex(got["v0"])
        if no_inputs(read):
            v0 = unmarked_v0(v0)
        ours = [sorted_leaves(m) for m in split(v0, n_in, n_out)[0]]
        differ = unrewritten([sorted_leaves(m) for m in split(read)[0]],
                             btclib_maps(p, n_in, n_out, 2), ours, made[1])
        for i, left in enumerate(differ):
            if left:
                raise Mismatch(self.name, "psbt.v2", inputs, rust, {"differing": sorted(map(repr, left))[:4]},
                               f"the version 0 PSBT of it differs from btclib's in map {i}")

    def published(self):
        reads, locks = [], []
        for case in json.loads((ROOT / "tools/vectors/psbt-bips/vectors.json").read_text()):
            raw = bytes.fromhex(case["hex"])
            expect = case["expect"]
            if expect == "invalid":
                reads += [(raw, 0, "err"), (raw, 2, "err")]
            elif expect in ("valid", "signer-fails"):
                reads.append((raw, 2 if _says_v2(raw) else 0, "ok"))
            else:
                locks.append((len(reads), case["case"],
                              None if expect == "locktime none" else int(expect.split()[1])))
                reads.append((raw, 2, "ok"))
        got = self.read_all(reads)
        for at, name, want in locks:
            if got[at][0] != "ok" or got[at][1].get("locktime") != want:
                raise AssertionError(f"{name}: locktime {got[at]}, the BIP says {want}")

    # ----- PSBTs built to show a known difference

    def built(self):
        """Three PSBTs, each the case of a known difference no committed seed
        shows: a version 0 PSBT with two derivation pairs of one value
        (btclib refuses it), a version 2 PSBT with a previous transaction
        with no output at the spent index (btclib refuses it, HWI and Faraday
        read it), and a version 2 PSBT with a second global pair of type 0xfb
        that carries a byte of key data after the exact `0xfb` version pair
        (every judge and Faraday refuse it)."""
        def key(n):
            return FAST_G.mul(n).to_bytes_compressed()

        def p2wpkh(n):
            return b"\x00\x14" + bytes([n]) * 20

        origin = bytes.fromhex("01020304") + (0x80000000).to_bytes(4, "little")
        tx = CTransaction()
        tx.version = 2
        tx.vin.append(CTxIn(COutPoint(0x11 << 200, 0), b"", 0xFFFFFFFD))
        tx.vout.append(CTxOut(50_000, p2wpkh(1)))
        v0 = MAGIC + write_map([(b"\x00", tx.serialize_without_witness())]) + write_map(
            [(b"\x06" + key(3), origin), (b"\x06" + key(4), origin)]) + write_map([])
        prev = CTransaction()
        prev.version = 2
        prev.vin.append(CTxIn(COutPoint(0x22 << 200, 0), b"", 0xFFFFFFFF))
        prev.vout.append(CTxOut(100_000, p2wpkh(2)))
        glob = [(b"\x02", (2).to_bytes(4, "little")), (b"\x04", ser_compact(1)),
                (b"\x05", ser_compact(1)), (b"\xfb", (2).to_bytes(4, "little"))]
        inp = [(b"\x00", prev.serialize()), (b"\x0e", ser_uint256(prev.txid_int)),
               (b"\x0f", (5).to_bytes(4, "little"))]
        out = [(b"\x03", (90_000).to_bytes(8, "little")), (b"\x04", p2wpkh(3))]
        v2 = MAGIC + write_map(glob) + write_map(inp) + write_map(out)
        inp_ok = [(b"\x0e", ser_uint256(prev.txid_int)), (b"\x0f", (0).to_bytes(4, "little"))]
        keyed = MAGIC + write_map(glob + [(b"\xfb\x00", (2).to_bytes(4, "little"))]) \
            + write_map(inp_ok) + write_map(out)
        self.read_all([(v0, 0, None), (v2, 2, None), (keyed, 2, None)])

    # ----- random PSBTs

    def key(self):
        return FAST_G.mul(self.rng.randrange(1, N)).to_bytes_compressed()

    def der(self):
        rng = self.rng

        def num(x):
            b = x.to_bytes((x.bit_length() + 7) // 8, "big")
            return b"\0" + b if b[0] & 0x80 else b
        r, s = rng.randrange(1, N), rng.randrange(1, N)
        if not self.exotic:
            # What btclib asks of a signature: an r that is a point's x
            # and a low S.
            r = int.from_bytes(self.key()[1:], "big") % N
            s = min(s, N - s)
        r, s = num(r), num(s)
        body = b"\x02" + bytes([len(r)]) + r + b"\x02" + bytes([len(s)]) + s
        return b"\x30" + bytes([len(body)]) + body + bytes([rng.choice([1, 2, 3, 0x81, 0x82, 0x83])])

    def origin(self, n=None):
        n = self.rng.randint(0, 5) if n is None else n
        return self.randbytes(4) + b"".join(self.rng.randrange(2**32).to_bytes(4, "little")
                                            for _ in range(n))

    def xpub(self, depth):
        """BIP-32's 78 bytes of a mainnet xpub at `depth`."""
        return (bytes.fromhex("0488b21e") + bytes([depth]) + self.randbytes(4)
                + self.rng.randrange(2**32).to_bytes(4, "big") + self.randbytes(32) + self.key())

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
            ty = {"global": rng.choice([0x07, 0x08, 0x40, 0xfa] if self.exotic else [0x40, 0xfa]),
                  "in": rng.choice([0x40, 0x7f, 0xf0]),
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
            prev = None
            if rng.random() < 0.4:
                vout = rng.randint(0, 2)
                prev = self.prev_tx(vout)
                tx.vin.append(CTxIn(COutPoint(prev.txid_int, vout), b"", rng.choice([0xFFFFFFFF, 0xFFFFFFFD])))
                m.append((b"\x00", prev.serialize_without_witness()))
            else:
                tx.vin.append(CTxIn(COutPoint(rng.randrange(2**256), rng.randrange(4)), b"",
                                    rng.choice([0xFFFFFFFF, 0xFFFFFFFD])))
            if rng.random() < 0.7:
                out = CTxOut(rng.randrange(10**10), self.script())
                if prev is not None and not self.exotic:
                    out = prev.vout[vout]
                m.append((b"\x01", out.serialize()))
            for _ in range(rng.choice([0, 0, 1, 2])):
                m.append((b"\x02" + self.key(), self.der()))
            if rng.random() < 0.3:
                m.append((b"\x03", rng.choice([1, 2, 3, 0x81, 0x83] + [0x41] * self.exotic).to_bytes(4, "little")))
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
            depth = rng.randint(1, 5)
            glob.append((b"\x01" + self.xpub(depth), self.origin(depth)))
        if rng.random() < 0.2:
            glob.append((b"\xfb", (0).to_bytes(4, "little")))
        if rng.random() < 0.2:
            glob.append(self.unknown("global"))
        rng.shuffle(glob)
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
            rng.shuffle(m)
            outputs.append(m)
        glob = [(b"\x02", rng.choice([2, 3]).to_bytes(4, "little")),
                (b"\x04", ser_compact(len(inputs))), (b"\x05", ser_compact(len(outputs))),
                (b"\xfb", (2).to_bytes(4, "little"))]
        if rng.random() < 0.5:
            glob.append((b"\x03", rng.randrange(2**32).to_bytes(4, "little")))
        if rng.random() < 0.3:
            glob.append((b"\x06", bytes([rng.randrange(8)])))
        rng.shuffle(glob)
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
        self.exotic = version == 0 and rng.random() < 0.12
        glob, inputs, outputs = self.random_v0() if version == 0 else self.random_v2()
        raw = MAGIC + write_map(glob) + b"".join(write_map(m) for m in inputs + outputs)
        reads = [(raw, version, "ok")]
        if not self.exotic:
            reads.append((self.broken(glob, inputs, outputs, version), version, None))
        if rng.random() < 0.1:
            reads.append((raw, 2 - version, "err"))
        self.read_all(reads)

    # `fuzz`: one case a corpus PSBT (`fuzz_psbts`); a scale above 1 reads no
    # more of them, below 1 fewer.
    LOOPS = (("corpus", 1500), ("fuzz", len(FUZZ[0])))

    def prelude(self):
        self.published()
        self.built()
        print("psbt: fuzz corpora read: " + ", ".join(
            f"{v} {k.replace('_', ' ')}" for k, v in FUZZ[1].items()), file=sys.stderr)

    def case_fuzz(self):
        """A corpus PSBT, read as the version it says and as the other,
        judged as every other case is (btclib asked too: a PSBT it reads
        otherwise than the others is reported)."""
        i = self.case_at[1]
        if i >= len(FUZZ[0]):
            return
        name, raw = FUZZ[0][i]
        version = 2 if _says_v2(raw) else 0

        def late(*_):
            raise Mismatch(self.name, "psbt.fuzz", {"corpus file": name, "psbt": raw.hex()},
                           ("err", "no answer"), ("err", "no answer"),
                           f"no judge or helper finished this case in {CASE_SECONDS} s")

        # A case that cannot finish fails with its input named, never hangs.
        old = signal.signal(signal.SIGALRM, late)
        signal.alarm(CASE_SECONDS)
        try:
            self.read_all([(raw, version, None), (raw, 2 - version, None)])
        finally:
            signal.alarm(0)
            signal.signal(signal.SIGALRM, old)

    def case_corpus(self):
        self.corpus_case()


def _says_v2(raw):
    try:
        glob, _ = read_map(raw, 5)
    except Refused:
        return False
    v = get(glob, 0xfb)
    return v is not None and int.from_bytes(v, "little") == 2
