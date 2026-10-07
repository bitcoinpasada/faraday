# hmac

**Purpose.** HMAC-SHA512 for BIP-32 master key derivation (through
`bitcoin`), the BIP-39 seed (through `pbkdf2`), the session PIN hash,
the sealed-value nonce prefix and every key derived from the session key
(`osk_crypto::hmac_sha512`, `SessionKey::derive`).

**Why this crate.** RustCrypto's generic HMAC over any `digest` hasher:
~200 lines, `no_std`, RFC 4231 vectors in its tests and in ours. Writing
HMAC by hand is easy and getting the key-padding edge cases right is
where hand-written versions go wrong; the crate has had those reviewed.

**Cost.** Version 0.13 with `default-features = false` and `zeroize`:
`digest` and its subcrates, already required by `sha2`.

**Reopen when** `sha2` is reopened; the two move together.
