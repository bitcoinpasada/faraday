# Argon QR Vault (AQRV)

**Kind:** Spec
**Status:** Draft for outside review. Own specification, not yet reviewed outside Faraday; the only code is the test vectors' two scripts, and nothing is compiled into any app (`docs/STANDARDS.md` §0)
**Open:** a KEF envelope from Krux's own tests among the vectors (§11), an outside second implementation, an outside cryptographic review of the construction and its 4-byte and 8-byte tags (§13), public comment, and whether Faraday ever offers it

The Argon QR Vault (AQRV) is a proposed open format for one small secret
encrypted under a passphrase with Argon2id, small enough for a 29 × 29 QR
code, and a 12-word seed for a 25 × 25 one. The name says it uses Argon2id and
is a vault in a QR code. It was first called the Argon2id Compact Envelope
(ACE), an earlier working name.

Krux's KEF (§14) does the same job with PBKDF2-HMAC-SHA-256 on
microcontrollers, whose few megabytes of memory leave no room for a
memory-hard function at the sizes recommended for one. PBKDF2 is a sound
choice there: it is NIST's approved password-based key derivation (SP
800-132) and makes each guess cost compute. Where a device has the memory,
standard practice is a memory-hard function, and Argon2id is the first
choice among them. RFC 9106 makes Argon2id the variant every implementation
must support and recommends it with 2 GiB, or with 64 MiB where memory is
constrained; OWASP's Password Storage Cheat Sheet names Argon2id first,
scrypt where it is not available, and PBKDF2 where FIPS-140 validation is
required; Argon2 won the Password Hashing Competition in 2015. AQRV brings
that practice to a code of the same size, from 64 MiB (RFC 9106's second
recommended setting) to 1 GiB, for computers and phones that have the
memory. It is meant as a next step for QR vaults on such devices, beside KEF
and not in its place.

It has three sizes in one format, and the total length says which:

- **32 bytes:** a 25 × 25 code at low error correction, an 11-byte salt, a
  4-byte tag and 16 bytes of item: a 12-word seed, or a passphrase or note of
  up to 15 bytes. The code holds these bytes as 47 characters of text (§3),
  so it is never read as a seed. 25 × 25 is the size of Krux's smallest
  encrypted 12-word code (KEF version 5, §14) and of a Compact SeedQR of 24
  words.
- **53 bytes:** a 29 × 29 code at low error correction, a 4-byte tag and 32
  bytes of item. 29 × 29 is the size of a standard 24-word SeedQR.
- **62 bytes:** a 33 × 33 code at medium error correction, an 8-byte tag and
  37 bytes of item.

The 53-byte size is called the small size below and the 62-byte size the
large size; the 32-byte size is always called that.

The item is one of a fixed list (§3): a BIP-39 seed's entropy, a codex32
string, a BIP-39 passphrase, a short note or a raw 32-byte key. Everything
needed to open it, apart from the passphrase, is in those bytes.

This document is written for someone who has never seen Faraday. Faraday
is where the draft started; the format carries no name of Faraday's and
depends on nothing of it. A statement the authors cannot support is
written as a question in §13, not as a claim.

## 1. Purpose, and what it does not do

**Purpose.** A person can print or hand-copy a secret's encrypted form and
keep it where a finder could see it. A finder holds nothing until they
guess the passphrase, and each guess costs memory as well as time. Krux's
KEF format (§14) does this in a similar space under PBKDF2, which needs
no memory. AQRV is the same idea under a memory-hard function.

**It does not:**

- make a weak passphrase safe. The code can be photographed and attacked
  offline for as long as the secret holds money (§12);
- hold anything outside the list in §3: no descriptor, no name or date, no
  second item. An item that does not fit the large size needs another
  format;
- prove with high assurance that an item is the one that was sealed. The tag
  is 4 or 8 bytes and is shared among nine kinds, three at the 32-byte size
  (§8, §12). A reader shows
  what each kind allows a person to compare, and for some kinds that is
  nothing (§7);
- open on devices without the memory. The smallest step needs 64 MiB
  (§5). Most microcontroller signing devices cannot read it;
- hide that a code may be an AQRV code. Its first byte is distinctive
  (§9), and the 32-byte size's text always begins with `A`. The total length
  also shows which size it is, though not which kind it holds;
- take a new kind of item later under the same marker. A reader older than a
  new kind would say "does not open", so a later list takes a new marker (§4);
- allow revocation. A printed code still opens after a new one is made.

## 2. Notation

`||` is concatenation; `X[:n]` the first n bytes of X; `⊕` exclusive or.
Bytes are written in hexadecimal with a `0x` prefix. All strings named
as labels are ASCII, with no terminator.

Published definitions used, none changed:

| Name | Definition |
|---|---|
| Argon2id | RFC 9106, version 0x13 |
| HMAC-SHA-256 | RFC 2104 over FIPS 180-4 |
| HKDF-Expand | RFC 5869 §2.3, with SHA-256 |
| ChaCha20 | RFC 8439 §2.4, 256-bit key, 96-bit nonce, 32-bit counter |
| NFC | Unicode Standard Annex #15 |
| UTF-8 | RFC 3629 |
| Entropy and words | BIP-39 |
| codex32 strings and their checksum | BIP 93, with the character values of BIP-173 |

## 3. The envelope

An envelope is exactly 32, 53 or 62 bytes, and that length is its size:

| Field | Contents | 32 bytes: offset, length | 53 bytes: offset, length | 62 bytes: offset, length |
|---|---|---|---|---|
| header `H` | marker in the high 5 bits, memory step in the low 3 (§4) | 0, 1 | 0, 1 | 0, 1 |
| `salt` | random, new for every envelope | 1, 11 | 1, 16 | 1, 16 |
| `C` | the padded item `P`, encrypted (§6) | 12, 16 | 17, 32 | 17, 37 |
| `T` | the tag (§6) | 28, 4 | 49, 4 | 54, 8 |

The header, marker, steps, passphrase handling and every other step are the
same at every size. The length binds three things: how much of the rest is
the salt, how much is `C` and how much is `T`. It enters the tag as a byte
(§6), so no size can be read as another.

**The 32-byte size's salt.** A 25 × 25 code at low error correction holds 32
bytes (the QR table below). With a 16-byte salt the envelope would be 1 + 16 +
16 + 4 = 37 bytes, which does not fit, so the salt takes what is left once the
header, a 12-word seed and the 4-byte tag are placed: 11 bytes, 88 bits. RFC
9106 §3.1 says "16 bytes is RECOMMENDED for password hashing", and §4
(Parameter Choice), step 7, says "A length of 128 bits is sufficient for all
applications but can be reduced to 64 bits in the case of space constraints."
This size is that case: the space is the code, and 88 bits is above the 64
that §4 allows. What the salt does and what 88 bits give up is §12.

Nothing else exists in the clear: no length, no version field apart from the marker,
no identifier, no fingerprint, no iteration count.

**Items.** `P` is always the whole of `C`'s length, called the room: 16 bytes
at the 32-byte size, 32 at the small size, 37 at the large one. An item starts
at `P`'s first byte.
Everything after it is padding: bytes from the writer's random source, read
once and never stored, and the unused low bits of an item's last byte are
filled the same way. A reader discards them unread.

`kind` is not stored. It enters the tag, and a reader finds it by testing
every value below (§6, §7). So an envelope does not show what it holds, and
padding gives a guesser no known plaintext to test a guess against. Zero
padding would give one: for a 12-word seed, 128 bits of known plaintext that
identify the right passphrase with certainty. The cost is that a wrong
passphrase has nine chances to match at the small and large sizes, and three
at the 32-byte size (§8).

