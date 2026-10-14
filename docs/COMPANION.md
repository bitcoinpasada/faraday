# Faraday Companion

**Status:** spec v0.1 · 2026-10-12 · drafted from the owner's decisions
in discussion (`docs/DECISIONS.md` F15). Not built. §12 records each
decision; none is open. D7, the APK signing key, was replaced on
2026-10-14 (F25): the key is made, kept and used in Faraday, and its
spec is `docs/APK-SIGNING.md`.

The problem it solves: Faraday's device and the Starter both hand things
to a phone, and today the phone has nothing of Faraday's to take them.
The Pi has no USB and no HDMI, so QR codes are its only way out, and a
laptop's webcam reads a 2.8 inch panel badly. The Starter's guides live
on the website, so a phone opening them tells the website which maker's
computer is being set up. The BitLocker recovery key goes to the phone
as text pasted into a notes app that may sync it to a cloud. A wallet's
description, whose loss can lose a multisig's money, has no second home.

Faraday Companion is an Android app with no network access. It reads
the device's codes and shows codes for the device to read, keeps the
files that arrive, keeps wallet descriptions and BitLocker recovery
keys, and carries the Learn pages, the start-from-stick guides among
them. It never takes a seed, a private key or a sealed vault, and it
never signs.

People see it as **Faraday Companion**. It is not the Starter and not a
wallet; "online" is not in its name because it is not online.

## 0. Rules for the implementing agent

- `CLAUDE.md` and `CLAUDE.local.md` apply in full: every build and test
  through `local/remote-run.sh`; every dimension a token; labels, not
  prose, on working screens (explanations go in Learn pages); tests for
  user-facing behaviour only; `just` once at the end of a batch; no
  `rm`; no commits; no agents.
- Batches in §10's order; one review and one completion report each.
- The manifest asks for the camera and the unlock prompt and nothing
  else (§6.1). No `INTERNET`, no `ACCESS_NETWORK_STATE`, no
  storage permission, no library that brings one in. A change that
  needs another permission stops and asks the owner.
- No Android library beyond the platform and the Kotlin standard
  library, as upstream's shell (`opensigner/shells/android`,
  `app/build.gradle.kts`): no AndroidX, no Material, no Play services,
  no analytics, no crash reporting.
- The Companion never writes the clipboard and never opens a link
  (`ACTION_VIEW` or any intent carrying a URL). Data leaves it only
  through the two ways §6.3 names, each one press by the person.
- Completion report: items done, each with its tests and whether they
  pass; what was left out and why; docs changed; renders to look at. No
  restating the spec.

## 1. What it does and what it never does

**Phase 1, this spec: offline only** (owner, 2026-10-12).

| Does | Never does |
|---|---|
| Reads the device's codes, single and animated, and keeps each whole file (§4.2) | Takes seed words, SeedQR, CompactSeedQR, codex32, an extended private key, a private key, `ur:crypto-seed`, an encrypted backup or a sealed vault, by camera, paste or share (§5) |
| Shows a file, a PSBT or a description as codes for the device (§4.3) | Signs, seals, or holds any key that can spend |
| Takes a file shared from another app, and shares a kept file to one (§6.3) | Opens a network connection, a link, or the clipboard for writing |
| Keeps wallet descriptions, with each one's first address (§4.5) | Takes a TOTP secret (`otpauth://`): a code generator belongs on the device |
| Keeps BitLocker recovery keys from the Starter until the person deletes them (§4.6) | Backs anything up to a cloud or copies it to a new phone (§7) |
| Carries every Learn page, and opens the Starter's guide addresses as its own pages (§4.7) | Checks a balance, writes a payment or broadcasts: those are a later, separate app, if ever (§12, D10) |

Online use (broadcasting a finished transaction, watching a wallet,
writing a payment) is not part of the Companion. If it is ever built it
is a second app with its own name, so that this one never gains the
`INTERNET` permission and the check in §6 keeps its meaning.

## 2. What ships

- `faraday-companion-X.Y.Z-android.apk` in the same release as the Starter
  app and the images, listed in the same `SHA256SUMS` and covered by its
  signature (`README.md`, "Verifying a release").
