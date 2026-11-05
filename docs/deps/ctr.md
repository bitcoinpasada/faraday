# ctr

**Cryptography:** computes

**Purpose.** AES-256 in CTR mode with a 32-bit big-endian counter (`Ctr32BE`), for
Krux's encrypted code (KEF) versions 15 and 16, and inside the GCM of 20 and 21
(`faraday/faraday-kef`, `docs/KEF.md` §11).

**Why this crate.** It is the RustCrypto mode over the `aes` block cipher the tree
already trusts, on the same `cipher` 0.5, by the publisher of `cbc`, so it adds one
crate and nothing under it. `aes-gcm` builds on it too, so the tree holds it either
way. CTR written here would be a second counter to check against Krux's vectors
(`tools/vectors/kef/`).

**Cost.** Version 0.10 with `default-features = false`: one new crate in the lock
file. It has no `unsafe` of its own.

**Reopen when** Krux's format drops its counter modes, or Faraday stops reading
Krux's codes.

**Review.** Trusted by publisher (github:RustCrypto/block-modes) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.
