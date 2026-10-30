"""PSBTs against Bitcoin Core v31.1 itself for version 0, and against
HWI's `psbt.py` (`tools/reference/hwi/`) for version 2, which Core
refuses.

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
- Version 0 to version 2: every version 0 PSBT Faraday reads, made
  version 2 by Faraday (`PsbtV2::from_v0`) and by HWI (`convert_to_v2`),
  compared map by map as sets of pairs, leaving out the pairs HWI
  rewrites even without converting (its own writing of a field); and
  Faraday's made version 0 again (`to_v0`), which must be the bytes read
  where the unsigned transaction leads the global map, no
  `PSBT_GLOBAL_VERSION` is written and the transaction has an input.

Every PSBT of BIP-174's, BIP-370's and BIP-371's Test Vectors sections
(`tools/vectors/psbt-bips/`) is read both ways, and the side that decides
must give the BIP's answer; then random version 0 and 2 PSBTs with every
field Faraday reads or carries, in random order, and the same PSBTs
broken one rule at a time.

Still Faraday's own, and counted as known differences:

- Core reads a version 0 PSBT with version 2 fields as valid, the fields
  as unknown; HWI refuses it, as BIP-370 does, and decides it.
- A taproot signature of 65 bytes whose sighash byte is 0, which neither
  Core nor HWI refuses, and bytes after a version 2 PSBT's last map or a
  stream that ends inside one, which HWI reads: the BIPs refuse them, as
  Faraday does, by rules written in this area (`ours_v0`, `ours_v2`)
  over a splitter of the key-value maps written here.
- Faraday's version 2 reader leaves BIP-174's and BIP-371's fields to the
  bridge to version 0 and reads a BIP-370 key with key data as an
  unknown key. Where HWI refuses such a PSBT, HWI must read it once those
  pairs are taken out (`faraday_leaves`), to the same locktime and
  transaction. Where HWI takes a field's value shorter than its length
  says and writes it out repaired, and Faraday makes no version 0 PSBT,
  Core must refuse the version 0 PSBT of the PSBT's own pairs
  (`v0_of_pairs`).
"""

import base64
import io
import json
import sys

import core_node
from common import ROOT, Area, Mismatch, use_reference

use_reference("core")
from test_framework.crypto.secp256k1 import FAST_G, GE  # noqa: E402
from test_framework.messages import (  # noqa: E402
    COutPoint, CTransaction, CTxIn, CTxOut, deser_vector)
from test_framework.script import taproot_tree_helper  # noqa: E402

use_reference("hwi")
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


