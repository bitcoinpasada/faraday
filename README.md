# Faraday

Faraday is an offline Bitcoin signing core written in Rust, with OpenSigner
as the reference application built on it for DIY devices, Android, iOS and
desktop. No networking code exists in either. The crates and packages below
keep their original `osk-*` and `opensigner-*` names.

See `docs/FEATURES.md` (what is supported), `docs/PLANNING.md`
(architecture, threat model, decisions log), `docs/DESIGN.md` (the design
system) and `docs/UX.md` (jobs, navigation, flows).

There is no release yet: nothing has been published, no release key
exists, and anything offered as a signed OpenSigner build is not from
this project. Build from source, and read `docs/VERIFY.md` for what
checking a release will look like once there is one.

## What it supports

`docs/FEATURES.md` lists every protocol, format and feature in the tree,
with what is read, what is written and what is still missing. In short:
BIP-39 keys in all ten wordlists, SLIP-39 and Codex32 keys and backups,
BIP-85 children, Seed XOR and encrypted backups; single-sig, multisig,
taproot multisig, MuSig2, FROST and recovery wallets built on the device,
and miniscript, Taproot tree and watch-only wallets read from a
coordinator; PSBT signing with change verification, MuSig2 and FROST
signing checked against Bitcoin Core, BIP-137 and BIP-322 messages; BSMS
records; BC-UR and SeedQR; a key explorer and tools; and shells for a
Raspberry Pi card, a USB stick, Android and the desktop.

Not yet: PSBT version 2, anti-exfil, the iOS and browser shells, camera
capture on Windows, and translations. See the roadmap in
`docs/PLANNING.md` §8.

## Run the demo

**This is a development build. Use test seeds only. Never type a mnemonic
that holds funds into it.**

Prerequisites: stable Rust (see `rust-toolchain.toml`), a C compiler
(`secp256k1-sys` builds libsecp256k1 from source) and, for `just test`,
`cargo install cargo-nextest --locked`. On Linux the window
shell also needs the X11 or Wayland client libraries at runtime, including
`libxkbcommon-x11`; macOS and Windows need nothing extra.

```
# The desktop window: one fixed size, 960x640 at 160 dpi, with the
# sidebar beside the content pane.
cargo run -p opensigner-desktop

# The other screens the app is built for. --size and --dpi are review
# flags, not a way to run at an arbitrary size.
cargo run -p opensigner-desktop -- --size 480x640 --dpi 286              # the 2.8" panel
cargo run -p opensigner-desktop -- --size 240x320 --dpi 143 --scale 3    # the smallest panel
cargo run -p opensigner-desktop -- --size 1080x2340 --dpi 420 --scale 2  # a phone

# Every screen as PNGs at the four reference sizes (the smallest panel,
# the 2.8" panel, a phone and the desktop window), into out/snapshots/,
# plus the design-system gallery (docs/DESIGN.md) into out/snapshots/<size>/gallery/.
# The phone renders carry the 63 px (24 dp at 420 dpi) a gesture bar takes;
# the snapshot shell's --inset-bottom PX and --inset-top PX say it for a
# one-off run, and every other size reports none.
just snapshots

# The gallery on its own, interactively:
cargo run -p opensigner-desktop -- --app gallery

# The camera: Linux and macOS. On Linux, Scan finds the first /dev/video*
# that streams video and --camera names one; on macOS it opens the default
# AVFoundation device. --no-camera refuses so that the file fallback can be
# reviewed on a machine that has a webcam.
cargo run -p opensigner-desktop -- --camera /dev/video0
cargo run -p opensigner-desktop -- --no-camera

# The settings the app keeps between runs -- network, unit, auto-lock,
# auto-wipe and the PIN-pad shuffle, and nothing else. A webcam is upright,
# so the camera-rotation row is not offered here.
# They live in $XDG_CONFIG_HOME/opensigner/settings (or ~/.config/...) on
# Linux and in ~/Library/Application Support/OpenSigner/settings on macOS;
# --settings names another file and --no-settings keeps none, which is how
# a device with its defaults is reviewed.
cargo run -p opensigner-desktop -- --settings ./my-settings
cargo run -p opensigner-desktop -- --no-settings

# macOS grants a camera permission to an application bundle and to nothing
# else, so the camera comes through this rather than `cargo run`: it builds
# and ad-hoc signs out/mac/OpenSigner.app. Open it with right-click -> Open
# (twice, once per copy), since it is not notarised. A bare binary run from
# Terminal asks for Terminal's camera permission instead, which may or may
# not have been granted.
just mac-app
```

