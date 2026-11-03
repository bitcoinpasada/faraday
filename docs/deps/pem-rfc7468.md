# pem-rfc7468

**Cryptography:** none

**Purpose.** PEM (RFC 7468) for the private keys and certificates of a
Secure Boot item in a KDBX vault (`faraday/faraday-vault/src/kdbx/items/`,
`docs/VAULT.md` §7): a `PRIVATE KEY` or `CERTIFICATE` block read to its DER
bytes, its label checked, and DER written back as a block. A reader that
took another label, or text around the block, would let a field hold what
its name does not say, so a block of another label is not read.

**Why this crate.** It is the RustCrypto implementation, strict about the
encapsulation boundaries and Base64 (over `base64ct`), `no_std` with no
allocation in the calls used, so the keys are decoded and encoded into
Faraday's own wiping buffers. It is already in the graph under `der`
(`faraday-sb`'s RSA keys); no new crate enters the tree.

**Cost.** `pem-rfc7468` 1.0 with `default-features = false`: `decode` and
`encode` into buffers the caller passes, no `alloc`.

**Reopen when** a PEM form a KeePass app or `openssl` writes for these
fields is one the crate refuses.

**Review.** Trusted by publisher (github:RustCrypto/formats) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.
