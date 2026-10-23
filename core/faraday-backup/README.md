# faraday-backup

The KDBX 4 writer, apart from any screen that asks for it. `no_std` +
`alloc`. It stretches the passphrase with Argon2id at a cost the file
carries, and draws no randomness: salt and nonces come from a seed the
caller supplies.

## What it computes

- `kdbx`: a KDBX 4 database that a KeePass app opens with the same
  passphrase (ChaCha20 body, HMAC-SHA-256 block stream). `docs/KDBX.md`.
- `argon2id`: the Argon2id run, with its working memory asked for rather
  than taken; the self-test and the KDBX writer use it.

## Written in this crate

`kdbx.rs` and the Argon2id helper in `lib.rs`. The KDBX cipher stack comes
from `argon2`, `chacha20`, `sha2` and `hmac`.

## Checked against

- `tools/vectors/backup/` (one KDBX file), read by `tests/vectors.rs`: what
  this crate writes from the inputs in that folder's README.
- `tools/backup/read-kdbx.py` opens the KDBX file as another implementation
  would, and `pykeepass` where installed.
- `faraday/faraday-core/tests/all/kdbx.rs` for `kdbx::verify` end to end.

## `unsafe`

None. The workspace sets `unsafe_code = "forbid"`.

## Secrets

`kdbx.rs` is in `tools/lint-secrets.sh`'s `SECRET_FILES`. The KDBX body,
which holds the words in the clear until the cipher runs, is `Zeroizing`.
Passphrase-derived keys are wiped through `faraday_crypto::Secret` and
`Zeroize`.