# ----- the container: written here, to build PSBTs and for the rules
# below that neither Core nor HWI applies ----------------------------------


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
    follow the last."""
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


def zero_sighash_tap_sig(key, value):
    """BIP-341: a 65-byte taproot signature's sighash byte is never 0
    (the default type is written as 64 bytes)."""
    return (key == b"\x13" or key[:1] == b"\x14") and len(value) == 65 and value[64] == 0


def ours_v0(raw, n_in, n_out):
    """Why BIP-341 refuses a version 0 PSBT that Bitcoin Core and HWI
    read, or None: a zero sighash byte on a taproot signature."""
    maps, _ = split(raw, n_in, n_out)
    for m in maps[1:1 + n_in]:
        if any(zero_sighash_tap_sig(k, v) for k, v in m):
            return "a taproot signature with a sighash byte of 0 (BIP-341)"
    return None


def ours_v2(raw):
    """Why the BIPs refuse a version 2 PSBT that HWI reads, or None: the
    stream ends inside a map or goes on after the last (BIP-174), a zero
    sighash byte on a taproot signature (BIP-341)."""
    try:
        maps, n_in = split(raw)
    except Refused as e:
        return str(e)
    for m in maps[1:1 + n_in]:
        if any(zero_sighash_tap_sig(k, v) for k, v in m):
            return "a taproot signature with a sighash byte of 0 (BIP-341)"
    return None


def faraday_leaves(raw):
    """The version 2 PSBT without the pairs Faraday's reader does not
    read: BIP-174's and BIP-371's fields, and BIP-370's keys with key
    data (unknown keys to it). None when it cannot be split."""
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


def outcome(fn):
    try:
        return ("ok", fn())
    except Exception as e:  # noqa: BLE001 - every refusal is a refusal
        return ("err", f"{type(e).__name__}: {e}")


class Psbt(Area):
    name = "psbt"
    CORE = True
    faraday = (
        '`faraday_psbt::Psbt` (version 0, over `bitcoin`), `faraday_psbt::v2::PsbtV2` '
        "(version 2, Faraday's own)"
    )
    reference = (
        "Bitcoin Core v31.1's `bitcoind` on regtest for version 0 (`decodepsbt`, "
        "`combinepsbt` of one PSBT as its writer, the test framework's "
        "`taproot_tree_helper`); HWI's `psbt.py` (`hwi/`) for version 2 and the version 2 "
        "fields refused in version 0, its version 0 form read by Core; the refusals "
        "neither makes written in `area_psbt.py`"
    )
    reference_checked = (
        "Core's release tarball is checked against its pinned SHA-256 "
        "(`tools/reference/core/release.pin`) before every run, and the node's version "
        "is asked; HWI's vendored files are hashed against `references.sha256` before "
        "any run; a published vector's answer is required of the side that decides"
    )
    vectors = (
        "every PSBT of BIP-174's, 370's and 371's test vectors "
        "(`tools/vectors/psbt-bips/`), with BIP-370's locktimes"
    )
    corpus = (
        'random version 0 and 2 PSBTs with every field Faraday reads, taproot trees '
        'among them, in random order, read field for field as Core reads them and '
        'written back byte for byte as read and as Core writes them, then broken one '
        "rule at a time; each version 0 PSBT made version 2 as HWI's `convert_to_v2` "
        "makes it and back to its own bytes; BIP-370's locktime, transaction and "
        'version 0 form under every mix of required locktimes'
    )
    differences = [
        (
            "Version 0 made version 2: HWI's `convert_to_v2` leaves out the "
            '`PSBT_OUT_SCRIPT` of an output whose script is empty, which BIP-370 '
            'requires; Faraday writes it, and the rest of the PSBT must be the same.'
        ),
        (
            'Bitcoin Core reads a version 0 PSBT with a version 2 field as valid, the field '
            'as unknown; Faraday refuses it, as BIP-370 does, and so does HWI, which '
            'decides it.'
        ),
        (
            'A taproot signature of 65 bytes whose sighash byte is 0, which neither Core '
            "nor HWI refuses, and bytes after a version 2 PSBT's last map or a stream "
            'that ends inside one, which HWI reads: Faraday refuses each, as BIP-341 and '
            "BIP-174 do, and the refusal is checked by a rule written in the area."
        ),
        (
            'PSBT version 2 (`faraday_psbt::v2`): a BIP-370 key with key data is an unknown'
            " key; BIP-174's and BIP-371's fields inside are checked only when the PSBT is "
            'bridged to version 0, except a taproot signature whose sighash byte is 0, '
            'refused on reading. HWI checks them on reading; the area requires HWI to '
            "read the PSBT without those pairs, to Faraday's locktime and transaction. "
            'Where HWI reads a value shorter than its length says and makes a version 0 '
            'PSBT of it, which Faraday does not, Core must refuse the version 0 PSBT of '
            "the PSBT's own pairs."
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
            elif r[0] == "ok" and h[0] == "ok" and r[1]["v0"] and h[1]["v0"]:
                # Both version 0 forms, read by Core.
                at[i] = len(calls)
                calls += [("decodepsbt", [b64(bytes.fromhex(x[1]["v0"]))]) for x in (r, h)]
        core = core_batch(calls) if calls else []
        for i, ((raw, version, expected), r, h) in enumerate(zip(reads, rust, hwi)):
            if version == 0:
                self.judge_v0(raw, r, core[at[i]:at[i] + 3], expected)
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
        if rust[0] == "err" and decoded[0] == "ok":
            # Core reads it; HWI must refuse it as BIP-370 does, or the
            # area's own rule as BIP-341 does.
            hwi = outcome(lambda: hwi_read(raw))
            why = outcome(lambda: ours_v0(raw, len(tx["vin"]), len(tx["vout"])))
            if hwi[0] == "err" and HWI_V2_IN_V0 in hwi[1]:
                decided = hwi
            elif why[0] == "ok" and why[1] is not None:
                decided = ("err", why[1])
            else:
                raise Mismatch(self.name, "psbt.v0", inputs, rust,
                               {"core": shown, "hwi": hwi},
                               "Faraday refuses what Core reads, and neither HWI nor a BIP "
                               "rule the area holds refuses it")
            self.known += 1
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
        if expected is not None and decided[0] != expected:
            raise Mismatch(self.name, "psbt.v0", inputs, rust, decided,
                           f"differs from the published vector ({expected})")

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

    def judge_v2(self, raw, rust, hwi, core, expected):
        """`core`: Core's readings of Faraday's and HWI's version 0
        forms, where both made one."""
        inputs = {"psbt": raw.hex()}
        decided = hwi
        leaves = False
        self.compared += 1
        if rust[0] == "err" and hwi[0] == "ok":
            why = ours_v2(raw)
            if why is None:
                raise Mismatch(self.name, "psbt.v2", inputs, rust, hwi,
                               "Faraday refuses what HWI reads, and no BIP rule the area "
                               "holds refuses it")
            self.known += 1
            decided = ("err", why)
        elif rust[0] == "ok" and hwi[0] == "err":
            left = faraday_leaves(raw)
            hwi = outcome(lambda: hwi_v2(left)) if left is not None else hwi
            if hwi[0] == "err":
                raise Mismatch(self.name, "psbt.v2", inputs, rust, hwi,
                               "Faraday reads what HWI refuses, even without the pairs "
                               "Faraday leaves to the bridge")
            self.known += 1
            decided = hwi
            leaves = True
        elif rust[0] != hwi[0]:
            raise Mismatch(self.name, "psbt.v2", inputs, rust, hwi)
        if rust[0] == "ok":
            got = rust[1]
            if bytes.fromhex(got["bytes"]) != raw:
                raise Mismatch(self.name, "psbt.v2", inputs, rust, hwi,
                               "Faraday does not write back the bytes it read")
            if (got["locktime"], got["tx"]) != (hwi[1]["locktime"], hwi[1]["tx"]):
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
                    raise Mismatch(self.name, "psbt.v2", inputs, rust, hwi,
                                   "HWI makes a version 0 PSBT of it, Core reads one, "
                                   "Faraday makes none")
                self.known += 1
            elif not leaves and (got["v0"] is None) != (hwi[1]["v0"] is None):
                raise Mismatch(self.name, "psbt.v2", inputs, rust, hwi,
                               "one makes a version 0 PSBT of it and the other does not")
            if core is not None:
                mine, theirs = core_view(core[0]), core_view(core[1])
                if mine[0] != "ok" or mine != theirs:
                    raise Mismatch(self.name, "psbt.v2", inputs, rust,
                                   {"core_reads_faradays": mine, "core_reads_hwis": theirs},
                                   "Core reads Faraday's version 0 form otherwise than HWI's")
        if expected is not None and decided[0] != expected:
            raise Mismatch(self.name, "psbt.v2", inputs, rust, decided,
                           f"differs from the published vector ({expected})")

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

    # ----- random PSBTs

    def key(self):
        return FAST_G.mul(self.rng.randrange(1, N)).to_bytes_compressed()

    def der(self):
        rng = self.rng

        def num(x):
            b = x.to_bytes((x.bit_length() + 7) // 8, "big")
            return b"\0" + b if b[0] & 0x80 else b
        r, s = (num(rng.randrange(1, N)) for _ in range(2))
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
        glob, inputs, outputs = self.random_v0() if version == 0 else self.random_v2()
        raw = MAGIC + write_map(glob) + b"".join(write_map(m) for m in inputs + outputs)
        reads = [(raw, version, "ok"), (self.broken(glob, inputs, outputs, version), version, None)]
        if rng.random() < 0.1:
            reads.append((raw, 2 - version, "err"))
        self.read_all(reads)

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
