# The Faraday vault

**Kind:** Spec
**Status:** Specified (F189); what is built is the four-slot FVLT format this replaces, whose account is `docs/history/VAULT-LOG.md`
**Open:** all of it: the age vault (§1 to §6), its memory steps and sizes (§2, §3), its TOML contents (§7), the QR vault (§2.1), sealing with the session's randomness (§5), the seal count by the vault identifier (§9, §11), the reader and the vectors (§10)

A vault is one file, an age v1 file encrypted under one passphrase,
holding a TOML document of the person's keys, wallets, entries and
notes. age v1 is the C2SP specification (`age-encryption.org/v1`,
c2sp.org/age), with a reference implementation outside Faraday (Go
`age`), a second (`rage`), and published vectors (the age testkit,
age-encryption.org/testkit). A vault therefore opens without Faraday:
`age -d` gives its document, which a text editor shows.

Faraday writes the passphrase part of age only (F178, F189 point 9):
the header, the scrypt stanza, the header MAC and the payload, over
Faraday's own scrypt (`core/faraday-crypto/src/scrypt.rs`, which wipes
its working memory), HMAC-SHA-256 and HKDF-SHA-256, and the
`chacha20poly1305` crate, in `faraday/faraday-vault`. The age
specification is what a second implementation is written against; this
file is Faraday's use of it and the TOML names inside, and the vectors
under `tools/vectors/vault/` settle a disagreement.

The FVLT format this replaces (four equal slots under Argon2id and
XChaCha20-Poly1305, `.fvlt`) is read by nothing once this is built
(F189 point 10). No vault exists anywhere, so nothing is migrated.

## 1. The file

A vault is an age v1 file with exactly one recipient stanza, of type
scrypt, read and written as the age specification says:

```text
age-encryption.org/v1
-> scrypt <salt, 16 bytes in base64> <log2 of the work factor>
<the wrapped file key, 32 bytes in base64>
--- <the header MAC in base64>
<payload: a 16-byte nonce, then the contents in 64 KiB chunks>
```

```text
wrap key    = scrypt(N = 2^logN, r = 8, p = 1, dkLen = 32,
                     S = "age-encryption.org/v1/scrypt" || salt, P = passphrase)
stanza body = ChaCha20-Poly1305(key = wrap key, nonce = 12 zero bytes, file key)
HMAC key    = HKDF-SHA-256(ikm = file key, salt = empty, info = "header")
MAC         = HMAC-SHA-256(HMAC key, the header up to and including "---")
payload key = HKDF-SHA-256(ikm = file key, salt = payload nonce, info = "payload")
chunk i     = ChaCha20-Poly1305(payload key, nonce = i as 11 bytes big-endian
                                 || 1 for the last chunk else 0, 64 KiB of contents)
```

The file key is 16 bytes, the salt 16 and the payload nonce 16, all new
for every seal (§5). With a work factor of two digits, as every step
Faraday writes has (§3), the header is 150 bytes: the version line 22,
the stanza's argument line 36, its body 44 and the MAC line 48. Each
chunk adds a 16-byte tag; a vault's contents are a whole number of
chunks, the last of them full, which age allows.

**Binary only.** Faraday writes the binary form, never age's ASCII
armor, and reads the binary form only; an armored file (`age -a`) is
named and not opened. `.age` is the name Faraday writes and the one its
lists group its own files by.

**Recognised by its contents.** A file the person selects is a vault
when its first line is `age-encryption.org/v1` and its header holds one
scrypt stanza, whatever its name or extension: a copy renamed, or
with no extension, opens. An age file whose one stanza is not scrypt, or with
more than one stanza, is named "An age file for a key, not a
passphrase" and not opened; age itself refuses a scrypt stanza beside
any other. The wipe asks for its hold on any file named `.age`, whatever
it holds.

## 2. Sizes

