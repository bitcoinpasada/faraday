# base16ct

**Cryptography:** encodes

**Purpose.** Hexadecimal text (RFC 4648 §8, base 16), in one place:
`faraday_crypto::hex`, which every Faraday crate that writes or reads hex
calls (private keys and their hex, typed entropy, BIP-85's hex, vault
fingerprints, transaction and PSBT values, the Starter's disk records),
and, directly, `faraday-pgp` (paperkey's base16 lines, through `upper`
and `mixed`) and `faraday-payload` (the payload manifest's SHA-256
sums); only `faraday-payload` does not depend on `faraday-crypto`.

**Why this crate.** It is RustCrypto's base16, by the publisher the hashes
and ciphers here come from, and the lock file already holds it (through
`serdect`). It is `no_std`, and it encodes and decodes "without
data-dependent branches or lookup tables", which a private key's hex
needs. `hex-conservative`, also trusted and in the lock, looks digits up
in a table and is not used for this. It checks the length before it reads
a digit.

**Cost.** Version 1.0.0 with `default-features = false` and the `alloc`
feature: no new crate in the lock file. The edges `faraday-crypto`,
`faraday-pgp` and `faraday-payload` to `base16ct` are new.

**Reopen when** no Faraday crate has a hex caller left, or the crate stops
being constant time with respect to the data.

**Review.** Trusted by publisher (github:RustCrypto/formats) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.
