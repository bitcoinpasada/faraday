# WIF private keys

Bitcoin Core's own vectors for the text forms of keys and addresses,
which `faraday-bip`'s WIF reader (`core/faraday-bip/src/wif.rs`) is
tested against (`core/faraday-bip/tests/all/wif.rs`): every private key
in the valid file reads to its key, its form (compressed or not) and its
chain, and nothing else in either file reads as a private key, as Core's
`key_io_tests.cpp` checks `DecodeSecret`.

| File | What | Source | SHA-256 |
|---|---|---|---|
| `key_io_valid.json` | Valid private keys (WIF, with the key's bytes, `isCompressed` and the chain) and valid addresses, copied unchanged | Bitcoin Core `src/test/data/key_io_valid.json`, tag `v31.1` (commit `9be056a8a72b624dae9623b2f7bded92c2a21c91`) | `90bd1d35d12763e0d00c5400b2c9fe551e532a821e1e466c20cad3aced70a7fe` |
| `key_io_invalid.json` | Strings that are no valid key or address on any chain, copied unchanged | Bitcoin Core `src/test/data/key_io_invalid.json`, the same tag | `c3ca74ddad7c01faaca7c26063537feb73ff2be03eb7fc83a42a655d2979261d` |

To refresh: download both files at a newer tag, check what changed and
why, and update the table.