In the window: Home → **Keys** → *Load a key* → *Type the words* → 12 →
English, then type the BIP-39 test mnemonic on the on-screen keyboard (or
with the physical keyboard; Enter takes the first suggestion):

```
abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about
```

Skip the passphrase or add `TREZOR`, add the key, set the session PIN,
then open **Keys** and tap the fingerprint (`73c5da0a` without a
passphrase) for the key menu: **Addresses** lists the addresses and opens
each with its QR, **Wallet export** shows the descriptor and the account
key as QRs, **Backup** shows the words, their numbers and the two seed
QRs, each inside a panel that reveals while a finger is on it or for
30 s from the eye in the app bar. The
tier badge in the status line says what this device can be trusted with;
the gear beside it is Settings, whose About section carries the version
and the self-test result.

### Create a key with dice

Home → **Keys** → *Create a key* → *Dice* → 12 → English, then roll a
real six-sided die fifty times and enter each roll on the pad (or type
the digits 1–6). Each roll shows for half a second and then masks; touch
and hold the panel to see them all. The ✓ key continues once fifty are
in. The sanity step shows the count per face and flags long runs, skew
and counting patterns; a row shows the math — the
entropy (SHA-256 of the rolls as digits, the same as
`echo -n 6666666651… | sha256sum`), the checksum bits and how the first
word is read. The words appear as wordlist numbers; touch and hold the
panel to read them, write them down, then answer the quiz (every word,
random order). Skip the passphrase and add the key. Key menu › Backup
runs the quiz again for any loaded key, with a helper mode that reveals
nothing, and shows the words or their numbers.

### Sign a PSBT

The desktop shell has no file dialog: `--psbt` is the file offered when
the app asks for one, `--files DIR` is the directory it lists when
nothing is offered (`$HOME/Downloads` when it exists, otherwise the
current directory), and `--out` is where the signed result goes. A regtest
spend from the test seed is committed as
`tools/vectors/psbt/demo-regtest.psbt` (regenerate it with
`cargo run -p osk-psbt --example inspect -- --make-test-psbt <file> --network regtest`).

```
cargo run -p opensigner-desktop -- \
    --psbt tools/vectors/psbt/demo-regtest.psbt --out signed.txn
```

Settings › Network → **Regtest**, load the key as above (skip the
passphrase), then **Sign** → *Read a file*. Read the summary,
outputs, inputs and warnings, hold *Hold to sign*, and *Save to file*
writes the final transaction as hex to `signed.txn` (a partially signed
PSBT would be written as binary). *Show signature bytes* shows the exact
signature for comparison with another signer.

A transaction with more than one SegWit v0 input is signed only when
each input carries the transaction it spends. Sparrow includes those in a
QR only for a keystore registered with device type Krux. A transaction
with one input is signed without them.

### Sign a message, and check a signed one

**Sign** → *Sign a message* opens the scanner for any text at all: a QR,
or *Read a file* (`tools/vectors/message.txt` is a message with newlines
in it). The message is shown whole, with the key, the script type of the
address it will be signed for and the format above it; *Continue*, then
*Hold to sign*. The result carries the address, the format and the
signature, which *Show as QR* puts on one code, the signature row shows
whole for comparison, and *Save to file* writes as `signature.txt`.

**Verify** → *Signed message* reads the same text form back and says
whether the signature holds and which address form it checked
(`tools/vectors/signed-message.txt` is one this device made).

The text form, which is what the file holds and the QR carries, so an
air-gapped device needs no typing:

```
bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu
KAH7H1Xb3Ols3JJIDJldXqa+3XAMKyMeuPbmuNjV/npTDDF/JfcUMgC8n+Eyer6cqIRo16HsXMxQjyMBdfzqMkI=
I hold the key behind the address below.
```

The address on the first line, the signature on the second, the message
from the third line to the end with its newlines kept.

Two formats. **BIP-137** is the legacy compact signature with a recovery
header, for a p2pkh address and, in the header ranges Electrum and
Trezor settled on, for p2sh-p2wpkh (35 to 38) and p2wpkh (39 to 42); it
is the default, because more verifiers read it. **BIP-322** "simple" is
the witness of the BIP's `to_sign` transaction, for p2wpkh and p2tr; it
is the only form a taproot address has, and a nested-segwit address has
none, since a simple signature carries no scriptSig. Checking reads the
recovery id from the header and asks the address which form to rebuild,
so a signature from a wallet that always writes the p2pkh range verifies
against the segwit address it was made for.

### Scan QR codes, and show them

