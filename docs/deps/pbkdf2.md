# pbkdf2

**Purpose.** PBKDF2-HMAC-SHA512, 2048 rounds, the BIP-39 mnemonic-to-seed
function (`osk_crypto::pbkdf2_hmac_sha512`).

**Why this crate.** The function is fifteen lines, but the crate's
version is the one the `bip39` dev-dependency cross-checks against and
it carries the RFC 6070 and BIP-39 vectors. With `hmac` already present
it adds no new primitive, only the iteration loop.

**Cost.** Version 0.13 with `default-features = false` and the `hmac`
feature: no crates beyond `hmac` and `digest`. The `password-hash`,
`parallel` and `simple` features (string-encoded hashes, rayon) stay
off.

**Reopen when** never on its own; it goes wherever `hmac` goes.
