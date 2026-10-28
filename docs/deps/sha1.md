# sha1

**Cryptography:** computes

**Purpose.** The fingerprint of a version-4 OpenPGP key is SHA-1 of the
key (RFC 4880 §12.2), and GnuPG names keys by it, so Faraday's GPG keys
(`faraday/faraday-pgp`, `docs/history/PLAN.md` §7) need SHA-1 to say which key they
are. It is used for fingerprints only: every signature Faraday writes is
over SHA-256 or SHA-512.

**Why this crate.** It is the RustCrypto implementation, by the same
authors as `sha2` and `hmac` already in the graph, `no_std`, and version
0.11 is on the same `digest 0.11` generation, so it adds one crate and
nothing under it.

**Cost.** One crate, `sha1` 0.11 with `default-features = false`.

**Reopen when** Faraday writes version-6 keys, whose fingerprints are
SHA-256; this crate then goes.

**Review.** Trusted by publisher (github:RustCrypto/hashes) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.