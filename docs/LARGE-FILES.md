# Large files on a stick visit

**Status:** spec v0.2 · 2026-10-14 · drafted from the owner's decision
(`docs/DECISIONS.md` F26), with a security review's fifteen findings
applied the same day. Not built.

The problem it solves: a stick visit reads at most 18 MiB of a file
(`faraday-files` `MAX_READ`, `faraday-core` `READ_MAX`). The figure was
set because a vault with four 4 MiB slots is the largest thing Faraday
writes; it was never a judgement about what a person needs to sign. A
release artifact, an APK or a disk image to sign with GPG can be larger,
and today it is listed and cannot be ticked. Lifting the cap means
holding large files in memory that a vault unlock also needs, so a large
file asks first, and it can come in as only the few hashes signing
needs.

This applies to every file read on a stick or SD card visit, not only
to APKs. `docs/APK-SIGNING.md` is the first user.

## 0. Rules for the implementing agent

- `CLAUDE.md` applies in full: every build and test on the build machine; every dimension a token; labels, not
  prose, on working screens; tests for user-facing behaviour only;
  `just` once at the end of a batch; no `rm`; no commits; no agents.
- Batches in §12's order; one review and one completion report each.
- The privilege split of `docs/history/PLAN.md` §4.3 stays as it is: the disk process
  reads and writes FAT and nothing else; the app decides what a file is
  and hashes what it signs. The disk process still treats the app as
  hostile: no request lets the app rename or remove a file other than
  the one write it has open (§8).
- Completion report: items done, each with its tests and whether they
  pass; what was left out and why; docs changed; renders to look at. No
  restating the spec.

## 1. What is decided

- No ceiling on reading: FAT32's largest file, 4 GiB less one byte, is
  the only size limit.
- A file up to **64 MiB** comes in as today, without asking. A larger
  one is listed unticked with its size, and ticking it offers **Import
  the whole file** or **Import only what signing needs**.
- A whole import is refused where it would leave too little memory to
  open the vaults this session knows, or could meet (§4). Every copy
  Faraday makes of a file counts, and so does what is made from it.
- A signing-only import streams the file once and keeps its name, size
  and hashes, and for an APK what the APK signature needs (§5). Any size
  works. Everything kept comes from that one pass.
- A signed APK from a signing-only import is written on a later visit,
  by copying the unsigned file from that stick with the signing block
  inserted, every chunk checked against the first read (§7).

## 2. Sizes

- **No cap.** `MAX_READ` and `READ_MAX` go. A file is read up to FAT32's
  4 GiB less one byte; FAT16 files are smaller still.
- **The threshold is 64 MiB.** It is 3.5 times the old cap, holds an
  APK (14 MiB as a debug build) with room to grow, and is
  about 6% of a Pi 3's 1 GB. Up to it, a file comes in ticked as
  `docs/FLOWS.md` decision 29 says, inside §4's rule for the visit's
  total.
- **The boot import reads no file over the threshold.** Its sheet counts
  them ("1 vault · 2 PSBTs · 1 large file"); a later stick visit offers
  them.
- **Names** are bounded for every file on a visit, whatever its size
  (§9).

## 3. On the visit

A file over the threshold is listed on **On the stick** unticked, with
its size ("faraday-1.2.img · 1.8 GiB"). Ticking it opens a sheet titled
with the file's name and size, with two choices:

- **Import the whole file**, with three rows under it: **Size**, **Free
  after importing**, **{vault} needs** (§4). Inert, with the reason in
  `WARN`, where §4 refuses it; the free-after row in `WARN` where §4
  warns.
- **Import only what signing needs**, with the row **Signs**: "GPG" for
  any file, "APK and GPG" for a file whose end and central directory
  read as an APK. The sheet reads those two to tell (bounded, §9); they
  decide this row and guide the read, and nothing kept comes from them
  (§5).

**Cancel** leaves it unticked. The choice is shown on the ticked row
("Whole" or "Signing only") and pressing the row opens the sheet again.

**Reading** a large file shows its progress on the visit, "Reading
{name} · 120 of 1800 MiB", with **Stop**, which ends the read and keeps
nothing of the file. These end a read the same way, and nothing of the
file is kept:

- a stick pulled during the read;
- a request that times out (§8);
- a part that comes back with any length but the one asked for, or a
  file whose size in the directory is no longer the size listed:
  "{name} changed while it was read" in `WARN`.

