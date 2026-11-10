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
| `pykeepass-chacha20-argon2id-plain-large.kdbx` | ChaCha20 | Argon2id | none | 4.0, one 1.2 MiB attachment (`attachment.bin`, SHAKE-256 of `kdbx vector attachment`): the stream has more than one block |
| `pykeepass-chacha20-argon2id-plain-history.kdbx` | ChaCha20 | Argon2id | none | 4.0, the entry's earlier password in its history and a second entry in the recycle bin |
| `pykeepass-chacha20-argon2id-gzip-nearly-full.kdbx` | ChaCha20 | Argon2id | gzip | 4.0, one attachment (`disk.img`) of 128 MiB less 64 KiB of zeros, so the contents sit 64 KiB under the vault's largest (`faraday_vault::file::CEILING`) in about 130 KiB on disk: the test that a seal past the ceiling is refused opens it |
| `keepassxc-2.7-aes-argon2d-gzip-edited.kdbx` | AES-256 | Argon2d | gzip | 4.0, as KeePassXC 2.7.12 saved it after an edit: the `Example` entry's password changed twice (`hunter4`, with three earlier versions in its history), its `note.txt` attachment (`public attachment`), a group `Second` holding an entry `Other` (user `bob`, password `secret5`), and the database's custom data (`KPXC_DECRYPTION_TIME_PREFERENCE`, `KPXC_RANDOM_SLUG`, `_LAST_MODIFIED`); on `Example`, a tag (`sample`), icon number 5 and an item of custom data (`my.note`), set with pykeepass before `keepassxc-cli` saved the file, which kept all three |
| `keepassxc-keyfile-password.kdbx` | AES-256 | Argon2d | gzip | 4.0, `pykeepass-aes-argon2d-gzip.kdbx` locked by `keepassxc-cli` with the key file `keepassxc.keyx` as well as its passphrase (`kdbx vector passphrase`), plus an entry `Other` (user `bob`, password `secret5`) |
| `keepassxc-keyfile-only.kdbx` | AES-256 | Argon2d | gzip | 4.0, as above with the key file and no passphrase (refused: a vault has a passphrase) |
| `kdbxweb-yubikey-password.kdbx` | AES-256 | Argon2d | gzip | 4.0, made by kdbxweb 2.1.1 (KeeWeb's library, MIT) under the passphrase `keyed vector passphrase` and a YubiKey's challenge-response function, HMAC-SHA1 under the secret `000102030405060708090a0b0c0d0e0f10111213`; entries `Example` and `Other` as above |
| `keepassxc-3.1-aeskdf.kdbx` | AES-256 | AES-KDF | gzip | 3.1 (refused) |

How they are made: `python3 -I -B tools/vectors/kdbx/make.py OUT_DIR [NAME...]`
(the distribution's pykeepass, 4.1.1.post1, and its construct library), which changes pykeepass's
blank database to each shape and reads it back under pykeepass. pykeepass
writes KDBX 4.0 only; the 4.1 file is the same database with the minor
version in its header set to 1, the only difference 4.1 makes outside the
XML. `keepassxc-3.1-aeskdf.kdbx` is `keepassxc-cli import` 2.7.12's
output (passphrase `pw`), which writes KDBX 3.1 under AES-KDF, so it is
the vector for the older version Faraday names and does not open.

`keepassxc-2.7-aes-argon2d-gzip-edited.kdbx` is `pykeepass-aes-argon2d-gzip.kdbx`
given a tag, an icon and custom data by pykeepass and then edited by `keepassxc-cli` (`make.py`'s `keepassxc_edited`, name `keepassxc-edited`);
KeePassXC kept the file's cipher, KDF (8 MiB, 2 passes, 2 lanes), compression and
version, and pykeepass reads it back. It is not reproducible byte for byte
(KeePassXC draws its own randomness): the file is the vector.

The KeePass export's vector (`tools/vectors/backup/`) is read and written
again byte for byte by the same tests.

`reread.py FILE PASSPHRASE` opens a file with pykeepass and prints its entries, for the test that reads back what `faraday-vault` wrote.

`keyed.py` makes and reads, with pykeepass, a file that needs a key file or a
YubiKey's secret (`docs/VAULT.md` §3.2); `faraday/faraday-vault/tests/keyed.rs`
runs it in both directions. It writes nothing into this folder.

`keyfile_write.py OUT_DIR` makes `faraday-key-file.keyx` (a KeePass XML key
file version 2.0 of fixed bytes, laid out as KeePassXC 2.7.10 writes it) and
`pykeepass-key-file.kdbx` (pykeepass 4.1.1: a KDBX 4 file under the
passphrase `key file writer passphrase` and that key file, one entry
`entry-a`); `keepassxc-cli` 2.7.10 opens it with that key file. Faraday's own
key file writer is held to the first byte for byte by
`faraday/faraday-vault/tests/keyfile_write.rs`. `keepassxc-key-file.keyx` is
a key file `keepassxc-cli db-create --set-key-file` wrote, which the writer
gives back unchanged. Same licence as the other made vectors.

`keepassxc.keyx` is the key file `keepassxc-cli` made (a KeyFile 2.0
document). `make-keepassxc.sh OUT_DIR` makes it and the two `keepassxc-keyfile-*`
files (keepassxc-cli 2.7.10; `db-create` writes KDBX 3.1, so they start as
`pykeepass-aes-argon2d-gzip.kdbx`, which `db-edit` keeps as KDBX 4). Like the
edited file they are not reproducible byte for byte. `make-yubikey.sh OUT_DIR`
(with `make-yubikey.mjs`) makes the kdbxweb file, with `npm install` of kdbxweb
2.1.1 and hash-wasm 4.13.0 (MIT, Argon2) into a scratch directory; kdbxweb takes
the YubiKey as a function from the challenge to the 20-byte response, so no
hardware is involved. kdbxweb ends the inner header with a 4-byte field (as the outer one), where KeePass writes it empty and Faraday requires that; the script writes that one field empty and changes nothing else. Faraday's reader computing the same composite key from the
secret is the outside check of its YubiKey response.
