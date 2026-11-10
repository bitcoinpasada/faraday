# KEF: Krux's encrypted code, read and written in full

**Kind:** Reference
**Status:** Built (F219). Reading and writing KEF in full: all twelve versions, scanning a code, making one from a seed or in Tools, the backup checklist item, the stick files and the code on Paper, checked against Krux's own code and not on a Krux device.

KEF (Krux Encryption Format) is an envelope for encrypted bytes: a label,
a version, a PBKDF2 count and an AES payload. It is one project's
published specification, with that project's code as its reference. This
document says what Faraday does with it. All values were read from the
sources in §1.

## 1. Sources

selfcustody/krux, tag `v26.09.0`, commit
`2b20e3370169731a0d89839a4113ad9a1b732078` (the latest release on
2026-11-04), MIT:

- `src/krux/kef.py` (SHA-256 `deb09c47bf20998696f11ed9b9fcc95bcb856b2def8428190c1044c1cf119a59`)
- docs/getting-started/features/encryption/kef-specifications.en.md, "the specification" below
- `tests/test_kef.py`, `tests/test_encryption.py`, `tests/shared_mocks.py`
- `src/krux/pages/encryption_ui.py`, `mnemonic_loader.py`, `wallet_settings.py`, `home_pages/wallet_descriptor.py`, `datum_tool.py`, `src/krux/pages/__init__.py`, `src/krux/encryption.py`, `src/krux/baseconv.py`, `krux_settings.py`

And the firmware Krux pins: selfcustody/MaixPy
`ec79e131f5a1977bdcddaa24f6cdfc3497ae2496`,
`components/micropython/port/src/moducryptolib_maix.c`; selfcustody/micropython
`b11923f020e0caf2572e5874957f29128da2bb45`, `extmod/moddeflate.c`.

Where the specification's prose and the code differ, this document
follows the code, which the specification calls the reference: the prose
under v7 says `SHA256(plaintext)`, while the code and the specification's
own formula hash the compressed bytes.

## 2. The claim

Against `docs/STANDARDS.md` §0: its cryptography is standard and not Faraday's own, and it
can be checked against code Faraday did not write, but it is neither a BIP, SLIP or RFC nor a
convention other wallets use, since one project uses it. It enters by the decision naming that (F219).

The specification asks that a claim of support be clear and precise.
Faraday's:

> Reads and writes KEF versions 0, 1, 5, 6, 7, 10, 11, 12, 15, 16, 20 and
> 21, as in Krux v26.09.0; 0 and 1 are labelled legacy. Reads an envelope as
> raw bytes or as hex, base32, base43 or base64 text. Decompressed contents
> up to 100 KiB; PBKDF2 counts up to 100,000,000. Checked against Krux's
> code, and not on a Krux device. Left out: Krux's `seeds.json` store, and a
> passphrase that is itself a KEF code.

Nor is anything of a later Krux release claimed until it is read here.

Left out, and what a person does instead:

- **`seeds.json`** is Krux's store on flash and SD card (`{id: {"b64_kef": …}}`,
  and an older form with `version`, `key_iterations` and `data`). It is not in
  KEF's specification. A person can open an envelope from it only as a file of
  its own bytes.
- **A passphrase that is itself a code**, which Krux can scan, and which may
  itself be KEF. A person holding one can read it off and type it, unless it is
  not text; **Enter as hex** (§9) covers that.

Versions 0 and 1 are written one level further down, labelled legacy. The
specification marks them legacy, "consider using" 5 and 10, but **Krux still
can write them.** Its automatic choice (`suggest_versions`) never picks them, but the Datum
tool's version menu (`input_version_ui`) lists every entry of `VERSIONS`,
0 and 1 included, and `Cipher.encrypt` accepts them. The documentation's
"Legacy (<= v25.03.0)" describes the default, not what the firmware can do.

## 3. The envelope

```text
len_id (1) | id (len_id) | v (1) | i (3, big-endian) | cpl
k = PBKDF2-HMAC-SHA-256(password = K, salt = id, count, 32 bytes)
count = i * 10,000 if i <= 10,000, else i
```

`id` is 0 to 252 bytes. `i` = 0 is refused. The count therefore runs from
10,000 to 100,000,000, by multiples of 10,000 up to there and every value
from 10,001 to 16,777,215. A count that is a multiple of 10,000 is written
as its quotient, as `kef.wrap` does; `i` is 3 bytes either way, so that
form saves nothing, whatever Krux's settings page says.

## 4. The versions

| v | Name | Mode | IV | Padding | Compressed | Payload | Check |
|---|---|---|---|---|---|---|---|
| 0 | AES-ECB v1 | ECB | none | NUL | no | `E(P ‖ a ‖ pad)` | `a = SHA-256(P)[:16]`, hidden |
| 1 | AES-CBC v1 | CBC | 16 | NUL | no | `iv ‖ E(P ‖ a ‖ pad)` | `a = SHA-256(P)[:16]`, hidden |
| 5 | AES-ECB | ECB | none | NUL | no | `E(P ‖ pad) ‖ a` | `a = SHA-256(v ‖ P ‖ k)[:3]`, exposed |
| 6 | AES-ECB +p | ECB | none | PKCS#7 | no | `E(P ‖ a ‖ pad)` | `a = SHA-256(P)[:4]`, hidden |
| 7 | AES-ECB +c | ECB | none | PKCS#7 | yes | `E(Z ‖ a ‖ pad)` | `a = SHA-256(Z)[:4]`, hidden |
| 10 | AES-CBC | CBC | 16 | NUL | no | `iv ‖ E(P ‖ pad) ‖ a` | `a = SHA-256(v ‖ iv ‖ P ‖ k)[:4]`, exposed |
| 11 | AES-CBC +p | CBC | 16 | PKCS#7 | no | `iv ‖ E(P ‖ a ‖ pad)` | `a = SHA-256(P)[:4]`, hidden |
| 12 | AES-CBC +c | CBC | 16 | PKCS#7 | yes | `iv ‖ E(Z ‖ a ‖ pad)` | `a = SHA-256(Z)[:4]`, hidden |
| 15 | AES-CTR | CTR | 12 | none | no | `iv ‖ E(P ‖ a)` | `a = SHA-256(P)[:4]`, hidden |
| 16 | AES-CTR +c | CTR | 12 | none | yes | `iv ‖ E(Z ‖ a)` | `a = SHA-256(Z)[:4]`, hidden |
| 20 | AES-GCM | GCM | 12 | none | no | `iv ‖ E(P) ‖ a` | GCM tag `[:4]`, exposed, no associated data |
| 21 | AES-GCM +c | GCM | 12 | none | yes | `iv ‖ E(Z) ‖ a` | GCM tag `[:4]`, exposed, no associated data |

