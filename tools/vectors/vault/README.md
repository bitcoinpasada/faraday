# Vault vectors

Vault files in the format of `docs/VAULT.md`, and slot plaintexts,
published for any implementation to open. Faraday's Rust
(`faraday/faraday-vault/tests/vectors.rs`), `tools/vault/open.py` and
the cross-check's `vault` area (`tools/crosscheck/area_vault.py`) open
them all. A reproduction by an outside party is asked for in
`docs/AUDIT-SUBMISSIONS.md` §10.

Every vault is made at the lowest cost a reader accepts: Argon2id with
8 MiB of memory, one pass and one lane, and 64 KiB slots. Every slot in
use holds its seal count, 1. The passphrases and the records are fixed
and public; the words in the records are BIP-39's all-`abandon` test
phrase, and no coin is behind anything here.

| File | Passphrases | What it holds |
|---|---|---|
| `vault-one.fvlt` | `correct horse battery staple` | one slot in use, holding one record of every type but the retired 12 and the codex32 ones |
| `vault-three.fvlt` | `first passphrase`, `second passphrase`, `third passphrase` | three slots in use, each holding the same kinds of record |
| `vault-codex32.fvlt` | `codex32 passphrase` | BIP 93 vector 2's key with its codex32 set (type 2, field 5), and a codex32 share record (type 15) |
| `vault-nfd.fvlt` | `café` in NFD (U+0065 U+0301) | made with the passphrase in NFD; it opens under the NFC form (U+00E9) too (§3) |
| `two-keys.fvlt` | `first of two keys`, `second of two keys` | one slot whose tag verifies under both passphrases' slot keys; refused under each for its commitment block (§4) |
| `slots.json` | none | slot plaintexts after the commitment block, written by hand from §7, each with what a reader makes of it: `ok`, or the fault that refuses it |

How they are made:

    cargo run -p faraday-vault --example vectors -- tools/vectors/vault
    python3 -I -B tools/vectors/vault/two_keys.py tools/vectors/vault

The first writes the four vaults other than `two-keys.fvlt`, byte for
byte as committed (`faraday/faraday-vault/examples/vectors.rs`); the
second writes `two-keys.fvlt` and checks with PyNaCl that its slot
verifies under both keys.

To open one:

    python3 tools/vault/open.py tools/vectors/vault/vault-three.fvlt

reads the passphrase at the prompt and prints the records of the slot it
opens (`docs/VAULT.md` §10).
