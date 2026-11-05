# aes-gcm

**Cryptography:** computes

**Purpose.** AES-256 in GCM with a 12-byte nonce and a 4-byte tag, for Krux's
encrypted code (KEF) versions 20 and 21 (`faraday/faraday-kef`, `docs/KEF.md`
§11). Krux, the project whose format this is, seals a seed's QR code under that
mode and no other.

**Why this crate.** It is RustCrypto's GCM, by the publisher the other AEADs here
come from, over the `aes` and `cipher` 0.5 and `aead` 0.6 the build already holds,
and the tag is compared in constant time (`ctutils`). A GCM of our own would be a
second piece to review for one tag length. Its `AesGcm<Aes256, U12, U4>` is the
mode Krux writes, checked here against Krux's own vectors
(`tools/vectors/kef/`).

**Why its `hazmat` feature.** The crate allows tags of 12 to 16 bytes always and
a tag of 4 or 8 bytes only under `hazmat`, named so because a short tag is much easier to forge
by guessing than a full one. Krux's format fixes the tag at 4 bytes,
so reading its codes needs the feature; nothing else in Faraday seals under GCM
and the feature is not on for any other use (`faraday-kef` is the only
dependent). The short tag is the format's own, read and written as Krux does; a
forged code is a risk of the format (`docs/KEF.md`), not of this crate.

**Cost.** Version 0.11 with `default-features = false` and the features `aes`
and `hazmat`: no `alloc`, no `getrandom`. It adds `aes-gcm`, `ghash` and
`polyval` (`docs/deps/ghash.md`, `docs/deps/polyval.md`) and `ctr`
(`docs/deps/ctr.md`) to the lock file; `aead`, `cipher`, `ctutils` and
`universal-hash` are already there.

**Reopen when** Krux's format changes its GCM tag, or Faraday stops reading Krux's
codes.

**Review.** Trusted by publisher (github:RustCrypto/AEADs) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.