`Z` is raw deflate of `P` with a 1,024-byte window (`wbits` 10). AES is
always AES-256. CTR's counter block is the 12-byte IV and a 4-byte
big-endian counter from 0 (the firmware's `initial_value` default, and
what the CTR vector in §11 decrypts under). GCM is the standard
construction with a 96-bit IV.

The unused byte values refuse: a code with another `v` is not KEF.

### 4.1 Writing: strict

The ECB and NUL refusals are Krux's, with `fail_unsafe` on. A refusal says
which other version would work.

- **ECB:** the padded bytes (the check and padding included where they are
  encrypted) must have no two equal 16-byte blocks.
- **NUL padding, exposed check (5, 10):** the bytes before padding must not
  end in 0x00.
- **NUL padding, hidden check (0, 1):** the check's last byte must not be
  0x00, which refuses about 1 plaintext in 256. The plaintext itself may end in
  0x00, since the check follows it.
- **An IV is 12 or 16 fresh bytes of the session's** (`fresh_bytes`, F154,
  with Add randomness offered before the code is made), never reused, never
  supplied. ECB takes none.
- **Empty plaintext** is refused (Krux raises on it in versions 5 and 10
  and has no use for it elsewhere).
- **Compression:** a compressed version is written only for a plaintext of
  at most 1,024 bytes (§10).

### 4.2 Reading: tolerant, and vague

A payload is refused before any work unless, after its IV and exposed
check, ciphertext remains in whole blocks for ECB and CBC and at least one
byte for CTR and GCM, plus a hidden check where there is one. Krux's bound
is looser and lets an empty ciphertext reach its check, which then fails or
returns nothing; the result for a person is the same.

After decrypting:

- **PKCS#7:** the last byte `n` is taken as the padding's length without
  looking at the other padding bytes, as `kef._unpad` does; `n` = 0 or more
  than the length fails. The check decides, and every failure reads the same.
- **NUL padding:** trailing NULs are removed and the check tried; if it
  fails, NULs are put back one at a time and it is tried again, up to
  `min(bytes removed, |auth| + 1) + 1` tries in all (`removed.min(17) + 1` for a
  16-byte check). For versions 5 and 10 Krux's count is the same (checked at 1 to 8 NULs:
  version 5 opens with 4 put back and fails with 5, version 10 opens with 5 and fails
  with 6). For versions 0 and 1 Krux counts the bytes removed after taking the hidden
  check off the end as well, so it makes 17 or 18 tries where Faraday makes
  removed + 1: a hand-made code that needs a try past Faraday's last (a check whose
  16 bytes have all been moved back) opens on Krux and is refused here. No real
  code reaches it, as the check would be 16 zero bytes. For a hidden check, each try moves a byte from the check to the
  plaintext and adds a NUL to the check. This opens codes from a writer
  that skipped §4.1. A plaintext opened on any try after the first gets a
  note after it opens, as the specification suggests.
- **Compression** is undone after the check passes, with the limit of §7.

Duplicate ECB blocks are not refused on reading. A suggestion can still be refused when the code is
written, for repeated blocks in ECB, as on Krux; the refusal names the version that works (§4.1).

## 5. On a QR code

KEF stops at bytes; the encoding for transport is each writer's. Krux:

