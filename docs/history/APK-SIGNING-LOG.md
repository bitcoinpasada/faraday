# The APK signing key: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/APK-SIGNING.md` on 2026-10-22. Each keeps its
original number; the source keeps a one-line stub under the same heading.

## Header: the spec's version and build record (from docs/APK-SIGNING.md)

**Kind:** Spec
**Status:** Built; batch 2 as far as the key allows

**Switched off (F59, 2026-10-20).** APK signing is not in the app: the
switch `Faraday::apk_signing` is off by default. Everything here is
built and tested with the switch on, and comes back when it is turned on.

**Faraday Companion was dropped (F34, 2026-10-15).** This spec was
written for the Companion's APK key. **Sign an APK** stays, for the APK
of any app: the name and the package start empty, and the release
carries no APK. Where this text names the Companion, its APK, its
container, docs/COMPANION.md or faraday/companion-cert.pem, that is
history: §9 and batch 2 of §11 are not built and have no tool left to
build them with, and F34 says how the Companion comes back.

Spec v0.4 · 2026-10-14 · decided by the owner
(`docs/DECISIONS.md` F25; §12 records the answers), revised after an
adversarial review the same day. Batch 1 (§11) is built, with the changes
the build made (§13); its wait for two copies is now the shared backup
flow's, `docs/BACKUP-FLOW.md` (F31), built in batch B1a.
Batch 2 is built as far as the key allows (§11); `docs/LARGE-FILES.md`
(F26) is not built.

## §0 Rules for the implementing agent (from docs/APK-SIGNING.md)

- `CLAUDE.md` applies in full: every build and test on the build machine; every dimension a token; labels, not
  prose, on working screens (the explanations are §10's Learn page);
  tests for user-facing behaviour only; `just` once at the end of a
  batch; no `rm`; no commits; no agents.
- Batches in §11's order; one review and one completion report each.
- The private key never leaves the vault: no file, code, paper form or
  keystore that Faraday makes carries it (§3); `open.py --field`, run by
  the person without Faraday, is the one way out. A change that would
  need another stops and asks the owner.
- Completion report: items done, each with its tests and whether they
  pass; what was left out and why; docs changed; renders to look at. No
  restating the spec.

## §9 The release (from docs/APK-SIGNING.md)

**Historical (F34): the release no longer carries an APK, and
`stage-release.sh` has no `--companion`.** This is what the Companion's
release was, with the key in it:

1. The pinned container builds
   `faraday-companion-X.Y.Z-android-unsigned.apk`, checks it with
   `zipalign -c -P 16 4`, and prints its size and SHA-256.
2. **The second build**: the same container, from the same tag, on
   another computer that shares no account, key or network share with
   the build machine (COMPANION §10's "second build that matches"). It
   prints its SHA-256, which the person reads there, never through the
   build machine. The two must be equal; if they are not, nothing is
   signed.
3. On a Faraday stick (or Pi), in one session: a stick visit brings the
   unsigned APK in, from the build machine's output; **Sign an APK**,
   with the second build's SHA-256 entered in the review (§4.4); the
   signed APK goes out.
4. `faraday/tools/stage-release.sh` gains `--companion FILE`: it runs
   `apksigner verify --print-certs`, refuses an APK whose certificate is
   not faraday/companion-cert.pem's, and adds the APK and its line to
   `SHA256SUMS`.
5. A second visit in the same session signs `SHA256SUMS` with the
   release key, as before.

faraday/companion-cert.pem is the certificate, committed once the key
is made, as `faraday/release-key.asc` is. README's "Verifying a
release" gains a paragraph: the APK is signed with an APK key made in
Faraday whose private part has never left a Faraday vault; its
certificate is that file; its SHA-256 fingerprint in full; and
`apksigner verify --print-certs faraday-companion-X.Y.Z-android.apk`
prints the same as "Signer #1 certificate SHA-256 digest". F-Droid's
metadata names the same fingerprint in `AllowedAPKSigningKeys`.

## §11 Batches (from docs/APK-SIGNING.md)

- **1, the key and the signer.** `faraday-apk` (§5, §6, §2), the
  `faraday-sb` change, record type 12 with `open.py` (reading type 12,
  and `--field … --out`, §7) and the vectors, the stick's FAT volume
  serial on a vault's receipt (`docs/VAULT.md` §6, for §3's two sticks),
  `faraday-core/src/apk.rs`, the screens of §4, the Learn page, §8's
  tests and the fuzz target, the docs of §7. Needs no release build:
  the fixtures stand in. Renders: the tile, Make, the key's page and the
  review at `1280x800` and `480x640`.
- **2, with the Companion's batch C (dropped, F34: removed from the
  tree, the staging script and the README).** The container's step 1, the
  second build's step 2 written as a release step, the staging script's
  step 4, the README paragraph, the fastlane metadata and
  `AllowedAPKSigningKeys`.
  **Built (2026-10-14) as far as the key allows:** the container's
  step 1, the staging script's step 4 (`--companion`), the fastlane
  metadata. Left until the certificate exists: the README paragraph and
  `AllowedAPKSigningKeys`, both TODOs named in
  faraday/shells/android/fastlane/README.md. Step 2, the second build,
  is `just companion-release-apk` run from the tag on another computer.
