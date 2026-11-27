# crypto-primes

**Purpose.** The prime search behind RSA-2048 key generation for Faraday's
Secure Boot keys (`core/faraday-sb`, `docs/FLOWS.md` "Secure Boot"). It
is not called by Faraday's own code; `rsa` (`docs/deps/rsa.md`) uses it to
find the two primes of a new key. Faraday-only.

**Why this crate.** It is the prime generator the RustCrypto `rsa` crate is
built on, `no_std`, over `crypto-bigint`, drawing candidates only from the
caller's `CryptoRng`; its tests (Miller-Rabin base 2 and a strong Lucas
test, BPSW) are the standard ones. Writing a prime search by hand over a
big-integer library was the alternative and is not one. Its primality
testing runs in variable time.

**Cost.** One crate with `default-features = false`; the optional `rayon`
feature is off. It adds nothing to the graph beyond `crypto-bigint`,
`rand_core` and `cpubits`, which `rsa` already brings.

**Reopen when** `rsa` moves to a release that no longer needs it.

**Review.** Not reviewed by a person: an exemption with a note in
`supply-chain/config.toml` (F71, `docs/decisions/F71.md`). Its publisher
(a single maintainer, not the RustCrypto organisation's `github:` publisher
that `rsa` and `crypto-bigint` are trusted by) is not one F71 names, so no
trust entry is written. An AI read of 0.7.2 is recorded in
`supply-chain/audits.toml` as `ai-says-safe-to-deploy`, which grants no
trust: it names what was read.
