"""Taproot outputs with script trees against Bitcoin Core's
functional-test framework (`core/test_framework/script.py`'s
`taproot_construct`, with `key.py`'s tweak and `segwit_addr.py`'s
BIP-350 addresses).

- BIP-341's wallet vectors, the `scriptPubKey` section
  (`tools/vectors/sighash/bip341-wallet-test-vectors.json`): each
  internal key and script tree built by Faraday (`bitcoin`'s
  `TaprootBuilder`, `ScriptBuf::new_p2tr`, `Address::p2tr_tweaked`) to
  its leaf hashes, Merkle root, tweak, output key, script, address and
  control blocks, each checked against the reference first.
- Random trees of 1 to 12 leaves in random shapes, with random leaf
  scripts and leaf versions, over random internal keys, built by both;
  internal keys that are no point, odd leaf versions and 0x50 refused.
- Every control block checked against its output key and leaf script
  (`ControlBlock::decode` and `verify_taproot_commitment`, which
  `miniscript`'s finalizer and interpreter rest on), against BIP-341's
  script-path rule written out here; then the same block changed: its
  parity bit, its leaf version, a byte of the internal key or of a
  branch hash, a branch hash dropped or added, the wrong leaf script,
  another output key, and lengths BIP-341 does not allow.

BIP-341 leaves two rules to the text, written here as it states them: a
leaf version is even, and is not 0x50 ("the annex... cannot be confused
with a leaf version"); `taproot_construct` asserts the first and does not
check the second. A control block is 33 + 32m bytes with m at most 128.
"""

import functools
import json

from common import ROOT, Area, as_verify, use_reference

use_reference("bip352", "secp256k1lab", "src")
from secp256k1lab.secp256k1 import GE  # noqa: E402

use_reference("core")
import test_framework.key as K  # noqa: E402
import test_framework.script as S  # noqa: E402
from test_framework import segwit_addr  # noqa: E402
from test_framework.key import TaggedHash  # noqa: E402
from test_framework.messages import ser_string  # noqa: E402
from test_framework.script import CScript, taproot_construct  # noqa: E402

# A tree's output key is worked out again for each of its control blocks
# and each one changed: the same key tweaked by the same tweak, a point
# multiplication each time. `tweak_add_pubkey` is a pure function of two
# byte strings, and its answer a pair of bytes and a bool, or None:
# kept answers are the answers it would compute again. `script.py`
# calls it by the name it imported, so that name is pointed here too;
# the reference's files are unchanged. About 1.6 times faster.
_tweak_add_pubkey = K.tweak_add_pubkey
_kept_tweak = functools.lru_cache(maxsize=1 << 14)(_tweak_add_pubkey)


def tweak_add_pubkey(key, tweak):
    if type(key) is bytes and type(tweak) is bytes:
        return _kept_tweak(key, tweak)
    return _tweak_add_pubkey(key, tweak)


K.tweak_add_pubkey = S.tweak_add_pubkey = tweak_add_pubkey

TAPSCRIPT = 0xC0


class Refused(Exception):
    pass


def to_items(tree, leaves):
    """BIP-341's JSON tree as `taproot_construct`'s items, each leaf
    named by its place in depth-first order; `leaves` collects
    `[depth, script, version]` in that order."""
    def walk(node, depth):
        if isinstance(node, dict):
            name = len(leaves)
            leaves.append([depth, node["script"], node["leafVersion"]])
            return (name, bytes.fromhex(node["script"]), node["leafVersion"])
        assert len(node) == 2
        return [walk(node[0], depth + 1), walk(node[1], depth + 1)]
    if tree is None:
        return []
    if isinstance(tree, dict):
        return [walk(tree, 0)]
    return walk(tree, 0)


def reference_tree(internal, items, n):
    """What `taproot_construct` builds, in the Rust command's shape."""
    if any(v & 1 for _, _, v in flat(items)):
        raise Refused("a leaf version is odd")
    if any(v == 0x50 for _, _, v in flat(items)):
        raise Refused("0x50 is the annex's tag, not a leaf version")
    GE.from_bytes_xonly(internal)  # an internal key that is no point is refused
    info = taproot_construct(internal, items)
    controls, hashes = [], []
    for name in range(n):
        leaf = info.leaves[name]
        controls.append((bytes([leaf.version | info.negflag]) + internal
                         + leaf.merklebranch).hex())
        hashes.append(leaf.leaf_hash.hex())
    return {
        "output_key": info.output_pubkey.hex(),
        "parity": info.negflag,
        "tweak": info.tweak.hex(),
        "merkle_root": info.merkle_root.hex() if n else None,
        "leaf_hashes": hashes,
        "control_blocks": controls,
        "script_pubkey": bytes(info.scriptPubKey).hex(),
        "address": segwit_addr.encode_segwit_address("bc", 1, info.output_pubkey),
    }


