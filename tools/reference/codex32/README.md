# python-codex32, vendored

A BIP 93 (codex32) implementation in pure Python, copied unchanged, for
the crypto cross-check (`tools/crosscheck/area_codex32.py`, F79). BIP 93
itself carries only checksum fragments, not a whole implementation.

`shims/bip32.py` is written here, not upstream: python-codex32 imports
`bip32` (a package over libsecp256k1) only to derive a default identifier
from a seed's fingerprint. The cross-check always passes the identifier,
so the shim refuses that one call.

Source: `https://github.com/BenWestgate/python-codex32`, tag `v0.6.1`,
commit `4bb90c018c7c515c3d54cf669b89beb42fa26227`, folder `src/codex32/`,
`LICENSE` (MIT) and `LICENSES/BSD-3-Clause-Curr-Sneed.txt` (here
`LICENSE-BSD-3-Clause`, the licence of the parts taken from the BIP 93
authors' code). Fetched with `git clone` at that tag. The tag is not
signed: what pins the copy is the commit hash and the digests below.

SHA-256 of every file, so a change to the vendored copy is visible. The
same digests are in `tools/crosscheck/references.sha256`, which the
cross-check checks before it imports anything, refusing to run on a
mismatch.

| File | SHA-256 |
|---|---|
| `LICENSE` | `2c451c18c8a93f46ef08fefdb5e99c2250ebdb48f8657506fa721a04de709372` |
| `LICENSE-BSD-3-Clause` | `6800f8a021d9e4efb823862ceb9de410299055a1248a6004551b8ad416893b7b` |
| `codex32/__init__.py` | `64746fcaa1f3fef705cb0759833a5b3d9313adf12851e360daf3f53e8dd6ec01` |
| `codex32/bech32.py` | `94977ed02dc57a90d3d5080e0eac0bf31ec8073a0a3d358de51039dd972dd921` |
| `codex32/bip93.py` | `6630e1653d00b50e28d060fa32b70c0c4589b0c39a35dea978b4269022811c9d` |
| `codex32/checksums.py` | `5217f60781332ccfd38673c2de49ab113182467906b976b46eef9c4fbee39a5b` |
| `codex32/errors.py` | `b1c5c48d6d9c9486c9603eab2078fbc227c1369c574d502a1b7a8cb7c3e4c9cd` |
| `codex32/segwit_addr.py` | `4fc427d92a69a1155656eb5ba635033bf23b714b1fd4d0aca35ba0a8ba565ac2` |
| `shims/bip32.py` | `5f89c27d305125ac6223ea690207892b1b135face229b266a6db71db23c177a1` |