A vault is one of five sizes, the size of its contents before
encryption; the contents are padded to it (§7), so a file's length says
only which size it is (F189 point 4):

| Size | Contents | File | Suits |
|---|---|---|---|
| QR (§2.1) | one item or a note, top-level keys | about 408 B | one seed, one codex32 share, one passphrase or a short note, printed as a code |
| 64 KiB | 65,536 B | 65,718 B | keys, wallets, a few entries |
| 256 KiB | 262,144 B | 262,374 B | the default: keys, wallets, notes, GPG and Secure Boot keys, about a hundred entries |
| 1 MiB | 1,048,576 B | 1,048,998 B | many entries and notes |
| 4 MiB | 4,194,304 B | 4,195,494 B | the largest |

A file's length is the header's 150 bytes, the payload nonce's 16, the
contents, and 16 bytes for each 64 KiB chunk. The size is chosen when a
vault is made and can be changed on an open vault (§8); the next seal
writes the new size. A reader opens a file whose contents are up to 4
MiB of any length, as a file edited by hand and encrypted again with
`age` is (§7.1), and refuses a longer one from the file's length,
before any work; the next seal pads it to the smallest size that holds
it.

### 2.1 The QR size

A QR vault holds one item, a note, or nothing yet (F189 point 6): a
seed's words (12 or 24 words of the English list, as a SeedQR's are; a
Seed XOR part is 24), a codex32 share, a BIP-39 passphrase with the
fingerprint of the key it belongs to, or any text typed into it. No
item is longer than a 24-word seed: a passphrase longer than that, a
seed of another list or word count, and a longer text go in a file
vault.

Its contents are the item's keys at the top of a TOML document, named
as in §7 and written one to a line: a seed is `words = "…"`; a codex32
share is `codex32 = "…"` and `for = "…"` (the fingerprint of its key);
a passphrase is `passphrase = "…"` and `for = "…"`; a note is
`note = "…"`. At most one of `words`, `codex32`, `passphrase` and
`note` is present: a QR vault with none of them is empty. There is no
`vault` table: a QR vault has no seal count and no identifier, and
every seal is a new code (§8). The text is padded with line feeds to the
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
outside ASCII its UTF-8 length. The age file is 182 bytes more than its
plaintext, 408 bytes; the code is version 15 (77 × 77 modules) at
medium error correction, which holds 412 bytes in byte mode.

A QR vault is a vault like the others: it unlocks with its passphrase,
is listed on Vaults while open, marked as a QR vault, and shows what it
holds. What it holds can be changed at any time, however it was made
(§8): a key put in, the text typed or edited, one replaced by the
other, or emptied. Each change is sealed as a new code; a code printed
before still opens, to what it held when it was sealed, so the screen
says to print the new code and destroy the old. It is not a secret
under `docs/QR.md` §7.1: it is drawn and printed as any code is, and
offered as a labelled PNG (`docs/QR.md` §4 items 3 and 4) whose label lines
are "QR vault", the vault's name and the date it was sealed, never what
it holds.

Scanned wherever a SeedQR is taken, it asks for its passphrase, opens,
and its words go to the seed sheet (**Use without a passphrase** /
**Add a passphrase**, `docs/FLOWS.md`), then load as a scanned SeedQR's
do; a share goes to Add a key's codex32 form, and a passphrase to the
seed it belongs to, found by its fingerprint. The Starter scans and
prints a QR vault and never asks for its passphrase.

## 3. Keys

