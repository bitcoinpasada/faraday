# python-codex32, vendored

A BIP 93 (codex32) implementation in pure Python, copied unchanged, for
the crypto cross-check (`tools/crosscheck/area_codex32.py`, F79). BIP 93
itself carries only checksum fragments, not a whole implementation.

Two copies, from two commits: `codex32/`, the release v0.6.1, for
parsing, checksums, interpolation and the worksheets; and
`correction/codex32/`, a later commit whose `correction.py` is the error
correction decoder (`docs/CODEX32.md` §8.4), for
`tools/crosscheck/area_codex32_correct.py`. v0.6.1 has no decoder, and
the later package's API differs from it, so each area imports its own,
in a process of its own.

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

## The decoder

Source: `https://github.com/BenWestgate/python-codex32`, commit
`f19d4627c5725a9f6cfa995a1f89fe62e1becb4c` (no tag), folder
`src/codex32/`: `correction.py` and the modules it imports when the
package is imported, copied unchanged into `correction/codex32/`. Its
`LICENSE` and `LICENSES/BSD-3-Clause-Curr-Sneed.txt` are byte for byte
the ones above (`LICENSE`, `LICENSE-BSD-3-Clause`), so they are not
copied twice. Fetched from the repository's raw files at that commit on
2026-10-23. Left out, because nothing imported here reaches them: the
insertion and deletion search (`indel.py`, `_alignment.py`,
`_competitors.py`, imported only inside `correct()`, which the
cross-check does not call), the command line (`cli.py`, `_cli_*.py`)
and Bitcoin Core's wallet calls (`_bitcoin_core.py`). The cross-check
calls `_correct_fixed`, the fixed-length decoder, which the package's
frozen corpus test runs (`tests/test_correction_bch.py` at that commit).
The corpus is in `tools/vectors/codex32-correction/`.

| File | SHA-256 |
|---|---|
| `correction/codex32/__init__.py` | `c37544fd253cdec9fc41645b166436c008a5b05058d23847d384ffeffdc49fdc` |
| `correction/codex32/_bip32.py` | `79bd103695b16e148952938cc14d454bd823e8726a3d3421599fe2a043217aa6` |
| `correction/codex32/bech32.py` | `43f260285ef0c8e3d9891c006e63ef03a15fc0eff1e05ab789ebe20755150710` |
| `correction/codex32/bip93.py` | `3aadd72fff7969d2d8ee29296f57b700ddcec1484cd6fddbd34657f7a3c1e067` |
| `correction/codex32/checksums.py` | `c4f3858181e600d0932957845765a2b8e81f028449c34b59dfcc351e7b7fc423` |
| `correction/codex32/correction.py` | `3fab7f0fe4ccab44030960d309d2aff5aceb0364451495483120e1341e91b962` |
| `correction/codex32/errors.py` | `ee5fe2cfd6ffce4c6f497588a83b4c574b3c2837e10b4eff243eaa56f944d911` |
| `correction/codex32/generation.py` | `02f9ab0274297921ac1b07a6803ea734db197b0a48cd14c4944ba8657e9b0bee` |
| `correction/codex32/gf32.py` | `f239145d1ccf0f0b266fee9ee6dbba9d533e223ef388733c29454a7fb4baea30` |
| `correction/codex32/profiles/__init__.py` | `da517d33eb5a88fd949f1eba1605988f2a534dbff261b3188d9dfeba8480399f` |
| `correction/codex32/profiles/bip39.py` | `4c991d6d695514180de7ce41da410b3de69419f3d8a7da02f72f8eb678f53b1a` |
| `correction/codex32/profiles/cl32.py` | `3ad457dae0ff58ccfe5bff7743a3b163df7a3b93bbb66405242347027fb15b57` |
| `correction/codex32/profiles/ms32.py` | `c391e79357969b70e3fadc40a6e9db8ec3e5818fec288c5df9c7917992832088` |
| `correction/codex32/wallet.py` | `b18dc4b0650e07709c8803f5debd7db41d33345aac08209acf1edd5302eb4bbf` |