| `kind` | Item | Layout | Length comes from | Item bytes | 32-byte size | Small size | Large size |
|---|---|---|---|---|---|---|---|
| 0x10 | BIP-39 entropy, 12 words | entropy | `kind` | 16 | yes | yes | yes |
| 0x14 | BIP-39 entropy, 15 words | entropy | `kind` | 20 | no | yes | yes |
| 0x18 | BIP-39 entropy, 18 words | entropy | `kind` | 24 | no | yes | yes |
| 0x1C | BIP-39 entropy, 21 words | entropy | `kind` | 28 | no | yes | yes |
| 0x20 | BIP-39 entropy, 24 words | entropy | `kind` | 32 | no | yes | yes |
| 0x30 | A codex32 string | §3.1 | a 3-bit size field | 21, 25, 29, 33 or 37 | no | up to 192 bits | all |
| 0x40 | A BIP-39 passphrase | `len` (1 byte), then `len` bytes of UTF-8 | `len`, 1 to room − 1 | 2 to 16, 2 to 32, or 2 to 37 | up to 15 bytes of text | up to 31 bytes of text | up to 36 bytes of text |
| 0x41 | A note | `len` (1 byte), then `len` bytes of UTF-8 | `len`, 1 to room − 1 | 2 to 16, 2 to 32, or 2 to 37 | up to 15 bytes of text | up to 31 bytes of text | up to 36 bytes of text |
| 0x50 | A raw 32-byte key | the key | `kind` | 32 | no | yes | yes |

**The kinds of a size.** The 32-byte size holds the three kinds whose item can
fit 16 bytes, 0x10, 0x40 and 0x41, and no other: every other kind's shortest
item is longer than its room. A reader and a writer at that size compute and
compare the tags of those three kinds only (§6). The small and large sizes hold
all nine.

The list is fixed for this marker (§4). A writer refuses an item that does
not fit the size it is writing. A writer asked for the 32-byte or the small
size with an item that does not fit refuses; it never moves to a larger size
by itself. It may always write a smaller item at a larger size, for a larger
salt at the small size or the longer tag at the large one.

A passphrase or a note is stored as the bytes given, in no particular
normal form: BIP-39 applies its own NFKD when a passphrase is used, and a
reader gives back what was sealed. A writer refuses empty text and text that
is not valid UTF-8.

The passphrase kind fits what a person types, not a long one: six words of
EFF's long list average about 47 bytes, over every limit. Those need the
OpenPGP form (§14). The 32-byte size's 15 bytes hold a short passphrase or a
PIN.

**A passphrase's link to its seed is left out.** The OpenPGP QR vault's `for`
(`docs/VAULT.md` §1.2) names the seed a passphrase goes with by its 4-byte
fingerprint, and AQRV could carry the same. The recommendation is to leave it
out, as a required field and as an optional one, for three reasons. A seed's
fingerprint is public wherever a descriptor or a PSBT naming its origin has
been shared, so those 4 bytes would be known plaintext, which is what the padding exists to
avoid. It takes 4 of the 15, 31 or 36 bytes of text. An optional field would also
need a flag, and it tells a reader nothing it can check: the fingerprint of a
seed does not confirm a passphrase. A person with several seeds writes on the
sheet which one a code goes with.

**Other items.** The raw key (0x50) is proposed because one kind covers a
Nostr private key, a BIP-340 key or a single Bitcoin private key, none of
which fits as text: a Nostr `nsec` string is 63 characters. Left out:

- a SLIP-39 share. The 20-word form (25 bytes) fits; the 33-word form (42
  bytes) does not fit even the large size, so the kind would serve half of
  what it names;
- a BIP-32 extended key (78 bytes), a descriptor, or a 64-byte seed: too long;
- a TOTP secret or a PIN: these are notes (a 20-byte TOTP secret in base32 is
  32 characters, so it needs the large size).

**The QR code.** The 53- and 62-byte envelopes are one byte-mode segment.
The 32-byte envelope is one alphanumeric-mode segment of exactly 47
characters, its text form below. Neither has an ECI designator.

