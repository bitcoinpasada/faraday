# Large files on a stick visit

**Status:** spec v0.1 · 2026-10-14 · drafted from the owner's decision
(`docs/DECISIONS.md` F26). Not built.

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

- `CLAUDE.md` and `CLAUDE.local.md` apply in full: every build and test
  through `local/remote-run.sh`; every dimension a token; labels, not
  prose, on working screens; tests for user-facing behaviour only;
  `just` once at the end of a batch; no `rm`; no commits; no agents.
- Batches in §12's order; one review and one completion report each.
- The privilege split of `PLAN.md` §4.3 stays as it is: the disk process
  reads and writes FAT and nothing else; the app decides what a file is
  and hashes what it signs.
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
  open the vaults this session knows (§4).
- A signing-only import streams the file once and keeps its name, size
  and hashes, and for an APK what the APK signature needs (§5). Any size
  works.
- A signed APK from a signing-only import is written on a later visit,
  by copying the unsigned file from that stick with the signing block
  inserted, every chunk checked against the first read (§7).

## 2. Sizes

- **No cap.** `MAX_READ` and `READ_MAX` go. A file is read up to FAT32's
  4 GiB less one byte; FAT16 files are smaller still.
- **The threshold is 64 MiB.** It is 3.5 times the old cap, holds the
  Companion's APK (14 MiB as a debug build) with room to grow, and is
  about 6% of a Pi 3's 1 GB. Up to it, a file comes in ticked as
  `docs/FLOWS.md` decision 29 says, inside §4's rule for the visit's
  total.
- **The boot import reads no file over the threshold.** Its sheet counts
  them ("1 vault · 2 PSBTs · 1 large file"); a later stick visit offers
  them.

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
  read as an APK. The sheet reads those two to tell (bounded, §9).

**Cancel** leaves it unticked. The choice is shown on the ticked row
("Whole" or "Signing only") and pressing the row opens the sheet again.

**Reading** a large file shows its progress on the visit, "Reading
{name} · 120 of 1800 MiB", with **Stop**, which ends the read and keeps
nothing of the file. A stick pulled during a read is the same as Stop.

## 4. Import the whole file

The file is held in memory and becomes a File like any other: GPG's
**Sign a file**, **Sign with APK key**, **Show as QR** where it fits the
envelope's 256 KiB, written to another stick.

**What it costs.** On the stick and the Pi a file in Files is held
twice in RAM: in the app, and in `/run/faraday`, where the Inbox is
kept across a lock (`PLAN.md` §5.3). So a whole import of *S* bytes
costs 2*S*. The box save writes each item from the app's own copy and
makes no third.

**What a vault needs.** For a vault, *V* = its Argon2id memory + twice
its file size + 64 MiB. That is the unlock's memory, the file and a
sealed copy beside it while it is written, a slot's plaintext and room
for the app (`docs/VAULT.md` §3.1, "memory free to open"). Sealing needs
less than unlocking, so a vault that can be unlocked can also be sealed.

**The need, *N*,** is the largest *V* of the vaults this session knows:
open, locked in Files, or on the stick shown. Each vault's cost is read
from its header without a passphrase. Where the session knows no vault,
*N* is the *V* of the largest preset in VAULT §3.1 that fitted the
memory free when Faraday started, with the largest vault file (16 MiB).
*F* is the memory free now, as the Create vault form reads it (Linux's
`MemAvailable`).

- **Refused** when *F* − 2*S* < *N*. Running out of memory during an
  unlock or a seal is not possible after a whole import. The row reads
  "Not enough memory to open {vault}" ("… to open a vault" where *N*
  came from a preset). Signing only is still offered.
- **Warned** when *F* − 2*S* < *N* + 128 MiB: the free-after row in
  `WARN`. Importing goes on.
- **The visit's total** follows the same rule. Files at or under the
  threshold come in ticked until their total would cross the refusal
  line. Past it, a file is unticked with the same reason, and ticking it
  is refused.

