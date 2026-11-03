# base64ct

**Cryptography:** encodes

**Purpose.** Standard base64 (RFC 4648 §4, `+/` alphabet, `=` padding) in
four places: `faraday_bip::base64` (PSBT versions 0 and 2, BIP-322
signatures, BSMS, and BIP-85's password through `encode_into`),
`faraday-pgp`'s armour, `faraday-backup`'s KDBX writer (protected values
and the UUID) and the Starter's macOS property-list reader (`diskutil`'s
data) and `faraday-vault`'s KDBX XML reader and writer (UUIDs, times and
protected values, decoded strictly).

**Why this crate.** It is RustCrypto's base64, by the publisher the hashes
and ciphers here come from, and the lock file already holds it (through
`argon2`, `spki` and `pem-rfc7468`). It is `no_std`, and its encoding and
decoding are constant time with respect to the data, which BIP-85's
password needs, since the password is a secret. It refuses padding bits
that are not zero and a missing `=`, so an encoding has one spelling.

**Cost.** Version 1.8.3 with `default-features = false` and the `alloc`
feature: no new crate in the lock file. The edges `faraday-vault`, `faraday-bip`,
`faraday-pgp`, `faraday-backup` and the Starter's shell to `base64ct` are
new.

**Reopen when** no Faraday crate has a base64 caller left, or the crate
stops being constant time with respect to the data.

**Review.** Trusted by publisher (github:RustCrypto/formats) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.