- The same APK on F-Droid, built by F-Droid from the release's tag and
  published with the developer's signature when F-Droid's build matches
  it (F-Droid's reproducible-build method; §8).
- Arm64 phones and the x86_64 emulator, as upstream's shell builds
  (`abiFilters`); Android 8.0 (API 26) and later, as upstream.
- Signed with an APK signing key of the Companion's own, made, kept and
  used in Faraday as the release's OpenPGP key is (§12, D7;
  `docs/APK-SIGNING.md`). It is not the release's OpenPGP key: Android
  accepts no Ed25519 APK signature, it checks an APK's signature
  against the key that signed the installed copy, and every update must
  carry the same one, so losing it means a new app, not an update.

## 3. How it is built

- `faraday/faraday-companion`: the Companion's state, screens and
  decisions, no I/O, tested as `faraday-core` is, through actions in and
  commands out. It is a Faraday app, not OpenSigner's: it draws with
  Faraday's own drawing (`faraday-ui`, the look the desktop app and
  Faraday Starter have: the wordmark with its name, Faraday's palettes,
  cards, sheets), as the Starter does, on a phone's width, portrait
  only. Upstream's `osk-ui` screens are not its look.
- `faraday/faraday-companion-ffi`: the JNI entry points, a copy of
  `opensigner/opensigner-ffi`'s handle table and command encoding with
  the Companion's commands. `libfaraday_companion.so`.
- `faraday/shells/android`: the Kotlin shell, copied from
  `opensigner/shells/android` at a named commit (as
  `faraday/shells/window/src/camera.rs` names its source): the one
  Activity, the view that blits the core's frame and feeds it touches,
  the Camera2 channel that hands the core the luma plane, the document
  picker and creator, the Keystore keys and the system's unlock prompt
  (§7, D4). Of upstream's `KeptSecret.kt` only the Keystore handling
  is copied; nothing that keeps a signing key. Two additions, platform
  API only: the share target and the share source (§6.3).
- Application ID `app.faradayoffline.companion` (§12, D6), version name
  the release's, version code rising with each release.
- **What the core is made of** (§12, D1: a core of its own). The Starter holds a `Faraday` in online mode limited to
  Transfer and Learn (`docs/STARTER.md` §2). The Companion does not: that
  would put the vault, the signer and the key loaders into a phone
  app's library, unreachable from its screens but present. Its core
  depends on `osk-ui`, `osk-shell-api`, `osk-codec` (to recognise what
  arrives, including what it refuses), `faraday-qr` (the transports and
  the file envelope), `osk-learn` and `osk-bip` (a description's
  addresses from its public keys). It does not depend on
  `faraday-core`, `opensigner-core`, `faraday-vault`, `faraday-pgp`,
  `faraday-sb`, `faraday-apk`, `osk-keep`, `osk-backup`, `osk-psbt` or
  `osk-entropy`;
  the `lint` recipe fails if `cargo tree -p faraday-companion` lists
  any of them. The `bitcoin` library under `osk-bip` can derive a
  private key, as any wallet library can; what the crate graph shows is
  that none of Faraday's own code that loads, keeps or signs with one
  is there. Batch A moves what Transfer and Learn need out of
  `faraday-core` into crates both use, and does the same for the
  scanner (`faraday-scanner` depends on `opensigner-core` today; the
  decode is `osk-codec`'s `rqrr` feature), without changing what the
  desktop app or the Starter does, and reports anything it could not
  leave behind.
- Strings are in Faraday's strings file,
  `faraday/faraday-strings/src/en.rs`, with a `companion_` prefix,
  as `CLAUDE.md` requires; the Kotlin shell holds
  only the launcher label. `CLAUDE.md`'s screen rule gains: the
  Companion's screens are this document's §4, drawn as Faraday's
  apps are.
- Builds: `just companion-apk` (debug, the build machine's own tools,
  as `just android-apk`) and `just companion-release-apk` (unsigned,
  in a pinned container on the build machine with the SDK, NDK, JDK,
  build-tools and Gradle versions named in it). The release build
  writes `faraday-companion-X.Y.Z-android-unsigned.apk`, checks it with
  `zipalign -c -P 16 4`, and prints its size and SHA-256; Faraday signs
  it at any size (`docs/APK-SIGNING.md` §4.4, §9;
  `docs/LARGE-FILES.md`). Gradle's downloads are pinned by
  `gradle/verification-metadata.xml`; the wrapper's checksum is named in
  `gradle-wrapper.properties`.
- **No path and nothing about the build computer in the APK**, as
  `docs/STARTER.md` §2: the Rust library built with the remap prefixes
  (`faraday/image/path-remap.sh`), Gradle's `vcsInfo` and dependency
  blob off as upstream has them, and the staging script's `strings`
  check run over the `.so` and `classes.dex` inside the APK.

## 4. Screens

Drawn as Faraday Starter's screens are, with the header **FARADAY
COMPANION** (Faraday's wordmark and the Companion's name), Faraday's
palettes, cards, rows and sheets. The scanner and the codes look as the
desktop app's Scan and QR sheets do. Where this section names a DESIGN
§5 screen it names what the screen does, not how it looks.

### 4.1 Home

Rows, in this order: **Scan a Faraday QR code**, **Send to Faraday**,
**Received** (with the count kept), **Wallet descriptions** (the
count), **BitLocker keys** (the count; the row is shown only when one
is kept),
**Start from the stick**, **Learn**, **About**.

### 4.2 Scan a Faraday QR code

One scanner for every code Faraday shows: the device's (the stick or the
Pi), the Starter's guide addresses (§4.7) and its BitLocker key (§4.6).
(Named "Receive from Faraday" until 2026-10-13, owner.)