The passphrase is put in Unicode NFC (Unicode Standard Annex #15), and
scrypt takes the NFC form's UTF-8 bytes; two passphrases with one NFC
form are one passphrase (F159 point 2). Nothing else is done to it: no
character is mapped or refused, and compatibility characters ("ﬁ",
"Ａ"), case and spaces are kept as typed. `age` itself does not put a
passphrase in NFC: a vault whose passphrase has a character with two
forms opens under `age -d` only when typed in NFC, which the vectors
show (§10).

scrypt's work factor is N = 2^logN, with r = 8 and p = 1 as age fixes
them. Its memory is 1 KiB × N. Faraday writes five steps, logN 16 to 20
(F189 point 3):

| logN | Memory | Bits added |
|---|---|---|
| 16 | 64 MiB | ≈17 |
| 17 | 128 MiB | ≈18 |
| 18 | 256 MiB, the default | ≈19 |
| 19 | 512 MiB | ≈20 |
| 20 | 1 GiB | ≈21 |

**Cost limits.** A reader refuses a work factor above 2^20 before it
allocates anything. A work factor below 2^16, which another age
implementation can write, is opened, with its bits shown. The work factor is in the clear header, so
the memory a vault needs is shown beside it before its passphrase is
asked for. scrypt runs on the worker (F184), so the screen is drawn
while it works.

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
log2(2N), the number of scrypt's 1 KiB block steps in one guess (N
written and N read): ≈ logN + 1, from ≈17 to ≈21. That is an estimate:
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
falls short. **Where it will be opened** marks the largest step that
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

**Where the numbers come from.** scrypt's paper (Percival, 2009)
suggests N = 2^20, r = 8, p = 1 for file encryption; age's own tools
write N = 2^18 by default; OWASP's lowest for scrypt is N = 2^17, r = 8,
p = 1. 64 MiB is the step a Pi Zero 2 W opens with room to spare, 128
MiB OWASP's lowest, 256 MiB age's default, and 1 GiB the paper's. r and
p are age's: p = 1 because parallelism is the defender's, and without
threads it would add time without adding work for an attacker, who runs
guesses in parallel anyway.

"Memory free to open" is scrypt's memory, plus twice the vault file
(the file and a sealed copy beside it while it is written), plus 100 MiB
(`APP_MIB`) for the contents and the app. One function in
`faraday-core` gives it from the memory and the size, as
`vault_need_mib` does today; Unlock's check, Create's choice for **This
computer** and the import rule of `docs/LARGE-FILES.md` §4.1 all use
it. On Faraday the whole system is a few tens of megabytes in RAM; on
another operating system, or under `age` on a desktop, the margin is
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
| Raspberry Pi 3B+ | to be measured | to be measured | to be measured | to be measured | — |
| Raspberry Pi 4 (4 GB) | to be measured | to be measured | to be measured | to be measured | to be measured |
| Raspberry Pi 5 (8 GB) | to be measured | to be measured | to be measured | to be measured | to be measured |
| Recent x86-64 laptop | to be measured | to be measured | to be measured | to be measured | to be measured |

The Raspberry Pi rows come from the dev card's timings: a Pi booted from
a `dev=1` image writes each vault made and unlocked into
`faraday-timings.txt` as a `work` row naming its memory and how long it
took (`docs/DEVELOPING.md` §6.2), and a step the board could not
allocate as "failed: no memory". p is 1, so a multi-core machine opens
no faster than a single core; the Pi's cores are much slower than a
laptop's.

**What the form enforces.**

- Memory above what this machine can allocate is refused: making a vault
  runs its scrypt, so a vault this machine could not open is never
  written.
- Memory above what the weakest machine chosen gives a vault (above) is
  a warning naming that machine.
- An unlock time above 10 seconds on the slowest chosen machine, by the
  reference table, is a warning.
- The summary panel always shows the unlock memory and the unlock time
  here.

**When a vault does not fit.** Opening a vault whose memory this machine
cannot allocate (its memory free to open, above) says so before the
passphrase is typed: "Needs 1.1 GiB. This computer has 900 MiB free."
It does not start scrypt.

## 4. Opening

