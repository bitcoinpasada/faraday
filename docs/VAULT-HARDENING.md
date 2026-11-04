# Vault hardening: the file's name, the passphrase, key commitment and the seal count

**Kind:** Spec
**Status:** Specified (F159), amended by F189 and F190; the passphrase in NFC (§3) and the seal count (§5.1 to §5.7) built in the FVLT format; the file's name (§2), the commitment block (§4), the edit list (§6) and the outside review (§10) replaced by F189 and moved to `docs/history/VAULT-HARDENING-LOG.md`
**Open:** the passphrase in NFC and the seal count in the KDBX vault, built with `docs/VAULT.md` (F190)

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

**F189 (2026-11-02).** The vault is now an age file under one
passphrase (`docs/VAULT.md`), and what this spec did to the FVLT format's
name, magic and slots is replaced: an age file's header MAC does what
the commitment block did, and the container, no longer Faraday's own,
needs no outside review of its own. §2, §4, §6 and §10 are in
`docs/history/VAULT-HARDENING-LOG.md`. The passphrase in NFC (§3) and the
seal count (§5) carry over, as `docs/VAULT.md` §3, §5, §7 and §11 say:
the count is `vault.seals` in the TOML, and a backup plan names another
vault by the vault identifier inside it, not by its salt (§5.6). What
follows in §3 and §5 is their account for the FVLT format, built.

**F190 (2026-11-02).** The age file is replaced in turn: a file vault is
a KDBX 4 database and a QR vault an RFC 9580 OpenPGP message, both under
Argon2id. The passphrase in NFC and the seal count carry over as before;
a KDBX file's HMACs, keyed from the passphrase's key, do what the age
header MAC did.

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

Moved to `docs/history/VAULT-HARDENING-LOG.md`.

### 2.1 A vault is known by its contents

Moved to `docs/history/VAULT-HARDENING-LOG.md`.

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

Moved to `docs/history/VAULT-HARDENING-LOG.md`.

### 4.1 The change

Moved to `docs/history/VAULT-HARDENING-LOG.md`.

### 4.2 Its size and place

Moved to `docs/history/VAULT-HARDENING-LOG.md`.

### 4.3 Tests

Moved to `docs/history/VAULT-HARDENING-LOG.md`.

## 5. The seal count