The size listed when the visit opened is the file's size for the whole
read; the app asks for no byte past it.

**Locks during a transfer.** The idle lock waits while a read or a
write in parts runs; a stick is attached, so no secret is open. The
lock button stops the transfer first, as Stop does, then locks.

## 4. Import the whole file

The file is held in memory and becomes a File like any other: GPG's
**Sign a file**, **Sign with APK key**, **Show as QR** where it fits the
envelope's 256 KiB, written to another stick.

### 4.1 The rule

**What a vault needs, *V*.** One function in `faraday-core`,
`vault_need_mib(memory_mib, slot_len)`, gives it from the vault's
header, and both this rule and Unlock's check (`vault_unlock_check`)
use it: *V* = its Argon2id memory + twice
its file size + 100 MiB. That is the unlock's memory, the file and a
sealed copy beside it while it is written, and the 100 MiB `APP_MIB`
that Create already leaves beside a vault for a slot's plaintext and
the app (`docs/VAULT.md` §3.1, "memory free to open"). Sealing needs
less than unlocking, so a vault that can be unlocked can also be sealed.

**The memory, *F*.** `MemAvailable`, read once by the first process
after power-on as it starts, before the boot import reads anything, and
kept in the `kept` set across locks. It is not read again for this
rule: a later figure already counts some of what Faraday holds and not
the rest (memory reserved and not yet touched, an Inbox not yet saved),
and a reservation that succeeds proves nothing, since Linux lends memory
it may not have.

**What Faraday holds, *L*.** The app keeps a ledger, in bytes, of what
it holds that an unlock cannot use:

- every Inbox and Outbox item's bytes, twice: the app's copy and the
  one in `/run/faraday` (§4.2);
- each signing-only entry's and each pending signed APK's kept bytes,
  twice (§5, §7);
- every read and every product in progress, at its full size from the
  moment it starts, before any part arrives.

An item deleted, dropped at a lock or written and cleared leaves the
ledger as it leaves the boxes.

**The need, *N*,** is the largest *V* of the vaults this session knows:
open, locked in Files, or on any stick seen this power-on whose vault
header was read. Each vault's cost is read from its header without a
passphrase. Where the session knows no vault, *N* is the *V*, with the
largest vault file (16 MiB), of the largest memory that fits *F*: of
`docs/VAULT.md` §3.1's presets and of Create's custom memories, 64 MiB
to 4 GiB, the larger counts.

**The cost, *C*,** of an action is twice the bytes it adds:

- a whole import of *S* bytes: 2*S*;
- **Sign with APK key** on a whole import: twice the signed APK's size,
  since it goes For the stick beside the unsigned file;
- a signing-only import: twice the bytes it keeps (§5);
- GPG's **Sign a file**: none. The file is hashed where it is held;
  `faraday-pgp` signs from a hash state over the data (§6) and copies
  no file.

The rule:

- **Refused** when *F* − *L* − *C* < *N*. The row reads "Not enough
  memory to open {vault}" ("… to open a vault" where *N* came from a
  memory that fits). On the import sheet, signing only is still
  offered. A refused Sign with APK key says the same on its review, and
  the hold is inert.
- **Refused** also when the filesystem that holds `/run/faraday` has
  less room free than the bytes the action adds there (`statvfs`): "Not
  enough room to keep it across a lock".
- **Warned** when *F* − *L* − *C* < *N* + 128 MiB: the free-after row
  in `WARN`. Importing goes on.
- **The visit's total** follows the same rule, each file's cost entering
  the ledger as its read starts. Files at or under the threshold come in
  ticked until their total would cross the refusal line. Past it, a
  file is unticked with the same reason, and ticking it is refused. Two
  reads on one visit cannot both pass on the same free memory.

**Free after importing** shows *F* − *L* − *C*.

A Pi 3 with about 900 MiB free at power-on, nothing else held and a
Light vault with a 1 MiB file on the stick (*N* = 64 + 2 × 1 + 100 =
166 MiB) takes a whole file of up to about 367 MiB. A Pi 3 with no vault
known takes *N* from the largest memory whose *V* fits 900 MiB, 512 MiB
(Medium, or Custom 512): 512 + 32 + 100 = 644 MiB, so about 128 MiB.

**A vault unknown at import time** can still need more than *N*: a
Medium vault on another stick while a Light one was known. Unlock's
check then refuses, as it does today, against the memory free now; where
Faraday holds files over the threshold, its error names them, largest
first, "{file} holds {n} MiB", each with **Delete {file}**. Deleting one
frees both its copies, and Unlock checks again.

