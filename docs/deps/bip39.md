# bip39

**Purpose.** The reference the tests compare Faraday's own BIP-39 with. It
is a dev-dependency only, of `faraday-bip` (`tests/all/crosscheck.rs`),
`faraday-entropy` (the XOR tests) and `faraday-samples` (the fingerprint
check): no release binary carries it. `faraday_bip::bip39` is Faraday's own
and is what the app runs.

**Why this crate.** An implementation Faraday did not write, so that the
word lists, the checksum and the seed derivation are checked against
others' code as well as against the published vectors. Its word lookup
takes a time that follows the words, which is why the app does not use it.

**Cost.** Version 2.2.2 with `default-features = false` and the features
`all-languages` and `zeroize`. Its dependencies are `bitcoin_hashes` and
`zeroize`, both already in the lock file.

**Reopen when** Faraday's BIP-39 is checked against another reference, or
the crate stops building without `std`.

**Review.** Not reviewed: an exemption (`safe-to-run`) in `supply-chain/config.toml`, which a dev-dependency needs; it does not reach a release.