A Pi 3 with about 900 MiB free and a Light vault on the stick (*N* =
64 + 2 × 1 + 64 = 130 MiB) takes a whole file of up to about 385 MiB.
A Pi 3 with no vault known takes *N* from Medium (512 + 32 + 64 = 608
MiB), so about 146 MiB.

Reading a whole import uses the streamed read (§8) into one buffer the
app reserves at the file's size before the first part; a reservation
that fails is the refusal above.

## 5. Import only what signing needs

The file is read once, in parts, front to back (an APK's end and central
directory first, §8). The app hashes every part as it arrives and keeps
only:

- the name, the size, the stick it came from (its label and FAT serial),
  and the file's SHA-256;
- an OpenPGP SHA-256 state and an OpenPGP SHA-512 state over the file's
  bytes, as a binary signature (type `0x00`) hashes them before its
  trailer. Faraday makes no text-mode signature. Both of `PLAN.md` §7's
  hashes run, so Sign a file still asks which;
- for a file that reads as an APK (`docs/APK-SIGNING.md` §5): the 1 MiB
  chunk digests of its entries' section, its central directory and end
  record as bytes (bounded, §9), the manifest's package, versions and
  `minSdkVersion`, and the refusal if one applies. Each local header is
  checked for alignment as its bytes pass.

The digests of a 4 GiB file take 128 KiB. Nothing else of the file
stays in memory: each part is wiped once hashed.

**In Files** the item is a signing-only entry: its name, its size,
"Signing only", the stick it came from. Its actions are **Sign a file**
(GPG), **Sign with APK key** for an APK, and **Delete**. It cannot be
shown as codes or saved as a picture, written to a stick, put in a
vault, opened as any kind, or read for codes. It holds hashes and no
bytes of the file, so a lock keeps it, as it keeps public files
(`PLAN.md` §5.3).

## 6. Signing a signing-only file

- **GPG.** Sign a file finishes a copy of the chosen hash state with the
  signature's hashed part and trailer, and signs. The detached signature
  goes For the stick as any signature does: `{file}.asc`, with **Show as
  QR** and **PNG**.
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
name and size. Otherwise it is inert, with "Needs {source name} on this
stick" in `WARN`. Any stick holding an identical copy works, not only
the one it was first read from.

**Before writing.** The output stays under FAT32's 4 GiB, or the row
reads "Over 4 GiB on FAT32" and is inert (checked when it is signed,
too). The stick's free space holds the output; otherwise "Needs {n} MiB
free".

**Writing,** with the progress line "Writing {name} · 120 of 210 MiB"
and **Stop**:

1. A hidden temporary file, `.faraday-{output name}.part`, as every
   write makes one (`faraday-files`), replacing one left by an earlier
   attempt.
2. The source read in parts. Each part is checked before it is written:
   the entries' chunks against the kept chunk digests, the central
   directory and end record against the kept bytes, the whole against
   the kept SHA-256. Then the temporary file gets the entries, the
   signing block, the central directory, and the end record with its
   offset raised. Only bytes already checked are written.
3. The temporary file read back in parts, its SHA-256 compared with the
   one the app worked out for the output while copying.
4. Renamed to the output name, asking first where a file of that name
   is there (`docs/FLOWS.md` decision 58).

**Refused, nothing written.** A part that differs from the first read:
"{source name} changed since it was read. Nothing written." The
temporary file is removed and the pending APK stays. Stop, a read-back
that differs, or a failed write do the same. A stick pulled mid-write
leaves only the hidden temporary file, never a file under the output
name, and the next attempt replaces it.

**After.** The pending APK stays For the stick, tagged on this stick as
any file written, so it can be written beside another copy. Delete
removes it.

**Across a lock and power-off.** A lock keeps the pending APK in
`/run/faraday` with the Outbox: its block, central directory, end
record, chunk digests, sizes and names, none of them secret. Power-off
and the panic key lose it, as they lose the Outbox; signing again needs
the key's vault unlocked.

## 8. The disk process

`faraday-files`' frame protocol (`proto.rs`) gains parts. Each is still
one request and one response, so the protocol keeps its shape, its
hostile-peer checks and its wiping:

- **ReadPart** (stick, name, offset, length up to 4 MiB) → the bytes.
  The app asks for the parts it wants in the order it wants them: an
  APK's last 64 KiB, then its central directory, then the file from the
  start. A whole import is the same, into one buffer.
- **WriteBegin** (stick, name) → the temporary file made, empty.
  **WritePart** (stick, bytes) appends. **WriteEnd** closes it, still
  under its temporary name. **Rename** (stick, temporary name, name,
  replace) puts it in place. **Abandon** removes it. The disk process
  holds at most one write open, and abandons it when the stick goes.
- `ReadQr` answers with a picture's codes only; its bytes come by
  ReadPart like any file's.
- `MAX_FRAME` stops following the largest file: a part of 4 MiB plus
  room for a name, 5 MiB. `Read` and `Write` of a whole file stay for
  files up to one part.
- The timeout is per part, not per file.

The copy-with-insert of §7 is driven by the app, part by part, through
ReadPart and WritePart, rather than run inside the disk process. So
the app writes only bytes it has just checked, the hash is the app's
own and never the disk process's, and the disk process stays a FAT
driver that knows nothing of APKs. Each byte crosses the pipe twice,
in memory, against a stick's 10 to 30 MB/s.

## 9. Untrusted input

Bounds that hold whatever the file's size (`PLAN.md` §4.4):

- the end record searched in the last 65,557 bytes;
- a central directory over 16 MiB is "Not an APK" (that is more than
  100,000 entries);
- `AndroidManifest.xml` at most 1 MiB compressed and 1 MiB inflated;
- the digests at 32 bytes per MiB of file.

The fuzz targets of `docs/APK-SIGNING.md` §5 feed the reader its parts
in every order the app asks for them.

## 10. The Pi and Faraday Desktop

- **The Pi** works the same way through an SD card visit. Its 1 GB
  makes §4's refusal come sooner, and signing only is the usual way to
  sign a large file there.
- **Faraday Desktop**'s sticks are folders read through the same storage
  code (`README.md`, "Run the desktop shell"), so it has the same visit,
  threshold, choice and rule, with the computer's free memory as *F*.
  One behaviour is simpler than two, and the desktop also opens vaults.
  Transfer's picker sends codes only, bounded by the envelope's 256 KiB,
  and is unchanged.

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
  - Stop during a read keeps nothing of the file;
  - a signing-only APK signs, and the next visit writes the signed APK;
  - a source changed between the visits (one byte in the middle) is
    refused and no file appears under the output name;
  - a stick without the source shows "Needs {source name} on this
    stick".
- Judged by other tools (skipped with a note where missing):
  - the visit-2 copy is byte for byte what `apksigner sign` (§8 of
    `docs/APK-SIGNING.md`'s flags) makes of the same unsigned file;
  - a GPG detached signature over a signing-only file passes `gpg
    --verify` against the file.
- `faraday-files/tests/`: ReadPart, WritePart, WriteEnd, Rename and
  Abandon over a FAT image; a write abandoned or cut short leaves no
  file under the final name.
- **The large fixture** is made at test time and never committed:
  `tools/vectors/apk/make.sh large 200` stores 200 MiB of fixed bytes
  (a ChaCha20 stream from a fixed key) as an asset in the small fixture
  APK, aligned. Every judged check above runs on it.

## 12. Batches

`docs/APK-SIGNING.md` batch 1 comes first, on today's reading: it signs
an APK up to the current 18 MiB, which holds the Companion's.

- **L1, any size.** The parts in the frame protocol (§8); no cap; the
  threshold; the boot import's count; the sheet with Import the whole
  file only; §4's rule, the visit's total included; progress and Stop;
  writes in parts, so an Outbox file of any size is written. Its tests.
- **L2, signing only.** §5 to §7: the streamed hashes, the signing-only
  entry, GPG over it, the pending signed APK and its write; §9's
  bounds in the fuzz targets; the 200 MiB fixture. Its tests.

APK batch 2 (the release) can land after APK batch 1 and does not wait
for L1 or L2, so long as the release APK stays under 18 MiB until L1 is
built.