Reading a whole import uses the streamed read (§8) into one buffer the
app reserves at the file's size before the first part, its cost already
in the ledger. A whole import is parsed from that buffer alone, never
from what the sheet read.

### 4.2 Saving the boxes (L1)

Today `save_boxes` clones every Inbox and Outbox item into the command
it sends, and `Boxes::save` empties each box directory and writes every
item again, on every call. With a large file that is a third copy and a
rewrite of the whole file at every toast or setting. L1 changes both:

- Items are held as shared, immutable buffers, and the save command
  carries references to them, not copies.
- `Boxes::save` writes only the items added since the last save and
  removes only the items gone; an unchanged item is never written again.
  Each is written under a temporary name in its box and renamed, so a
  failed write leaves the box as it was.
- Each box saves on its own: a failure in one leaves that box's last
  saved state and the others are still saved. A failed save says
  "Cannot keep {name} across a lock".
- Signing-only entries are kept in `/run/faraday/signing/` and pending
  signed APKs in `/run/faraday/pending/`, each in its own fixed format
  (§5, §7). Neither directory's files are read as Inbox or Outbox
  files, and no Inbox or Outbox file is read as either.

The same applies to the desktop's `Boxes::Memory`, which holds the
second copy in the shell.

## 5. Import only what signing needs

The sheet's reads of the end and central directory (§3) decide its
**Signs** row and nothing else. The import then reads the file once, in
parts, front to back, and the app hashes every part as it arrives.
Everything it keeps comes from that one pass:

- the name, the size, the stick it came from (its label and volume
  serial, §8);
- an OpenPGP SHA-256 state and an OpenPGP SHA-512 state over the file's
  bytes, as a binary signature (type `0x00`) hashes them before its
  trailer. Faraday makes no text-mode signature. Both of `docs/history/PLAN.md` §7's
  hashes run, so Sign a file still asks which. The file's SHA-256 shown
  anywhere is a copy of the SHA-256 state, finished: the hash a person
  compares and the hash GPG signs are one computation;
- for a file that reads as an APK (`docs/APK-SIGNING.md` §5): the 1 MiB
  chunk digests of its entries' section, its central directory and end
  record as bytes (bounded, §9), the manifest's package, versions and
  `minSdkVersion` inflated from the manifest entry as it passes, and the
  refusal if one applies. Each local header is checked for alignment as
  its bytes pass.

**For an APK,** the reader needs the central directory before the
stream reaches it, to know where the manifest and each local header
are, so the copy the sheet read guides it. When the stream reaches the
central directory and end record, those bytes must equal the sheet's
copy, byte for byte. Where they differ the import ends, "{name} changed
while it was read", and nothing is kept. The chunk digests and the kept
central directory and end record are the stream's bytes.

The digests of a 4 GiB file take 128 KiB. Nothing else of the file
stays in memory: each part is wiped once hashed. The kept bytes, at most
a 16 MiB central directory, its end record and the digests, enter the
ledger (§4.1) twice.

**In Files** the item is a signing-only entry: its name, its size,
"Signing only", the stick it came from. Its actions are **Sign a file**
(GPG), **Sign with APK key** for an APK, and **Delete**. It cannot be
shown as codes or saved as a picture, written to a stick, put in a
vault, opened as any kind, or read for codes. One name in Files is one
item: a whole file and a signing-only entry of the same name replace
each other, as a file copied in replaces one of its name today. "Already
in by name and size" compares the entry's kept size.

**Across a lock** it holds hashes and no bytes of the file, so a lock
keeps it, as it keeps public files (`docs/history/PLAN.md` §5.3), and its name stays
as the names of public files do. It is kept in `/run/faraday/signing/`
(§4.2), one file per entry: a magic `FSOE`, a version byte, then each
field with its length. The hash states are kept whole: each one's
chaining value, the count of bytes hashed and the unfinished block. A
file there that does not read in this format is dropped. Power-off and
the panic key lose them.

## 6. Signing a signing-only file

- **GPG.** `faraday-pgp` gains a detached signature made from a hash
  state over the data in place of the data: it finishes a copy of the
  state with the signature's hashed part and the v4 trailer (`0x04`,
  `0xFF`, the hashed part's length as four bytes), and signs. A whole
  file uses the same call, the app hashing the file where it is held, so
  no copy of it is made (L1). The detached signature goes For the stick
  as any signature does: `{file}.asc`, with **Show as QR** and **PNG**.
