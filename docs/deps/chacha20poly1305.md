# chacha20poly1305

**Cryptography:** computes

**Purpose.** The AEAD behind `faraday_crypto::Sealed`: a loaded key's seed and words are encrypted in memory
under the session key and decrypted only inside a closure.

**Why this crate.** RustCrypto's ChaCha20-Poly1305 (RFC 8439): pure Rust,
`no_std`, constant-time by construction (no table lookups indexed by
secret data), audited in 2020 alongside the rest of the `AEADs`
repository, and the same maintainers as the `sha2`, `hmac` and `pbkdf2`
crates already in use, so the trait generation (`digest` 0.11, `cipher`
0.5, `aead` 0.6) stays consistent. AES-GCM would need either hardware
acceleration or a bitsliced software path to be constant-time; ChaCha
does not.

**Cost.** Version 0.11 with `default-features = false` and the `zeroize`
feature (the cipher state wipes on drop): `aead`, `chacha20`, `poly1305`,
`cipher`, `universal-hash`, `inout`, `crypto-common` and `hybrid-array`
(the last three already in the graph). No `alloc`, no RNG feature: the
nonce comes from the session key's counter, which is the point of §16.21.
No `unsafe` outside the optional SIMD paths, which are gated on target
features and absent from the Pi and generic builds.

**Reopen when** the crate count forces a
choice between this and an in-house implementation (~300 lines, but
constant-time review is the cost), or if a platform's hardware AEAD is
worth the shell-side plumbing.

**Review.** Trusted by publisher (tarcieri) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.