- A **mnemonic** is shown as **base43 text** (`B43CHARS`,
  `0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ$*+-./:`, Electrum's), which fits
  QR's alphanumeric mode.
- A **descriptor** is shown as **raw bytes** (`SeedQRView(binary=True)`).
- `decrypt_kef` takes raw bytes, or text it hints as hex (either case),
  base32, base43 or base64, in that order, the first that parses winning.

Faraday writes base43 in alphanumeric mode, at medium error correction,
for everything that fits one code, and reads all five forms. A text form
must re-encode to the same text, to narrow what matches (§8). Larger
contents are refused, with the file saved (§15.4); numbered or BBQr parts
of a KEF are read and not written.

**Low error correction** is offered for version 5, one level down, since a
smaller code is the reason to pick that version: a 24-word code then reaches
29 × 29. Low restores about 7% of a code's words where medium restores about
15%, so it tolerates about half the damage. A code on paper meant to last is
safer at medium.

Sizes at medium error correction unless a row says low, by the capacity
tables, with an 8-byte id:

| Contents | Version | Bytes | Base43 characters | Code |
|---|---|---|---|---|
| 12 words | 20 | 45 | 66 | 33 × 33 |
| 24 words | 20 | 61 | 90 | 33 × 33 (version 4's whole capacity) |
| 24 words, 16-byte id | 20 | 69 | 102 | 37 × 37 |
| 12 words | 5 | 32 | 47 | 29 × 29 |
| 24 words | 5 | 48 | 70 | 33 × 33 |
| 12 words | 5, at low | 32 | 47 | 25 × 25 (version 2's whole capacity) |
| 24 words | 5, at low | 48 | 70 | 29 × 29 |
| 24 words | 10 | 65 | 95 | 37 × 37 |
| 12 words | 0, legacy | 45 | 66 | 33 × 33 |
| 24 words | 0, legacy | 61 | 90 | 33 × 33 |
| 12 words | 1, legacy | 61 | 90 | 33 × 33 |
| 24 words | 1, legacy | 77 | 113 | 37 × 37 |

Binary at the same sizes gives the same codes: 61 bytes also fits version 4.
The base43 form is what Krux's mnemonic path writes, and what a person can
transcribe. A QR vault (`docs/VAULT.md` §2.1) is 77 × 77 for every contents.

## 6. What goes in and what comes out

**What Krux puts in:**

| Krux's path | Plaintext | Read back by |
|---|---|---|
| Encrypt mnemonic (`bip39.mnemonic_to_bytes`) | the entropy, 16 or 32 bytes (read as entropy unless it is text, below) | `mnemonic_from_bytes` from storage; from a code, the loader below |
| Wallet descriptor (`descriptor.to_string().encode()`) | descriptor text | wallet loader, decoded as UTF-8 |
| Passphrase | UTF-8 text | `wallet_settings`, decoded as UTF-8 |
| Datum tool | any bytes under about 50 KB | shown as text, or as `0x` hex |

Krux's loader, given decrypted bytes from a code, takes 12 or 24 words
separated by spaces, 16 or 32 bytes as entropy, or 48 or 96 digits as a
SeedQR. A KEF envelope inside a KEF envelope is opened in turn.

**What Faraday writes:**

| Faraday holds | Plaintext | Default |
|---|---|---|
| A 12- or 24-word seed | its entropy, as Krux | 20 |
| A seed of 15, 18 or 21 words, or another list's | refused: Krux's code loader takes 12 or 24 of the English list; the file vault takes it | |
| A BIP-39 passphrase, a codex32 share, a note | its UTF-8 text, nothing around it | 20 |
| A wallet's policy or descriptor | its text | 20; 21 is offered |

There is no padding and nothing around the plaintext: Krux must be able to
read it. **So a code shows its contents' length**, and whether it is a
12- or a 24-word seed (§12).

**What Faraday reads.** A plaintext is untrusted, whatever passed its check
(§7.3). It takes the road a scanned code of that form takes, through
`classify` and the existing intake (`docs/SCANNING.md` §4), each with its
own checks. Faraday adds only:

- **16 or 32 bytes that are not text** are entropy: they go to the seed sheet (`docs/FLOWS.md`)
  with the fingerprint line of §7.3, as a CompactSeedQR's do. Text here is valid UTF-8 holding
  no control character other than tab, newline and carriage return; that length of text
  goes on as a note. About one real 16-byte seed in 10,650 is also valid UTF-8; nearly all of those hold a control character and go to the seed sheet. The few that do not would show as a garbled note, so a
  note of exactly 16 or 32 bytes also offers **Read as a seed**, which sends those same
  bytes down this branch. A note of another length never offers it.
- **Text that names nothing** is a note, shown as a secret (DESIGN §4.10).
  It may be a passphrase, and nothing in the bytes says so: the card offers
  "Use as a passphrase".
- **Bytes that are not text** are shown as hex and go no further.
- **A plaintext that parses as KEF** is offered as one, to a depth of 8.

Nothing decrypted is written to disk unless the person then saves it.
Faraday Starter does not scan or print a KEF code. It prints only a blank template to
copy one onto, which holds nothing of any code (§15.5).

## 7. What comes from a scanned code

An envelope is attacker's data until its check passes, and only then is
it still only probably the person's.

### 7.1 Size

Contents decompress to at most **100 KiB (102,400 bytes)**, Krux's own
limit (`DEFLATEIO_MAX_DECOMPRESSED_SIZE`), so whatever Krux can open is
opened here; through `miniz_oxide`'s
`decompress_to_vec_with_limit`, as BBQr does. Reading takes any window up
to 32 KiB, which the specification allows.

A scanned text in base43 of more than **8,192 characters** is not taken for a
code and is not decoded: base43 is worked out digit by digit, its time grows
with the square of the text, and an envelope is far shorter (a 24-word code in
base43 is 90 to 113 characters, §5). The check is on the length, before any decoding.

### 7.2 Iterations

The version, the id and the count are public, and are shown before the
passphrase is asked for. A hostile code can demand 100,000,000.

- Nothing runs before the press.
- The work is one job on the worker (F184, `docs/WORKERS.md`), its function
  taking `c: &Compute` with a "Costs" line (`docs/COMPUTE.md`).
- **The work really stops on Cancel.** WORKERS §4 otherwise lets a job run
  on with its answer dropped; a stale 100,000,000-count job would hold the
  one worker for minutes. The loop checks a cancel flag between steps and
  reports its count, so the bar is determinate.
- The screen shows the count and the estimated time here, from a short timed
  run on the worker (`docs/WORKERS.md` §2.1): the count shows alone until
  the time comes, and pressing **Open** never waits for it. Over 550,000, the most Krux itself writes (§10), the card says so
  and asks once more: a person who made this code on a Krux did not choose it.

### 7.3 A wrong passphrase that passes

The check is short, so a wrong passphrase passes about 1 time in 16,777,216
(version 5), 1 in 4,294,967,296 (the 4-byte versions) or practically never
(0, 1). Krux's tests hold such a pair (§11). What comes out is random bytes,
and **16 or 32 random bytes are a valid seed**.

So the check is never treated as proof. A seed from KEF comes to the sheet
with its **fingerprint** (without a BIP-39 passphrase, the form Krux's
default id holds), for the person to compare against what they wrote down.
Where the id is 8 hex characters, it is shown beside the fingerprint and
marked when they match; a difference is only a note, since an id may be
anything. A person is told to compare; nothing here can decide.

### 7.4 Messages

One failure message for every failure after the passphrase is entered:
wrong passphrase, a check that fails, padding, or a stream that will not
inflate. The specification asks for this, and it gives nothing away. A code
that does not parse is not called a failed KEF; it goes on as whatever else
it is (§8), or as an unknown code. Krux's growing delay after a failure is
not copied: anyone holding a code can guess offline, so a delay on this
device protects nothing.

## 8. Telling a KEF code from other codes

An envelope has no magic number; the specification says a parse only makes
one "likely". Parsing alone is weak: of the 65,536 pairs of `len_id` and `v`,
116 fit 32 random bytes (about 1 in 564) and 48 fit 16 (about 1 in 1,365).

So KEF is **the classifier's last arm** (`core/faraday-codec/src/classify.rs`,
`docs/SCANNING.md` §4.6), reached only when nothing stricter claims the code:

1. SeedQR (48 or 96 digits), codex32 (its checksum), the QR vault (its exact
   OpenPGP packets, `docs/VAULT.md` §1.2), PSBTs, descriptors, addresses,
   keys and the rest keep their places and all come first.
2. **Raw 16 or 32 bytes stay a CompactSeedQR.** A 32-byte code really can be
   both: version 5 with an 8-byte id and 12 words is exactly 32 bytes, in raw
   form, which Krux's mnemonic path does not write. When such a code also
   parses, the seed sheet carries **Open as an encrypted code**, one level
   down, and no question comes first.
3. Then KEF: `len_id` at most 252 (Krux's `unwrap` does not check this, its
   `wrap` never writes more), `v` known, `i` not 0, the payload's size per §4.2.
   Text forms in Krux's order (§5), each re-encoding to the same text.

A match never runs anything. It gives a card naming the version, id and count
with **Open** and **Not encrypted**, which sends it on as an unknown
code. The cost of a false match is one extra press, which is why the arm may
be this loose: it must accept whatever Krux wrote.

An id is shown as text when it is UTF-8 with no control characters, otherwise
as `0x` hex, as Krux shows it. The card and the passphrase sheet show its first
24 characters, then `…`.

## 9. Passphrase bytes

The specification: `K` is bytes, UTF-8 and unnormalised, with a minimal set of
characters advised on writing and several normalisations allowed on reading.

**Writing: printable ASCII only** (0x20 to 0x7E), 1 to 200 characters; an empty passphrase is refused. Krux's
key field offers exactly those 95 characters on its four keypads and takes at most
200 (`load_key`, `ENCRYPTION_KEY_MAX_LEN`), ASCII is the same in every normal form,
and the rule can be stated in a line. Other text, or a longer one, is refused at the field. This is stricter than the vault's
NFC (`docs/VAULT.md` §3), because a vault opens in Faraday and a KEF
code must open on a device with a small keypad. The dice passphrase comes
first.

**Reading:** the field's text as typed, in UTF-8. If it holds anything outside
ASCII, then also NFC and NFD where they differ: at most three runs, each a full
PBKDF2, said on the card before the press. One level down, **Enter as hex**
takes the bytes themselves, since a Krux key scanned from a code can be any
bytes. A passphrase scanned from a code is left out (§2).

The passphrase is not stored; it is given to the job by move, as Unlock's is.

## 10. Defaults

- **Version 20.** Krux's default mode too; the only family with a keyed check
  over the ciphertext (§12).
- **Id: 8 random bytes**, from the session's `fresh_bytes`. 64 bits keeps a
  24-word code at 33 × 33. NIST SP 800-132 asks for at least 128 random bits of
  salt; 16 bytes gives that at 37 × 37, one level down. A salt here needs only to
  differ between codes under one passphrase, which 64 bits does.
- **The fingerprint, Krux's default, is one level down** beside 16 bytes and a
  typed label, and is not the default: it would put the wallet's fingerprint,
  which also appears in descriptors and PSBTs, on a printed code; would tell its
  holder which wallet it opens; and would give every code of a seed under one
  passphrase and count the same key. A label or the fingerprint is public and
  printed, and the form says so. Either adds to the count a random change under
  10% of it, as Krux does, so a reused id does not mean a reused key.
- **Iterations: 100,000.** Krux's default, so every Krux opens it in the time its
  owner expects. Krux's setting (`pbkdf2_iterations`) runs from 10,000 to 500,000,
  and it adds a change under 10% of the setting to every new code, whatever its id
  (`KEFEnvelope.__init__`), so at most 549,999. A count typed for one code replaces
  that and is taken from 10,000 to 550,000 (`input_iterations_ui`; the field's title
  says 510K). So 550,000 is the most Krux itself writes, and the setting's range is
  what Faraday offers. One level down: 10,000 to 500,000, and with a label or the
  fingerprint the change on top. The form says what §12 says: the passphrase matters far more.
  How long 500,000 takes on Krux's slowest board is not known here (§13).

Compression is not automatic: Krux's choice of 21 at 120 bytes and above is
offered, not taken, since `miniz_oxide`'s dictionary is 32 KiB, and the
specification is strict about a 1,024-byte window on writing. Below 1,024 bytes
of input no match can reach further, which is why §4.1 stops there.

## 11. Implementation

**Established code used (F178):**

| Need | Source | State |
|---|---|---|
| AES-256 block | `aes` 0.9 | in the tree, trusted |
| ECB | a loop over `aes`'s block calls | a few lines; the `ecb` crate adds nothing |
| CBC | `cbc` 0.2 | in the tree, reading and encrypting |
| CTR | `ctr` 0.10 (`Ctr32BE`) | RustCrypto/block-modes |
| GCM | `aes-gcm` 0.11, `AesGcm<Aes256, U12, U4>` | RustCrypto/AEADs, with `ghash` 0.6 and `polyval` 0.7 |
| PBKDF2 | §11.1 | |
| SHA-256 | `sha2` through `faraday-crypto` | in the tree |
| Deflate | `miniz_oxide` | in the tree; KEF is the one place Faraday compresses with it (BBQr and KDBX only inflate) |

**A 4-byte GCM tag does not need a piece of our own.** `aes-gcm` 0.11.1's
`TagSize` is implemented for U12 to U16 always and for **U4 and U8 under its
`hazmat` feature** (source, `src/lib.rs`). That feature is the one Faraday
enables; the dependency page says why. `aead`, `ctutils` and `universal-hash` are
already in the tree or trusted. `aes-gcm`, `ctr`, `ghash` and `polyval` have pages
in `docs/deps/` and `cargo vet` trust entries by publisher (F71).

### 11.1 The piece written here

Reason: `only-part-used` (ESTABLISHED §1.1), over `aes`, `cbc`, `ctr`, `aes-gcm`,
`sha2`, `hmac` and `miniz_oxide`:

- The envelope, the version table, padding, the two SHA-256 checks and the
  tolerant reading of §4.2; base43.
- **PBKDF2's loop** over `hmac`, for one 32-byte block, with a cancel check and a
  count (§7.2). The `pbkdf2` crate is one call that cannot stop. Nothing in the
  closed list of ESTABLISHED §1.1 speaks of stopping, so `only-part-used` is the
  reason, with the note; ten lines, in `core/faraday-crypto/src/hash.rs` beside
  `pbkdf2_hmac_sha256`, checked against that function and RFC 7914's vectors.

### 11.2 Where it lives

- **Parsing, no cryptography:** `unwrap` and the text forms in
  `core/faraday-codec` (`kef.rs`), beside `classify`, which takes nothing costly.
- **The rest:** a new crate faraday/faraday-kef, as `faraday-pgp` is for the QR
  vault, depending on `faraday-crypto` and the crates above. Every function that
  derives a key takes `c: &Compute`, says "Costs a PBKDF2 of `count` rounds" and
  takes bytes, holding no key of its own (F70).
- **Sealing a seed:** the entropy comes through `Keys` by a named reveal
  (`docs/KEYS.md`). `Reveal` is a closed list: the reveal for this is `Reveal::KefCode`
  in `faraday-keys`, with its doc.
- **Opening a seed:** decrypted entropy goes in as a scanned CompactSeedQR's does,
  wiped on its way, and is key material from there. What is in `faraday-kef`'s
  buffers is wiped as `faraday-pgp`'s are (`zeroize`).
- **Costs:** `Work::Kdf`, and the job in WORKERS's list. No drawing path reaches it.

The manifest has the action (`docs/crypto/actions.toml`), with the STANDARDS row, the crypto
Learn block and the `written_here` entry; its `independence` is others' code (Krux's).

## 12. Trade-offs

Accurate enough to be the website's explainer, and to be read as a warning.

**What KEF is for.** A short code, opened by a Krux device or by Faraday. The
QR vault's strengths (`docs/VAULT.md` §2.1) are the opposite ones: it hides what it holds, takes more
memory to attack and is a public standard, at 77 × 77.

**Each version.**

| Versions | For | Against |
|---|---|---|
| 20, 21 (GCM) | Everything; Krux's default | 4-byte tag |
| 15, 16 (CTR) | Devices without GCM | The check is an unkeyed hash inside the ciphertext. Someone who knows the plaintext can change the code to say anything else and fix the check, without the passphrase |
| 10, 11, 12 (CBC) | The same | Larger: a 16-byte IV |
| 5, 6, 7 (ECB) | The smallest code | No IV: below |
| 0, 1 | Legacy, one level further down: what Krux's first releases wrote, for a Krux that may open nothing newer (§13) | Replaced by 5 and 10, and larger than them: a 16-byte check. About 1 plaintext in 256 is refused (§4.1). 0 also has ECB's leak |

**ECB's leak.** Under one key, equal 16-byte blocks of plaintext give equal blocks
of ciphertext, and the same plaintext gives the same code each time. A refusal of
duplicates covers one code's own blocks. It does not cover two codes: under one
passphrase, id and count, the same seed gives the same code, which says the two
hold the same thing. Random entropy has no repeats to find within a code; text
can have similar blocks that stop short of being equal. Faraday's random id gives
each code its own key, which removes the case between codes, and ECB still is not
the default. With a typed label or the fingerprint, only the count's random change
gives a code its own key, and two codes can draw the same one. Its use is
a smaller code: 29 × 29 for 12 words, and at low error correction 25 × 25, or 29 × 29
for 24 words (§5).

**Truncated checks.** 3 or 4 bytes, against 16 in the QR vault and in versions 0 and 1. They say the
passphrase was probably right; they do not prove the bytes are what was written.
A wrong passphrase passes 1 time in 16,777,216 or 4,294,967,296 (§7.3), and a
change made to a GCM code passes 1 time in 4,294,967,296 for each time the person enters a passphrase. The specification
sees their purpose the same way. It also says a false pass yields "no value", which
is true of the secret and not of the seed sheet, hence the fingerprint.

**PBKDF2 is not memory-hard.** A guess costs `count` runs of HMAC-SHA-256, and SHA-256
is what specialised hardware is best at; nothing about it needs memory. As work,
100,000 adds ≈16.6 bits to a passphrase, 500,000 ≈18.9, 100,000,000 ≈26.6. The QR
vault's Argon2id adds ≈17.6 to ≈21.6 by the same count (`docs/VAULT.md` §3.1) and
also needs its memory for every guess, which this count does not show. OWASP's
Password Storage Cheat Sheet asks for 600,000 for PBKDF2-HMAC-SHA-256, more than
Krux's setting allows.

In guesses, from a public hashcat benchmark
(https://gist.github.com/Chick3nman/32e662a5bb63bc4f51b847bb422222fd): 8,865.7 kH/s
for mode 10900, PBKDF2-HMAC-SHA256 at 999 iterations, on one graphics card. At 100,000
that is about 88,000 guesses a second.

| Passphrase | To try them all, on one such card |
|---|---|
| 6 dice words, 77.5 bits | about 76 billion years |
| 4 dice words, 51.7 bits | about 1,300 years; 1,000 cards, about 16 months |
| 8 random lower-case letters, 37.6 bits | about 27 days |
| One dice word and 4 digits, 26.2 bits | about 15 minutes |

**So the passphrase is the whole of the protection.** The specification: "KEF offers
no expectation of security for a weak user-supplied `key`." A code is meant to be
printed and kept, and can be copied unnoticed and attacked for ever.

**The length shows.** 12 or 24 words can be told from the code's size, and a note's
length from its own. A QR vault shows neither.

**No outside review is known.** The specification invites scrutiny; none was found
in its text, Krux's repository or its release notes. It uses standard primitives in a
format of one project's.

**One project uses it.** Krux wrote it for itself and asks others to adopt it; no other
implementation was found. Leaving Krux means carrying Faraday's reader, or the
specification and any AES library.

**Krux's own firmware bug, v26.09.0.** The GCM tag buffer in its firmware was smaller
than the 16 bytes the SDK wrote into it, a buffer overflow, in `moducryptolib_maix.c`. The release
fixes the buffer, "maintaining compatibility with existing KEF envelopes": envelopes do
not change, and tags were always 4 bytes. It concerns the device's memory, not a code,
and Faraday does not share that code. The firmware's comment adds that only the first 4
of the engine's 16 tag bytes "have ever been exercised in the field".

## 13. What the sources did not settle

- **A code from a real Krux device.** All vectors here come from Krux's tests, which run
  over PyCryptodome and not the K210's engine. That the engine's first 4 tag bytes equal
  standard GCM's follows from Krux's claim of compatibility and is not shown here. No device
  is at hand, so KEF is offered as checked against Krux's code and not on a device, and the
  claim says so (§2). A device's code joins the vectors if one ever turns up.
- **How long a count takes on Krux's slowest board**, for the default (§10), and on a Pi 3B+
  for §7.2's estimate.
- **What Krux's older releases open.** Whether a release up to v25.03.0, which the
  specification's "Legacy" names, opens anything but versions 0 and 1 was not read.
- **`miniz_oxide`'s dictionary:** §10's bound assumes it offers no 1,024-byte window; the
  cross-check confirms each compressed output inflates under `wbits` 10.
- **Whether Krux's numbered or BBQr parts carry a KEF** from its Datum tool as Faraday's
  reassembly expects. `view_qr` was not read, which is why Faraday writes no such parts (§15.4).

## 14. Tests

Behaviour a person or Krux would notice, and vectors:

- **Krux's vectors, kept as data** in tools/vectors/kef/, MIT, with their source.
  `tests/test_encryption.py`'s `*_ENCRYPTED_QR` constants and `test_kef.py`'s CTR, GCM and
  grind cases. Checked here by a script of Faraday's over `hashlib` and pyca/cryptography:
  `CTR_ENCRYPTED_KEF` with `key`, 10,000, gives `CTR_ENTROPY`; `GCM_ENCRYPTED_KEF` with
  `test key`, 100,000, gives `TEST_WORDS_ENTROPY`; the grind envelope with `abc` gives
  `I'm 16 raw bytes`. `test_kef.py`'s ECB and CBC constants did not authenticate under the
  keys tried and end in the same three bytes: they appear to be for parsing only.
- **The wrong passphrase that passes:** the same grind envelope with `421a8f` passes version
  5's check and gives `27602bca…0d70`. Faraday must open it, as garbage, and show its
  fingerprint.
- **Both ways against Krux's code**, a new area `kef` in the cross-check
  (`tools/crosscheck/`, F79): `src/krux/kef.py` vendored unchanged in `tools/reference/krux/`,
  with shims as for `qr.py`: `ucryptolib` and `uhashlib_hw` over `hashlib` and an AES library,
  `deflate` at `wbits` -10. **`kef.py` runs this way:** it imports only those three, and it ran
  under such shims, all twelve versions making and opening their own payloads. `test_kef.py`
  does not run this way: it needs Krux's `conftest.py`, its mocks, `pytest`, `embit` and
  PyCryptodome. Its data is taken and not its code.
  Each of the twelve versions, each way, over: both sizes of entropy, short and long
  text, NUL-ended text, text whose hidden check ends in 0x00, repeated blocks, ids of 0, 8 and
  252 bytes, counts at each edge. Refusals agree where Krux's do, and §8's
  stricter ones are listed as facts.
- **Written compressed codes** inflate under `zlib` at `wbits` -10.
- **A fuzz target**, faraday/fuzz/fuzz_targets/kef.rs, on the parse and on decryption with a
  fixed key and a count of 1 through an internal entry: never panics, never gives more than the
  limit, seeds from the vectors.
- **Refusal cases:** `i` = 0, `len_id` over 252, an unknown version, a bad length, an inflation
  over 102,400 bytes, an empty ciphertext.
- **Recognition:** every other kind's vectors are not taken as KEF; the 32-byte case of §8.
- **Cancel:** a job at 100,000,000 stops at once, and the worker takes the next.
- **A code copied by hand:** a code of each side, drawn on its blank template's grid, reads back
  as that code and opens to the seed under its passphrase; a different code is named as not this
  one, and nothing scanned there is loaded or filed.
- **The Starter's template:** each size makes a PDF under its name, with the fixed squares of a
  code of that side and no code on it.

## 15. Screens, in outline

Strings are not written here. Every string goes to `en.rs`; a screen review names its Material page.

- **Making one.** A seed: §15.1, a form on the seed on the backup board's Customize page,
  one level down, adding no answer to the question on vaults. Anything else §6 writes: §15.2,
  a tool in Tools. Both have the first step of §15.3, at §10's defaults. A code too large for
  one QR, or to copy by hand: §15.4.
- **The code.** A labelled PNG and a file to the Outbox; the label never
  says what it holds. Not a secret under `docs/QR.md` §7.1. A seed's code goes on a stick or on
  Paper, where it is copied by hand onto a blank template (§15.5).
- **Scanning one.** §8's card: version, id, count, time here. **Open** asks for the passphrase;
  **Enter as hex** is one level down. A bar with Cancel. Then §6's road, the fingerprint on
  the seed sheet. The card follows SCANNING §5's; the sheet is a Material 3 dialog (Dialogs, with
  a text field from Text fields) as the other sheets are.
- **Learn** explains §12; a working screen says what it is and no more. Practice gets a sample.

### 15.1 The form on a seed

**On the board.** Customize backup, and only Back up's board: a wallet flow's own plan has no
such form, since its keys are made after it (as codex32 strings, `docs/BACKUP-FLOW.md` §17.23).
A seed's chip in the list has a ⋮ with **Add to…** (§17.20); under its boxes, past a divider,
stands **Krux code…**. Pressed, the same menu lists **Krux code to…**: under each place's name its
Paper and each stick's files, and **New stick** where + stick would take it. Never a vault. Decided
(owner, 2026-11-05): a code goes on a stick or on Paper. On Paper means copied by hand onto a blank
template (§15.5), which people will often prefer even though the code is encrypted. A person may
also print the picture from a stick, on any computer, since it is not a secret; that is the
stick's form, and the plan does not know of it. A seed §6 refuses has the row dimmed, with `kef_not_offered` under it, worked out
when the menu opens (a lookup in `faraday-keys`, no words parsed). Material 3: Menus, a submenu;
on the small panel the sheet's list changes in place.

**Its box.** A chip of its own in a stick's files or a place's Paper:
`[ ◉ 9a6a2580 Krux code × │ ⋮ ]`, on the read-only board "9a6a2580 · Krux code", tagged sealed. It
has no form toggles, so a person reads one more form of the seed, and nothing about it can turn
into words: Move to… and Copy to… list only places' Paper and sticks' files, and Take off takes it
off. A later seed form toggled on the same stick or Paper is its own chip beside it, with its own
hold. Nothing is held for it at the drop, since no plain file is made. After the lock it takes no
changes, as any file or paper (§15.8 of the backup flow).

**In the plan.** Two `plan::What`s, tag Sealed, as every form has one for its zone:
`What::KefCode(i)` at `At::Stick` and `What::KefCodePaper(i)` at `At::Place`. One chip kind,
`board::Thing::Kef(i)`, which the list does not show; the box it lands in says which `What`. A `What` because the map already
gives what a form needs: place and stick renumbering, Move and Copy, the plan's text, the after-lock
refusal. A table beside the map would redo each by hand. It costs far fewer of the sites SeedQR
has, since it joins no list of copies:

- They have arms in `plan/mod.rs` (`is_file`, `seed_index`, the shape check, their labels, and
  for the paper form the Templates item's condition and its count),
  `plan/text.rs` (codes `kef{i}` and `kefp{i}`), `board/mod.rs` (`forms_in`, `things_of`, the name, the menu),
  `board/lines.rs`, `glance/mod.rs` (its line), `backups/mod.rs` (a file, not a copy),
  `backup_flow/wallet.rs` and `bstep.rs`.
- They have none in `seed_form`, `seed_spots`, the place's reach, `is_secret_file` or `secrets.rs`,
  and none in `scan_key.rs` beyond the scan of a drawn copy below, which reads no `What`.

**What it counts for.** Decided (owner, 2026-11-05): a Krux code does not count as a backup, on a
stick or on Paper. It is an extra copy: `kept_nowhere`, `alone`, `lost_why`, how
near a place brings a spend, and the balance do not read it. The plan cannot hold its passphrase
or say where it is, so a seed kept only as a Krux code still reads "Seed {} is kept nowhere" and
holds The plan card (`docs/BACKUP-FLOW.md` §17.5), a stick holding only codes may carry the
public files, and a place holding only a drawn code keeps nothing of the seed.

**The checklist step.** `Item::KefCode(i)`, one per seed with a code anywhere on the map, however
many sticks and places. In order: after the copies on paper, the vaults and `Item::SeedFiles`, before the
sheets, the public files and the envelopes, so before **Lock and write**, while the key is
loaded. `bstep.rs` numbers it from a range taken from the top of the passphrases' unused one. Its
title is "Krux code of {seed}", its status **Not made**, **Not checked** or **Checked**. A step in
the item's own card, not a sub-flow: the first step (§15.3), then the bar, then the code with
**Check my code**. It leaves nothing behind when closed but a code already made.

**Checked**, with no copy on Paper, means the code opens to this seed under a passphrase typed again: the field that made
it was wiped by the job, **Check my code** opens one field and **Compare**, a `Faraday::kef_open`
on the code's bytes with the bar and Cancel, and the plaintext is compared inside `faraday-keys`
with the seed's entropy (a named call under `Reveal::KefCode`, giving a yes or a no). A wrong one
reads so and may be tried again; **Make another** goes back to the first step. A code that is
checked ends the item. Its stick's read-back is then §17.12 item 10's: the picture decoded and
checked against the SHA-256 of its base43 text, and the file's bytes.

**On Paper.** The road of a SeedQR on its copy item (F29, `docs/BACKUP-FLOW.md` §17.24), with the
passphrase added to the check. Where the map holds `KefCodePaper(i)`, the Templates item names the
sheet (§15.5) and the Code card shows, in place of the picture, these labels and `copy_grid`
(`screens/parts.rs`), **Blocks** ticked · **Rows**, its blocks lettered as on the Krux sheet, rows A to E
and columns 1 to 5 at 25 × 25:

| Code | **Size** reads | **By hand** reads | Under them |
|---|---|---|---|
| 25 × 25, 29 × 29, 33 × 33 or 37 × 37 | its side | **Sheet**: Faraday Starter, Blank templates, Krux code, its side | the grid, **Scan my copy**, **Make another** |
| Larger, one QR | its side | **No**, and **Largest**: 37 × 37 | the picture, **Make another** |

- **Scan my copy** opens the camera on the drawn copy (`ScanPurpose::CheckKef`), reading it as the
  code's base43 text. Anything else reads "Not this code" on the scanner. Nothing scanned there is
  loaded or filed. The copy that reads closes the camera and opens one field and **Compare**, as
  **Check my code** does, but on the bytes scanned. It opens to the seed: **Checked**, for the code
  and every stick's copy of it, since they are the same bytes. A wrong passphrase reads so and may
  be tried again. As a SeedQR's, the scan is one per seed, not one per place.
- **Not on Paper**, a ghost button on the item, takes the seed's `KefCodePaper(i)` off every place
  and closes the grid, as **Keep the words only** does for a SeedQR, and the foot offers it after
  three scans without a match. No copy stays marked drawn in part, since a code is not a secret.
  With no copy left on a stick, the item goes too.
- **With no camera** there is nothing to type back: a drawn code has no words. The card offers
  **Check my code** on the code's own bytes and a label **Drawn copy**: **Not scanned**. That ends
  the item, since a code counts for nothing and must not hold the lock. No **Scan my copy** shows.
- **In Practice** the scanner offers the code made as its sample (`samples/put.rs`).

Closed before it is checked, the item keeps the code: open again, it reads it from its `.kef` file
under the names the backup record holds, with no passphrase asked, and shows the grid at the mark
it had, so a long copy can be left and taken up.

**What is written.** At the item, `Faraday::kef_save_made` puts `krux-v{version}-{id}.png` and
`.kef` in the Outbox, under the names the Outbox gives them; no fingerprint is in either. Each
stick holding `KefCode(i)` lists those two files, and `wallet_slot_files` takes them with the
stick's other files at **Lock and write**. A plan with the code on Paper alone still has them in
the Outbox, and no stick is asked to take them. A seed's code at §10's defaults fits one QR and
each size of §15.5 (§5). The code is made once and copied to each stick and each Paper, never made
per stick or per place.

**Saved.** The plan's text holds `kef{i}` in the stick's line, `kefp{i}` in the place's, and nothing more: no version, count,
id or options, and never the passphrase. The wallet's backup record keeps, per seed, the two file
names and the status, whether a drawn copy was scanned among them, as it keeps the other marks, so the lock and the unlock after it leave the
item done. A plan read back with its seed loaded and no such record has the item open, at the
defaults, and makes a new code beside the old one. A plan naming a seed §6 refuses reads without
that form, as a codex32 split that cannot be made does (§17.23).

### 15.2 The tool in Tools

A tile on the Tools page (`Screen::Catalog`, `catalog.rs`), **Krux code**, under Keys and seeds,
tags KEF, `Go::KefCode`. It is not a calculator. "Nothing here is a secret" is the module comment
of `faraday-core/src/tools/mod.rs`, and "None of them touches a key" the Calculators' line, each
about the six calculators on `Screen::Tools`, and both stay true and stay as they are: the Tools
page has held tiles for secrets since BIP-85 and Convert a seed. Like Convert, it has its own folder,
screen (`Screen::KefTool`, in `lib.rs`'s code map) and state, reached through `ToolsArea`, adding no
field to `Faraday`. Faraday Starter has none. It keeps nothing when left, so has no sidebar row;
what was typed is wiped, as New key's is (`docs/FLOWS.md`).

It is a step flow of three cards: **Contents**, the first step (§15.3), **Code** (the bar, the
code, **Check my code**, **Make another**). The picture and file go to the Outbox as it is made,
and a toast names the file.

| §6's contents | Where the text comes from | Offered |
|---|---|---|
| A passphrase, a codex32 share, a note | **Type it**: one field, masked behind its eye (DESIGN §4.10), UTF-8 up to the codec's limit | Yes; no kind is asked, the bytes are the text |
| A wallet's descriptor | **A loaded wallet**: its row; the checksummed descriptor, public, needing no reveal; Krux's wallet loader reads it | Yes |
| A held key's BIP-39 passphrase | A key's row; it would need a reveal of its own, since `Reveal::KefCode` lends no stored text | No |
| A vault's note or password | The item's action in Vault contents, under `Reveal::VaultEntry` | No |

The held sources are not offered because each would open a new way out of `faraday-keys`, and typing reaches the
same code without one. **Check my code** compares against the text the flow still holds. The
fingerprint as an id is offered only for **A loaded wallet**: typed text has none. Its Code card
shows the picture, and for a code that fits a blank template (side up to 37) **Size**, **By hand**
and a secondary **Copy by hand**: pressed, the picture is replaced by the same `copy_grid` as the
seed's item (§15.1), blocks lettered as on the Starter's Krux sheet, the mark at block A1 for each
new code, and the button reads **Show the code**. A code over 37 reads **By hand: No** and
**Largest: 37 × 37**, with no button. The view is `KefForm`'s (reset by **Make another**). The
tool has no **Scan my copy**, and its picture can be printed from a stick as any.

### 15.3 The first step

Shared by both. It reads, top to bottom:

- **Generate with dice**, the primary action, opening the entry Create a vault has
  (`docs/FLOWS.md`, Generate with dice), with its list, rolls, words, strength and, on Faraday Demo,
  the roller; **Use these words** fills the field. In Practice the public passphrase stands in, as
  in Create a vault (`docs/DEMO.md` §5.2).
- **Passphrase**, one masked field with its eye, under §9's rule, refused at the field. One field
  only, since **Check my code** is the second typing.
- **Options**, a secondary button with the line "Version 20 · 100,000 rounds · random id", opening
  a sheet: **Version**, option buttons under each mode's name, the ECB group with `kef_ecb_line`
  under it, **Legacy** one level further for 0 and 1; **Low error correction**, a switch, with
  version 5 only; **Rounds**, a number field from 10,000 to 500,000; **Id**, option buttons
  (Random 8 bytes, Random 16 bytes, A label with its field, The fingerprint), the last two with
  `kef_public_id_line`. Nothing else. On a seed whose code is also on Paper, the line ends in the
  code's side ("· 33 × 33") and the sheet has **Size** and **By hand** labels as in §15.1, so a
  person can pick a smaller code before making it. They are worked out when an option is pressed,
  by a lookup from the length §5's table comes from (`kef_make::side`), with no seal; a compressed
  version's length is known only once sealed, and its **Size** reads so.
- The Randomness row, since the press draws the IV and the id (DESIGN §4.1, F157), then **Make the
  code**. A refusal of §4.1 stands as a line above it and names the version that works.
- While it works, Material 3's determinate progress indicator from the rounds done, with **Cancel**.

**The pieces.** Three, each with its state in its own folder (`docs/STRUCTURE-PASS.md` §7):

- `faraday-core/src/dice_entry/` (`mod.rs`, `screen.rs`) is the shared dice entry: `DiceEntry`
  holds the rolls, the list and the roller, and takes a `dice_entry::Press` (a face, the list, Use,
  Close, the roller's act). It knows nothing of who opened it: the owner passes its focus, the time,
  the randomness for the roller and how a press is wrapped as an `Action`, and takes back from Use
  the phrase, its bits and the simulated marks. Out of `vault/dice.rs`, `vault/dice_screen.rs` and
  `vault/area.rs` come `dice_words`, `dice_word_indices`, `dice_aim`, `STRONG_BITS` and the foot
  panel (`docs/LAYOUT.md` §16.3). `app.vaults.state` keeps which passphrase it is for and fills
  `CreateForm`; Create a vault looks and behaves as before.
- `faraday-core/src/kef_form/` (`mod.rs`, `screen.rs`) is §15.3 and the code's card: `KefForm` holds
  the passphrase, its `DiceEntry`, `kef_make::Options`, the grid's mark and where it stands (entry,
  making, made, scanning, checking, checked), and is wiped when dropped. `BackupState` owns one while a Krux item is open,
  `ToolsArea` one for the tool; one job runs at a time, as the worker does already.
- `worker/kef_make.rs` stays the backend. No new file holds an `impl Faraday`
  (`tools/lint-layout.counts`): the glue goes in `compute.rs`, as `kef_make_seed` does.

Drawing reads: the picture is made once by `kef_make::made`, on the press that sees the job end.

### 15.4 A code too large for one QR

Refused, and the file saved. §5's numbered and BBQr parts are not written: whether Krux reads them
as a KEF is not known (§13), and a code that may open nowhere is what a refusal is for. The largest
code is 117 × 117 (`QrView::kef`, QR version 25), about 1,440 base43 characters, or about 980 bytes
of envelope. It can only happen in the tool, with a long note or a wallet of many keys. The Code
card then has no picture: `kef_code_too_big`, the file's name, **Check my code** and **Make another**.

**Too large to copy by hand** is a different thing, and not a refusal: 37 × 37 is the largest blank
template (§15.5), and a larger code that fits one QR is made as any, its picture and file saved. A
seed's code passes 37 × 37 only through Options: a label of 30 bytes or more on 24 words at
version 20, or version 1 with a 16-byte id. On a seed with a copy on Paper, the labels of §15.1 say
so, before it is made and on the Code card, and the item stays **Not checked** until **Make
another** gives a code that fits or **Not on Paper** takes the copy off.

### 15.5 The blank template for a code

Decided (owner, 2026-11-05): Faraday Starter prints blank templates for a code. It still does not
scan or print a code (F219): a blank holds nothing of any code, of any wallet or of the passphrase,
and may go to any printer. **Blank templates** (`docs/STARTER.md` §3.8) gets a third **Form**,
**Krux code**, whose rows are **Size** and **Places**. Each size is one sheet, named by its side:

| **Size** | QR version | What §5 says fits it | File |
|---|---|---|---|
| 25 × 25 | 2 | 12 words, version 5 at low | `faraday-blank-template-krux-code-25x25.pdf` |
| 29 × 29 | 3 | 24 words, version 5 at low; 12 words, version 5 | `faraday-blank-template-krux-code-29x29.pdf` |
| 33 × 33, chosen | 4 | 12 or 24 words at the default, version 20; 24 words, version 5; 12 or 24 words, version 0; 12 words, version 1 | `faraday-blank-template-krux-code-33x33.pdf` |
| 37 × 37 | 5 | 24 words, version 20 with a 16-byte id; 24 words, version 10 or version 1 | `faraday-blank-template-krux-code-37x37.pdf` |

Each option says on its line what the table says. The sizes stop at 37 × 37, which holds every row
of §5; a seed's code passes it only through the Options §15.4 names.

**One sheet per side, whatever the error correction.** A sheet prints only what every code of that
side shares, by the SeedQR sheet's rule (`structural_fixed`, which holds through QR version 6, one
alignment square): finders, separators, timing, alignment, in black. The format areas stay blank,
since they differ with the error correction and the mask, and the person draws them. So 29 × 29
serves medium and low alike.

**The sheet.** A sheet file `kind: krux`, `side:` and `places:`, drawn by `faraday-starter`'s
`sheets.rs`, which uses no `faraday-kef`. The grid is on a page of its own at true size, as the
Codex32QR's: squares of 16 points at 25 and 29, 14 at 33, 12.5 at 37, the heavier lines every five
and numbered as the SeedQR's, which make six blocks a side at 29, seven at 33 and eight at 37. Above
it, lines to fill in for what the picture's label says (version, rounds, id). Its line says the
passphrase is never written on it. It does not carry the other sheets' "Never photographed or scanned
by any other device" (`docs/CODEX32.md` §1.1 item 35), which is about forms that hold a secret; a
Krux code is made to be scanned by a Krux device. The places page and **Print**, **Save the PDF** are
as for the other forms.

**On the checklist.** The **Templates** item comes before the copies, and a plan keeps no options
(§15.1), so its clause, "{n} from Blank templates, Krux code, 33 × 33", counts one per place and
seed holding `KefCodePaper(i)`, at the side of the code if one is made and §10's default otherwise.
A person who then picks another size in Options sees it named on the Code card before copying, and
prints that sheet. A plan with only such copies reads only this clause.

**Faraday itself offers no such file.** As for every other blank template (`docs/BACKUP-FLOW.md`
§10.2), the checklist names the Starter's sheet and writes none to a stick, since the Starter runs
on the computer with the printer.