1. Read the header as age says: the version line, one stanza, the MAC
   line. The stanza is scrypt with three arguments, a salt that is the
   canonical base64 of 16 bytes and a work factor in decimal with no
   leading zero; the work factor is at most 2^20 (§3); the body is
   exactly 32 bytes, checked before it is decrypted, as age requires
   against partitioning oracles. The file's length gives contents of at
   most 4 MiB (§2). Any fault refuses the file, naming it, before any
   memory is taken.
2. Show the memory the vault needs, and stop if this machine has not
   got it (§3.1).
3. Put the passphrase in NFC and run scrypt on the worker. The stanza
   body is decrypted with the wrap key: a tag that fails means the
   passphrase is wrong.
4. Check the header MAC under the file key. A MAC that fails refuses the
   file. The MAC is keyed by the file key, so whichever passphrase opens
   a file, one file key and so one content verifies: what the FVLT
   format's commitment block was for (F189).
5. Decrypt the payload chunk by chunk into memory held by
   `faraday-keys`, wiped as it drops, checking every tag and that the
   last chunk is marked last and no data follows it. Any failure refuses
   the file, and nothing of the contents is read.
6. Read the contents (§7). Any fault refuses the file, naming it.
7. Compare the seal count with what the session has seen (§11).

Several vaults may be open at once.

A backup plan puts one seed in each vault by default, each vault on its
own stick at its own place (2026-10-10, `docs/history/NEW-WALLET.md` §5,
`docs/WALLETS.md` §5): a vault per seed, so no place and no passphrase
reaches more than one key unless the person ticks more seeds into one.

## 5. Sealing

