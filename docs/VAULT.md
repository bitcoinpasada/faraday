# The Faraday vault

**Status:** v0.1 · 2026-10-04 · built 2026-10-05 in `faraday/faraday-vault` (format, create, open, seal, records), with the vectors in `tools/vectors/vault/` and the reference reader `tools/vault/open.py`, which opens both vectors. Not built yet: §3.1's Raspberry Pi timings, and the writer of record type 9 (`HANDOFF.md` item 12 says why). Types 7 and 8 are written by `faraday-pgp` and `faraday-sb` through the app.

A vault is one file holding four equal slots. Each passphrase opens one
slot. A slot no passphrase opens is indistinguishable from one that holds
data, so the file does not say how many slots are in use. This file is
the format; where an implementation and this file disagree, this file is
what a second implementation is written against, and the vectors under
`tools/vectors/vault/` settle it.

The cryptography is OpenSigner's `osk-backup` format (`docs/BACKUP.md`),
whose crate `core/osk-backup` also provides the Argon2id `Cost` type:
Argon2id version 1.3 and XChaCha20-Poly1305. The slot structure follows the duress
record of OpenSigner's kept-key blob, which is random bytes under a random
key when no duress PIN is set (`opensigner-core/src/keep.rs`).

## 1. The file

Every integer is little-endian. Offsets are from the start of the file.

| At | Bytes | What |
|---|---|---|
| 0 | 4 | magic, `OFVT` (`4f 46 56 54`) |
| 4 | 1 | version, 1 |
| 5 | 4 | Argon2id memory cost, KiB |
| 9 | 4 | Argon2id passes |
| 13 | 4 | Argon2id lanes |
| 17 | 4 | slot plaintext length *L* |
| 21 | 32 | Argon2id salt |
| 53 | 4 × (*L* + 40) | slots 0 to 3 |

Slot *i* starts at 53 + *i* × (*L* + 40):

| At | Bytes | What |
|---|---|---|
| 0 | 24 | XChaCha20-Poly1305 nonce |
| 24 | *L* | ciphertext |
| 24 + *L* | 16 | Poly1305 tag |

A file is exactly 53 + 4 × (*L* + 40) bytes. Every vault has four slots,
whether one passphrase is set or four.

## 2. Slot sizes

The person chooses *L* when creating a vault. A reader accepts these and
no others:

| *L* | File size | Suits |
|---|---|---|
| 65 536 (64 KiB) | 262 357 B | keys, wallets, a few entries |
| 262 144 (256 KiB) | 1 048 789 B | the default: keys, wallets, notes, GPG and Secure Boot keys, about a hundred entries |
| 1 048 576 (1 MiB) | 4 194 517 B | many entries and notes |
| 4 194 304 (4 MiB) | 16 777 429 B | the largest |

All four slots of a vault have the same size. The size is fixed for the
life of the vault.

## 3. Keys

```text
K      = Argon2id(passphrase, salt, memory, passes, lanes), 32 bytes
K_slot = HKDF-SHA256(ikm = K, salt = empty, info = "Faraday vault v1 slot")
AD_i   = bytes 0..53 of the file || i      (one byte, 0 to 3)
```

The passphrase is its UTF-8 bytes, unnormalised, as in `osk-backup`. One
passphrase gives one `K_slot`; it is tried against every slot. The header
is associated data for every slot, so a changed header fails every tag
rather than costing work under the wrong parameters.

**Cost limits.** A reader checks the stated cost before allocating
anything: memory 8 MiB to 4 GiB, passes 1 to 10, lanes exactly 1. Outside
that, the file is refused. Inside it, a failed allocation reports
"Needs *n* MiB". Faraday creates vaults with at least 64 MiB and
2 passes; the lower reader limits exist so that a vault made elsewhere
under RFC 9106's or OWASP's smaller settings is still read.

The cost is in the header, which is not secret, so the memory a vault
needs is shown beside it before a passphrase is asked for.

### 3.1 Choosing a cost

The cost cannot be changed after creation (§8), and a vault opens only on
a machine that can allocate its memory. The cost is therefore chosen for
the weakest machine the vault will ever be opened on, not for the machine
it is created on.