Home › **Scan** routes any code by what it holds (SeedQR, PSBT, animated
BC-UR, address, descriptor, account key, words). On Linux the scanner
opens a camera; everywhere else, and with `--no-camera`, it offers
*Read a file* instead, and the file's content goes through the same
routing. A frame is the preview and the shell reads the codes in it on a
worker thread (`opensigner/shells/scanner`), handing the core each
payload it finds: a decode costs more than the gap between frames on a
small board, and the core has one thread for the screen. The snapshot shell can also feed
PNGs of QR codes as camera frames (`frame tools/fixtures/qr/address.png`
in a script; `just qr-fixtures` regenerates the fixtures from the test
seed and the demo PSBT), which is how `tools/scripts/scan-routing.txt`
walks every route. Key detail › Wallet › *Export to coordinator* shows
the descriptor or account key as a QR for Sparrow and friends; Key
detail › Backup › *SeedQR* / *CompactSeedQR* show the seed as a code that
stays blank until the panel is touched and held; and the Sign result has
*Show as QR*, static when it fits and animated `ur:crypto-psbt` otherwise
(`tools/scripts/export-qr.txt` snaps all of them).

### Register a multisig wallet

A multisig wallet is a BIP-388 wallet policy: a descriptor template whose
keys are `@0/**`, `@1/**` … and the list of keys those stand for, the
form Sparrow, Ledger and Coldcard exchange. Scan or read one (the
descriptor of the same wallet does as well, and `multi`/`sortedmulti`
becomes a policy) and its review states the quorum, the script type, each
cosigner's fingerprint with MINE where a loaded key matches, and the
descriptor's checksum, with the descriptor itself one tap away. **Use
this wallet** registers it for the session: while it is in use, the
wallet is a row under the keys on **Keys**, the Sign flow verifies change
outputs of that wallet by re-deriving them (not by trusting the
coordinator's claim), and **Verify** answers for its addresses. Nothing
is remembered across restarts, and a wipe or exit forgets the wallets
with the keys. `tools/scripts/wallet.txt` walks all of it with the
fixtures in `tools/vectors/psbt/wallet-2of3.*`.

### Explore a key

Home › **Explore** is the live BIP-39/BIP-32 workspace over a loaded key
or over words typed just for exploring (nothing is added to the session,
and leaving asks before it erases them). It is two screens. **Keys** puts
the applied derivation path across the top as tappable crumbs, the BIP
purpose presets under them, and the fingerprint and extended keys of the
crumb that is boxed, then both encodings of the account together and the
rows into this key's addresses and into the second screen. **Words and
bits** has every word with its number and 11-bit group, the checksum
arithmetic with its verdict first, and the PBKDF2 seed and master key.
The path itself is typed on a digits-`/`-`h` keyboard whose ✓ is dead
while the text does not parse. Words, numbers, bits, the seed and every
private key sit in a panel that reveals them while a finger is on it.
What a path, a checksum, a seed and the encodings are is a Learn page,
not a paragraph on these screens. `tools/scripts/explore.txt` walks it. On a wide window the six
areas are the sidebar, which is beside every screen.

### Session PIN, auto-lock, auto-wipe

The first key loaded in a session asks for a 4–8 digit PIN, typed twice
on the on-screen pad (touch or mouse only; Settings › *Shuffle PIN pad*
scrambles the digits). It lives only in memory, as a salted hash, and it unlocks
the lock screen of this session and nothing else. After two minutes
without input the screen locks: the master keys are dropped, the seeds
stay sealed in memory under a session key that is rotated on every lock,
and how many keys are sealed, a PIN pad and the wipe rule are all that
shows; the last minute
before the lock is counted down on Home and inside the wizards. Eight
wrong PINs wipe every key. After ten minutes without input the keys are
wiped whether locked or not. Both timers are in Settings › Session with
a one-line why each. Settings › *Wipe all keys* and *Wipe and exit* each
open a screen that says what will happen, with a hold to do it. `tools/scripts/session.txt` walks
all of it.

## Android

The Android shell is a Kotlin app around the same core: one Activity, one
View that blits the core's framebuffer, and a hand-written JNI layer
(`opensigner/opensigner-ffi`). The Activity pads its view by the system
bars and the display cutout, so the frame it hands the core is the
visible area and it reports no insets of its own; the black window
background fills under the gesture bar. Apart from the Kotlin standard library it
has no dependencies — no AndroidX, no Material, no Play services, no
analytics, and not `androidx.biometric` either: the authentication prompt
behind a kept key is the platform's own. Two permissions: the camera,
asked for at the first scan and used only to read QR codes, which the Rust
side decodes from the frames itself; and `USE_BIOMETRIC`, which shows that
prompt and reads no biometric data. Refuse the camera and the scanner says
"No camera" while *Read a file* keeps working. There is no INTERNET
permission, so the app cannot reach the network even by accident.

### Prerequisites

- The Android SDK, with `platform-tools`, `platforms;android-36`,
  `build-tools;36.0.0` and `ndk;29.0.14206865`, which are the versions the
  Gradle files pin, and (for `just android-run`) `emulator` and a system
  image.
- A JDK 21. Gradle itself comes from the wrapper in
  `opensigner/shells/android`; no Android Studio is needed.
- `cargo install cargo-ndk`, and
  `rustup target add aarch64-linux-android x86_64-linux-android`.

Two environment variables, and nothing hard-coded anywhere in the repo:

```
export ANDROID_HOME="$HOME/Android/Sdk"          # wherever yours is
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/<version>"
```

### The four commands

```
just android-lib   # cross-compile libopensigner.so for arm64-v8a and x86_64
just android-apk   # assemble the debug APK (rebuilds the library first)
just android-run   # boot the emulator if needed, install, launch, screenshot

just android-release-apk   # the release APK, unsigned, in a pinned container
```

The first three build on this machine with the toolchain it has, which is
what development wants. `just android-release-apk` is the release build:
it runs in the container `tools/build/android/Dockerfile` describes, where
the JDK, the Android SDK, the NDK and Rust are all pinned, so that a
second builder gets the same APK. It is unsigned; see **Reproducible
builds** below.

Sideload onto a phone with the SDK's `adb`:

```
"$ANDROID_HOME/platform-tools/adb" install -r \
    opensigner/shells/android/app/build/outputs/apk/debug/app-debug.apk
```

### What it can and cannot do

Everything the desktop shell can do: scan a QR code with the camera, load
a key by typing the words, create one from dice, coins or hex, back it up
and take the quiz, sign a PSBT read through the system document picker,
verify an address, export a wallet descriptor, explore a key, and the
session PIN with auto-lock and auto-wipe — and, on a device with a secure
element, one thing the desktop shell cannot: keep a key on the device.
Files go in and out through
`ACTION_OPEN_DOCUMENT` and `ACTION_CREATE_DOCUMENT`; entropy comes from
`SecureRandom`.

**The camera** is Camera2 and nothing else (`docs/PLANNING.md` §16.35):
frames at 640 × 480, the camera permission asked for at the first scan; a
refusal leaves the scanner offering *Read a file*.

**A key kept on the device** is what the Android Keystore adds (§6, §15
item 32). *Keep on this device*, offered as a key is added, stores every
key's words as ciphertext the core made, and keys added or forgotten
afterwards follow without another authentication; the shell wraps those bytes again with an
AES-256-GCM key of its own and keeps them in `filesDir/kept`. Opening
them needs the session PIN *and* one HMAC from a second Keystore key that
never leaves the secure element and requires an authentication — a strong
biometric or the device credential — for every single use. So a copy of
the file cannot be attacked offline: each guess costs one authentication
on that one chip, and forgetting that key — its own *Forget*, or
*Settings › Wipe all keys* — deletes the file and both keys together,
which makes every copy of the bytes permanently useless. Both keys ask for StrongBox first and fall back to the TEE, and
the app reports which it got in *Settings › About › Secure hardware*,
read from key attestation and confirmed against `KeyInfo`. The same
attestation says how the device booted, which is *Settings › About ›
Verified boot*: on a phone whose bootloader is unlocked the system asking
the element for the key is not the one the manufacturer signed, so *Keep
on this device* carries a caution above the hold. The person may still
keep the key.

The settings — network, unit, auto-lock, auto-wipe and the PIN-pad
shuffle — and the kept key's blob are the only two files the app writes,
both in its own private storage, which no other app can read and neither
backup nor device-to-device transfer copies. The camera code turns every
frame upright from the sensor's orientation, so the camera-rotation row is
not offered.

**The tier depends on the device.** A phone whose Keystore is backed by a
TEE or a StrongBox chip runs as Tier B (`docs/PLANNING.md` §3) and can
keep a key. Where there is no secure element — no secure lock screen, an
Android older than 11, or an emulator whose Keystore is software, which is
what the `headless` AVD reports — the app runs as Tier C, keeps nothing,
and a key lives in memory for one session. Either way the APK
`just android-apk` builds is debug-signed and built with this machine's
toolchain; the release APK is the unsigned one from the pinned container.
Use test seeds only.

The window sets `FLAG_SECURE`, so screenshots and screen recordings of the
app come out black — including `adb exec-out screencap`. That is the flag
working. `just android-run` captures through the emulator console instead
(`tools/android-screenshot.py`), which reads the host framebuffer and works
on an emulator only.

## Raspberry Pi

`just pi-bin` builds the shell in `opensigner/shells/pi` as a static
32-bit ARM binary, into `out/pi/<board>/opensigner-pi`. The board
directory names the toolchain; for both boards there today that is the
`armv7-unknown-linux-gnueabihf` Rust target and the distribution's
`gcc-arm-linux-gnueabihf` (or `OPENSIGNER_PI_CC`). A panel, a touch
controller and a camera are all it talks to, and it is told which panel
it has: `--size`, `--dpi`, `--touch-name` and `--touch-grid` are flags,
not constants, so one binary serves every panel.

`just pi-image` puts that binary on a bootable microSD image: Buildroot in
a container, one board and one panel configured, an initramfs with no
shell, no login and no network, and the shell as the only program the
board runs. The board, the panel and the dev variant are directories that
combine — `just board=pi3 panel=waveshare-28dpi dev=0 pi-image`, which is
the default and the owner's device; `just dev=1 pi-image` adds a serial
login and a
console on the panel for bringing a new one up. A Pi Zero 2 W board
directory is there and untested. The image build lives in
this repository and shares nothing with any other project;
`opensigner/shells/pi/image/README.md` describes the layout, how to add a
board or a panel, how to write a card and what to expect at first boot.

The file channel is the `OSKDATA` FAT partition of the card or stick the
device booted from, and on the stick every FAT partition on every other
USB disk as well, plugged in before or after boot. A file is put there
from another computer. Nothing else is read: no other filesystem, and
none of the machine's own disks. *Sign → Read a file* lists what is
there, newest first, with each file's name — the partition's name before
it when more than one is plugged in — and the date it was written; tap
one and it is read. *Save to file* writes `signed.psbt`
to the `OSKDATA` partition when it is there and to the first other
partition otherwise, never over a file already there, so a second save is
`signed-2.psbt`. The screen names the file only once the card holds it,
and says *not saved* when there is no card to write to, so a save the
device could not make is never reported as one; power the device off
before taking the card out.

The same partition keeps the settings, as `opensigner-settings.txt`: the
network, the unit, the auto-lock and auto-wipe timers, the PIN-pad
shuffle and the camera rotation, written whenever one of them is changed
on the Settings screen and read back at the next boot. It is a few lines
of text a person can read, and *Sign → Read a file* never offers it. No
card, or a card with no such file, is a device with its defaults; keys
and anything about a key are never written anywhere.

## Releases

Releases go on GitHub with a `manifest.txt` of SHA-256 sums, signed with
the OpenSigner release key, and the artifacts it covers: the Pi card
image, the unsigned Android APK, the Linux binary and the macOS bundle.
`docs/VERIFY.md` is the page for users — import the key, check its
fingerprint, verify the manifest, check the file — and it also says what
to do with each artifact once it is verified. The release key's
fingerprint is printed there.

`just release VERSION` builds what this machine can build, copies the
artifacts under `out/release/VERSION/` and writes the manifest;
`just release-sign VERSION` signs it through the machine's
`opensigner-release-sign` command and verifies the result against
`tools/release/pubkey.asc`. The macOS bundle is built on a Mac with
`just mac-app` and added to the release directory by hand.

## Reproducible builds

No artifact ships until two independent builders have rebuilt it from the
tagged source and got the same bytes (`docs/PLANNING.md` §11.1). So every
artifact `just release` produces is built in a container with the base
image pinned by digest and every compiler pinned by version, from
`tools/build/`, with the commit date as `SOURCE_DATE_EPOCH` and host paths
remapped out of the output.

```
just linux-bin           # out/linux/opensigner-desktop, static against musl
just android-release-apk # out/android/release/, the unsigned APK and its .so
just pi-image            # the card image, Buildroot at a pinned tag
just reproduce           # the first two twice, from clean, and diffed
just pi=1 reproduce      # with the card image too; hours, so ask for it
```

`just reproduce` is this machine checking itself before a release: it
prints each artifact's SHA-256 from both runs and fails on any
difference. It does not replace the second builder. `docs/VERIFY.md` has
the steps for someone reproducing a published release.

## Repository rules

- No GitHub Actions or hosted CI. Everything builds and tests locally;
  `just reproduce` is the local double build, and reproducibility is
  confirmed by independent builders.
- Nothing under `local/` is committed; use it for private notes.

## License

MIT. See `LICENSE`.
