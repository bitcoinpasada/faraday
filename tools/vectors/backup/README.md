# Encrypted-backup vectors

Each vector is an `osk-backup` file: a `.hex` with the bytes on a single
line and a `.oskb` with the same bytes as a file, which is what the
snapshot scripts hand the scanner and what a person would hold. The test
asserts each pair is equal, so neither can drift from the other. The
format is `docs/BACKUP.md`.

`backup-12-english` and `backup-24-japanese` are version 2, which this
build still opens and no longer writes. Both are 123 bytes: a version-2
backup is the same length whatever key it holds, so two vectors of
different word counts are here to show it.

The four `backup-v3-*` vectors are version 3, one per payload kind, and
are what this build writes. The words and the seed vectors are both 345
bytes, because a key's two doors share a padding step; the note is 345
too, and the sheet is 601 because its payload needs a second step.

The vectors exist so that two implementations can be shown to agree on
the format: `core/osk-backup/tests/vectors.rs` opens them with
`osk-backup` and again with `tools/backup/decrypt.py`, and both must
print the same words.

## What is in them

Every vector is written under the passphrase `correct horse battery
staple`, at 65536 KiB, 3 passes and 1 lane, which is the cheapest of the
three memory costs Settings offers and what a device recommends where
the shell reports less than a gigabyte. The salt is each seed's first 32
bytes and the nonce its last 24.

| | version | payload | seed | length |
|---|---|---|---|---|
| `backup-12-english` | 2 | the BIP-39 all-zero-entropy key: `abandon` ×11 `about`, English | `00 01 … 37` | 123 |
| `backup-24-japanese` | 2 | 32 bytes of `42`, Japanese, 24 words | `80 81 … b7` | 123 |
| `backup-v3-words-12-english` | 3 | the same 12-word English key | `00 01 … 37` | 345 |
| `backup-v3-seed-32` | 3 | a 32-byte master seed, every byte `42` | `80 81 … b7` | 345 |
| `backup-v3-note` | 3 | the note `Keys in the safe.\nWords with the notary.\n` | `40 41 … 77` | 345 |
| `backup-v3-sheet` | 3 | a 2-of-3 `wsh(sortedmulti(...))` descriptor, the name `Cold storage`, and a note | `c0 c1 … f7` | 601 |

The salt and the nonce are counted up on purpose: a vector is read by
people, and a fixed pattern is checked at a glance. The two vectors start
from different seeds because they share a passphrase, and a salt reused
across two files costs an attacker one Argon2id run for both. Nothing the
device writes looks like this — a real backup draws both from the session
key.

## How they were made

`encrypted::seal` over those keys and that passphrase, at those costs,
with those seeds. The test rebuilds each from exactly those inputs and
compares, so a file cannot drift from the code that writes it: if the
format changes, the test fails until the vectors are written again.

## The KDBX vector

`backup-kdbx-words-12-english.kdbx` is the other form the same key
leaves in: a KDBX 4.0 database holding one entry, titled `73c5da0a`,
whose password is the same twelve English words and whose notes are
`12 words, English`. It is written under the same passphrase and at the
same Argon2id cost as the others, from 140 bytes of seed — the master
seed, the encryption IV, the Argon2id salt and the inner random
stream's key — counted up from `00` to `8b`. What is in one is
`docs/BACKUP.md`.

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

## Reading one by hand

```
python3 tools/backup/decrypt.py tools/vectors/backup/backup-v3-sheet.oskb \
    --passphrase 'correct horse battery staple'
```
