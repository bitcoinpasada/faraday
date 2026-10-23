# Large files on a stick visit: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/LARGE-FILES.md` on 2026-10-22. Each keeps
its original number; the source keeps a one-line stub under the same
heading.

## Header and introduction (from docs/LARGE-FILES.md)

Spec v0.2 · 2026-10-14 · drafted from the owner's decision
(`docs/DECISIONS.md` F26), with a security review's fifteen findings
applied the same day. Not built.

This applies to every file read on a stick or SD card visit, not only
to APKs. `docs/APK-SIGNING.md` is the first user.

## §0 Rules for the implementing agent (from docs/LARGE-FILES.md)

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

## Removed with APK signing (2026-10-23, F99)

The spec's APK parts were cut when APK signing was removed: the Sign with
APK key actions, the APK reads of the sheet, the chunk digests of §5, the
APK bounds of §9 and its fuzz targets, the 200 MiB APK fixture, and §7
below, which `docs/LARGE-FILES.md` keeps as a heading. §7 as it stood:

### 7. Writing a signed APK on a later visit

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
