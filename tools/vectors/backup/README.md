# KDBX vector

`backup-kdbx-words-12-english.kdbx` is a KDBX 4.0 database holding one
entry, titled `73c5da0a`, whose password is the twelve words of the
all-zero-entropy English key (`abandon` ×11 `about`) and whose notes are
`12 words, English`. It is written under the passphrase `correct horse
battery staple`, at 65536 KiB, 3 passes and 1 lane, from 140 bytes of
seed (the master seed, the encryption IV, the Argon2id salt and the inner
random stream's key) counted up from `00` to `8b`. The format is
`docs/KDBX.md`.

`core/faraday-backup/tests/vectors.rs` rebuilds the file from exactly
those inputs and compares, so it cannot drift from the code that writes
it, and reads it again with `tools/backup/read-kdbx.py` where Python's
libraries are installed.

There is no `.hex` beside it: a KDBX file is never scanned, and 1955
bytes do not fit a QR.

```
python3 tools/backup/read-kdbx.py \
    tools/vectors/backup/backup-kdbx-words-12-english.kdbx \
    --passphrase 'correct horse battery staple'
```

Where `pykeepass` is installed, that is what reads it, and its agreement
is the check that the file is KDBX and not merely self-consistent.
Where it is not, the script's own reader runs and says so.
