# KEF: Krux's encrypted code, read and written in full

**Kind:** Spec
**Status:** Specified; nothing built. No decision number yet.
**Open:** All of it: the decision, the crate, the classifier's arm, the worker job, the screens, the cross-check area, the fuzz target, the dependency pages. Questions a source could not settle are in §13.

KEF (Krux Encryption Format) is an envelope for encrypted bytes: a label,
a version, a PBKDF2 count and an AES payload. It is one project's
published specification, with that project's code as its reference. This
document says what Faraday does with it, beside the QR vault
(`docs/VAULT.md` §2.1). All values were read from the sources in §1.

## 1. Sources

selfcustody/krux, tag `v26.09.0`, commit
`2b20e3370169731a0d89839a4113ad9a1b732078` (the latest release on
2026-11-04), MIT:

- `src/krux/kef.py` (SHA-256 `deb09c47bf20998696f11ed9b9fcc95bcb856b2def8428190c1044c1cf119a59`)
- docs/getting-started/features/encryption/kef-specifications.en.md, "the specification" below
- `tests/test_kef.py`, `tests/test_encryption.py`, `tests/shared_mocks.py`
- `src/krux/pages/encryption_ui.py`, `mnemonic_loader.py`, `wallet_settings.py`, `home_pages/wallet_descriptor.py`, `datum_tool.py`, `src/krux/baseconv.py`, `krux_settings.py`

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
convention other wallets use, since one project uses it. It enters by a decision naming that.

The specification asks that a claim of support be clear and precise.
Faraday's, once built:

> Reads KEF versions 0, 1, 5, 6, 7, 10, 11, 12, 15, 16, 20 and 21. Writes
> versions 5, 6, 7, 10, 11, 12, 15, 16, 20 and 21. As in Krux v26.09.0.
> Reads an envelope as raw bytes or as hex, base32, base43 or base64 text.
> Decompressed contents up to 100 KiB; PBKDF2 counts up to 100,000,000.

Not claimed: Krux's `seeds.json` store (§13), a passphrase that is itself
a KEF code, and anything of a later Krux release until it is read here.

Versions 0 and 1 are read and not written: the specification marks them
legacy, "consider using" 5 and 10. **Krux still can write them.** Its
automatic choice (`suggest_versions`) never picks them, but the Datum
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

Each refusal is Krux's, with `fail_unsafe` on. A refusal says which other
version would work.

- **ECB:** the padded bytes (the check and padding included where they are
  encrypted) must have no two equal 16-byte blocks.
- **NUL padding, exposed check (5, 10):** the bytes before padding must not
  end in 0x00.
- **NUL padding, hidden check (0, 1):** never written here.
- **An IV is 12 or 16 fresh bytes of the session's** (`fresh_bytes`, F154,
  with Add randomness offered as for a QR vault), never reused, never
  supplied. ECB takes none.
- **Empty plaintext** is refused (Krux raises on it in the NUL-padded
  versions and has no use for it elsewhere).
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
  `min(bytes removed, |auth| + 1) + 1` tries in all, as `_authenticate`
  does. For a hidden check, each try moves a byte from the check to the
  plaintext and adds a NUL to the check. This opens codes from a writer
  that skipped §4.1. A plaintext opened on any try after the first gets a
  note after it opens, as the specification suggests.
- **Compression** is undone after the check passes, with the limit of §7.

Duplicate ECB blocks are not refused on reading.

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
contents go as raw bytes in the numbered or BBQr parts Faraday already
writes.

Sizes at medium error correction, by the capacity tables, with an 8-byte id:

| Contents | Version | Bytes | Base43 characters | Code |
|---|---|---|---|---|
| 12 words | 20 | 45 | 66 | 33 × 33 |
| 24 words | 20 | 61 | 90 | 33 × 33 (version 4's whole capacity) |
| 24 words, 16-byte id | 20 | 69 | 102 | 37 × 37 |
| 12 words | 5 | 32 | 47 | 29 × 29 |
| 24 words | 5 | 48 | 70 | 33 × 33 |
| 24 words | 10 | 65 | 95 | 37 × 37 |

Binary at the same sizes gives the same codes: 61 bytes also fits version 4.
The base43 form is what Krux's mnemonic path writes, and what a person can
transcribe. A QR vault is 77 × 77 for every contents.

## 6. What goes in and what comes out

**What Krux puts in:**

| Krux's path | Plaintext | Read back by |
|---|---|---|
| Encrypt mnemonic (`bip39.mnemonic_to_bytes`) | the entropy, 16 or 32 bytes | `mnemonic_from_bytes` from storage; from a code, the loader below |
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

Unlike the QR vault, there is no padding and no TOML: Krux must be able to
read it. **So a code shows its contents' length**, and whether it is a
12- or a 24-word seed (§12).

**What Faraday reads.** A plaintext is untrusted, whatever passed its check
(§7.3). It takes the road a scanned code of that form takes, through
`classify` and the existing intake (`docs/SCANNING.md` §4), each with its
own checks. Faraday adds only:

- **16 or 32 bytes** are entropy: they go to the seed sheet (`docs/FLOWS.md`)
  with the fingerprint line of §7.3, as a CompactSeedQR's do.
- **Text that names nothing** is a note, shown as a secret (DESIGN §4.10).
  It may be a passphrase, and nothing in the bytes says so: the card offers
  "Use as a passphrase".
- **Bytes that are not text** are shown as hex and go no further.
- **A plaintext that parses as KEF** is offered as one, to a depth of 8.

Nothing decrypted is written to disk unless the person then saves it.
Faraday Starter does not scan or print a KEF code.

## 7. What comes from a scanned code

An envelope is attacker's data until its check passes, and only then is
it still only probably the person's.

### 7.1 Size

Contents decompress to at most **100 KiB (102,400 bytes)**, Krux's own
limit (`DEFLATEIO_MAX_DECOMPRESSED_SIZE`), so whatever Krux can open is
opened here; through `miniz_oxide`'s
`decompress_to_vec_with_limit`, as BBQr does. Reading takes any window up
to 32 KiB, which the specification allows.

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
  run. Over 550,000, the most Krux's own screen accepts, the card says so
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
with **Open** and **This is not encrypted**, which sends it on as an unknown
code. The cost of a false match is one extra press, which is why the arm may
be this loose: it must accept whatever Krux wrote.

An id is shown as text when it is UTF-8 with no control characters, otherwise
as `0x` hex, as Krux shows it.

## 9. Passphrase bytes

The specification: `K` is bytes, UTF-8 and unnormalised, with a minimal set of
characters advised on writing and several normalisations allowed on reading.

**Writing: printable ASCII only** (0x20 to 0x7E). Krux's keypads offer letters,
digits and symbols (§13), ASCII is the same in every normal form, and the rule can be stated in a
line. Other text is refused at the field. This is stricter than the vault's
NFC (`docs/VAULT-HARDENING.md`), because a vault opens in Faraday and a KEF
code must open on a device with a small keypad. The dice passphrase comes
first, as for a vault.

**Reading:** the field's text as typed, in UTF-8. If it holds anything outside
ASCII, then also NFC and NFD where they differ: at most three runs, each a full
PBKDF2, said on the card before the press. One level down, **Enter as hex**
takes the bytes themselves, since a Krux key scanned from a code can be any
bytes. A passphrase scanned from a code is not offered on the first build (§13).

The passphrase is not stored; it is given to the job by move, as Unlock's is.

## 10. Defaults

- **Version 20.** Krux's default mode too; the only family with a keyed check
  over the ciphertext (§12).
- **Id: 8 random bytes**, from the session's `fresh_bytes`. 64 bits keeps a
  24-word code at 33 × 33. NIST SP 800-132 asks for at least 128 random bits of
  salt; 16 bytes gives that at 37 × 37, one level down. A salt here needs only to
  differ between codes under one passphrase, which 64 bits does.
- **Not the fingerprint**, Krux's default: it would put the wallet's fingerprint,
  which also appears in descriptors and PSBTs, on a printed code; would tell its
  holder which wallet it opens; and would give every code of a seed under one
  passphrase and count the same key. A label typed one level down is public and
  printed, and the form says so; a typed label gets Krux's small random change
  to the count (within 10%), so a reused label does not mean a reused key.
- **Iterations: 100,000.** Krux's default (`pbkdf2_iterations`, range 10,000 to
  500,000), so every Krux opens it in the time its owner expects. One level down:
  10,000 to 500,000. The form says what §12 says: the passphrase matters far more.
  How long 500,000 takes on Krux's slowest board is not known here (§13).