- **The owner**, between them: the key made on the stick of a published
  release that has batch 1, in the vault the owner chooses (§3, §12),
  that vault written to two sticks, and the certificate committed as
  faraday/companion-cert.pem (§12; dropped with batch 2).

Batch 1 signs APKs up to today's 18 MiB read; any size comes with
`docs/LARGE-FILES.md`'s batches L1 and L2, after batch 1. Batch 2 does
not wait for them while the release APK stays under 18 MiB.

## §12 Decided by the owner (2026-10-14), items 3 and 6 (from docs/APK-SIGNING.md)

3. Signing waits for a written vault (§3); after the review, for
   receipts from two sticks with different FAT serials. Replaced by F31
   (2026-10-14): the shared backup flow's two writes, the second refused
   on the stick holding the first copy and confirmed by a hold.

6. The certificate's name was "Faraday Companion" (superseded, F34:
   the field starts empty).

## §13 What batch 1's build changed or chose, items 4 to 6 (from docs/APK-SIGNING.md)

Items 4 to 6 below describe batch 1's placeholder for the backup; batch B1a
replaced it with the shared flow (`docs/BACKUP-FLOW.md`; `docs/history/BACKUP-FLOW-LOG.md` has its build), which keeps the
certificate and the signing as items 1 to 3 and 7 to 9 say.

4. **The wait is a log, not receipts** (§3): the last receipt is
   replaced by the next write, so a key made this session keeps the
   sealings that hold it and its first write (name, length, hash), kept
   across a lock. `StickInfo`, the disk protocol and the receipt carry
   the FAT volume serial (`faraday-fat`, `faraday-files`,
   `faraday-disk`), for display; nothing gates on it.
5. **Back up first locks the vault** (§3), since a stick is handed only
   to a clean process, and the key's page comes back at the next unlock.
   The two writes are placeholders for the shared backup flow.
6. **Make is the vault's form** with two fields and the hint, then the
   vault buttons where several are open; the result is a panel on the key's
   page (§4.2), not step cards. The certificate's SHA-256 is a mono row,
   not a reference row to Compare (no such screen).

## §13 What batch 1's build changed or chose, the rest (from docs/APK-SIGNING.md)

Items 1 and 2 were already stated in §2.2 and §6; items 3, 7 and 9 are now stated in §4.4, §7 and §10; item 8 is the header's pointer to `docs/LARGE-FILES.md`.

Where this document was wrong or silent, the build followed the closest
existing pattern (`gpg.rs`, `secure_boot/mod.rs`) or `apksigner`:

1. **Zero bytes before the block and the end record's offset** (§6):
   `apksigner` pads the entries to a multiple of 4096 before the block,
   and its digest counts the end record with the offset of the block's
   start. Found by the byte-identical test.
2. **The v3 SDK range is 24 to 2147483647** whatever the APK's
   `minSdkVersion` (§2.2), measured.
3. **Files and the signed file.** (A manifest attribute named like
   one of the four it reads with another resource ID, the ID with another
   name, a `package` with an ID, a namespaced element at the first two
   levels, more than 256 attributes, or an unsigned or absurd
   `minSdkVersion` is "Not an APK".) The output is `x.apk` only for
   `x-unsigned.apk`; any other name, `x-unsigned` included, gains
   `-signed`, so `x-unsigned` becomes `x-unsigned-signed.apk`. A file is an APK when its central
   directory names `AndroidManifest.xml`; a PSBT, a picture or random
   bytes is not listed at all (§4.4's "Not an APK" is for a ZIP that
   names a manifest and breaks a rule of §5). A signed APK is "Already
   signed", with up to 4095 zero bytes between the last entry and the
   block allowed, as `apksigner` writes them. A `zipalign`-padded extra
   field may end in zero bytes that do not frame (`zipalign` pads so);
   the reader allows them and refuses a ZIP64 field. A stored directory
   entry (a name ending in `/`) is not checked for alignment.
4. to 6. Batch 1's placeholder for the backup, replaced by the shared flow
   (`docs/BACKUP-FLOW.md`, batch B1a): moved to `docs/history/APK-SIGNING-LOG.md`.
7. **`open.py`** reads types 11 (a backup plan, missing from its table)
   and 12; `--index N` counts from 1.
8. **Not built** (`docs/LARGE-FILES.md`): files over today's 18 MiB read,
   signing-only files, the single streamed pass of §5's last paragraph,
   and the review's "Not enough memory to open {vault}". `faraday-apk`
   reads through a `Source` in parts, so they need no change to it.
9. **No info button on the Tools tile**: the page is named by Learn's `?`
   on the vault contents while an APK key is shown; the Tools page has
   no per-tile info.
