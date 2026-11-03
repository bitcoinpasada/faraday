# der

**Purpose.** The CMS SignedData and Authenticode structures that
`faraday-sb` writes and reads (`faraday/faraday-sb/src/cms.rs`): a
SignedData with one signer, the `.auth` file's detached form, Authenticode's
SpcIndirectDataContent and the signed attributes, as `der` types. Faraday
wrote the DER by hand before (a 164-line `der.rs`); only these structures
stay Faraday's own (`docs/ESTABLISHED.md` §3.2). Faraday-only.

**Why this crate.** It is RustCrypto's ASN.1 DER, already in the graph
(through `rsa`, `spki`, `pkcs8` and `x509-cert`), and the one place that
decides how a length, a SET OF or an INTEGER is written, so the
structures are DER by the crate's own encoder and a signature covers
what it encoded. Naming it makes it a direct dependency of `faraday-sb`.

**Cost.** Version 0.8 with `default-features = false` and the `alloc` and
`oid` features: no new crate in the lock file beyond `x509-cert`'s
(`der_derive`, `flagset`). The structures are written with the crate's
`Encode`, `Decode` and `Sequence` traits and no derive of Faraday's.

**Reopen when** `faraday-sb` writes no CMS or Authenticode structure.

**Review.** Trusted by publisher (github:RustCrypto/formats) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.
