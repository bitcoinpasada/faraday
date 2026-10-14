# The Starter

**Status:** spec v0.2 · 2026-10-12 · decided by the owner in discussion
(`docs/DECISIONS.md` F11 and F12). Not built. v0.2 is v0.1 with the
owner's answers to a review of it: §12 records each decision, §13 lists
what changed. One decision stays open (D11).

The problem it solves: getting Faraday onto a stick or a Pi card, and a
laptop to start from it, takes a terminal, `dd`, a firmware menu and a
Secure Boot setting, and on Windows a wrong step can leave the computer
asking for a BitLocker recovery key nobody wrote down. The Starter is
one file a person downloads for their operating system and opens. It
makes a Faraday stick or Pi card, shows how to start this computer from
the stick and puts it back as it was, moves files to and from the device
by QR code, opens the desktop app for testing, and installs itself if
the person wants it in the Start menu or app menu.

People see the app as **Faraday Starter** (owner, 2026-10-12, DECISIONS
F16), and the code's crates use `starter` as well (DECISIONS F16,
2026-10-14). `docs/DESIGN.md` §5 has a reusable screen called Hub (a
Navigate screen); the two are unrelated. The desktop app is **Faraday
Desktop**; the stick is Faraday.

## 0. Rules for the implementing agent

- `CLAUDE.md` and `CLAUDE.local.md` apply in full: every build and test
  through `local/remote-run.sh`; every dimension a token; labels, not
  prose, on working screens (explanations go in Learn pages); tests for
  user-facing behaviour only; `just` once at the end of a batch; no
  `rm`; no commits; no agents.
- Batches in §11's order, except where §11 says a batch is independent;
  one review and one completion report each. A batch that needs an
  owner decision from §12 does not start until the owner has confirmed
  it.
- §6's BitLocker step is optional (F21): an encrypted drive never
  closes the firmware steps; by default it shows the recovery key, and
  suspending is the extra-safe choice. Another encryption product
  closes them until its rescue is confirmed (§6.4). The extra-safe path does not ship until §6.6's runs 1–6
  pass in full. The default path ships after a check of its page on
  Windows before the Windows release (§6.6); runs 7 and 8 are optional
  (owner, 2026-10-13: run 5 and run 1 already show the prompt and the
  key shown).
- The Windows builds are tried on a Windows computer the owner
  provides. §6.6 says which of its runs may happen on an owner's
  computer (the Home run, with the owner) and which never do (anything
  that resets firmware keys or leaves Secure Boot off).
- The implementing agent never runs a writer against a disk it has not
  been given for the purpose.
- Nothing in the Starter reaches a network. No HTTP client, no update
  check, no telemetry, no crash reporting. The one link it shows (the
  guides on the website, §7.4) is a QR code a phone opens.
- Completion report: items done, each with its tests and whether they
  pass; what was left out and why; docs changed; renders to look at. No
  restating the spec.

## 1. What ships

### 1.1 Release files

One release carries every file, all listed in one `SHA256SUMS` signed
as today (`README.md`, "Verifying a release"). The release notes and
the website lead with the two Starter downloads; the rest are under "Other
downloads".

