# faraday-backup

Argon2id at a cost a file carries, apart from any screen that asks for
it. `no_std` + `alloc`. The self-test runs it against a known answer. The
vault (`faraday-vault`) takes this crate's `Cost` but stretches its
passphrases through `faraday_crypto::argon2` itself, which `argon2id` wraps:
PyCA and argon2-cffi judge that shared call through the cross-check's
`vault.argon2id`, and the vault's own wrapper (Argon2d or Argon2id, the
cost mapping) is judged through whole files by the outside readers in the
`vault` area. A KeePass export is a one-entry vault (`docs/VAULT.md` §8),
written by `faraday-vault`; this crate no longer has a writer of its own.

## What it computes

- `argon2id`: the Argon2id run, with its working memory asked for rather
  than taken, at a `Cost` (`DEVICE_PARAMS` is the device's own, `MIN_PARAMS`
  Argon2id's minimum for tests).

## Written in this crate

The Argon2id helper in `lib.rs`, over the `argon2` crate through
`faraday_crypto::argon2`.

## Checked against

- The cross-check's vault area (`tools/crosscheck/area_vault.py`):
  Argon2id at random small costs against argon2-cffi and PyCA
  cryptography's, and the known answer in `tools/vectors/vault-primitives/`.
- The self-test's known answer (`core/faraday-selftest`) and this crate's own tests.

## `unsafe`

None. The workspace sets `unsafe_code = "forbid"`.

## Secrets

Passphrase-derived keys are wiped through `faraday_crypto::Secret`.
