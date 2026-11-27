# KDBX vector

`backup-kdbx-words-12-english.kdbx` is a KDBX 4.0 database holding one
entry, titled `73c5da0a`, whose password is the twelve words of the
all-zero-entropy English key (`abandon` ×11 `about`) and whose notes are
`12 words, English`. It is written under the passphrase `correct horse
battery staple`, at 65536 KiB, 3 passes and 1 lane, from 284 bytes of
seed (the master seed, the encryption IV, the Argon2id salt, the inner
random stream's key and nine UUIDs, the group's first) counted up from
`00` to `ff` and on from `00` to `1b`. It is the file the app's earlier,
separate export writer made: a KeePass export is now a one-entry vault
(`docs/VAULT.md` §8), and this file is the proof that the vault's code
opens it and writes it again byte for byte.

`faraday/faraday-vault/tests/kdbx.rs` (`the_export_vector_opens_and_is_written_again_byte_for_byte`)
opens it with its passphrase and seals the same document from the same
bytes, and compares.

There is no `.hex` beside it: a KDBX file is never scanned, and 1955
bytes do not fit a QR.
