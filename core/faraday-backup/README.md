# faraday-backup

Two encrypted file formats, apart from any screen that asks for them.
`no_std` + `alloc`. Both stretch the passphrase with Argon2id at a cost the
file carries, and neither draws randomness: salt and nonces come from a seed
the caller supplies.

## What it computes

- `oskb`: the .oskb format (version 3): a key's words, a master seed, a note
  or a wallet's recovery sheet, sealed with XChaCha20-Poly1305 and padded to
  a multiple of 256 bytes. `docs/BACKUP-FORMAT.md` writes it byte for byte.
- `kdbx`: a KDBX 4 database that a KeePass app opens with the same
  passphrase (ChaCha20 body, HMAC-SHA-256 block stream).

## Written in this crate

Both file formats, by module (`oskb.rs`, `kdbx.rs`). Argon2id, the AEAD and
the KDBX cipher stack come from `argon2`, `chacha20poly1305`, `chacha20`,
`sha2` and `hmac`.

## Checked against

- `tools/vectors/backup/` (`.oskb` versions 2 and 3, one KDBX file), read by
  `tests/vectors.rs`: what this crate writes from the inputs in that
  folder's README, and what it reads back.
- `tools/backup/decrypt.py` reads a .oskb file with two published libraries;
  `tools/backup/read-kdbx.py` opens the KDBX file as another implementation
  would, and `pykeepass` where installed.
- `faraday/faraday-core/tests/all/kdbx.rs` for `kdbx::verify` end to end.

## `unsafe`

None. The workspace sets `unsafe_code = "forbid"`.

## Secrets

`oskb.rs` and `kdbx.rs` are in `tools/lint-secrets.sh`'s `SECRET_FILES`.
The words, seed, note and sheet fields (`Seed`, `Note` and the sheet's
descriptor, name and note) are `Zeroizing<Vec<u8>>`; the plaintext buffers
and the KDBX body, which holds the words in the clear until the cipher
runs, are `Zeroizing` too. Passphrase-derived keys are wiped through
`faraday_crypto::Secret` and `Zeroize`.
