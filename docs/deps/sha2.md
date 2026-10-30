# sha2

**Cryptography:** computes

**Purpose.** SHA-256 for BIP-39 checksums and dice-roll hashing, SHA-512
as the PRF behind HMAC and PBKDF2 (`faraday_crypto::{sha256, hmac_sha512,
pbkdf2_hmac_sha512}`). In the Starter, the SHA-256 of a stick read back
with the image's own IDs put back, hashed as it is read
(`faraday/shells/starter/src/writer.rs`, `docs/STICK-IDENTITY.md` §4.3),
as `faraday-payload` hashes the image it decompresses.

**Why this crate.** RustCrypto's implementation: pure Rust, `no_std`,
tested against the NIST vectors in its own suite and against the BIP-39
vectors in ours. `bitcoin_hashes` also ships SHA-256 and SHA-512, but its
API is shaped for Bitcoin tagged hashes rather than for feeding `hmac`
and `pbkdf2`, and using it for both would tie the KDF to rust-bitcoin's
release cadence. The two implementations cross-check each other in the
self-test.

**Cost.** Version 0.11 with `default-features = false` and `zeroize`
(the hasher state wipes on drop): `digest` with `crypto-common` and
`hybrid-array`, `cfg-if`, and `cpufeatures` (a runtime check for SHA
extensions on x86 and ARM; no `unsafe` on targets without them).

**Reopen when** `bitcoin` moves to `bitcoin_hashes` 1.x and that becomes
the only hash crate in the graph, which would make a second SHA family
worth removing. The 1.x copy that the `ur` crate brought left with it
(§16.61).

**Review.** Trusted by publisher (github:RustCrypto/hashes) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.