Compression is not automatic: Krux's choice of 21 at 120 bytes and above is
offered, not taken, since `miniz_oxide`'s dictionary is 32 KiB, and the
specification is strict about a 1,024-byte window on writing. Below 1,024 bytes
of input no match can reach further, which is why §4.1 stops there.

## 11. Implementation

**Established first (F178):**

| Need | Source | State |
|---|---|---|
| AES-256 block | `aes` 0.9 | in the tree, trusted |
| ECB | a loop over `aes`'s block calls | a few lines; the `ecb` crate adds nothing |
| CBC | `cbc` 0.2 | in the tree; read only today, encrypt comes with it. Its page changes (`docs/deps/cbc.md` says nothing here encrypts) |
| CTR | `ctr` 0.10 (`Ctr32BE`) | **new**, RustCrypto/block-modes |
| GCM | `aes-gcm` 0.11, `AesGcm<Aes256, U12, U4>` | **new**, RustCrypto/AEADs, brings `ghash` 0.6 and `polyval` 0.7, also new |
| PBKDF2 | §11.1 | |
| SHA-256 | `sha2` through `faraday-crypto` | in the tree |
| Deflate | `miniz_oxide` | in the tree; compressing is new to it here (BBQr and KDBX only inflate) |

**A 4-byte GCM tag does not need a piece of our own.** `aes-gcm` 0.11.1's
`TagSize` is implemented for U12 to U16 always and for **U4 and U8 under its
`hazmat` feature** (source, `src/lib.rs`). That feature is the one Faraday would
enable; the dependency page says why. `aead`, `ctutils` and `universal-hash` are
already in the tree or trusted. `aes-gcm`, `ctr`, `ghash` and `polyval` need pages
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
  (`docs/KEYS.md`). `Reveal` is a closed list: the implementation uses the variant
  the QR vault's seal uses if that one's meaning covers this, otherwise adds a
  variant in `faraday-keys` with its doc, in the same commit.
- **Opening a seed:** decrypted entropy goes in as a scanned CompactSeedQR's does,
  wiped on its way, and is key material from there. What is in `faraday-kef`'s
  buffers is wiped as `faraday-pgp`'s are (`zeroize`).
- **Costs:** `Work::Kdf`, and the job in WORKERS's list. No drawing path reaches it.

The manifest gains an action (`docs/crypto/actions.toml`), and with it the STANDARDS
row, the crypto Learn block and the `written_here` entry, in the commit that adds
the code, whose `independence` is others' code (Krux's).

## 12. Trade-offs

Accurate enough to be the website's explainer, and to be read as a warning.

**What KEF is for.** A short code, opened by a Krux device or by Faraday. The
QR vault's strengths are the opposite ones: it hides what it holds, takes more
memory to attack and is a public standard, at 77 × 77.

**Each version.**

| Versions | For | Against |
|---|---|---|
| 20, 21 (GCM) | Everything; Krux's default | 4-byte tag |
| 15, 16 (CTR) | Devices without GCM | The check is an unkeyed hash inside the ciphertext. Someone who knows the plaintext can change the code to say anything else and fix the check, without the passphrase |
| 10, 11, 12 (CBC) | The same | Larger: a 16-byte IV |
| 5, 6, 7 (ECB) | The smallest code | No IV: below |
| 0, 1 | Reading only | Replaced by 5 and 10 |

**ECB's leak.** Under one key, equal 16-byte blocks of plaintext give equal blocks
of ciphertext, and the same plaintext gives the same code each time. A refusal of
duplicates covers one code's own blocks. It does not cover two codes: under one
passphrase, id and count, the same seed gives the same code, which says the two
hold the same thing. Random entropy has no repeats to find within a code; text
can have similar blocks that stop short of being equal. Faraday's random id gives
each code its own key, which removes the case between codes, and ECB still is not
the default. Its use is a smaller code: 29 × 29 for 12 words.