| Size | Version | Modules | Level | Capacity | Error correction |
|---|---|---|---|---|---|
| 32 bytes | 2 | 25 × 25 | L | 47 alphanumeric characters | 10 codewords in one block of 44; corrects up to 4, as 2 of the 10 are kept for misdecode protection (ISO/IEC 18004's table of error correction characteristics) |
| 53 bytes | 3 | 29 × 29 | L | 53 bytes | 15 codewords in one block of 70; corrects up to 7 |
| 62 bytes | 4 | 33 × 33 | M | 62 bytes | 18 codewords in each of two blocks of 50; corrects up to 9 in each |

The capacities are ISO/IEC 18004's table of the number of symbol characters
and input data capacity (Table 7 in the 2015 edition). Version 2 at L has 34
data codewords, 272 bits; an alphanumeric segment of 47 characters takes the
4-bit mode, the 9-bit character count of versions 1 to 9, 23 pairs of 11 bits
and one last character of 6, which is 272 bits exactly, and 48 characters
would take 277. So 47 characters fill version 2-L as 32 bytes in byte mode
do.

A writer may draw an envelope larger than this for more tolerance, such as
the 53 bytes at 33 × 33 medium or the 32-byte size's 47 characters at 29 × 29
medium (version 3, whose alphanumeric capacity at M is 61). The payload, the
mode, the version and the level in the table are the format; the module matrix
is one encoder's output, whose mask is that library's choice (§11). A reader
takes the payload and does not look at the mode, the version or the level: the
payload's length alone says which size it is, so those 53 bytes still have a
4-byte tag. The longer tag comes only from writing 62 bytes.

**The 32-byte size as text.** In a QR code the 32-byte envelope is written as
text, never as 32 bytes of binary (why is §9):

- The 32 bytes are one unsigned number `V`, big-endian, so `0 ≤ V < 256^32`.
- `V` is written in base 45, most significant digit first, as exactly 47
  digits, left-padded with `0`. Digit `d` is the character at index `d` of
  QR's alphanumeric table, in that table's own order:
  `0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:` (the ten digits, the 26
  upper-case letters, then space, `$`, `%`, `*`, `+`, `-`, `.`, `/` and `:`,
  so `A` is 10, space 36 and `:` 44).
- 45^47 is more than 256^32, so 47 digits hold any 32 bytes, and each envelope
  has one text and each text below 256^32 one envelope. An envelope's first
  byte is at least 0xF8, so `V` is at least 0xF8 × 256^31, which is more than
  45^46: every envelope takes all 47 digits, and its first is always `A`
  (`V` lies between 10 × 45^46 and 11 × 45^46). The padding rule therefore
  never adds a `0` to an envelope; it says only that the width is fixed. In
  base 43 the same number would take 48 characters, one more than the code
  holds.
- A reader takes the text exactly as the scanner decodes it: 47 characters,
  each in the table, upper case only. A lower-case letter is not in the table
  and is not read as its capital. Nothing is trimmed: the last character is a
  space in about 1 in 45 codes. A text whose value is 256^32 or more is not 32
  bytes and is not recognised (§9).
- A `.aqrv` file holds the 32 raw bytes, not the text (§9.1).

### 3.1 A codex32 string

A codex32 string (BIP 93) is the two characters `ms`, the separator `1`, a
threshold, 4 characters of identifier, a share index, a payload and 13
characters of checksum. BIP 93 says the checksum characters "contain no
information", so they are not stored. The prefix never varies, so it is not
stored either. Every other character is stored as its 5-bit value, packed
most significant bit first after a 4-bit head:

```text
bits    3      1      5           20           5       5 × c
        size   case   threshold   identifier   index   payload
```

- `size` gives the seed's length and with it `c`, the payload's characters.
  5 to 7 are refused.
- `case` is 0 for a string in lower case and 1 for upper. BIP 93 requires one
  case throughout, the identifier and the index included, so one bit keeps the
  exact string, and it recommends upper case for handwriting and for QR codes,
  so both occur. The checksum's values do not depend on case.
- `threshold` holds the BIP-173 value of the character `0` or of a digit from
  `2` to `9` (`0` is 15, `2` is 10); any other value is refused. A threshold of `0` with an index other than `s` is refused,
  as BIP 93 requires.

| `size` | Seed | `c` | String's characters | Threshold to payload, in bits | With the head, in bits | Item bytes | Small size |
|---|---|---|---|---|---|---|---|
| 0 | 16 bytes, 128 bits | 26 | 48 | 160 (exactly 20 bytes) | 164 | 21 | yes |
| 1 | 20 bytes, 160 bits | 32 | 54 | 190 | 194 | 25 | yes |
| 2 | 24 bytes, 192 bits | 39 | 61 | 225 | 229 | 29 | yes |
| 3 | 28 bytes, 224 bits | 45 | 67 | 255 | 259 | 33 | no |
| 4 | 32 bytes, 256 bits | 52 | 74 | 290 (about 36 bytes) | 294 | 37, the whole room | no |

All the payload's bits are stored, including the 1 to 4 beyond the seed's
length in every form but the 160-bit one. BIP 93 pads with arbitrary
bits there, and they enter the checksum, so a string cannot be remade without
them: its test vectors 3 and 4 list 4 and 16 valid strings for one seed that
differ only in those bits. In a share they are also data, since a share's
characters are combined one by one to recover the secret.

BIP 93's 64-byte seed (103 characters of payload and a long checksum) does not
fit. All five forms here use its regular checksum. Its seed is a BIP-32 master
seed, not BIP-39 entropy, so this kind and the entropy kinds are not
interchangeable.

**Writing.** A writer takes a string with the prefix `ms`, one case, a valid
checksum, a valid threshold and one of the five lengths, and refuses any other.

**Reading.** A reader takes `size`, refuses it if the item does not fit the
room, checks `threshold`, and makes `ms1` followed by the characters and the
checksum BIP 93 gives for them (its `ms32_create_checksum` over
`bech32_hrp_expand("ms")`, which is `[3, 3, 0, 13, 19]`, followed by the data
characters), in the stored case. BIP 93's reference code folds that expansion
into its polymod's starting residue, 0x23181b3, which is the residue of
`[3, 3, 0, 13, 19]` from 1; the two are the same computation.
Since the checksum is computed here, it always passes and shows nothing about
the item (§7, §12).

An index of `s` is a codex32 secret and not a share. The kind holds it all the
same, and a reader says which it has.

All of this was worked through with a throwaway script against BIP 93's own
test vectors: every valid string of up to 256 bits in them, each in the case
it is given in, came back exactly. That is a check of the layout's arithmetic, not an
implementation (§11).

## 4. The header

```text
bit     7 6 5 4 3   2 1 0
        1 1 1 1 1   s s s     marker 0b11111, step sss
```

The marker is this version of the format, with this list of kinds and these
three sizes. A reader refuses a step it does not define. A later version takes
marker 0b11110 or another total length; it does not reuse this one. A reader
older than a kind would only find no tag that matches, and say "does not
open", which is not something a person could understand or act on. So a new
kind takes a new marker, and an older reader then says it does not know the
code. Bytes 0xF0 to 0xF4, which a marker of 0b11110 would use, can begin valid
UTF-8, so what §9 says of UTF-8 text would not hold for that marker as it
stands.

## 5. Memory steps

Argon2id takes, for every step:

| Parameter | Value |
|---|---|
| Passes `t` | 3 |
| Lanes `p` | 4 |
| Output length | 32 bytes |
| Version | 0x13 |
| Secret `K`, associated data `X` | none |
| Salt | the envelope's 16 bytes, or 11 at the 32-byte size |
| Memory `m` | by step, below |

| Header | Step | Memory | `m` (KiB) | 1 KiB block steps per guess (`m` × `t`) | log2 |
|---|---|---|---|---|---|
| 0xF8 | 0 | 64 MiB | 65,536 | 196,608 | ≈17.6 |
| 0xF9 | 1 | 128 MiB | 131,072 | 393,216 | ≈18.6 |
| 0xFA | 2 | 256 MiB | 262,144 | 786,432 | ≈19.6 |
| 0xFB | 3 | 512 MiB | 524,288 | 1,572,864 | ≈20.6 |
| 0xFC | 4 | 1 GiB | 1,048,576 | 3,145,728 | ≈21.6 |
| 0xFD to 0xFF | 5 to 7 | reserved, refused | | | |

Step 0 is RFC 9106's second recommended option as it stands (3 passes, 4
lanes, 64 MiB, 128-bit salt, 256-bit tag), except for the 32-byte size's
88-bit salt (§3). The other steps keep its passes
and lanes and raise its memory. A code can state no more than 1 GiB, so a
hostile code cannot ask a reader for more. Lanes do not change a result:
a reader on one thread computes the same output more slowly.

64 MiB stays the floor. A lower step would fall under both of RFC 9106's
recommended options, and is not wanted: a device without that memory has KEF
(§14), which needs none.

A writer chooses for the weakest machine the code will ever be opened
on, and 256 MiB is the suggested default.

**What small devices can read.** Memory is a hard limit; time is not. A
machine without the memory cannot open a code at any speed.

| A device with | Can read |
|---|---|
| A few MiB, up to about 32 MiB, of working memory (microcontroller signing devices, such as the 8 MB K210 boards Krux runs on) | Nothing |
| 512 MB (a Raspberry Pi Zero 2 W) | 64 and 128 MiB |
| 1 GB | up to 256 MiB; 512 MiB with little else running |
| 2 GB or more | every step |

The times on small devices are not measured here.

## 6. Keys, encryption and the tag

With `N` the envelope's total length as one byte (32, 53 or 62), `r` the room
(16, 32 or 37), `t` the tag's length (4, 4 or 8) and the salt 11, 16 or 16
bytes:

```text
pw      = UTF-8 bytes of the NFC form of the passphrase           (§10)
PRK     = Argon2id(pw, salt, m, t = 3, p = 4), 32 bytes
K_enc   = HKDF-Expand(PRK, "argon-qr-vault-1 enc", 32)
K_mac   = HKDF-Expand(PRK, "argon-qr-vault-1 mac", 32)
C       = P ⊕ ChaCha20(K_enc, nonce = twelve 0x00 bytes, counter = 0)[:r]
T(kind) = HMAC-SHA-256(K_mac, H || N || salt || kind || C)[:t]
```

The HMAC's message is always 30 bytes at the 32-byte size (1 + 1 + 11 + 1 +
16), 51 at the small size and 56 at the large one (1 + 1 + 16 + 1 + `r`), so
its fields are not ambiguous. Because the tags are cuts of one function under
one key, a 4-byte tag would be the start of an 8-byte tag over the same
message, and two 4-byte tags over the same message would be equal. `N` makes
the messages differ from their second byte, so that a tag at one size says
nothing about another.

**Writing.**

1. Refuse an empty passphrase, and an item that does not fit the size asked
   for (§3). Draw `salt`, 16 bytes or 11 at the 32-byte size, from a cryptographic random source, for
   this envelope alone. Sealing the same item again draws a new one. The
   writer's random draws come in this order: the salt, then for codex32 only
   the byte that fills the item's unused low bits, then the padding.
2. Make `P` and `kind` (§3), drawing the fill byte and the padding.
3. Derive the keys, encrypt, and compute `T` for every kind of the size (§3:
   nine, or three at the 32-byte size). If any other kind's tag equals this
   kind's, start again from step 1, drawing a new salt, fill byte and padding.
   This happens with probability about 2^-31 at the 32-byte size, 2^-29 at the
   small size and 2^-61 at the large one.
4. Output `H || salt || C || T(kind)`.

**Reading.**

1. Apply §9's recognition. Show the memory the step needs. A reader that can
   learn before asking for the passphrase that it cannot have the memory
   refuses then; otherwise a failed allocation during Argon2id is reported
   as "not enough memory", in words distinct from "does not open".
2. Derive the keys. Compute `T` for every kind of the size (nine, or three at
   the 32-byte size) and compare each with the stored `T`. A tag that would
   match a kind the size does not hold is never computed.