Sealing writes the open vault back to memory as a file, and every seal
is a new age file (F189 point 7), as age requires ("A new salt MUST be
generated for each stanza and each file"):

- **New random bytes.** The salt (16 bytes), the file key (16) and the
  payload nonce (16) are one `fresh_bytes("vault seal", 48)`, two
  answers of the system's randomness (F154), so whatever the person
  added through Add randomness is mixed in (`docs/OWN-RANDOMNESS.md`
  §2.3). A seal waits for the answers it needs and never stretches
  fewer. Nothing is sealed before they arrive: a lock with a changed
  vault open then seals nothing and says so, and the vault stays open
  with its changes.
- **scrypt runs again**, at the vault's step, on the worker (F184): a
  new salt is a new wrap key. The passphrase is kept while the vault is
  open, in `faraday-keys`, wiped as it drops: it is what a seal needs
  that the contents do not hold. A lock that seals a vault waits for its
  scrypt.
- **The seal count is one higher** (§7, `vault.seals`), with the time
  by this computer's clock when it is known, written by the vault crate
  alone. The app keeps the highest count it has seen of each vault
  (§11); a backup plan can carry the counts of other vaults across a
  power-off (§7, `plan`).
- **The contents are padded to the vault's size.** A vault whose
  changes do not fit its size is never closed at a lock. Entries read
  from the camera or from files are added only if they fit the room
  left; if not, none is added and the toast (and the camera's note) says
  how many bytes they take and how many are left. Should a changed
  vault's contents still exceed its size, it cannot be sealed: the lock
  does not go ahead, nothing is wiped, the vault stays open with its
  changes and the Lock sheet names the vault, by how many bytes it is
  over, and the next size that holds it (§8). "Lock without sealing" is
  the person's own choice to lose those changes. Locking one vault alone
  (a backup's "Lock it and make a new vault") refuses the same way with
  a toast.

Sealing happens on every lock (`docs/history/PLAN.md` §5.3), and for one
vault alone when a backup's vault item locks it to make the next vault
("Lock it and make a new vault"); that vault closes and the rest stay
open. A QR vault is sealed only by **Save** on its page or in the
backup flow (§8), when what it holds changes, never at a lock, so its
printed code stays current until it is changed. The sealed file goes For the stick
(Files' half that a stick visit writes). With more than one vault file
For the stick, a visit ticks one, the first the last write did not
write, so each vault goes to its own stick.

## 6. Writing back

A vault is named when it is made (owner, 2026-10-07): its file is that
name, letters, digits, hyphens and underscores, spaces made hyphens,
with `.age`; left empty it is `vault.age`. A name taken already becomes
`name-2.age`, as backup files are named. A name can say what a vault
holds to anyone who sees the stick; leaving it empty keeps the old rule.

The name is not the vault's identity, and neither are its bytes' header:
every seal has a new salt, so a copy cannot be told from another vault
without opening it (F189 point 7). The session keeps, for each vault it
opened or sealed since power-on, the SHA-256 of every copy of it it
read or wrote: the copy opened, and each seal. A file on a stick is a
copy of that vault when its SHA-256 is among them. Writing back
replaces that copy; no older copy is kept. On a stick visit:

1. Find the file on the visited stick whose SHA-256 is one of the sealed
   vault's copies. None: offer to write it as a new file. More than one:
   refuse and name both. A file under the name asked for that is not a
   copy is never written over, whatever the visit's Already on the stick
   sheet chose (`docs/FLOWS.md` decision 58): the new one goes beside it
   as `name-2.age`. The visit holds no passphrase and opens nothing: a
   process that has unlocked a vault is never handed a stick
   (`docs/history/PLAN.md` §5.1). Counts are compared at unlock instead
   (§11).
2. Write the sealed vault as a new file beside it, read it back and
   compare byte for byte.
3. Delete the original and rename the new file to the name it is
   written under: the original's, unless the person renamed the file
   For the stick since a visit last wrote it (`docs/FLOWS.md` decision
   61), when it takes the new name, or `name-2.age` where another file on
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

A vault's contents are a TOML v1.0.0 document in UTF-8, then line feeds
to the vault's size (§2): `age -d` gives a file a text editor shows,
and TOML reads the padding as blank lines. Each item is in its published
text form (F189 point 5). Which TOML names mean what is Faraday's own
rule, written here, and fails closed: an unknown table or key refuses
the file, naming it, as an unknown record type does in the FVLT format.

Every vault but a QR vault has one `vault` table, first:

| Key | What |
|---|---|
| `format` | 1. Required. |
| `id` | the vault identifier: 16 bytes in 32 lower-case hex digits, drawn once when the vault is made (`fresh_bytes("vault id", 16)`) and kept for its life. Required. By it a backup plan names another vault (`plan`, below); it is not a secret, and only whoever opens the vault reads it. |
| `name` | the vault's label (≤ 64 bytes), shown on Vaults while open. Starts as the vault's name, or "Vault" when none is typed (F57). |
| `seals` | how many times the vault has been sealed, an integer, 1 or more. Required. Written by `faraday-vault` alone: each seal writes one more than the vault was opened with (or than the last seal of the same opening). |
| `sealed` | the time of the last seal, a TOML offset date-time in UTC by the clock of the computer that sealed it; absent when that clock was not set. |

Each kind of item is an array of tables, one table per item, in any
order. The FVLT record each takes the place of is named in the first
column:

| FVLT record | TOML | Keys, and the published form of each |
|---|---|---|
| 2 Bitcoin key | `[[seed]]` | `words`, the BIP-39 words of its list, one space between, or `hex`, a BIP-32 seed of 16 to 64 bytes in lower-case hex: exactly one of the two; `list` for words of a list other than English (`japanese`, `korean`, `spanish`, `chinese-simplified`, `chinese-traditional`, `french`, `italian`, `czech`, `portuguese`); `label`; `load = false` when it is not loaded at unlock (F62; absent, it loads); `held = true` while it was made in Faraday with its backup not done, so it signs nothing until it is (`docs/history/NEW-WALLET.md` §14.3); `passphrase`, its BIP-39 passphrase, present only when the person chose to keep it with the seed; `codex32`, a table of its codex32 set's facts (below); `child`, a table of its BIP-85 facts (below) |
| 3 Wallet | `[[wallet]]` | `policy`, its BIP-388 wallet policy, or `descriptor`, its descriptor (BIP 380) for a wallet whose key has no origin: exactly one; `name`; `load = false` as a seed's |
| 4 Note | `[[note]]` | `text` |
| 5 Recovery sheet | `[[sheet]]` | `descriptor`, `name`, `note`; the three together at most 16 KiB |
| 6 Entry | `[[entry]]` | `title` (required), `username`, `password`, `url`, `notes`, `otp`: an `otpauth://` URI (the Key URI Format; a bare key is kept as one, `docs/TOTP.md` §4.3) |
| 7 GPG key | `[[gpg]]` | `key`, the armored OpenPGP secret key as `gpg --export-secret-keys --armor` writes it, unprotected (required); `public`, the public certificate as last exported, armored; `revocation`, the revocation certificate made with the key (F32), armored; `revoked`, when the key was marked revoked, an offset date-time |
| 8 Secure Boot keys | `[[secure_boot]]` | `owner`, the owner GUID in its text form; `pk_key`, `kek_key`, `db_key`, each a PEM `PRIVATE KEY` (PKCS#8, RFC 7468); `pk_cert`, `kek_cert`, `db_cert`, each a PEM `CERTIFICATE` |
| 10 Signed amounts | `[[signed]]` | `tx`, the SHA-256 of an unsigned transaction in hex; `amounts`, the amount each input stated, in satoshis, in order. One table per transaction; two for one transaction refuse the file, naming it. Written by `docs/WALLETS.md` §3.3 when that setting is on; the oldest are dropped first when the vault is full |
| 11 Backup plan | `[[plan]]` | `wallet`, as a `wallet` table's `policy` or `descriptor` holds it; `answers`, the plan's lines, an array of strings, as the FVLT record's field 2 (`docs/history/VAULT-LOG.md`, §7 of the FVLT format), with `seals {id} {count} {digest}` for each other vault the plan places whose copy the session has seen: that vault's identifier, its seal count and the SHA-256 of the file at that count, never the plan's own vault; `places`, the places' names in order (empty for "Place n"); `holds`, what each place, vault, stick of files or device holds, a line each; `marks`, the notes on the map's things, a line each (`checked`, `keychecked`, `lost`, `exposed`, `holder`, `vaultname`); never a secret. Written when the backup's checklist is made with a vault open and when the plan is changed; the next backup of that wallet starts from it |
| 13 Wallet share | `[[wallet_share]]` | `text`, the share as its `.txt` file holds it (`docs/BACKUP-FLOW.md` §15.6); `name`; `index`, from 1; `of`, how many; `wallet`, as a plan's |
| 14 Passphrase | `[[passphrase]]` | `for`, the fingerprint (8 lower-case hex digits) of the key it is the BIP-39 passphrase of, the key's words with this passphrase; `text`, the passphrase. Both required; two for one key refuse the file, naming the key |
| 15 Codex32 share | `[[share]]` | `codex32`, a codex32 string (BIP 93) in lower case of a set whose threshold is 2 or more, never its secret; `for`, the fingerprint of the key its set makes. Both required; at most one per share index and key |
| 16 Seal count | `vault.seals`, `vault.sealed` | above |
| 1 Slot label | `vault.name` | above |
| 9 Pending signing round, 12 (retired) | none | not carried: the round is compiled out of the app (F109), and 12 is no build's |

A seed's `codex32` table is its set's plain facts (`docs/CODEX32.md`
§9.4), never a string: `k`, the threshold (0, the secret alone, or 2 to
9); `id`, the identifier, four bech32 characters in lower case;
`lengths`, the length in characters of the set's strings (one length is built: written as an array of one entry, read when every entry is equal); `unknown`, how many
strings exist whose share index is not known; `shares`, an array of
tables, `index` (a bech32 character) and `state` (`made`, `placed`,
`lost`, `exposed`, `destroyed`). At most 31 strings, each index once.

A seed's `child` table is its BIP-85 facts, present only when the person
chose **Record** at **Record it as a child seed?** (F166): `parent`, the
parent key's fingerprint; `application`, 39; `language`, BIP-85's
language number; `words`, the seed's own word count; `index`, at most
2³¹ − 1. Public: a fingerprint and numbers. A seed held as `hex` takes
none.

```toml
[vault]
format = 1
id = "3f0c9a6e2b7d41c58e19a0b2c4d6e8f0"
name = "Savings"
seals = 12
sealed = 2026-10-28T14:02:00Z

[[seed]]
words = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
label = "Cold key"

[[entry]]
title = "Mail"
username = "me"
password = "correct horse battery staple"
otp = "otpauth://totp/Mail:me?secret=JBSWY3DPEHPK3PXP&issuer=Mail"
```

**What refuses the file.** A document that is not TOML v1.0.0; a
`vault` table missing, out of place or without `format`, `id` or
`seals`; a table or key not named here; a value of the wrong type; a
required key missing; a key given twice where one is allowed (TOML
refuses it already); words that are not a valid BIP-39 phrase of their
list; a policy or descriptor that does not parse; a PEM block other than
the one its key names; an `otp` that is not an `otpauth://` URI Faraday
reads; a GPG key not of Faraday's own shape: an Ed25519 primary key with
one Ed25519 signing subkey, unprotected. Each names what it found and
where (the table and the key, or TOML's line). Nothing of a refused file
is loaded.

An open vault's view shows each `wallet` with the `plan` this vault
keeps for it, as a chart read from the backup up: the places, vaults,
files and software of the plan, the wallet's keys under them (a key
whose `seed` is in this vault reads "In this vault"), the wallet at the
foot (`docs/history/NEW-WALLET.md` §7.2). A wallet with no `plan` here
reads "No backup plan in this vault". Each node opens its sheet; its
actions load the wallet, and its seeds this vault keeps, first. Opening
one writes nothing into the vault.

### 7.1 Edited by hand

A vault is a file a person may open and change without Faraday: `age -d
vault.age > vault.toml`, an edit in a text editor, and `age -p -o
vault.age vault.toml` (binary, not `-a`). Faraday opens the result when
every item reads by §7, at any length up to 4 MiB (§2) and any work
factor up to 2^20 (§3); the `vault` table must still be there, as the
editor found it. What was edited is taken as the person's: the seal
count is read as written, and the next seal writes one more than it, at
the vault's size and step.

## 8. Creating

**Create a vault** on Vaults takes a name, the memory step (§3.1), the
size (§2), and a passphrase typed twice; it refuses an empty
passphrase. **Add randomness** is offered before the vault is made
(F189 point 8, `docs/OWN-RANDOMNESS.md` §2.2). Making it draws the
identifier and seals the empty document (`vault` alone) as the first
seal, count 1.

Every seal is a new file, so an open vault's passphrase, memory step and
size can each be changed (**Change passphrase**, **Memory**, **Size**,
on the vault's ⋮): the change takes effect at the next seal, which runs
scrypt under the new passphrase and step and pads to the new size. A
size the contents do not fit is not offered. Copies of the vault sealed
before keep their old passphrase.

A **QR** size in Create a vault makes a QR vault (§2.1), empty, with
the same name, memory step and passphrase; there is no separate flow
for it. Its page then offers **Add a key** (a seed's words, a codex32
share, or a BIP-39 passphrase with its key's fingerprint, chosen from
the keys on the device) and a text field for a note, which shows the
room left as it is typed and takes no more than §2.1's maximum; both
can be used again at any time to replace what it holds. **Save** seals
it as a new code with a new salt and file key; the memory step and
passphrase can be changed as a file vault's can. The backup flow,
wherever a seed, a share or a passphrase is placed, makes a QR vault
holding it in the same way, **Add randomness** offered first. Several
shares or parts take **one shared passphrase** or **a passphrase for
each**, and the flow says to keep each in a separate place.

A QR vault's page and the backup flow, wherever one is placed, offer
**Show as QR**, **PNG to the Outbox** (the labelled picture, §2.1) and
**To the Outbox** (the `.age` file, which `age -d` opens); a stick visit
writes the Outbox as it writes any file. After each save the new code
is shown, with the note that an earlier printed code still opens to
what it held before.

## 9. What the vault promises

**Holds.** Without its passphrase a vault shows that it is an age file
for a passphrase, its memory step and which of the five sizes it is;
nothing of its contents. A changed byte anywhere refuses the whole file:
the stanza's tag, the header MAC and every chunk's tag are checked
before anything is read (§4). Whichever passphrase opens a file, it
opens to one content, since the header MAC is keyed by the file key.
Every QR vault has one length, so a code does not say which item it
holds.

**No slots, and decoys.** A vault has one passphrase. A decoy is made
inside a wallet's own standards: a BIP-39 passphrase gives another
wallet from the same words, and another derivation path another
account; a vault opened under pressure shows what it holds. That a vault
exists, that the feature exists (Faraday is public), and whether what a
person shows under pressure is believable are outside the promise.

**Two copies from different times.** Every seal is a new file under a
new salt and file key, so two copies show only that the vault was sealed
between them, not what changed.

**Rollback.** An older copy of a vault still opens: whoever can write
the stick and holds an older copy can put it back. Its contents cannot
be altered, but they can be replaced by an earlier state of themselves
(a seed removed since put back, a backup marked done shown as not done,
a `signed` table taken back). The seal count is inside the encryption
(§5), and the app names a copy older when it is opened and the session
has seen a newer one: one opened or sealed since power-on, or one an
open vault's backup plan records by the vault identifier (§7, `plan`).
A copy sitting unopened in Files is not compared: reading its count
would cost a scrypt of its own. It cannot tell when no newer copy was
seen since power-on and no open vault's plan records one, when a vault
and its plan are put back together, or while the vault is locked. Such
an unlock stops at a warning (§11). VeraCrypt's and LUKS's containers
have no rollback protection.

**Tampering and loss.** A vault is checked when it is opened. A changed
byte in a vault nobody opened goes unnoticed until it is next opened,
and then the whole vault is refused: nothing in it can be changed, and
a damaged file loses it all. A copy at another place is the remedy.

**Offline guessing.** Anyone with the file has its salt and work factor
and can try passphrases on their own computers, each guess costing one
scrypt at the vault's step: a vault is as strong as its passphrase
(§3.1).

**Not done.** scrypt's second half reads memory at places the
passphrase decides, a cache-timing side channel on the computer that
unlocks. It is later work under the Security scope, as wiping freed
copies is.

## 10. Reading one by hand

A vault opens without Faraday: `age -d vault.age` (Go `age` or `rage`)
asks for the passphrase and prints the TOML of §7, padding included.
`tools/vault/open.py` becomes a reader of that TOML: given the output of
`age -d`, it prints each item, and with `--field TABLE.KEY --out FILE`
(`--index N` choosing among several) it prints nothing of the vault and
writes that one value to a new file created with mode 0600, refusing a
file that exists; that is how a secret value (a GPG key's armored
secret key, `gpg.key`) reaches another tool without Faraday.

The vectors in `tools/vectors/vault/` are made again as age files: a
256 KiB vault holding one item of every kind, a 64 KiB vault, a QR vault
of each item kind, one made with "café" in NFD (e and U+0301) that opens
under its NFC form, a file whose payload ends early, and files refused
for each rule of §1, §4 and §7. Faraday's age is checked against the age
testkit's scrypt and header vectors, and in the F79 area `vault` against
Go `age` both ways: each opens the files the other made (F189 point 9).

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
