# BIP-174, BIP-370 and BIP-371 PSBT vectors

`vectors.json` is every PSBT of BIP-174's, BIP-370's and BIP-371's Test
Vectors sections, each with its case name and what the BIP says of it
(`invalid`, `valid`, `signer-fails`, and for BIP-370's timelock cases
`locktime N` or `locktime none`). The crypto cross-check's `psbt` area
(`tools/crosscheck/area_psbt.py`) reads it. `tools/vectors/psbt/bip174.txt`,
which Faraday's own tests read, is the same BIP-174 set taken earlier
from the BIP's master branch.

`extract.py` wrote it from the BIPs' texts in the bitcoin/bips
repository at commit `c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1`
(2026-10-22); nothing was retyped:

| File | SHA-256 |
|---|---|
| `bip-0174.mediawiki` | `f2a8e1a9c9e31cc7f607b3c7e2419c63eeccc4bd5b725968cde54ab8cfa1d410` |
| `bip-0370.mediawiki` | `b2b6e9099592fa04179383f9a1fd8b201fbeef4470f4c3f1c050d25589979c57` |
| `bip-0371.mediawiki` | `c2e8143a3c9aee0ee38d863cf40947ddf8ee67b4f40a748fb6925ccc5ad4b791` |

To refresh: download the three at a newer commit into a folder, check the
digests changed for a reason, update them in `extract.py`, and run
`python3 -I tools/vectors/psbt-bips/extract.py FOLDER`.
