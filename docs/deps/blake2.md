# blake2

**Cryptography:** computes

**Purpose.** BLAKE2b (RFC 7693) at a digest length of 1 to 64 bytes, for one
caller: AEZ's `Extract` step in `faraday-bip`, which hashes a key that is
not already 48 bytes into 48 bytes. LND's aezeed hands AEZ a 32-byte scrypt
key, so every aezeed read runs it. `faraday_crypto::blake2b` is a wrapper of
about 25 lines over the crate's `Blake2bVarCore`.

**Why this crate.** It is RustCrypto's BLAKE2, by the publisher the other
hashes here come from, and `argon2` already builds on it, so the lock file
holds it. Faraday wrote the compression function itself before; the crate's
is checked against RFC 7693's vectors and the digests CPython's `hashlib`
gives at every length, and `Blake2bVarCore` takes the length at run time as
the wrapper does. With the `zeroize` feature its state is wiped when it is
dropped.

**Cost.** Version 0.11 with `default-features = false` and the `zeroize`
feature: no new crate in the lock file. The edge `faraday-crypto -> blake2`
is new.

**Reopen when** `faraday-crypto` has no BLAKE2b caller left, or the crate
stops wiping its state.

**Review.** Trusted by publisher (github:RustCrypto/hashes) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.