- **APK.** Sign an APK shows the same review (`docs/APK-SIGNING.md`
  §4.4) from the kept fields, and builds the APK Signing Block from the
  kept chunk digests. The block, the central directory and the end
  record are kept as a **pending signed APK** For the stick, named as
  the signed APK will be, with the size it will have. Its bytes are
  written on a visit (§7). The block is not secret: it holds signatures
  and the certificate.

## 7. Writing a signed APK on a later visit

**The row.** On the visit's For the stick column the pending APK reads
"{output name} · signed APK · {size}" and, under it, "From {source
name}". It is ticked when the stick shown holds a file with the source's
name, compared as FAT compares names (ignoring case), and size.
Otherwise it is inert, with "Needs {source name} on this stick" in
`WARN`. Any stick holding an identical copy works, not only the one it
was first read from; a different file of that name and size is refused
at its first chunk that differs, before any unchecked byte is written.

**Before writing.**

- The output stays under FAT32's 4 GiB, or the row reads "Over 4 GiB on
  FAT32" and is inert (checked when it is signed, too).
- The stick's free space holds the output rounded up to whole clusters,
  plus the clusters a hidden temporary file of that name left by an
  earlier attempt frees. Otherwise "Needs {n} MiB free".
- The top folder has room for the directory slots of both names, the
  temporary one and the output (on FAT16 the top folder is fixed).
  Otherwise "The stick's top folder is full".

**Writing,** with the progress line "Writing {name} · 120 of 210 MiB"
and **Stop**:

1. **WriteBegin** with the output name: the disk process makes the
   hidden temporary file, `.faraday-{output name}.part`, as every write
   makes one, removing one of that name left by an earlier attempt.
2. The source read in parts. Each part is checked before it is written:
   the entries' chunks against the kept chunk digests, the central
   directory and end record against the kept bytes, the whole against
   the kept SHA-256. Then the temporary file gets the entries, the
   signing block, the central directory, and the end record with its
   offset raised. Only bytes already checked are written.
3. The temporary file read back in parts (ReadBack), its SHA-256
   compared with the one the app worked out for the output while
   copying.
4. **Rename** to the output name, asking first where a file of that
   name is there (`docs/FLOWS.md` decision 58).

**Refused, nothing written.** A part that differs from the first read:
"{source name} changed since it was read. Nothing written." The
temporary file is removed (Abandon) and the pending APK stays. Stop, a
timeout, a read-back that differs, or a failed write do the same.

**A stick pulled.** FAT has no atomic rename, and Faraday's keeps the
order it has (`faraday-fat` `rename`): the temporary file's entries are
erased before the output's are written, so that two names never share
one chain. What a pull leaves:

- during steps 1 to 3: the hidden temporary file, never a file under
  the output name. The next attempt replaces it;
- over a file of that name, after the old file is removed and before
  the rename: the old file gone and the new one whole under its hidden
  name;
- in the instant of the rename itself: neither name, the written
  clusters unclaimed until a computer checks the stick (`fsck.vfat`,
  `chkdsk`).

A file under the output name is always the whole, checked output. That
is the promise; FAT allows no stronger one.

**After.** The pending APK stays For the stick, tagged on this stick as
any file written, so it can be written beside another copy. Delete
removes it.

**Across a lock and power-off.** A lock keeps the pending APK in
`/run/faraday/pending/` (§4.2): its block, central directory, end
record, chunk digests, sizes and names, none of them secret, in its own
format (a magic `FPAP`, a version byte, each field with its length). It
is never an Outbox file and is never written to a stick as one.
Power-off and the panic key lose it, as they lose the Outbox; signing
again needs the key's vault unlocked.

## 8. The disk process

`faraday-files`' frame protocol (`proto.rs`) gains parts. Each is still
one request and one response, so the protocol keeps its shape, its
hostile-peer checks in both directions and its wiping.

**Listing.**

- **List** answers the sticks: each one's id, label, whether it is the
  boot medium, and its volume serial from the boot sector. No files.
- **ListFiles** (stick) answers that stick's top-level files and sizes.
  One stick per answer, so an answer holds one top folder at most
  (`MAX_ROOT_BYTES`, 2 MiB of slots, about 2.4 MiB of names).