A scanner, as the desktop app's Scan sheet, with Paste and no Type. Whatever arrives is
routed by §5. A file in the Faraday envelope, a PSBT, a finished
transaction or a description arrives whole, is kept in Received, and the
screen names it and offers **Open**. An animated code shows its parts
as the device's own scanner does. The same file arriving twice (the
device's code loops) is kept once, by its SHA-256, as Transfer's
Receive does today.

### 4.3 Send to Faraday

A file chosen from Received or from the system document picker, or
shared in from another app (§6.3), shown as codes the way Transfer's
Send shows it: a PSBT as `ur:crypto-psbt` (BBQr one choice away), a
description as text, anything else in the Faraday file envelope, up to
the envelope's 256 KiB. The Animated toggle and the frame rate as the
QR screen has them. The screen stays on while a code is shown.

### 4.4 Received

A list, newest first: name, kind, size, when. Each opens a Record with
**Show as QR**, **Share**, **Save** (the system document creator) and
**Delete**. A received description also has **Keep in Wallet descriptions**. The
list's menu has **Delete all**. Files are kept until deleted.

### 4.5 Wallet descriptions

Wallet descriptions kept by the person: from Received, from a scan or
paste on this screen, or shared in. Each is the text as it arrived,
read by the same reader the device uses, and a name the person gives
it. Its Record: kind, keys (`k of n`), each key's fingerprint, the
first receive address, **Show as QR**, **Share**, **Rename**,
**Delete**. The first address is there to tell two copies of a wallet
apart; it is worked out by the same code as Faraday's, so it does not
count as the check of the Spend tab's step 5, which wants a second
program (`docs/FAMILY.md` §5). The Learn page says so.

A description reveals every address of the wallet and so its balance
and history. It cannot spend.

### 4.6 BitLocker keys

The Starter shows a BitLocker recovery key as a plain-text QR code
(`docs/STARTER.md` §6.2, D18): eight groups of six digits and a line naming
the computer and the drive. The Companion recognises that text when it
is scanned or pasted, on any screen, and keeps it under the computer
and drive it names. The format is written and read by one function in a
crate both apps use, tested in both directions.

A kept key's Record: the computer, the drive, the key (DESIGN's long
secret: masked, shown on hold), when it was kept, and **Delete**. Under
them, one action: **Back in Windows** (owner, 2026-10-12, D5). It
opens a sheet titled **Delete this key?** with two lines of fact, as
F1's sheet carries one:

- "Not needed if Windows started without asking for it."
- "Faraday Starter shows it again the next time Faraday is set up on
  this computer."

