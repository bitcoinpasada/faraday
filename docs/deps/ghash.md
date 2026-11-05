# ghash

**Cryptography:** computes

**Purpose.** The GHASH function of GCM (NIST SP 800-38D), which `aes-gcm` calls to
authenticate the 4-byte tag of Krux's encrypted code (KEF) version 20 and 21
(`docs/deps/aes-gcm.md`, `docs/KEF.md` §11). Faraday names it in no source file; it
is in the tree below `aes-gcm`.

**Why this crate.** It is RustCrypto's, the one `aes-gcm` is written over, a thin
wrapper that uses `polyval` for the field arithmetic. Using another would mean
patching `aes-gcm`.

**Cost.** Version 0.6: one new crate in the lock file, over `polyval`
(`docs/deps/polyval.md`).

**Reopen when** `aes-gcm` stops using it, or GCM leaves the tree.

**Review.** Trusted by publisher (github:RustCrypto/universal-hashes) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.
