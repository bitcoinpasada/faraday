# AQRV vectors

Test vectors for the Argon QR Vault (AQRV) (`docs/AQRV.md` §11): one
small secret under a passphrase, in 53 or 62 bytes. No implementation of
the format exists yet; these vectors are for the first ones, and for a
reviewer who writes a reader from the document alone.

| File | What it holds |
|---|---|
| `aqrv.json` | every case, each with its inputs, every intermediate value and the expected outcome |
| `gen.py` | writes the other files; `--check` compares without writing |
| `check.py` | a second reader and writer, written from `docs/AQRV.md` §2 to §10 alone, run over every case |
| `qr-53.png`, `qr-62.png` | the QR code of the 24-word step 0 case at each size (version 3 level L, 29 × 29; version 4 level M, 33 × 33), 8 pixels a module, 4 modules of quiet zone, 1-bit greyscale, with the three chunks a PNG needs and no metadata |

## What computed them

Libraries the format's authors did not write:

- Argon2id: argon2-cffi (25.1.0), over the reference implementation.
- HMAC-SHA-256, SHA-256 and SHAKE-256: Python's `hmac` and `hashlib`.
- HKDF-Expand: written in `gen.py` over `hmac` (one block, RFC 5869 §2.3),
  and compared in every derivation with PyCA cryptography's `HKDFExpand`.
- ChaCha20: PyCA cryptography (RFC 8439, nonce of twelve zero bytes,
  counter 0).
- The QR code: Project Nayuki's `qrcodegen.py`, vendored in
  `tools/reference/qrcodegen/`, one byte-mode segment, no ECI, the version
  and level fixed, the mask chosen by the library.
- The codex32 checksum: BIP 93's `ms32_polymod` and `ms32_create_regular_checksum`,
  copied from the BIP's text in `tools/vectors/bip93/`.

Everything is fixed by the script: passphrases are in it, a salt is
the first 16 bytes of SHA-256 of `ACE vector salt ` and the case's name, and
entropy, keys and padding are SHAKE-256 of a label and the case's name. A second run
writes the same bytes. `gen.py` also reads every case back with a plain reading of
§6 and §9 and stops if an answer differs from the one it writes; that reader
checks the vectors and is not meant to be used.

## A case

`aqrv.json` has `parameters` (the steps, labels, sizes, kinds and the three
outcomes), `cases`, `writer_refuses` and `qr`. Every case has `name`, `group`,
`size` (53 or 62), `passphrase`, `envelope` (hex) and `expect`. A case that
opens also has:

- `step`, `memory_kib`, `header`, `salt`, `kind` and `kind_name`;
- `item_bytes`: the item as laid out (§3), unused low bits zero;
- `padding`: the bytes after the item, from the writer's random source, given
  as input; `last_byte_fill` the bits that fill the item's unused low bits
  (codex32 only). `P` is the item bytes with that fill, then the padding;
- `values`: `pw` (UTF-8 of the NFC form), `PRK`, `K_enc`, `K_mac`, `keystream`
  (the first room bytes), `P`, `C`, `hmac_message` (`H || N || salt || kind || C`),
  `tags` (all nine kinds, keyed by kind) and `tag` (the one stored);
- `expect.item`: what a reader shows. `type` is `entropy`, `raw_key`, `passphrase`,
  `note` or `codex32`, with `hex`, `text` or `string` (the exact string, in its
  case) and, for codex32, `is_secret`.

`expect.outcome` is one of:

- `opens`: recognised, one kind's tag matches, the item reads as `expect.item`;
- `does_not_open`: recognised, a passphrase is asked for, and the code does not
  open. `expect.why` and `expect.reason` say how;
- `not_recognised`: not treated as AQRV (§9), so no passphrase is asked for.

## Groups