and **Delete the key** (primary) and **Keep it**. Both close the
sheet. **Delete** on the Record asks once, without the two lines. The
Learn page (§4.7) says the rest: if Windows
started without asking for the key, the guard worked as expected and
the copy here is no longer needed; it can be deleted anyway, for
privacy, at any time; and the Starter shows the key again before its
firmware steps the next time Faraday is set up on that computer, so it
can be scanned in again then. The key stays valid until it is changed
in Windows, whether or not it is deleted here.

The key is never shared, never saved to a file and never shown as a
code by the Companion: it came from the Starter and goes back to the
person's eyes only.

### 4.7 Start from the stick, and Learn

Every Learn page `osk-learn` carries, as the device and the desktop app
show them, with **Start from the stick** opening the boot pages
(`docs/learn/faraday/boot/`) as below. A scanned address of the
website's guides (`https://faradayoffline.app/start/<page>`, the code
beside each guide in the Starter, `docs/STARTER.md` §7.4) opens the
Companion's own copy of that page, so following the Starter's code with
the Companion tells the website nothing. Any other web address that
arrives is shown as text and is not opened (§0).

**Start from the stick** (owner, 2026-10-13) asks two things, once
each, and remembers the answers, so a person sees only the pages for
their computer:

- The first time: one question, **Which computer are you starting?**,
  answered **Windows** or **Linux** (Mac once Faraday runs on Macs,
  `docs/MAC.md`).
- After that it opens on that computer's page, headed with the choice
  and **Change**:
  - the general guide for that system (`windows`, `linux`);
  - on Windows, **BitLocker first**, opening the BitLocker pages
    (`bitlocker`, `secure-boot-bitlocker`);
  - **Choose the maker** the first time, which lists the maker pages
    (`faraday-learn`'s `MAKERS`) and **Another maker** (the general
    guide); after that the maker chosen, with **Change**;
  - **If something goes wrong**: `not-starting`,
    `screens-that-record`, `replace-faraday`.
- The system and the maker are kept in the app's own storage, as the
  theme is, outside the encrypted stores: they say which computer is
  set up, and nothing else of the person's. **Change** asks again.
- A guide address scanned from the Starter (below) opens its page
  whatever is remembered, and changes nothing remembered.

**The guides are numbered steps** (owner, 2026-10-13). The maker
pages and the Windows and Linux guides are written as steps a person
follows at the computer: `Before you start`, then `Steps` and
`Afterwards`, each step one paragraph beginning `1. `, `2. `, …, a
step's fallback a short paragraph under it ("If it doesn't open: …"),
its sketch under it; then `If something is different` and `Sources`.
Nothing the pages said before is dropped. The maker list is one word
each: **Dell**, **HP**, **Lenovo** (then **ThinkPad** or **Other**),
**ASUS**, **Acer**, **MSI**, **Framework**, **Surface**, **Other**;
**Other** opens a page of the common firmware and boot-menu keys,
taken from the maker pages, with "search online for your computer's
model and *boot menu key*", and the same steps.

**Steps are ticked off.** On a guide page the Companion draws each
step's number in a circle, its fallbacks and sketches indented under
it. Tapping the circle turns it green with the number in white, and
tapping again clears it; a drag that scrolls never ticks. Under the
title, `3 of 8 done`; at the end, once one is ticked, **Start again**.
The ticks are kept per page, as the computer and maker are, so a phone
that stops the app while its person is in the firmware keeps the place.

A new page, `docs/learn/faraday/companion.md`: what the Companion keeps
and refuses; that it has no network access and how anyone can check it
(§6); what a description reveals; the recovery key and deleting it
(§4.6); installing it on a phone that only takes verified developers'
apps (§8).

### 4.8 About

