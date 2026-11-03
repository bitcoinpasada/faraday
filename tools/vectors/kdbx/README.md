# KDBX vectors

KDBX 4 databases written by programs Faraday's authors did not write, for
`faraday/faraday-vault/tests/kdbx.rs` and the fuzz target `kdbx_file` to
read (`docs/VAULT.md` §7.1). Every `pykeepass-*` file holds one entry,
titled `Example`, with the user name `alice`, the password `hunter2` and
the URL `https://example.org/`, under the passphrase `kdbx vector
passphrase`, at a low cost, Argon2 with 8 MiB of memory, 2 passes and 2 lanes.

| File | Cipher | KDF | Compression | Version |
|---|---|---|---|---|
| `pykeepass-aes-argon2d-gzip.kdbx` | AES-256 | Argon2d | gzip | 4.0 |
| `pykeepass-aes-argon2id-gzip.kdbx` | AES-256 | Argon2id | gzip | 4.0 |
| `pykeepass-chacha20-argon2d-gzip.kdbx` | ChaCha20 | Argon2d | gzip | 4.0 |
| `pykeepass-chacha20-argon2id-gzip.kdbx` | ChaCha20 | Argon2id | gzip | 4.0 |
| `pykeepass-chacha20-argon2id-plain.kdbx` | ChaCha20 | Argon2id | none | 4.0 |
| `pykeepass-aes-argon2id-gzip-v41.kdbx` | AES-256 | Argon2id | gzip | header marked 4.1 |
| `pykeepass-twofish-argon2d-gzip.kdbx` | Twofish | Argon2d | gzip | 4.0 (refused) |
| `keepassxc-3.1-aeskdf.kdbx` | AES-256 | AES-KDF | gzip | 3.1 (refused) |

How they are made: `python3 -I -B tools/vectors/kdbx/make.py OUT_DIR`
(pykeepass 4.1.1 and its construct library), which changes pykeepass's
blank database to each shape and reads it back under pykeepass. pykeepass
writes KDBX 4.0 only; the 4.1 file is the same database with the minor
version in its header set to 1, the only difference 4.1 makes outside the
XML. `keepassxc-3.1-aeskdf.kdbx` is `keepassxc-cli import` 2.7.12's
output (passphrase `pw`), which writes KDBX 3.1 under AES-KDF, so it is
the vector for the older version Faraday names and does not open.

The KeePass export's vector (`tools/vectors/backup/`) is read and written
again byte for byte by the same tests.
