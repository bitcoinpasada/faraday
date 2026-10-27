# The Faraday vault

**Kind:** Reference
**Status:** Built; §3.1's Raspberry Pi timings are not measured yet

The format is built in `faraday/faraday-vault`, with the vectors in
`tools/vectors/vault/` and the reference reader `tools/vault/open.py`,
which opens the vectors and takes one field out of a record with
`--field TYPE.FIELD --out FILE`.

A vault is one file holding four equal slots. Each passphrase opens one
slot. A slot no passphrase opens is indistinguishable from one that holds
data, so the file does not say how many slots are in use. This file is
the format; where an implementation and this file disagree, this file is
what a second implementation is written against, and the vectors under
`tools/vectors/vault/` settle it.

The cryptography is Argon2id version 1.3 and XChaCha20-Poly1305; the
Argon2id `Cost` type is `core/faraday-backup`'s. An unused slot is random bytes
the size of a used one (§8, step 4), as the duress record of upstream's
kept-key blob was when no duress PIN was set; Faraday has no duress PIN,
and that blob went with the upstream app (F68).

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

The passphrase is its UTF-8 bytes, unnormalised. One
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
bits are not measured, and the form says so (`vault::cost_bits`,
`Vaults::phrase_bits`).

**Two sizes, then Customise** (owner, 2026-10-10; `docs/history/SIMPLIFY.md`
§3.1). The Create form's first card is **Size**, two rows: **PCs only**
(Medium, 512 MiB, 3 passes) and **PCs and a Raspberry Pi** (Light, 64
MiB, 3 passes), the second chosen by default, each with its unlock time
here, or what this computer has free where that falls short (owner,
2026-10-11: the vault's memory is one figure, its Argon2id memory). Since the size has a default, the
form opens on Name and passphrases. A small **Customise** button opens
the three cards the size stands for in its place: where the vault will
be opened, the unlock cost, and the space per passphrase.

**Presets.** Customise asks where the vault will be opened and marks
the largest preset that fits the weakest machine chosen as suggested.
**This computer** is ticked by default and can be unticked (owner,
2026-10-11); it gives a vault the largest custom memory that fits in
what it has free beside twice the vault file and 100 MiB (below), at
most 4 GiB, and its row reads
that with its free memory in parentheses, "1 GiB (1.2 GiB free)".
Another machine gives half its memory:

| Preset | Memory | Passes | Memory free to open | Opens on |
|---|---|---|---|---|
| Light | 64 MiB | 3 | 166 MiB (196 MiB) | anything, including a 512 MB Pi Zero 2 W |
| Standard | 256 MiB | 3 | 358 MiB (388 MiB) | a 1 GB Raspberry Pi 3 and anything larger |
| Medium | 512 MiB | 3 | 614 MiB (644 MiB) | a 1 GB Pi 3 with little else running; any PC |
| Strong | 1 GiB | 4 | 1,126 MiB (1,156 MiB), about 1.1 GiB | a Pi 4 or 5 with 2 GB or more; a PC with 2 GB or more |
| Maximum | 2 GiB | 2 | 2,150 MiB (2,180 MiB), about 2.1 GiB | a Pi 4 or 5 with 4 GB or more; a PC with 4 GB or more |
| Custom | 64 MiB to 4 GiB | 2 to 10 | memory + twice the file + 100 MiB | chosen by the person |

The figures are for the default slot size's 1 MiB file, and in
parentheses for the largest, 16 MiB (§2).

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

"Memory free to open" is the Argon2id memory, plus twice the vault
file (the file and a sealed copy beside it while it is written), plus
100 MiB (`APP_MIB`) for a slot's plaintext and the app. One function in
`faraday-core`, `vault_need_mib(memory_mib, slot_len)`, gives it from the
unlock memory and the slot size (a header's, or the form's); Unlock's
check (`vault_unlock_check`), Create's choice for **This computer** and
the import rule of `docs/LARGE-FILES.md` §4.1 all use it. On Faraday
the whole system is a few tens of megabytes in RAM; on another
operating system, or in the reference reader on a desktop, the margin
is whatever that system leaves free.

Two cautions are not settled by the table and are measured before
release: the Pi 3 image is 32-bit, where one allocation of 1 GiB
or more may fail even with the memory free; and the time column below is
empty until measured.

**Time.** Faraday runs one 64 MiB pass at boot and states the unlock
time of each preset on this computer. Times on other machines come from
the reference table, filled by `faraday/tools/argon2-bench` on reference hardware
and published with each release:

| Machine | Light | Standard | Strong | Maximum |
|---|---|---|---|---|
| Raspberry Pi Zero 2 W | to be measured | — | — | — |
| Raspberry Pi 3B+ | to be measured | to be measured | to be measured | — |
| Raspberry Pi 4 (4 GB) | to be measured | to be measured | to be measured | to be measured |
| Raspberry Pi 5 (8 GB) | to be measured | to be measured | to be measured | to be measured |
| Recent x86-64 laptop (2026-10-05) | 0.3 s | 1.2 s | 6.9 s | 8.2 s |

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
- Memory above what the weakest machine chosen gives a vault (above)
  is a warning naming that machine.
- An unlock time above 10 seconds on the slowest chosen machine, by the
  reference table, is a warning.
- The summary panel always shows the unlock memory and the unlock time
  here.

**When a vault does not fit.** Opening a vault whose memory this machine
cannot allocate (its memory free to open, above) says so before the
passphrase is typed: "Needs 2.1 GiB.
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

A backup plan puts one seed in each vault by default, each vault on its
own stick at its own place (2026-10-10, `docs/history/NEW-WALLET.md` §5,
`docs/WALLETS.md` §5): a vault per seed, so no place and no passphrase
reaches more than one key unless the person ticks more seeds into one.

## 5. Sealing

Sealing writes the vault back to memory as a file:

- The header is unchanged.
- The open slot is re-encrypted under the same `K_slot` with a new
  24-byte nonce: the first 24 bytes of SHA-256 over a label, the
  session's seed (the shell's first 32 bytes of entropy) and a count of
  the seals this process has made. No two seals of a process share a
  nonce, and two processes share one only if the system gave both the
  same seed. Nothing is sealed before the seed arrived: a lock with a
  changed vault open then seals nothing and says so, and the vault stays
  open with its changes. A 24-byte nonce is the reason the cipher is
  XChaCha20 and not ChaCha20.
