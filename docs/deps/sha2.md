# sha2

**Purpose.** SHA-256 for BIP-39 checksums and dice-roll hashing, SHA-512
as the PRF behind HMAC and PBKDF2 (`osk_crypto::{sha256, hmac_sha512,
pbkdf2_hmac_sha512}`).

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
