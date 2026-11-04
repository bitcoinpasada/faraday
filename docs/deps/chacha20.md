# chacha20

**Cryptography:** computes

**Purpose.** The stream cipher the KDBX 4 writer needs
(`core/faraday-backup/src/kdbx.rs`). KDBX encrypts its body and its inner random stream with plain
ChaCha20 (RFC 8439), not with an AEAD: the authentication is KeePass's
own HMAC-SHA-256 over the header and over each block of ciphertext.
It is also the generator of the Secure Boot keys
(`faraday/faraday-sb/src/rng.rs`): `ChaCha20Rng`, seeded from one fresh
32-byte draw, feeds the RSA key generation and the signing's blinding.

**Why this crate.** It is the RustCrypto stream cipher, by the publisher
F71 trusts, with the `cipher` 0.5 interface the tree's other primitives
use. Writing ChaCha20 by hand instead would be ~100 lines of
constant-time review to avoid a maintained crate.

**Cost.** Version 0.10 with `default-features = false` and the `cipher`
and `zeroize` features: no `alloc`. `faraday-backup` takes no RNG feature: its nonce and key come
from the file's header and the session key; `faraday-sb` takes `rng`
(over `rand_core` 0.10.1, already in the tree). No `unsafe` outside the
optional SIMD paths, which are gated on target features.

**Reopen when** KDBX is dropped and the Secure Boot generator changes.

**Review.** Covered by audits imported from other organisations (`supply-chain/imports.lock`); not read here.