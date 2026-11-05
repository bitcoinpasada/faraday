# cbc

**Cryptography:** computes

**Purpose.** AES-256 in CBC mode, two uses. It decrypts the body of a KDBX
database a KeePass app saved under its default cipher, with PKCS #7 padding
(`faraday/faraday-vault/src/kdbx`, `docs/VAULT.md` §7.1); there it only
decrypts, Faraday writes ChaCha20. It also encrypts and decrypts Krux's
encrypted code (KEF) versions 1, 10, 11 and 12, with the padding each version
names, NUL or PKCS #7 (`faraday/faraday-kef`, `docs/KEF.md` §11): CBC is written
there, so something now encrypts under it.

**Why this crate.** It is the RustCrypto mode over the block cipher the
tree already trusts (`aes`, `docs/deps/aes.md`), by the same authors and
on the same `cipher` 0.5 as `chacha20` and the AEADs, so it adds one
crate and nothing under it. In KDBX the ciphertext is authenticated by the block
stream's HMAC-SHA-256 before it reaches this crate, which is why an
unauthenticated mode is acceptable there and why the crate's warning about
padding oracles does not apply: a file whose HMAC fails is never
decrypted. Krux's CBC versions have no authentication of their own beyond the
format's SHA-256 check of the plaintext, which is Krux's design, read and written
as it is; the person gets no oracle, since nothing answers a guess but the app
on the same machine.

**Cost.** One crate, `cbc` 0.2 with `default-features = false`: without
its `block-padding` feature, which would bring `block-padding` (a crate
by a publisher F71 does not name) into the tree, so padding is handled
in `faraday-vault` and in `faraday-kef` themselves. `aes` there is
the same 0.9 as for AEZ, now with its block cipher used as well as its
round.

**Reopen when** KeePass apps write another cipher by default, Krux's format
drops its CBC versions, or Faraday stops reading files saved by other programs.

**Review.** Trusted by publisher (github:RustCrypto/block-modes) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.