3. None matches: the code does not open. A reader cannot tell a wrong
   passphrase from a damaged code, and says so. More than one matches: refuse in the
   same words as "does not open", since no conforming writer makes this.
4. One matches: decrypt, and read the item by its kind's layout. A size, length
   or threshold out of range, an item that does not fit the room or text that
   is not valid UTF-8 means the code does not open, in the same words: no
   conforming writer makes this. The rest of `P` is discarded unread.
5. Show the item as §7 says, only after this step.

### 6.1 Why this construction

The construction is **generic Encrypt-then-MAC** (Bellare and Namprempre,
ASIACRYPT 2000; with nonces and associated data, Namprempre, Rogaway and
Shrimpton, EUROCRYPT 2014), over ChaCha20 and HMAC-SHA-256 with its output
cut to 4 or 8 bytes, under keys derived by HKDF-Expand from Argon2id's output.

Every primitive is published, but **the composition with a short tag is
not a named standard**: no RFC or NIST document defines it. That holds for
every option that fits, as the table shows. It is the first thing a
reviewer is asked about. The table's chances are for one tag value at each
tag length, written `2^-t` for a tag of `t` bits, 32 or 64; a reader testing
nine kinds multiplies each by nine, and three by three (§8).

| Option | Fits the small and large sizes? | Forgery chance per try | Status of a 4-byte and an 8-byte tag | Why or why not |
|---|---|---|---|---|
| **Encrypt-then-MAC, ChaCha20 and HMAC-SHA-256 cut to 4 or 8 (chosen)** | Yes | 2^-t plus HMAC's PRF advantage | Both below RFC 2104 §5's recommended 80 bits; 4 bytes is at the floor of NIST SP 800-107 Rev. 1 §5.3.3, which allows 32 | A cut-down PRF is a PRF, so the bound is simple and does not depend on message length. The reader needs only Argon2id, SHA-256 and ChaCha20, and ChaCha20 runs in constant time in plain software without AES hardware. Stated in full above, so any library that has the three primitives can implement it |
| AES-256-GCM, tag cut to 4 or 8 (KEF's version 20 at 4) | Yes, with a fixed nonce | About ℓ × 2^-t, with ℓ the GHASH blocks, here about 5 (Ferguson, 2005) | NIST SP 800-38D allows 32 and 64 bits only under its Appendix C's limits, and the second preliminary draft of its revision (June 2026) removes every tag under 96 bits | Named mode, and readable by more hardware. Worse bound, a standard withdrawing the length, and libraries that refuse short tags (some allow only 12 to 16 bytes) |
| ChaCha20-Poly1305, tag cut to 4 or 8 | Yes | Not a published bound for a cut tag | RFC 8439 defines no truncation | The authors know of no analysis to rest on. age uses this cipher with its tag uncut (§6.2) |
| AES-SIV (RFC 5297), AES-GCM-SIV (RFC 8452) | Only by changing them | | Neither defines truncation | The tag is also the counter's start, so cutting it changes the construction |
| AES Key Wrap (RFC 3394, SP 800-38F) | 32 bytes of item at 53 bytes only with a 12-byte salt; 37 bytes of item at 62 only with a 13-byte salt (RFC 5649's padded form, 48 bytes); 16 bytes of item at 32 only with a 7-byte salt, 56 bits, under RFC 9106 §4's 64 | About 2^-64 | Its 8-byte check is built in and defined | A wide-block construction made for wrapping keys: no nonce, and a changed bit garbles everything. Costs 3 or 4 bytes of salt, which RFC 9106 §4 permits under space constraints (64 bits at least). §13, question 3 |

### 6.2 The nonce

The nonce is fixed at zero, and the key is what is new each time. `K_enc`
comes from Argon2id over the passphrase and a random salt, 128 bits or 88 at
the 32-byte size, so a key encrypts one message of 16, 32 or 37 bytes and
nothing else. A keystream never repeats unless the same passphrase and the
same salt are used twice. With a good random source, two salts are likely to
repeat only after about 2^44 envelopes at the 32-byte size and 2^64 at the
others, all under one passphrase. A
fixed nonce under a per-file key is what age's scrypt recipient does with
ChaCha20-Poly1305.

**When this is not safe.** If a writer's random source repeats a salt,
two envelopes under one passphrase with different items share a keystream,
and `C1 ⊕ C2` reveals the exclusive or of the two items to anyone who has
both, without a guess. A random source that bad has probably also made a
weak seed. Drawing the nonce from the tag (a synthetic nonce) would limit
this for no extra bytes and more complexity: §13, question 2.

Nothing in the envelope stores a nonce, so nothing is spent on one.

### 6.3 Why two derived keys

One key is not used for two jobs. The HKDF labels keep them apart and name
the format, so another version's keys differ. RFC 5869 §3.3 allows skipping
Extract when the input is already a strong key. Argon2id's output is taken
as one (§13, question 9).

## 7. What is authenticated

| Field | Bound by |
|---|---|
| Marker and step (`H`) | the tag. The step also sets Argon2id's `m`, which enters its hash, so a changed step gives a different key |
| Total length | the tag, as `N`. It also sets how much is the salt and how much is `C` |
| `salt` | the tag and Argon2id |
| `kind` | the tag, not stored |
| `C` | the tag |
| The passphrase | through the keys |

All of this is to at most 32 bits at the 32-byte and small sizes and 64 at the
large one, less what three or nine kinds take (§8), which is all such a tag can give. The QR
code's own error correction is not a security check. Nothing about
a person, a wallet or a date is in the envelope, so nothing is
authenticated about them.

**What a reader shows.** Not every kind has a fingerprint, so what a person can
compare differs by kind. A format check here is a check of a false match, not
of the right item.

| Kind | Shown | What a person can compare | Checked by the reader |
|---|---|---|---|
| BIP-39 entropy | The words (BIP-39 English), which BIP-39's checksum completes, and the BIP-32 master fingerprint, 8 lower-case hex digits, taken with an empty BIP-39 passphrase | The fingerprint | Nothing more: any bytes of these lengths are valid |
| codex32 string | The string, its threshold, identifier and index, and whether it is a share or a secret; a secret (index `s`) with the fingerprint of the master key of its seed, in the form above | The identifier and threshold; a secret's fingerprint. A share has no fingerprint until a set is combined | Size, threshold and BIP 93's threshold rule. The checksum is computed, so it shows nothing |
| BIP-39 passphrase | The text | Its reading; a fingerprint only with the seed it goes with | Length and UTF-8 |
| Note | The text | Its reading | Length and UTF-8 |
| Raw key | The 32 bytes as 64 lower-case hex digits | Nothing in the format. A reader may also derive a public key where a person says what the key is for; the vectors do not | Nothing |

## 8. False matches

Taking HMAC-SHA-256 as a PRF under keys that look random, a wrong key matches
each kind's tag with chance 2^-32 or 2^-64, so a list of n kinds has n of them:

| Kinds tested | Small size, 4-byte tag | About 1 in | Large size, 8-byte tag | About 1 in |
|---|---|---|---|---|
| 1 | 2^-32 | 4.3 billion | 2^-64 | 1.8 × 10^19 |
| 2 (12 and 24 words, as an earlier draft had) | 2^-31 | 2.1 billion | 2^-63 | 9.2 × 10^18 |
| 5 (BIP-39 entropy alone) | 2^-29.7 | 859 million | 2^-61.7 | 3.7 × 10^18 |
| **9 (this list)** | **2^-28.8** | **477 million** | **2^-60.8** | **2.0 × 10^18** |

At the 32-byte size the reader tests three kinds (§3) under a 4-byte tag: 2^-30.4,
about 1 in 1.4 billion.

A false match also has to pass §6's format check to reach a person. Five
entropy kinds and the raw key pass always. With random bytes, the codex32 kind
passes in about 9.4% of cases at the small size (3 of 8 `size` values, a valid
threshold) and 15.7% at the large one (5 of 8); the passphrase and note kinds
each pass in about 0.45% (a length in range and valid UTF-8, which is mostly a
very short text), and the same at the 32-byte size's room of 16. So what
reaches a person is about 6.1 of the nine chances at the small size, 6.2 at the
large one, and 1.01 of the three at the 32-byte size, where the 12-word kind
passes always:

| Event, per attempt | 32-byte size | Small size | Large size | What a person sees |
|---|---|---|---|---|
| Wrong passphrase on a genuine code | 2^-32.0, about 1 in 4.3 billion | 2^-29.4, about 1 in 700 million | 2^-61.4, about 1 in 3 × 10^18 | A wrong item that looks valid: usually a seed, since any bytes make valid BIP-39 words |
| A code damaged past its error correction, right passphrase | about the same | about the same | about the same | The same |
| A forged or swapped code, without the passphrase | the same | the same | the same | The same. Without the key, the forger can choose at most which bits flip (§12), not the item |
| Any other code of 53 or 62 bytes, or of 47 characters whose 32 bytes start with one of the five defined values (5 in 256 of random byte codes; about 1 in 220 random texts of the table's characters, §9) | Asks for a passphrase, then the same | Asks for a passphrase, then the same | Asks for a passphrase, then the same | Normally "does not open" |

A wrong seed from a false match has a different fingerprint. This is why
a reader shows the fingerprint and a sheet may carry a space to write it,
outside the envelope: a person who keeps it detects every event above that
gives a seed. For the kinds without one, §7's table is what is left.

## 9. Recognition

A QR code's payload is treated as AQRV when one of these holds:

1. It is exactly 53 or 62 bytes, and its first byte is 0xF8 to 0xFC.
2. It is text of exactly 47 characters that §3's text form decodes to 32
   bytes, and the first of those bytes is 0xF8 to 0xFC.

Any other payload of 47 characters is not recognised: one with a character
outside QR's alphanumeric table (a lower-case letter among them), one whose
value is 256^32 or more, and one whose first byte is outside 0xF8 to 0xFC. A
payload of 32 bytes of binary is not recognised either, whatever its first
byte. A file is §9.1.

A false match only asks for a passphrase and fails at the tag; only the
tag proves a match. There is no room for a longer marker at 25 × 25 or 29 ×
29: one more byte needs a larger code.

**Why a high marker never reads as KEF.** Checked against Krux's published
KEF specification and `kef.py` on Krux's main branch (fetched 2026-11-04,
links in §16). A KEF
envelope is `len_id (1) || id (len_id) || version (1) || iterations (3) ||
cipher payload`, with `len_id` from 0 to 252. So a KEF envelope's length
is `5 + len_id + payload`, and:

```text
in 32 bytes:   len_id + payload = 27
in 53 bytes:   len_id + payload = 48
in 62 bytes:   len_id + payload = 57
```

- `len_id` is at most 27, 48 or 57, and that only with an empty payload, which no
  version can decrypt. In Krux's code the shortest payload `decrypt`
  accepts is 10 bytes (version 15), which brings them to 17, 38 and 47.
- AQRV's first byte is at least 248. A KEF envelope with `len_id` of 248 is
  at least 253 bytes long, and `len_id` of 253 or more is outside KEF's
  range altogether. Krux's `unwrap` reads the version at index
  `1 + len_id`, off the end of 32, 53 or 62 bytes, and answers "Invalid format".
- The other way, a KEF envelope of 32, 53 or 62 bytes never starts with 248 or
  more, so an AQRV reader never takes one. Length alone would not separate them:
  a version 20 envelope with an empty id is exactly 53 bytes and starts with
  0x00, one with a 9-byte id is exactly 62 and starts with 0x09, and a version 5
  envelope of 12 words with an 8-character id is exactly 32 and starts with
  0x08 (`docs/KEF.md` §8). That last one is a `.aqrv` file's length; in a QR
  code Krux writes it as text, below.

**KEF's base43 text.** Krux writes a KEF version 5 code of 12 words as 47
characters of base43 (`docs/KEF.md` §5), Electrum's alphabet
`0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ$*+-./:`. That is QR's table without space
and `%`, so a KEF text is a 47-character payload in AQRV's alphabet. Its digits
and letters have the same values in both alphabets (0 to 35), and the
symbols differ. Neither format takes the other's text:

- A KEF text read by AQRV. A KEF envelope of at most 32 bytes has `len_id` at
  most 27 (above), so its value is below 28 × 256^31, which is less than 10 ×
  43^46, and its base43 text starts with a digit, `0` to `9`; with Krux's
  default 8-character id it starts with `2`. Read in base 45, a text starting
  with a digit has a value below 10 × 45^46, which is less than 0xF8 × 256^31,
  so its first byte is below 0xF8 and it is not recognised. A vector shows it
  (§11).
- An AQRV text read by KEF. Most hold a space or `%` (all but about 1 in 8),
  which base43 does not have. The rest start with `A`, 10 in base43 as well, so
  their base43 value is at least 10 × 43^46 and below 43^47, which is 32 bytes
  with a first byte of 30 to 33. Read as KEF, that is a `len_id` above the 27
  that 32 bytes allow: at 31 to 33 Krux's `unwrap` reads the version off the
  end, as above, and at 30 the version is the last byte, with no room for the
  count or any payload. The text's
  odd length is not hex. Krux's reader also tries base32 and base64 on text
  (`docs/KEF.md` §5); those readings were not worked out here, and a KEF match
  of either would only offer to try a passphrase.

**Why the 32-byte size is text.** A 24-word Compact SeedQR is 32 bytes of
binary, and any 32 bytes are valid 24-word entropy. As 32 bytes of binary, an
AQRV code would be shown as a 24-word seed by a reader that knows Compact
SeedQRs and not AQRV: a wallet that is not the one sealed, with no history and
no warning. And 5 in 256 Compact SeedQRs of 24 words, about 1 in 51, would pass
recognition. Written as 47 characters, an AQRV code is not binary, so a
Compact SeedQR reader does not take it, and a Compact SeedQR is never 47
characters, so an AQRV reader does not take one. It costs no size: 47
characters is version 2-L's whole alphanumeric capacity, the same code as 32
bytes. A standard SeedQR is 48 or 96 digits, never 47 characters, and a text
starting with a digit is never recognised.

**Other codes.** 0xF8 to 0xFF never occur in valid UTF-8, so no valid
UTF-8 text code (a descriptor, a word list, a UR or BBQr part) can match.
A Compact SeedQR is 16 or 32 bytes of binary and never matches. An RFC 9580 message under a
passphrase is far longer, and begins with a tag 3 packet. A text in
another encoding, such as Latin-1, can start with those bytes: it asks for
a passphrase and fails.

Other text of 47 characters in QR's table passes only if it starts with `A`
and its value falls in the marker's range, about 1 in 220 of random texts; it
then asks for a passphrase and fails at the tag.

### 9.1 A .aqrv file

A `.aqrv` file holds exactly the envelope's bytes, 32, 53 or 62, with nothing
before or after: no text form, not even the 32-byte size's 47 characters, no
newline and no header of its own. The text
form is §15's printed sheet, which is not a file of this kind.

A reader takes a file as AQRV only if its length is exactly 32, 53 or 62 bytes and
§9's recognition passes on its bytes. Otherwise it says the file is not an
AQRV code and asks for no passphrase.

A writer names the file with the `.aqrv` extension. The suggested media type is
`application/vnd.aqrv`; it is not registered with IANA. Until it is,
`application/octet-stream` is used.

A file and a QR code of the same envelope are the same bytes, so either opens
with the same passphrase.

## 10. The passphrase

The passphrase is put in NFC, and Argon2id takes that form's UTF-8 bytes.
Nothing else is done: no character is mapped or refused; compatibility
characters, case and spaces are kept as typed. Two passphrases with one
NFC form are one passphrase. A passphrase read from a file or a pipe has one
trailing line ending (`\n` or `\r\n`) taken off and nothing else. A writer refuses an empty one and requires no
other minimum, since a typed passphrase's strength cannot be measured; it
should offer a generated one (§12's words) and say that the code is only
as strong as its passphrase.

BIP-39 takes its own passphrase in NFKD. AQRV's choice differs, on purpose
and only in this: a person who types "ﬁ" and later "fi" has two
passphrases. Whether NFC or NFKD serves a format meant to outlast its
keyboards better is §13, question 8. This is the passphrase that opens the
code, not a BIP-39 passphrase held as an item (§3), which is stored as given.

## 11. Test vectors

The vectors are `tools/vectors/aqrv/aqrv.json`, with a README and the QR
code of one vector at each of the three sizes as a PNG. They are computed by
`tools/vectors/aqrv/gen.py` with libraries the format's authors did not write
(argon2-cffi for Argon2id; Python's `hmac` and `hashlib`; PyCA cryptography's
ChaCha20), so that they do not rest on a first implementation. The entropies of the
entropy cases are BIP 39's published test entropies (all `00`, `7f`, `80` or
`ff` bytes; the step 0 seeds read "abandon … about" and "zoo … vote"), taken
from the Trezor reference vectors, and each such case records its words;
the 20- and 28-byte ones, which BIP 39 does not list, have the words of
python-mnemonic. Each case gives
every intermediate value (`pw`, `PRK`, `K_enc`, `K_mac`, the keystream, `C`,
all nine tags) and the whole envelope, and says what a reader must do with it:
open to given fields, not open, or not recognise it. They cover, at the small
and large sizes unless an item fits only one:

- every defined step, each with a 24-word seed;
- every kind, with its padding bytes given as input: all five lengths of
  entropy, the raw key, a passphrase and a note of the shortest and longest
  length, and text that is not ASCII;
- codex32 strings from BIP 93's own test vectors: vector 2's upper-case shares
  and its secret, vector 3's shares and its four strings of one seed, vector
  4's sixteen, and the 160-, 192- and 224-bit strings of vectors 6 to 8, each
  coming back as the exact string;
- a passphrase that is not ASCII, in NFC and in NFD, opening to one result;
- the all-zero and all-one seeds, to show padding and `kind`;
- must-fail cases: a wrong passphrase; a bit changed in each field; a
  reserved step; 52, 54, 61 and 63 bytes; marker 0b11110; a 62-byte envelope
  cut to 53, and on each size a tag made with the other size's `N`;
- must-fail cases whose tag is right, made with the key: a `len` of 0 and one
  past the room; text that is not valid UTF-8; a codex32 `size` of 5 to 7 and
  of 3 or 4 at the small size; a threshold field of value 1 (the character `p`); a threshold of `0` with
  an index other than `s`;
- must-not-recognise cases: real KEF version 20 envelopes of 53 bytes with an
  empty id and 62 bytes with a 9-byte id, built from `docs/KEF.md`'s layout;
- at the 32-byte size: a 12-word seed at every step; a passphrase and a note
  of the shortest and longest (15 bytes) length and not ASCII; a note of
  multi-byte characters ending at the room; the all-zero and all-one 12-word
  seeds; the NFC and NFD passphrase; the same must-fail cases as above with 31
  and 33 bytes, a 53-byte envelope cut to 32, and a tag made with `N` = 53 on
  it and with `N` = 32 on a 53-byte one; right tags for kinds the size does not
  hold (24 words, codex32, a raw key), which a reader does not test; right tags
  over a `len` of 0 and of 16 and over text that is not UTF-8; a KEF version 5
  envelope of 32 bytes with an 8-character id, which is not recognised; and
  items a writer refuses at this size (24 words, a 16-byte note, a codex32
  string);
- the 32-byte size's text form (`qr_text`): one vector's 47 characters, which
  decode to its envelope and open, and texts that are not recognised: the
  value 256^32, 46 and 48 characters, a lower-case letter, a character outside
  the table, and the KEF version 5 envelope above as Krux's base43 text;
- the QR code of one vector at each size, as an image and as its module
  matrix, the 32-byte one holding its 47 characters. The payload, the mode,
  the version and the level are normative; the matrix is
  one encoder's output, its mask that library's choice, and a reader compares
  decoded bytes, not modules. The vectors give each random draw of §6 (the
  salt, the fill byte, the padding).

`tools/vectors/aqrv/check.py` is a second reader and writer, written from §2 to §10
alone with other libraries (PyCA cryptography's Argon2id; ChaCha20 written out from
RFC 8439), and it agrees with every case, the sealed envelopes, the text forms and the QR
pictures, which zbar decodes to the envelope's bytes, or at the 32-byte size to its 47
characters (zxing-cpp is run too where it is installed). It was written by Faraday's authors,
so it is not the outside implementation this section and the **Open:** line ask for.
The Rust library `aqrv`, the command line `aqrv-cli` and the desktop test app in `aqrv/` are also by Faraday's authors; the library's tests read every case and the three pictures, and `aqrv vectors` runs them. They are not an outside implementation either.

A vector where two tags agree is not offered: finding one takes about 2^31
Argon2id runs at the 32-byte size, 2^29 at the small size and 2^61 at the large one, so that branch of
§6 is tested by the code's structure and not by a published input.

## 12. Security analysis

**Offline guessing is the attack that counts.** Anyone who sees the code
holds the salt, the ciphertext and the tag, and can guess for as long as
the item matters. The cost of one guess is one Argon2id run; it cannot be
shared across codes because of the salt, and it cannot be skipped.

**What the salt does, and the 32-byte size's 88 bits.** A random salt, new for
each code, does three things: no guessing work can be done before the code is
seen, since the salt is not known until then; one run of Argon2id tests a guess
against one code only, so many codes cannot be attacked as a batch; and each
code has its own keys, which is what makes the zero nonce safe (§6.2). It does
not make one code harder to guess: that is the passphrase and the memory step.
88 bits keeps all three. Against 128 bits, what it gives up is the margin
against a repeat: two random salts are likely to agree only after about 2^44
codes under one passphrase, against 2^64, which in practice is nothing for codes
made by people. RFC 9106 §4 allows 64 bits where space is short (§3).

The tag does not change this much at the small size. It rejects all but about
1 in 477 million wrong guesses at no cost beyond Argon2id. Without it, an
attacker would test each guess by deriving an address (BIP-39's 2,048 rounds,
then BIP-32) and looking it up, which is small beside Argon2id. So the tag
saves an attacker little. It is also not enough on its own: of 2^77.5 six-word
passphrases, about 2^48.7 pass the tag, and each survivor still has to be
checked against the chain. A seed under a BIP-39 passphrase gives no such check.

**The large size changes that.** Of the same 2^77.5, about 2^16.7 pass an
8-byte tag, and for anything up to four words (2^51.7) a pass is almost
certainly the right passphrase. The 8-byte tag is a better check for a person
and also a better test for a guesser, who no longer needs the chain, or
anything else outside the code. That is why the passphrase's strength has to
stand alone at the large size. The same goes for a text kind at either size:
readable text says the guess is right.

**The cost per step.** A figure for scale, not an estimate of any real
attacker: 2^40 Argon2 block steps a second, about a million cores each
filling about 1 GiB a second. Memory is what the figure leaves out: at 64
MiB that many cores hold 64 TiB, and at 1 GiB, 1 PiB. The times include
Argon2: a guess costs three passes over its memory in 1 KiB blocks, 2^17.6
block steps at 64 MiB, 2^19.6 at 256 MiB and 2^21.6 at 1 GiB, which is where
each column's guesses a second come from. The total work is the passphrase's
bits plus those. The KEF column is Krux's KEF (§14) at its default of 100,000
PBKDF2-HMAC-SHA-256 iterations, 200,000 SHA-256 compressions a guess, on the
same million cores at about 2^25 compressions a second each (about 2 GB a
second with SHA extensions): 2^45 a second, about 176 million guesses a
second, 31 times the 64 MiB rate. Time to try every passphrase of EFF's long
list (12.925 bits a word; half this on average):

| Passphrase | Passphrase bits | Total work, bits (64 MiB / 256 MiB / 1 GiB) | 64 MiB (5.6 million guesses/s) | 256 MiB (1.4 million) | 1 GiB (350,000) | KEF (176 million) |
|---|---|---|---|---|---|---|
| 3 words | 38.8 | 56.4 / 58.4 / 60.4 | 1 day | 4 days | 16 days | 45 minutes |
| 4 words | 51.7 | 69.3 / 71.3 / 73.3 | 21 years | 83 years | 331 years | 241 days |
| 5 words | 64.6 | 82.2 / 84.2 / 86.2 | 160,000 years | 640,000 years | 2.6 million years | 5,100 years |
| 6 words | 77.5 | 95.1 / 97.1 / 99.1 | 1.3 billion years | 5 billion years | 20 billion years | 40 million years |

Without Argon2, one SHA-256 a guess at 2^45 a second tries every 3-word
passphrase in under a second, every 4-word one in about 2 minutes and every
5-word one in about 9 days. KEF's figure counts its compute only: it needs
almost no memory, so GPUs and dedicated hardware, which Argon2's memory
resists, make it cheaper still.

The five steps span a factor of 16, four bits; one more word is 12.9 bits.
The word count matters more than the step. A person-chosen passphrase can
be far weaker than any row. What dedicated hardware actually achieves
against these parameters is §13, question 6. These figures count work
only; they do not measure how much memory costs an attacker, and they are
for today's hardware, while a printed code can last decades.

**The honest limits of a short tag.**

- **Forgery.** A forger's try passes the tag with probability 2^-30.4 at the
  32-byte size, 2^-28.8 at the small size and 2^-60.8 at the large one. The first is low only because each try
  needs a person to scan a code and type a passphrase, and to do it again if it
  fails. A system that retries codes automatically with a stored passphrase
  would make this bound worthless. No such use is in scope, and it should not
  be built.
- **A swapped code.** Someone who replaces a printed code with their own,
  without the passphrase, gets "does not open", except with the same
  probability. Someone who knows the passphrase can make any code, and no tag
  length would help.
- **Bit flips.** `C` is a stream cipher's output. A changed bit changes the
  same bit of the item, and passes with the same probability. The 8-byte tag
  makes this negligible for a person's tries; a wide-block cipher would make
  the result garbage.
- **Key commitment.** None. An attacker can make a code that opens under two
  chosen passphrases: two Argon2id runs, then about 2^32 HMACs over different
  `C` at the small size and 2^64 at the large one. Nothing in the format's use
  tells the attacker which one a person typed. §13, question 4.
- **A wrong item.** A false match gives a wrong, valid-looking item about once
  in 700 million wrong tries at the small size and once in 4.3 billion at the
  32-byte size (§8). A person who has the
  fingerprint detects it for a seed. A person who does not would see a wallet
  with no history.
- **A wrong share.** A codex32 string's checksum is computed on reading, so it
  adds nothing against a false match or a flipped bit, which would come back
  with a valid checksum. BIP 93's checksums are preserved when shares are
  combined, so such a share in a set recovers a wrong secret that also has a
  valid checksum. Only the fingerprint of the recovered seed shows it. A person
  using shares under the small size should keep that fingerprint.
- **A raw key** has no check of any kind after the tag: a false match gives 32
  bytes that look as right as any others.

**Other limits.**

- A bad random source at writing breaks the salt (§6.2) and the padding.
- Argon2id's memory access depends on its input after its first half
  pass, as RFC 9106 describes, so a side channel on the reader could reveal
  something about a passphrase. AQRV is meant for offline devices.
- A malicious writer can hide data in padding or the salt. Any format with
  random fields allows this, and a writer that malicious already has the item.
- Low error correction tolerates less damage than the same bytes at medium
  (§3); a 25 × 25 code corrects up to 4 codewords.
- A 32-byte code's text is read as decoded. A scanner or program that trims
  spaces loses the last character of about 1 in 45 codes, and the code is then
  not recognised (§3).
- Implementing the 32-byte size needs an Argon2id that takes an 11-byte salt.
  libsodium's `crypto_pwhash` takes no salt length: its salt is
  `crypto_pwhash_SALTBYTES` bytes, 16 (`crypto_pwhash.h` and
  `crypto_pwhash_argon2id.h`, read for this). RFC 9106's reference code takes
  any salt of 8 bytes or more (`ARGON2_MIN_SALT_LENGTH` in its `argon2.h`),
  and argon2-cffi and PyCA cryptography, which made and checked the vectors,
  take 11 bytes. A code that is the only copy should be scanned back after printing,
  and is better kept with a second copy.

## 13. Questions for an outside cryptographer

Answers to any of these are wanted, and an answer of "this is wrong" most
of all.

1. **The tags.** Is Encrypt-then-MAC with HMAC-SHA-256 cut to 32 or 64 bits
   sound for this use, in which every verification costs a person typing a
   passphrase? Both are below RFC 2104 §5's 80 bits, and 32 is at the floor of
   NIST SP 800-107 Rev. 1 §5.3.3 (no less than 32 bits). That document is
   reported, not checked here at the source, to be planned for withdrawal. Does
   4 bytes do something 0 bytes would not? Is the 8-byte tag's better check
   worth what it gives a guesser (§12)?
2. **The nonce.** Is a fixed zero nonce under a key derived per salt
   acceptable? Or should the nonce be synthetic (taken from the tag, in the
   manner of SIV), to limit the damage of a repeated salt, at the cost of a
   less simple reader?
3. **Another shape.** Is AES Key Wrap with a 12-byte salt (1 + 12 + 40 = 53
   bytes) or a 13-byte one (1 + 13 + 48 = 62) better than this? It gives an
   8-byte built-in check and no bit malleability, against a smaller salt and
   a stronger test for a guesser. How should it carry `kind`?
4. **Key commitment and a chosen code.** Does any realistic use give an
   attacker an oracle that makes the lack of commitment matter?
5. **Many kinds.** Is testing nine tags, with `kind` in the MAC input and random
   padding, sound? Is a false-match chance of about 2^-29 instead of 2^-32 a
   fair price for hiding what a code holds? Should the list be shorter: for
   example the four shorter entropy lengths as one kind with a length byte
   (six kinds instead of nine, 0.6 of a bit), or no raw key? Does a reader's
   format check after the tag (§6) help or give a guesser anything new?
6. **Guessing cost.** What do current GPUs, FPGAs and possible ASICs achieve
   against Argon2id at `t` = 3, `p` = 4 at these memory sizes, with published
   time-memory trade-offs counted, and the depth-reducing attacks of Alwen
   and Blocki? Is 3 passes at 64 MiB better spent as one pass at more memory?
7. **Two sizes.** Is one set of keys for both sizes right, with `N` in the
   HMAC's message to keep the tags apart, or should the size also enter the HKDF
   labels? Is one byte of `N` needed at all, given that the two messages already
   differ in length?
8. **The passphrase.** NFC, NFKD, or bytes as typed, for a format that may
   be opened decades later on a different keyboard?
9. **HKDF.** Is Expand without Extract right for Argon2id's output, or should
   Argon2id output 64 bytes split in two with no HKDF at all?
10. **Recognition.** Is a 5-bit marker and the length enough? Should the
    marker bits enter Argon2id as associated data? Many libraries do not
    expose that input, and the tag covers it, which is why it does not.
11. **Anything else** in this document that a reviewer would not sign.

## 14. Comparison with KEF and with an OpenPGP QR vault

**At 25 × 25, a 12-word seed.** Krux's KEF version 5 and AQRV's 32-byte size
are the same size of code. As `docs/KEF.md` §3 to §5 and §10 describe it:

| | AQRV, 32 bytes | KEF version 5, Krux's defaults |
|---|---|---|
| Salt | Random, 11 bytes (88 bits), new for each code, naming nothing | The `id`: by default the wallet's fingerprint as 8 hex characters, 32 bits, in the clear, the same for every code of that wallet, and known to anyone who has seen its descriptor or a PSBT |
| Work before the code is seen | None: the salt is unknown | Possible for a known wallet. Krux adds a random change under 10% to the count, which is in the clear, but one PBKDF2 run to the highest count passes every lower one |
| Key derivation | Argon2id, 64 MiB to 1 GiB | PBKDF2-HMAC-SHA-256, 100,000 iterations; no memory cost |
| Cipher | ChaCha20 under keys used once | AES-256-ECB. The same seed under the same passphrase, id and count gives the same code; only the count's random change keeps two codes of a wallet apart, and two can draw the same one |
| Check | HMAC-SHA-256 over the whole envelope and `kind`, cut to 4 bytes | `SHA-256(v ‖ P ‖ k)[:3]`, over the plaintext and key, exposed; not over the ciphertext |
| In the clear | The marker, the step and the salt | The wallet's fingerprint, the version and the count |
| In the QR code | 47 characters of base-45 text in QR's alphanumeric table, alphanumeric mode | 47 characters of base43 text, alphanumeric mode |
| Read as a seed by a reader of Compact SeedQRs | No: it is text (§9) | No: it is text |
| Read as the other format | No (§9) | No (§9) |
| Small devices | Needs 64 MiB or more | Runs on microcontrollers |
| Standing | This draft: a Rust implementation by Faraday's authors in `aqrv/` (library, command line, desktop test app), checked against the vectors and not an outside one; no review | One project's published specification, in use on its devices |

KEF's version 5 does better in two of these: it runs on microcontrollers,
and it is in use. Both are written as text, so neither is taken for a seed.
A person can
change its id to something other than the fingerprint (`docs/KEF.md` §10).

**All sizes.**

| | AQRV | Krux's KEF, version 20 | The OpenPGP QR vault (`docs/VAULT.md` §1.2) |
|---|---|---|---|
| Key derivation | Argon2id, 64 MiB to 1 GiB | PBKDF2-HMAC-SHA-256, Krux's default 100,000 iterations; no memory cost | Argon2id, 64 MiB to 1 GiB |
| Cipher and tag | ChaCha20, HMAC-SHA-256 cut to 4 or 8 bytes | AES-256-GCM, tag cut to 4 bytes | AES-256-OCB, full 16-byte tags |
| Salt | Random, 16 bytes; 11 at the 32-byte size | The `id` field; by default the wallet's fingerprint, in the clear, changeable by the person | Random, 16 bytes |
| Bytes for a 24-word seed (KEF worked out, assuming raw entropy and raw bytes in the code) | 53, or 62 with the longer tag | 61 with an 8-character fingerprint id; 69 with a random 16-byte id | 396 |
| Code | 32 bytes: 25 × 25, low; 53 bytes: 29 × 29, low; 62 bytes: 33 × 33, medium | 61 bytes: 33 × 33, medium; 69 bytes: 33 × 33 low or 37 × 37 medium | 77 × 77, medium |
| Copied by hand | Plausibly, not yet tried | Plausibly | No |
| Holds | A fixed list (at the 32-byte size, 12 words or up to 15 bytes of text): BIP-39 entropy of any length, a codex32 string up to 192 bits (256 at the large size), up to 31 bytes (36) of passphrase or note, or a raw 32-byte key | Any bytes, with versions for several | A seed, a codex32 share, a BIP-39 passphrase or a note, up to 226 bytes of text |
| Hides what it holds | Yes, within a size. A share over 192 bits needs the large size | No: the length shows | Yes |
| Standing | This draft: a Rust implementation by Faraday's authors in `aqrv/` (library, command line, desktop test app), checked against the vectors and not an outside one; no review | One project's published specification, in use on its devices | An RFC 9580 message (Proposed Standard), which several outside programs open; not yet in the app |
| Small devices | Needs 64 MiB or more | Runs on microcontrollers | Needs 64 MiB or more |

**Could AQRV be a version of KEF?** Not at these sizes, the 32-byte one
included: its header, id length, version and count take 1 + 1 + 1 + 3 bytes
before any salt. KEF's envelope keeps
its id length byte, version byte and 3 iteration bytes: 1 + 16 + 1 + 3 + 32
+ 4 = 57 bytes, over 29 × 29's 53, and 1 + 16 + 1 + 3 + 37 + 8 = 66, over 33 ×
33 medium's 62, with the iteration field meaning nothing under Argon2id.
Offering an Argon2id version to KEF at 33 × 33 is another way to the same aim,
and one the authors of KEF would decide. KEF's strength is that it runs on
small devices; AQRV gives that up for memory hardness.

AQRV does not replace the OpenPGP form, which has a published standard and
outside readers. It is for when size is the point.

## 15. A printed sheet (not part of the format)

A sheet may print the envelope's values beside the code, so a person can
read them without a scanner:

```json
{"format":"AQRV","version":1,"memory_mib":256,"passes":3,"lanes":4,
 "salt":"<22 or 32 hex>","ciphertext":"<32, 64 or 74 hex>","tag":"<8 or 16 hex>"}
```

Every value comes from the envelope, and every one can be worked out again
from a scan, so losing the text loses nothing. The text is never an input to
the cryptography. It also offers a way back from a damaged code: typed in, it
remakes the envelope, whose size the lengths of its values give. A mistyped
character looks like a wrong passphrase. The name holds no word for a seed or a
wallet, so a sheet can use it without saying what it holds.

## 16. The path to a standard

| Step | State |
|---|---|
| A neutral name | Argon QR Vault, abbreviated AQRV, with the full name first. Web searches on 2026-11-11 for "Argon QR Vault" found no format of that name: the nearest were an npm tool, argon-vault, that encrypts key and value secrets under a passphrase without QR codes, and unrelated QR vault apps and sites such as QRVault and VaultQR. A search for "AQRV" found no standard, product or term of that name (only unrelated names such as ArqVR, a 3D architecture company, and function names in Rust's hexagon intrinsics). A search for the `.aqrv` file extension found no use of it; the nearest are `.aq` and `.aqz` (a genealogy program) and `.qrv`. The format was first called the Argon2id Compact Envelope (ACE), an earlier working name. To check again before public comment |
| A written specification | This document, a draft |
| Test vectors | §11: made, and agreed by a second reader of the same authors, which is not an outside implementation |
| An outside second implementation | None. Anyone who is not one of the authors may write one from this document alone |
| An outside cryptographic review | None. §13 is what is asked; how a review is recorded is `docs/AUDIT-SUBMISSIONS.md` §10 |
| Public comment | Not begun. KEF went to the Delving Bitcoin forum; a BIP or a SLIP is possible after a review |

Until a review is recorded, the format stays out of Faraday's app: this
specification is Faraday's own, and `docs/STANDARDS.md` §0 keeps such work
in the repository, compiled out. Nothing here is built, so nothing today is
compiled out either. A review that finds the 4-byte tag unacceptable ends the
32-byte and small sizes and not the aim, since the large size has an 8-byte tag.

**References.** RFC 9106 (Argon2); RFC 2104, RFC 3629, RFC 5869, RFC 8439; RFC
3394, RFC 5649, RFC 5297, RFC 8452, RFC 9580; NIST SP 800-107 Rev. 1; NIST SP 800-38D and
its revision's second preliminary draft
(`https://csrc.nist.gov/pubs/sp/800/38/d/r1/2prd`); Krux's KEF specification
(`https://selfcustody.github.io/krux/getting-started/features/encryption/kef-specifications`)
and code (`https://github.com/selfcustody/krux/blob/develop/src/krux/kef.py`);
the C2SP age specification (`https://c2sp.org/age`); BIP-39; BIP-173; BIP 93;
UAX #15; ISO/IEC 18004 (QR codes).