Version and build ID; the APK signing key's SHA-256 certificate
fingerprint (the one `apksigner verify --print-certs` prints), read
from the system at run time (the package's signing certificates), so a
debug build shows its debug key's and a person can compare a release's
with README's; "Network access: none";
the permissions the app holds, read from the system at run time, not
written into the screen; how to check the APK against `SHA256SUMS` and
its signature.

## 5. What arrives and where it goes

Every scan, paste and shared-in file is classified once, by the reader
the device uses (`osk-codec`, `faraday-qr`), then:

| Arrived | Goes to |
|---|---|
| Faraday file envelope | Received, after its SHA-256 is checked |
| PSBT (any form `docs/QR.md` §1 reads), finished transaction | Received, as `.psbt` or the transaction as hex text |
| Wallet description (any form the device reads), account key (`crypto-account`, `crypto-hdkey`, xpub) | Received; on the Wallet descriptions screen, straight into Wallet descriptions |
| BitLocker recovery key text (§4.6) | BitLocker keys |
| The website's guide address (§4.7) | That Learn page |
| Address | Shown as text with its kind |
| Any other text | Shown as text; not kept unless the person presses **Keep** |
| Seed words, SeedQR, CompactSeedQR, codex32, xprv, private key, `ur:crypto-seed`, encrypted backup, sealed vault, `otpauth://` | **Refused, and not kept anywhere.** One line names what it is and says it stays on the Faraday device: "This is a seed. Seeds stay on the Faraday device." The bytes are dropped from memory when the line is shown. |

A refusal is the same whether the item came by camera, paste or share.
A file shared in that holds a refused kind is refused whole.

## 6. Offline, and how anyone can check it

### 6.1 What makes it offline

Android gives an app network access only if its manifest asks for
`android.permission.INTERNET`. Without it the system refuses the app's
sockets below the app, whatever its code does: the check is made by the
system on the app's user ID, not by the app. The Companion's manifest
asks for `CAMERA` and `USE_BIOMETRIC` and nothing else.
`USE_BIOMETRIC` opens nothing on its own: it lets the app show the
system's unlock prompt (D4), granted at install, with no access to any
biometric data.

### 6.2 How it is checked

- By anyone, on the APK: `aapt2 dump permissions
  faraday-companion-X.Y.Z-android.apk` lists what it asks for. F-Droid
  shows the same list on the app's page. The Learn page gives the
  command.
- That the APK is the source: the release build is reproducible
  (`docs/PLANNING.md` §11.1's Android row: identical after removing the
  signing block, the `.so` bit-identical). A person can build the tag
  and compare; F-Droid does so before it publishes.
- In `just`: the `lint` recipe reads the merged release manifest and
  fails on any permission not in §6.1's list, on `INTERNET` by name, on any `<uses-library>`, and
  on any Gradle dependency; it greps the shell's Kotlin for
  `ACTION_VIEW`, `Uri.parse("http`, `ClipboardManager.setPrimaryClip`
  and `java.net.`, and fails on a match.

### 6.3 The only ways out

The Companion opens no connection, but Android lets an app hand data to
another app, and that app may be online. So the claim the Companion
makes is exact: it opens no connection; data leaves it only when the
person presses one of these:

- **Share**: the system share sheet with the one file chosen, through a
  read-only content provider of the Companion's own (platform
  `ContentProvider`, no AndroidX `FileProvider`) granting a read
  permission for that file alone. BitLocker keys are never shared (§4.6).
- **Save**: the system document creator, which writes where the person
  picks.

And in: the share target (`ACTION_SEND` for one file or text), the
document picker, the camera, and Paste. Nothing else.

## 7. What it keeps, and where

- Two files in the app's private storage, each encrypted with AES-GCM
  under an Android Keystore key of its own that cannot leave the
  phone's secure hardware where the phone has it. The core hands the
  shell the bytes to keep; the shell encrypts and writes them, as
  upstream's shell keeps its blob.
  - **Received**: its key needs nothing from the person.
  - **Wallet descriptions and BitLocker keys** (owner, D4): its key works only after
    the person unlocks with the phone's own lock, asked by the system's
    prompt each time either screen is opened. On Android 11 and later
    the key is bound to each unlock (`setUserAuthenticationParameters`
    with a timeout of 0, strong biometric or the device credential); on
    Android 8 to 10 it is valid for 30 seconds after the system's
    confirm-credential screen. The Companion asks again after it has
    been in the background. A scan or share that brings a description
    or a recovery key asks at the moment it is kept.
  - A phone with no screen lock cannot hold such a key. Wallet
    descriptions and BitLocker keys then say "Set a screen lock on
    this phone to keep wallet descriptions and BitLocker keys." and
    keep nothing; Received works as before.
