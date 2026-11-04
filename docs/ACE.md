# Argon Compact Envelope (ACE)

**Kind:** Spec
**Status:** Draft for outside review. Own specification, not yet reviewed outside Faraday; no code exists and nothing is compiled into any app (`docs/STANDARDS.md` §0)
**Open:** everything: no implementation, the test vectors (§11), an outside second implementation, an outside cryptographic review of the construction and its 4-byte tag (§13), public comment, and whether Faraday ever offers it

ACE is a proposed open format for one BIP-39 seed encrypted under a
passphrase with Argon2id, in 53 bytes, small enough for a 29 × 29 QR code
at low error correction. That is the size of a standard 24-word SeedQR; a
Compact SeedQR of 24 words is 25 × 25. Everything
needed to open it, apart from the passphrase, is in those 53 bytes.

This document is written for someone who has never seen Faraday. Faraday
is where the draft started; the format carries no name of Faraday's and
depends on nothing of it. A statement the authors cannot support is
written as a question in §13, not as a claim.

## 1. Purpose, and what it does not do

**Purpose.** A person can print or hand-copy a seed's encrypted form and
keep it where a finder could see it. A finder holds nothing until they
guess the passphrase, and each guess costs memory as well as time. Krux's
KEF format (§14) does this in a similar space under PBKDF2, which needs
no memory. ACE is the same idea under a memory-hard function.

**It does not:**

- make a weak passphrase safe. The code can be photographed and attacked
  offline for as long as the seed holds money (§12);
- hold anything but a 12-word or 24-word seed's entropy: no codex32
  share, no BIP-39 passphrase, no note, no descriptor, no name or date.
  A wallet that uses a BIP-39 passphrase keeps it separately;
- prove with high assurance that a seed is the one that was sealed. Its
  tag is 4 bytes (§8, §12). A reader shows the wallet's fingerprint so
  that a person who has it written down can compare it;
- open on devices without the memory. The smallest step needs 64 MiB
  (§5). Most microcontroller signing devices cannot read it;
- hide that a code may be an ACE code. Its first byte is distinctive
  (§9);
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
| Entropy and words | BIP-39 |

## 3. The envelope

An envelope is exactly 53 bytes:

| Offset | Length | Field | Contents |
|---|---|---|---|
| 0 | 1 | header `H` | marker in the high 5 bits, memory step in the low 3 (§4) |
| 1 | 16 | `salt` | random, new for every envelope |
| 17 | 32 | `C` | the padded entropy `P`, encrypted (§6) |
| 49 | 4 | `T` | the tag (§6) |

Nothing else exists: no length, no version field apart from the marker,
no identifier, no fingerprint, no iteration count.

**Padding.** `P` is always 32 bytes:

- A 24-word seed: `P` is its 32 bytes of entropy; `kind` is 0x20.
- A 12-word seed: `P` is its 16 bytes of entropy followed by 16 bytes
  from the writer's random source, read once and never stored; `kind` is
  0x10.

`kind` is not stored. It enters the tag, and a reader finds it by testing
both values (§6, §7). So an envelope does not show which length it holds,
and a 12-word seed's padding gives a guesser no known plaintext to test a
guess against. Zero padding would give one: 128 bits of known plaintext
that identify the right passphrase with certainty. The cost is that a
wrong passphrase has two chances to match (§8).

Other BIP-39 lengths (15, 18 and 21 words) are not in this version: each
would add a chance of a false match (§13, question 5).

**The QR code.** The 53 bytes are one byte-mode segment with no ECI
designator. That is QR version 3 (29 × 29 modules) at low error
correction, whose byte-mode capacity is exactly 53 bytes. Version 3-L has
15 error correction codewords in one block of 70 and corrects up to 7
damaged codewords. A writer may draw the same bytes larger, such as
version 4 (33 × 33) at medium, which holds 62 bytes, for more tolerance;
a reader takes the payload and does not look at the version or the level.

## 4. The header

```text
bit     7 6 5 4 3   2 1 0
        1 1 1 1 1   s s s     marker 0b11111, step sss
```

The marker is this version of the format. A reader refuses a step it does
not define. A later version takes marker 0b11110 or another total length;
it does not reuse this one.

## 5. Memory steps

Argon2id takes, for every step:

| Parameter | Value |
|---|---|
| Passes `t` | 3 |
| Lanes `p` | 4 |
| Output length | 32 bytes |
| Version | 0x13 |
| Secret `K`, associated data `X` | none |
| Salt | the envelope's 16 bytes |
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
lanes, 64 MiB, 128-bit salt, 256-bit tag). The other steps keep its passes
and lanes and raise its memory. A code can state no more than 1 GiB, so a
hostile code cannot ask a reader for more. Lanes do not change a result:
a reader on one thread computes the same output more slowly.

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

The times on small devices are not measured here. Whether a lower step
should exist for microcontrollers is §13, question 7: a step below 64 MiB
falls under both of RFC 9106's recommended options, and it is the
envelope's main limit on adoption.

## 6. Keys, encryption and the tag

```text
pw      = UTF-8 bytes of the NFC form of the passphrase           (§10)
PRK     = Argon2id(pw, salt, m, t = 3, p = 4), 32 bytes
K_enc   = HKDF-Expand(PRK, "argon-compact-envelope-1 enc", 32)
K_mac   = HKDF-Expand(PRK, "argon-compact-envelope-1 mac", 32)
C       = P ⊕ ChaCha20(K_enc, nonce = twelve 0x00 bytes, counter = 0)[:32]
T(kind) = HMAC-SHA-256(K_mac, H || salt || kind || C)[:4]
```

The HMAC's message is always 50 bytes (1 + 16 + 1 + 32), so its fields
are not ambiguous.

**Writing.**

1. Refuse an empty passphrase. Draw `salt`, 16 bytes, from a
   cryptographic random source, for this envelope alone. Sealing the same
   seed again draws a new one.
2. Make `P` and `kind` (§3).
3. Derive the keys, encrypt, and compute `T` for both kinds, 0x10 and
   0x20. If the two are equal, start again from step 1. This happens with
   probability 2^-32.
4. Output `H || salt || C || T(kind)`.

**Reading.**

1. Apply §9's recognition. Show the memory the step needs; refuse before
   asking for the passphrase if the machine cannot allocate it.
2. Derive the keys. Compute `T(0x10)` and `T(0x20)` and compare each with
   the stored `T`.
3. Neither matches: the code does not open. A reader cannot tell a wrong
   passphrase from a damaged code, and says so. Both match: refuse, since
   no conforming writer makes this.
4. One matches: decrypt. For 0x20, the entropy is all 32 bytes of `P`; for
   0x10, the first 16, and the rest is discarded unread. Make the words by
   BIP-39, which adds their checksum. Show the words only after this step,
   with the wallet's fingerprint.

### 6.1 Why this construction

The construction is **generic Encrypt-then-MAC** (Bellare and Namprempre,
ASIACRYPT 2000; with nonces and associated data, Namprempre, Rogaway and
Shrimpton, EUROCRYPT 2014), over ChaCha20 and HMAC-SHA-256 with its output
cut to 4 bytes, under keys derived by HKDF-Expand from Argon2id's output.

Every primitive is published, but **the composition with a 4-byte tag is
not a named standard**: no RFC or NIST document defines it. That holds for
every option that fits, as the table shows. It is the first thing a
reviewer is asked about. The table's chances are for one tag value; a reader
testing two kinds doubles each (§8).

