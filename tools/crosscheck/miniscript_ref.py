"""python-bip380 (`tools/reference/miniscript/`), Antoine Poinsot's
miniscript and descriptors, set up for the areas that use it: `sign`
(the satisfactions a finalized input must carry) and `miniscript` (the
scripts a descriptor pays to).

python-bip380 imports `coincurve` and `bip32`, two packages over
libsecp256k1; the shims beside it stand in for them over
`secp256k1lab`, and the areas give it raw keys only, derived by
btclib (`bip32_ref.py`). It hashes RIPEMD-160 with `hashlib.new("ripemd160")`, which
an OpenSSL without its legacy provider refuses; that one name is sent
to the BIPs repository's `ripemd160.py` (`tools/reference/bip352/`),
so the answer does not depend on how the machine's OpenSSL was built.

A miniscript is read from its text only. python-bip380's script reader
reads no `pkh` (it names a variable it does not have) and no 32-byte
key under Tapscript, so the areas keep each script beside the text it
was compiled from (`Scripts`), as a wallet keeps its descriptor.
"""

import hashlib

from common import use_reference

use_reference("bip352", "secp256k1lab", "src")  # for the shims

use_reference("bip352")
from ripemd160 import ripemd160  # noqa: E402

_new = hashlib.new


class _Ripemd160:
    def __init__(self, data=b""):
        self.data = bytes(data)

    def update(self, data):
        self.data += bytes(data)

    def digest(self):
        return ripemd160(self.data)


def _hashlib_new(name, data=b"", **kw):
    if name.lower() == "ripemd160":
        return _Ripemd160(data)
    return _new(name, data, **kw)


hashlib.new = _hashlib_new

use_reference("miniscript", "shims")
use_reference("miniscript")
from bip380.descriptors.utils import TreeNode, tapleaf_hash  # noqa: E402,F401
from bip380.miniscript import Node, SatisfactionMaterial  # noqa: E402,F401
from bip380.utils.script import CScript  # noqa: E402,F401


def compile_ms(text, tap):
    """The script of a miniscript with raw keys, in P2WSH or Tapscript."""
    return bytes(Node.from_str(text, is_taproot=tap).script)


class Scripts:
    """Scripts and the miniscript text each was compiled from."""

    def __init__(self):
        self.texts = {}

    def add(self, text, tap):
        script = compile_ms(text, tap)
        self.texts[(script, tap)] = text
        return script

    def node(self, script, tap):
        """python-bip380's miniscript for a script added before; None
        for any other script."""
        text = self.texts.get((bytes(script), tap))
        if text is None:
            return None
        node = Node.from_str(text, is_taproot=tap)
        if bytes(node.script) != bytes(script):
            raise AssertionError("python-bip380 compiles the text to another script")
        return node


def has_pkh(node):
    """Whether a miniscript checks a key by its hash anywhere."""
    from bip380.miniscript.fragments import Pkh
    return isinstance(node, Pkh) or any(has_pkh(sub) for sub in node.subs)


def satisfy(node, signatures, max_sequence, max_lock_time):
    """The smallest non-malleable witness python-bip380 finds, or None.
    `signatures` maps a key's bytes to its signature."""
    material = SatisfactionMaterial(signatures=dict(signatures), max_sequence=max_sequence,
                                    max_lock_time=max_lock_time)
    return node.satisfy(material)