- `allowBackup` is false and the data-extraction rules exclude every
  domain, as upstream's: nothing goes to a cloud backup or to a new
  phone. Moving to a new phone means scanning or sharing each thing
  across again.
- `FLAG_SECURE`, as upstream: no screenshots, and the recents list
  shows a blank card.
- Uninstalling the app deletes everything it kept; the Learn page says
  so.

## 8. Distribution, and Google's developer verification

- The release page and F-Droid (§2). Both under the project's public
  name; F-Droid asks for no identity and builds from source. F-Droid
  copies the signature from the release's APK onto its own build of
  the tag (`apksigcopier`), which works because Faraday's signed APK is
  the unsigned one with the APK Signing Block inserted and nothing else
  changed (`docs/APK-SIGNING.md` §6); its metadata names the
  certificate's SHA-256 in `AllowedAPKSigningKeys`. F-Droid's
  metadata (summary, description, screenshots) is kept in the repository
  in the fastlane layout (`faraday/shells/android/fastlane/`), and the
  merge request to F-Droid's data repository is the owner's to open
  (§10, batch C).
- Google's developer verification applies to every app installed on a
  certified Android phone, from any source: from 30 September 2026 in
  Brazil, Indonesia, Singapore and Thailand, everywhere in 2027 (as
  announced; to be checked when batch C starts). An app whose developer
  has not registered with Google, with an identity document, installs
  there only through Android's advanced flow (developer options, a
  restart, a day's wait) or `adb`. Phones running an Android without
  Google's certification are not affected.
- Faraday is published under a pseudonym (DECISIONS F11). Registering
  would tie the Companion to a verified identity, so it is not
  registered (owner, D8). The Learn page and the website say how to
  install it on a phone that asks. To be looked at again before 2027.

## 9. Security