| Option | Fits 53 bytes? | Forgery chance per try | Status of a 4-byte tag | Why or why not |
|---|---|---|---|---|
| **Encrypt-then-MAC, ChaCha20 and HMAC-SHA-256 cut to 4 (chosen)** | Yes | 2^-32 plus HMAC's PRF advantage | Below RFC 2104 §5's recommended 80 bits; at the floor of NIST SP 800-107 Rev. 1 §5.3.3, which allows 32 | A cut-down PRF is a PRF, so the bound is simple and does not depend on message length. The reader needs only Argon2id, SHA-256 and ChaCha20, and ChaCha20 runs in constant time in plain software without AES hardware. Stated in full above, so any library that has the three primitives can implement it |
| AES-256-GCM, tag cut to 4 (KEF's version 20) | Yes, with a fixed nonce | About ℓ × 2^-32, with ℓ the GHASH blocks, here about 5 (Ferguson, 2005) | NIST SP 800-38D allows 32 bits only under its Appendix C's limits, and the second preliminary draft of its revision (June 2026) removes every tag under 96 bits | Named mode, and readable by more hardware. Worse bound, a standard withdrawing the length, and libraries that refuse short tags (some allow only 12 to 16 bytes) |
| ChaCha20-Poly1305, tag cut to 4 | Yes | Not a published bound for a cut tag | RFC 8439 defines no truncation | The authors know of no analysis to rest on. age uses this cipher with its tag uncut (§6.2) |
| AES-SIV (RFC 5297), AES-GCM-SIV (RFC 8452) | Only by changing them | | Neither defines truncation | The tag is also the counter's start, so cutting it changes the construction |
| AES Key Wrap (RFC 3394, SP 800-38F) | 57 bytes with a 16-byte salt: no. 53 only with a 12-byte salt | About 2^-64 | Its 8-byte check is built in and defined | A wide-block construction made for wrapping keys: no nonce, and a changed bit garbles everything. Costs 4 bytes of salt, which RFC 9106 §4 permits under space constraints (64 bits at least). §13, question 3 |

### 6.2 The nonce

The nonce is fixed at zero, and the key is what is new each time. `K_enc`
comes from Argon2id over the passphrase and a random 128-bit salt, so a
key encrypts one 32-byte message and nothing else. A keystream never
repeats unless the same passphrase and the same salt are used twice. A
fixed nonce under a per-file key is what age's scrypt recipient does with
ChaCha20-Poly1305.

**When this is not safe.** If a writer's random source repeats a salt,
two envelopes under one passphrase with different seeds share a keystream,
and `C1 ⊕ C2` reveals the exclusive or of the two seeds to anyone who has
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
| `salt` | the tag and Argon2id |
| `kind` | the tag, not stored |
| `C` | the tag |
| The passphrase | through the keys |

All of this is to at most 32 bits, which is all a 4-byte tag can give. The QR
code's own error correction is not a security check. Nothing about
a person, a wallet or a date is in the envelope, so nothing is
authenticated about them.

## 8. False matches

Taking HMAC-SHA-256 as a PRF under keys that look random:

| Event | Chance per attempt | What a person sees |
|---|---|---|
| Wrong passphrase on a genuine code | 2 × 2^-32 = 2^-31, about 1 in 2.1 billion | A wrong seed that looks valid: any 16 or 32 bytes make valid BIP-39 words |
| A code damaged past its error correction, right passphrase | 2^-31 | The same |
| A forged or swapped code, without the passphrase | 2^-31 per try: the reader tests two kinds | The same. Without the key, the forger can choose at most which bits flip (§12), not the seed |
| Any other 53-byte code whose first byte is one of the five defined values (5 in 256 of random codes) | Asks for a passphrase, then 2^-31 | Normally "does not open" |

A wrong seed from a false match has a different fingerprint. This is why
a reader shows the fingerprint and a sheet may carry a space to write it,
outside the envelope: a person who keeps it detects every event above.

## 9. Recognition

A payload is treated as ACE when both hold:

1. It is exactly 53 bytes.
2. Its first byte is 0xF8 to 0xFC.

A false match only asks for a passphrase and fails at the tag; only the
tag proves a match. There is no room for a longer marker at 29 × 29: one
more byte needs 33 × 33.

**Why a high marker never reads as KEF.** Checked against Krux's published
KEF specification and `kef.py` on Krux's main branch (fetched 2026-11-04,
links in §16). A KEF
envelope is `len_id (1) || id (len_id) || version (1) || iterations (3) ||
cipher payload`, with `len_id` from 0 to 252. So a KEF envelope's length
is `5 + len_id + payload`, and in 53 bytes:

```text
len_id + payload = 48
```

- `len_id` is at most 48, and that only with an empty payload, which no
  version can decrypt. In Krux's code the shortest payload `decrypt`
  accepts is 10 bytes (version 15), which brings it to 38.
- ACE's first byte is at least 248. A KEF envelope with `len_id` of 248 is
  at least 253 bytes long, and `len_id` of 253 or more is outside KEF's
  range altogether. Krux's `unwrap` reads the version at index
  `1 + len_id`, off the end of 53 bytes, and answers "Invalid format".
- The other way, a 53-byte KEF envelope never starts with 248 or more, so
  an ACE reader never takes one. Length alone would not separate them: a
  version 20 envelope with an empty id is exactly 53 bytes, and starts
  with 0x00.