**Memory is a hard limit, time is not.** A machine without the memory
cannot open the vault at any speed. A slow machine opens it slowly.

**The passphrase matters more than the cost.** Doubling the memory or the
passes doubles the work of every guess, which is worth one bit of
passphrase strength. The whole range from 64 MiB to 4 GiB is a factor of
64, six bits. One more word from the EFF long list is 12.9 bits. A
six-word dice passphrase (77.5 bits) is out of reach at any cost here; a
short or reused passphrase is not protected by any cost here. The Create
form says this beside the cost.

**The bits shown.** The Create form states what each cost adds as
log2(memory in KiB × passes), the number of Argon2id 1 KiB block steps
in one guess: Light ≈17.6 bits, Standard ≈19.6, Medium ≈20.6, Strong
and Maximum ≈22.0. That is an estimate: it counts work only, not how much harder
memory is than time for an attacker's hardware, so Strong and Maximum
come out equal. A passphrase the dice made has its own bits exactly
(words × log2 of the list's length, shown rounded to a tenth). The form
shows the two added as the passphrase's strength, for each passphrase and
the weakest in the summary, and only while the field still holds the
dice's words. Every figure that includes the cost's bits is written with
≈, never =, and the dice's own bits without it. A typed passphrase's own
bits are not measured, and the form says so (`vaults::cost_bits`,
`Vaults::phrase_bits`).

**Two sizes, then Customise** (owner, 2026-10-10; `docs/SIMPLIFY.md`
§3.1). The Create form's first card is **Size**, two rows: **PCs only**
(Medium, 512 MiB, 3 passes) and **PCs and a Raspberry Pi** (Light, 64
MiB, 3 passes), the second chosen by default, each with its unlock time
here and the memory it needs free. Since the size has a default, the
form opens on Name and passphrases. A small **Customise** button opens
the three cards the size stands for in its place: where the vault will
be opened, the unlock cost, and the space per passphrase.

**Presets.** Customise asks where the vault will be opened and marks
the largest preset that fits the weakest machine chosen as suggested:

| Preset | Memory | Passes | Memory free to open | Opens on |
|---|---|---|---|---|
| Light | 64 MiB | 3 | about 150 MiB | anything, including a 512 MB Pi Zero 2 W |
| Standard | 256 MiB | 3 | about 350 MiB | a 1 GB Raspberry Pi 3 and anything larger |
| Medium | 512 MiB | 3 | about 600 MiB | a 1 GB Pi 3 with little else running; any PC |
| Strong | 1 GiB | 4 | about 1.1 GiB | a Pi 4 or 5 with 2 GB or more; a PC with 2 GB or more |
| Maximum | 2 GiB | 2 | about 2.1 GiB | a Pi 4 or 5 with 4 GB or more; a PC with 4 GB or more |
| Custom | 64 MiB to 4 GiB | 2 to 10 | memory + about 100 MiB | chosen by the person |

**Where the numbers come from.** No published recommendation names a
minimum of four passes; the four in RFC 9106 is lanes. RFC 9106 §4
recommends Argon2id at 2 GiB with 1 pass, or 64 MiB with 3 passes where
memory is short, both with 4 lanes. libsodium's presets are 64 MiB with 2
passes (interactive), 256 MiB with 3 (moderate) and 1 GiB with 4
(sensitive). OWASP's lowest is 19 MiB with 2 passes. Light is RFC 9106's
second option, Standard libsodium's moderate, Strong libsodium's
sensitive, and Maximum is RFC 9106's first option with a second pass.
Medium is Standard's three passes at twice the memory, the PCs-only
size.
The designers' advice is to take as much memory as the target allows and
then add passes to fill the time budget, which is how the presets are
ordered. Lanes are 1 here, not 4: a lane is parallelism for the defender,
and without threads it would add time without adding work for an
attacker, who runs guesses in parallel anyway.

"Memory free to open" is the Argon2id memory plus the vault file, one
slot's plaintext and the running system. On Faraday the whole system
is a few tens of megabytes in RAM; on another operating system, or in the
reference reader on a desktop, the margin is whatever that system leaves
free.

Two cautions are not settled by the table and are measured before
release: OpenSigner's Pi 3 image is 32-bit, where one allocation of 1 GiB
or more may fail even with the memory free; and the time column below is
empty until measured.

**Time.** Faraday runs one 64 MiB pass at boot and states the unlock
time of each preset on this computer. Times on other machines come from
the reference table, filled by `tools/argon2-bench` on reference hardware
and published with each release:

| Machine | Light | Standard | Strong | Maximum |
|---|---|---|---|---|
| Raspberry Pi Zero 2 W | to be measured | — | — | — |
| Raspberry Pi 3B+ | to be measured | to be measured | to be measured | — |
| Raspberry Pi 4 (4 GB) | to be measured | to be measured | to be measured | to be measured |
| Raspberry Pi 5 (8 GB) | to be measured | to be measured | to be measured | to be measured |
| Recent x86-64 laptop (Ryzen 7 7445HS, 2026-10-05) | 0.3 s | 1.2 s | 6.9 s | 8.2 s |

The laptop row was measured by `cargo run --release -p
faraday-argon2-bench`, three runs a preset, the middle kept.
The Raspberry Pi rows wait for the boards.

Lanes are 1 everywhere, so a multi-core machine opens no faster than a
single core; the Pi's cores are much slower than a laptop's, and the Pi
times are expected to be several times the laptop's.

**What the form enforces.**

- Memory above what this machine can allocate is refused: creating a
  vault derives its key, so a vault this machine could not open is never
  written.
- Memory above half of the smallest RAM among the machines the person
  chose is a warning naming that machine.
- An unlock time above 10 seconds on the slowest chosen machine, by the
  reference table, is a warning.
- The summary panel always shows the memory needed to open and the
  unlock time here.

**When a vault does not fit.** Opening a vault whose memory this machine
cannot allocate says so before the passphrase is typed: "Needs 2 GiB.
This computer has 1.5 GiB free." It does not attempt the derivation.

## 4. Opening

1. Check the magic, the version, *L* against §2, the file length against
   *L*, and the cost against §3.
2. Derive `K_slot` once.
3. Try to decrypt **all four** slots, each with its own nonce and `AD_i`,
   in place and one at a time, zeroizing a buffer whose tag fails. The
   work is the same whichever slot opens and whether any does.
4. Exactly one tag verifies: that slot is open. None: the passphrase is
   wrong. More than one: refuse the file (it cannot be written by this
   format, since slots under one key are never created).
5. Parse the opened plaintext (§7). Any fault is a refusal of that slot.

Several vaults may be open at once, each with one open slot.

## 5. Sealing

Sealing writes the vault back to memory as a file:

- The header is unchanged.
- The open slot is re-encrypted under the same `K_slot` with a fresh
  24-byte nonce drawn from the shell's entropy. A random 24-byte nonce
  is the reason the cipher is XChaCha20 and not ChaCha20.
- **Every other slot is copied byte for byte.** A session cannot tell an
  unused slot from one that another passphrase opens, so it never
  rewrites, re-randomises or clears one.

Sealing happens on every lock (`PLAN.md` §5.3). The sealed file goes
For the stick (Files' half that a stick visit writes).

## 6. Writing back

A vault is named when it is made (owner, 2026-10-07, reversing the
fixed name of 2026-10-04): its file is that name, letters, digits,
hyphens and underscores, spaces made hyphens, with `.ofv`; left empty it
is `vault.ofv`. A name taken already becomes `name-2.ofv`, as OpenSigner
names its backups. A name can say what a vault holds to anyone who sees
the stick; leaving it empty keeps the old rule. The name is not the
vault's identity. A vault is known by its salt and its size, and
writing back replaces the previous copy; no older copy is kept. On a
stick visit:

1. Find the file on the visited stick whose salt and length equal the
   sealed vault's. None: offer to write it as a new file. More than one:
   refuse and name both.
2. Write the sealed vault as a new file beside it, read it back and
   compare byte for byte.
3. Delete the original and rename the new file to the original's name.
4. FAT has no atomic replace. If a visit finds a new file that reads back
   whole and no original, it completes step 3; if it finds both, it
   keeps the original and discards the new file.

Once written and matched, the vault leaves For the stick and stays in
Files as the stick now holds it, and the visit's receipt keeps the
SHA-256 of the bytes written (owner, 2026-10-10; `docs/SIMPLIFY.md`
§4.3). The receipt is kept across a lock under the key `receipt` and is
gone at power-off. From it each vault file's currency is computed,
never stored (§3.5): **Never written** (sealed For the stick, on no
receipt), **On {label} · current** (the receipt wrote the bytes it has
now), **Changed since written** (open with unsaved changes, or sealed
bytes that differ from the receipt's). A vault copied in, unchanged and
on no receipt, reads none of the three.

## 7. Slot contents

A slot's plaintext is *L* bytes:

| At | Bytes | What |
|---|---|---|
| 0 | 1 | slot format, 1 |
| 1 | … | records, then the end record |
| … | … | zero, to *L* |

A **record** is a type byte, a `u32` body length, and the body. Type 0 is
the end record and has no length or body. Every byte after it must be
zero. An unknown type refuses the slot: a build reads only what it knows.

A record **body** is a sequence of fields: a field number (`u8`), a `u16`
length, and the bytes. Text is UTF-8 and is validated. A required field
missing, a field repeated where it may not be, or a body that does not end
exactly where its last field does, refuses the slot.

| Type | Record | Fields |
|---|---|---|
| 1 | Slot label | 1 label (≤ 64 bytes). At most one per slot. |
| 2 | Bitcoin key | 1 key: an `osk-backup` kind-1 (34 bytes) or kind-2 (65 bytes) payload; 2 label; 3 flags (`u8`: bit 0 load at unlock); 4 BIP-39 passphrase, present only when the person chose to store it |
| 3 | Wallet | 1 BIP-388 text, or the descriptor for a wallet whose key has no origin; 2 name |
| 4 | Note | 1 the note, as `osk-backup` kind 3 |
| 5 | Recovery sheet | 1 the sheet, as `osk-backup` kind 4 |
| 6 | Entry | 1 title (required); 2 username; 3 password; 4 URL; 5 notes; 6 TOTP secret as an `otpauth://` URI |
| 7 | GPG key | 1 primary Ed25519 seed (32 bytes); 2 primary creation time (`u32`); 3 signing subkey seed (32 bytes); 4 subkey creation time; 5 user ID (repeatable); 6 expiry (`u32` seconds after creation, 0 for none); 7 the public certificate as last exported |
| 8 | Secure Boot keys | 1 owner GUID (16 bytes); 2 PK private key, PKCS#8 DER; 3 PK certificate, DER; 4 KEK key; 5 KEK certificate; 6 db key; 7 db certificate |
| 9 | Pending signing round | 1 scheme (`u8`: 1 MuSig2, 2 FROST); 2 SHA-256 of the unsigned transaction it is bound to; 3 the key's fingerprint; 4 the round's secret state, as `osk-psbt` serialises it; 5 used (`u8`), set when round 2 signs. Written by the flow in `docs/WALLETS.md` §4.2 and, for FROST, by the carry's secret sheet (`docs/FLOWS.md` decision 6), with the carry file as field 4; Faraday removes the record once its round has signed, so it is gone at the next seal |
| 10 | Signed amounts | 1 SHA-256 of an unsigned transaction; 2 the amount each input stated, `u64` per input in order (repeatable per record: one record per transaction). Written by `docs/WALLETS.md` §3.3 when that setting is on; the oldest records are dropped first when the slot is full |
| 11 | Backup plan | 1 the wallet, as a type 3 record's field 1 holds it; 2 the plan's answers, a line each (`name value`: `seeds`, `places`, `split`, `sticks`, `wallet`, `software`, `form`, `omit`, `pass i`); 3 a place's name (≤ 128 bytes, repeatable: one per place, in order, empty for "Place n"); 4 what one place, the vault, a stick of files, the software or the seeds on their own devices hold, a line (repeatable). Written when the backup's checklist is made with a vault open (`docs/WALLETS.md` §5), over the record this slot kept for the same wallet; the next backup of that wallet starts from it. The only place a place's name is kept |

In each type the first field is required and every other field is
optional; type 10's field 2 is also required. Fields marked repeatable
may appear more than once; no other field may. A key's field 1 must be a
payload `osk-backup` reads as its kind, and a recovery sheet's field 1
likewise.

Records appear in any order. A slot with no records holds only the format
byte and the end record, and is still a used slot.

## 8. Creating

The **Create vault** form takes the slot size, the Argon2id cost, and one
to four passphrases, each typed twice. It refuses two equal passphrases.

1. Draw a 32-byte salt.
2. Choose the slot positions for the passphrases by a random permutation
   of 0 to 3, so the first passphrase is not always slot 0.
3. Each used slot gets its contents (§7) under its own `K_slot`.
4. Each unused slot gets 24 random bytes of nonce and *L* + 16 random
   bytes, from a ChaCha20 keystream keyed by the shell's entropy: the
   same distribution as a nonce and a ciphertext.

**Passphrases are set only at creation.** A session that opened one slot
cannot tell whether another slot is free, so adding a passphrase later
could overwrite a slot another passphrase opens. Changing a passphrase,
adding one, the cost or the slot size means creating a new vault and
moving contents into it. The form says plainly that a slot whose
passphrase is not entered is not carried over, because Faraday cannot
know whether such a slot exists.

## 9. What the slots promise

**Holds.** Without a passphrase, a single copy of a vault file shows four
random-looking slots of equal size. It does not say how many passphrases
exist or which slot a passphrase opens. Unlocking takes the same work
whichever slot opens. Nothing on screen counts slots or reports free
ones.

**Does not hold.** Someone with **two copies of a vault from different
times** can compare them. A slot's bytes change only when a session that
opened it seals it, so a slot whose bytes changed between the copies is in
use. This is the same limitation VeraCrypt documents for hidden volumes.
Also outside the promise: that a vault exists at all; that the feature
exists, since Faraday is public; and whether the contents of the slot
a person opens under pressure are believable, which is the person's to
arrange. For Bitcoin keys, a BIP-39 passphrase gives the same kind of
deniability inside a single slot.

**Tampering.** A slot is checked only when it is opened. Bytes changed in
a slot nobody opened go unnoticed until its passphrase is next used, and
then that slot fails to open. Changing a slot's bytes destroys it; it
cannot alter its contents.

## 10. Reading one by hand

`tools/vault/open.py` opens a vault with a passphrase and prints the
records of the slot it opens, over the standard library and either
`argon2-cffi` with `pynacl` or `cryptography` 44 or newer, as OpenSigner's
`tools/backup/decrypt.py` does. A vault therefore opens without
Faraday. The vectors in `tools/vectors/vault/` are opened by both
implementations in the tests: a 64 KiB vault with one passphrase, and a
64 KiB vault with three, each slot holding one record of every type.

## 11. What the app remembers of a locked vault

(owner, 2026-10-10; `docs/SIMPLIFY.md` §3.4, decision 4.) While a slot
is open, the app keeps a summary of it: the file's name and salt, the
slot's name, its wallets (name and shape), its keys (fingerprints), how
many entries it holds, its backup maps' lines (record type 11, field 4)
and the time it was last open by this computer's clock. The summaries
are rebuilt on unlock, after every change made on a vault screen and as
the session seals, and written into the kept state at lock under the key
`vault-summaries` (`memory.rs`), which the next process reads back. They
are gone at power-off, with the rest of `/run/faraday`.

The Vaults list's row of a locked vault with a summary goes by the
slot's name ("Main"), keeps its file name, and reads the summary in
place of its size and cost: "Savings 2 of 3 · key 9a6a2580 · 12 entries
· seen 14:02" (one or two wallets and keys by name, more by count).
Home's Unlock lead is "Unlock Main" with the same line. A locked vault
with no summary keeps its row as before and reads "Unlock to see what
it holds".

**The trade-off.** Wallet names and shapes, key fingerprints, the backup
map's places and the slot's name stay in RAM, and in `/run/faraday`,
while the vault is locked, until power-off. None of it spends: no seed,
no words, no entry's fields, no descriptor. It does say which slot was
open, which §9's deniability does not cover while the machine stays on;
powering off ends it.