| File | What it is |
|---|---|
| `Faraday-Starter-X.Y.Z-Windows.exe` | the Starter for Windows (x86_64) |
| `Faraday-Starter-X.Y.Z-Linux.tar.gz` | the Starter for Linux (x86_64): one file inside, `Faraday-Starter-X.Y.Z-Linux`, with its executable bit (§1.2) |
| `faraday-desktop-X.Y.Z-linux-x86_64` | the desktop app alone (today's `faraday-X.Y.Z-linux-x86_64`, renamed) |
| `faraday-desktop-X.Y.Z-windows-x86_64.exe` | the desktop app for Windows (§2) |
| `faraday-stick-X.Y.Z-x86_64-uefi.img.xz` | the stick image |
| `faraday-pi-X.Y.Z-pi3-waveshare-28dpi.img.xz` | the Pi image: the board family, then the one panel it is built for (§9); the panel part goes when §9.3's profiles arrive |
| `Faraday-Starter-X.Y.Z-macOS.zip` | the Starter for macOS, one universal file (x86_64 and arm64), `Faraday Starter.app`, ad-hoc signed (`docs/MAC.md` §6; built on a Mac, MAC batch A3c) |
| `faraday-desktop-X.Y.Z-macos.zip` | the desktop app for macOS (MAC §5, batch D) |
| `faraday-stick-X.Y.Z-arm64-apple.img.xz` | the stick image for Macs with M1 or M2 (MAC §4.1, batch A1) |
| `faraday-mac-boot-X.Y.Z.bin` | the Apple Silicon start-up file that starts any Faraday stick (MAC §4.4, batch A2) |

The macOS rows arrive with their batches. Every Starter carries the
Apple Silicon image and the start-up file template in its payload once
they exist; the macOS Starter carries the macOS desktop app.

`SHA256SUMS` lists the `.tar.gz` and the file inside it, under the
inner name, so a person can check either.

- Images are published as `.xz` only. `SHA256SUMS` lists each `.xz` and
  also the `.img` inside it, under the `.img` name, so `sha256sum -c
  --ignore-missing SHA256SUMS` checks a download, and after
  decompressing, the image; the Starter shows the `.img` line for what
  it writes.
- No dev image is published, as a file or inside the Starter.
  Developers build dev images (`just dev=1 faraday-stick-image`) and
  write them with the Starter itself: `faraday-starter --image
  out/stick/faraday-x86_64-uefi-dev.img` (or a `.img.xz`, or a
  `-boot.vfat` for keep data) puts that file in place of the payload's
  stick image, through the same screens and the same writer, with the
  Confirm page saying the image is not the Starter's own and showing
  its SHA-256. `tools/flash-stick` (the window, `write.sh`, the
  launcher) was removed in batch L, when this worked (owner, 2026-10-12).
- The README's "Verifying a release" gains: verifying on Windows
  (`Get-FileHash -Algorithm SHA256` for the sums, Gpg4win for the
  signature), and why the Windows Starter is unsigned (§10.2).
- Writing a plain image without the Starter: Raspberry Pi Imager and
  balenaEtcher take the `.xz` as it is; other tools after decompressing
  it (7-Zip on Windows, `xz -d` on Linux). The README says each in one
  line.

### 1.2 One Linux file for every distribution

The 0.1.1 desktop app needs glibc 2.43 (checked 2026-10-12: `objdump
-T` on the release binary shows `GLIBC_2.43`), which
Ubuntu 24.04 (2.39) and Debian 12 (2.36) do not have. The Starter and
the desktop app for release are built in a pinned container against
glibc 2.31 (Debian 11), so one x86_64 file runs on Ubuntu 20.04 and
later, Debian 11 and later, Fedora 32 and later, and Arch. This is the
`just faraday-linux-bin` the README said was not set up (built in batch R:
`faraday/build/linux/`; Debian 11 has reached end of life, so the
container takes its packages from `archive.debian.org`). Every other
library the apps use at run time (X11 or Wayland, xkbcommon) is loaded
from the desktop the app runs on, as today.

No `.deb`, `.rpm`, Flatpak, Snap or AppImage. The Linux Starter is
published inside a `.tar.gz` (owner, D14): a browser drops the
executable bit from a bare download, and a `.tar.gz` keeps it, so the
person extracts the archive (a double-click in every file manager, or
`tar xf`) and opens the file inside. Nobody runs `chmod`. The README
and the website show the two steps. Formats for software centres are a
later decision, if people get stuck there.

### 1.3 What is inside the Starter

The Starter carries its payload: the bytes of the release's `.xz`
files, unchanged, and a manifest naming each with its two SHA-256 sums
(of the `.xz` and of the `.img` or binary inside) and the build ID it
carries. The payload is built in by the Starter's build from `out/release/`
and is the same bytes the release publishes beside it.

| Starter | Carries |
|---|---|
| Linux | stick image, Pi image, desktop app (as `.xz`) |
| Windows | stick image, Pi image, the Windows desktop app (as `.xz`) |

Before a payload file is written to a disk or extracted to run, the Starter
app checks it against its manifest and refuses one that does not match.
It does not hash the whole payload at start: a start is instant, and
the check matters at the moment the bytes are used. That catches
corruption, not tampering: the defence against tampering is
`SHA256SUMS` and its signature over the Starter file itself, which
About (§3.7) explains.

Decompression is a pure-Rust xz decoder with an entry in `docs/deps/`
(the choice is batch R's, after reading the candidates). Faraday does
not link liblzma: xz-utils 5.6.0 and 5.6.1 carried a backdoor in 2024,
and a C decompressor is the wrong place to take that risk again. The
decoder streams: an image is decompressed as it is written, never into
a temporary file.

**The manifest and where it is built** (orchestrator, 2026-10-12).
A release is staged in two passes. The first, `just
faraday-release-stage`, stages the plain files and also writes the
payload to `out/payload/VERSION/`, outside the folder that is
uploaded, one folder per system, `linux/` and `windows/`. Each holds
the two image `.xz` files (hard links to the staged ones), that
system's desktop app compressed by the same `xz -9e -T1`
(`faraday-desktop-VERSION-linux-x86_64.xz` in `linux/`,
`faraday-desktop-VERSION-windows-x86_64.exe.xz` in `windows/`), and its
own `MANIFEST`; the two manifests have the same first line. The Windows
desktop app is taken from `out/windows/faraday.exe` beside
`out/linux/faraday`, and staging refuses it when it does not carry the
build ID as a string. It is published as
`faraday-desktop-VERSION-windows-x86_64.exe` with its line in
`SHA256SUMS`, and scanned for the build computer's paths as every
published file is. `MANIFEST` is plain text: a
first line `# Faraday Starter payload VERSION, commit <full hash>`, then
one line per file, its fields separated by one tab: the payload file's
name, its SHA-256, the name of what is inside it, that file's SHA-256,
that file's size in bytes, and the build ID it carries. The Starter's
build embeds a system's folder, named by `FARADAY_STARTER_PAYLOAD`
(`out/payload/VERSION/linux` for the Linux Starter,
`out/payload/VERSION/windows` for the Windows one); a Starter built
without it carries no payload and offers only `--image`. A Starter
offers only the desktop app for its own system: a `faraday-desktop-`
file counts only when its name ends `-linux-x86_64.xz` on Linux or
`-windows-x86_64.exe.xz` on Windows, so a Starter built against another
payload never offers another system's app. The second
pass (batch L) builds the Starters against the payload and adds them,
the `.tar.gz` and its inner file to the release folder and its
`SHA256SUMS`. The manifest's parser and the verifying, streaming
decoder are a crate of their own, `faraday/faraday-payload`, no I/O
beyond `Read` and `Write`, so its refusals are tested without a disk.

**The Pi image's build ID.** The Pi's kernel is a compressed zImage,
so its release string is not in the image's bytes. The Pi image build
writes `faraday-build.txt` to the boot partition, holding the kernel's
release string and a newline (`6.6.84-faraday-0.2.0`), so
`faraday-boot`'s `find_release` reads a Pi card's boot partition as it
reads a stick's, and the staging script and the Starter read the Pi's
build ID the same way.

Estimated size: about 40 MB on each system (11 + 12 MB images, 4 MB
desktop app, the Starter itself about 13 MB); the Windows desktop app is
a 10 MB `.exe`, about 4 MB as `.xz`, so the Windows Starter is about 40
MB too.

## 2. How it is built

- `faraday/faraday-starter`: the Starter's state, screens and decisions, no
  I/O, tested as `faraday-core` is, through actions in and commands out.
  It draws with `faraday-core::ui` and `osk-ui`, so it looks like
  Faraday and obeys the same tokens.
- `faraday/shells/starter`: the window (winit and softbuffer, as the
  desktop shell; both run on Windows), the payload, and one module per
  operating system for what differs: listing disks, writing them,
  installing, reading this computer's facts, restarting into firmware.
- **Transfer and Learn are the desktop app's own screens.** The
  Starter holds a `Faraday` in online mode, limited to Transfer and Learn,
  and draws it when one of them is open; Back returns to the Starter's
  Home. The desktop app's `--transfer` already opens it this way. Batch
  R checks that `faraday-core` can be held this way and adds what is
  missing, without changing what the desktop app does.
- **On Linux the Starter writes through udisks2 and never runs as
  root** (owner, D13). It talks to udisks2 over D-Bus through a
  pure-Rust D-Bus crate with a `docs/deps/` entry (batch L chooses it).
  Disks come from udisks2's objects: `Drive` (vendor, model, serial,
  size, `ConnectionBus`, `Removable`, `Ejectable`), `Block` and
  `Partition` (the partitions, their names and sizes), `Filesystem`
  (labels, mount points). Partitions the desktop mounted are unmounted
  through `Filesystem.Unmount`; the disk is opened with
  `Block.OpenDevice` in `rw` mode with `O_EXCL` and `O_SYNC`, and the
  descriptor udisks2 hands back is what the Starter writes, flushes,
  reads back from and `Block.Rescan`s. polkit decides whether a
  password is asked for (`org.freedesktop.udisks2.open-device`); batch
  L records what each distribution it tries asks, and the Learn page
  says a password may be asked. A distribution without udisks2 gets
  one line, "This computer cannot write a stick from here", and the
  README's plain-image route. The v0.1 path (`pkexec <its own path>
  --write …`, as `tools/flash-stick/write.sh` did) is not built unless
  batch L finds a tried desktop distribution without udisks2; if it is,
  `--write` runs before anything else in the binary is initialised.
- **On Windows the privileged writer is the same binary.** The Starter
  starts itself elevated (`ShellExecuteExW` with `runas`) with
  `--write` and the mode, the payload file, the device and the
  confirmed serial and size; the elevated copy reports progress over a
  named pipe only the person's account can open. `--write` is handled
  before anything else in the binary is initialised: no window, no
  payload beyond the one file named, nothing read from the environment.
  The writer checks the device again before it writes (§4.3) and never
  takes a path from anything but its arguments. The BitLocker calls
  (§6) go through the same elevated copy, as `--bitlocker <verb>`.
- On Windows, winit already brings in `windows-sys`; the Win32 calls
  the shell needs (disks, volumes, registry, shortcuts, elevation) go
  through it, and no other Windows crate is added.
- **The desktop app runs on Windows too** (owner, 2026-10-12, D20;
  F11 said there was none). Nothing in it is Linux by design: it draws
  through winit and softbuffer as the Starter does. Batch C made the
  changes it needs. Built: the home folder is read by one function,
  `home()`, from `HOME` and, when that is unset, `USERPROFILE`, for the
  sticks folder (`%USERPROFILE%\faraday-sticks`), Downloads
  (`%USERPROFILE%\Downloads`; a Downloads folder Windows has moved
  elsewhere is not followed) and the PDFs; Open Downloads and the
  Nakamoto Institute link run `explorer.exe` from `%SystemRoot%`;
  `osk-crypto`'s `pin-pages` pins through `VirtualLock` and
  `VirtualUnlock`, two `kernel32` functions declared by hand, under the
  same audited-unsafe rules as the Unix pair; the window is a GUI
  program (no console) whose `.exe` carries the icon, the version and an
  `asInvoker` manifest, from the same code as the Starter's
  (`faraday/build/windows/resources.rs`); and `just faraday-windows-bin
  build out/windows out/build/run faraday-desktop faraday` builds it
  into `out/windows/faraday.exe` through the same checks as the Starter
  (the default arguments build the Starter). The computer's free memory
  is read only on Linux; elsewhere the vault offers its largest unlock
  memory and checks none against this computer. The camera module the
  desktop app shares with the Starter is §8's. The Windows Starter
  carries the Windows desktop app, Try the desktop app and the install
  check box appear on Windows, and the release has
  `faraday-desktop-X.Y.Z-windows-x86_64.exe` (§1.1).
- **On macOS** (`docs/MAC.md` §6, F13): a `macos` module beside
  `linux` and `windows`. Disks from `diskutil list -plist` and
  `diskutil info -plist` (whole, physical, on USB or the built-in SD
  reader); unmounted with `diskutil unmountDisk`; opened through
  `/usr/libexec/authopen`, whose system password prompt hands back a
  descriptor for `/dev/rdiskN`, so no part of the Starter runs as
  root; one descriptor serves the write and the read-back; after it,
  `diskutil unmountDisk` again, so the volumes macOS mounts after a
  write are not left mounted and the stick stays listed. Facts from
  `sysctl hw.model` (the model identifier), `uname -m` and
  `sysctl.proc_translated`. Every program by its absolute path. Built
  on a Mac (Apple's SDK licence), as a universal binary; until there is
  one, `tools/check-macos.sh` type-checks the module for both Mac
  targets on the build machine and its parsers are tested on Linux
  against hand-written answers (HANDOFF item 73).
- Builds: Linux in the §1.2 container; Windows cross-compiled for
  `x86_64-pc-windows-gnu` in a pinned container on the build machine.
  No hosted CI (`CLAUDE.md`). Batch W tries the Windows Starter on a
  Windows computer, natively.
- **No path and nothing about the build computer in a published
  file** (owner, 2026-10-12). Every recipe that builds something
  published sources `faraday/image/path-remap.sh` or does the same
  inside its container (Rust's `--remap-path-prefix`, C's
  `-ffile-prefix-map`, for the home folder, cargo's registry and git
  checkouts, and the source tree); no code compiles in
  `CARGO_MANIFEST_DIR`, `HOME` or `USER`; a kernel names
  `buildroot@buildroot`, as Buildroot sets it; a `.tar.gz` is made
  with `--owner=0 --group=0 --numeric-owner --sort=name
  --mtime=@$SOURCE_DATE_EPOCH` and `gzip -n`, so it records no user
  name, file time or file name. The staging script checks every file
  it publishes, and every `.img`, binary and inner file inside one,
  with `strings`: it refuses one carrying the building account's home
  folder, its user name followed by `@`, the computer's host name as a
  word, `/home/<name>/`, `/Users/<name>/` or `C:\Users\<name>\`, and
  says which file and which string. The repository names none of
  these values; the script reads them on the computer it runs on.
- Strings are in Faraday's strings file,
  `faraday/faraday-strings/src/en.rs`, with a `starter_` prefix, as
  `CLAUDE.md` requires. `CLAUDE.md`'s screen rule
  gains: the Starter's screens are this document's §3, built from
  DESIGN §4's components.

## 3. Screens

Every screen uses DESIGN §4's components and §5's intents. Titles and
labels are the defaults; the build may tighten wording, not change
what a screen does.

### 3.1 Home (Navigate): a checklist, then the rows

Title: the wordmark with **Starter** beside it (F16), the second word in
the same Archivo Expanded at a lighter weight (400) and in the grey of
secondary text,, on every page that
shows the wordmark, so a person can tell the Starter from the desktop
app and the stick at a glance. A card for this computer, then the checklist of
the path from download to a running stick, then the other rows (owner,
D12: Home leads with the next step for this computer's state, not with
a menu).

**This computer** (a card): maker and model (§7.1); whether Secure
Boot is on; on Windows, whether the system drive is encrypted and, if
the Starter suspended BitLocker, that it is suspended (§6.3). On a
computer that cannot start a Faraday stick (§7.1: an ARM computer, or
one that started in legacy BIOS mode), one line says so; Make a stick
stays, for another computer.

**The checklist.** Each item carries its state, read live each time
Home is drawn, never from a file the Starter keeps (§5.1). One item is
marked as the next step, and a tap on the card's action opens it:
while the Starter's BitLocker suspension stands, Turn Secure Boot back
on and then Resume BitLocker, whichever applies first; otherwise the
first of Make a Faraday stick and Start this computer from the stick
that is not done (Start is never ticked, so it is the next step once a
stick is attached, whatever BitLocker's state; F21). The BitLocker item
is never the next step. Without a marker (Linux, or a Windows
computer the Starter never suspended) Turn Secure Boot back on is
listed and never marked. "Resumed" is known only in the session that
resumed, since the marker goes with the suspension. Items that do not
apply to this computer are not shown.

| Item | State shown | Done when |
|---|---|---|
| **Make a Faraday stick** → §3.2 | "No Faraday stick attached" or the attached stick's Faraday version (read as §4.1 reads it) | a Faraday stick is attached |
| **Back up the BitLocker recovery key** → §6.5 (Windows, only when a drive is protected, or the Starter's suspension stands, or it was resumed). Optional (F21): drawn without a number or tick circle, after the numbered items, never the next step; under it the note of §6.5 and the link **What Secure Boot and BitLocker are** (§3.1b) | "Optional", "Key shown" (the key has been shown in this session; held in memory, never in a file), "Suspended by Faraday" or "Resumed" | not ticked: nothing waits on it |
| **Start this computer from the stick** → §3.4 | "Secure Boot: on" or "off" | never ticked; it is where the firmware steps live |
| **Turn Secure Boot back on** → §3.4 at step 4 (shown while Secure Boot is off, unless §6.3's marker says it was already off when the Starter suspended BitLocker; a computer with no marker is shown it whenever Secure Boot is off, since the Starter keeps no other record) | "Secure Boot: off" | Secure Boot is on |
| **Resume BitLocker** → §6.3 (Windows, only while the Starter's suspension stands) | "Suspended by Faraday"; offered only with Secure Boot as the marker recorded it (§6.3) | protection is on |

**The card's button** (owner, 2026-10-12) names the next action for
this computer's state, and a button that restarts asks once first: a
sheet, "This restarts the computer now. Save your work first.", with
**Restart** and **Cancel**, and the QR code of this computer's guide
(§7.4). In this order, the first that applies:

| This computer | Button | After Restart |
|---|---|---|
| no Faraday stick attached | **Make a Faraday stick** | opens §3.2; no sheet |
| Windows, another encryption product found and its rescue not confirmed (§6.4) | **I have its rescue disk or password**, under the line naming it | confirms it; no sheet |
| Secure Boot on, or not known | **Restart into the BIOS** | §7.3's firmware restart |
| Secure Boot off | **Restart from the stick** | Windows: `shutdown /r /o /t 0`, Windows' *Use a device*; Linux: the stick, once (below) |

While the Starter's BitLocker suspension stands, the checklist's own
next step leads instead: Turn Secure Boot back on gives **Restart into
firmware settings**, and Resume BitLocker gives **Resume BitLocker**,
offered once Secure Boot is as the marker recorded it. Another encryption
product not yet confirmed (§6.4) closes the restart buttons: the card shows
**Restart into the BIOS** disabled, and the person confirms its rescue disk
or password on §6.5's page, from Home's BitLocker item (F21). On a computer
that cannot start a Faraday stick (§7.1) the button stays **Make a
Faraday stick**, for another computer.

**Restart from the stick on Linux** starts the attached Faraday stick
at the next start, once, so the person presses nothing at power-on.
Every Faraday stick's boot partition carries the same partition GUID
and lies at the same place (the image's genimage.cfg), so one firmware
boot entry, labelled `Faraday stick`, starts any of them. The Starter
runs `efibootmgr` through `pkexec` (polkit asks for the password; the
Starter itself stays unprivileged, D13): it creates that entry once
with `--create-only`, so it is never added to `BootOrder`, pointing at
`\EFI\BOOT\BOOTX64.EFI` on the attached stick's boot partition, reuses
it when it already exists, sets `BootNext` to it, and then runs
`systemctl reboot`. Uninstall removes the entry. Where `efibootmgr` or
`pkexec` is missing, or the password is declined, the button falls
back to **Restart into the BIOS** with one line saying why.
The stick cannot start with Secure Boot on, which is why that state
offers the firmware restart instead.

**On a Mac** (the maker begins "Apple", on macOS, Linux or Windows;
`docs/MAC.md` §6): *This computer* shows the Mac's name and model
identifier from the Starter's Mac table and no Secure Boot line. An Intel
Mac's checklist is *Make a Faraday stick*, then **Start this Mac from
the stick**, whose button opens §3.4's Mac steps: there is no restart
into firmware settings or from the stick on a Mac, and no Turn Secure
Boot back on (the steps put a T2 Mac's Startup Security back). A Mac
with 32-bit firmware, and for now every Apple Silicon Mac, gets one
line, "This Mac cannot start Faraday" or "… yet", and keeps *Make a
Faraday stick*, for another computer. Once the Apple Silicon image and
start-up file exist (MAC A1, A2), an M1 or M2 Mac's checklist is *Set
up this Mac to start Faraday*, then *Start Faraday*, with *Remove
Faraday from this Mac* in the rows once set up (MAC §4.5, §4.6).

A person who comes back with Secure Boot off and BitLocker suspended
sees Turn Secure Boot back on as the next step, and after that Resume
BitLocker; a first-time visitor sees Make a Faraday stick. Nothing in
the checklist writes to the computer except what its items do.

**The rows**, under the checklist:

- **Make a Pi card** → §3.3
- **Transfer** → the desktop app's Transfer (§2)
- **Try the desktop app** → opens it (§5.3). Its row says "Test seeds
  only". On Windows too, when the Starter carries the Windows desktop
  app (§2).
- **Install** or **Installed** → §3.5
- **About** → §3.7
- **Learn** → the Starter's Learn pages (§7)

### 3.1a Screens that can keep what they show (owner, 2026-10-12)

A TV, or any screen with its own network, can keep what Faraday
shows: smart TVs sample what is on the screen, second by second, from
every input, for automatic content recognition (the US FTC's 2017
case against Vizio), a casting receiver shows the picture on another device,
and a remote session sends it elsewhere. The Starter reads this
computer's screens and warns, by name, when one is:

- **a TV**: from its EDID (Linux: `/sys/class/drm/*/edid`, readable by
  every user; Windows: the monitor's EDID in the registry and
  `DisplayConfigGetDeviceInfo`'s output technology; macOS: its IOKit
  display entry): a monitor name naming a TV, a TV maker's PNP ID with
  the HDMI vendor block and TV timings, or the HDMI-CEC address a TV
  carries. The rule is one pure function over EDID bytes, tested
  against made-up EDIDs for a TV, a monitor and a laptop panel;
- **a wireless display** (Windows' Miracast output technology, a
  macOS AirPlay display);
- **a remote session** (Windows `SM_REMOTESESSION`; an X11 or Wayland
  session over the network where the platform says so).

**What else can capture what Faraday shows or what is typed** (owner,
the same day: think the cases through; warn ahead of time in the Starter
app, and on the stick do the best with what it knows; never warn
about what the stick makes harmless). The question for each device is
whether it can capture the screen or the keys without Faraday's help:

| Threat | The Starter, ahead of time | The stick, at start or before the first secret |
|---|---|---|
| A TV or other screen that records, on HDMI | the screen's EDID, as above | the base block of the EDID the firmware passed (below): a name naming a TV or a KVM is held at start by that name. Otherwise, before the first secret, the reminder "On a TV? Stop: a smart TV can record its screen.", unless the EDID settled it: the maker is not a TV maker, or the computer has its own panel (DMI chassis type) |
| A KVM switch or IP-KVM (captures the screen, types from afar) | its USB identity (the Linux USB-gadget vendor `1d6b` with a keyboard and a disk; strings naming KVM) and the EDID name its capture side reports | its USB identity, from what `faraday-grant` reads |
| A wireless keyboard receiver (keys travel over the air; owner, 2026-10-12: a notice, not a hold, about keyboards only, since a mouse's movements reveal next to nothing and the Starter cannot tell what a receiver has paired) | its USB identity (strings naming a receiver, dongle or wireless; the common receivers' IDs), as one line that holds nothing: "Wireless receiver attached. A wireless keyboard's typing can be picked up nearby: use a wired keyboard or the on-screen keyboard." | the same line as a reminder before the first secret (F24), and the on-screen keyboard is offered in every layout (the wide one included) |
| Running inside a virtual machine (its host sees and can change everything) | says the stick must start on the real computer, and that the Starter cannot see the host's devices | refuses to go on: no Continue anyway |
| Remote desktop, screen sharing, a wireless display | only around showing the BitLocker recovery key (§6.5) | none: the stick has no network |

Not warned about: Wi-Fi, Bluetooth and USB network adapters, for which
the stick has no driver (NET, WLAN and BT are forbidden), so they are
inert while Faraday runs; and USB video capture cards, which the stick
does bind as a camera (UVC, for the QR scanner) but which carry what
is plugged into them, not what Faraday shows, to a computer with no
network. Naming them would confuse. Not
detectable, and said once on the Learn page: a passive HDMI splitter,
a keylogger inside a cable or keyboard, a camera pointed at the
screen.

The warning is on Home's This computer card and on the Start page,
and it holds the steps that would put a secret on that screen: the
restart buttons and the BitLocker guard's Show the recovery key stay
closed while such a screen is attached, with **Check again**. Its
line names the screen ("Samsung QN65 is a TV. A TV can record what it
shows: use another screen or computer."). A screen read as a TV that
is not one can be let through once per session with **This is not a
TV**, which asks once. The Learn page "Screens that record" says why.

On the stick, Faraday has no graphics driver (DRM is forbidden; the
picture is the firmware's framebuffer). Its kernel is 6.18 longterm
with five commits from 6.19 (owner, 2026-10-14; F23): the EFI stub
reads the EDID of the screen the firmware draws on (GOP's EDID
protocols) and puts its base block in `boot_params` at 0x140, in
`/sys/kernel/boot_params/data`, which the stick also reads for Secure
Boot. The base block carries the monitor name and the maker's PNP ID
but none of the CEA-861 extension, so on the stick only a name names a
TV (or a KVM): such a screen is held at start by its name, on a laptop
too (its lid may be shut with a TV on HDMI), and Check again does not
release it, since the screen is still the one the firmware named;
Continue anyway does. A screen whose maker is not a TV maker, or a
computer's own panel, needs no reminder. A TV maker's separate screen
whose name names no TV, or a firmware that passed no EDID, gets the
reminder.

**What the stick shows, and when** (owner, 2026-10-14; F24). What the
stick found (a KVM, a screen named a TV or a KVM, a virtual machine)
is named on a sheet at start, before anything is unlocked, as above.
What it did not find but may capture a secret is a reminder: the TV
line above, and the wireless receiver's line (or the Bolt's) while one
is attached. Nothing shows a reminder at start or on Home. The first
action that would put a secret on the screen or have one typed
(loading or making a key, scanning, unlocking, showing a seed or a
key) opens one sheet, **Before a secret is shown**, listing what was
found and the reminders due. With only reminders it offers **Cancel**
and **I understand**; with something found, Check again and Continue
anyway as before, and Continue anyway goes on from the reminders too.
When that action also needs a stick pulled, the reminders are on the
sheet that asks for it, and pulling the stick goes on from them. A
reminder seen once does not come back this session; a new one (a
receiver plugged in later) comes at the next secret.

### 3.1b What Secure Boot and BitLocker are (owner, 2026-10-12)

Wherever the Starter shows Secure Boot or BitLocker (Home's card on
Linux, Home's BitLocker item on Windows, the Start page, the guard's
page) a small link, **What Secure Boot and BitLocker are**, opens the Learn page of that name
(id `secure-boot-bitlocker`): what each is and what it guards against;
why starting Faraday from a stick puts neither the computer nor its
data at risk (the stick's kernel has no driver for the computer's own
drives, SATA, NVMe or eMMC, so it cannot read or write them, and no
network); what suspending BitLocker does, in plain terms (the drive
stays encrypted; its key is left readable on the drive until resumed);
the risk that remains, and that it is short (while suspended, someone
holding the computer could read its drive; while Secure Boot is off,
the computer would start software not signed by a key its firmware
trusts, Microsoft's on most computers sold with Windows); and that
the Starter offers turning both back on after every Faraday session.

### 3.1c The boot entry on Linux

The Faraday stick entry that Restart from the stick makes (§3.1) is
explained by a link on its confirmation, **About the boot entry**
(Learn: what an entry in the firmware's boot list is, that this one
is never in the boot order and starts only when asked, and that it
names Faraday to anyone who lists the firmware's entries). Once the
computer is back from a Faraday session, Home offers **Remove the
Faraday boot entry** beside the checklist. Setting the next start
asks for the password every time, kept entry or not; keeping it only
spares creating it again, and removing it leaves no trace of Faraday
in the firmware. Uninstall removes it too.

### 3.2 Make a Faraday stick

0. **For which computer?** (Choose), only once the payload carries
   both stick images (MAC A1): "A PC or an Intel Mac" or "A Mac with
   M1 or M2". Nothing is chosen for the person, as with the disk; the
   row that fits this computer says "This computer". Keep data refuses
   a stick of the other kind (MAC §4.8).
1. **Which stick?** (Navigate) The USB disks attached now, each as a
   row for a person, not for a developer: maker and model, size, and
   what is on it. A Faraday stick says "Faraday stick" and the Faraday
   version it holds (§4.1); any other disk says the names of its
   volumes and that it holds files ("2 volumes, files on them"), or
   "Empty". The device path and serial are not on this screen; Confirm
   has them. Faraday sticks come first, then the others by size, the
   smallest first. The list follows sticks plugged in and pulled out. A
   disk the computer uses (§4.2) is shown dimmed with the reason and
   cannot be chosen. **No disk is ever chosen for the person**, even
   when there is one (owner, 2026-10-12): the one attached USB disk is
   as often a backup drive as a stick.
2. **What to write?** (Choose)
   - *New Faraday stick*: the whole stick; everything on it is lost.
   - *Replace Faraday, keep data*: only on a Faraday stick. The boot
     partition is written; the data partition, with its vaults and
     settings, is not read and not written. One line under it states
     the fact the Learn page explains (§4.4). The stick's own upgrade
     rules apply (F2, F5; owner, D15): the same build string as the
     stick already holds is refused with the reason; an older Faraday
     over a newer stick is warned about on Confirm and can still be
     written; anything else is written.
3. **Confirm** (Record) the stick (maker, model, size, device, serial),
   what is on it (its Faraday version, when it is a Faraday stick),
   what is written (the build ID and the `.img` SHA-256), the warning
   from step 2 if there is one, and the action button: *Erase and
   write* or *Write*. Then the system's own prompt where there is one
   (§2).
4. **Writing** (Progress): written of total, then *Reading back*.
5. **Done** (Result): "The stick can be pulled out."; a row **In the
   boot menu**, "something like" the entry this computer's boot menu
   will list the stick as, from the stick's own vendor and model and
   this computer's maker (Dell, ASUS, MSI, Acer and any other: `UEFI:
   <name>, Partition 1`; Lenovo consumer and Framework: `EFI USB Device
   (<name>)`; HP: `USB Hard Drive - <name>`), shown also on §3.4's
   step that chooses the stick while a Faraday stick is attached, and
   not shown when the stick reports neither; then the next step:
   **Start this computer from the stick** (§3.4). On failure the
   reason the writer gave, and nothing claims success.

### 3.3 Make a Pi card

As §3.2. Two choices come before the card once there is something to
choose: **Which Pi?** once there is more than one Pi image (§9.4), and
**Which screen?** once there is more than one panel profile (§9.3).
With one image for the 2.8 inch panel (D1), neither is shown and the
page says which Pi and screen the card is for. The card list also
offers the computer's own SD card reader (§4.2). *Replace Faraday,
keep data* is offered once the Pi image can be replaced that way
(§9.5).

### 3.4 Start this computer from the stick

The guide for this computer (§7), opened at its maker's page where
there is one and at the general page otherwise, with the steps in
order and the buttons the operating system allows. The steps shown
follow this computer's state:

1. **Restart into the BIOS** (§7.3). Its confirmation ("Save
   your work first: this restarts the computer") carries the QR code
   of this page's address on the website (§7.4), because that is the
   moment the person needs the steps on a phone; the same code is at
   the end of the page.
2. The firmware steps for this maker: turn Secure Boot off; turn Fast
   Boot off where it hides USB. When Secure Boot is already off, this
   step says so and has nothing to do.
3. **Start from the stick:** the maker's boot menu key, or Windows'
   *Use a device*.
On an Intel Mac the steps are the Mac's, from the `mac-intel` guide
(MAC §3.1, §3.2), and none of them is a button: on a T2 Mac, lower
Startup Security in macOS Recovery; then shut down and hold Option at
power-on, and choose **EFI Boot** (after the firmware password, if
one is set); on a T2 Mac, afterwards, Startup Security back to Full.
A Mac the table does not know, and not ARM, gets the T2 steps marked
as for a Mac with the T2 chip only.

4. **Afterwards:** turn Secure Boot back on before starting Windows.
   Not shown when §6.3's marker says Secure Boot was already off when
   the Starter suspended BitLocker: that computer's owner chose to
   have it off and is not told to turn it on. With no marker the Starter
   app keeps no record and shows the step whenever Secure Boot is off.
   On Windows, Resume BitLocker follows (§6.3).

On Windows, after the steps, the same optional BitLocker row as Home's
(§3.1; F21), unnumbered, with its state and a secondary button **Back
up the recovery key** that opens §6.5's page, and without the note. The
steps are never held by BitLocker: they are offered whatever its state.
Another encryption product not yet confirmed (§6.4) still closes the
restart button.

### 3.5 Install and uninstall

**Install** (Choose): one check box, *Also install the desktop app,
for testing with test seeds only*, unchecked (on Linux and on Windows,
§2, §5.2); then **Install**. Done shows where it went. When the running copy
is newer than the installed one, the row reads **Update the installed
Faraday**.

**Installed** opens a Record of what is installed and where, with
**Uninstall**, which asks once. §5 says what each does.

### 3.6 Transfer and Learn

The desktop app's screens (§2). On Windows, Transfer offers Send and
Receive; Receive reads the camera through Media Foundation (§8).

### 3.7 About

Version; each payload file with its build ID and SHA-256; the release
key's fingerprint as the README prints it; how to check the Starter
file itself with `SHA256SUMS` and its signature; on Windows, why the
file is unsigned (§10.2).

## 4. Writing a stick or a card

### 4.1 What the writer does

As `tools/flash-stick/write.sh` did (removed in batch L, D21; this
section is the reference): refuse unless
the device is still the stick that was confirmed (serial and size);
unmount what the desktop mounted from it; write; flush; read the whole
write back and compare; ask the system to read the partition table
again. *Keep data* writes the boot partition at its offset after the
same partition-table checks as `faraday/image/stick-boot.sh`, and
checks the partition table is unchanged afterwards. The source is
decompressed from the payload while it is written, and the SHA-256 of
the decompressed stream must equal the manifest's before the writer
reports success.

**What the Starter reads from a Faraday stick.** Only the boot
partition, and only to learn which Faraday it holds: the build string
(F5) the stick's own Upgrade a Faraday stick reads from another stick
(`faraday-core/src/upgrade.rs`): the kernel release string in
`EFI/BOOT/BOOTX64.EFI` on a stick, or `faraday-build.txt` on a Pi
card (§1.3). On Linux it is read from the desktop's own mount of
OSKBOOT; when OSKBOOT is not mounted, the Starter has udisks2 mount it
read-only without asking for a password (`auth.no_user_interaction`),
reads it and unmounts it (batch L: opening the raw partition needs
`OpenDevice`, whose polkit prompt would ask for a password just to list
disks, and `faraday-fat` does not enter subdirectories; it only checks
the boot sector's OSKBOOT label). A stick whose version cannot be read
shows as one made before versions were recorded. That version is shown
on Which stick and Confirm and drives the keep-data rules (§3.2).
OSKDATA is never mounted, opened, read or written (§4.4).

### 4.2 Which disks are offered

- A whole disk on USB, removable or hot-pluggable (Linux: udisks2's
  `Drive.ConnectionBus` = `usb`, `Removable` or `Ejectable`); or, for
  a Pi card, an SD card in the computer's own reader (Linux `mmcblk*`;
  Windows bus types SD and MMC).
- Never a disk with a partition the computer uses: on Linux mounted
  anywhere but `/run/media`, `/media` or `/mnt` (udisks2's
  `Filesystem.MountPoints`), or swap (`Swapspace.Active`); on Windows
  the disk holding the Windows, boot or page-file volume.
- Never a disk smaller than the image.
- A disk is dimmed, not hidden, when refused, with the reason in one
  line, so the person sees why their disk is not offered.

### 4.3 Windows specifics

List disks through `IOCTL_STORAGE_QUERY_PROPERTY` (bus type, vendor,
product, serial) and `IOCTL_DISK_GET_DRIVE_GEOMETRY_EX` (size); find
the system disk through its volumes' disk extents. Before writing, lock
and dismount every volume on the disk and hold the locks for the whole
write; write `\\.\PhysicalDriveN` unbuffered, write-through, in
sector-aligned buffers; then `IOCTL_DISK_UPDATE_PROPERTIES`. Batch W
records what Windows does with the stick after the write (drive letters
given to OSKBOOT and OSKDATA, folders it writes such as `System Volume
Information`) and reports it; whether to stop Windows giving OSKDATA a
letter (the GPT "no drive letter" attribute) is an owner decision
(§12, D11).

### 4.4 Keep data

Offered to everyone on a Faraday stick (F11). The writer never reads
or writes OSKDATA. The fact the person should know is that plugging the
stick into this computer is what exposes it: the computer can read and
write the data partition whatever the Starter does, and Windows mounts
it with a drive letter. The vaults there are encrypted; the settings
file is not. The Learn page "Replacing Faraday on a stick" says this
and points to Settings → Upgrade a Faraday stick, which upgrades a
stick from another stick without an online computer.

## 5. Install, run without installing, uninstall

### 5.1 Running without installing

Opening the downloaded file runs the Starter. It writes nothing to the
computer except what the person asks for (a stick, a card, a received
file, the desktop app's cache under §5.3) and, on Windows, the one
marker §6.3 describes when it suspends BitLocker. Home's checklist
(§3.1) reads its state live; there is no settings or state file.

### 5.2 Installing

Per user, never needing administrator rights.

| | Linux | Windows |
|---|---|---|
| The Starter | `~/.local/bin/faraday-starter` (the XDG place for a user's executables, and on PATH on most desktops) | `%LOCALAPPDATA%\Programs\Faraday Starter\Faraday Starter.exe` |
| The desktop app, if ticked | `~/.local/bin/faraday-desktop` | `%LOCALAPPDATA%\Programs\Faraday Starter\Faraday Desktop.exe`, beside the Starter |
| Menu | `~/.local/share/applications/app.faradayoffline.FaradayStarter.desktop` "Faraday Starter", and `…FaradayDesktop.desktop` "Faraday Desktop" (its description: test seeds only) | Start menu shortcuts `Faraday Starter` and, with the desktop app, `Faraday Desktop` |
| Icon | `~/.local/share/icons/hicolor/256x256/apps/app.faradayoffline.FaradayStarter.png` | in the `.exe` |
| Uninstall entry | — | `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Faraday Starter`: name "Faraday Starter", version, publisher "Faraday", icon, size, `UninstallString` = `"…\Faraday Starter.exe" --uninstall` |
| Logon reminder | — | only while the Starter's BitLocker suspension stands (§6.3) |

On macOS: the Starter at `~/Applications/Faraday Starter.app`, a copy
of the running bundle made with `ditto` (which keeps its signature),
and with the desktop app (from MAC batch D)
`~/Applications/Faraday Desktop.app`; per user, no administrator. The
bundle is the menu entry and carries the icon. A bundle of that name
whose identifier is not `app.faradayoffline.FaradayStarter` is in the
way, as on Linux. The cache (§5.3) is `~/Library/Caches/faraday/X.Y.Z/`.
Uninstall removes exactly those.

The menu file names are reverse-DNS, so they never collide with a
developer's own `faraday.desktop`. The two executables' names are plain, so
install replaces, and uninstall removes, `faraday` and
`faraday-desktop` only when the file there answers `--version` as a
Faraday Starter or desktop app (orchestrator, 2026-10-12); a file of
the same name that does not is left alone, and Install says it is in
the way.

### 5.3 The desktop app from the Starter

Try the desktop app runs the installed desktop app if there is one;
otherwise the Starter decompresses it from its payload into
`$XDG_CACHE_HOME/faraday/X.Y.Z/` (default `~/.cache/faraday/X.Y.Z/`;
on Windows `%LOCALAPPDATA%\faraday\cache\X.Y.Z\Faraday Desktop.exe`),
checks its SHA-256 against the manifest, and runs it. Every start
checks the cached copy again and writes it again when the sum is not
the manifest's. On Windows the installed copy is the decompressed
payload file, checked against its manifest line, written beside its
place and renamed in; the Start menu shortcut `Faraday Desktop` is
beside the Starter's, and Installed lists the desktop app and its
shortcut when they are there. The app is started on its own, so
closing the Starter does not end it; a start that fails within 0.4
seconds is reported. The desktop app opens on its own; it is
the same program as the plain release file, and can also be opened
without the Starter.

### 5.4 Uninstalling

Removes exactly the files and entries §5.2 lists for this computer, and
the §5.3 cache. It never touches anything the person made: received
files, the desktop app's `~/faraday-sticks` and settings. On Windows
the running copy cannot delete itself; the uninstaller finishes by
starting a short command that removes the folder once the Starter has
exited.

On Windows, while the Starter's BitLocker suspension stands (§6.3),
Uninstall first offers **Resume BitLocker** when Secure Boot is on,
and when it is off says in one line that protection stays suspended
until it is resumed, and where Windows itself resumes it (Control
Panel → BitLocker Drive Encryption → Resume protection). Either way
the logon reminder and the marker are removed with the rest; the
suspension itself is never undone silently and never left without the
person having read that line.

## 6. The BitLocker guard (Windows)

**The requirement (owner, 2026-10-14, F21, after F17 of 2026-10-13):**
nobody who follows the Starter's steps in order is asked for the
BitLocker recovery key, and anybody who wants a backup of it can have
one. BitLocker checks Secure Boot only when Windows starts, so a person
who turns Secure Boot off, starts the stick and turns Secure Boot back
on before Windows starts again is never asked. The BitLocker step is
therefore optional and never closes the firmware steps: by default it
shows the key, for a slip (Windows started while Secure Boot is off);
suspending BitLocker, which also covers the slips and the firmware's
keys changing, is the extra-safe choice. This section is how, and §6.6
is how it is proved. (2026-10-12's requirement made suspending the only
way to the firmware steps; F17 made the key in hand the default, still
before them.)

### 6.1 Why it happens

BitLocker, and Windows 11's Device encryption, which turns it on
without asking on many laptops, seals the drive's key to the TPM's
measurement of how the computer started, which on most laptops
includes the Secure Boot state (PCR 7). Turning Secure Boot off and
starting Windows, or turning it back on with different keys than before
(a firmware's "restore factory keys"), and Windows asks for the 48-digit
recovery key. Clearing the TPM does the same. Starting Faraday from a
stick does not, on its own: Windows is not running then.

### 6.2 The guard

The Starter finds out whether any Windows volume on this computer is
protected, and if one is, offers the BitLocker step, optional (F21),
at one of two levels:

- **The default: a backup of the recovery key.** The page says where
  the key is also kept (a Microsoft account, for Device encryption
  under one) and offers **Show the recovery key** as in part 1 below,
  with its QR code, its photo line and Save, but with no groups typed
  back; **Done** hides it and returns to Home, which reads "Key shown"
  for the rest of the session (memory only). Nothing on the computer
  changes, and nothing waits on it.
- **Extra safe: suspend BitLocker.** Both parts below, in this order
  (owner, D16: the key first, so that a person who stops midway has
  changed nothing on the computer). Unchanged from 2026-10-12.

1. **The recovery key is in hand.** The Starter reads the volume's
   recovery password (`GetKeyProtectorNumericalPassword`) and shows it,
   the way Windows' own "Back up your recovery key" does. A volume with
   no recovery password gets one added first
   (`ProtectKeyWithNumericalPassword`), and that is the one shown. The
   person keeps it one of these ways (owner, D18):
   - writes it down or prints it;
   - scans the QR code beside it with a phone: the code holds the key
     as plain text (eight groups of six digits, with a line naming the
     computer and the drive), which the phone's camera shows and offers
     to copy, so it can be pasted into the phone's notes. Under the
     code one line: "Or take a photo of this screen.";
   - **Save**, to a file, offered only when a drive other than the
     protected ones is attached and it is not a Faraday stick;
     otherwise the button is not shown.
   Then the person types two of its eight groups back, chosen by the
   Starter, **with the key hidden** (DESIGN §4.10's secret, concealed)
   from the moment the groups are asked for; a copy still on screen
   would prove nothing about the written one. The key is never written
   anywhere by the Starter unless the person chooses Save, and never
   to the Faraday stick.
2. **Protection is suspended.** The Starter suspends BitLocker on every
   protected operating-system volume, with no restart count, so it
   stays suspended until resumed (`Win32_EncryptableVolume.
   DisableKeyProtectors` with a count of 0; batch W confirms against
   Microsoft's documentation and §6.6 that 0 means until resumed, as
   `Suspend-BitLocker -RebootCount 0` does). While suspended, Windows
   starts without the key whatever the firmware and the TPM report:
   Secure Boot off, Secure Boot on with other keys, a cleared TPM. At
   this moment the Starter writes its marker (§6.3).

On the extra-safe path, suspending alone already prevents the prompt;
the key in hand covers what the guard cannot see (a firmware update, a repair shop, a
protector resumed by someone else).

All of this is done by the elevated writer (§2) through WMI's
`root\CIMV2\Security\MicrosoftVolumeEncryption`, which answers in
numbers in every Windows language and exists on Home editions with
Device encryption. Nothing parses `manage-bde`'s localised text.

Knowing whether the guard is needed does not need elevation: Home
reads the volume's BitLocker protection state through the shell
property `System.Volume.BitLockerProtection`. If that cannot be read,
the Starter treats the drive as protected. Only the elevated WMI
reading concludes that the drive is not encrypted, and only from
`ProtectionStatus` 0 with `ConversionStatus` 0 (fully decrypted); a
drive WMI does not list, and every shell value whose meaning Microsoft
does not document (2 among them, until batch W's first run on a
Windows computer records what each state reads), counts as not known
(orchestrator, 2026-10-12).

If the person declines the elevation prompt, the page says why in one
line and offers the step again. The firmware steps stay offered on
either path (F21).

### 6.3 Resuming, only when it is safe

Resuming is the person's act, through the Starter, never a task that
runs at boot (owner, D17: v0.1's SYSTEM startup task was boot-time
persistence by an unsigned program, and its action would have had to
live somewhere no user can write; a reminder costs one click and one
elevation prompt and keeps the Starter out of the boot path).

- **The marker.** When it suspends, the Starter writes one small file,
  `%LOCALAPPDATA%\faraday\bitlocker-suspended`, holding the volumes it
  suspended and whether Secure Boot was on at that moment (§7.1's
  registry value). The marker is how the Starter knows the suspension
  is its own and not an administrator's, and how §3.4 step 4 and
  Home's Turn Secure Boot back on know Secure Boot was on before. It
  is deleted when protection is resumed through the Starter, and by
  Uninstall.
- **Resume BitLocker** is never offered in the Windows session that
  suspended (the marker records that boot; a marker without it counts
  as from an earlier boot), so nobody undoes the step they have just
  taken; it shows "Needs Windows restarted first" instead.
  After that it is offered, on Home's checklist and on the
  guard's page, only while the marker exists and Secure Boot is now as
  the marker recorded it: on, for a computer that had it on when the
  Starter suspended, which is nearly every one; off, for one that
  already had it off (orchestrator, 2026-10-12, after batch G: with
  "on" alone such a computer would stay suspended for good, and
  Windows Home has no Control Panel route to resume it). It resumes protection on the marker's volumes
  (`EnableKeyProtectors`, through the elevated copy, §2) and deletes
  the marker. Resuming seals the key to how this start was measured,
  so a start with Secure Boot on as before, or on with different keys
  (a firmware that cleared its keys and needed "restore factory
  keys"), resumes cleanly. While Secure Boot is off on a computer
  whose marker recorded it on, the button is not offered and the row
  says why; protection stays suspended, which
  costs protection, never access.
- **The logon reminder.** An installed Starter (§5.2) adds, at the
  moment it suspends, one per-user Run entry
  (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Faraday Starter` =
  `"…\Faraday Starter.exe" --remind`). At logon, `--remind` reads the marker
  and the Secure Boot state and does one of three things: no marker,
  or protection already on, it removes the Run entry and exits with no
  window; marker and Secure Boot not as the marker recorded it, it
  exits with no window; marker and Secure Boot as recorded, it opens
  the Starter on the guard's page with Resume BitLocker as the one
  thing to do. A Starter run without
  installing has no logon entry: the guard's Done page says "Open
  Faraday again after Secure Boot is back on, to resume BitLocker",
  and Home shows it as the next step whenever the Starter is opened.
  The Run entry points at the installed copy only, never at a file in
  Downloads.

### 6.4 Where the guard cannot reach

- A person who changes firmware settings without the Starter. Every
  other place the steps appear (the website's guides, §7.4–§7.5) says,
  before any firmware step, that a backup of the recovery key is
  optional, and to turn Secure Boot back on before Windows starts.
- Another encryption product (VeraCrypt system encryption, a vendor's
  own). The Starter says it found an encrypted drive it does not
  manage, and does not offer the firmware steps until the person
  confirms they have that product's rescue disk or password. VeraCrypt
  is detected by its driver; anything else falls under "drive not
  readable as BitLocker".
- Linux with a disk unlocked by the TPM (`systemd-cryptenroll
  --tpm2-pcrs=7`): the Linux guide says the passphrase will be asked
  for, and to have it. The Starter detects a `tpm2-device` in
  `/etc/crypttab` and shows that line first.
- A person without an administrator account. The elevation prompt
  asks for an administrator's password they do not have. The guard's
  page says so in one line ("An administrator of this computer has to
  do this step") and offers it again; the firmware steps stay hidden.
- A computer whose BitLocker an employer manages. The suspension may
  be refused (`DisableKeyProtectors` fails) or undone by policy at the
  next start. On a refusal the page says the computer's BitLocker is
  managed and that its administrator has to suspend it; the firmware
  steps stay hidden. A suspension undone later is caught by the
  checklist's live state ("Protected" again, with the marker still
  present): the page says so and offers the step again.

### 6.5 What the person sees

One page, optional (F21), opened from Home's BitLocker item or the Start
page's **Back up the recovery key**, when a drive is protected, titled
**BitLocker recovery key**, in this order: the note ("To start
Faraday, Secure Boot goes off, then back on before Windows starts.
Windows then starts as usual and does not ask for the BitLocker
recovery key. A backup of the key is optional: it gets you into
Windows if it ever starts while Secure Boot is still off."), the one
explanation on a working screen, an exception the owner made so the step
reads as optional, on Home and on this page and nowhere else; the state
("Drive C: is encrypted with BitLocker"); a value row, **Also kept in**:
"Microsoft account: aka.ms/myrecoverykey" (where an employer's computer
keeps it instead is said in the Learn page, not here); **Show the
recovery key** as the primary button (the elevation prompt), then the
key, its QR code, "Or take a photo of this screen." and Save where
§6.2 allows it, with no groups typed back, and **Done**, which hides the
key and returns to Home, where the item reads "Key shown" (held in
memory, never in a file; set when the key arrives on screen); and
**Extra safe: suspend BitLocker**, a secondary button. Nothing waits on
any of it: **I have my recovery key** is gone.

Extra safe turns the same page into the guard as 2026-10-12 built it,
until the page is left: **Show the recovery key**; the key, its QR
code, the photo line and Save; **I have kept it**, after which the key
is hidden and the two groups are asked for; **Suspend BitLocker**;
then Done: "Windows starts without the key until BitLocker is
resumed. Resume it here once Secure Boot is back on." and, for a
Starter not installed, "Open Faraday again to do that." The page is the
same one Resume BitLocker opens later (§6.3). Learn: "BitLocker and
starting from a stick".

### 6.6 Acceptance, before batch W ships

Runs 1 to 6 test the extra-safe path (§6.2, suspending); runs 7 and 8
test the default path, the key shown and nothing suspended (F17, F21). Runs 7 and 8 are
optional (owner, 2026-10-13): Windows checks Secure Boot only when it
starts, which run 5 shows, and run 1 shows the key Starter shows is
BitLocker's own; the default path changes nothing on the computer. In
their place, before the Windows release, the default page is looked
at on Windows: Show the recovery key without the type-back, and Done
returning to Home.

Two places, because Windows Home cannot be tested in a virtual machine
(owner, D19): Home has no BitLocker switch, only Device encryption,
which Windows turns on by itself and only on hardware that passes its
checks, which a virtual machine fails.

**Pro, in a virtual machine first:** Windows 11 Pro with a virtual TPM
and Secure Boot on (QEMU with OVMF's Microsoft-key variables and
`swtpm`, or Hyper-V where there is one), with BitLocker turned on on
the system drive:

1. Through the Starter: guard, then Secure Boot off in the firmware,
   start the Faraday stick image, power off, Secure Boot on, start
   Windows. **No recovery prompt**; Resume BitLocker offered at logon
   (the reminder, with the Starter installed) and on Home; resuming
   works; the marker and the Run entry are gone.
2. As 1, but start Windows with Secure Boot still off. **No recovery
   prompt**; protection still suspended; the reminder shows nothing;
   Home says so and does not offer Resume; turning Secure Boot on and
   starting again offers it.
3. As 1, with the firmware's keys reset to default before turning
   Secure Boot on. **No recovery prompt.**
4. The elevation declined: the page says why and offers the step
   again; the firmware steps stay offered (F21).
5. Without the Starter, Secure Boot off and Windows started: the
   recovery prompt appears. This shows the virtual machine reproduces
   the problem, so 1–3 prove something.
6. Uninstall while suspended, with Secure Boot on and with it off
   (§5.4).
7. The default path: the key shown only, nothing suspended; Secure Boot off in the firmware, the Faraday stick chosen
   from the boot menu on that same restart, power off, Secure Boot on,
   start Windows. **No recovery prompt**; protection was on throughout.
8. As 7, but Windows started while Secure Boot is off. **The recovery
   prompt appears**, and the key Faraday Starter showed unlocks it.
   This shows the default path's slip is the one the guides warn of,
   and that the backed-up key is enough to recover from it.

**Home, on a computer of the owner's, with the owner, after the Pro
runs pass:** a Windows 11 Home computer with Device encryption on,
provided by the owner. The owner runs it, following the Starter on the
screen, with the agent's steps written beforehand. Before the run the
owner checks that the drive's recovery key is in their Microsoft
account, where Home puts it, so the worst case is typing it once. Runs
1 and 2 only. Never 3 (the firmware's keys reset) and never 5 (the
deliberate recovery prompt) there: those stay in the virtual machine.
A Home virtual machine on 24H2 or later, signed in with a Microsoft
account, may turn Device encryption on by itself; if batch W gets one,
Home's runs 1 to 5 may happen there instead, and the owner's run
becomes a check of run 1 alone.

Each run is recorded in the completion report: the steps, and a
screenshot of Windows' first screen after each start (a photo of the
screen for the owner's runs).

## 7. Starting a computer from the stick

### 7.1 What the Starter reads about this computer

Read locally, shown only on this computer, sent nowhere.

| | Windows | Linux |
|---|---|---|
| Maker, model, family | `HKLM\HARDWARE\DESCRIPTION\System\BIOS` (`SystemManufacturer`, `SystemProductName`, `SystemFamily`) | `/sys/class/dmi/id/sys_vendor`, `product_name`, `product_family` |
| Started in UEFI mode | `GetFirmwareType` | `/sys/firmware/efi` exists |
| Secure Boot | `HKLM\SYSTEM\CurrentControlSet\Control\SecureBoot\State\UEFISecureBootEnabled` | the `SecureBoot-8be4df61-…` variable under `/sys/firmware/efi/efivars`, readable by every user |
| ARM | `IsWow64Process2`'s native machine | `uname -m` |
| Encryption | §6.2 | §6.4 |

An ARM computer cannot start the x86_64 stick (Windows on Snapdragon
laptops); one started in legacy BIOS mode needs UEFI turned on first,
which its maker's page covers.

On macOS: maker "Apple Inc.", the model identifier from `sysctl
hw.model` (the same string DMI's `product_name` gives on an Intel Mac),
ARM from `uname -m` or `sysctl.proc_translated` (an x86_64 Starter
under Rosetta), a virtual machine from `kern.hv_vmm_present`. What
kind of Mac it is (T2, 32-bit firmware, the Apple chip) comes from the
Starter's table of model identifiers (MAC §7), on any system running on a
Mac.

### 7.2 The guides are Learn pages

Each guide is a Learn page in `docs/learn/faraday/boot/`, round-tripped
into `osk-learn` as every Learn page is. The Starter and the Companion
carry them (`faraday-guides`), and the website publishes the same text
(§7.4); the desktop app and the stick carry only the address of the
guide for this computer, not its text. Pages:

- *Starting a Windows computer from the stick*: BitLocker first (§6),
  Fast Startup (Windows' Shut down is not a full shut down; restart, or
  turn Fast Startup off), the firmware settings through Settings →
  System → Recovery → Advanced startup, *Use a device*, then Secure
  Boot back on.
- *Starting a Linux computer from the stick*, with §6.4's TPM line.
- *BitLocker and starting from a stick* (§6).
- One page per maker family, published several at a time (§7.6,
  D6). Each has the same sections: Keys; Turn Secure Boot off; Turn Fast Boot off
  (where it hides USB); Start from the stick; Turn Secure Boot back on
  (including restoring factory keys where turning it off cleared
  them); Known quirks. Each ends with a line of where it comes from:
  the maker's support page and the date it was read. No page names
  the model it was tried on: the computers tried are the owner's, and
  their make and model are personal information (the owner's rule;
  MAC.md M6 decided the same for Macs).
- *When the stick does not start*: not in the boot menu (Fast Boot,
  USB boot off, a USB-C port that does not boot, CSM), a black screen,
  a firmware password set by an employer.

### 7.3 Buttons that remove steps

- **Restart into the BIOS**: Windows `shutdown /r /fw /t 0`
  (elevated); Linux `systemctl reboot --firmware-setup`. Both work on
  any maker in UEFI mode, so nobody has to catch a key at power-on to
  reach Secure Boot.
- **Restart to choose a device** (Windows): `shutdown /r /o /t 0`,
  Windows' advanced startup, whose *Use a device* starts the stick on
  many computers without the boot menu key. Offered once Secure Boot is
  off.

Each button asks once ("This restarts the computer now. Save your work first.").

### 7.4 On a phone

The website publishes each guide at
`https://faradayoffline.app/start/<page>`; the Starter shows that
address as a QR code beside the guide. The website's build is the
website repository's work, from these Learn pages (its own HANDOFF).
The address is the only one the Starter contains.

### 7.5 The stick reminds

At power-off, a stick that started with Secure Boot off says so, with
the maker's key for the firmware settings when the maker has a page:
"Secure Boot is off. Turn it back on before starting Windows." The
stick reads the Secure Boot state from `/sys/kernel/boot_params/data`
(byte `0x1ec`: 2 off, 3 on; readable by every user on x86 and needing
no efivarfs, which the stick's kernel does not build) and the maker
from `/sys/class/dmi/id/sys_vendor`. Checked on a laptop 2026-10-12:
byte `0x1ec` = 2 with the SecureBoot variable 0.

Beside that line the power-off sheet shows the QR code of the guide
for this computer (owner, 2026-10-12): one code per computer, the same
the Starter shows on its Start page and its restart confirmation
(§3.4, §7.4), so a person scans one code for the whole procedure. The
guide holds both halves: turning Secure Boot off and turning it back
on. It is the maker's page when one matches the firmware's maker,
family and model (`learn::maker`, as the Starter chooses it, read on
the stick from `/sys/class/dmi/id/`), the general Windows page
otherwise: `https://faradayoffline.app/start/<page>`, with no section
anchor. Only on the stick, only when it started with Secure Boot off;
the code is an address, so the stick reaches nothing.

### 7.6 Maker families and where to start

The first release carries the general Windows and Linux pages, whose
firmware route (§7.3) needs no key on any maker. Maker pages are
published several families at a time, never one alone, each from the
maker's own documentation (owner, D6 as changed by F14: a page for
one family alone would tell which maker the owner tried). A family
tried on a computer is checked there before its group is published,
and nothing published says which families were tried. The keys here
are starting points from common knowledge, **every one to be checked
against the maker's own documentation** before its page
is written.

| Family | Boot menu | Firmware settings | To check |
|---|---|---|---|
| Dell (Inspiron, XPS, Latitude, Precision) | F12 | F2 | |
| HP consumer (Victus, Pavilion, OMEN, Envy) | Esc, then F9 | Esc, then F10 | |
| HP business (EliteBook, ProBook, ZBook) | Esc, then F9 | Esc, then F10 | Sure Start |
| Lenovo ThinkPad | F12 (or Enter, then F12) | F1 | |
| Lenovo IdeaPad, Yoga, Legion | F12 (Fn+F12), or the Novo button | F2 (Fn+F2) | |
| ASUS (VivoBook, ZenBook, ROG, TUF) | Esc | F2 | Secure Boot "Other OS" |
| Acer (Aspire, Swift, Nitro, Predator) | F12, once enabled in the firmware | F2 | Secure Boot is greyed out until a supervisor password is set; remove it afterwards |
| MSI | F11 | Del | |
| Microsoft Surface | hold Volume Down while pressing power | hold Volume Up while pressing power | no custom keys |
| Framework | F12 | F2 | |
| Samsung Galaxy Book | F10 | F2 | |
| Gigabyte, Aorus | F12 | F2 | |
| Razer | F12 | F1 or Del | |
| Huawei, Honor | F12 | F2 | |
| Dynabook, Toshiba | F12 | F2 | |

A maker without a page, or not read, gets the general Windows or Linux
page, whose firmware route (§7.3) needs no key. Which makers were
tried is not written down anywhere published.

### 7.7 Keeping Secure Boot on (later)

Faraday can make its own Secure Boot keys and sign `BOOTX64.EFI`. On a
firmware that lets a db key be added beside Microsoft's, both Windows
and Faraday start with Secure Boot on and there is nothing to turn back
on. A later batch writes that guide, per maker, never suggesting the
Microsoft keys be removed (on some laptops that stops the graphics
firmware loading). Surface has no custom keys. Not in the first
release (§12, D9).

Enrolling a key does not escape §6: the db's contents are part of the
TPM's PCR 7 measurement, so adding a key changes it exactly as turning
Secure Boot off does, and Windows asks for the recovery key unless
BitLocker is suspended. The guard runs once before the enrolment, and
Resume after it. What the guide removes is the second firmware visit
every time, not the first.

### 7.8 Firmware sketches

The maker pages carry sketches of the firmware's screens, drawn by
Faraday's own code to look like a firmware setup screen: square
corners, firmware colours and a text-mode font (F18). Never a maker's
screenshot. Each is labelled a mock-up.

- **Data.** `faraday_learn::sketch` holds a `Sketch` per screen: its
  id, its palette (black, blue, grey, white, red or gold: the screen
  colour nearest the family's own as the makers' pictures show it,
  black where they show black or nothing), a title line, the tabs or side list with the open
  one, a heading or path, a message, the rows near the one to find
  (label and value; a group heading; the row to find, as it is or
  changed from its value to another), a dialog over the screen (title,
  message, choices, buttons and the one to choose), and the key hints
  along the bottom. The words are the firmware's own English, read
  from the makers' simulators, manuals and support articles on
  2026-10-13; they are data beside the pages, not Faraday's interface
  text, so they are not in `faraday-strings`. One sketch stands for a
  family of menus; none names a model or a maker inside the frame, a
  stick is "USB Flash Drive" and a confirm code is an example.
- **Marker.** A page asks for a sketch with a paragraph of its own,
  `::sketch <id>`, after the step it shows. `faraday_learn::sketch_of`
  finds it; a test fails on a marker that names no sketch.
- **Drawing.** `faraday_ui::sketch(ui, &sketch, x, y, w)` draws it and
  returns its height. It looks like a firmware screen, not like
  Faraday, whatever the theme: square corners everywhere, no shadow;
  every word in Terminus, a text-mode face (SIL OFL 1.1,
  `tools/fonts/`), at a size that grows with the frame's width; the
  colours of a PC's text mode (`faraday_ui::pal::vga`), set per
  palette in one table in `faraday-ui/src/firmware.rs`. A title bar
  with the title centred; the open tab or side item as a solid bar;
  values in one column; the row to find on the highlight bar, a change
  shown as `[Enabled] → [Disabled]` with the new value in the value
  colour; a dialog as a box with a double line, its title set into the
  top edge and its buttons as `[ Yes ]  [ No ]` with the one to choose
  on the highlight bar; the key hints under a rule at the foot, the key
  in the key colour. Under the frame, in Faraday's own secondary text,
  one line from `faraday-strings` (`ui_sketch_mock_up`): "Mock-up: the
  real screen may look different." Narrow, the tabs or side list fold
  to the open one, the row to find puts its value under its label,
  a long label or key hint wraps, and other words are cut rather than
  run over.
- **Where.** Faraday Starter draws them in its Learn sheet (the held
  desktop app's, `Faraday::held`), and Faraday Companion in its Learn.
  The stick, the Pi and Faraday Desktop leave the marker paragraph out
  and never print it. The website, built from the same pages in its
  own repository, has to leave it out as well.
- **The set.** By the layouts the makers share: the tabbed setup
  (Acer, HP consumer, HP business, ASUS, MSI), the side-list setup
  (ThinkPad, IdeaPad, Dell, Surface), a list page (Framework), the
  save dialog (ThinkPad, IdeaPad, HP consumer, ASUS notebooks and
  desktops), the boot menu as a box (ASUS, MSI) or a full-screen list
  (IdeaPad, HP consumer, Framework, Dell), and HP's confirm code
  (consumer and business). Steps with no screen read are left without
  one: the ThinkPad and HP business boot menus, the save step on Dell,
  HP business, MSI and Framework (each has the screen the step starts
  from), HP desktops, and Acer's boot menu.

## 8. Transfer

Transfer is the desktop app's (`faraday-core/src/transfer.rs`): Send
shows a file as animated codes for the device's Scan; Receive reads the
device's codes with the camera and saves each whole file into the
Downloads folder. On Linux the camera is `opensigner-v4l2`, as the
desktop app's. On Windows it is `faraday-mediafoundation`
(`faraday/shells/mediafoundation`), built as `opensigner-avfoundation`
is: one small C file, `src/camera.c`, behind a two-function C ABI, that
lists Media Foundation's video capture devices and reads the first, or
the one chosen on the scan sheet, as NV12 near 640 x 480 on a thread of
its own; the calls declared in the crate, every `unsafe` block with its
`SAFETY` comment, `docs/deps/cc.md` for the build. Media Foundation's
DLLs are loaded when the camera is used, not imported, so the Starter
still starts on Windows' N editions, which lack them; there, and when
Windows' camera privacy setting refuses the program, Receive says the
camera is unavailable and offers the file instead. The Starter and the
Windows desktop app use it alike (§2), through `faraday-window`.

## 9. Pi compatibility

**For now, one screen** (owner, D1: the Waveshare 2.8 inch DPI panel
only). §9.1's checks at other sizes and §9.3's profile mechanism are
deferred until a second screen has been tried on a Pi (owner, D5):
with one panel the card is the image as it is built today, written
whole. §9.2 to §9.3 stay as the design for that day; nothing in them
is built before it.

### 9.1 Layout already follows the screen

The app lays out in units from the framebuffer's pixel size and the
panel's dpi; the 2.8 inch panel (480 × 640 at 286 dpi, 268 units wide)
gets the compact layout, a laptop the wide one. A larger panel needs no
stretching, only its dpi and a check at its size with the snapshot
tour: 800 × 480 at 133 dpi (the official 7 inch) and 720 × 1280 at
294 dpi (a phone-sized 5 inch) first, since nothing has been rendered
at either shape. A screen that renders wrong at its size is fixed in
the layout, for every screen of that shape.

### 9.2 What cannot be detected

HDMI screens and the Raspberry Pi Touch Display (DSI) report
themselves to the firmware. DPI and SPI panels, the 2.8 inch included,
report nothing: their timings, pin map and touch controller have to be
in `config.txt` before the Pi starts, and a wrong guess is a blank
screen. So the Pi image carries a profile per supported screen, and
the screen is chosen when the card is made.

### 9.3 Profiles

One Pi image (per board family, §9.4) carries every profile under
`faraday/panels/` on its boot partition, and `config.txt` ends with
`include faraday-panel.txt`. Making a card writes `faraday-panel.txt`
as a copy of the chosen profile into the boot partition image before
it is written (`faraday-fat` edits the in-memory image); the writer
reads back what it wrote. A profile holds the firmware lines (timings,
overlays, pins) and the shell's three facts that `panel.conf` bakes
into `/etc/opensigner/args` today (dpi, touch controller name, touch
grid), which move to the kernel command line the firmware passes
(`faraday.dpi=`, `faraday.touch=`, `faraday.touch_grid=`), so one image
serves every panel. A card made from the plain image (without the Starter
app) starts with the 2.8 inch profile, and the README says which one
line to change on its boot partition for each other screen.

When the day comes, that batch starts by checking each mechanism on
the Pi 3 (does the firmware take `include` as this needs; does the
shell get the command line) and writes down what it found before
building.

### 9.4 Boards (§12, D3)

1. Pi 3B, 3B+ and Zero 2 W: one 32-bit image; the Zero 2 W board is in
   the tree already.
2. Pi 4: its own kernel and firmware, the same mechanism.
3. Pi 5: later, if at all. It is 64-bit only, its display goes through
   KMS (`DRM` is forbidden on every Faraday image), and its cameras
   need libcamera.

### 9.5 Screens and cameras

The first and, for now, only screen is the 2.8 inch panel (§12, D1);
the next profile comes with the next screen tried. A USB webcam works
on every Pi, because the Pi's USB already allows cameras
(PLAN.md §3); a camera module works on the legacy camera stack, which
reads Camera Module v1 and v2, not v3. The Starter's Pi card page says
which. HDMI with no touch needs a USB keyboard and mouse, which the Pi
forbids today (§12, D2). *Replace Faraday, keep data* for a Pi card
follows when the Pi has its upgrade (PLAN.md §5.5).

## 10. Security

### 10.1 What the Starter is trusted with

The Starter runs on an online computer and writes the system a person
will trust with their keys. It carries the release's own bytes, checks
them against its manifest, shows their sums, reads every write back,
and has no network code. The writer runs elevated for one write and
takes its device, serial and size as arguments it checks again. The
Starter never reads a Faraday stick's data partition.

### 10.2 Unsigned on Windows

The Windows Starter is not code-signed: a code-signing certificate
names a verified person or company, and Faraday publishes under a
pseudonym (owner, 2026-10-12, for privacy). Windows shows "Windows
protected your PC" on first run; *More info* → *Run anyway* opens it.
The website, the README and About say this, and how to check the file
with `SHA256SUMS` and its signature instead. Batch W runs the Starter
under Microsoft Defender on a Windows computer and reports any
detection.

A detection is likely at some point, not only at that run: an unsigned
download that asks for elevation and suspends BitLocker matches
heuristics that change monthly, and §6.3's design (no boot-time task)
exists partly to give them less to match. So the plan for the day it
happens is written now, not then: the website's download page and the
README carry a short section, "If Windows or an antivirus blocks the
file", that says what the Starter does to a computer (§10.3) and how
to check the file against `SHA256SUMS` and its signature; the Starter
itself cannot say anything, since it is the thing blocked. Whether to
submit the file to Microsoft's false-positive form is the owner's call
at the time, since the form asks for an identity; this spec does not
assume it.

The macOS Starter is ad-hoc signed and not notarized, for the same
reason (MAC M8): macOS refuses it on first open; System Settings →
Privacy & Security → *Open Anyway* opens it. The README and About say
so, and how to check the zip against `SHA256SUMS` and its signature.

### 10.3 What it does to the computer

Only what the person asks for: a written stick or card, an install, a
saved file, the BitLocker suspension with its marker and, when
installed, its logon reminder (§6), a restart. Uninstall takes back what install did (§5.4).
On an Apple Silicon Mac (MAC §4.5a), the Faraday start-up volume and
the shrunk macOS container, which the Starter's *Remove Faraday from
this Mac* takes back.

## 11. Batches

| Batch | What | Needs | Order |
|---|---|---|---|
| R | Release groundwork: the glibc 2.31 container for the desktop app; `.xz` images; the new names; the Linux `.tar.gz`; `SHA256SUMS` with the `.img` and inner-file lines; the Starter's payload build; the xz decoder and its `docs/deps` note; README "Verifying a release" for Windows and the two Linux steps | decided | first |
| L | The Starter on Linux: crates (§2), the D-Bus crate and its `docs/deps` note, Home as a checklist, Make a stick through udisks2 (with what each tried distribution asks for), the stick's version read from OSKBOOT and the keep-data rules, Make a Pi card for the one image, Install and uninstall, Try the desktop app, Transfer and Learn, About, `--image` for developers; then `tools/flash-stick` removed and the README's mention of it with it | decided | after R |
| G | Guides: the general Windows and Linux pages and §7.2's other pages with their sources; detection on Linux; Restart into the BIOS on Linux; the stick's power-off reminder (§7.5); the first group of maker pages, several families at once (D6, F14); the website item in its own repository | decided | after R, independent of L and W |
| W | The Starter on Windows: window, install and uninstall, disks and writing, detection, the BitLocker guard with the marker and the logon reminder and §6.6's acceptance, the firmware buttons, Transfer Send; Defender run; the report on what Windows does to a written stick (D11) | decided; D11 is answered by this batch's report | after L; needs G's general Windows page only |
| C | Windows camera for Transfer Receive, shared with the desktop app; the desktop app on Windows (§2's changes) and its install | built (HANDOFF item 76): C1, the desktop app on Windows with its install, its Try and a payload folder per system; C2, the camera on Media Foundation. To be tried on Windows | after W |
| P | Pi: one image for the 2.8 inch panel, Make a Pi card tried on hardware; profiles, boards and other sizes when a second screen is tried | decided (D1, D2, D3, D5) | after L, independent of G, W and C |
| S | Keeping Secure Boot on (§7.7) | D9 | last |
| MAC A3a, A3b | The Starter's macOS module, and Home on a Mac, without a Mac (`docs/MAC.md` §8, HANDOFF item 73) | decided | after L (owner, 2026-10-13) |
| MAC A3c, D | The Starter built and tried on a Mac; the desktop app on macOS | a Mac (MAC M7) | after A3b |

Tests, by batch, state what a person sees or can do: the disks offered,
refused and their order (recorded udisks2 answers on Linux, as a
stand-in `/sys` serves `faraday-grant`'s tests; recorded answers on
Windows); no disk chosen until tapped; a stick that is not the
confirmed one refused; keep data offered only for a Faraday stick,
refusing the same build string and warning on an older one, and never
writing past the boot partition (against a regular file holding a
stick image, as `stick-boot.sh` allows); Home's checklist showing the
next step for each state (no stick, stick attached, protected,
suspended with Secure Boot off, suspended with it on); install writing
exactly §5.2's files into a temporary home, and uninstall removing
exactly them; the BitLocker guard's order (the key confirmed with it
hidden before suspension; the firmware steps offered whatever
BitLocker's state, and closed by another encryption product until its
rescue is confirmed (F21); Resume offered only with the marker and Secure Boot as it recorded; `--remind`
showing a window in one state of three) against a recorded WMI; the
guide chosen for a maker string; a payload file that does not match
its manifest refused; the stick's power-off line for each Secure Boot
byte.

## 12. Decisions

All taken by the owner on 2026-10-12 in discussion, except D11, which
waits for batch W's report. Where a decision changed v0.1, §13 says
what moved.

- **D1 Pi screens:** the Waveshare 2.8 inch DPI panel only, for now.
  Others as they are tried on a Pi.
- **D2 HDMI on the Pi:** not in the first round; it needs USB keyboard
  and mouse on the Pi, with the stick's new-keyboard approval code.
- **D3 Boards:** Pi 3B, 3B+ and Zero 2 W first, Pi 4 next, Pi 5 later.
- **D4 Windows Transfer:** Send and Receive; Receive reads the camera
  through Media Foundation (§8, batch C).
- **D5 Panel profiles (§9.3) and layout checks at other sizes (§9.1):**
  deferred until a second screen is tried.
- **D6 Makers:** the general Windows and Linux pages, and maker pages
  published for several families at once, each checked against the
  maker's own documentation (owner, 2026-10-12, DECISIONS F14,
  changing "only for a family tried on a computer": a page for one
  tried family alone would tell which maker the owner has).
- **D7 BitLocker:** the guard as §6 describes: the key shown and
  confirmed, then suspended until resumed; since F21 an optional
  backup that never comes before the firmware steps.
- **D8 The stick's power-off reminder:** yes, as §7.5.
- **D9 Keeping Secure Boot on:** a later batch.
- **D10 Names:** "Faraday Starter" to people (F16, which changed v0.2's
  "Faraday"), "Faraday Desktop" for the desktop app; `faraday-starter` in the
  code.
- **D11 OSKDATA on Windows:** open. Decided after batch W reports what
  Windows writes to a stick.
- **D12 Home:** a checklist that leads with this computer's next step,
  then the rows (§3.1).
- **D13 Linux writer:** udisks2 over D-Bus, no root in the Starter;
  the pkexec path only if a tried distribution lacks udisks2 (§2).
- **D14 Linux download:** a `.tar.gz`, so the executable bit survives
  and nobody runs `chmod` (§1.2).
- **D15 Keep data reads the stick's version** from the boot partition
  and applies the stick's own upgrade rules (§3.2, §4.1).
- **D16 Guard order:** the key in hand first, then suspend (§6.2).
- **D17 Resuming:** the person's act through the Starter, with a logon
  reminder for an installed copy; no boot-time task (§6.3).
- **D18 The key on a phone:** a plain-text QR code and "Or take a
  photo of this screen"; Save to a file only when a drive for it is
  attached (§6.2).
- **D19 Acceptance for Windows Home:** on a Home computer of the
  owner's, with the owner, after Pro passes in a virtual machine
  (§6.6).
- **D20 The desktop app on Windows:** yes, from batch C, carried by
  the Windows Starter and offered by its Install (§2, §3.5). Built in
  C1: the payload has a folder per system, each Starter offers only its
  own system's desktop app, and Windows install, Try and uninstall
  mirror Linux's (§5.2, §5.3).
- **D21 `tools/flash-stick`:** removed once the Starter writes a
  developer's own image (`--image`, §1.1), in batch L.

## 13. What changed from v0.1

Changed on 2026-10-13 by F17 (owner): the BitLocker step on Windows is
by default the recovery key in hand (where it is also kept, Show the
recovery key without typing it back, **I have my recovery key**, which
opens the firmware steps for the session and changes nothing on the
computer); suspending is **Extra safe: suspend BitLocker**, the guard
as built, offered and never required. §0, §3.1's table, §3.4 step 1,
§6's requirement, §6.2, §6.5 and §6.6 (runs 7 and 8) follow it. The
guides say to choose the stick from the boot menu on the restart that
saves Secure Boot off, and to turn Secure Boot back on before Windows
starts again.

Changed on 2026-10-14 by F21 (owner): the BitLocker step is optional.

From v0.1:

- Home is a checklist of the path, with live state, not a menu (D12).
- Linux writes through udisks2 without root; pkexec is the fallback
  only (D13). The Linux download is a `.tar.gz` (D14).
- Which stick never pre-selects a disk, shows what is on each disk
  and the Faraday version a stick holds, and keeps the device path and
  serial for Confirm. Keep data refuses the same build and warns about
  an older one (D15).
- The guard shows and confirms the key before suspending, hides the
  key while the groups are typed back, offers a QR code and a photo
  for the phone, and offers Save only when there is somewhere to save
  (D16, D18). It names the two cases it cannot pass (no administrator
  account, a managed computer).
- No SYSTEM startup task. A marker file, Resume BitLocker offered only
  with Secure Boot on, and a per-user logon reminder for an installed
  copy (D17). Uninstall handles a standing suspension.
- §6.6 tests Pro in a virtual machine and Home on a computer of the
  owner's, with the owner (D19).
- §3.4 shows only the steps that apply, including the case of Secure
  Boot already off, and puts the phone QR on the restart confirmation.
- §7.6 ships the general pages plus tried makers (D6). §7.7 notes that
  enrolling a key also changes PCR 7.
- The desktop app comes to Windows in batch C (D20); its install check
  box is on both systems. `HOME` and `mlock` get their Windows
  equivalents.
- One Pi screen for now; profiles and other sizes wait for a second
  screen (D1, D5). The Pi image name carries the board and, for now,
  the panel.
- Payload files are checked when used, not at start. Linux installs to
  `~/.local/bin`. Batch W no longer waits on D11; G is independent of
  L and W; P of G, W and C.
- `tools/flash-stick` is replaced by the Starter's `--image` and
  removed in batch L (D21).
- §10.2 writes down what the website and README say when an antivirus
  blocks the file.

Folded in on 2026-10-13 from `docs/MAC.md` §6 (F13), when the owner
picked up the Mac work with the Starter:

- §1.1: the macOS Starter, the macOS desktop app, the Apple Silicon
  stick image and the start-up file, each with its batch.
- §2: the `macos` module (disks through `diskutil`, writing through
  `authopen`, unmounted rather than ejected after a write so the stick
  stays listed), type-checked and tested without a Mac until there is
  one.
- §3.1: Home on a Mac, on any system: no Secure Boot line or firmware
  restart; the Intel Mac's checklist; one line for a Mac that cannot
  start Faraday; the Apple Silicon checklist for later.
- §3.2: *For which computer?*, once there are two stick images.
- §3.4: the Intel Mac's steps, none of them a button.
- §5.2: install on macOS. §7.1: the facts on macOS and the Mac table.
- §10.2: ad-hoc signed, not notarized; *Open Anyway*. §10.3: the
  Apple Silicon start-up volume.
- §11: MAC batches A3a, A3b, A3c and D.