What the Companion holds: received files (PSBTs, which show amounts and
the wallet's keys; descriptions), wallet descriptions, and BitLocker
recovery keys. None of them spends bitcoin. A phone that is compromised
can read all of them: the wallets' balances and history, and the key to
the named computer's Windows drive. That is why the recovery key is
offered for deletion (§4.6) and never leaves the app, and why the store
is encrypted and kept out of backups.

What a compromised Companion cannot do is move money. It relays PSBTs
and transactions; it can alter one, and the device then shows what the
altered one pays, the change checked against the wallet, before
anything is signed (`docs/FAMILY.md` §8), and the transaction id the
person writes down there is what any broadcaster must report
(`docs/FAMILY.md` §11). The Companion is no more trusted than the
coordinator it stands beside.

It cannot learn a seed by accident: §5 refuses every secret kind on
every way in, and its core does not contain the code that would load
one (§3, D1).

## 10. Batches

- **A, the core and the shell.** The crate split of §3 (D1); the
  Companion's core, FFI and Kotlin shell; Home, Receive, Send, Received,
  Learn and About on the emulator; §5's routing and refusals; §6.2's
  lint. Renders: `just snap` gains the Companion's screens at one phone
  size.
- **B, keeping.** Wallet descriptions, BitLocker keys (§4.6, with the format shared
  with the Starter, which batch W of `docs/STARTER.md` writes; until then
  against the format as §6.2 of that document gives it), the two
  encrypted files and the unlock before Wallet descriptions and BitLocker keys
  (§7), the share target and source (§6.3), the guide addresses
  (§4.7), the Learn page.
- **C, release.** The pinned container, the reproducible unsigned
  release APK and a second build that matches it, made from the tag on
  another computer, whose SHA-256 Faraday's review requires before it
  signs (`docs/APK-SIGNING.md` §4.4, §9), the staging script's
  `strings` check over the APK, `stage-release.sh --companion` (the
  signed APK checked by `apksigner` against
  `faraday/companion-cert.pem`, and its line in `SHA256SUMS`), README's
  "Verifying a release" paragraph on the APK certificate, the fastlane
  metadata and `AllowedAPKSigningKeys` (`docs/APK-SIGNING.md` §9). The
  key itself is `docs/APK-SIGNING.md`'s batch 1, built into Faraday
  before this batch, since it needs no release APK; the owner makes it
  on a stick and commits its certificate before the first release.
  Opening the F-Droid merge request is the owner's.
  **Built (2026-10-14), but for what needs the certificate.**
  `just companion-release-apk` builds in `faraday/build/android/`
  (versions named in its Containerfile; Gradle's downloads pinned by
  `faraday/shells/android/gradle/verification-metadata.xml`, written by
  `just companion-gradle-verification`, and the wrapper's
  `distributionSha256Sum`); three builds, two checkouts, fresh containers
  and Gradle homes gave one SHA-256. `stage-release.sh VERSION --companion
  FILE` is built and tried with a throwaway key; it refuses while
  `faraday/companion-cert.pem` is missing. The fastlane metadata is in
  `faraday/shells/android/fastlane/`. Left for the owner: the key and the
  certificate, `AllowedAPKSigningKeys`, README's fingerprint paragraph
  (TODOs in `fastlane/README.md` and README.md), the second build on
  another computer, and §8's check of Google's developer verification
  dates.
- **Tried on a phone**, by the owner, with a stick and with the Pi:
  receive a file and a signed PSBT from each; send a PSBT shared from a
  phone wallet; the Starter's recovery key scanned and deleted.

## 11. Tests

User-facing behaviour only (`CLAUDE.md`):

- A file the device sends as codes (single, animated, and with frames
  missed and the loop read again) arrives whole and is kept once.
- A PSBT shared in comes out as codes the device's scanner reads as the
  same PSBT; a description likewise.
- Every refused kind in §5, by scan, paste and share, is refused with
  its line and nothing is kept: the store after the attempt is the store
  before it.
- The Starter's recovery-key text is read back as the computer, the
  drive and the key it was written with; **Delete the key** leaves no
  key in the store.
- The website's guide address opens the page it names; another web
  address is shown as text.
- A kept description's first address matches the published vector for
  that description (BIP-84's, and the interop files of
  `docs/INTEROP.md`).
- The store written and read back through the shell's encryption gives
  the same received files, wallet descriptions and BitLocker keys.
- On the emulator: Wallet descriptions and BitLocker keys do not open
  until the phone's unlock is given, and ask again after the app has
  been in the background; with no screen lock set they keep nothing and
  say so, and Received still works.

## 12. Decisions

All taken by the owner on 2026-10-12: D2, D3, D5 and D10 in discussion,
the rest as recommended in v0.1 of this document.

- **D1 The core:** a core of its own without Faraday's vault, signer or
  key loaders, so that "holds no seed" is a property of the crate
  graph, checked by a lint, as "reaches no network" is a property of
  the manifest (§3). The Starter's way, a `Faraday` limited to Transfer
  and Learn, was the less work and was not taken.
- **D2 Phase 1 only:** the offline app; nothing online in it (§1).
- **D3 Name:** "Faraday Companion" to people; `faraday-companion` in
  the code.
- **D4 Opening the kept things:** the phone's own unlock, asked by the
  system's prompt, before Wallet descriptions and BitLocker keys open,
  with their file's key bound to it (§7).
- **D5 The recovery key:** kept until the person deletes it, with
  **Back in Windows** offering the deletion; the sheet and the Learn
  page say that a key Windows did not ask for is not needed here, that
  it can be deleted anyway, and that it can be scanned in again when
  Faraday is next set up on that computer (§4.6).
- **D6 Application ID:** `app.faradayoffline.companion`, from the
  website's name; it cannot change after release.
- **D7 The APK signing key** (replaced 2026-10-14, `docs/DECISIONS.md`
  F25): made, kept and used in Faraday, as the release's OpenPGP key is.
  An RSA key in a vault (record type 12), whose sealed copies on sticks
  are its second copy; the release APK is built unsigned on the build
  machine and signed on a Faraday stick by **Sign an APK**, once an
  independent second build's SHA-256, entered in the review, matches.
  The spec is
  `docs/APK-SIGNING.md`. Until 2026-10-14 it read: generated and kept by
  the owner outside Faraday.
- **D8 Google's verification:** not registering, for the pseudonym;
  the install route on a phone that asks is documented (§8). To be
  looked at again before 2027.
- **D9 iPhone:** none in phase 1.
- **D10 Online use:** not in the Companion. A separate app, if ever,
  decided on its own.
