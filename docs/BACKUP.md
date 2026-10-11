# `osk-backup`: the file this device encrypts things into

One container holds everything OpenSigner encrypts under a passphrase a
person types: a key's words, a master seed, a note, a wallet's recovery
sheet. One spec, one reference decryptor
(`tools/backup/decrypt.py`), one decrypt path to audit.

This file is the format. The implementation is
`core/osk-backup/src/oskb.rs`; where the two disagree,
this file is what a second implementation is written against, and the
vectors under `tools/vectors/backup/` settle it.

The history and the decisions behind it are `docs/PLANNING.md` §16.78
(version 1), §16.96 (version 2) and §16.112 (version 3).

The same things also leave the device in a second form, KDBX 4, for a
reader that is a KeePass app rather than OpenSigner. That form is
written and never read, and has its own section below.

## The file

Every integer is little-endian. Offsets are from the start of the file.

| At | Bytes | What |
|---|---|---|
| 0 | 4 | magic, `OSKB` (`4f 53 4b 42`) |
| 4 | 1 | version: 2 or 3 |
| 5 | 4 | Argon2id memory cost, KiB |
| 9 | 4 | Argon2id passes |
| 13 | 4 | Argon2id lanes |
| 17 | 32 | Argon2id salt |
| 49 | 24 | XChaCha20-Poly1305 nonce |
| 73 | *n* | ciphertext |
| 73 + *n* | 16 | Poly1305 tag |

Bytes 0 to 48 — the magic, the version, the three costs and the salt —
are the AEAD's associated data. A file whose header was changed fails
the tag rather than costing work under the wrong parameters.

The key is `Argon2id(passphrase, salt)` with a 32-byte output, version
1.3, at the cost the header states. The passphrase is its UTF-8 bytes,
unnormalised. The cipher is XChaCha20-Poly1305 with a detached tag.

## The plaintext

### Version 2

34 bytes, always a key's words:

| At | Bytes | What |
|---|---|---|
| 0 | 1 | word count: 12, 15, 18, 21 or 24 |
| 1 | 1 | wordlist index |
| 2 | 32 | entropy, zero-padded on the right |

The word count says how many of the 32 bytes are the key's. The pad must
be zero. A version-2 file is always 123 bytes.

### Version 3

A kind byte, the payload's length, the payload, and zero padding to the
next multiple of 256 bytes:

| At | Bytes | What |
|---|---|---|
| 0 | 1 | payload kind: 1, 2, 3 or 4 |
| 1 | 2 | payload length |
| 3 | *len* | payload |
| 3 + *len* | *pad* | zero, to the next multiple of 256 |

The padding must be zero, and the plaintext must be in the smallest step
the payload fits: a file padded further than that is not one this build
wrote. A file's length therefore says which 256-byte step its payload is
in and nothing finer.

The payload kinds:

**1, words.** The version-2 plaintext, unchanged in meaning: 34 bytes,
the word count, the wordlist index and 32 bytes of entropy zero-padded
on the right.

**2, a master seed.** 65 bytes: the seed's length (16 to 64), then 64
bytes of seed zero-padded on the right. This is what a SLIP-39 or a
codex32 key is — its master secret is its seed, and it has no words. The
seed alone does not say which of the two it came from, and does not have
to: either way it is a key with no words.

Kinds 1 and 2 are both one step, so a file that holds a key is 345 bytes
whichever door the key came through.

**3, a note.** The note's UTF-8 bytes, nothing else.

**4, a recovery sheet.** Three fields, each a `u16` length and that many
bytes, in this order:

| Field | What |
|---|---|
| 1 | the wallet's descriptor, with its checksum |
| 2 | the name the person gave the wallet, empty where it has none |
| 3 | the sheet's note, empty until one is written |

A payload must end exactly where its last field does.

A payload is at most 16 384 bytes, so the largest plaintext a reader
allocates for is 16 640.

### The wordlist index

The index a words payload carries is the wordlist's place in
`Language::ALL`:

| Index | Wordlist |
|---|---|
| 0 | English |
| 1 | Japanese |
| 2 | Korean |
| 3 | Spanish |
| 4 | Chinese (simplified) |
| 5 | Chinese (traditional) |
| 6 | French |
| 7 | Italian |
| 8 | Czech |
| 9 | Portuguese |

## The Argon2id cost

The header states the cost, so any device with the memory opens any
file. The device writes the memory the person chose in Settings ›
Backup memory — 64 MiB, 256 MiB or 1 GiB — with three passes and one
lane, which is what every Argon2id in this app uses. The kept-key blob
(`docs/PLANNING.md` §6) keeps its own 64 MiB: that cost is paid on every
unlock, not once per file.

A reader checks the stated cost before it allocates anything or runs a
single pass, because the header is only associated data: a changed cost
can do no more than fail the tag, and by then the memory has been taken
and the passes run. OpenSigner refuses a file above 4 GiB outright,
attempts the allocation below that, and says "Needs *n* MiB" where the
allocation fails. Passes must be 1 to 3 and lanes 1.

## Reading one by hand

```
python3 tools/backup/decrypt.py tools/vectors/backup/backup-v3-note.oskb \
    --passphrase 'correct horse battery staple'
```

The script needs the standard library plus either `argon2-cffi` and
`pynacl`, or `cryptography` 44 or newer. Nothing is fetched: the BIP-39
wordlists come from this repository and are checked against the SHA-256
recorded beside each one.

## KDBX 4, the other form

The same things leave the device as a KDBX 4 database for a person whose
reader is a KeePass app rather than OpenSigner (`docs/PLANNING.md`
§16.112 rule 3). It is written and never read: nothing on the device
opens a KDBX file.

The format is KeePass's own and is not restated here. It is specified at
<https://keepass.info/help/kb/kdbx.html>, with the KDBX 4 changes at
<https://keepass.info/help/kb/kdbx_4.html>; both pages are copied into
`tools/reference/kdbx4/` with the date they were fetched and their
SHA-256, and the implementation is
`core/osk-backup/src/kdbx.rs`.

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
| document | one group named `OpenSigner`, one entry per item |
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

The Argon2id cost is the same one `osk-backup` is written at, so the two
forms of one export cost a guess the same, and a KeePass app pays what
this device paid. Every file is offered as `opensigner.kdbx`, as an `osk-backup` file is offered one name, so a file says nothing about what it holds; the entry's title inside carries the fingerprint, the note's first line or the wallet's name.

Two things about the text. A KDBX document is XML, so the five markup
characters are escaped and a control character XML has no representation
for is dropped; an `osk-backup` file is the lossless form. And the inner
random stream is KeePass's process-memory protection, not part of the
file's security, which is the passphrase, Argon2id, ChaCha20 and the
HMACs.

## Reading a KDBX file by hand

```
python3 tools/backup/read-kdbx.py \
    tools/vectors/backup/backup-kdbx-words-12-english.kdbx \
    --passphrase 'correct horse battery staple'
```

The script opens the file with `pykeepass` where it is installed, and
otherwise with a reader of its own over `cryptography`; it says on
standard error which one ran. Any KeePass app opens the same file.

## What a file does not say

The name a saved file is offered under carries no fingerprint and no
date, and is the same string for every key (§16.96). Nothing outside the
ciphertext says whose key it is, which wallet it belongs to, or how long
the seed is. What a file holds is learned by opening it.
