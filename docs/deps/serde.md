# serde

**Purpose.** The interface `serde_json` reads through (`docs/deps/serde_json.md`):
`faraday_bip::json::parse` builds Faraday's `Value` with a `Visitor`, and
`faraday_intake::electrum::text_holds_secret` deserializes an Electrum file
into a visitor that keeps a flag per value and no string.

**Why this crate.** `serde_json` is written against it; there is no
other way to use that parser. Faraday uses only the `de` traits
(`Deserialize`, `Deserializer`, `Visitor`, `MapAccess`, `SeqAccess`) and
derives nothing.

**Cost.** Version 1.0.229 with `default-features = false` and `alloc`
(`faraday-bip` is `no_std`). Its lock entry lists `serde_core` and
`serde_derive`; `serde_json` already needed the first.

**Reopen when** `faraday-bip` and `faraday-intake` have no JSON reader
left.

**Review.** Trusted by publisher (David Tolnay, dtolnay) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.