| Group | Cases |
|---|---|
| `step` | a 24-word seed at each of the five steps, at both sizes. Steps 1 to 4 run Argon2id at 128 MiB to 1 GiB; every other case uses 64 MiB |
| `kind` | all five lengths of entropy, the raw key, and a passphrase and a note of the shortest length, the longest (31 bytes of text at 53, 36 at 62) and non-ASCII, at both sizes; a note of multi-byte characters that ends exactly at the room |
| `seed` | the all-zero and all-one seeds (12 words at both sizes, 24 words at both): the padding and `kind` are all that differ in what is hidden |
| `unicode` | a non-ASCII passphrase given in NFC and in NFD: the same envelope and result; `fine` against `ﬁne` (U+FB01), which is a different passphrase |
| `codex32` | the codex32 strings of BIP 93 vectors 1, 2, 3, 4, 6, 7 and 8 (secrets, shares, derived shares, and the alternate encodings of vectors 3 and 4), each returning the exact string in its own case. A string of 21 to 29 item bytes is sealed at 53; one of 33 or 37 only at 62; the first string of each vector that fits 53 is also sealed at 62. Vector 5 (a 64-byte seed) and an invalid checksum are in `writer_refuses` |
| `fail` | a wrong passphrase; one bit changed in the header's step (does not open), in its marker (not recognised), in the salt, in `C` and in the tag; steps 5 to 7; marker `0b11110` (first byte `0xF0`); lengths 52, 54, 61 and 63; a 62-byte envelope cut to its first 53 bytes; a tag made with the other size's `N`, on each size |
| `fail-right-tag` | cases whose tag is right, made with the key over a `P` the format refuses: a `len` of 0 and of the room (one past `room - 1`), for a passphrase and a note; text that is not UTF-8; a codex32 `size` of 5, 6 and 7, and of 3 and 4 at 53 bytes; a threshold field of 1; threshold `0` with the index `a`. The reader finds exactly one kind's tag matching and then refuses the item |
| `not-recognised` | KEF version 20 envelopes, built by `gen.py` from `docs/KEF.md` §3 and §4: 53 bytes with an empty id and 62 bytes with a 9-byte id. Each is real: PBKDF2-HMAC-SHA-256 at 100,000 iterations, AES-256-GCM, a 12-byte IV, the tag cut to 4 bytes. The case lists the passphrase and plaintext |

## Things to know

- A threshold of `1`: `1` is the codex32 separator and not a data character, so there
  is no character to write. The case sets the 5-bit threshold field to the value 1 (the
  character `p`), which §3.1 refuses as neither `0` nor `2` to `9`.
- The reader cases with a changed step (the header's low bit) run Argon2id at 128 MiB.
- A KEF envelope from Krux's own tests is not here: `tools/reference/krux/` holds
  only `bbqr.py` and `qr.py`, with no test file, and no code was fetched for this. The
  two envelopes above stand for it.
- A pair of kinds whose tags agree is not offered (`docs/AQRV.md` §11): `gen.py` stops
  if a case has one, and every salt in the file avoids it.
- The QR pictures were decoded by zxing-cpp and by zbar (the decoders `tools/crosscheck`
  uses), and both return the envelope's bytes. `check.py` also reads the PNG itself and
  compares its modules with the matrix in `aqrv.json`, and the three chunks.

## The second reader

`check.py` was written from the document, with the generator unread, and uses other
libraries: PyCA cryptography's Argon2id; HKDF-Expand and HMAC-SHA-256 over `hmac` and
`hashlib`; ChaCha20 written out from RFC 8439 and checked on its §2.4.2 vector first. It
takes an envelope and a passphrase and answers not recognised, refused (with the rule) or
opened (with the fields), and it seals the inputs of every case that opens and must make the
same envelope. It compares every intermediate value the file gives, the outcome, the item and,
for a case whose tag is right, the kind and the rule that refuses it. It is a second reader
by Faraday's authors, so it is not the outside implementation `docs/AQRV.md` §11 asks for.

## Running

    python3 -I -B tools/vectors/aqrv/gen.py            # writes the files
    python3 -I -B tools/vectors/aqrv/gen.py --check    # compares, exit 1 on a difference
    python3 -I -B tools/vectors/aqrv/check.py          # the second reader; --no-qr skips the decoders

`gen.py` needs `argon2-cffi` and `cryptography`; `check.py` needs `cryptography` (a version with Argon2id), and for the pictures `zxingcpp`, Pillow and `zbarimg`, each skipped with a note if missing. The whole run takes about 40 seconds and up
to 1 GiB of memory (the one Argon2id run at the 1 GiB step at a time).
