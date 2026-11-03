# cbc

**Cryptography:** computes

**Purpose.** AES-256 in CBC mode with PKCS #7 padding, to decrypt the body
of a KDBX database a KeePass app saved under its default cipher
(`faraday/faraday-vault/src/kdbx`, `docs/VAULT.md` §7.1). Only decrypting:
Faraday writes ChaCha20, and nothing here encrypts under CBC.

**Why this crate.** It is the RustCrypto mode over the block cipher the
tree already trusts (`aes`, `docs/deps/aes.md`), by the same authors and
on the same `cipher` 0.5 as `chacha20` and the AEADs, so it adds one
crate and nothing under it. The ciphertext is authenticated by the block
stream's HMAC-SHA-256 before it reaches this crate, which is why an
unauthenticated mode is acceptable here and why the crate's warning about
padding oracles does not apply: a file whose HMAC fails is never
decrypted.

**Cost.** One crate, `cbc` 0.2 with `default-features = false`: without
its `block-padding` feature, which would bring `block-padding` (a crate
by a publisher F71 does not name) into the tree, so the PKCS #7 padding of
an authenticated body is removed in `faraday-vault` itself. `aes` there is
the same 0.9 as for AEZ, now with its block cipher used as well as its
round.

**Reopen when** KeePass apps write another cipher by default, or Faraday
stops reading files saved by other programs.

**Review.** Trusted by publisher (github:RustCrypto/block-modes) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.
