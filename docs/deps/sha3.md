# sha3

**Cryptography:** computes

**Purpose.** An OpenTimestamps proof may take a value through Keccak-256
(the original padding, as Ethereum uses it), so walking a proof
(`faraday/faraday-ots`, `docs/decisions/F230.md`) needs the hash. A path
through it is named and not counted; it is never used for a signature or
a key.

**Why this crate.** It is the RustCrypto implementation, by the same
authors as `sha2` and `sha1` already in the graph, `no_std`, with
`Keccak256` beside SHA3-256, on the same `digest 0.11` generation. It
adds `sha3` and `keccak` (`docs/deps/keccak.md`).

**Cost.** One crate, `sha3` 0.11 with `default-features = false`.

**Reopen when** `faraday-ots` stops following paths through Keccak-256.

**Review.** Trusted by publisher (github:RustCrypto/hashes) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here. The older `sha3` 0.10 that the rPGP cross-check program takes is a separate, exempted version (`docs/deps/pgp.md`).
