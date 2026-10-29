# btclib, ecc and btclib-wallet, vendored as their wheels

The cross-check's `msg` area asks btclib for BIP-137 (`btclib.ecc.bms`)
and btclib-wallet for BIP-322 (`btclib_wallet.bip322`), and compares
the answers with Faraday's (`tools/crosscheck/area_msg.py`). This folder
holds the four wheels those modules need, each byte for byte the file
PyPI publishes, whose SHA-256 is the one in btclib-wallet's own `uv.lock`
at the release below.

| Wheel | Project | Release, commit | Licence | SHA-256 |
|---|---|---|---|---|
| `btclib_wallet-2026.10.29-py3-none-any.whl` | btclib-wallet, <https://github.com/btclib-org/btclib-wallet> | v2026.10.29, `25f9638e25790816102bcc91261e3bf33384ac21` | MIT | `778c23fe9bb66615e20ca8101ea3d1fb61b9d63838cd0dd4d2651333541e0a38` |
| `btclib-2026.10.24-py3-none-any.whl` | btclib, <https://github.com/btclib-org/btclib> | v2026.10.24, `e69bb4a6275f6c069d037e5c371010158c655dc6` | MIT | `a4a88030aacb32543dc123db4b00baba164ff29990d75e47d42f77a088726a6f` |
| `ecc-2026.10.23-py3-none-any.whl` | ecc, <https://github.com/btclib-org/ecc> | v2026.10.23, `7790699f682f52ec172f180b7b49e6b277ee201f` | MIT | `ee86bc2f0622f8835b11d8f84f1f03d9d3ae7b30b17d3082bf6cd43af29d7bfc` |
| `typing_extensions-4.16.0-py3-none-any.whl` | typing-extensions, <https://github.com/python/typing_extensions> | 4.16.0 | PSF-2.0 | `481caa481374e813c1b176ada14e97f1f67a4539ce9cfeb3f350d78d6370c2e8` |

btclib-wallet 2026.10.29 asks for btclib 2026.10.24 or later and ecc
2026.10.23 or later; these are the releases its `uv.lock` locks, the ones
its tests ran on. Each licence allows copying with its notice; the
notices are in each wheel's `.dist-info/licenses/`.

The packages' files in the btclib, ecc and btclib-wallet wheels were
compared with the tagged commits above and are the same files. The wheels
are kept whole rather than unpacked: unpacked, their comments' citations of
btclib's own issues would read as citations this repository's commit check
refuses, and the files would have to be changed.

## How the run uses them

`common.use_wheels` unpacks the four wheels, after `references.sha256` has
been checked, into a scratch folder of each worker process's own, removed
at exit, and puts it first on the import path. (The packages read their
network and curve tables beside their modules, which an import from the
zip cannot serve.)

btclib's curve arithmetic is its own Python, not libsecp256k1: the area
calls `ecc.curves.set_libsecp256k1_serving(serving=False)` and stops if
btclib would still delegate. btclib's optional `secp256k1` extra is not
vendored.

## What it is, and who wrote it

btclib is by Ferdinando Ametrano and the btclib developers; btclib-wallet
was split out of it in 2026-09. Neither
shares code with rust-bitcoin, rust-secp256k1 or libsecp256k1, so with
the pure-Python curve it is the second lineage F160 item 7 names.
btclib-wallet's `bip322.py` verifies through btclib's script interpreter
and states the BIP's three results (valid, invalid, inconclusive); its
tests run the BIP's vectors and btcd's.

Its one gap the area meets: btclib's interpreter applies MINIMALIF to
witness v0 scripts only, so two of btcd's negative vectors, a legacy P2SH
script in the full form whose `OP_IF` takes a non-minimal true, are
valid to it (its `bip322.py` says so). Faraday reads no full form; the
area requires btcd to call them invalid and Faraday to refuse them.

## Updating

With network, in a scratch folder: take the release's `uv.lock`, download
the wheels it names from PyPI, check each against its hash there, compare
the wheels' packages with a checkout of the tagged commits, copy them here,
change the table above and `area_msg.py`'s list, and write the new hashes
into `tools/crosscheck/references.sha256`, in a reviewed commit.
