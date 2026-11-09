# The Faraday vault

**Kind:** Spec
**Status:** Specified (F190, amending F189; F208; F226); the app reads and seals KDBX 4 file vaults, each seal under the vault's own passphrase, at its own size (one of four, until F206 is built) and at the memory step at or above its own; `faraday-pgp` holds the QR vault's OpenPGP message (§1.2), which `faraday-vault`'s `qr` seals and opens as one call and the app does not yet make or open; the first format's account is in `docs/history/VAULT-LOG.md`
**Open:** not built: key files and a YubiKey's typed secret (§3.2, F208: the reader, the composite key, the unlock's two choices, what a seal holds, Key file on the vault's ⋮, their vectors and their checks against `keepassxc-cli` and pykeepass); a file vault that grows by itself up to 256 MiB, with the memory checks, the cheaper size read and the file read in pieces (F206; §2, §5, §8, §9 read as they will be; §3.1's table, §4's limits and `docs/SECURITY-MODEL.md`'s "Fixed size" still describe four sizes up to 4 MiB until then), and the QR vault tool (§13, F226: Make, Open, and Save as a QR vault on a file vault's item)

A vault is one file encrypted under one passphrase with Argon2id, in
one of two published formats (F190):

- **A file vault is a KDBX 4 database**, the format of KeePass, holding
  the person's keys, wallets, entries and notes as KeePass entries.
  pykeepass, `keepassxc-cli` and keepass-rs open what Faraday writes;
  KeePass, KeePassDX and Strongbox read the format and have not been tried
  on it.
- **A QR vault is an RFC 9580 OpenPGP message** under a passphrase,
  holding one item, small enough to print as a code. Sequoia's `sq`,
  GopenPGP, OpenPGP.js and rPGP open it; GnuPG does not, since it
  follows LibrePGP rather than RFC 9580.

A vault therefore opens without Faraday. Faraday writes only the part
of each format it uses (F178, F190 point 11), in `faraday/faraday-vault`
for KDBX and `faraday/faraday-pgp` for OpenPGP, over Faraday's Argon2id,
HKDF and HMAC and vetted ciphers. KeePass's pages (pinned in
`tools/reference/kdbx4/`) and RFC 9580 are what a second implementation
is written against; this file is Faraday's use of them and the names
inside, and the vectors under `tools/vectors/vault/` settle a
disagreement.

The first format (four equal slots under Argon2id and
XChaCha20-Poly1305, `.fvlt`) and the age format F189 chose before it are
read by nothing (F189 point 10). No vault exists anywhere, so nothing is
migrated.

## 1. The file

### 1.1 A file vault: KDBX 4

A file vault is a KDBX 4 database, read and written as KeePass's
specification says. Faraday writes version 4.0, every integer
little-endian:

```text
signature 1 0x9AA2D903, signature 2 0xB54BFB67, version 0x00040000
header fields:  cipher ChaCha20 (RFC 8439), compression none,
                master seed (32 bytes), encryption IV (12),
                KDF parameters: Argon2id ($UUID 9E298B19-56DB-4773-B23D-FC3EC6F0A1E6),
                  S salt (32), M memory in bytes, I passes 3, P lanes 4, V 0x13
                end of header
SHA-256 of the header, HMAC-SHA-256 of the header
HMAC block stream: blocks of 1 MiB, each HMAC-SHA-256, length, ciphertext;
                   then an empty block
plaintext:      inner header (inner random stream ChaCha20, its 64-byte key, end),
                then the XML document (§7), then line feeds to the vault's size (§2)
```

```text
composite key   = SHA-256(SHA-256(passphrase))      -- the passphrase alone
                  or with other components, §3.2
T               = Argon2id(composite key, S, M, I, P), 32 bytes
encryption key  = SHA-256(master seed || T)
HMAC base       = SHA-512(master seed || T || 0x01)
header HMAC key = SHA-512(0xFFFFFFFFFFFFFFFF || HMAC base)
block i's key   = SHA-512(i as 8 bytes || HMAC base)
```

The header is 249 bytes; with its SHA-256 and HMAC, 313. Each block adds
36 bytes (its HMAC and length), and the empty block that ends the stream
36 more. Every field marked secret in §7 is `Protected="True"`,
encrypted with the inner random stream as KeePass does: process-memory
protection in a KeePass app, not part of the file's security, which is
the passphrase, Argon2id, ChaCha20 and the HMACs.

**Recognised by its contents.** A file the person selects is a vault
when it begins with KDBX's two signatures and a major version of 4,
whatever its name or extension. A KDBX 3.1 file (major version 3) is
named "A KeePass file of an older version" and not opened; any other
file that begins with KDBX's signatures and is not a vault (KDBX 5, KDBX 4
of a length no vault has, or cut short) is named "A KeePass file". `.kdbx` is the
name Faraday writes and the one its lists group its own files by. The
wipe asks for its hold on any file named `.kdbx`, `.keyx` or `.pgp`,
whatever it holds.

What Faraday reads beyond what it writes, from a file a KeePass app
saved, is §7.1.

### 1.2 A QR vault: an OpenPGP message

A QR vault is an RFC 9580 message encrypted under a passphrase, binary,
three packets in this order and nothing else:

```text
SKESK v6 (tag 3):  version 6, AES-256 (9), OCB (2),
                   S2K Argon2 (type 4): salt (16), passes 3, lanes 4, memory exponent,
                   OCB IV (15), the encrypted session key (32) and its tag (16)
SEIPD v2 (tag 18): version 2, AES-256, OCB, chunk size octet 10 (64 KiB), salt (32),
                   then the encrypted chunk and its tag (16), then the final tag (16)
  plaintext:       one literal data packet (tag 11): format 'b', no file name,
                   date 0, then the QR TOML of §2.1, 226 bytes
```

```text
S2K output  = Argon2id(passphrase, salt, memory = 2^exponent KiB, passes, lanes), 32 bytes
KEK         = HKDF-SHA-256(ikm = S2K output, salt = empty, info = 0xC3 || 6 || 9 || 2)
session key = OCB-decrypt(KEK, IV, the encrypted session key, ad = 0xC3 || 6 || 9 || 2)
message key, IV prefix = HKDF-SHA-256(ikm = session key, salt = the SEIPD salt,
                                      info = 0xD2 || 2 || 9 || 2 || 10), 32 + 7 bytes
chunk i     = OCB(message key, IV prefix || i as 8 bytes, ad = the info bytes)
final tag   = OCB over nothing, nonce for the next index,
              ad = the info bytes || the plaintext's length as 8 bytes
```

RFC 9580 §5.3, §5.13.2 and §3.7.1.4 are the definitions; the lines above
are a summary. The SKESK packet is 90 bytes, the SEIPD packet 306 with a
226-byte plaintext: 396 bytes in all, QR version 15 at medium error
correction (§2.1). GCM would save 3 bytes; OCB is the one AEAD RFC 9580
requires every implementation to have.

**Recognised by its contents.** A scanned code or a file is a QR vault
when it parses as exactly those packets, with lengths in any form RFC
9580 allows. A SKESK v4 or a SEIPD v1 (the older OpenPGP that GnuPG
writes), a public-key encrypted session key, a second SKESK, a
compressed data packet, a signature, or anything after the SEIPD
packet (padding packets, tag 21, are ignored, as RFC 9580 §5.14
requires, after the SEIPD packet and after the literal data), is named
"An OpenPGP message Faraday does not open" and not opened. The reader
accepts any literal date and any chunk size octet from 0 to 16 (several
chunks), as RFC 9580 requires; Faraday writes date 0, one chunk, octet
10 and no padding. The S2K's cost is checked right after the SKESK, before
the SEIPD packet is read. Faraday writes the binary form, never ASCII armor, and reads the
binary form only; `.pgp` is the name it writes.

## 2. Sizes

A QR vault has one length (§2.1). A file vault has no size to choose
(F206): it starts at 32 KiB of plaintext and grows by itself, each seal
padding it to the smallest multiple of 32 KiB that holds its contents, up
to 256 MiB. The plaintext is the inner header, the XML document (history and a recycle bin are in it, so they count), the
bytes of the attachments something refers to (§7.1) and the line feeds after the document. It shrinks in
the same way when what it holds does: a seal remembers nothing of the
size before.

| Vault | Plaintext | File | Holds |
|---|---|---|---|
| QR (§2.1) | 226 B, one item or a note | 396 B, an OpenPGP message | one seed, one codex32 share, one passphrase or a short note, printed as a code |
| New, or small | 32,768 B | 33,153 B | keys, wallets, a few entries |
| Larger | a multiple of 32,768 B | the length below | whatever it holds |
| The largest | 268,435,456 B (256 MiB) | 268,445,021 B | a KDBX file with its attachments |

A file vault's length is the 313 bytes of §1.1's header, its hash and
its HMAC, the plaintext, and 36 bytes for each 1 MiB block and the empty
block after them. The plaintext is the inner header, the XML document
and the line feeds after it (history and a recycle bin are in the document,
so they count). A reader opens a file whose plaintext is up to 256 MiB of
any length, as a KeePass app's save is (§7.1), and refuses a longer one
from the file's length, before any work, and a compressed one that states
more than 256 MiB once it is decrypted. The next seal pads whatever it
opened to the smallest step that holds it. A file made when a vault had
one of four sizes (64 KiB to 4 MiB) is already on a step, and takes the
same rule at its next seal, so one that holds little gets smaller.

**Memory comes before size** (F206 point 4). A file within the ceiling
may not fit the computer, and an allocation that fails ends the app and
every open vault's unsealed changes. So what the system has free (§3.1) is
checked, against one function, before the work and with no memory taken:

- **Before a file comes in from a stick:** twice the file, the app's copy
  and the one that carries it across a lock. The length is in the listing.
  A vault over 64 MiB is listed unticked; one that does not fit cannot be
  ticked and says "Too large for this computer's memory".
- **Before an unlock:** Argon2id's memory, twice the file and 100 MiB
  (`APP_MIB`). Unlock says "Needs 1.6 GiB. This computer has 1 GiB free."
  before the passphrase is typed.
- **Once it is decrypted, and before a seal:** Argon2id's memory, 16 times
  the document, twice the attachments and 100 MiB. Measured on 256 MiB of
  real entries, the peak is 15 times the document; on one 256 MiB
  attachment, twice. A file that does not fit stays locked, with the same
  message. A seal that does not fit stops a lock as §5 says.
