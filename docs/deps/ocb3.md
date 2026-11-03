# ocb3

**Cryptography:** computes

**Purpose.** AES-256 in OCB mode (RFC 7253), the one AEAD RFC 9580 requires
every implementation to have, for the QR vault's OpenPGP message
(`faraday/faraday-pgp/src/message.rs`, `docs/VAULT.md` §1.2): it wraps the
session key in the symmetric-key encrypted session key packet and encrypts
each chunk and the final tag of the data packet. The nonce is 15 bytes and
the tag 16, which this crate supports (nonces of 6 to 15 bytes).

**Why this crate.** It is RustCrypto's OCB3, by the publisher the other AEADs
here come from, over the `aes` and `cipher` 0.5 and `aead` 0.6 the build
already holds, and the tag is compared in constant time (`ctutils`). OCB is
checked here against RFC 9580's own vectors (A.10, AES-128 with OCB, byte
for byte) and against Sequoia, OpenPGP.js and rPGP in the F79 area `vault`.
Writing OCB here would be a second implementation to review for a mode used
in this one message.

**Cost.** Version 0.2 with `default-features = false`: no `alloc`, no
`getrandom`. It adds `ocb3` and `dbl` (the doubling in GF(2^128) OCB uses)
to the lock file; `aead`, `cipher`, `ctutils` and `hybrid-array` are already
there.

**Reopen when** RFC 9580's mandatory AEAD changes, or the QR vault's message
moves to another mode.

**Review.** Trusted by publisher (github:RustCrypto/AEADs) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here. `dbl` is trusted by its publisher (tarcieri) the same way.
