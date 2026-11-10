# keccak

**Cryptography:** computes

**Purpose.** The Keccak-f permutation that `sha3` (`docs/deps/sha3.md`)
runs for Keccak-256 in `faraday/faraday-ots`.

**Why this crate.** It is the permutation crate `sha3` 0.11 depends on,
by the same RustCrypto developers; nothing in Faraday calls it directly.

**Cost.** One crate, `keccak` 0.2, taken only through `sha3`.

**Reopen when** `sha3` leaves the graph.

**Review.** Trusted by publisher (github:RustCrypto/sponges) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here. The older `keccak` 0.1 that the rPGP cross-check program takes is a separate, exempted version (`docs/deps/pgp.md`).