def flat(items):
    for item in items:
        if isinstance(item, tuple):
            yield item
        else:
            yield from flat(item)


def verify_control(output_key, control, script):
    """BIP-341's script-path check: the control block's length, the leaf
    hash, the branch hashes in order (each pair sorted), the tweak, and
    the output key and its parity."""
    if len(control) < 33 or (len(control) - 33) % 32 or (len(control) - 33) // 32 > 128:
        raise Refused("a control block of the wrong length")
    version = control[0] & 0xFE
    if version == 0x50:
        raise Refused("0x50 is not a leaf version")
    internal = control[1:33]
    try:
        GE.from_bytes_xonly(internal)
        GE.from_bytes_xonly(output_key)
    except (ValueError, AssertionError) as e:
        raise Refused("not a point") from e
    k = TaggedHash("TapLeaf", bytes([version]) + ser_string(script))
    for j in range((len(control) - 33) // 32):
        e = control[33 + 32 * j:65 + 32 * j]
        k = TaggedHash("TapBranch", k + e if k < e else e + k)
    t = TaggedHash("TapTweak", internal + k)
    q = tweak_add_pubkey(internal, t)
    if q is None:
        return {"valid": False}
    return {"valid": q[0] == output_key and int(q[1]) == control[0] & 1}


class Taproot(Area):
    name = "taproot"
    faraday = (
        "`bitcoin`'s `TaprootBuilder` and `TaprootSpendInfo` (as "
        "`faraday_bip::tapmulti` and `miniscript`'s `tr()` build trees), "
        '`ScriptBuf::new_p2tr` as `faraday_psbt::sign` checks a key path, '
        '`Address::p2tr_tweaked`, `ControlBlock` decoding and '
        '`verify_taproot_commitment`'
    )
    reference = (
        "Core's `taproot_construct`, `key.py`'s tweak and `segwit_addr.py`; BIP-341's "
        'script-path check written out in `area_taproot.py`'
    )
    reference_checked = (
        'its vendored files are hashed against `references.sha256` before any run, and '
        "a published vector's answer is required of the reference as well as of "
        "Faraday; each of BIP-341's wallet vectors is checked against the reference "
        'first'
    )
    vectors = (
        "BIP-341's `scriptPubKey` wallet vectors, all seven (`tools/vectors/sighash/`)"
    )
    corpus = (
        'trees of 0 to 12 leaves in random shapes with random scripts and leaf versions '
        'over random internal keys: leaf hashes, Merkle root, tweak, output key and '
        'parity, script, address, control blocks; each control block checked, then '
        'changed eleven ways; internal keys that are no point, odd leaf versions and '
        '0x50 refused'
    )
    differences = [
        (
            'none'
        ),
    ]
    not_compared = (
        'none'
    )

    def tree(self, internal, items, leaves, expected=None):
        inputs = {"internal": internal.hex(), "leaves": leaves}
        raw = self.compare("taproot.tree", inputs,
                           lambda: reference_tree(internal, items, len(leaves)), expected)
        return raw

    def verify(self, output_key, control, script):
        inputs = {"output_key": output_key.hex(), "control": control.hex(), "script": script.hex()}
        self.compare("taproot.verify", inputs, lambda: verify_control(output_key, control, script),
                     normalize=as_verify)

    def published(self):
        v = json.loads((ROOT / "tools/vectors/sighash/bip341-wallet-test-vectors.json").read_text())
        for case in v["scriptPubKey"]:
            given, inter, exp = case["given"], case["intermediary"], case["expected"]
            leaves = []
            items = to_items(given["scriptTree"], leaves)
            internal = bytes.fromhex(given["internalPubkey"])
            want = {
                "output_key": inter["tweakedPubkey"],
                "tweak": inter["tweak"],
                "merkle_root": inter["merkleRoot"],
                "leaf_hashes": inter.get("leafHashes", []),
                "control_blocks": exp.get("scriptPathControlBlocks", []),
                "script_pubkey": exp["scriptPubKey"],
                "address": exp["bip350Address"],
            }
            ref = reference_tree(internal, items, len(leaves))
            for key, value in want.items():
                if ref[key] != value:
                    raise AssertionError(f"the reference differs from BIP-341 at {key}: {ref[key]}")
            self.tree(internal, items, leaves, expected=("ok", ref))
            for (_, script, _), control in zip(leaves, ref["control_blocks"]):
                self.verify(bytes.fromhex(ref["output_key"]), bytes.fromhex(control),
                            bytes.fromhex(script))

    def random_key(self):
        while True:
            x = self.randbytes(32)
            try:
                GE.from_bytes_xonly(x)
                return x
            except (ValueError, AssertionError):
                continue

    def random_script(self):
        r = self.rng.random()
        if r < 0.4:
            return bytes(CScript([self.random_key(), 0xAC]))
        if r < 0.5:
            return b""
        return self.randbytes(self.rng.randrange(1, 80))

    def random_version(self):
        r = self.rng.random()
        if r < 0.8:
            return TAPSCRIPT
        while True:
            v = self.rng.randrange(0, 256) & 0xFE
            if v != 0x50:
                return v

    def random_tree(self, n):
        """A random shape over `n` leaves, as items and DFS leaves."""
        leaves = []

        def build(count, depth):
            if count == 1:
                name = len(leaves)
                script, version = self.random_script(), self.random_version()
                while any(bytes.fromhex(s) == script and v == version for _, s, v in leaves):
                    script = self.random_script()
                leaves.append([depth, script.hex(), version])
                return (name, script, version)
            left = self.rng.randrange(1, count)
            return [build(left, depth + 1), build(count - left, depth + 1)]
        if n == 1:
            return [build(1, 0)], leaves
        return build(n, 0), leaves

    def case_tree(self):
        internal = self.random_key()
        n = self.rng.randrange(0, 13)
        items, leaves = ([], []) if n == 0 else self.random_tree(n)
        raw = self.tree(internal, items, leaves)
        if raw[0] != "ok":
            raise AssertionError("both refused a valid tree")
        out = raw[1]
        output_key = bytes.fromhex(out["output_key"])
        for (_, script, _), control in zip(leaves, out["control_blocks"]):
            script, control = bytes.fromhex(script), bytes.fromhex(control)
            self.verify(output_key, control, script)
            self.broken_controls(output_key, control, script)

    def case_refused(self):
        """Refusals: a key that is no point, an odd leaf version, 0x50."""
        while True:
            bad = self.randbytes(32)
            try:
                GE.from_bytes_xonly(bad)
            except (ValueError, AssertionError):
                break
        items, leaves = self.random_tree(self.rng.randrange(1, 4))
        self.tree(bad, items, leaves)
        items, leaves = self.random_tree(self.rng.randrange(1, 4))
        j = self.rng.randrange(len(leaves))
        version = self.rng.choice([0x50, self.rng.randrange(0, 256) | 1])
        leaves[j][2] = version
        items = self.renamed(items, j, version)
        self.tree(self.random_key(), items, leaves)

    def renamed(self, items, j, version):
        if isinstance(items, tuple):
            return (items[0], items[1], version) if items[0] == j else items
        return [self.renamed(i, j, version) for i in items]

    def broken_controls(self, output_key, control, script):
        rng = self.rng
        cases = [
            bytes([control[0] ^ 1]) + control[1:],
            bytes([(control[0] & 1) | rng.choice([0xC2, 0xFA, 0x50, 0x52])]) + control[1:],
            control[:1] + bytes([control[1] ^ (1 << rng.randrange(8))]) + control[2:],
            control + self.randbytes(32),
            control[:-1],
            control + b"\0",
            control[:33],
        ]
        if len(control) > 33:
            at = rng.randrange(33, len(control))
            cases.append(control[:at] + bytes([control[at] ^ 1]) + control[at + 1:])
            cases.append(control[:-32])
        for c in cases:
            self.verify(output_key, c, script)
        self.verify(output_key, control, script + b"\x51")
        self.verify(self.random_key(), control, script)

    LOOPS = (("tree", 400), ("refused", 40))

    def prelude(self):
        self.published()
