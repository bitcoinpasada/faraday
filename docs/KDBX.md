# KDBX 4: the encrypted file Faraday writes for others

**Kind:** Reference
**Status:** Built

## The file

Faraday writes a KDBX 4 database for a person whose reader is a KeePass
app. It is the one encrypted file Faraday writes for others, and it is
written and never read: nothing on the device opens a KDBX file.

The format is KeePass's own and is not restated here. It is specified at
<https://keepass.info/help/kb/kdbx.html>, with the KDBX 4 changes at
<https://keepass.info/help/kb/kdbx_4.html>; both pages are copied into
`tools/reference/kdbx4/` with the date they were fetched and their
SHA-256, and the implementation is
`core/faraday-backup/src/kdbx.rs`.

What this device puts in one:

| | What |
|---|---|
| version | 4.0 |
| cipher | ChaCha20, RFC 8439, with a 12-byte encryption IV |
| compression | none |
| key derivation | Argon2id 1.3, at the memory the person chose, 3 passes and 1 lane, with a 32-byte salt |
| composite key | the passphrase alone; no key file |
| authentication | the header's SHA-256 and HMAC-SHA-256, then the HMAC block stream over the ciphertext |
| inner random stream | ChaCha20, with a 64-byte key |
| document | one group named `Faraday`, one entry per item |
| attachments | none |

An entry carries a Title, a Notes and a Password field, the last marked
`Protected` and encrypted with the inner random stream. What goes in
them:

| Export | Title | Password | Notes |
|---|---|---|---|
| a key's words | the master fingerprint | the words, separated by single spaces whatever the wordlist | the word count and the wordlist |
| a master seed | the master fingerprint | the seed as lower-case hex | its length |
| a note | the note's first line, or `Note` where it starts blank | empty | the note |
| a recovery sheet | the wallet's name | empty | the descriptor and the sheet's note, under the headings the plain export uses |
| a vault entry | its title; "Password" for a password BIP-85 made, whose title names its parent and index (F172) | its password | its username and URL, then its notes, a line that is a BIP-85 path left out |

A KeePass app pays the Argon2id cost this device paid. The entry's title
inside carries the fingerprint, the note's first line or the wallet's name,
and an entry's file is named after its title: `password.kdbx` for a
BIP-85 password, `password-2.kdbx` and on beside another waiting.

Two things about the text. A KDBX document is XML, so the five markup
characters are escaped and a control character XML has no representation
for is dropped. And the inner
random stream is KeePass's process-memory protection, not part of the
file's security, which is the passphrase, Argon2id, ChaCha20 and the
HMACs.

## Reading one by hand

```
python3 tools/backup/read-kdbx.py \
    tools/vectors/backup/backup-kdbx-words-12-english.kdbx \
    --passphrase 'correct horse battery staple'
```

The script opens the file with `pykeepass` where it is installed, and
otherwise with a reader of its own over `cryptography`; it says on
standard error which one ran. Any KeePass app opens the same file.
