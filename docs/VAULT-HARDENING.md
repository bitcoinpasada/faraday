# Vault hardening: the file's name, the passphrase, key commitment and the seal count

**Kind:** Spec
**Status:** Specified (F159); the passphrase in NFC (§3), the commitment block (§4), the seal count (§5.1 to §5.7) and a vault known by its contents (§2.1) built
**Open:** the outside review of the container (§10)

The vault's format is `docs/VAULT.md`. This spec changes it in four
places and renames its file. Every change was settled by the owner on
2026-10-28 (F159). No vault exists anywhere: Faraday has no users yet
(owner; the precedent is F144). Nothing here reads a vault of the
format before it, and no change carries a fallback, a migration or a
second reading path. A build of this spec refuses every file an earlier
build wrote, and an earlier build refuses every file this one writes
(§9.3).

`docs/VAULT.md` is being changed by other work at the same time (a fresh
nonce for every seal, and §9's rollback). This spec does not edit it;
§6 is the edit list the build applies to it.

## 1. What this assumes

**The seal's nonce is fresh** (in flight as this is written, F154,
`docs/OWN-RANDOMNESS.md`). Each seal takes its 24-byte nonce from a
fresh answer of the system's randomness, and falls back to the session's
seed and its count of seals only where no fresh answer is available.
This spec does not specify the nonce. It assumes only:

- `Opened::seal` in `faraday/faraday-vault/src/lib.rs` is still given
  the nonce by its caller, so the vault crate draws nothing (the crate's
  rule, its doc comment's last paragraph). Where the in-flight change
  moves the draw, §4 and §5 apply to whatever function writes the slot.
- Nothing here reads the nonce. The seal count (§5) is not a nonce and
  never feeds one: two seals with the same count under one key still
  take different nonces.

**The account of the construction** (`docs/CRYPTO-LEARN.md` §18) states
each change here as planned until the batch that builds it lands, and
that batch rewrites its sentences (§9.4).

## 2. The file's name and its magic

**The change.** A vault file's name ends in `.fvlt` (Faraday Vault), in
place of `.ofv`. Its first four bytes are `FVLT` (`46 56 4c 54`), in
place of `OFVT` (`4f 46 56 54`). The version byte after the magic is
kept as it is: 1. Nothing reads `.ofv` or `OFVT` afterwards: a file so
named, or so beginning, is a file Faraday does not read, as F144 has it
for the formats before Faraday's names.

**Why `.fvlt`** (owner, 2026-10-28). It matches the magic, and no other
use of it was found. `.fv` was considered and is taken: HEASARC's fv
scripts (Debian's `ftools-fv` package) and Fresh View's caches use it,
and EDK II's firmware volumes are written `.Fv`. `.ofv` predates
Faraday's name. shared-mime-info (2026-10-28) registers no glob for
`*.fvlt` or `*.fv`; a desktop file manager shows a `.fvlt` file as
unknown data, which is what a vault is to it.

**Why the magic changes now.** The format's bytes change anyway with §4,
and every vector is made again. `OFVT` predates the name, as `.ofv`
does. A four-byte magic keeps `HEADER_LEN` at 53 and every offset of
VAULT §1.

**On FAT.** `vault.fvlt` has a four-letter extension, so FAT keeps it as
a long name with the short name `VAULT~1.FVL` beside it, as it keeps
`savings-unsigned.psbt` (`faraday/faraday-fat/src/lib.rs`, `short_name`).
Faraday reads the long name. A copy made by a tool that writes short
names only is named `VAULT~1.FVL`; it is still a vault (§2.1).

### 2.1 A vault is known by its contents

(owner, 2026-10-28, 23:40 EDT.) A file the person selects, from Files,
the Inbox, a stick's listing or any other path that opens a file, is a
vault when its first four bytes are `FVLT` and its length is one of the
four vault sizes, `file_len(L)` for an *L* of `SLOT_SIZES` (VAULT §2),
whatever its name or extension: `vault.fvlt`, `VAULT~1.FVL`, `vault`, or
anything else. A file that matches neither stays refused as today: it is
a file Faraday does not read. `read_header` still checks the rest (the
version, the header's own *L* against the length, the cost) before any
work. `.fvlt` stays what Faraday writes, and what its lists group by.

The sites that read the magic and the size instead of the name:

- `faraday/faraday-intake/src/lib.rs`, `is_vault`: the magic and one of
  the four sizes (today the magic alone), which the Inbox and Files use
  to classify a file read;
- `faraday/faraday-files/src/lib.rs`: the writer's choice of the vault
  path (`name.ends_with(".ofv") && bytes.starts_with(b"OFVT")` becomes
  the bytes alone), `vault_identity` and `whole_vault` (the size rule
  added);
- `faraday/faraday-core/src/visit_taken.rs`, `is_vault(name, bytes)`:
  the bytes alone;
- a stick's listing (`faraday/faraday-core/src/lib.rs`, the stick kind
  by extension): a file whose size is one of the four vault sizes and
  whose name does not end in `.fvlt` has its first four bytes read once
  when the listing is made, and is listed as a vault when they are the
  magic. The read costs a file read; it is done once per listing, kept
  on the listing's state and never repeated while drawing;
- `faraday/faraday-core/src/boot_import/mod.rs`'s count of vaults and
  `faraday/faraday-core/src/stick/wipe.rs`'s `is_vault_name`: the
  listing's kind, so a vault under another name is counted and gets the
  wipe's vault warning.

The lists that group the files Faraday wrote stay by the name `.fvlt`:
`faraday/faraday-core/src/stick/pills.rs`,
`faraday/faraday-core/src/backup_flow/area.rs` and `steps.rs`.

**Tests** (`faraday/faraday-files/tests/`, `faraday/faraday-intake/tests/`,
`faraday/faraday-core/tests/all/stick_with_vault.rs`, as a person sees it):
a vault named `VAULT~1.FVL` and one named `vault`, with no extension,
are listed as vaults and open from Files, the Inbox and a stick's
listing; a file named `x.fvlt` holding other bytes is refused as a file
Faraday does not read; a file beginning `FVLT` whose length is not a
vault size is not a vault.

**The rename is mechanical** (batch 0, §9). Outside `docs/history/`,
`.ofv` appears 537 times in 159 files (2026-10-28), and `OFVT` in 23
places. `docs/history/` is not changed: it records what was.

1. Find every spelling:
   `git grep -n -i -e '\.ofv' -e '\bofv\b' -e 'OFVT' -e 'VAULT_MAGIC' -e 'OFV\b' -- ':!docs/history'`.
   The identifiers count: `vault_ofv` and `vault_file_ofv` in
   `faraday/faraday-strings/src/en.rs` become `vault_fvlt` and
   `vault_file_fvlt` with every use; the local `ofv` in
   `faraday/faraday-core/src/backup_flow/steps.rs` and the test helper
   `ofv` in `faraday/faraday-core/tests/all/backup_flow_wallet.rs`
   become `fvlt`.
2. The magic: `faraday_intake::VAULT_MAGIC`
   (`faraday/faraday-intake/src/lib.rs`), from which `faraday_vault::MAGIC`
   is taken; the literals `b"OFVT"` in `faraday/faraday-files/src/lib.rs`
   (three: the classifier, the header peek, the Inbox's vault test) and
   `faraday/faraday-core/src/visit_taken.rs`; `MAGIC` in
   `tools/vault/open.py` and `tools/crosscheck/area_vault.py`; the
   tests that write a fake header (`faraday/faraday-disk/tests/disks.rs`,
   `faraday/faraday-files/tests/oversized.rs`,
   `faraday/faraday-files/tests/replace.rs`,
   `faraday/faraday-storage/tests/all/vault_write.rs`,
   `faraday/faraday-core/tests/all/common.rs`,
   `faraday/faraday-intake/tests/all/split_seed.rs`).
3. The extension where a name is matched, made or split: the Inbox's and
   Files' classification (`faraday/faraday-files/src/lib.rs`, the
   `ends_with(".ofv")` beside the magic; `faraday/faraday-core/src/lib.rs`,
   the stick kind's `"ofv"`; `faraday/faraday-core/src/boot_import/mod.rs`,
   its count of vaults), the stick's pills and wipe
   (`faraday/faraday-core/src/stick/pills.rs`,
   `faraday/faraday-core/src/stick/wipe.rs`), the backup flow
   (`faraday/faraday-core/src/backup_flow/area.rs`, `steps.rs`), the
   visit (`faraday/faraday-core/src/visit_taken.rs`), the name a vault is
   written under (`faraday/faraday-core/src/vault/mod.rs`, `stem.ofv`
   and `stem-n.ofv`; `faraday/faraday-core/src/vault/area.rs`, the stem's
   longest length, which is measured before the extension and keeps its
   value; `faraday/faraday-core/src/vault/screen.rs`), the file menu's
   fixed extension (`faraday/faraday-core/src/file_menu.rs`), the test
   kit's and the budget tests' `vault.ofv`
   (`faraday/faraday-core/src/testkit.rs`,
   `faraday/faraday-core/src/work/budget.rs`,
   `faraday/faraday-core/src/work/budget/caches.rs`), the desktop shell
   (`faraday/shells/desktop/src/lib.rs`) and the Starter's install test
   (`faraday/shells/starter/tests/all/install.rs`).
4. The FAT tests' short names: `VAULT   OFV` in
   `faraday/faraday-fat/src/lib.rs`'s `short_names_are_unique_and_upper_case`
   becomes `VAULT~1 FVL` with a long name, and `VAULT.OFV` in
   `faraday/faraday-fat/tests/judged.rs` and
   `faraday/faraday-files/tests/replace.rs` becomes a name that tests the
   same rule (`VAULT.FVL`, an 8.3 name, against `vault.fvlt`'s short
   name).
5. The files whose names change (`git mv`), all made again in batch 1
   since their bytes change:
   - `faraday/faraday-samples/data/sample.fvlt`, written by
     `faraday/faraday-samples/examples/regenerate.rs` and named in
     `faraday/faraday-samples/src/data.rs` and
     `faraday/faraday-samples/tests/all/passphrase.rs`;
   - `tools/vectors/vault/vault-one.fvlt`, `vault-three.fvlt` and
     `vault-codex32.fvlt`, written by `faraday/faraday-vault/examples/vectors.rs`
     and named in `faraday/faraday-vault/tests/open_py.rs`,
     `tools/crosscheck/area_vault.py`, `docs/VAULT.md` §10 and
     `docs/CODEX32.md`;
   - the fuzz seeds `faraday/fuzz/corpus/classify_file/vault-head.fvlt.bin`
     and `faraday/fuzz/corpus/vault_header/vault-one-head.fvlt.bin`,
     whose first bytes are the magic and are written again with it.
6. The tests that read a vault by name, which change with it: every
   file `git grep -l '\.ofv' -- '*tests*' '*testkit*'` lists, 81 files
   on 2026-10-28, 69 of them in `faraday/faraday-core/tests/all/` (from
   `amounts.rs` to `wallet_names.rs`), the rest named in items 2 to 5.
   A test's name or assertion that says "ofv" says "fvlt".
7. The text a person reads: the strings in
   `faraday/faraday-strings/src/en.rs`; the guide's pages, 70 mentions
   in `docs/guide/screens/` and `docs/guide/walk/` (from
   `back-up-wallet.md` to `vaults-and-passwords.md`); the Learn page
   `docs/learn/faraday/vaults.md` ("Its file ends in .ofv") and every
   other `docs/learn/` mention, through `just learn-import`; the docs
   (`docs/VAULT.md`, `docs/BACKUP-FLOW.md`, `docs/FLOWS.md`,
   `docs/WALLETS.md`, `docs/SPEND-FROM-BACKUP.md`,
   `docs/LARGE-FILES.md`, `docs/CODEX32.md`); `docs/decisions/F54.md`, the one decision that
   names it, keeps `.ofv` where it records what was decided then, as
   decisions are records.
8. The pictures: the snapshot tours that name a vault file
   (`faraday/shells/snapshot/src/tours/`: `again.rs`, `boot.rs`,
   `create_from.rs`, `default.rs`, `desktop.rs`, `flow.rs`,
   `from_vault.rs`, `guide/files_and_sticks.rs`,
   `guide/screen_states.rs` and the others the grep lists), and the
   guide's pictures that show a file name, drawn again with
   `just guide-pictures`: every picture under `docs/guide/pictures/`
   whose bytes hold `ofv` (`4-4-dropped.png` and
   `multisig-with-others-who-signs.png` among them, found by
   `git grep -l -a ofv -- docs/guide/pictures`) and every picture of
   Files, the visit, the backup flow and Vaults that shows one. The
   longer name (`vault.fvlt` is a character longer than `vault.ofv`)
   can change a line's wrapping on the Pi's panel: one tour of Files and
   one of the visit are rendered at 240 x 320 and looked at
   (`just faraday-snap 240x320 <tour>`).
9. The website's own repository names `.ofv` in its pages; that is its
   own change, made when it next takes a pin.

The rename changes no behaviour, so it adds no test: the tests above
keep passing with the new name, and the classification's tests
(`faraday/faraday-files/tests/`) now prove a `.fvlt` file is a vault and
an `.ofv` file is not.

## 3. The passphrase in NFC

### 3.1 The change

Every vault passphrase is put in Unicode Normalization Form C (NFC,
Unicode Standard Annex #15) before Argon2id, at creation and at opening
(coordinator, 2026-10-28, on the owner's call for NFC alone). Argon2id
takes the NFC form's UTF-8 bytes. Nothing else is done to it: no
character is mapped or refused, and case and spaces are kept as typed.
The normalisation is in `faraday-vault`, so no caller can skip it:

- `create`, `open` and `derive` take each passphrase as `&str`, not
  `&[u8]`, and normalise it first;
- `create` compares the NFC forms for `SamePassphrase`, so two
  passphrases that differ only in their Unicode form are refused as the
  same (two slots under one key would be a file that never opens, VAULT
  §4 step 4);
- the NFC form is held in a `Zeroizing<String>` and wiped when the key
  is derived. The normaliser's own small buffer of combining marks is
  not wiped; `faraday-bip`'s NFKD of a BIP 39 passphrase has the same
  buffer.

`faraday-keys` (`faraday/faraday-keys/src/vault.rs`, `create` and
`OpenVaults::unlock`) passes the typed `&str` through, and
`tools/vault/open.py` normalises the passphrase it reads the same way.
The in-tree `unicode-normalization` 0.1.25 (Unicode 17.0.0, used by
`faraday-bip` for BIP 39's NFKD) gives NFC; no dependency is added.

**NFC, not NFKC.** NFC keeps compatibility characters as typed: the
full-width "Ａ" (U+FF21) and the ligature "ﬁ" (U+FB01) stay distinct
from "A" and "fi". RFC 8265's OpaqueString profile for passwords uses
NFC for that reason. NIST SP 800-63B's password guidance names NFKC or
NFKD for verifiers; Faraday follows the owner's choice, NFC, and the
account says which (CRYPTO-LEARN §18.3). What OpaqueString adds beyond
NFC is not done now (§12).

### 3.2 Format version

No change of version. An ASCII passphrase is its own NFC form and
derives the key it did before, and the vectors' passphrases are ASCII.
Nothing in the file records the normalisation: it is part of what `K`
is computed from, as VAULT §3 writes it.

### 3.3 Tests

- **A known answer** (`faraday/faraday-vault/src/tests.rs`,
  `a_passphrase_opens_in_either_unicode_form`): a vault made with
  "café" as U+0063 U+0061 U+0066 U+00E9 opens with U+0063 U+0061
  U+0066 U+0065 U+0301, and one made with the second opens with the
  first; each opens the same slot and reads the same contents.
- **The vector:** tools/vectors/vault/vault-nfd.fvlt (new, made by
  `faraday/faraday-vault/examples/vectors.rs`): one slot, made with the
  NFD passphrase. `faraday/faraday-vault/tests/open_py.rs` opens it with
  `tools/vault/open.py` under the NFC and the NFD form.
- **Two forms are one passphrase:** `create` with "é" in NFC for one
  slot and NFD for another is `SamePassphrase`
  (`create_refuses_what_it_cannot_keep` gains the case). A no-break
  space and a space are two passphrases, as NFC keeps them apart.
- **The cross-check** (`tools/crosscheck/area_vault.py`). The reference
  normalises with `unicodedata.normalize("NFC", …)`. Its cases: NFD
  input (e and U+0301; A and U+030A; Hangul syllables as conjoining
  jamo L V T), singletons NFC replaces (U+212B ANGSTROM SIGN, U+2126 OHM
  SIGN), two combining marks in either order (U+0061 U+0323 U+0302 and
  U+0061 U+0302 U+0323 both give U+1EAD), and compatibility characters
  NFC keeps (U+FB01, U+FF21). Random cases draw code points assigned in
  Python's `unicodedata.unidata_version`. The Faraday driver reports
  `unicode_normalization::UNICODE_VERSION`; where Python's version is
  newer than the crate's, the area stops and names both, since a code
  point assigned only in the newer could compose on one side alone.

## 4. Key commitment

### 4.1 The change

XChaCha20-Poly1305 does not commit to its key: a ciphertext and tag can
be made that verify under two chosen keys, and decrypt to different
plaintexts under each (Len, Grubbs and Ristenpart, "Partitioning Oracle
Attacks", USENIX Security 2021, give the construction for Poly1305).
For a vault, that is one slot that opens under two passphrases, each
showing other contents. Faraday already refuses a file where one
passphrase opens two slots; it cannot refuse one slot two passphrases
open, because each opening sees one key.

The fix is Albertini, Duong, Gueron, Kölbl, Luykx and Schmieg's padding
fix ("How to Abuse and Fix Authenticated Encryption Without Key
Commitment", USENIX Security 2022): the plaintext starts with a run of
zero bytes, and decryption requires them back as zeros. Under a second
key the same ciphertext decrypts those bytes to other keystream, so a
slot that verifies under two keys shows zeros under at most one, unless
the two keys' keystreams agree in those bytes.

### 4.2 Its size and place

- `COMMIT_LEN = 32` (new constant in `faraday/faraday-vault/src/lib.rs`):
  32 zero bytes, 256 bits. Two keys whose first 32 keystream bytes
  agree under one nonce are a collision of 256 bits: about 2^128 trials
  by the birthday bound, each of which is an Argon2id at the vault's
  cost, since a slot key exists only as a passphrase's. The paper's own
  bound for this length under ChaCha20 is a question for the outside
  review (§10).
- **Where:** the first 32 bytes of the slot's plaintext. In
  XChaCha20-Poly1305 the plaintext is encrypted from the ChaCha20 block
  counter 1 (block 0 makes the Poly1305 key;
  `draft-irtf-cfrg-xchacha-03`, and `chacha20poly1305` 0.11.0 seeks to
  64 bytes), so the zero block's ciphertext is the first 32 bytes of
  block 1's keystream under HChaCha20(`K_slot`, the nonce's first
  16 bytes).
- **The slot's plaintext** becomes:

  | At | Bytes | What |
  |---|---|---|
  | 0 | 32 | zero (the commitment block) |
  | 32 | 1 | slot format, 1 |
  | 33 | … | records, then the end record |
  | … | … | zero, to *L* |

- ***L* and the file sizes do not change.** *L* stays the slot's
  plaintext length and the file stays 53 + 4 × (*L* + 40) bytes, so
  VAULT §2's table and `SLOT_SIZES` stand. The room for records falls
  by 32 bytes in every size: `Contents::encode` is given *L* − 32.
  The fullness checks (`Error::Full`, VAULT §5's room left, the Lock
  sheet's bytes over) read the room from the one function that gives it,
  so they move with it.
- **Who writes it:** `encrypt_slot` writes the zero block and the
  encoded contents after it; `decrypt_slot` checks it.
- **Opening:** after a tag verifies, the 32 bytes are checked to be zero
  in constant time (an OR of every byte, compared once). A slot whose
  tag verifies and whose block is not zero is `Error::Commitment`, and
  the file is refused, as two slots opening is: this format never writes
  one. A slot whose tag fails is not opened, as now.
- **Version:** stays 1 (§9.3).

### 4.3 Tests

- **A slot valid under two keys is refused**
  (`faraday/faraday-vault/src/tests.rs`,
  `a_slot_made_for_two_passphrases_opens_under_neither`): the vector
  tools/vectors/vault/two-keys.fvlt (new), made by a script beside the
  vectors from Len, Grubbs and Ristenpart's method: two passphrases are
  stretched at the vector's cost, their slot keys and the slot's nonce
  give the two Poly1305 keys, and one free 16-byte ciphertext block is
  solved so that the tag verifies under both. The cross-check area
  first proves the vector is what it says: PyNaCl's XChaCha20-Poly1305
  verifies the slot under each key. Then Faraday's `open` under each
  passphrase is `Commitment`, and so is `tools/vault/open.py`'s.
- **A slot with a non-zero block** (made honestly under one key with its
  first byte 1) is `Commitment`, and its contents are never parsed.
- **Every vault Faraday makes opens:** the existing round trips
  (`each_passphrase_opens_its_own_contents`,
  `sealing_changes_only_the_open_slot`, `records_of_every_type_read_back`)
  run unchanged over the new layout, and the regenerated vectors open in
  both implementations.
- **The room:** a slot filled to *L* − 32 seals, one byte more is `Full`
  (`faraday/faraday-core/tests/all/vault_full_slot.rs`'s bytes move by 32).
- **The cross-check** reproduces `create` and `seal` byte for byte with
  the block, and refuses each malformed block it is given.

## 5. The seal count

### 5.1 The record

A new record type, **16, Seal count**, in every slot:

| Field | What |
|---|---|
| 1 | the count: how many times this slot has been written, `u64`, 1 or more. Required. |
| 2 | the time of the last seal, `u64` seconds since 1970-01-01 UTC by the computer's clock that sealed it; absent when that clock was not set. |

- Exactly one per slot. A slot without one, or with two, is refused
  (`Fault`): no vault exists without it (§9.3).
- **The vault crate writes it, and no caller does.** `create` writes
  count 1 in every used slot. `Contents::decode` takes the record out of
  the contents and gives it to `Opened`, which keeps the count and time
  it was opened with (`Opened::seals()`). `Opened::seal` takes `&mut self`
  and the time (`now: Option<u64>`), writes count + 1, and keeps the new
  count, so two seals of one opening count 2. A `Contents` given to
  `create` or `seal` that holds a type 16 record is `Error::Contents`.
- **What it does not show.** The record is inside the slot, encrypted:
  no one without the passphrase reads it, and the file's size, its other
  slots and its header do not change with it.

### 5.2 Shown at unlock

Unlock's result and the open vault's head carry one line of values:
"Sealed 12 times · last 28 Oct 2026, 14:02" (the time by the clock that
sealed, as the clock's own; "last time not recorded" when field 2 is
absent). Strings: `vault_seals_one` ("Sealed once"), `vault_seals` ("Sealed {} times"), `vault_sealed_last`
("last {}"), `vault_sealed_no_time` ("last time not recorded"),
`vault_seals_older` and `vault_seals_newer_copy` (§5.3), in
`faraday/faraday-strings/src/en.rs`.

### 5.3 Comparing copies

The app keeps, from power-on, the highest count it has seen for each
slot it opened: by the vault's salt and *L* (VAULT §6's identity) and the
slot's position, which is never shown. It is kept with the summary of
VAULT §11 (a field of the summary, kept across a lock under
`vault-summaries`, gone at power-off), raised at every unlock, every seal
and every copy read below. Kept with it: the SHA-256 of the slot's 40 +
*L* bytes at that count, so two different seals with one count are told
apart.

The comparisons:

1. **Unlock.** A slot opened with a count lower than the highest seen
   for it opens, and Unlock and the vault's head say so:
   "An older copy: sealed 9 times. A copy sealed 12 times was open since
   power-on." The same count with other bytes: "Another copy was sealed
   12 times too, with other changes."
2. **At unlock, the other copies in Files.** A stick visit cannot read a
   count: a process that has unlocked a vault holds its key and is never
   handed a stick (`docs/history/PLAN.md` §5.1), and a visit's process
   holds none. So at unlock, every other copy of the same vault (same
   salt and size) in the Inbox or the Outbox has the opened slot read
   with the key just derived (`Opened::copy_seals`, one decryption each,
   no Argon2id): a higher count says "An older copy: sealed 9 times. A
   copy sealed 12 times is in Files."; the same count with other bytes
   says "Another copy was sealed 12 times too, with other changes". To
   compare a stick's copy, copy it into Files before unlocking.
3. **A backup check.** BACKUP-FLOW §18.3's check "A vault's stick read on
   a stick visit, and the vault unlocked from it" dates that stick
   checked only when its unlock names no older copy.
4. **The vault locked.** A copy's count cannot be read without the slot
   key, which goes at the lock. The visit compares the file's SHA-256
   against the receipt, as VAULT §6 does now, and says nothing of
   counts.

### 5.4 What it detects, and what it cannot

**It detects:**

- an older copy of a slot opened after a newer one was opened, sealed or
  read in the same power-on (comparison 1);
- an older copy opened while a newer copy of the same vault is in Files
  (2): the case where a person carries on from an old stick at one place
  while another place holds later changes;
- two copies sealed from one copy in different places, by their equal
  count and different bytes (1, 2).

**It cannot detect:**

- a rollback when no newer copy is seen in the same power-on and no open
  vault's backup plan records a newer one (§5.6): an older copy put back
  while the computer is off, opened after power-on, with no newer copy
  read since, reads as current. Faraday keeps nothing else across
  power-off (VAULT §11);
- a rollback of every copy at once, at every place;
- a rollback of a slot this session did not open: the count of another
  passphrase's slot is that slot's, read only with its passphrase;
- anything while the vault is locked (5.3, item 4), and so at a stick
  visit, which never has the key;
- a stick's copy not copied into Files before unlock (2): it is not
  read, and a visit can write an older copy over it;
- whether an older copy is an attack or a place not yet visited: both
  read "older"; the count says which is newer, not why;
- the time: field 2 is the sealing computer's clock, which can be wrong
  or unset (a Raspberry Pi has no clock that keeps time when off). It is
  shown, never compared.

**What it costs.** The count says how often a slot is written. A person
who opens a decoy slot under pressure shows its count beside the slot
they use daily, which they do not show; a decoy sealed twice in a year
says it is little used. That is the person's to arrange, as the decoy's
contents are (VAULT §9), by opening and sealing the decoy. The account
says so (CRYPTO-LEARN §18.3).

**What no one can do with it.** Raise a copy's count without its
passphrase: the record is inside the slot's ciphertext, and the tag
covers it.

### 5.5 Tests

- `faraday/faraday-vault/src/tests.rs`: a new vault's slots open with
  count 1; each seal of one opening adds one, and the time sealed is
  read back; a slot with no type 16 record, or two, is refused; a
  `Contents` holding one is refused by `create` and `seal`.
- The vectors carry the record; the cross-check reads and writes it on
  both sides, and its random slots hold one.
- `faraday/faraday-core/tests/all/vault_seal.rs` (or a new
  `vault_seal_count.rs`), as a person sees it:
  - Unlock of a new vault shows "Sealed once" (`vault_seals_one`), and
    after a second seal "Sealed 2 times";
  - unlock a copy, seal it twice, lock, unlock the first copy: Unlock says
    it is older;
  - another copy of the vault in Files with a higher count: Unlock says
    it is older and where the newer one is; the newer copy opened with
    the older beside it says nothing;
  - two copies sealed from one copy: the second unlocked says another was
    sealed the same number of times;
  - power off (a new process without the kept state), with no vault
    holding a plan that names it open: the older copy opens with no
    warning, which is the limit 5.4 states.

### 5.6 The count in the backup plan

(owner, 2026-10-28, 23:45 EDT.) A backup plan's record (type 11)
carries the seal count of the other vaults it places, so a copy can be
named older across a power-off, from another vault's record.

**The line.** In field 2, one line per copy known:

`seals <salt> <L> <position> <count> <digest>`

the vault's salt (64 hex digits) and *L* (decimal), its slot's position
(0 to 3), the slot's seal count, and the SHA-256 of the slot's 40 + *L*
bytes at that count (64 hex digits). These are §5.3's fields: salt and
*L* are the vault's identity on a stick (VAULT §6), the position picks
the slot, whose count is its own, and the digest tells two seals of one
count apart. The plan's vault number `v` is not used: no file on a stick
can be matched to it. Fewer fields would not do: without the position a
decoy slot's count would be compared against the plan's slot's; without
the digest the same-count outcome is lost.

**Writing.**

- A line is written for each vault the plan places that is not the
  vault the record is in, and whose copy this session knows: one the
  backup flow made or sealed for this plan (`flow_sealed`), or an open
  vault holding this plan's wallet. Its count and digest are the highest
  this session has seen for it (§5.3), counting a seal at this lock.
- No line names the record's own vault: its digest would be of the
  ciphertext that holds the line itself, and a line about its own vault
  rolls back with it.
- A line for a copy not seen this session is carried as it was. A line
  is replaced only by one with a higher count, or the same count from
  this session.
- Lines are written whenever a type 11 record is written (the checklist
  made, the plan changed, a note on the chart), and at every lock before
  sealing. At a lock, the vaults are sealed in the order they were
  opened, each vault's lines taking the new count and digest of the
  vaults sealed before it, and the earlier state of those sealed after
  it. Nothing is circular, and a vault whose lines change at the lock is
  sealed with the rest.
- `faraday/faraday-core/src/plan/text.rs`'s `Answers::from_text` reads
  `seals` past, as it does `held` (`"seals" => continue`), and the plan
  area reads the lines itself.

**Reading.** At an unlock, and whenever a vault holding a plan opens
while others are open, each `seals` line in an open vault's plans is
compared with each open slot of the same salt, *L* and position, and is
treated as a copy seen (it raises §5.3's highest seen). The outcomes are
§5.3's:

- the line's count is higher: the slot is an older copy: "An older copy:
  sealed 9 times. {vault name}'s backup plan records a copy sealed 12
  times";
- the same count and another digest: "Another copy was sealed 12 times
  too, with other changes";
- otherwise the copy is current, and nothing is said.

Never refused: the slot opens and stays open.

**Older builds.** A `seals` line is an answer line an earlier build does
not know, and `Answers::from_text` returns `None` for an unknown key
(`_ => return None`): an earlier build would not read the plan. No
earlier build reads an `FVLT` file at all (§9.3), so none meets one.

**What it still cannot detect:**

- a vault no open vault's plan names;
- a plan rolled back with its own vault: an older copy of vault A holds
  older lines for B, and says nothing of B's later seals;
- every vault rolled back together;
- a slot the plan does not place (another passphrase's slot);
- a vault opened while no vault holding a plan that names it is open;
  the comparison runs when that vault is opened later in the session.

**What it reveals** to whoever opens the slot holding the plan:

- the other vaults' salts and sizes. They already learn from the plan
  which vaults exist and where their sticks are kept; the salt and size
  let them pick out those files on a stick;
- the position of the slot of each other vault that holds the plan's
  seeds, and how many times it was sealed. Someone who also holds that
  vault and watches a decoy slot opened in it under pressure can tell,
  from a position that differs, that the decoy is not the plan's slot.
  That is the cost of naming a copy across vaults.

**Tests** (`faraday/faraday-core/tests/all/`, as a person sees it):

- a plan placing two vaults, both sealed: each one's record holds a line
  for the other with its count and digest, and none for itself;
- power off; unlock vault A (with the plan), then an older copy of B:
  Unlock names B older, from A's plan;
- unlock B's older copy first, then A: the warning appears when A opens;
- B at the plan's count with other bytes: "sealed 12 times too";
- B current: nothing is said;
- an older copy of A with its older line for B: no warning on B, the
  limit stated above;
- a `seals` line is kept across a plan edit that does not see that vault.

### 5.7 The Learn page

(owner, 2026-10-28, 23:38 EDT.) The vaults Learn page,
`docs/learn/faraday/vaults.md` (Faraday's own page, read into
`faraday/faraday-learn/src/lib.rs`'s `OWN` table by `include_str!`),
gains a section **Older copies**, after **Slots and decoy passphrases**.
It lands with batch 2b, since a Learn page says only what is built. Its
text:

> ## Older copies
>
> Unlock shows how many times a vault has been sealed: "Sealed 12 times
> · last 28 Oct 2026, 14:02". A seal is a save: Faraday seals a vault
> when it locks it with changes. Opening a vault and changing nothing
> does not count. The count is inside the encryption, so no one can
> change it without the passphrase.
>
> Someone who can write to a stick can put an older copy of a vault
> back. They cannot read it or change what is in it, but the older copy
> opens. Compare the count with how often you remember changing the
> vault: a count lower than you expect is an older copy.
>
> Faraday remembers the counts it has seen until the computer is turned
> off. Opening an older copy after a newer one in that time says so: "An
> older copy: sealed 9 times."
>
> A backup plan kept in one vault records the counts of the other
> vaults it uses. Open the vault with the plan first, then the others: a
> vault older than the copy the plan records is named as older, even
> after the computer was turned off. With one vault, its plan is in the
> same file and goes back with it; then the count at unlock, against
> what you remember, is the check.
>
> Unlock also reads the other copies of the vault in Files, with the
> passphrase just typed. If one has been sealed more times, Unlock says
> "An older copy: sealed 9 times. A copy sealed 12 times is in Files."
> Open that copy to carry on from it.
>
> The count cannot show every copy put back at once, at every place; a
> vault no backup plan names; or another passphrase's slot, whose count
> only that passphrase reads.
>
> A decoy slot has its own count. Opened under pressure, a decoy sealed
> twice beside a vault used every day shows it is little used. Change
> something in the decoy now and then, so its count grows.

The guide's pages that cover Unlock and the visit say the same in their
own words and pictures, in batch 2b: `docs/guide/screens/vaults.md`
("Unlock": the Sealed line and the older-copy line),
`docs/guide/screens/stick-visit.md` (not changed: a visit reads no
counts), and `docs/guide/walk/vaults-and-passwords.md`
(§3 "Unlock it" and §10 "Unlock it again, and load from it", whose
pictures show the line), the pictures drawn again with
`just guide-pictures`.

## 6. VAULT's edit list

For the build to apply to `docs/VAULT.md` (not edited here; see the
introduction):

- **Preamble:** the reference reader's options unchanged; "A vault is
  one file" unchanged.
- **§1:** magic `FVLT` (`46 56 4c 54`); version 1 unchanged.
- **§3:** "The passphrase is its UTF-8 bytes, unnormalised." becomes
  "The passphrase is put in Unicode NFC, and Argon2id takes the NFC
  form's UTF-8 bytes; two passphrases with one NFC form are one
  passphrase."
- **§4 step 3:** "The work is the same whichever slot opens and whether
  any does" is not exact today: `chacha20poly1305` decrypts only a slot
  whose tag verifies, so a passphrase that opens none skips one slot's
  decryption and the parse. It becomes "The work is the same whichever
  slot opens; a passphrase that opens none does not decrypt or parse a
  slot, which the result says anyway."
- **§4 step 4:** add the commitment block: a slot whose tag verifies and
  whose first 32 bytes are not zero refuses the file.
- **§5:** sealing writes the seal count (type 16) as count + 1 and the
  time; the comparisons point to §11.
- **§6:** step 1 says the visit reads no counts, since it holds no key
  (this spec §5.3 item 2).
- **§7:** the slot plaintext table gains the 32 zero bytes at 0 and
  moves the format byte to 32; the records' table gains type 16; "Every
  byte after it must be zero" unchanged; "a slot with no records holds
  only the format byte and the end record" becomes "holds the
  commitment block, the format byte, its seal count and the end record".
- **§9 Holds/Does not hold:** a slot is committed to its key; the seal
  count is readable to whoever opens the slot (5.4's cost).
- **§10:** the vectors are `.fvlt` and add vault-nfd.fvlt and
  two-keys.fvlt.
- **§7, type 11:** field 2's list of lines gains `seals salt L position
  count digest`, the seal count of another vault's copy (this spec
  §5.6), read past by the plan's answers and read by the plan area.
- **§11:** the summary keeps the slot's highest seal count and its
  slot's SHA-256 (5.3).
- **§1 or §6:** a vault is recognised by its magic and a vault size,
  whatever its name (§2.1).
- **Every `.ofv`:** `.fvlt` (batch 0).

## 7. The other files

| File | Change | Batch |
|---|---|---|
| `faraday/faraday-vault/src/lib.rs` | `Opened::position()` and a public `slot_range(slot_len, i)`, for the digest of §5.3 and §5.6 (a SHA-256 over up to 4 MiB: costly, counted as `FileHash`); `COMMIT_LEN`; `encrypt_slot` and `decrypt_slot` write and check the block; `Error::Commitment`; `create`, `open`, `derive` take `&str` and put it in NFC; `Opened` keeps the seal count and time, `seal(&mut self, …, now)`; the crate doc's list | 1 |
| `faraday/faraday-vault/src/records.rs` | type 16; `Contents::encode` given *L* − 32; decode takes the seal record out | 1 |
| `faraday/faraday-vault/Cargo.toml` | `unicode-normalization` (workspace, no default features), already vetted for `faraday-bip` | 1 |
| `faraday/faraday-vault/src/tests.rs` | §3.3, §4.3, §5.5 | 1 |
| `faraday/faraday-vault/examples/vectors.rs`, `faraday/faraday-vault/tests/open_py.rs`, `tools/vectors/vault/` (its README and `slots.json`) | the vectors made again; the two new vectors | 1 |
| `tools/vault/open.py`, `tools/vault/primitives.py` | the magic, NFC, the block, type 16 | 1 |
| `tools/crosscheck/area_vault.py` | §3.3's and §4.3's cases; its docstring's account of the format | 1 |
| `faraday/faraday-keys/src/vault.rs` | `&str` passphrases; `sealed` passes the time; the copy-read of §5.3 (`OpenVaults::copy_seals`, costs one decryption) |2b |
| `faraday/faraday-core/src/vault/mod.rs`, `area.rs`, `screen.rs` | the count shown; the highest seen kept on the summary; Unlock's lines |2b |
| `faraday/faraday-core/src/visit_taken.rs` and the visit's writing | §5.3 items 2 and 3 |2b |
| `faraday/faraday-core/src/memory.rs` | the summary's new fields under `vault-summaries` | 2b |
| `faraday/faraday-core/src/plan/text.rs`, `faraday/faraday-core/src/plan/` and the backup flow's record writing | §5.6's lines: read past by the answers, written and read by the plan area | 2b |
| `faraday/faraday-intake/src/lib.rs`, `faraday/faraday-files/src/lib.rs`, `faraday/faraday-core/src/visit_taken.rs`, `faraday/faraday-core/src/lib.rs`, `faraday/faraday-core/src/boot_import/mod.rs`, `faraday/faraday-core/src/stick/wipe.rs` | §2.1: a vault by its magic and size | 2a |
| `faraday/faraday-strings/src/en.rs` | §5.2's strings |2b |
| `docs/crypto/actions.toml` | vault-create, vault-unlock and vault-seal: their limits, own calls (NFC is Neutral; the commitment block Conservative; the seal count Neutral), tests and standards (Unicode UAX #15) added to the standards table; re-stamped by the recipe | 1, 2 |
| `docs/SECURITY-MODEL.md` "Vault cryptography" | the passphrase in NFC, the commitment block and the seal count; its "random 24-byte nonce, drawn fresh every time the vault is sealed", which the in-flight nonce work makes true | 1, 2 |
| `docs/learn/faraday/vaults.md` | the extension; its "Passphrase strength" section moves into the account (CRYPTO-LEARN §18.4), its facts kept | 0, 3 |
| `docs/learn/faraday/vaults.md` | §5.7's section **Older copies**, after **Slots and decoy passphrases** | 2b |
| `docs/guide/screens/vaults.md`, `docs/guide/screens/stick-visit.md`, `docs/guide/walk/vaults-and-passwords.md` | the Sealed line at Unlock, the older-copy line, a newer copy not written over at a visit; their pictures drawn again | 2b |
| `docs/BACKUP-FLOW.md` §18.3 | the check's row: dated only for the highest count seen (§5.3 item 3) |2b |
| `docs/AUDIT.md` §3 | generated: the commitment block is written here | 1 |

## 8. Tests that are not added

No test states a constant (`COMMIT_LEN`'s value) or the layout against
this document: the vectors and the second implementation hold the bytes,
and the tests above are what a person or a second reader notices.

## 9. Order of work

### 9.1 Batches

The vault work is to be built in one night (owner, 2026-10-28). Six
batches, each passing `just` alone:

0. **The rename** (Sonnet, mechanical): §2's list, `.ofv` to `.fvlt`
   and `OFVT` to `FVLT`; the vectors made again by
   `faraday/faraday-vault/examples/vectors.rs` with the new magic, the
   cross-check stamp renewed with `just crosscheck`, and the pictures
   drawn again.
1. **The format** (Opus for the vault crate, its NFC and the two-key
   vector; Sonnet for open.py, the area and the docs): §3, §4, §5.1 in
   `faraday-vault`, `Opened::position` and `slot_range`, the vectors,
   the reference reader, the cross-check, VAULT's §1, §3, §4, §7 and §10
   edits, the manifest's entries re-stamped. One batch, since the
   vectors are made once with all three changes.
2. **The app**, in two parts:
   - 2a, **a vault by its contents** (Sonnet): §2.1's sites and tests;
   - 2b, **the count in the app** (Sonnet, from §5.2 to §5.6): the count
     shown, the highest seen, the visit's comparisons, the plan's
     `seals` lines, BACKUP-FLOW §18.3's row, VAULT §5, §6, §7's type 11
     row, §9 and §11, the Learn page's **Older copies** and the guide's
     pages (§5.7).
3. **The account** (`docs/CRYPTO-LEARN.md` §18): its manifest form,
   check.py's and gen.py's rules, faraday-learn's pages, the sentences
   marked planned.
4. **The outside review** (§10), after 1 and 2.

**What runs together and what merges when:**

- 0 and 3 start together; neither touches the other's files, and each
  merges on its own.
- 1 starts with them, on the same base. It merges after 0, rebased: the
  only overlap is the magic, the vectors' names, `tools/vault/open.py`
  and `tools/crosscheck/area_vault.py`, and its vectors are made again
  after the rebase.
- 2a starts after 0 has merged (it needs the new name and magic) and
  merges on its own; it does not wait for 1.
- 2b starts once 1's `faraday-vault` interface is in place
  (`Opened::seals`, `Opened::seal(&mut self, …, now)`,
  `Opened::position`, `slot_range`), and merges after 1.
- The last to merge runs one `just` over the whole, and
  `just crosscheck --fresh`, as for a release, since the format changed
  (F79).

### 9.2 Within the in-flight work

Batch 1 lands after the fresh-nonce change, since both change
`Opened::seal` and VAULT §5; it takes that change's signature as it
lands.

### 9.3 Why no version change

Every file this spec's builds write begins `FVLT`, which no earlier build
reads (`Magic`); every earlier file begins `OFVT`, which this build does
not read. Within `FVLT`, there is one format: the one this spec ends
with. Batch 0 writes `FVLT` files of the old slot layout for the span of
one commit; no build of that commit is released, and batch 1's vectors
replace them. The version byte stays 1.

### 9.4 The account's sentences

Each batch that lands rewrites the account's planned sentences for its
change from planned to built (CRYPTO-LEARN §18.3 marks which), and
`docs/crypto/check.py` fails while a sentence is marked planned for an
item this spec's Open line no longer names (CRYPTO-LEARN §18.6).

## 10. The outside review of the container

**Scope.** The container only: the header, the slot key, the slots'
encryption and associated data, the filler of unused slots, opening all
four, sealing, the passphrase in NFC, the commitment block, the
seal count, and the record and payload layouts of VAULT §7 (F97). Not in
scope: Argon2id, XChaCha20-Poly1305 and HKDF themselves (published, and
checked against their vectors and a second implementation); the app's
screens; the backup plan.

**What to ask the reviewer:**

1. Does the header bind every parameter that affects decryption, and
   can any header, valid or crafted, make a reader do more work than the
   limits state, or reveal anything about a slot?
2. Is a slot bound to its file and position by its associated data, and
   is moving or copying slots between files or positions always
   refused?
3. Are unused slots indistinguishable from used ones in one copy,
   including their nonces, given how `create` draws them?
4. Does opening all four slots leak, by time or by result, which slot
   opened? (VAULT §4's note on a passphrase that opens none.)
5. Is the padding fix with 32 zero bytes at the start of the plaintext
   sufficient for key commitment here, and what is its bound under
   XChaCha20? Is refusing the file the right response?
6. Is NFC alone the right preparation for a key-derivation input, given
   §12's note on code points unassigned in the normaliser's Unicode
   version?
7. Do the seal's nonces (the in-flight fresh draw, with its fallback)
   rule out a repeated nonce under one key, including after a restored
   VM snapshot?
8. Does the seal count, inside the slot, change anything the format
   promises (VAULT §9), and is 5.4's list of what it cannot detect
   complete?
9. Do the record and payload parsers (`records.rs`, `layouts.rs`,
   `codex32.rs`) refuse every malformed slot, and can a crafted slot
   that verifies make them misbehave?
10. Are secrets wiped where the code says (the stretched key, Argon2id's
    memory, a failed slot's buffer, the NFC passphrase)?

**The files they need:** `docs/VAULT.md`; this spec;
`docs/SECURITY-MODEL.md` ("Vault cryptography", "Why this construction
holds up"); F97, F144 and F159; `faraday/faraday-vault/src/lib.rs`,
`records.rs`, `layouts.rs`, `codex32.rs`, `entries.rs` and `tests.rs`; `core/faraday-backup`'s `Cost` and Argon2id;
`faraday/faraday-keys/src/vault.rs` (where a passphrase and a nonce
reach the crate); `faraday/faraday-core/src/vault/mod.rs`'s
`vault_sealed` and `faraday/faraday-core/src/fresh.rs` (the nonce's
source); `tools/vault/open.py` and `tools/vault/primitives.py`;
`tools/crosscheck/area_vault.py`; `tools/vectors/vault/` and
`tools/vectors/vault-primitives/`; the `Cargo.lock` entries and
`docs/deps/` pages of `argon2`, `chacha20poly1305`, `chacha20`, `hmac`,
`sha2` and `unicode-normalization`.

**What they get back:** their findings are filed as
`docs/AUDIT-SUBMISSIONS.md` sets out, and each is answered there.

## 11. Settled for the build

Settled by the coordinator on 2026-10-28, under the owner's word to have
the vault work done overnight; the owner reviews them.

1. **NFC alone** (the owner's call: "redo with just NFC"). No PRECIS
   profile, no space mapping, no refused characters; two passphrases with
   one NFC form are refused as the same (§3). OpaqueString in full is
   §12's.
2. **A stick visit cannot compare counts**, so none is written over by
   count: a visit's process holds no key, and a process that holds one
   is never handed a stick (PLAN §5.1). The comparison is at unlock,
   against the other copies in Files (§5.3 item 2). Settled by what the
   code allows.
3. **Reversed by the owner** (2026-10-28, 23:45 EDT): the backup plan
   records each other vault's seal count (§5.6).
4. **Reversed by the owner** (2026-10-28, 23:40 EDT): a file is a vault
   by its magic and size, whatever its name (§2.1).

## 12. What this does not do

- It does not make a rollback detectable across power-off except
  through an open vault's backup plan (§5.6), or for a slot not opened
  (5.4).
- It does not change the Argon2id cost limits, the slot sizes, the
  number of slots or the header's fields but the magic.
- It does not let a passphrase be added or changed after creation
  (VAULT §8).
- It does not protect a slot whose bytes are damaged: a changed byte
  still loses that slot, and a changed header all four (VAULT §9). The
  remedy is a copy at another place.
- It does not prepare the passphrase as RFC 8265's OpaqueString, only
  puts it in NFC (§11 item 1). OpaqueString would also map spaces other
  than U+0020 to U+0020 and refuse control characters, default-ignorable
  code points (the variation selector U+FE0F most emoji carry, the
  zero-width space), noncharacters, old Hangul jamo, code points
  unassigned in the implementation's Unicode version, and U+200D outside
  its allowed contexts (emoji joined by it). The unassigned refusal is
  what would make a passphrase's NFC form the same under every later
  Unicode version: with NFC alone, a passphrase holding a code point
  unassigned in the normaliser's version could normalise to other bytes
  after an upgrade of `unicode-normalization`, and its slot would not
  open until the earlier version is used. It would need a PRECIS crate
  (a new vetted dependency) or a table generated from the Unicode
  Character Database, and it would refuse some passphrases a phone
  keyboard types. A later change, if the owner wants it.
