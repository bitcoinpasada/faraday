# ed25519-dalek

**Purpose.** Ed25519 signatures for Faraday's GPG keys
(`faraday/faraday-pgp`, `PLAN.md` §7): the certification primary key and
the signing subkey are Ed25519, and every self-signature, detached
signature and revocation certificate Faraday writes is one Ed25519
signature over an OpenPGP digest. Faraday-only: OpenSigner's own graph
does not use it.

**Why this crate.** It is the Ed25519 implementation the Rust ecosystem
reviews and uses most, by the dalek-cryptography maintainers, constant
time over `curve25519-dalek`, `no_std` without an allocator, and its keys
zeroize on drop. Version 3 is on the RustCrypto generation the rest of the
graph is on (`digest 0.11`, `sha2 0.11`, `signature 3`), so it adds no
second hashing stack. Writing Ed25519 by hand over a field implementation
was the alternative and is not one.

**Cost.** Version 3 with `default-features = false` and `zeroize`: five
crates, `ed25519-dalek`, `ed25519`, `signature`, `curve25519-dalek` and its
`curve25519-dalek-derive` proc macro (on `syn`, `quote` and `proc-macro2`,
which the graph already has for `zeroize_derive`). The default features
are off: `fast` (precomputed tables, which a signer making a handful of
signatures does not need), `std` and `rand_core`; Faraday's keys come from
a vault's seeds, never from an RNG inside the crate.

**Reopen when** OpenPGP v6 keys (RFC 9580) are wanted, which take Ed25519
the same way but would let SHA-1 go; or when an Ed25519 implementation
enters OpenSigner's own graph and one copy should serve both.
