# Vault vectors

File vaults in the format of `docs/VAULT.md` (KDBX 4), and the files a
reader refuses, published for any implementation to open. Faraday's Rust
(`faraday/faraday-vault/tests/vectors.rs`) and `tools/vault/open.py` with
pykeepass (`tests/open_py.rs`) read them; the cross-check does not, and a
run of them by outside programs is later work. A reproduction by an
outside party is asked for in `docs/AUDIT-SUBMISSIONS.md` §10.

Every file is made at the lowest cost a reader accepts: Argon2id with
8 MiB of memory, one pass and one lane, except `vault-sealed.kdbx`. The
passphrases, ids, clock (2023-11-14T22:13:20Z) and random bytes are fixed
and public; the words are BIP-39's all-`abandon` test phrase, and no coin
is behind anything here. The GPG and Secure Boot keys are placeholders.

| File | Passphrase | What it holds |
|---|---|---|
| `vault-all.kdbx` | `correct horse battery staple` | 256 KiB, "Everything": one item of every kind of §7 (two seeds, a words one with every custom data and a hex one; two wallets, a policy and a descriptor), a person's entry with its TOTP, a backup plan and signed amounts |
| `vault-small.kdbx` | the same | 64 KiB, "Small": an entry and a note |
| `vault-codex32.kdbx` | the same | "Shares": BIP 93 vector 2's seed with its codex32 set's facts, and a share |
| `vault-nfd.kdbx` | `café` in NFD (U+0065 U+0301) | "Accents", made with the passphrase in NFD; it opens under the NFC form (U+00E9) too (§3) |
| `vault-sealed.kdbx` | the same as `vault-all` | made, opened and sealed once more through `file::seal`: at 64 MiB, 3 passes and 4 lanes, seal count 2, a note and an entry added, renamed "Sealed again" |
| `cut-stream.kdbx` | the same | a vault whose block stream ends before its empty last block: damaged (§4) |
| `refused-*.kdbx` | the same | one file per rule that refuses a file (§1, §3, §4, §7, §7.1): a file that is not KDBX, KDBX 5 and 4.2, an unknown header field, AES-KDF, another KDF, another cipher, another compression, memory below 8 MiB and above 1 GiB, passes 0 and 101, lanes 0 and 65, Argon2 version 0x10, a secret key in the KDF parameters, a KDF without its version, a changed header hash, header HMAC (read as a wrong passphrase), block HMAC and block data, a file cut inside a block, and an XML document not well formed, with a DOCTYPE, an entity, a declaration after the start, another encoding, bytes after the root, nesting past 64, a protected value not canonical Base64, not UTF-8, or a root that is not `KeePassFile` |

KDBX 3.1 and Twofish are refused by files of other programs, used where
they are: `tools/vectors/kdbx/keepassxc-3.1-aeskdf.kdbx` and
`pykeepass-twofish-argon2d-gzip.kdbx`. Not here: Salsa20 as the inner
stream and an empty passphrase (this build writes neither, so no file of
its own makes them), a file too long for the largest vault (6 MiB of
it would be committed; `tests/kdbx.rs`'s
`a_file_too_long_for_the_largest_vault_is_refused_before_any_work` makes
one), and a QR vault's vectors (the QR vault is not built).

How they are made:

    cargo run --locked -p faraday-vault --example vectors -- tools/vectors/vault

writes every file byte for byte as committed
(`faraday/faraday-vault/examples/vectors.rs`). The item vaults put each
item into a document as `file::seal` does and seal at the cheap cost,
because `file::seal` always writes the 64 MiB step; `vault-sealed.kdbx` is
the one made through `file::create` and `file::seal`.

To open one:

    python3 tools/vault/open.py tools/vectors/vault/vault-all.kdbx

reads the passphrase at the prompt and prints the vault and each item
under §7's names (`docs/VAULT.md` §10).
