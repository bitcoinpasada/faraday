# polyval

**Cryptography:** computes

**Purpose.** POLYVAL (RFC 8452), the carry-less multiplication in GF(2^128) that
`ghash` is built on, and through it the tag of `aes-gcm` for Krux's encrypted code
(KEF) version 20 and 21 (`docs/deps/ghash.md`, `docs/KEF.md` §11). Faraday names it
in no source file.

**Why this crate.** It is RustCrypto's, below `ghash`, with the processor's
carry-less multiply (PCLMULQDQ, ARMv8's PMULL) where it has them and a software
fallback with no secret-indexed table where it does not.

**Cost.** Version 0.7: one new crate in the lock file, over `cpubits`,
`cpufeatures` and `universal-hash`, which the tree already holds. Its `unsafe` is
the intrinsics backends, entered only after `cpufeatures` finds the instructions.

**Reopen when** `ghash` stops using it, or GCM leaves the tree.

**Review.** Trusted by publisher (github:RustCrypto/universal-hashes) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.
