# BIP-85's reference implementation, vendored

`bip85/` is BIP-85's reference implementation, copied unchanged, for the
crypto cross-check (`tools/crosscheck/area_bip85.py`, F79). It imports
`mnemonic` (vendored in `tools/reference/bip39/`), `base58` (vendored
here, unchanged) and `pycoin`, of which it uses only hardened BIP-32
derivation, WIF and xprv serialisation.

`shims/pycoin/` is written here, not upstream: those few calls, over
HMAC-SHA512 and base58check, refusing anything else (a non-hardened step
would need curve arithmetic). It stands in for all of pycoin so that the
reference runs unchanged; BIP-85's published vectors, which the
cross-check runs first, check it.

| What | Source | Commit |
|---|---|---|
| `bip85/__init__.py`, `bip85/app.py`, `LICENSE` | `https://github.com/ethankosakovsky/bip85` (no release tags) | `435a0589746c1036735d0a5081167e08abfa7413` |
| `base58/__init__.py`, `base58/COPYING` | `https://github.com/keis/base58`, tag `v2.1.1` | `11c293f4479fafffbc7b766fcb703a835c02dccd` |
| `shims/pycoin/` | written here | |

Fetched with `git clone` at those commits. Neither repository signs its
tags: what pins the copy is the commit hash and the digests below.

SHA-256 of every file, so a change to the vendored copy is visible. The
same digests are in `tools/crosscheck/references.sha256`, which the
cross-check checks before it imports anything, refusing to run on a
mismatch.

| File | SHA-256 |
|---|---|
| `LICENSE` | `a2129f2f0b009174a5be4a1b56c10fc9b505d5daf5b41335b042b5c9b8f9ff61` |
| `base58/COPYING` | `cf4694f040b7a31cd8ec3dbcd0b583a601c0d4c37de016be7aa0a06e3a6a3a54` |
| `base58/__init__.py` | `ec7bc75fe227f2f63f009206a8f621311ba26d8916344534ca2aae6a90832a9c` |
| `bip85/__init__.py` | `3cc141d108a6ca8ebcb965bd5e687907b14d062e7bd365ffa5e2e52e9739f089` |
| `bip85/app.py` | `15aa15b8f63b45a81282e8d51c5cd9883b1828adb3cda5c73174eda5ebf32559` |
| `shims/pycoin/__init__.py` | `0194a34385f4c75797961dbc808ad287547d38d4a4ddbf3c54aaf5b7901fe86b` |
| `shims/pycoin/encoding/__init__.py` | `d6a068ca1e73b1a66dfca95637f2233eaac9f1368c8c89bc598b62e798c059cc` |
| `shims/pycoin/encoding/bytes32.py` | `f91b8c817eadfa1ea961419864f0b97808831992c2b8ef1f9a2026429aa2b7e0` |
| `shims/pycoin/symbols/__init__.py` | `e1930b16118cdd57e68a7549116d709731c1bef7036d2b6c1998e0fb7ddeaf9c` |
| `shims/pycoin/symbols/btc.py` | `3ede16bb0f3e5f91227684ab56d914b32afd9ad118c2166fed7435c5ba571d80` |
