# ripemd

**Cryptography:** computes

**Purpose.** An OpenTimestamps proof may take a value through RIPEMD-160
(the older calendars' own step), so reading and walking a proof
(`faraday/faraday-ots`, `docs/decisions/F230.md`) needs the hash to know
what the path leads to. A path through it is named and not counted; it is
never used for a signature or a key.

**Why this crate.** It is the RustCrypto implementation, by the same
authors as `sha2` and `sha1` already in the graph, `no_std`, and version
0.2 is on the same `digest 0.11` generation, so it adds one crate and
nothing under it.

**Cost.** One crate, `ripemd` 0.2 with `default-features = false`.

**Reopen when** `faraday-ots` stops following paths through RIPEMD-160.

**Review.** Trusted by publisher (github:RustCrypto/hashes) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here. The older `ripemd` 0.1 that the rPGP cross-check program takes is a separate, exempted version (`docs/deps/pgp.md`).