**Reading.**

- **ReadPart** (stick, name, size, offset, length up to 4 MiB) answers
  exactly min(length, size − offset) bytes. It is refused where the
  file's size in the directory is not `size`. The app asks for the parts
  it wants in the order it wants them: an APK's last 64 KiB, then its
  central directory, then the file from the start. A whole import is
  the same, into one buffer.
- The disk process keeps, for the one file being read, where in its
  cluster chain the last part ended, and a bitmap of the clusters
  passed, one bit per cluster of the volume. A part that follows on
  walks forward only; a part before it (the APK's tail, then its start)
  walks from the start once more. A chain that comes back to a cluster
  already passed is refused, as is one that ends before the size or
  leaves the volume (`chain_start`'s rules). This is dropped when
  another file is read, the stick goes, or the pipes are opened again.
- A `.ofv` read at offset 0 first finishes interrupted vault writes, as
  `read` does today (`finish_vault_writes`).
- **ReadQr** refuses a picture over the threshold before reading it,
  and answers a picture's codes only; its bytes come by ReadPart like
  any file's.

**Writing.**

- **WriteBegin** (stick, name, replace): the disk process checks the
  name (§9), makes the hidden temporary file `.faraday-{name}.part`
  itself, removing one of that name left by an earlier attempt, and
  holds it as the one write open.
- **WritePart** (bytes, up to 4 MiB) appends to the open write.
- **WriteEnd** closes it, still under its temporary name.
- **ReadBack** (offset, length up to 4 MiB) reads the open write's
  temporary file.
- **Rename** puts the open write in place by `write_any`'s rules: a
  sealed vault over its own file, found by salt and length from the
  temporary file's header; the settings file over the settings file;
  any other file over one of its name only when `replace`; never over a
  vault.
- **Abandon** removes the open write's temporary file.

No request names a temporary file, and none renames or removes a file
that is not the open write: **Wipe** stays the only way to delete a
person's file. The disk process holds at most one write open, and
abandons it when the stick goes, when another WriteBegin comes, and
when its pipes are opened again (a new app process after a lock: the
disk process outlives the app's).

**What goes in parts.** Every file over one part, vault files and their
write-backs included, goes by ReadPart and the write requests. `Read`
and `Write` of a whole file stay for files up to one part.

**Frames.** `MAX_FRAME` stops following the largest file: a part of 4
MiB plus room for a name, 5 MiB. A ListFiles answer and a ReadQr answer
fit under it.

**Timeouts,** per request, not per file: List, ListFiles, ReadPart,
ReadBack, WritePart and WriteEnd 30 seconds; WriteBegin, Rename and
Abandon 120 seconds, since each may free a chain of up to 4 GiB; `Write`
keeps its 120. A request that times out ends the transfer as Stop does.

The copy-with-insert of §7 is driven by the app, part by part, through
ReadPart and WritePart, rather than run inside the disk process. So
the app writes only bytes it has just checked, the hash is the app's
own and never the disk process's, and the disk process stays a FAT
driver that knows nothing of APKs. Each byte crosses the pipe twice,
in memory, against a stick's 10 to 30 MB/s.

## 9. Untrusted input

Bounds that hold whatever the file's size (`docs/history/PLAN.md` §4.4):

- the end record searched in the last 65,557 bytes;
- a central directory over 16 MiB is "Not an APK" (that is more than
  100,000 entries);
- `AndroidManifest.xml` at most 1 MiB compressed and 1 MiB inflated;
- the digests at 32 bytes per MiB of file;
- a picture over the threshold is not decoded for codes (§8).

**Names.** A file's name comes from the stick. Every file on a visit,
whatever its size, is listed and can be ticked only where its name:

- holds no control or format character (Unicode categories Cc and Cf,
  bidirectional controls included);
- is at most 230 bytes of UTF-8, so `.faraday-{name}.part`,
  `{name}.asc` and an APK's `-signed` stay inside FAT's 255 characters.

Any other file is listed with its size and each such character shown as
`?`, inert, with "Rename it on another computer" in `WARN`. The disk
process refuses such a name in WriteBegin.

The fuzz targets of `docs/APK-SIGNING.md` §5 feed the reader its parts
in every order the app asks for them, a central directory streamed
different from the one read first among them.

## 10. The Pi and Faraday Desktop

- **The Pi** works the same way through an SD card visit. Its 1 GB
  makes §4's refusal come sooner, and signing only is the usual way to
  sign a large file there.
- **Faraday Desktop**'s sticks are folders read through the same storage
  code (`README.md`, "Faraday Desktop"), so it has the same visit,
  threshold, choice and rule, with two differences, since the computer's
  memory changes with what else runs on it:
  - *F* is the free memory now, and *L* counts only reads and products
    in progress, since what Faraday holds already shows in it.
  - Where the system gives no figure (macOS for now,
    `memory_available_mib`), the sheet shows **Size** and **{vault}
    needs** only and refuses nothing, as Create and Unlock check nothing
    there.
  A folder's file can change while it is read; §3's size check catches
  it as it catches a stick's. One behaviour is simpler than two, and
  the desktop also opens vaults. Transfer's picker sends codes only,
  bounded by the envelope's 256 KiB, and is unchanged.

## 11. Tests

User-facing behaviour (`CLAUDE.md`). The free memory is an input the
shell reports, so a test sets it. A stick is a folder, as the desktop
app's are.

- In `faraday-core/tests/large_files.rs`:
  - a file over 64 MiB is listed unticked with its size, and ticking it
    offers both choices, with "APK and GPG" for an APK;
  - a whole import that would leave less than *N* is refused, naming the
    vault on the stick, and signing only is still offered;
  - files under the threshold whose total crosses the line come in
    ticked up to it;
  - after a whole import, Sign with APK key on it is refused where its
    signed copy would cross the line;
  - Unlock refused for memory names the large file held, and its
    Delete lets the unlock go on;
  - a whole import, then a lock: the next process keeps it and every
    other Inbox and Outbox file;
  - Stop during a read keeps nothing of the file;
  - a part answered short, or a file whose size changed, ends the read
    with "{name} changed while it was read";
  - a signing-only APK whose central directory reads differently in the
    stream than on the sheet keeps nothing;
  - a signing-only APK signs, and the next visit writes the signed APK;
  - a source changed between the visits (one byte in the middle) is
    refused and no file appears under the output name;
  - a stick without the source shows "Needs {source name} on this
    stick";
  - a file whose name holds a bidirectional control is inert with
    "Rename it on another computer".
- Judged by other tools (skipped with a note where missing):
  - the visit-2 copy is byte for byte what `apksigner sign` (§8 of
    `docs/APK-SIGNING.md`'s flags) makes of the same unsigned file;
  - a GPG detached signature over a signing-only file, and one over a
    whole file, pass `gpg --verify` against the file.
- `faraday-files/tests/`: ReadPart, WritePart, WriteEnd, ReadBack,
  Rename and Abandon over a FAT image; a write abandoned or cut short
  leaves no file under the final name; Rename never goes over a vault;
  a chain that loops is refused; a write left open is abandoned when
  the pipes are opened again; a vault over one part written back over
  its own file.
- **The large fixture** is made at test time and never committed:
  `tools/vectors/apk/make.sh large 200` stores 200 MiB of fixed bytes
  (a ChaCha20 stream from a fixed key) as an asset in the small fixture
  APK, aligned. Every judged check above runs on it.

## 12. Batches

`docs/APK-SIGNING.md` batch 1 comes first, on today's reading: it signs
an APK up to the current 18 MiB.

- **L1, any size.** The protocol of §8 (List and ListFiles, ReadPart
  with its chain position, the write requests, ReadQr's bound, the
  timeouts), vault files read and written back in parts; no cap; the
  threshold; §9's names; the boot import's count; the sheet with Import
  the whole file only; §4's rule with `vault_need_mib` shared with
  Unlock, *F* kept from power-on, the ledger, the visit's total and
  Unlock's Delete; §4.2's box saving; `faraday-pgp` signing from a hash
  state; progress, Stop and the idle lock's wait; writes in parts, so an
  Outbox file of any size is written. Its tests.
- **L2, signing only.** §5 to §7: the single streamed pass, the
  signing-only entry and its box, GPG over it, the pending signed APK,
  its box and its write; §9's bounds in the fuzz targets; the 200 MiB
  fixture. Its tests.

APK batch 2 (the release) can land after APK batch 1 and does not wait
for L1 or L2, so long as the release APK stays under 18 MiB until L1 is
built. Until L1, `README.md`, `docs/history/PLAN.md` and `docs/FLOWS.md` give
today's 18 MiB and mark this spec's behaviour as not built; L1's
completion report changes them.
