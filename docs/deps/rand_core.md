# rand_core

**Purpose.** RSA key generation in `rsa` 0.10 draws from a generator of
`rand_core` 0.10's `CryptoRng`; `faraday-sb`'s `rng.rs` implements it over
a ChaCha20 keystream keyed by 32 bytes of the system's entropy, which the
app passes in. Nothing in Faraday draws randomness inside a library.

**Why this crate.** It is the trait `rsa` takes; implementing it is the
only way to hand `rsa` a generator. `rand_chacha` was the alternative and
is on `rand_core` 0.9, which `rsa` 0.10 does not take; `chacha20` is
already in the graph for the vault, so the generator is a dozen lines.

**Cost.** One crate, already brought by `rsa`.

**Reopen when** `rsa` moves to another generator trait.
