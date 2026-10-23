# Signature hashes

The published signature hashes the crypto cross-check's `tx` area
(`tools/crosscheck/area_tx.py`) checks Bitcoin Core's reference code and
Faraday with, before any random case.

| File | What | Source | SHA-256 |
|---|---|---|---|
| `core-sighash.json` | 500 legacy signature hashes over random transactions, scripts, inputs and hash types, copied unchanged | Bitcoin Core `src/test/data/sighash.json`, tag `v31.1` (commit `9be056a8a72b624dae9623b2f7bded92c2a21c91`) | `52cf23c2076e7f129c71d5508631d3e5ae3be1b1cb0585c0e23bbb4bb373e924` |
| `bip341-wallet-test-vectors.json` | BIP-341's wallet vectors (output keys, addresses, key-path signature hashes), copied unchanged | bitcoin/bips `bip-0341/wallet-test-vectors.json`, commit `c0c5aea41e6a334d34db4ca87ae53bbfd7f268d1` | `403e19fb81dd1f31e745699216308f61fb403774b2aafa87b631b8f7c042d37f` |
| `bip143.json` | BIP-143's thirteen example signature hashes: the unsigned transaction, the input, the scriptCode, the amount, the hash type and the hash | written by `extract.py` from bitcoin/bips `bip-0143.mediawiki` at the same commit | of the source text: `62bc71351563e68baeb12643c68355d217953ae9eb6a6e68b2b0323275b6beec` |

`core-sighash.json`'s hashes are written as Core prints a `uint256`,
byte-reversed. To refresh `bip143.json`, download the BIP's text, check
its digest changed for a reason, update it in `extract.py`, and run
`python3 -I tools/vectors/sighash/extract.py bip-0143.mediawiki`.