- **A compressed file** states its plaintext only inside the encryption:
  once decrypted, that stated length, at most 256 MiB, takes the file's place
  in the same function before its one buffer is allocated. It cannot
  inflate past that buffer.

A machine that does not say what it has free opens a file of at most
4 MiB, and only at 64 MiB of Argon2id (§3.1). A 32-bit build counts at most
2 GiB free, whatever the machine has.

The padding is line feeds after the document's closing tag, which XML
allows ("Misc" after the root element) and every KeePass app reads:
KeePassXC 2.7.12 and pykeepass 4.2.0 open a padded file, and KeePassXC
keeps Faraday's custom data when it saves one. A KeePass app's save
drops the padding (KeePassXC writes a random-length value of its own,
`KPXC_RANDOM_SLUG`, in its place), so the length of a file saved there
says more until Faraday seals it again.

### 2.1 The QR size

A QR vault is an OpenPGP message (§1.2) and holds one item, a note, or
nothing yet (F189 point 6, F190 point 5): a
seed's words (12 or 24 words of the English list, as a SeedQR's are; a
Seed XOR part is 24), a codex32 share, a BIP-39 passphrase with the
fingerprint of the key it belongs to, or any text typed into it. No
item is longer than a 24-word seed: a passphrase longer than that, a
seed of another list or word count, and a longer text go in a file
vault.

Its contents, the literal data of §1.2, are the item's keys at the top
of a TOML v1.0.0 document, a rule of Faraday's own (F189 point 6, kept
by F190 point 5), written one to a line: a seed is `words = "…"`; a codex32
share is `codex32 = "…"` and `for = "…"` (the fingerprint of its key);
a passphrase is `passphrase = "…"` and `for = "…"`; a note is
`note = "…"`. At most one of `words`, `codex32`, `passphrase` and
`note` is present: a QR vault with none of them is empty. A QR vault has
no seal count and no identifier, and every seal is a new code (§13). The text is padded with line feeds to the
one length every QR vault has, 226 bytes, so a code does not show what
it holds, or whether it holds anything. That length is the line of a
24-word seed of the list's longest words: the start `words = "` is 9
bytes, the words 215, the closing quote and the line feed 2. A 12-word
seed is padded to it. The longest passphrase is 193 bytes of TOML text
(`QR_PASSPHRASE_MAX`): the `passphrase` line is 13 bytes and the text's
quotes and line feed 3, the `for` line 17. The longest note is 216
bytes of TOML text (`QR_NOTE_MAX`): the start `note = "` is 8 bytes and
the closing quote and line feed 2; a quote, a backslash or a line
break in the text takes two bytes as its escape, and a character
outside ASCII its UTF-8 length. The OpenPGP message is 170 bytes more
than its plaintext, 396 bytes (§1.2); the code is version 15 (77 × 77
modules) at medium error correction, which holds 412 bytes in byte
mode.

A QR vault is not kept open (§13, F226): the tool in Tools makes one
from a note or a loaded key, or from a file vault's item, and opens one
to hand what it holds where that item goes; it is never listed on Vaults.
Every code is made whole; a code printed before still opens, to what it
held when it was sealed, so a person who makes a new one prints it and
destroys the old. It is not a secret
under `docs/QR.md` §7.1: it is drawn as any code is, and offered as a
labelled PNG (`docs/QR.md` §4 items 3 and 4) whose label lines are "QR
vault", the vault's name and the date it was sealed, never what it holds.
A code of 77 × 77 modules is not copied by hand: its paper is that PNG,
carried on a stick to a computer with a printer, which may be online. The
code holds no name and no date; only the label does.

Scanned wherever a SeedQR is taken, it opens the tool's Open card (§13),
which asks for its passphrase and hands its words to the seed sheet, a
share to Add a key's codex32 form, a passphrase to the seed it belongs
to, found by its fingerprint, and a note to an open file vault. Faraday
Starter does not scan or print a QR vault.

## 3. Keys