In the KDBX vault (F190) the count and its time are the database's custom
data `faraday.seals` and `faraday.sealed` (`docs/VAULT.md` §7), and a copy is compared when it is
opened, not while it sits in Files (`docs/VAULT.md` §9, §11).

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
`vault_seals_older` and `vault_seals_newer_copy` (§5.3), and the
older-copy dialog's `vault_older_head` ("Open this copy?"),
`vault_older_back` ("Keep locked") and `vault_older_open` ("Open"), in
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
   for it stops at a dialog before it opens (owner, 2026-10-29; §11
   item 5), which names the vault and what was found:
   "An older copy: sealed 9 times. A copy sealed 12 times was open since
   power-on." The same count with other bytes: "Another copy was sealed
   12 times too, with other changes." Faraday cannot tell an older copy
   the person kept from one put back by someone else, so the person
   decides; the unlock is never refused. The dialog is Material's basic
   dialog (`docs/DESIGN.md` §4.13; Material 3, Dialogs), **Open this
   copy?** with a warning icon, the vault's name, the line in the
   warning tone, and its actions dismissive first:
   - **Keep locked** (and Escape, and a press beside the dialog) closes
     the slot, its key wiped, and puts back what was remembered of the
     vault before (VAULT §11); the counts read stay seen. The sheet the
     unlock was made on (the boot import's) comes back.
   - **Open** goes on with the unlock as it would have gone with no
     warning: its toast, its codex32 shares gathered, its Load sheet or
     the flow it was opened for. The vault's head keeps the line in the
     warning tone.

   Until **Open**, nothing of the slot is loaded or shown: the slot is
   held behind the dialog (`vault::OlderHold`), and a dialog put away
   another way (a sheet that came over it and closed) is **Keep
   locked**. A slot already open when another vault's plan names it
   older (§5.6) stays open, and its head says so.
2. **At unlock, the other copies in Files.** A stick visit cannot read a
   count: a process that has unlocked a vault holds its key and is never
   handed a stick (`docs/history/PLAN.md` §5.1), and a visit's process
   holds none. So at unlock, every other copy of the same vault (same
   salt and size) in the Inbox or the Outbox has the opened slot read
   with the key just derived (`Opened::copy_seals`, one decryption each,
   no Argon2id): a higher count says "An older copy: sealed 9 times. A
   copy sealed 12 times is in Files."; the same count with other bytes
   says "Another copy was sealed 12 times too, with other changes". The
   unlock stops at item 1's dialog. To compare a stick's copy, copy it
   into Files before unlocking.
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
  read "older"; the count says which is newer, not why. So the unlock
  asks (5.3 item 1) and does not refuse;
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
  - unlock a copy, seal it twice, lock, unlock the first copy: Unlock
    stops at the dialog, which says it is older; the unlock does not
    land, through ticks too, until **Open**, which lands it and leaves
    the line on the vault's head; **Keep locked**, or Escape, leaves it
    locked, and the next unlock stops at the dialog again;
  - another copy of the vault in Files with a higher count: Unlock says
    it is older and where the newer one is; the newer copy opened with
    the older beside it says nothing;
  - two copies sealed from one copy: the second unlocked says another was
    sealed the same number of times;
  - power off (a new process without the kept state), with no vault
    holding a plan that names it open: the older copy opens with no
    warning, which is the limit 5.4 states.

### 5.6 The count in the backup plan

In the KDBX vault (F190) the line is `seals {id} {count} {digest}`: the vault
identifier, the count, and the SHA-256 of the file at that count
(`docs/VAULT.md` §7). The salt changes at every seal there.

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

Never refused: at the slot's own unlock it stops at §5.3's dialog and
opens on **Open**; a slot already open when the plan's vault opens stays
open, and its head says so.

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
> still unlocks. Compare the count with how often you remember changing
> the vault: a count lower than you expect is an older copy.
>
> Faraday remembers the counts it has seen until the computer is turned
> off. Unlocking an older copy after a newer one in that time stops
> before the vault opens and says so: "An older copy: sealed 9 times."
> Faraday cannot tell an older copy you kept from one someone put back.
> Open carries on with that copy; Keep locked leaves it locked.
>
> A backup plan kept in one vault records the counts of the other
> vaults it uses. Open the vault with the plan first, then the others: a
> vault older than the copy the plan records is named as older, even
> after the computer was turned off. With one vault, its plan is in the
> same file and goes back with it; then the count at unlock, against
> what you remember, is the check.
>
> Unlock also reads the other copies of the vault in Files, with the
> passphrase just typed. If one has been sealed more times, Unlock stops
> and says "An older copy: sealed 9 times. A copy sealed 12 times is in
> Files." Keep this one locked and open that copy to carry on from it.
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
("Unlock": the Sealed line, the older-copy line and the older-copy
dialog),
`docs/guide/screens/stick-visit.md` (not changed: a visit reads no
counts), and `docs/guide/walk/vaults-and-passwords.md`
(§3 "Unlock it" and §10 "Unlock it again, and load from it", whose
pictures show the line), the pictures drawn again with
`just guide-pictures`.

## 6. VAULT's edit list

Moved to `docs/history/VAULT-HARDENING-LOG.md`.

## 7. The other files

| File | Change | Batch |
|---|---|---|
| `faraday/faraday-vault/src/lib.rs` | `Opened::position()` and a public `slot_range(slot_len, i)`, for the digest of §5.3 and §5.6 (a SHA-256 over up to 4 MiB: costly, counted as `FileHash`); `COMMIT_LEN`; `encrypt_slot` and `decrypt_slot` write and check the block; `Error::Commitment`; `create`, `open`, `derive` take `&str` and put it in NFC; `Opened` keeps the seal count and time, `seal(&mut self, …, now)`; the crate doc's list | 1 |
| `faraday/faraday-vault/src/records.rs` | type 16; `Contents::encode` given *L* − 32; decode takes the seal record out | 1 |
| `faraday/faraday-vault/Cargo.toml` | `unicode-normalization` (workspace, no default features), already vetted for `faraday-bip` | 1 |
| `faraday/faraday-vault/src/tests.rs` | §3.3, §4.3, §5.5 | 1 |
| `faraday/faraday-vault/examples/vectors.rs`, `faraday/faraday-vault/tests/open_py.rs`, `tools/vectors/vault/` (its README and `slots.json`) | the vectors made again; the two new vectors | 1 |
| `tools/vault/open.py` | the magic, NFC, the block, type 16 | 1 |
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
this document: the vectors and the second reader (a second
implementation Faraday's own authors wrote, not checked against an
outside source, F160, F177) hold the bytes,
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

Moved to `docs/history/VAULT-HARDENING-LOG.md`.

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
5. **An older copy asks before it opens** (the owner's, 2026-10-29).
   Faraday cannot tell an older version of the same vault from one put
   back by an attacker, so an unlock that finds the slot older (§5.3
   items 1 and 2: a lower count than one seen, or the same count with
   other bytes, and §5.6's plan lines read at that unlock) stops at a
   warning naming what it found, with **Open** to go on and **Keep
   locked** to go back; only after **Open** does the vault open. Never
   refused. Before, it opened and said so in the warning tone and a
   toast.
6. **A stick visit keeps today's rule** (the owner's, 2026-10-29,
   confirming item 2): a visit writes a vault sealed this session over
   the stick's copy of the same vault (the file whose SHA-256 is one the session has seen, VAULT §6)
   and does not refuse to write over a copy it does not recognise.
   Alternatives not taken: refusing to write over a stick copy whose
   bytes no receipt or read recorded, and a list of each copy's digest
   in the backup plan for the visit to check the stick's copy against.

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