**Truncated checks.** 3 or 4 bytes, against 16 in the QR vault. They say the
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

In guesses, taking a round 10^10 HMAC-SHA-256 per second for one attacker's graphics
card (an assumption for scale, not a measurement, §13), at 100,000: 100,000 guesses
a second.

| Passphrase | To try them all, on one such card |
|---|---|
| 6 dice words, 77.5 bits | about 68 billion years |
| 4 dice words, 51.7 bits | about 1,160 years; 1,000 cards, about a year |
| 8 random lower-case letters, 37.6 bits | about 24 days |
| One dice word and 4 digits, 26.2 bits | about 13 minutes |

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

## 13. Open questions

- **A code from a real Krux device.** All vectors here come from Krux's tests, which run
  over PyCryptodome and not the K210's engine. That the engine's first 4 tag bytes equal
  standard GCM's follows from Krux's claim of compatibility and is not shown here. A code
  from a device, in each mode, goes into the vectors before release.
- **How long a count takes on Krux's slowest board**, for the default (§10), and on a Pi 3B+
  for §7.2's estimate.
- **The guess rate in §12** is for scale. A measurement from a named source replaces it
  before the website uses it.
- **`seeds.json`**, Krux's store on flash and SD card (`{id: {"b64_kef": …}}`, and an older
  form with `version`, `key_iterations` and `data`). It is not KEF's specification. Not read
  on the first build; a person can open the envelope only as a file of its own bytes.
- **A passphrase scanned from a code**, which Krux allows, itself possibly KEF. Not on the
  first build. A person holding one can read it off and type it, unless it is not text;
  **Enter as hex** covers that.
- **`miniz_oxide`'s dictionary:** §10's bound assumes it offers no 1,024-byte window; the
  cross-check confirms each compressed output inflates under `wbits` 10.
- **Krux's keypads** being ASCII only, which §9's rule rests on: `pages/keypads.py` and its
  character sets were not read.
- **Whether Krux's numbered or BBQr parts carry a KEF** from its Datum tool as Faraday's
  reassembly expects. `view_qr` was not read.

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
  Each of the twelve versions (the ten for writing) over: both sizes of entropy, short and long
  text, NUL-ended text, repeated blocks, ids of 0, 8 and 252 bytes, counts at each edge, the
  written 0-and-1 codes from Krux to Faraday only. Refusals agree where Krux's do, and §8's
  stricter ones are listed as facts.
- **Written compressed codes** inflate under `zlib` at `wbits` -10.
- **A fuzz target**, faraday/fuzz/fuzz_targets/kef.rs, on the parse and on decryption with a
  fixed key and a count of 1 through an internal entry: never panics, never gives more than the
  limit, seeds from the vectors.
- **Refusal cases:** `i` = 0, `len_id` over 252, an unknown version, a bad length, an inflation
  over 102,400 bytes, an empty ciphertext.
- **Recognition:** every other kind's vectors are not taken as KEF; the 32-byte case of §8.
- **Cancel:** a job at 100,000,000 stops at once, and the worker takes the next.

## 15. Screens, in outline

Strings are not written here. Every string goes to `en.rs`; a screen review names its Material page.

- **Making one.** In the backup flow beside **QR vault**: **Krux code (KEF)**. The first step
  has what to hold and the passphrase, dice first. Written at version 20, 8-byte random id,
  100,000. **Options**, one level down, hold the version (all ten, grouped by mode, ECB's
  carrying its one line), the count, the id (16 bytes, or a typed label) and nothing else.
  A refusal of §4.1 names the version that works.
- **The code.** A labelled PNG and a file to the Outbox, as a QR vault's; the label never
  says what it holds. Not a secret under `docs/QR.md` §7.1.
- **Scanning one.** §8's card: version, id, count, time here. **Open** asks for the passphrase;
  **Enter as hex** is one level down. A bar with Cancel. Then §6's road, the fingerprint on
  the seed sheet.
- **Learn** explains §12; a working screen says what it is and no more. Practice gets a sample.