**Other codes.** 0xF8 to 0xFF never occur in valid UTF-8, so no valid
UTF-8 text code (a descriptor, a word list, a UR or BBQr part) can match.
Compact SeedQRs are 16 and 32 bytes. An RFC 9580 message under a
passphrase is far longer, and begins with a tag 3 packet. A text in
another encoding, such as Latin-1, can start with those bytes: it asks for
a passphrase and fails.

## 10. The passphrase

The passphrase is put in NFC, and Argon2id takes that form's UTF-8 bytes.
Nothing else is done: no character is mapped or refused; compatibility
characters, case and spaces are kept as typed. Two passphrases with one
NFC form are one passphrase. A writer refuses an empty one and requires no
other minimum, since a typed passphrase's strength cannot be measured; it
should offer a generated one (§12's words) and say that the code is only
as strong as its passphrase.

BIP-39 takes its own passphrase in NFKD. ACE's choice differs, on purpose
and only in this: a person who types "ﬁ" and later "fi" has two
passphrases. Whether NFC or NFKD serves a format meant to outlast its
keyboards better is §13, question 8.

## 11. Test vectors

**To be filled:** no vectors exist yet, because no implementation exists.
They will be a JSON file with a README, published with the document and
computed with libraries the format's authors did not write (argon2-cffi
for Argon2id; Python's `hmac` and `hashlib`; a published ChaCha20), so
that they do not rest on a first implementation. Each will give every
intermediate value (`pw`, `PRK`, `K_enc`, `K_mac`, the keystream, `C`,
both tags) and the 53 bytes. They will cover:

- every defined step, each with a 24-word seed;
- a 12-word seed, with its padding bytes given as input;
- a passphrase that is not ASCII, in NFC and in NFD, opening to one result;
- the all-zero and all-one seeds, to show padding and `kind`;
- must-fail cases: a wrong passphrase; a bit changed in each field; a
  reserved step; 52 and 54 bytes; marker 0b11110;
- a must-not-recognise case: a real 53-byte KEF version 20 envelope with
  an empty id, and a KEF envelope from Krux's own tests;
- the QR code of one vector as an image and as its module matrix.

A vector where both tags agree is not offered: finding one takes about
2^32 Argon2id runs, so that branch of §6 is tested by the code's structure
and not by a published input.

## 12. Security analysis

**Offline guessing is the attack that counts.** Anyone who sees the code
holds the salt, the ciphertext and the tag, and can guess for as long as
the seed matters. The cost of one guess is one Argon2id run; it cannot be
shared across codes because of the salt, and it cannot be skipped.

The tag does not change this much. It rejects all but 1 in 2^31 of wrong
guesses at no cost beyond Argon2id. Without it, an attacker would test each
guess by deriving an address (BIP-39's 2,048 rounds, then BIP-32) and
looking it up, which is small beside Argon2id. So the tag saves an attacker
little. It is also not enough on its own: of 2^77.5 six-word passphrases,
about 2^46.5 pass the tag, and each survivor still has to be checked against
the chain. A seed under a BIP-39 passphrase gives no such check.

**The cost per step.** A figure for scale, not an estimate of any real
attacker: 2^40 Argon2 block steps a second, about a million cores each
filling about 1 GiB a second. Memory is what the figure leaves out: at 64
MiB that many cores hold 64 TiB, and at 1 GiB, 1 PiB. Time to try every
passphrase of EFF's long list (12.925 bits a word; half this on average):

| Passphrase | Bits | 64 MiB (5.6 million guesses/s) | 256 MiB (1.4 million) | 1 GiB (350,000) |
|---|---|---|---|---|
| 3 words | 38.8 | 1 day | 4 days | 16 days |
| 4 words | 51.7 | 21 years | 83 years | 331 years |
| 5 words | 64.6 | 160,000 years | 640,000 years | 2.6 million years |
| 6 words | 77.5 | 1.3 billion years | 5 billion years | 20 billion years |

The five steps span a factor of 16, four bits; one more word is 12.9 bits.
The word count matters more than the step. A person-chosen passphrase can
be far weaker than any row. What dedicated hardware actually achieves
against these parameters is §13, question 6. These figures count work
only; they do not measure how much memory costs an attacker, and they are
for today's hardware, while a printed code can last decades.

**The honest limits of a 4-byte tag.**

- **Forgery.** A forger's try succeeds with probability 2^-31. That is low
  only because each try needs a person to scan a code and type a passphrase,
  and to do it again if it fails. A system that retries codes automatically
  with a stored passphrase would make this bound worthless. No such use is
  in scope, and it should not be built.
- **A swapped code.** Someone who replaces a printed code with their own,
  without the passphrase, gets "does not open", except with probability
  2^-31. Someone who knows the passphrase can make any code, and no tag length would help.
- **Bit flips.** `C` is a stream cipher's output. A changed bit changes the
  same bit of the seed, and passes with probability 2^-31. A full tag would
  make this negligible; a wide-block cipher would make the result garbage.
- **Key commitment.** None. An attacker can make a code that opens under two
  chosen passphrases: two Argon2id runs, then about 2^32 HMACs over different
  `C`. Nothing in the format's use tells the attacker which one a person
  typed. §13, question 4.
- **Wrong seed.** A false match gives a wrong, valid-looking seed about once
  in 2.1 billion wrong tries (§8). A person who has the fingerprint detects
  it. A person who does not would see a wallet with no history.

**Other limits.**

- A bad random source at writing breaks the salt (§6.2) and the padding.
- Argon2id's memory access depends on its input after its first half
  pass, as RFC 9106 describes, so a side channel on the reader could reveal
  something about a passphrase. ACE is meant for offline devices.
- A malicious writer can hide data in a 12-word seed's padding or the salt.
  Any format with random fields allows this, and a writer that malicious
  already has the seed.
- Low error correction tolerates less damage than the same bytes at medium
  (§3). A code that is the only copy should be scanned back after printing,
  and is better kept with a second copy.

## 13. Questions for an outside cryptographer

Answers to any of these are wanted, and an answer of "this is wrong" most
of all.

1. **The tag.** Is Encrypt-then-MAC with HMAC-SHA-256 cut to 32 bits sound
   for this use, in which every verification costs a person typing a
   passphrase? It is below RFC 2104 §5's 80 bits and at the floor of NIST SP
   800-107 Rev. 1 §5.3.3 (no less than 32 bits). That document is reported,
   not checked here at the source, to be planned for withdrawal. Does 4 bytes
   do something 0 bytes would not, and is 8 bytes, at a larger code, worth it?
2. **The nonce.** Is a fixed zero nonce under a key derived per salt
   acceptable? Or should the nonce be synthetic (taken from the tag, in the
   manner of SIV), to limit the damage of a repeated salt, at the cost of a
   less simple reader?
3. **Another shape.** Is AES Key Wrap with a 12-byte salt (1 + 12 + 40 = 53
   bytes) better than this? It gives an 8-byte built-in check and no bit
   malleability, against a smaller salt and a stronger test for a guesser.
   How should it carry `kind`?
4. **Key commitment and a chosen code.** Does any realistic use give an
   attacker an oracle that makes the lack of commitment matter?
5. **`kind`.** Is testing two tags, with `kind` in the MAC input and random
   padding, sound? Is a false-match chance of 2^-31 instead of 2^-32 a fair
   price for hiding the seed's length? Should other BIP-39 lengths be in,
   each costing another chance?
6. **Guessing cost.** What do current GPUs, FPGAs and possible ASICs achieve
   against Argon2id at `t` = 3, `p` = 4 at these memory sizes, with published
   time-memory trade-offs counted, and the depth-reducing attacks of Alwen
   and Blocki? Is 3 passes at 64 MiB better spent as one pass at more memory?
7. **Small devices.** Should a step under 64 MiB exist, for devices with 8 to
   32 MiB? How much does Argon2id at that size give over a long PBKDF2?
8. **The passphrase.** NFC, NFKD, or bytes as typed, for a format that may
   be opened decades later on a different keyboard?
9. **HKDF.** Is Expand without Extract right for Argon2id's output, or should
   Argon2id output 64 bytes split in two with no HKDF at all?
10. **Recognition.** Is a 5-bit marker and the length enough? Should the
    marker bits enter Argon2id as associated data? Many libraries do not
    expose that input, and the tag covers it, which is why it does not.
11. **Anything else** in this document that a reviewer would not sign.

## 14. Comparison with KEF and with an OpenPGP QR vault

| | ACE | Krux's KEF, version 20 | The OpenPGP QR vault (`docs/VAULT.md` §1.2) |
|---|---|---|---|
| Key derivation | Argon2id, 64 MiB to 1 GiB | PBKDF2-HMAC-SHA-256, Krux's default 100,000 iterations; no memory cost | Argon2id, 64 MiB to 1 GiB |
| Cipher and tag | ChaCha20, HMAC-SHA-256 cut to 4 bytes | AES-256-GCM, tag cut to 4 bytes | AES-256-OCB, full 16-byte tags |
| Salt | Random, 16 bytes | The `id` field; by default the wallet's fingerprint, in the clear, changeable by the person | Random, 16 bytes |
| Bytes for a 24-word seed (KEF worked out, assuming raw entropy and raw bytes in the code) | 53 | 61 with an 8-character fingerprint id; 69 with a random 16-byte id | 396 |
| Code | 29 × 29, low | 61 bytes: 33 × 33, medium; 69 bytes: 33 × 33 low or 37 × 37 medium | 77 × 77, medium |
| Copied by hand | Plausibly, not yet tried | Plausibly | No |
| Holds | A 12- or 24-word seed | Any bytes, with versions for several | A seed, a codex32 share, a BIP-39 passphrase or a note, up to 226 bytes of text |
| Hides 12 from 24 words | Yes | No: the length shows | Yes |
| Standing | This draft: no implementation, no review | One project's published specification, in use on its devices | An RFC 9580 message (Proposed Standard), which several outside programs open; not yet in the app |
| Small devices | Needs 64 MiB or more | Runs on microcontrollers | Needs 64 MiB or more |

**Could ACE be a version of KEF?** Not at this size. KEF's envelope keeps
its id length byte, version byte and 3 iteration bytes: 1 + 16 + 1 + 3 + 32
+ 4 = 57 bytes, over 29 × 29's 53, with the iteration field meaning nothing
under Argon2id. Offering an Argon2id version to KEF at 33 × 33 is another way
to the same aim, and one the authors of KEF would decide. KEF's strength is
that it runs on small devices; ACE gives that up for memory hardness.

ACE does not replace the OpenPGP form, which has a published standard and
outside readers. It is for when size is the point.

## 15. A printed sheet (not part of the format)

A sheet may print the envelope's values beside the code, so a person can
read them without a scanner:

```json
{"format":"ACE","version":1,"memory_mib":256,"passes":3,"lanes":4,
 "salt":"<32 hex>","ciphertext":"<64 hex>","tag":"<8 hex>"}
```

Every value comes from the 53 bytes, and every one can be worked out again
from a scan, so losing the text loses nothing. The text is never an input to
the cryptography. It also offers a way back from a damaged code: typed in, it
remakes the envelope. A mistyped character looks like a wrong passphrase. The
name holds no word for a seed or a wallet, so a sheet can use it without
saying what it holds.

## 16. The path to a standard

| Step | State |
|---|---|
| A neutral name | Proposed here. Two web searches (2026-11-04) found no format of this name; "ACE" is used in unrelated fields, such as the IETF's ACE working group. To check again before public comment |
| A written specification | This document, a draft |
| Test vectors | §11, not made |
| An outside second implementation | None. Anyone who is not one of the authors may write one from this document alone |
| An outside cryptographic review | None. §13 is what is asked; how a review is recorded is `docs/AUDIT-SUBMISSIONS.md` §10 |
| Public comment | Not begun. KEF went to the Delving Bitcoin forum; a BIP or a SLIP is possible after a review |

Until a review is recorded, the format stays out of Faraday's app: this
specification is Faraday's own, and `docs/STANDARDS.md` §0 keeps such work
in the repository, compiled out. Nothing here is built, so nothing today is
compiled out either. A review that finds the 4-byte tag unacceptable ends the
29 × 29 form and not the aim, since 33 × 33 at medium holds 9 bytes more.

**References.** RFC 9106 (Argon2); RFC 2104, RFC 5869, RFC 8439; RFC 3394,
RFC 5297, RFC 8452, RFC 9580; NIST SP 800-107 Rev. 1; NIST SP 800-38D and its revision's second
preliminary draft (`https://csrc.nist.gov/pubs/sp/800/38/d/r1/2prd`); Krux's
KEF specification
(`https://selfcustody.github.io/krux/getting-started/features/encryption/kef-specifications`)
and code (`https://github.com/selfcustody/krux/blob/develop/src/krux/kef.py`);
the C2SP age specification (`https://c2sp.org/age`); BIP-39; UAX #15; ISO/IEC
18004 (QR codes).