- **Every other slot is copied byte for byte.** A session cannot tell an
  unused slot from one that another passphrase opens, so it never
  rewrites, re-randomises or clears one.

Sealing happens on every lock (`docs/history/PLAN.md` §5.3), and for one vault alone
when a backup's vault item locks it to make the next vault ("Lock it and
make a new vault"); that vault closes and the rest stay open. The sealed
file goes For the stick (Files' half that a stick visit writes). With
more than one vault file For the stick, a visit ticks one, the first the
last write did not write, so each vault goes to its own stick.

## 6. Writing back

A vault is named when it is made (owner, 2026-10-07, reversing the
fixed name of 2026-10-04): its file is that name, letters, digits,
hyphens and underscores, spaces made hyphens, with `.ofv`; left empty it
is `vault.ofv`. A name taken already becomes `name-2.ofv`, as
backup files are named. A name can say what a vault holds to anyone who sees
the stick; leaving it empty keeps the old rule. The name is not the
vault's identity. A vault is known by its salt and its size, and
writing back replaces the previous copy; no older copy is kept. On a
stick visit:

1. Find the file on the visited stick whose salt and length equal the
   sealed vault's. None: offer to write it as a new file. More than one:
   refuse and name both. A different vault under the name asked for is
   never written over, whatever the visit's Already on the stick sheet
   chose (`docs/FLOWS.md` decision 58): the new one goes beside it as
   `name-2.ofv`.
2. Write the sealed vault as a new file beside it, read it back and
   compare byte for byte.
3. Delete the original and rename the new file to the name it is
   written under: the original's, unless the person renamed the file
   For the stick since a visit last wrote it (`docs/FLOWS.md` decision
   61), when it takes the new name, or `name-2.ofv` where another file
   on the stick has that.
4. FAT has no atomic replace. If a visit finds a new file that reads back
   whole and no original, it completes step 3; if it finds both, it
   keeps the original and discards the new file.

Once written and matched, the vault leaves For the stick and stays in
Files as the stick now holds it, and the visit's receipt keeps the
SHA-256 of the bytes written (owner, 2026-10-10; `docs/history/SIMPLIFY.md`
§4.3) and the stick's label and FAT volume serial. The backup flow
(`docs/BACKUP-FLOW.md`, F31) does not tell sticks apart by the serial,
which sticks written from one image can share; it refuses a second copy
on the stick holding the first, by the file's SHA-256. A copy of a vault from Files is never written over the same vault (same
salt) on a stick unless this session unlocked or wrote that vault. The receipt is kept across a lock under the key `receipt` and is
gone at power-off. From it each vault file's currency is computed,
never stored (§3.5): **Never written** (sealed For the stick, on no
receipt), **On {label} · current** (the receipt wrote the bytes it has
now), **Changed since written** (open with unsaved changes, or sealed
bytes that differ from the receipt's). A vault copied in, unchanged and
on no receipt, reads **On {label} · current** for the stick it was read
from, which the app remembers with the SHA-256 of the bytes read (kept
across a lock under the key `vault-from`, gone at power-off), or
**Unchanged** when that stick is not known.

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
| 2 | Bitcoin key | 1 key: a words payload (34 bytes) or a seed payload (65 bytes), below; 2 label; 3 flags (`u8`: bit 0 not loaded at unlock, so the Load sheet an unlock shows starts it unticked, `docs/DECISIONS.md` F62; absent, or bit 0 clear, it starts ticked; bit 1 made in Faraday with its backup not done, so it signs nothing until it is, `docs/history/NEW-WALLET.md` §14.3); 4 BIP-39 passphrase, present only when the person chose to store it; 5 the key's codex32 set, a codex32 set payload (below): the set's plain facts and never a string (`docs/CODEX32.md` §9.4), present only for a key whose seed is split into a set Faraday knows of; a slot without it reads as before, and a build before it refuses a slot whose key record holds it |
| 3 | Wallet | 1 BIP-388 text, or the descriptor for a wallet whose key has no origin; 2 name; 3 flags (`u8`: bit 0 not loaded at unlock, as a key's; absent, it loads; F62, and a build before F62 refuses a slot whose wallet holds it) |
| 4 | Note | 1 the note, its UTF-8 bytes |
| 5 | Recovery sheet | 1 the sheet, as a sheet payload, below |
| 6 | Entry | 1 title (required); 2 username; 3 password; 4 URL; 5 notes; 6 TOTP secret, an `otpauth://` URI (a bare key is stored as one, `docs/TOTP.md` §4.3) |
| 7 | GPG key | 1 primary Ed25519 seed (32 bytes); 2 primary creation time (`u32`); 3 signing subkey seed (32 bytes); 4 subkey creation time; 5 user ID (repeatable); 6 expiry (`u32` seconds after creation, 0 for none); 7 the public certificate as last exported; 8 the revocation certificate made with the key, as OpenPGP packets, secret (F32); 9 when the key was marked revoked (`u32`) |
| 8 | Secure Boot keys | 1 owner GUID (16 bytes); 2 PK private key, PKCS#8 DER; 3 PK certificate, DER; 4 KEK key; 5 KEK certificate; 6 db key; 7 db certificate |
| 9 | Pending signing round | 1 scheme (`u8`: 1 MuSig2); 2 SHA-256 of the unsigned transaction it is bound to; 3 the key's fingerprint; 4 the round's secret state, as `faraday-psbt` serialises it; 5 used (`u8`), set when round 2 signs. Written by the flow in `docs/WALLETS.md` §4.2, compiled out of the app (F109); the app writes none and reads none into a spend |
| 10 | Signed amounts | 1 SHA-256 of an unsigned transaction; 2 the amount each input stated, `u64` per input in order (repeatable per record: one record per transaction; a slot holding two records for one transaction is refused, naming it). Written by `docs/WALLETS.md` §3.3 when that setting is on; the oldest records are dropped first when the slot is full |
| 11 | Backup plan | 1 the wallet, as a type 3 record's field 1 holds it; 2 the plan's answers, a line each (`name value`: `seeds`, `places`, `split`, `wallet`, `software`, `form`, `omit`, `vault v` the seeds vault *v* holds, `sticks v` the places that keep its stick, `pass i`, and `held {fp}` for each key of the wallet made in Faraday whose backup is not done, a fingerprint and never a secret, `docs/history/NEW-WALLET.md` §14.3, and once the map is edited on the chart a `spot` line per spot, `spot place0 w0 sheet stick0` (where, then what it holds in codes: `w`/`q`/`p` a seed's words, SeedQR or passphrase, `sheet`, `share`, `stick` a vault's stick, `seed`/`seedp` a seed in a vault without or with its passphrase, `vpass` a passphrase alone in a vault (a type 14 record), `wallet`, `file`), read back as the plan in place of the answers' rule, `docs/history/NEW-WALLET.md` §9.7; `preset k` the preset the plan started from and `custom 1` once the board was opened by Customize backup (`docs/BACKUP-FLOW.md` §17.6); the `layers` and `route` lines, and a `vpw` code in a `spot` line, that test builds wrote while the access layers were in (withdrawn 2026-10-19, `docs/DECISIONS.md` F38) are read past, never refused; a plan written before 2026-10-10 has one `sticks` line with no number and no `vault` line, and reads as one vault holding every seed here; one with no `spot` line is drawn from its answers); 3 a place's name (≤ 128 bytes, repeatable: one per place, in order, empty for "Place n"); 4 what one place, a vault, a stick of files, the software or the seeds on their own devices hold, a line (repeatable), a place named by number only ("Place n": the place's name is field 3's alone, and is never written here), and what a vault holds that a later plan dropped ("vault.ofv holds seed 9A6A2580"); a record written before 2026-10-10 may still name the place here instead of by number, and reading it into the kept summary (§11) replaces that name with its place number; 5 a note on a thing of the map, a line (repeatable, `docs/history/NEW-WALLET.md` §9.5): `checked place0 w0 2026-10-10`, `keychecked 9a6a2580 2026-10-10` (the date a key's holder confirmed its backup), `lost place0 w0`, `exposed place0 w0`, `holder 9a6a2580 Alice's Coldcard`, `vaultname 0 Cold storage`; never a secret, and never copied into the summary a lock keeps (§11). 6 retired: an access layer's name, written only by test builds while the access layers were in (withdrawn 2026-10-19, `docs/DECISIONS.md` F38); still allowed (≤ 128 bytes, repeatable) so a slot that holds one opens, and read by nothing. Written when the backup's checklist is made with a vault open (`docs/WALLETS.md` §5), and when the plan is changed or a note made on the wallet's chart, over the record this slot kept for the same wallet; the next backup of that wallet starts from it. The only place a place's name, a key's holder, a vault's name, a date checked and a mark are kept |
| 12 | (retired) | Was an APK signing key. No build knows it: a slot holding a type 12 record is refused, like any unknown type. The number is not used again. |
| 13 | Wallet share | 1 the share: a part of a multisig wallet's description with some keys left off, as the share's `.txt` file holds it (`docs/BACKUP-FLOW.md` §15.6); 2 the wallet's name; 3 which share, `u8`, from 1; 4 how many shares, `u8`; 5 the wallet, as a type 11 record's field 1 holds it, by which a share is told from another wallet's of the same name. Repeatable per slot, one record per share and wallet. Written by the backup checklist's vault item when the plan keeps a share in that vault; a build before 2026-10-15 refuses a slot that holds one |
| 14 | Passphrase | 1 the fingerprint of the key it is the BIP-39 passphrase of, the key's words with this passphrase (4 bytes); 2 the passphrase. Both required; at most one record per key and slot, and a slot holding two for one key is refused, naming the key. Written by the backup checklist's vault item when the plan keeps a seed's passphrase in a vault that does not hold the seed (`docs/BACKUP-FLOW.md` §17.12 item 4, `What::PassphraseSealed`); a vault that holds the seed keeps the passphrase in the key record's field 4 instead. Loading a key record with no field 4 from any open vault uses the passphrase a type 14 record in an open vault keeps for it, found by the fingerprint its words make with it. A build before F37 refuses a slot that holds one |
| 15 | Codex32 share | 1 the string, a codex32 string (BIP 93) in lower case, of a set whose threshold is 2 or more and never its secret (the secret is the seed, kept as a key record); 2 the fingerprint of the key its set makes (4 bytes). Both required; at most one record per share index and key in a slot. What the backup plan places as a string in a vault (`docs/CODEX32.md` §9.5, `What::Codex32Share` in a vault); written by the backup checklist's vault item and the flow's round, and gathered across the open vaults at unlock towards its key (`docs/CODEX32.md` §2); its string is secret (`faraday-keys`' `SECRET_FIELDS`). A build before it refuses a slot that holds one |

An open slot's view shows each type 3 wallet with the type 11 plan this
slot keeps for it, as a chart read from the backup up: the places,
vaults, files and software of the plan, the wallet's keys under them (a
key whose type 2 record is in this slot reads "In this vault"), the
wallet at the foot (`docs/history/NEW-WALLET.md` §7.2). A wallet with no type 11
record here reads "No backup plan in this vault". Each node opens its
sheet (§9); its actions load the wallet, and its seeds this slot keeps,
first. Opening one writes nothing into the slot.

In each type the first field is required and every other field is
optional; type 7's field 2, type 9's fields 2, 3 and 4, type 10's field 2, type 14's field 2 and type 15's field 2 are also required. A type 7 record without field 3, the signing subkey's seed, is kept and shown by its user IDs and signs, exports and renews nothing: the format keeps the field optional so that no slot is refused over it, and Faraday's own keys always write it. A build before F32 refuses a slot holding a type 7 record with field 8 or 9. Fields marked repeatable
may appear more than once; no other field may. A key's field 1 must be a
words payload or a seed payload, and a recovery sheet's field 1 a sheet
payload, exactly as laid out below.

**Payload layouts.** Five byte strings are the vault's own, the first four written and
read by `faraday/faraday-vault/src/layouts.rs`:

| Payload | Bytes |
|---|---|
| Words (a key record's field 1, 34 bytes) | the word count (`u8`: 12, 15, 18, 21 or 24); the word list's index (`u8`): 0 English, 1 Japanese, 2 Korean, 3 Spanish, 4 Chinese (simplified), 5 Chinese (traditional), 6 French, 7 Italian, 8 Czech, 9 Portuguese; the entropy (16 to 32 bytes, `count x 4 / 3` of them) zero-padded on the right to 32 bytes. Every pad byte must be zero. |
| Seed (a key record's field 1, 65 bytes) | the seed's length (`u8`, 16 to 64), then the seed zero-padded on the right to 64 bytes. Every pad byte must be zero. |
| Note (a note record's field 1) | the note's UTF-8 bytes. |
| Recovery sheet (a sheet record's field 1) | three fields in this order, the wallet's descriptor, its name and the sheet's note, each a little-endian `u16` length followed by that many bytes of UTF-8. Nothing may follow the third; the whole is at most 16 KiB, and a longer one is refused when read as when written. |
| Codex32 set (a key record's field 5, 7 bytes and 2 per string) | the threshold k (`u8`: 0, the secret alone, or 2 to 9); the identifier, four bech32 characters in lower case; each string's length in characters (`u8`: 48, 54, 61, 67, 74 or 127); how many strings exist whose share index is not known (`u8`); then for each string whose index is known, the index (a bech32 character in lower case) and its state (`u8`: 0 made, 1 placed, 2 lost, 3 exposed, 4 destroyed). At most 31 strings in all, each index once; at k = 0 the one string `s` and nothing else. No string is kept: the record holds the seed, and a split set's strings cannot be made again from it alone (`docs/CODEX32.md` §9.4). Written and read by `faraday/faraday-vault/src/codex32.rs`. |

A key record's field 1 of 34 bytes is read as words and any other length
as a seed.

Records appear in any order. A slot with no records holds only the format
byte and the end record, and is still a used slot.

## 8. Creating

The **Create vault** form takes the slot size, the Argon2id cost, and one
to four passphrases, each typed twice. It refuses an empty passphrase and two equal passphrases.
Every slot's label (type 1) starts as the vault's name, or "Vault" when
none is typed, whichever passphrase opens it (F57): a slot opened under
pressure names no other. A slot can be renamed once open.

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
exist or which slot a passphrase opens, and an opened slot's default
label is the vault's name, the same for every slot (§8). Unlocking takes the same work
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
`argon2-cffi` with `pynacl` or `cryptography` 44 or newer, as
`tools/vault/primitives.py` does. A vault therefore opens without
Faraday. It writes nothing, except with `--field TYPE.FIELD --out
FILE`: then it prints nothing of the
slot and writes that one field's bytes, of the record of that type
(`--index N` choosing among several), to a new file created with mode
0600, refusing a file that exists. That is how a secret field (a GPG key's
primary seed, type 7 field 1, for instance) reaches another tool without
Faraday. The vectors in `tools/vectors/vault/` are opened by both
implementations in the tests: a 64 KiB vault with one passphrase, and a
64 KiB vault with three, each slot holding one record of every type but the retired 12 and the codex32 ones; and a 64 KiB vault, `vault-codex32.ofv`, whose one slot holds BIP 93 vector 2's key with its codex32 set (type 2, field 5) and a codex32 share record (type 15).

## 11. What the app remembers of a locked vault

(owner, 2026-10-10; `docs/history/SIMPLIFY.md` §3.4, decision 4.) While a slot
is open, the app keeps a summary of it: the file's name and salt, the
slot's name, its wallets (name, shape and descriptor checksum), its keys
(each key's fingerprint and its 32-byte master id, the SHA-256 of the
master public key and chain code with the passphrase applied,
`wallet::master_id`; owner, 2026-10-21), how many entries it holds,
its backup maps' lines (record type 11, field 4), the checksums of the
wallets it keeps a plan for, and the time it was last open by this
computer's clock. The Backups screen lists a locked vault's wallets
from it (`docs/history/SIMPLIFY.md` §5), and its vault line says a seed is kept
in a locked vault only when the seed's master id, the session key's or
the one the backup's record kept, is the summary's for that key: the
fingerprint finds the candidate, the id says it is that seed (refuse
what cannot be verified rather than assume: F65's rule, which
`CLAUDE.md` keeps since F72). A
summary kept before the ids were verifies nothing, and the line reads
"A vault is locked". The summaries are rebuilt on unlock, after every
change made on a vault screen and as the session seals, and written
into the kept state at lock under the key `vault-summaries`
(`memory.rs`), which the next process reads back. They are gone at
power-off, with the rest of `/run/faraday`.

The Vaults list's row of a locked vault with a summary goes by the
slot's name ("Savings"), keeps its file name, and reads the summary in
place of its size and cost: "Savings 2 of 3 · key 9a6a2580 · 12 entries
· seen 14:02" (one or two wallets and keys by name, more by count).
Home's Unlock lead is "Unlock Savings" with the same line. A locked vault
with no summary keeps its row as before and reads "Unlock to see what
it holds".

**The trade-off.** Wallet names, shapes and descriptor checksums, key
fingerprints and master ids, the backup map's places and the slot's
name stay in RAM, and in `/run/faraday`,
while the vault is locked, until power-off. None of it spends: no seed,
no words, no entry's fields, no descriptor. It does say which slot was
open, which §9's deniability does not cover while the machine stays on;
powering off ends it.

## 12. Searching an open slot

An open slot's contents carry Material's search bar (DESIGN §4.3) at the
top of the items column for **Wallets**, **Entries** and **GPG keys**,
the kinds whose records hold a public title: an entry's title (type 6,
field 1), a wallet's name (type 3, field 2), a wallet share's wallet name
(type 13, field 2), a GPG key's user ID (type 7, field 5). Typing keeps
the items whose title holds what is typed, case not counted, as it is
typed; the × shows them all again; none found reads **No titles match**.
Usernames, URLs and notes are not searched: titles only. Nothing secret
is read to search: Notes has no bar, since a note's title is its first
line, a secret; Bitcoin keys have none, since a key is named by its
fingerprint, worked out rather than kept; Secure Boot holds one set.
The codes screen has a bar of its own (`docs/TOTP.md` §5.3).

What each bar finds is kept on the open slot beside what is read of its
records (`vault/search.rs`, `Found`), dropped by every change to the
records and to a bar's text, and found again before the next frame, so
drawing only looks it up. A lock empties both bars.
