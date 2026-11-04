# serde_json

**Purpose.** The grammar of JSON (RFC 8259) for the wallet files other
wallets write: `faraday_bip::json::parse` reads a text through serde_json
and builds Faraday's `Value`, and `faraday_intake::electrum::text_holds_secret`
looks at an Electrum file's keystores to see whether one holds a private
key or a seed. `serde` (with `alloc` only) is its interface.

**Why this crate.** It replaces a parser of Faraday's own (about 300
lines) with the one the Rust ecosystem reads JSON with, so the grammar, the
string escapes and the surrogate pairs are others' code. The rules that
make an import safe stay in front of it and are Faraday's: a text over
1 MiB is refused before it is read, an object that names a key twice is
refused (`Error::DuplicateKey`, a key written with an escape counted),
and nesting deeper than 32 is refused (serde_json's own limit of 128
stays behind it). Numbers become the decimal text Rust writes for the
`u64`, `i64` or `f64` serde_json reads; `arbitrary_precision` is off,
since it changes serde_json for every user in the workspace.

**Secrets.** The Electrum check deserializes into a visitor that keeps a
flag per value and no string, so no copy of a key or seed is made beyond
serde_json's own scratch buffer for a string with escapes.

**Cost.** Version 1.0.151 with `default-features = false`, `alloc`
(`faraday-bip` is `no_std`) and `float_roundtrip`, without which a number's
`f64` can be one step from the nearest, so that the same number written two
ways reads as two values and another reader takes a third. It adds no
dependency; a common float costs about a tenth more to read, a hostile one
of hundreds of digits about four times. A member that needs `std` (the tests' writers)
asks for it. serde_json and its dependencies (`itoa`, `memchr`,
`serde_core`, `zmij`) were already in the lock file as dev-dependencies;
the edges `faraday-bip -> serde_json` and `faraday-intake -> serde_json`
and `serde` are new.

**Reopen when** `faraday-bip` has no JSON reader left, or serde_json stops
building without `std`.

**Review.** Trusted by publisher (github:dtolnay/json) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.
