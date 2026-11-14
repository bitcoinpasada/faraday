"""Makes `krux_codes.json`: the KEF codes of every version, sealed by Krux's own
`Cipher.encrypt` and `wrap` (the cross-check's Krux source, `tools/reference/krux/`).

Run from the repository root:
`python3 -B tools/vectors/kef/make_krux_codes.py > tools/vectors/kef/krux_codes.json`.
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "crosscheck"))

import json
import zlib

import area_kef as a
from area_kef import krux_kef

KEY = bytes([7] * 32)
VERSIONS = a.VERSIONS


def iv_for(v):
    return bytes(((i * 37 + 11) & 255) for i in range(a.iv_len(v)))


def make(plain, v, fail_unsafe=True):
    try:
        return a.krux_seal(KEY, plain, v, b"an id", 10_000, iv_for(v), fail_unsafe).hex()
    except Exception:
        return None


e16 = bytes(range(1, 17))
e32 = bytes(range(100, 132))
text = b"a longer note, with words in it, that goes past one block of sixteen bytes and a bit more"
wallet = (b"wsh(sortedmulti(2,[aabbccdd/48h/0h/0h/2h]xpub1,[11223344/48h/0h/0h/2h]xpub2))" * 20)[:700]
plains = {"entropy16": e16, "entropy32": e32, "text": text, "wallet": wallet}
rep = (b"0123456789abcdef" * 100)[:1000]


def raw_deflate(b):
    c = zlib.compressobj(9, zlib.DEFLATED, -10)
    return c.compress(b) + c.flush()


out = {
    "made_by": "selfcustody/krux tag v26.09.0 src/krux/kef.py (tools/reference/krux/, unchanged), "
               "its Cipher.encrypt and wrap on the shims of tools/crosscheck/, with key 07 x32, "
               "id 'an id', count 10000 and the IV bytes (i*37+11) mod 256; null where Krux refuses",
    "id": "616e206964",
    "count": 10000,
    "codes": {str(v): {n: {"plain": p.hex(), "code": make(p, v)} for n, p in plains.items()}
              for v in VERSIONS},
    "repetitive": {str(v): {"plain": rep.hex(), "code": make(rep, v)} for v in (7, 12, 16, 21)},
    "inflate_over": {"plain": raw_deflate(bytes(a.INFLATE_LIMIT + 1)).hex()},
    "inflate_at": {"plain": raw_deflate(bytes(a.INFLATE_LIMIT)).hex()},
}
for k in ("inflate_over", "inflate_at"):
    out[k]["code"] = make(bytes.fromhex(out[k]["plain"]), 15)
out["version_0_small"] = []
for n in range(1024):
    p = bytes([n & 255, n >> 8, 7, 7])
    out["version_0_small"].append([p.hex(), make(p, 0)])
print(json.dumps(out, separators=(",", ":")))