The passphrase is put in Unicode NFC (Unicode Standard Annex #15), and
both formats take the NFC form's UTF-8 bytes: the KDBX composite key is
made from them (§1.1) and the OpenPGP S2K takes them as its passphrase
(§1.2). Two passphrases with one NFC form are one passphrase (F159
point 2). Nothing else is done to it: no character is mapped or refused,
and compatibility characters ("ﬁ", "Ａ"), case and spaces are kept as
typed. KeePass apps and `sq` do not put a passphrase in NFC: a vault
whose passphrase has a character with two forms opens there only when
typed in NFC, which the vectors show (§10).

Argon2id is version 1.3 (0x13) with 3 passes and 4 lanes in both
formats (F190 point 6), no secret key and no associated data. Faraday
writes five memory steps:

| Memory | KDBX `M` (bytes) | OpenPGP exponent | Bits added |
|---|---|---|---|
| 64 MiB | 67,108,864 | 16 | ≈17.6 |
| 128 MiB | 134,217,728 | 17 | ≈18.6 |
| 256 MiB, the default | 268,435,456 | 18 | ≈19.6 |
| 512 MiB | 536,870,912 | 19 | ≈20.6 |
| 1 GiB | 1,073,741,824 | 20 | ≈21.6 |

**Cost limits.** A reader checks the stated cost before it allocates
anything, and refuses outside: memory 8 MiB to 1 GiB, passes 1 to 100,
lanes 1 to 64, Argon2 version 0x13, and for KDBX no secret key (`K`) or
associated data (`A`) in the KDF parameters. Faraday writes 3 passes;
the wider reader limits exist so that a database KeePassXC made, whose
passes it times to a second on the computer that made it, and its lanes
to that computer's threads, still opens. A KDBX file under Argon2d opens
as well (§7.1). The cost is in the clear header of both formats, so the
memory a vault needs, and its passes, are shown before its passphrase
is asked for, with the unlock time here worked out from them where
passes above 3 make it longer than the step's. Argon2id runs on the
worker (F184), so the screen is drawn while it works.

### 3.1 Choosing a cost

A vault opens only on a machine that can allocate its memory, so the
step is chosen for the weakest machine the vault will ever be opened on,
not for the machine it is made on. It can be changed on an open vault
(§8), and the next seal is written at the new step.

**Memory is a hard limit, time is not.** A machine without the memory
cannot open the vault at any speed. A slow machine opens it slowly.

What a machine has free is Linux's `MemAvailable`, Windows' available
physical memory, and on macOS the free, inactive and speculative pages
`vm_stat` prints times its page size.

**A machine that does not say what it has free** (where that cannot be
read) unlocks and makes only vaults of 64 MiB, which opens anywhere:
Unlock says so for a vault of more and does not start, and Create
suggests 64 MiB and refuses a larger step, naming it. Going ahead on a
guess could take more memory than the machine has.

**The passphrase matters more than the cost.** Doubling the memory
doubles the work of every guess, which is worth one bit of passphrase
strength. The five steps span a factor of 16, four bits. One more word
from the EFF long list is 12.9 bits. A six-word dice passphrase (77.5
bits) is out of reach at any step here; a short or reused passphrase is
not protected by any step here. The Create form says this beside the
step.

**The bits shown.** The Create form states what each step adds as
log2(memory in KiB × passes), the number of Argon2id's 1 KiB block
steps in one guess: ≈17.6 at 64 MiB to ≈21.6 at 1 GiB. That is an estimate:
it counts work only, not how much harder memory is than time for an
attacker's hardware. A passphrase the dice made has its own bits exactly
(words × log2 of the list's length, shown rounded to a tenth). The form
shows the two added as the passphrase's strength while the field still
holds the dice's words. Every figure that includes the step's bits is
written with ≈, never =, and the dice's own bits without it. A typed
passphrase's own bits are not measured, and the form says so.

**Memory on the Create form.** The form's **Memory** card lists the five
steps with 256 MiB chosen, each with the machines it opens on (below)
and its unlock time here, or what this computer has free where that
falls short. **Where will you open it?** marks the largest step that
fits the weakest machine chosen as suggested. **This computer** is
ticked by default and can be unticked; it gives a vault the largest step
that fits in what it has free beside twice the vault file and 100 MiB
(below). Another machine gives half its memory:

| Memory | Memory free to open | Opens on |
|---|---|---|
| 64 MiB | 165 MiB (172 MiB) | anything, a 512 MB Pi Zero 2 W included |
| 128 MiB | 229 MiB (236 MiB) | a Pi Zero 2 W and anything larger |
| 256 MiB | 357 MiB (364 MiB) | a 1 GB Raspberry Pi 3 and anything larger |
| 512 MiB | 613 MiB (620 MiB) | a 1 GB Pi 3 with little else running; any PC |
| 1 GiB | 1,125 MiB (1,132 MiB), about 1.1 GiB | a Pi 4 or 5 with 2 GB or more; a PC with 2 GB or more |

The figures are for the default size's 256 KiB file, and in parentheses
for the largest, 4 MiB (§2). A vault meant for a Pi Zero 2 W takes 64 or
128 MiB.

**Where the numbers come from.** RFC 9580 recommends Argon2id with 3
passes, 4 lanes and 64 MiB where 2 GiB cannot be spared, and 1 pass, 4
lanes and 2 GiB where it can; RFC 9106 gives the same two choices;
KeePassXC makes Argon2 databases at 64 MiB. 64 MiB is the step a Pi Zero
2 W opens with room to spare and RFC 9580's second choice as it stands;
the steps above it keep its passes and lanes and raise its memory. 2
GiB is past what a Pi 3 or a 2 GB Pi 4 can give. Lanes do not change
the work of a guess: four lanes of a quarter of the memory each are the
same blocks. Faraday computes them one after another, so a multi-core
machine opens a vault no faster under Faraday; a KeePass app or `sq`
that runs them on four threads opens it faster.

"Memory free to open" is Argon2id's memory, plus twice the vault file
(the file and a sealed copy beside it while it is written), plus 100 MiB
(`APP_MIB`) for the contents and the app. One function in
`faraday-core` gives it from the memory and the size, as
`vault_need_mib` does today; Unlock's check, Create's choice for **This
computer** and the import rule of `docs/LARGE-FILES.md` §4.1 all use
it. On Faraday the whole system is a few tens of megabytes in RAM; on
another operating system, or in a KeePass app or `sq` on a desktop, the margin is
whatever that system leaves free.

Two cautions are not settled by the table and are measured before
release: the Pi 3 image is 32-bit, where one allocation of 512 MiB or 1
GiB may fail even with the memory free; and the time column below is
empty until measured.

**Time.** Faraday states the unlock time of each step on this computer.
Times on other machines come from the reference table, filled on
reference hardware and published with each release:

| Machine | 64 MiB | 128 MiB | 256 MiB | 512 MiB | 1 GiB |
|---|---|---|---|---|---|
| Raspberry Pi Zero 2 W | to be measured | to be measured | — | — | — |
| Raspberry Pi 3B+ | 2.1 s | to be measured | to be measured | 17.2 s | — |
| Raspberry Pi 4 (4 GB) | to be measured | to be measured | to be measured | to be measured | to be measured |
| Raspberry Pi 5 (8 GB) | to be measured | to be measured | to be measured | to be measured | to be measured |
| Recent x86-64 laptop | to be measured | to be measured | to be measured | to be measured | to be measured |

The Pi 3B+ figures are one unlock each, timed on the worker at 1400 MHz, 3 passes. The Zero 2 W, 4 and 5 stay to be measured.

The Raspberry Pi rows come from the dev card's timings: a Pi booted from
a `dev=1` image writes each vault made and unlocked into
`faraday-timings.txt` as a `work` row naming its memory and how long it
took (`docs/DEVELOPING.md` §6.2), and a step the board could not
allocate as "failed: no memory". Faraday runs the lanes on one core
(above); the Pi's cores are much slower than a laptop's.

**What the form enforces.**

- Memory above what this machine can allocate is refused: making a vault
  runs its Argon2id, so a vault this machine could not open is never
  written.
- Memory above what the weakest machine chosen gives a vault (above) is
  a warning naming that machine.
- An unlock time above 10 seconds on the slowest chosen machine, by the
  reference table, is a warning.
- The summary panel always shows the unlock memory and the unlock time
  here. A window too low to hold the panel whole has the summary and
  Create vault in the page after the last step instead.

**When a vault does not fit.** Opening a vault whose memory this machine
cannot allocate (its memory free to open, above) says so before the
passphrase is typed: "Needs 1.1 GiB. This computer has 900 MiB free."
It does not start Argon2id.

### 3.2 A key file and a YubiKey's secret

A file vault may need more than its passphrase (F208). KDBX 4 records
nothing that says so, and Faraday writes nothing that does. The passphrase
is always one of the components: a file with none is refused (§7.1).

```text
composite key = SHA-256( SHA-256(passphrase)
                         || key file's key          -- 32 bytes, when there is one
                         || SHA-256(response) )     -- 20 bytes hashed, when there is a secret

challenge = S, the Argon2 salt of the file's own header
padded    = S, then 64 - len(S) bytes of that value, to 64 bytes
message   = padded, less its last byte and every equal byte before it
response  = HMAC-SHA-1(secret, message)
```

The key file's key and the secret's response are both inside the composite
key, so both are stretched by Argon2id with the passphrase. A key file's
key does not change between seals. The response does, since the salt does.

**A key file** is read in this order, as KeePass does:

1. **XML:** a file the vault's XML reader reads whole, whose root is
   `KeyFile`. `Meta/Version` of `1.0` or `1.00` gives `Key/Data` as Base64;
   `2.0` or `2.00` gives it as hexadecimal, white space ignored, with the
   `Hash` attribute required and equal to the first four bytes of the key's
   SHA-256. A `KeyFile` document with another version, no key, a key that
   is not 32 bytes, a value that does not decode or a hash that fails is
   refused, naming the file: "This key file is damaged."
2. **32 bytes:** a file of exactly 32 bytes is the key.
3. **Hexadecimal:** a file of exactly 64 characters, each `0-9`, `a-f` or
   `A-F`, is the key they spell.
4. **Any other file:** its SHA-256 is the key. A document that is not well
   formed, or has another root, or a DOCTYPE, is such a file.

An empty file and a file beginning with KDBX's signatures are refused as
key files. The file is read within the memory check of a file's read, and
its bytes are wiped once the key is made.

**A YubiKey's secret** opens a KeePassXC file whose key was programmed for
HMAC-SHA-1 challenge-response with variable input (`hmac-lt64`, what
`ykman` writes). It is 40 hexadecimal digits, spaces and colons ignored;
anything else is refused before Argon2id runs ("A YubiKey secret is 40
hexadecimal digits. This has 38."). A key programmed for fixed 64-byte
input is not covered, and reads as wrong credentials.

## 4. Opening

**A file vault.**

1. Read the header as KDBX 4 says: the signatures, the version, the
   header fields. Check what §7.1 allows: the cipher, the compression,
   the KDF and its cost (§3). The file's length gives a plaintext of at
   most 4 MiB (§2). Any fault refuses the file, naming it, before any
   memory is taken.
2. Show the memory the vault needs, and stop if this machine has not
   got it (§3.1).
3. Take the passphrase, and whichever of **Add a key file** and **Add a
   YubiKey secret** the person used (§3.2). Each is optional, since the
   file does not say what it needs. A key file is chosen from those a
   stick holds and read through the disk process. A faulty key file or
   secret stops here, before any memory is taken for Argon2id.
4. Put the passphrase in NFC, make the composite key (§3.2) and run
   Argon2id on the worker.
5. Check the header's HMAC. A HMAC that fails means the credentials are
   wrong (the header's SHA-256, checked first, tells a damaged header
   from wrong credentials), and the message says what was tried:
   "Wrong passphrase, or this file also needs a key file or a YubiKey
   secret." with neither; "Wrong passphrase or key file.", "Wrong
   passphrase or YubiKey secret." or "Wrong passphrase, key file or
   YubiKey secret." with them. It cannot say which was wrong. The HMAC
   keys come from the stretched key, so whichever credentials open a
   file, one content verifies.
6. Check every block's HMAC, then decrypt the stream into memory held by
   `faraday-keys`, wiped as it drops; inflate it if it is compressed,
   stopping at 4 MiB. Any failure refuses the file, and nothing of the
   contents is read.
7. Read the inner header and the XML (§7, §7.1). Any fault refuses the
   file, naming it.
8. Compare the seal count with what the session has seen (§11).

Neither the key file, its name, nor the secret is remembered once the
vault is locked (§11).

**A QR vault.**

1. Parse the three packets (§1.2) and check the S2K's cost (§3). Any
   fault refuses the code, naming it, before any memory is taken.
2. Show the memory, as for a file vault.
3. Put the passphrase in NFC and run Argon2id on the worker. Decrypt the
   session key: a tag that fails means the passphrase is wrong.
4. Decrypt the chunk and check its tag and the final tag, into memory
   held by `faraday-keys`. Any failure refuses the code.
5. Read the literal data packet and the QR TOML (§2.1). Any fault
   refuses the code, naming it.

Several vaults may be open at once.

A backup plan puts one seed in each vault by default, each vault on its
own stick at its own place (2026-10-10, `docs/history/NEW-WALLET.md` §5,
`docs/WALLETS.md` §5): a vault per seed, so no place and no passphrase
reaches more than one key unless the person ticks more seeds into one.

## 5. Sealing

Sealing writes the open vault back to memory as a file, and every seal
is a new file (F190 point 8), as both formats expect: KeePass draws a
new master seed and IV at every save, and RFC 9580 a new salt for every
S2K and every data packet.

- **New random bytes.** For a file vault, the master seed (32 bytes),
  the encryption IV (12), the Argon2 salt (32) and the inner stream key
  (64) are one `fresh_bytes("vault seal", 140)`, five answers of the
  system's randomness (F154); new entries' and groups' UUIDs (16 each) are
  one more answer of the seal, expanded by SHA-256 over a domain string, the
  answer and a counter. For a QR vault, the S2K salt (16), the OCB IV (15),
  the session key (32) and the data packet's salt (32) are one
  `fresh_bytes("qr vault seal", 95)`, three answers; so whatever the person
  added through Add randomness is mixed in (`docs/OWN-RANDOMNESS.md`
  §2.3). A seal waits for the answers it needs and never stretches
  fewer. Nothing is sealed before they arrive: a lock with a changed
  vault open then seals nothing and says so, and the vault stays open
  with its changes.
- **Argon2id runs again**, at the vault's step, on the worker (F184): a
  new salt is a new key. The passphrase is kept while the vault is
  open, in `faraday-keys`, wiped as it drops: it is what a seal needs
  that the contents do not hold. So are a key file's key and a YubiKey's
  secret (§3.2), never shown back, so a seal asks for nothing and the
  file it writes needs what the copy opened needed. With a secret, the
  response is computed for the new salt, so the file still opens in
  KeePassXC with that YubiKey or another holding the same secret; a salt
  drawn ending in `0x20` has that byte changed to `0x21`, so no seal rests
  on the key's padding rule. A lock that seals a vault waits for its
  Argon2id.
- **The file is read back before it is given.** The document the seal
  wrote is read again and written once more, and the bytes must be the
  same, so a document that does not parse, or that the reader takes
  otherwise than it was written (a protected value out of step with the
  inner stream), is caught; a value the writer got wrong in a way that
  still reads back the same is not. Then the file made is opened again with the keys it
  was made under, which checks its header, every block's HMAC, the
  decryption and the inner header, and the plaintext must be the one
  written. A file that fails is not given: nothing goes to the Outbox,
  a toast says the file did not read back (or, where the computer could
  not hold the file to read it, that it lacks the memory), and the vault
  stays open with its changes (an idle lock loses them, as for any seal
  that cannot be written). It does not run Argon2id a second time, so a key
  derived from the passphrase is not derived again: a wrong derivation
  would not be seen here. It does not compare the file with what
  another KeePass app reads.
- **A seal goes beside a file, never over one that is not its own.** The
  file a seal makes takes the vault's file name. A file of that name
  waiting in the Outbox is replaced only when it is an earlier copy of the
  same vault, told by its SHA-256 being among the copies the session
  remembers of that vault (§11); any other file of that name, such as a
  KeePass export, stays, and the seal is put beside it as `name-2.kdbx`
  (the first name free, or the earlier copy of this vault that holds it).
  A file the app cannot tell is a copy of this vault is treated as not
  one, and a vault opened from a copy older than one seen (§11), or sealed
  as often as one with other changes, replaces only the file it was opened
  from: the other copy stays, and the seal goes beside it. A name beside
  also stays clear of the files in Files.
- **The seal count is one higher** (§7, `faraday.seals`), with the time
  by this computer's clock when it is known, written by the vault crate
  alone. The app keeps the highest count it has seen of each vault
  (§11); a vault sealed in the session is seen at the count its seal
  wrote, one higher than the copy opened, so the copy opened is older
  than it once the vault is sealed; a backup plan can carry the counts
  of other vaults across a power-off (§7, `plan`).
- **The contents are padded to the smallest step that holds them** (§2).
  A vault's changes stop a lock in two cases only: the contents are past
  the 256 MiB a vault holds, or this computer has not the memory to seal it
  (§2). Entries read from the camera or from files, a form's edit and a
  copy from another vault are refused the same way past the ceiling, and
  none is added. The time an edit takes does not depend on the file's size:
  the document's bytes are measured once when it opens, and each change adds
  an upper bound of its own; only past the ceiling is the document written
  to count exactly (F206 point 8). The transaction's signed amounts (§7)
  lose their oldest only at the ceiling. In either case the lock does not
  go ahead, nothing is wiped, the vault stays open with its
  changes and the Lock sheet names the vault and says what stops it: by how
  many bytes it is over what a vault holds, or the memory the seal needs and
  what this computer has free. The check is made again as each
  vault is sealed, since the `seals` lines a plan takes from the vaults
  sealed before it can grow it, and a seed or answers that run out do
  the same: the lock stops there, the vaults sealed before it are closed
  with their files in the Outbox, and that vault and the ones after it
  stay open with their changes. A flow's state and Make a new wallet end
  only once every seal is done, so a lock that stops leaves them as
  they were. "Lock without sealing" is
  the person's own choice to lose those changes. That rule is for a lock
  the person asks for: an idle lock has no one to ask, so it goes ahead
  and wipes, those changes are lost, and the Locked sheet says that
  changes were not saved, too big or not writable. Locking one vault alone
  (a backup's "Lock it and make a new vault") refuses the same way with
  a toast.

Sealing happens on every lock (`docs/history/PLAN.md` §5.3), and for one
vault alone when a backup's vault item locks it to make the next vault
("Lock it and make a new vault"); that vault closes and the rest stay
open. A QR vault is never open, so never sealed at a lock: the tool seals
each code once, when it is made (§13). The sealed file goes For the stick
(Files' half that a stick visit writes). With more than one vault file
For the stick, a visit ticks one, the first the last write did not
write, so each vault goes to its own stick.

## 6. Writing back

A vault is named when it is made (owner, 2026-10-07): its file is that
name, letters, digits, hyphens and underscores, spaces made hyphens,
with `.kdbx`, or `.pgp` for a QR vault; left empty it is `vault.kdbx`
(`qr-vault.pgp`). A name taken already becomes
`name-2.kdbx`, as backup files are named. A name can say what a vault
holds to anyone who sees the stick; leaving it empty keeps the old rule.

The name is not the vault's identity, and neither are its bytes' header:
every seal has a new salt, so a copy cannot be told from another vault
without opening it (F190 point 8). The session keeps, for each vault it
opened or sealed since power-on, the SHA-256 of every copy of it it
read or wrote: the copy opened, and each seal. A file on a stick is a
copy of that vault when its SHA-256 is among them. Writing back
replaces that copy; no older copy is kept. On a stick visit:

1. Find the file on the visited stick whose SHA-256 is one of the sealed
   vault's copies. None: write it as a new file. More than one: refuse
   and name both. None, and a file has the name asked for, ignoring case:
   nothing is written until the person answers (F203): **Overwrite**, by a
   hold, naming the SHA-256 of the file shown, which the disk process
   refuses if the file has changed since; or **Save beside**, as
   `name-2.kdbx`. A file that cannot be read for its SHA-256 has no
   overwrite. The visit's Already on the stick sheet decides nothing for a
   vault. The visit holds no passphrase and opens nothing: a
   process that has unlocked a vault is never handed a stick
   (`docs/history/PLAN.md` §5.1). Counts are compared at unlock instead
   (§11).
2. Write the sealed vault as a new file beside it, read it back and
   compare byte for byte.
3. Delete the original and rename the new file to the name it is
   written under: the original's, unless the person renamed the file
   For the stick since a visit last wrote it (`docs/FLOWS.md` decision
   61), when it takes the new name, or `name-2.kdbx` where another file on
   the stick has that.
4. FAT has no atomic replace. If a visit finds a new file that reads back
   whole and no original, it completes step 3; if it finds both, it
   keeps the original and discards the new file.

Once written and matched, the vault leaves For the stick and stays in
Files as the stick now holds it, and the visit's receipt keeps the
SHA-256 of the bytes written (owner, 2026-10-10;
`docs/history/SIMPLIFY.md` §4.3) and the stick's label and FAT volume
serial. The backup flow (`docs/BACKUP-FLOW.md`, F31) does not tell
sticks apart by the serial, which sticks written from one image can
share; it refuses a second copy on the stick holding the first, by the
file's SHA-256. The receipt is kept across a lock under the key
`receipt` and is gone at power-off. From it each vault file's currency
is computed, never stored: **Never written** (sealed For the stick, on
no receipt), **On {label} · current** (the receipt wrote the bytes it
has now), **Changed since written** (open with unsaved changes, or
sealed bytes that differ from the receipt's). A vault copied in,
unchanged and on no receipt, reads **On {label} · current** for the
stick it was read from, which the app remembers with the SHA-256 of the
bytes read (kept across a lock under the key `vault-from`, gone at
power-off), or **Unchanged** when that stick is not known.

## 7. Contents

A file vault's contents are a KDBX 4 XML document (F190 point 3): one
root group named as the vault, the person's entries in it and in any
groups under it, and Faraday's items as entries in one group per kind.
Each item is in its published text form. Which names mean what is
Faraday's own rule, written here, and fails closed for Faraday's items:
an unknown kind, a field its kind does not name, a missing required
field or a malformed value makes that item Unreadable (F194): the file
still opens, and the item is named, never loaded or used and written
back unchanged.

**Custom data.** KDBX 4 gives the database and each entry a list of
custom data, key and value pairs, inside the encryption. Any KDBX
reader with the passphrase reads it; KeePassXC keeps it when it saves
and does not show it in its views. Faraday's keys begin `faraday.`;
other keys (KeePassXC's `KPXC_RANDOM_SLUG` and `_LAST_MODIFIED`, a
plugin's) are not read and are kept as written (§7.1).

The vault's own values are the database's:

| Where | What |
|---|---|
| `Meta/DatabaseName` | the vault's label (≤ 64 bytes), shown on Vaults while open. Starts as the vault's name, or "Vault" when none is typed. |
| `faraday.format` | `1`. Written by Faraday; missing or not 1, see **A file that is not Faraday's** below. |
| the root group's `UUID` | the vault identifier (owner, 2026-11-02): KDBX's own 16 bytes, drawn once when the vault is made (`fresh_bytes("vault id", 16)`) and kept for its life; KeePass apps keep a group's UUID when they save. Written as 32 lower-case hex digits wherever Faraday names it. By it a backup plan names another vault (`plan`, below); it is not a secret, and only whoever opens the vault reads it. |
| `faraday.seals` | how many times the vault has been sealed, a decimal integer, 1 or more. Written by `faraday-vault` alone: each seal writes one more than the vault was opened with (or than the last seal of the same opening). |
| `faraday.sealed` | the time of the last seal, an RFC 3339 date-time in UTC (`2026-10-28T14:02:00Z`) by the clock of the computer that sealed it; absent when that clock was not set. |
| `faraday.signed.{tx}` | one per transaction: `{tx}` the SHA-256 of an unsigned transaction in lower-case hex, the value the amount each input stated, in satoshis, comma-separated, in order. Written by `docs/WALLETS.md` §3.3 when that setting is on; the oldest are dropped first when the vault is full. |
| `faraday.plan.{n}` | one per backup plan, `{n}` from 1: the plan as a TOML v1.0.0 document of F189's `plan` keys (`wallet`, `answers`, `places`, `holds`, `marks`), read by the TOML reader already written. `answers` carries `seals {id} {count} {digest}` for each other vault the plan places whose copy the session has seen: that vault's identifier, its seal count and the SHA-256 of the file at that count, never the plan's own vault. Never a secret. Written when the backup's checklist is made with a vault open and when the plan is changed; the next backup of that wallet starts from it. |

**An entry** is a KeePass entry with no `faraday.kind`: the person's
password entries, whether Faraday made them or a KeePass app did.

| Field | What |
|---|---|
| `Title`, `UserName`, `Password` (protected), `URL`, `Notes` | as KeePass has them; an empty title reads "No title" on screen |
| `otp` (protected) | an `otpauth://` URI (the Key URI Format), as KeePassXC writes it (`docs/TOTP.md` §4.3); the only TOTP field Faraday writes. One it cannot read shows the Unreadable badge on the codes page and the entry; it never refuses the file |
| any other field | kept as written, with its protection, and shown on the entry's page; KeePass 2's `TimeOtp-*` fields and older KeePassXC's `TOTP Seed` and `TOTP Settings` are such fields, and are also read for the codes page (`docs/TOTP.md` §4.1), never rewritten |

An entry keeps its UUID, and the group it was in, by the path of group
names under the root; Faraday makes an entry in the root group.

**Faraday's items** are entries whose custom data names their kind,
`faraday.kind`. Faraday writes each in its kind's group under the root
and reads it in any group. Fields a kind does not name, and KeePass's
standard fields it leaves empty, must be absent or empty:

| Kind (`faraday.kind`), group | Record type | Fields, and the published form of each | Custom data |
|---|---|---|---|
| `seed`, Bitcoin keys | 2 | `Title`: its label, or "Key {fingerprint}". `Password` (protected): the BIP-39 words of its list, one space between; or `hex` (protected): a BIP-32 seed of 16 to 64 bytes in lower-case hex; exactly one of the two. `list`: a list other than English (`japanese`, `korean`, `spanish`, `chinese-simplified`, `chinese-traditional`, `french`, `italian`, `czech`, `portuguese`). `passphrase` (protected): its BIP-39 passphrase, present only when the person chose to keep it with the seed. | `faraday.load` `false` when it is not loaded at unlock (F62); `faraday.held` `true` while it was made in Faraday with its backup not done (`docs/history/NEW-WALLET.md` §14.3); `faraday.codex32` and `faraday.child`, below |
| `wallet`, Wallets | 3 | `Title`: its name. `policy`: its BIP-388 wallet policy, or `descriptor`: its descriptor (BIP 380) for a wallet whose key has no origin; exactly one. | `faraday.load` as a seed's |
| `note`, Notes | 4 | `Title`: "Note". `Notes`: the text. | |
| `sheet`, Recovery sheets | 5 | `Title`: its name. `descriptor`. `Notes`: the sheet's note. The three together at most 16 KiB. | |
| `gpg`, GPG keys | 7 | `Title`: the key's primary user ID. `key` (protected): the armored OpenPGP secret key as `gpg --export-secret-keys --armor` writes it, unprotected; required. `public`: the public certificate as last exported, armored. `revocation` (protected): the revocation certificate made with the key (F32), armored. `revoked`: when the key was marked revoked, an RFC 3339 date-time. | |
| `secure_boot`, Secure Boot | 8 | `Title`: "Secure Boot keys". `owner`: the owner GUID in its text form. `pk_key`, `kek_key`, `db_key` (protected): each a PEM `PRIVATE KEY` (PKCS#8, RFC 7468). `pk_cert`, `kek_cert`, `db_cert`: each a PEM `CERTIFICATE`. | |
| `wallet_share`, Wallet shares | 13 | `Title`: its name. `Password` (protected): the share as its `.txt` file holds it (`docs/BACKUP-FLOW.md` §15.6). `index`, from 1; `of`, how many; `wallet`, as a plan's. | |
| `passphrase`, Passphrases | 14 | `Title`: "Passphrase for {for}". `Password` (protected): the passphrase. `for`: the fingerprint (8 lower-case hex digits) of the key whose words it goes with. Both required. Several for one key are each kept and read: the app labels each ("1 of 2 passphrases for key {fingerprint}") and warns when saving one that the vault already holds one for the key, with Continue and Cancel; the warning is not built. | |
| `share`, Codex32 shares | 15 | `Title`: "Codex32 share for {for}". `Password` (protected): a codex32 string (BIP 93) in lower case of a set whose threshold is 2 or more, never its secret. `for`: the fingerprint of the key its set makes. Both required. Several of one share index and key are each kept and read: the app labels each ("Share 3 · also saved: another share 3 of this set") and warns when saving one that the vault already holds, with Continue and Cancel; the warning is not built. | |
| none: the database's custom data | 10, 11, 16, 1 | signed amounts, backup plans, the seal count, the label: above | |
| not carried | 9, 12 | the pending signing round is compiled out of the app (F109), and 12 is no build's | |

The group names are Faraday's strings, written in the language the app
is in; a kind is known by its custom data, never by its group's name, so
a group renamed or an entry moved in a KeePass app changes nothing.

A seed's `faraday.codex32` is its set's plain facts (`docs/CODEX32.md`
§9.4), never a string of the set, as one TOML inline table: `k`, the
threshold (0, the secret alone, or 2 to 9); `id`, the identifier, four
bech32 characters in lower case; `lengths`, the length in characters of
the set's strings (an array of one entry; read when every entry is
equal); `unknown`, how many strings exist whose share index is not
known; `shares`, an array of inline tables, `index` (a bech32
character) and `state` (`made`, `placed`, `lost`, `exposed`,
`destroyed`). At most 31 strings, each index once.

A seed's `faraday.child` is its BIP-85 facts, present only when the
person chose **Record** at **Record it as a child seed?** (F166), as one
TOML inline table: `parent`, the parent key's fingerprint;
`application`, 39; `language`, BIP-85's language number; `words`, the
seed's own word count; `index`, at most 2³¹ − 1. Public: a fingerprint
and numbers. A seed held as `hex` takes none.

**Each is the person's to keep or drop** (owner, 2026-11-02). Wherever a
seed is saved into a vault (Save on Vaults, the secret sheet's Save into
{vault}, and a backup flow's Save into {vault}) or copied or moved into one (§7.2: Copy, Move, an
Unlock {vault} row, New vault with this item), a seed that has codex32 facts or a BIP-85 origin
shows **Keep its codex32 set** and **Keep its BIP-85 origin**, each
ticked; unticked, the vault's entry is written without it, and the seed's
words are saved alone. Save on Vaults puts the ticks on a sheet of their own
before the save, for a seed that has one; the secret sheet carries them
beside Save into {vault}. A backup flow opens the same sheet at the press
that would save seeds into a vault, with each such seed by its label (no
label when there is one seed), then **Cancel**, which saves nothing and
leaves the step, and **Save**, which goes on with the press. A seed is asked
once in a flow, its choice holds for every vault of the flow it goes into
(a copy from a vault that holds it leaves the same fields out) and ends
with that flow and is used by no other. A choice holds for the save it was
made for: one made on the secret sheet is kept with that secret, through the
unlock or New vault it waits for, and ends with it; one made on Vaults ends
with the sheet. Another save of the seed starts ticked and asks for itself. A seed the flow makes has the
fields it has, no more. A seed already in an open vault offers **Remove
codex32 set** and **Remove BIP-85 origin** among the buttons of its page
while it has them and the vault can be written (a read-only vault offers
neither): each drops that custom data from the entry, and nothing else, at
the next seal, after a dialog saying that copies of the vault sealed before
still hold it. A change to the vault while the dialog is up leaves the
entry as it was. What the app knows of the set or the origin elsewhere (a
backup plan, the session's own state) is not changed by either.

What Faraday writes of an entry it makes is its UUID, its fields and its
custom data, and its times when this computer's clock is set; icons,
tags and AutoType are optional in KeePass's format and left out. A new
entry's UUID is 16 bytes of the session's randomness. What a KeePass app
wrote into a vault and Faraday does not read is carried through (§7.1).

**What refuses the file** (owner, 2026-11-03; F194). A KDBX file is a
standard format Faraday supports, and nothing in its contents refuses
it. Only these do: the format limits of §7.1 (KDBX 3.1 and older,
AES-KDF, Twofish or another cipher, Salsa20 as the inner stream, an
empty passphrase); a wrong passphrase, a failed HMAC or a damaged file;
and an XML document that is not well formed, or has a DOCTYPE, or an
entity other than XML's five and character references. The XML reader's
other limits are format limits too (named in §7.1): nesting past 64
elements, markup inside a text element the model reads, a protected value
that is not canonical Base64 (damage), and a `Meta` or `Root` that is
missing or repeated. A comment or a processing instruction, before the
root, after it or inside, is not refused: it is kept where it is and
skipped where Faraday reads text.

**Unreadable items.** A Faraday item that is damaged does not refuse
the file: it is listed on the vault's page as **Unreadable** with its
title, its UUID and what is wrong (the field or key), is never loaded
or used, and is written back as it was read at the next seal. An item
is damaged when it has a `faraday.kind` not named here; a field its kind
does not name, or a required one missing; words that are not a valid
BIP-39 phrase of their list; a policy or descriptor that does not
parse; a PEM block other than the one its field names; a
`faraday.codex32`, `faraday.child` or `faraday.plan.{n}` that is not the
TOML its key names; a GPG key not of Faraday's own shape (an Ed25519
primary key with one Ed25519 signing subkey, unprotected); an entry with
two fields of one name; two entries or two groups with one UUID; an entry,
or a group it is in, that cannot be told apart from what was read (two
UUIDs, a UUID that is not 16 bytes, a field with two values, text between
a field's key and value, and the like). The same holds
for the database's custom data: a `faraday.plan.{n}` or
`faraday.signed.{tx}` that does not read is kept as it was and named,
and not used.

**A file that is not Faraday's.** With `faraday.format` missing it is a
KeePass file Faraday did not make: it opens with its entries, its seal
count reads 0, and an entry with a `faraday.kind` is read as Faraday's
item. With `faraday.format` other than 1 (not a number, 0, or above 1) it
is a newer format and opens read-only: its other entries are shown, every
Faraday item is Unreadable, and the next seal writes nothing. A `Meta` or
database custom data that cannot be told apart from what was read (two
`faraday.format` items, two `DatabaseName`) counts as a newer format. With `faraday.seals` missing or
malformed the seal count starts again at 1: the next seal writes
`faraday.seals` as 1 and the file then reads as Faraday's. Nothing is
written into the file until a seal.

An open vault's view shows each wallet with the plan this vault keeps
for it, as a chart read from the backup up: the places, vaults, files
and software of the plan, the wallet's keys under them (a key whose
seed is in this vault reads "In this vault"), the wallet at the foot
(`docs/history/NEW-WALLET.md` §7.2). A wallet with no plan here reads
"No backup plan in this vault". Each node opens its sheet; its actions
load the wallet, and its seeds this vault keeps, first. Opening one
writes nothing into the vault.

### 7.1 Saved by another program

A vault is a file a person may open and change in a KeePass app:
KeePassXC, KeePass, KeePassDX or Strongbox, or pykeepass. Faraday opens
what such a program saves (owner, 2026-11-02; F190 point 2) within these
limits, beyond what it writes itself:

| | Faraday writes | Faraday reads |
|---|---|---|
| version | 4.0 | 4.0 and 4.1 |
| cipher | ChaCha20 | ChaCha20, AES-256 (CBC) |
| key derivation | Argon2id | Argon2id, Argon2d, within §3's limits |
| compression | none | none, gzip |
| inner random stream | ChaCha20 | ChaCha20 |
| key | the passphrase, and a key file if the person adds one | the passphrase, with a key file or a YubiKey's secret or both (§3.2) |

The head's "Saved in" line names the settings of a file that differs
from what Faraday writes, in the order version, cipher, key derivation,
compression; the version is named only when it is 4.1 ("Saved in KDBX 4.1
/ ChaCha20 / Argon2id / uncompressed").

Refused, naming what was found: KDBX 3.1 and older, AES-KDF, Twofish or
another cipher, Salsa20 as the inner stream, and a database whose
passphrase is empty. The XML document is refused for what the reader
cannot carry or place: a document not well formed, a DOCTYPE, an entity
other than XML's five and character references, an XML declaration that
is not at the start, anything after the root but XML whitespace, comments
and processing instructions, nesting past 64 elements, markup inside a
text element the model reads, a protected value that is not canonical
Base64 (damage), and a `Meta` or `Root` missing or repeated. These and
§7's list are all that refuses a file.

**A key file or a YubiKey** (F208). KDBX 4 records nothing that shows a
key file or a challenge-response key, so Faraday cannot tell such a file
from a wrong passphrase: both fail the header's HMAC. The person adds what
the file needs at unlock (§4), the message for wrong credentials says so,
and the vault's head names what the file needs while it is open
("Needs a key file", "Opens with a YubiKey"). A file that needs a key file
or a secret and no passphrase is refused, as a file with an empty
passphrase is.

**Carried through unchanged** (owner, 2026-11-02): everything a KeePass
app wrote that Faraday does not read is kept as it was and written back
where it was: attachments (the inner header's binaries, with their
protection flag, and the entries' references to them, except that an
attachment nothing refers to is dropped at the seal, below), custom icons and
icon numbers, times, tags, AutoType, colours, the groups and their
settings, deleted-object records, `PublicCustomData`, the database's
settings in `Meta`, and custom data whose key does not begin
`faraday.`, at the database, a group or an entry. Faraday keeps each
such element as the bytes it read, in its place, and an attachment's
bytes in memory held by `faraday-keys`, since an attachment can be a
secret; they count toward the vault's size (§2). An entry Faraday does
not change is written back as it was read; one it changes keeps
everything but its fields, its custom data and its last-modified time,
which it sets when the clock is set.

**Kept, and not read** (owner, 2026-11-03; F207): entry history, and the
recycle bin with the entries in it when `Meta`'s `RecycleBinEnabled` is
True or absent (KeePassXC counts an absent one as enabled); with it
False, the group the database names as its recycle bin is an ordinary
group. They are written back as they were, with `RecycleBinUUID` and
`RecycleBinEnabled`, and Faraday has no action that removes them. They
are old or deleted copies of secrets, so nothing in them is listed,
searched, counted or loaded: a key in the bin or in a history is not a
key of the vault, and no item from either is on a screen. Their
protected values are held as secrets in memory held by `faraday-keys`,
and their bytes count toward the vault's size (§2). A `Meta` with two
`RecycleBinUUID` elements keeps both groups out. An entry Faraday
changes gets no new history version, and an entry deleted in Faraday
goes with its own history; deleting moves nothing to the recycle bin.
A `RecycleBinUUID` that names the root group keeps the root listed: only
a group under the root is kept out. A group with any `UUID` naming the bin
is the bin, and a file with a bin that marks a group's `UUID` Protected is
refused, since the bin must be known without decrypting anything.

**An attachment nothing refers to** is dropped at the next seal, so an
entry deleted in Faraday takes its attachments out of the file. KDBX refers
to the inner header's binaries by position, so a dropped one is emptied in
place (its bytes gone, its protection flag and its position kept, no
reference rewritten) and the trailing ones are left off. A reference is
counted anywhere in the document as it will be written: an entry, an
entry's history, an entry in the recycle bin; an attachment two entries
share stays while one of them does. A reference that cannot be understood
(a `Ref` that is not a plain number, a `Binary` element that is not a
`Key` and a `Value` with a `Ref`) keeps every attachment for that seal; a
reference to a position that does not exist changes nothing. A file a
KeePass app saved with an attachment nothing refers to loses it at its
next seal too; a read-only vault is not sealed and is untouched. The
document is read once more for a vault that has attachments, at each
change and each seal; none is read for one without.

**The file's settings.** The next seal needs what the file needed (§5),
and writes the file as Faraday writes a vault: ChaCha20, Argon2id at the step of §3 nearest at or above the
file's memory, up to 1 GiB, with 3 passes or, when the file has more
(within the reader's 100), the file's own, so a seal never makes a
guess faster than the file allowed; no compression, its size padded. A file
saved in another cipher, KDF or compression says so on the vault's head
while it is open.

What was edited is taken as the person's: an entry added or changed,
an item's fields changed (and checked as §7 says; one that fails is
Unreadable, not refused), the seal count read as written, and the next
seal writes one more than it.

### 7.2 Copy or move an item

An item in an open vault offers **Copy to another vault…** and **Move to
another vault…** among its actions: a seed, a wallet, an entry, a note, a
recovery sheet, a GPG key, a Secure Boot set, a share or a passphrase. An
Unreadable item offers neither (it is never used), and a read-only vault
(F194) offers Copy alone.

Either opens a sheet listing where it can go:

- every other open vault, by name;
- each vault this session knows and is locked (§11), **Unlock {vault}**:
  the copy is made once it is open, and nothing is made if it stays locked.
  A locked vault whose file is not in Files is dimmed, with that as the
  reason;
- **New vault with this item**: Create a vault (§8). Create seals the empty
  vault as its first seal, as always; the vault then opens at once with
  the passphrase just typed, the item goes in, and it is in the file from
  the next seal (a Move's seal rule below covers it). Create offers only
  the sizes the item fits, worked out as the seal would write it with a
  name as long as the document takes (64 bytes) made of the character it
  writes longest (`&`, five bytes), so a size offered holds
  the item whatever name is typed; a size it does not fit is dimmed. The
  item waits only while that form is shown: leaving it, whichever way,
  ends the wait, and the form came back to through the in-progress row
  makes an ordinary vault. A backup flow's own Create vault takes no item.
  The Keep sheet's title names the vault the seed goes into as the press
  that asked chose it (a wallet flow's, which can cover several vaults,
  names them all).

A seed that has a codex32 set or a BIP-85 origin shows the ticks above
the list, each ticked; the rows are judged on what will be written, so a
tick changes which vaults and sizes fit. A Move says in its dialog when a
tick is off that the data is gone from the original too.

A vault that cannot take the item is listed, dimmed, with the reason:
it is read-only, it has no room at its size, or it already holds this
item. An item is the same when every field is the same, and a key also
when its fingerprint is. A second passphrase for a key, or a share of a
set the vault holds, is not the same item: it goes in after the dialog
every save of one has ("{vault} already holds a passphrase for this
key", **Continue** and **Cancel**), and is never refused.

A seed with codex32 facts or a BIP-85 origin shows §7's Keep ticks first.

**What goes.** The copy is a new entry with a new UUID, in the group of
its kind: its fields and Faraday's custom data. What a KeePass app wrote
that Faraday does not read (§7.1: attachments, tags, an icon and the like)
stays in the original vault. Where an entry has any, the dialog says so
before it is copied or moved. A wallet's backup plan is the vault's
(the database's custom data) and stays too: the copy reads "No backup
plan in this vault".

**Move** first says, in a dialog naming both vaults, that the item will be
gone from the original, and that copies of the original sealed before
still hold it. **Move** puts it in the target and removes it from the
original as **Hold to delete from vault** does; **Cancel** does nothing.

**The original is not sealed before the target** (the item would then be
in neither new file). Until the target has been sealed with the item, a
lock of the original alone stops and says to lock {vault} first, and a
lock of all seals the target first. A lock that lets the target go
unsealed lets the original go unsealed too, its changes lost and said so
on the Locked sheet, so the original's last sealed file still holds the
item.

A vault can wait for several: each item moved out of it names its own
target, and the original is sealed after every one of them (a chain, A to B
to C, seals C, then B, then A). A lock of all that finds an original whose
target is still open and unsealed stops and asks, as it does for a vault
that cannot be sealed, and lets the original go only when the person chose
Lock without sealing or an idle lock had no one to ask. A vault waiting for
another's seal in this way, through any number of vaults between, is dimmed
on the Move sheet of that other vault, "Lock {vault} first", since each
would wait for the other and no order of sealing would keep the item. A
backup flow never closes a vault an item was moved into while that item
waits for the vault's seal, nor one an item was moved out of, whose last
sealed file still holds the item (a dropped place of such a vault stays out
of the plan and the vault stays open, sealed by the next lock like any
other). A wallet flow's copy round takes out of an added vault only what
the flow put there: a record that was moved there, of any kind (a seed, a
passphrase record, a wallet description, a wallet share), is kept with what
the vault held before the flow, which is saved with the flow, so it stays
across a seal and a lock. A flow that copies a GPG key or a Secure Boot set
does not write a second record into a vault that holds it, one moved there
included. A flow that drops a place does not take out of a vault the GPG key
or Secure Boot set that was moved there while that vault waits for its seal;
once the record is deleted or moved on, that no longer holds. It takes the
record out of the dropped place's vault, or closes that vault, only while
another vault of the plan that is open really holds the record; where the
only other holder is locked and cannot be read, the record stays. An item
moved into a vault after the checklist's first save of it is kept with what
the vault held before, as one moved after the lock is, so a later change of
the plan does not take it out. Copy the key counts a vault added after the
first that holds the record (one moved there) as a holder, and finishes
where no added vault lacks it. It does not
take out of a vault the flow's own key it moved there itself; the flow's
own key, saved there before, it takes out when the place is dropped, and
the moved item stays. A wallet backup's save takes out of a vault a codex32
share of the plan's set only where the plan keeps that string in another
vault and the string is held elsewhere (`docs/CODEX32.md` §9.5), or is of a set the plan records as rotated away; a codex32 share the plan keeps in no vault, one moved
there included, stays, across a seal and a lock. A wallet backup's save
replaces this wallet's share of the plan's number, told by the wallet and
never by its name, with the plan's.

## 8. Creating

**Create a vault** on Vaults takes a name, the memory step (§3.1) and a
passphrase typed twice; it refuses an empty passphrase. It asks for no
size: a file vault starts at 32 KiB and grows by itself (§2). **Add randomness** is offered before the vault is made
(F189 point 8, `docs/OWN-RANDOMNESS.md` §2.2). Making it draws the
identifier and seals the empty database (its custom data alone) as the
first seal, count 1.

Every seal is a new file, so an open vault's passphrase and memory step
can each be changed. **Change passphrase** and **Memory**
are buttons on a writable vault's page, each opening a dialog; a
read-only vault offers neither. Change passphrase takes the new
passphrase typed twice, refusing an empty one and two that differ, and
does not ask for the old one: the vault is open. Memory lists the file's
five steps (§3.1) with the next seal's marked. It lists the
rows Create's Where will you open it? does (This computer, ticked, and
the five machines of §3.1), the dialog's own ticks: each opening starts
with This computer alone, as a vault file does not record its machines,
and they are neither Create's nor kept. The ticked machines set the
limit; a step above it is marked too large for the weakest, with the
warning, and stays choosable. It refuses at the action what this
computer cannot unlock with (Unlock's test), whatever is ticked. Earlier
copies keep their step, so a copy sealed at a step above another
machine's limit cannot be unlocked there. Either counts as an unsaved
change, so a lock seals it, and a toast says it takes effect at the
next seal, which runs Argon2id under the new passphrase and step. Copies
of the vault sealed before keep their old passphrase and step. Pressing
the step the vault already has, with none pending, changes nothing, and
pressing it with another pending takes that one back. Neither button is
offered, or taken,
while an entry or anything else is being written. Escape and the mouse
back close either dialog as Cancel does, dropping what was typed. An idle
lock whose seal cannot allocate the new step seals again at the file's own,
so its changes are kept, and the Locked sheet says the vault was sealed at
its earlier memory setting.

A vault opened with a YubiKey's secret offers **Remove YubiKey secret** on its
⋮, taking effect at the next seal (F208).

**A key file is optional** (F208). Create a vault offers **Add a key file**,
and the vault's ⋮ offers **Key file**: add, replace or remove, taking
effect at the next seal, as a passphrase does. Either way it is **Make a
key file** or **Choose a file**. Making one writes KeePass's XML version 2
(`{vault}.keyx`) holding 32 bytes of `fresh_bytes("vault key file", 32)`,
to the Outbox through a named reveal (`docs/KEYS.md`); any file from a stick
can be chosen instead, and §3.2 says what is made of it. Both say: "If this
file is lost or changed, the vault cannot be opened. Keep it apart from the
vault's file." Copies sealed before keep the key file they had. Faraday
never sets a YubiKey up: a secret is taken at unlock only, and a vault
opened without one cannot be given one. A QR vault has neither.

Create a vault makes a file vault only; a QR vault is made in Tools
(§13, F226).

**The KeePass export is a vault** (F190 point 10). **Export for KeePass**
on a person's entry makes a new vault holding that one entry, a
person's entry in the root group (§7), at the smallest size it fits,
Argon2id at the first memory step (§3.1), 3 passes and 4 lanes, under a
passphrase typed for it (8 characters or more), as Create a vault makes
one: its identifier, its seal and its entry's identifiers are drawn as a
vault's are (`fresh_bytes`), and the seal is count 1. The file is named
for the entry's title (`password.kdbx` for a BIP-85 password), goes beside
any file of that name waiting (`password-2.kdbx`), never over it (a seal's
file does the same, §5), and waits
in the Outbox sealed. The entry's title is its own, "Password" for a BIP-85 password,
and a notes line that is a BIP-85 path is left out. A KeePass app reads
it as a database of one entry; Faraday opens it as any vault. As
Create a vault does, the export reads its container back under the keys
it was made with, as a seal does, but not its document. Exporting a
key's words, a master seed, a note or a recovery sheet this way is not
built: the entry is the one item with an export. The export has no writer
of its own: the one that wrote it before (`docs/history/KDBX-LOG.md`) is
gone, and its one vector (`tools/vectors/backup/`) is opened and sealed
again byte for byte by the vault's tests. To read an export without
Faraday, `python3 tools/vault/open.py FILE` prints its entry.

## 9. What the vault promises

**Holds.** Without its passphrase a vault shows that it is a KDBX 4
database or an OpenPGP message for a passphrase, its Argon2id cost and
its length; nothing else of its contents. A changed byte
anywhere refuses the whole file: a KDBX file's header HMAC and every
block's HMAC, and an OpenPGP message's session key tag, every chunk's
tag and the final tag, are checked before anything is read (§4). A
KDBX file opens to one content whichever passphrase opens it, since its
HMACs are keyed from the passphrase's key. OCB does not commit to its
key, so a QR vault made by someone else could be built to open to two
contents under two passphrases; Faraday makes every QR vault under one.
Every QR vault has one length, so a code does not say which item it
holds.

**What a file vault's length says** (F206). A file vault's length says how
much it holds, to within 32 KiB. Anyone who sees the file, without its
passphrase, can tell a vault of a few keys from one of thousands of entries
or one carrying large attachments. An entry takes about a kilobyte, so 32
KiB is a few dozen entries either way; a large vault says it is large. All
vaults holding less than about 32 KiB, an empty one included, have one
length. A KDBX file a KeePass app saved has lost Faraday's padding (§7.1),
so its length says more until Faraday seals it.

**No slots, and decoys.** A vault has one passphrase. A decoy is made
inside a wallet's own standards: a BIP-39 passphrase gives another
wallet from the same words, and another derivation path another
account; a vault opened under pressure shows what it holds. That a vault
exists, that the feature exists (Faraday is public), and whether what a
person shows under pressure is believable are outside the promise.

**Two copies from different times.** Every seal is a new file under a
new salt and new keys, so two copies show that the vault was sealed
between them and, by their lengths, whether what it holds crossed a 32 KiB
step: that it grew or shrank, and by about how much. Removing a secret can
therefore show; which entry changed, and what it held, does not.

**Rollback.** An older copy of a vault still opens: whoever can write
the stick and holds an older copy can put it back. Its contents cannot
be altered, but they can be replaced by an earlier state of themselves
(a seed removed since put back, a backup marked done shown as not done,
a signed amount taken back). The seal count is inside the encryption
(§5), and the app names a copy older when it is opened and the session
has seen a newer one: one opened or sealed since power-on, or one an
open vault's backup plan records by the vault identifier (§7, `plan`).
A copy sitting unopened in Files is not compared: reading its count
would cost an Argon2id of its own. It cannot tell when no newer copy was
seen since power-on and no open vault's plan records one, when a vault
and its plan are put back together, or while the vault is locked. Such
an unlock stops at a warning (§11). VeraCrypt's and LUKS's containers
have no rollback protection.

**Tampering and loss.** A vault is checked when it is opened. A changed
byte in a vault nobody opened goes unnoticed until it is next opened,
and then the whole vault is refused: nothing in it can be changed, and
a damaged file loses it all. A copy at another place is the remedy.

**Offline guessing.** Anyone with the file has its salt and cost
and can try passphrases on their own computers, each guess costing one
Argon2id at the vault's step: a vault is as strong as its passphrase
(§3.1). A key file or a YubiKey's secret adds its own 256 or 160 bits for
whoever lacks it, and a person who loses it and has no copy loses the
vault. Without its passphrase, nothing shows that a vault needs either.

**Not done.** The first half of Argon2id's first pass reads memory
independently of the passphrase; the rest reads it at places the
passphrase decides, a cache-timing side channel on the computer that
unlocks. It is later work under the Security scope, as wiping freed
copies is.

## 10. Reading one by hand

A file vault opens in any KeePass app: KeePassXC shows the person's
entries and Faraday's items in their groups, and `keepassxc-cli ls
vault.kdbx` and `keepassxc-cli show -s vault.kdbx "Bitcoin keys/Cold
key"` print them. A QR vault opens with Sequoia: `sq decrypt vault.pgp`
asks for the passphrase and prints its QR TOML, padding included; a
code is first saved as its bytes (`zbarimg --raw -Sbinary code.png >
vault.pgp`). GnuPG does not open a QR vault (§1.2).

`tools/vault/open.py` reads a file vault with pykeepass and prints each
item as §7 names it; with `--field KIND.FIELD --out FILE` (`--index N`
choosing among several) it prints nothing of the vault and writes that
one value to a new file created with mode 0600, refusing a file that
exists; that is how a secret value (a GPG key's armored secret key,
`gpg.key`) reaches another tool without Faraday. Given a QR vault it runs
`sq decrypt` (Sequoia, on PATH; the passphrase goes to it through a pipe,
never as an argument or in a file) and prints the item as §2.1 names it, the
TOML keys without their padding; `--field KEY --out FILE` writes one key's
value the same way. For a QR vault `open.py` is a recovery reader: it parses
no OpenPGP and reads whatever `sq` opens under the passphrase and whose text
is the QR TOML. Refusing the messages §1.2's packets and §2.1's rules
exclude is Faraday's Rust (`faraday_vault::qr::open`), tested on the refused
vectors in `tests/qr.rs`; `open.py` and `sq` do not claim it.

The vectors in `tools/vectors/vault/` are the file vaults and refused
files `tools/vectors/vault/README.md` lists, written by
`faraday/faraday-vault/examples/vectors.rs` byte for byte: a 256 KiB
vault holding one item of every kind, a 64 KiB vault, one of a codex32
set, one made with "café" in NFD (e and U+0301) that opens under its NFC
form, a vault sealed once more at 64 MiB, 3 passes and 4 lanes, a file
whose stream ends early, and one file refused for each rule of §1, §3,
§4, §7 and §7.1. Faraday's Rust (`tests/vectors.rs`) and `open.py` with
pykeepass read them. Files of other programs are under
`tools/vectors/kdbx/` (pykeepass's, with the ciphers and KDFs Faraday
refuses or reads, one of KeePassXC's KDBX 3.1, and one KeePassXC 2.7.12
saved after an edit: AES-256, Argon2d, gzip, history, an attachment, a tag, an
icon number and custom data, whose history, attachment, tag, icon number
and custom data the next seal writes back byte for byte).

The QR vault's vectors, `tools/vectors/vault/qr/` (`README.md` there),
are Faraday's own, not an outside check: one vault of each item kind (a
12-word and a 24-word seed, a codex32 share, a passphrase, a note), an
empty one, one made with "café" in NFD that opens under its NFC form, and
four refused messages (a SKESK v4 with a SEIPD v1, a second SKESK, a
compressed packet, data after the SEIPD), written byte for byte by
`faraday/faraday-vault/examples/qr_vectors.rs`; Faraday's Rust
(`tests/qr.rs`) reads them and refuses the four, and `open.py` with Sequoia
opens the seven vaults.

The cross-check area `vault` (F79) takes Faraday's KDBX to pykeepass,
`keepassxc-cli` and keepass-rs (a test-only dependency, never in what
ships), both ways: each opens the files the other made (F190 point 11).
Faraday's files (every kind of §7, a TOTP
entry, a seal over each) are to open in pykeepass and `keepassxc-cli` with every field, group and
attachment; files they make are to open in Faraday with the same fields,
and Faraday's next seal to keep all that the file held, an entry's history
and the recycle bin among it; each reader is to refuse a
wrong passphrase, a changed byte and, where Faraday's NFC form differs
from the typed one, the NFD form; every file of `tools/vectors/vault/`
is read by both. KeePassXC ends a protected value at its first NUL, which
Faraday keeps, so such a Password reads shorter there (a known difference,
counted).

The QR vault's OpenPGP has its own area, `qrvault` (built, F211): Faraday's seals
opened by `sq`, by rPGP and by a PyCA judge, and rPGP's and the judge's
opened by Faraday. `sq` judges the primitives except Argon2, which it
takes from the RustCrypto crate the app uses; rPGP, a program of its
own (`tools/crosscheck/rpgp/`, outside the workspace and never shipped),
runs RustCrypto too and judges the format only; the judge is PyCA
cryptography's Argon2id, HKDF-SHA-256 and AES-256-OCB over OpenSSL, under
an OpenPGP framing the area writes from §1.2 (Faraday's second version,
not another program's) that accepts nothing but a QR vault. OpenPGP.js is
left out of it. Sequoia's default
passphrase encryption writes the older packets (a SKESK v4 and a SEIPD
v1, without Argon2), which a QR vault refuses (§1.2): `sq` 1.3.1
writes only those two packets for a password, under either profile
(`--profile rfc9580` included), so it cannot seal a QR vault, and the
area `qrvault` asserts that Faraday refuses what it writes. Measured (2026-11-02): OpenPGP.js 6.1.0 made a QR vault of the
226-byte plaintext of §2.1 at 64 MiB, 3 passes and 4 lanes in 396 bytes,
a SKESK v6 and a SEIPD v2, QR version 15 at medium error correction, and
Sequoia (pysequoia 0.1.36) opened it to the same plaintext.

Each reader has a fuzz target, seeded from the vectors: the KDBX header
and block stream, the inflate and the XML reader with §7's rules
(`kdbx_file`, `kdbx_body` for the padding, the inflate and the inner
header past the HMACs, and `kdbx_xml`), the QR vault's packets
(`pgp_qr_vault`) and its TOML (`contents`).

## 11. What the app remembers of a locked vault

(owner, 2026-10-10; `docs/history/SIMPLIFY.md` §3.4, decision 4.) While
a vault is open, the app keeps a summary of it: the file's name and the
vault identifier, the vault's name, its wallets (name, shape and
descriptor checksum), its seeds (each key's fingerprint and its 32-byte
master id, the SHA-256 of the master public key and chain code with the
passphrase applied, `wallet::master_id`; owner, 2026-10-21), how many
entries it holds, its backup maps' lines (`plan.holds`), the checksums
of the wallets it keeps a plan for, the time it was last open by this
computer's clock, and the highest seal count seen of it since power-on
(the count and the SHA-256 of the file at that count, so two seals of
one count are told apart; raised at every unlock and every seal).
Unlock and the open vault's head say "Sealed 12 times · last 28 Oct
2026, 14:02", and, when the copy opened is lower than a count seen, "An
older copy: sealed 9 times. A copy sealed 12 times was open since
power-on." (or "… {vault}'s backup plan records a copy sealed 12
times"), and for the same count with another file "Another copy was
sealed 12 times too, with other changes". Faraday cannot tell an older
copy from one put back by someone else, so such an unlock stops at a
dialog, **Open this copy?**, naming the vault and that line: **Keep
locked** closes the vault unread, and only **Open** opens it (owner,
2026-10-29; `docs/VAULT-HARDENING.md` §5.3). It is never refused. The
Backups screen lists a locked vault's wallets from it
(`docs/history/SIMPLIFY.md` §5), and its vault line says a seed is kept
in a locked vault only when the seed's master id, the session key's or
the one the backup's record kept, is the summary's for that key: the
fingerprint finds the candidate, the id says it is that seed (refuse
what cannot be verified rather than assume: F65's rule, which
`CLAUDE.md` keeps since F72). The summaries are rebuilt on unlock, after
every change made on a vault screen and as the session seals, and
written into the kept state at lock under the key `vault-summaries`,
which the next process reads back. They are gone at power-off, with the
rest of `/run/faraday`.

The Vaults list's row of a locked vault with a summary goes by the
vault's name ("Savings"), keeps its file name, and reads the summary in
place of its size and memory: "Savings 2 of 3 · key 9a6a2580 · 12
entries · seen 14:02" (one or two wallets and keys by name, more by
count). Home's Unlock lead is "Unlock Savings" with the same line. A
locked vault with no summary keeps its row as before and reads "Unlock
to see what it holds".

**The trade-off.** Wallet names, shapes and descriptor checksums, key
fingerprints and master ids, the backup map's places and the vault's
name stay in RAM, and in `/run/faraday`, while the vault is locked,
until power-off. None of it spends: no seed, no words, no entry's
fields, no descriptor. Powering off ends it.

## 12. Searching an open vault

An open vault's contents carry Material's search bar (DESIGN §4.3) at
the top of the items column for **Wallets**, **Entries** and **GPG
keys**, the kinds whose items hold a public title: an entry's `title`, a
wallet's `name`, a wallet share's `name`, a GPG key's user IDs. Typing
keeps the items whose title holds what is typed, case not counted, as it
is typed; the × shows them all again; none found reads **No titles
match**. Usernames, URLs and notes are not searched: titles only.
Nothing secret is read to search: Notes has no bar, since a note's title
is its first line, a secret; Bitcoin keys have none, since a key is
named by its fingerprint, worked out rather than kept; Secure Boot holds
one set. The codes screen has a bar of its own (`docs/TOTP.md` §5.3).

What each bar finds is kept on the open vault beside what is read of its
items (`vault/search.rs`, `Found`), dropped by every change to the items
and to a bar's text, and found again before the next frame, so drawing
only looks it up. A lock empties both bars.

## 13. The QR vault tool

A QR vault is made and opened in one place, a tool in Tools (F226), and
never kept open: it is not listed on Vaults, has no page, and is never
sealed at a lock. What the tool typed, opened or made is wiped when it is
left, at the idle warning and at a lock, as New key's is
(`docs/FLOWS.md`). To change what a code holds, a new one is made; a code
printed before still opens, to what it held. Faraday Starter has no such
tool.

**The tile.** **QR vault** on the Tools page (`catalog.rs`), in the Vaults
group, tags OpenPGP and Argon2id, opens the tool at its first card, which
offers **Make** and **Open**. Create a vault makes file vaults only: it
has no File or QR code choice.

**Make.** A step flow:

1. **Contents**: **Type a note** (one field, masked behind its eye,
   DESIGN §4.10, showing the room left as it is typed and taking no
   more than §2.1's `QR_NOTE_MAX`), or **A loaded key** (its row, for a
   key of 12 or 24 English words; any other key's row is dimmed, its line
   "Not 12 or 24 English words"). The words are sealed inside
   `faraday-keys` and never lent to the tool.
2. **Passphrase**: typed twice, not empty, not measured; a warning line
   says the code is only as strong as its passphrase. The memory step
   (§3.1), one of `SEAL_EXPONENTS`, the first by default; **Add
   randomness** (`docs/OWN-RANDOMNESS.md` §2.2); a name, as a vault's
   (§6: empty is `qr-vault.pgp`).
3. **Code**: sealed under Progress (one Argon2id run, §5), then the
   code, **Show as QR**, **PNG to the Outbox** (the labelled picture,
   §2.1) and **To the Outbox** (the `.pgp` file), and **Make another**.
   The seal is read back under the same passphrase before the code is
   shown, as a file vault's seal is.

**Save as a QR vault on a vault's item.** The page of an item in an open
file vault offers **Save as a QR vault** for a seed, a codex32 share, a
BIP-39 passphrase and a note, and for nothing else: a person's entry, a
wallet, a recovery sheet, a GPG key, Secure Boot keys and a wallet share
have no place in a QR vault's contents (§2.1). The button opens the
Make flow at its second step, with the item as its contents; the item
stays in the file vault, unchanged. An item too long has the button
too, and pressing it opens a dialog instead:

| The item | Fits when | Its dialog when it does not |
|---|---|---|
| A seed | 12 or 24 words of the English list, held as words | "This key does not fit in a QR vault. A QR vault holds 12 or 24 English words." |
| A codex32 share | always: BIP 93's longest string fits | none |
| A passphrase | its TOML text is at most `QR_PASSPHRASE_MAX` | "This passphrase is too long for a QR vault." |
| A note | its TOML text is at most `QR_NOTE_MAX` | "This note is {n} characters too long for a QR vault. Take {n} characters off its end to make it fit." |

A note's count is the fewest characters, each one Backspace takes (a
Unicode scalar value), whose removal from the end leaves a note that
fits, with its escapes and its UTF-8 lengths counted as §2.1 counts
them. A passphrase or a key gets no count, since a shorter one is
another secret. A seed's passphrase kept with it (§7) is not in the code,
and the Contents line says so: "Its passphrase is not in this code."

**Open.** A code scanned (the tool's camera, Home's **Scan**, or any
scanner that takes a SeedQR) or a `.pgp` file chosen in Files that parses
as §1.2's packets opens the tool's Open card with it. The passphrase is
typed once; the S2K's cost is checked before Progress starts (§4). A
wrong passphrase reads so and may be tried again; a message Faraday does
not open is named (§1.2). Opened, the card says what the code holds and
offers what that item takes, as the same item scanned in its own form
would:

| It holds | Offered |
|---|---|
| A seed's words | the seed sheet (**Use without a passphrase** / **Add a passphrase**, `docs/FLOWS.md`), then the flow of a scanned SeedQR |
| A codex32 share | Add a key's codex32 form, with the share typed in |
| A passphrase | **Add to key {fingerprint}** when that key is loaded; otherwise "Key {fingerprint} is not loaded", and the passphrase behind its eye |
| A note | the note behind its eye, and **Save as a note** in an open file vault (dimmed, "Open a vault first", when none is) |
| Nothing | "This QR vault is empty." |

What it opened to is held in `faraday-keys` (`Reveal::QrVault`,
`docs/KEYS.md`) and leaves only through these offers and the eye.
Nothing scanned is filed in Files; a `.pgp` chosen there stays there.
