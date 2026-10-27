# unicode-normalization

**Purpose.** `faraday_bip::bip39::Mnemonic::parse` reads a typed
sentence in NFKD before it matches the words, as BIP-39 requires, so a
word typed composed (é, ぞ, a Hangul syllable), as most keyboards type
it, is the listed word. The tests and the sample tools that wrote NFKD
forms before use the same crate.

**Why this crate.** NFKD needs the Unicode decomposition and
compatibility tables; writing them by hand would be a copy of this
crate's generated tables without its tests. It is `no_std` with
`alloc` (built here without its `std` feature), and the `bip39` crate
normalizes with it too.

**Cost.** One crate and `tinyvec`, its small-vector buffer; both were
already in the graph as test dependencies.

**Reopen when** the standard library offers normalization, or a word
list is published that is not NFKD-invariant in a way this changes.

**Review.** `unicode-normalization` is covered by an audit imported from
other organisations (`supply-chain/imports.lock`); `tinyvec` 1.13.2, which
it brings, is an exemption, not reviewed by a human: its diff from the
imported audits' 1.13.3 was read by an AI model for the project, recorded
under `ai-says-safe-to-deploy`, which grants no trust (F71,
`supply-chain/audits.toml`).
