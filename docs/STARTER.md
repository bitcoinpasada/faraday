# The Starter

**Kind:** Spec
**Status:** Partly built: the Starter on Linux and Windows, the Pi 3 card
**Open:** Home and navigation to Material 3 (§3.1, §3.1d, F91, planned); Faraday Desktop back in the Starter (§5.3, F83, batch DS, planned after the codex32 work); keeping Secure Boot on (§7.7, D9, batch S); panel profiles and the Pi 4 and Pi 5 (§9.3, §9.4); macOS (`docs/MAC.md`)

Decided by the owner (`docs/DECISIONS.md` F11, F12; F33 removed the
BitLocker step, F49 offers BitLocker's suspension, Resume and the key
on the sheet, never required, §6). §12 records each decision.

The build log is `docs/history/STARTER-LOG.md`.

The problem it solves: getting Faraday onto a stick or a Pi card, and a
laptop to start from it, takes a terminal, `dd`, a firmware menu and a
Secure Boot setting, and on Windows a wrong step can leave the computer
asking for a BitLocker recovery key nobody wrote down. The Starter is
one file a person downloads for their operating system and opens. It
makes a Faraday stick or Pi card, shows how to start this computer from
the stick and puts it back as it was, moves files to and from the device
by QR code, and installs itself if the person wants it in the Start
menu or app menu.

People see the app as **Faraday Starter** (owner, 2026-10-12, DECISIONS
F16), and the code's crates use `starter` as well (DECISIONS F16,
2026-10-14). The desktop app is **Faraday
Desktop**, published on its own as testnet-only files and not carried
by the Starter (F61, partly reversing F30; `docs/DEMO.md`) until batch
DS puts it inside the Starter (F83, §5.3); the stick is Faraday.

## 0. Rules for the implementing agent

Rules still in force:

- Nothing in the Starter reaches a network. No HTTP client, no update
  check, no telemetry, no crash reporting. The one link it shows (the
  guides on the website, §7.4) is a QR code a phone opens.
- No writer is run against a disk not given for the purpose.

The agent's working rules: moved to `docs/history/STARTER-LOG.md`.

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
| `faraday-stick-X.Y.Z-x86_64-uefi.img.xz` | the stick image |
| `faraday-pi-X.Y.Z-pi3-waveshare-28dpi.img.xz` | the Pi image: the board family, then the one panel it is built for (§9); the panel part goes when §9.3's profiles arrive |
| `faraday-desktop-X.Y.Z-testnet-only-linux-x86_64` | Faraday Desktop for Linux (x86_64), one executable file; testnet only for real seeds (`docs/DECISIONS.md` F61) |
| `faraday-desktop-X.Y.Z-testnet-only-windows-x86_64.exe` | Faraday Desktop for Windows (x86_64); testnet only for real seeds (F61) |
| `Faraday-Starter-X.Y.Z-macOS.zip` | the Starter for macOS, one universal file (x86_64 and arm64), `Faraday Starter.app`, ad-hoc signed (`docs/MAC.md` §6; built on a Mac, MAC batch A3c) |
| `faraday-stick-X.Y.Z-arm64-apple.img.xz` | the stick image for Macs with M1 or M2 (MAC §4.1, batch A1) |
| `faraday-mac-boot-X.Y.Z.bin` | the Apple Silicon start-up file that starts any Faraday stick (MAC §4.4, batch A2) |

Faraday Desktop is published on its own for Linux and Windows (F61,
partly reversing F30); no Starter carries it, installs it or offers to
try it. The macOS rows arrive with their batches. Every Starter carries the
Apple Silicon image and the start-up file template in its payload once
they exist.

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
  its SHA-256. tools/flash-stick (the window, `write.sh`, the
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
Ubuntu 24.04 (2.39) and Debian 12 (2.36) do not have. The Starter is
built in a pinned container against
glibc 2.31 (Debian 11), so one x86_64 file runs on Ubuntu 20.04 and
later, Debian 11 and later, Fedora 32 and later, and Arch. This is the
`just faraday-linux-bin` the README said was not set up (built in batch R:
`faraday/build/linux/`; Debian 11 has reached end of life, so the
container takes its packages from `archive.debian.org`). Every other
library the apps use at run time (X11 or Wayland, xkbcommon) is loaded
from the desktop the app runs on, as today. The desktop app keeps the
same recipe for a developer's build, unpublished (F30).

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
| Linux | stick image, Pi image |
| Windows | stick image, Pi image |

Before a payload file is written to a disk, the Starter
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
the two image `.xz` files (hard links to the staged ones) and its own
`MANIFEST`, nothing else; the two manifests have the same first line.
The two folders stay apart, since each Starter embeds its own and they
may differ later. `MANIFEST` is plain text: a
first line `# Faraday Starter payload VERSION, commit <full hash>`, then
one line per file, its fields separated by one tab: the payload file's
name, its SHA-256, the name of what is inside it, that file's SHA-256,
that file's size in bytes, and the build ID it carries. The Starter's
build embeds a system's folder, named by `FARADAY_STARTER_PAYLOAD`
(`out/payload/VERSION/linux` for the Linux Starter,
`out/payload/VERSION/windows` for the Windows one); a Starter built
without it carries no payload and offers only `--image`. A manifest
that names a `faraday-desktop-` file is refused, so no Starter is built
with one (F30). The second
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

Estimated size: about 36 MB on each system (11 + 12 MB images, the
Starter itself about 13 MB).

## 2. How it is built

- `faraday/faraday-starter`: the Starter's state, screens and decisions, no
  I/O, tested as `faraday-core` is, through actions in and commands out.
  It draws with `faraday-core::ui` and `faraday-draw`, so it looks like
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
  --write …`, as tools/flash-stick/write.sh did) is not built unless
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
  takes a path from anything but its arguments. (The BitLocker calls
  that went through it as `--bitlocker <verb>` are removed by F33,
  §6.6.)
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
  `faraday-crypto`'s `pin-pages` pins through `VirtualLock` and
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
  desktop app shares with the Starter is §8's. This is a developer's
  build: no release publishes it (F30).
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

Every screen is drawn with `faraday-ui`, in DESIGN §3's themes and
faces, from DESIGN §4's buttons, lines and sheets, as a page from the
top as DESIGN §5's pages are. Titles and
labels are the defaults; the build may tighten wording, not change
what a screen does.

### 3.1 Home: this computer, its checklist, then the cards

The Starter's layout and navigation follow Material 3's rules (F91,
planned, §3.1d): its window size classes, a navigation rail or bar, a
top app bar, cards in a responsive grid and one primary action a
screen. Only the rules are Material's; the look is Faraday's own
(`faraday-ui`, DESIGN's tokens and components), never another app's
colours or shapes. §3.1d says how the window is laid out; this section
says what Home holds.

Home's top app bar carries the wordmark with **Starter** beside it
(F16), the second word in the same Archivo Expanded at a lighter
weight (400) and in the grey of secondary text, on every page that
shows the wordmark, so a person can tell the Starter from the desktop
app and the stick at a glance. Under it, the This computer card, which
holds the checklist of the path from download to a running stick and
the screen's one primary button, then the other cards (owner, D12:
Home leads with the next step for this computer's state, not with a
menu).

**This computer** (a card): maker and model (§7.1); whether Secure
Boot is on (F33: no line on BitLocker, §6.2). On a
computer that cannot start a Faraday stick (§7.1: an ARM computer, or
one that started in legacy BIOS mode), one line says so; Make a stick
stays, for another computer.

**The checklist.** Each item carries its state, read live each time
Home is drawn, never from a file the Starter keeps (§5.1). One item is
marked as the next step, and a tap on the card's action opens it: the
first of Make a Faraday stick and Start this computer from the stick
that is not done (Start is never ticked, so it is the next step once a
stick is attached). Turn Secure Boot back on is listed while Secure
Boot is off and never marked. Items that do not apply to this computer
are not shown.

| Item | State shown | Done when |
|---|---|---|
| **Make a Faraday stick** → §3.2 | "No Faraday stick attached" or the attached stick's Faraday version (read as §4.1 reads it; on Windows "version not read" until keep data reads it) | a Faraday stick is attached |
| **Start this computer from the stick** → §3.4 | "Secure Boot: on" or "off" | never ticked; it is where the firmware steps live |
| **Turn Secure Boot back on** → §3.4 at step 4 (shown while Secure Boot is off, never marked), with the link **What Secure Boot and BitLocker are** (§3.1b) | "Secure Boot: off" | Secure Boot is on |

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
| Secure Boot off (on Windows, reported off now) | **Restart from the stick** | Windows: `shutdown /r /o /t 0`, Windows' *Use a device* (§7.3); Linux: the stick, once (below) |

Another encryption product not yet confirmed (§6.4) closes the restart
buttons: the card shows **I have its rescue disk or password** under the
line naming it, and **Restart into the BIOS** disabled until it is
pressed. On Windows, *Use a device* is shown only when Windows reports Secure Boot off at that moment (`UEFISecureBootEnabled` 0 under `HKLM\SYSTEM\CurrentControlSet\Control\SecureBoot\State`, read without elevation, as This computer reads it, §7.1), and hidden when it reads on or cannot be read (owner, 2026-10-14, F33); the stick
then starts with Secure Boot already off, and the stick's power-off
line still applies (§7.5). On a computer
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

A person who comes back to Windows with Secure Boot off sees Turn
Secure Boot back on listed; a first-time visitor sees Make a Faraday
stick. Nothing in
the checklist writes to the computer except what its items do.

**The checklist** is drawn inside the This computer card, under its
facts and above its button, so the card reads as one thing: this
computer, what is done, what is next.

**The other cards**, under the This computer card in the grid
(§3.1d), each a DESIGN §4.11 card with its heading and a row for each
thing it opens. They hold only what is about this computer and this
copy of the Starter; the destinations (§3.1d) hold the rest:

- **Faraday Desktop** (F83, §5.3): one line, the version the payload
  carries, and **Open Faraday Desktop**, a secondary button.
- **Update the installed Faraday**, a card shown only while the running
  copy is newer than the installed one (§3.5), with **Update**, a
  secondary button. Install itself is the top app bar's action
  (§3.1d).
- **The Faraday stick boot entry**, a card shown only while the entry
  is offered (§3.1c): its line (`starter_remove_entry_sub`, the
  removal under way, or why it failed) and **Remove the boot entry**,
  a secondary button.

Nothing else is on Home: Make a Pi card, Transfer, Blank templates and
Learn are destinations (§3.1d), About and Uninstall are in the top app
bar's ⋮ menu.

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
| Remote desktop, screen sharing, a wireless display | none: the Starter shows no secret (F33; it was checked only around the BitLocker recovery key) | none: the stick has no network |

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
restart buttons stay closed while such a screen is attached, with
**Check again**. Its
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

Wherever the Starter shows Secure Boot (Home's Turn Secure Boot back
on, the Start page) a small link, **What Secure Boot and BitLocker
are**, opens the Learn page of that name (id `secure-boot-bitlocker`):
what each is and what it guards against; why starting Faraday from a
stick puts neither the computer nor its data at risk (the stick's
kernel has no driver for the computer's own drives, SATA, NVMe or
eMMC, so it cannot read or write them, and no network); that Windows
started with Secure Boot back on usually asks for nothing, and when it
may (F49, §6.1); the risk while
Secure Boot is off (the computer would start software not signed by a
key its firmware trusts, Microsoft's on most computers sold with
Windows); and that the Windows guide says where Windows keeps the
recovery key (§6.7). Since F33 it says nothing of suspending or of a
backup.

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

### 3.1d Navigation and size classes (Material 3, F91)

**Size classes** (Material 3's window size classes, read from the
window's width in dp divided by the text size factor, F85, so a larger
text steps the layout down as a narrower window would):

| Class | Width | Navigation | Margins and gutters | Home's grid |
|---|---|---|---|---|
| Compact | under 600 dp | the navigation bar at the foot | 16 dp | one column |
| Medium | 600 to 839 dp | the navigation rail at the left | 24 dp | one column |
| Expanded | 840 dp and over | the navigation rail at the left | 24 dp | two columns; three from 1200 dp |

Every class change re-lays the page out at that width; nothing is
scaled to fit. A page's content is at most 1040 dp wide, centred in the
space beside the rail. DESIGN §3 maps the classes: the Starter's
compact is DESIGN's small panel, its medium and expanded are DESIGN's
wide layout with the rail in place of the sidebar.

**Destinations**, four, each a page one level deep with no menu
under it:

| Destination | Icon | Page |
|---|---|---|
| **Home** | the shield (`Ui::mark`) | §3.1 |
| **Make** | `Glyph::Stick` | two cards: **Faraday stick** (the attached stick's version, or "No Faraday stick attached"; **Make a Faraday stick**, §3.2) and **Pi card** (the Pi image's version, or "Not in this Starter"; **Make a Pi card**, §3.3, disabled without the image) |
| **Tools** | `Icon::Qr` | two cards: **Transfer** (Send and Receive, §3.6) and **Blank templates** (§3.8) |
| **Learn** | `Icon::Learn` | the Starter's Learn pages (§3.6, §7) |

On each destination its first card's button is the primary; on Make,
**Make a Faraday stick** is primary until a stick is attached, then
neither is.

**The navigation rail** (medium and expanded): 80 dp wide at the
window's left, in `SIDEBAR`, the four destinations from the top under
a 24 dp gap, each its icon over its label (caption), the current one
on an accent pill behind the icon and its label in `TEXT`, the others
in `MUTED`. **The navigation bar** (compact): 80 dp tall across the
window's foot, the same four, evenly spaced, drawn the same way. A
press opens the destination as it was last seen (F77's rule, per
destination); a press on the current one goes to its top.

**The top app bar**, 64 dp, across the page beside the rail (or the
whole width on compact): on Home the wordmark, on another destination
its name as the title. At the right end: the install action while it
applies, **Install** while nothing is installed and **Update** while the
running copy is newer (§3.5), a secondary button, and the ⋮ menu
(DESIGN §4.1) with what is not a destination:

- **About** → §3.7;
- **Uninstall** → §3.5's Installed record, with its Uninstall, shown
  while a copy is installed;
- **Remove the boot entry**, shown while offered (§3.1c); the same as
  its card on Home.

The page scrolls under the top app bar, which stays.

**Pages inside a task** (§3.2's steps, §3.4, §3.5, §3.7, §3.8's pages,
and Transfer and Learn's own screens) are not destinations: they keep
the top app bar with a back arrow and the page's name in place of the
wordmark, and the rail or the bar is not drawn, so a write in progress
cannot be left by a press beside it. Back lands on the destination and
the scroll it was opened from (F77). Transfer and Learn are the
desktop app's screens (§2), drawn in the space under the top app bar.

**One primary action a screen** (Material's filled button is Faraday's
`Style::Primary`, tonal is `Secondary`, outlined is `Ghost`): Home's is
the This computer card's button (§3.1's table), a destination's is
named above, a task page's is its step's own. Every other button on the
screen is secondary or ghost.

**Tokens** (`core/faraday-draw/src/tokens.rs`): the class widths
(`WINDOW_MEDIUM` 600, `WINDOW_EXPANDED` 840, `WINDOW_GRID_THREE`
1200), `CONTENT_MAX` 1040, `RAIL_W` 80, `NAV_BAR_H` 80, `APP_BAR_H` 64,
`MARGIN_COMPACT` 16, `MARGIN_WIDE` 24, the destination pill
(`NAV_PILL_W` 56, `NAV_PILL_H` 32), and Home's card minimum width
(`CARD_MIN_W` 320). All are multiples of Material's 4 dp grid. None of
the Starter's pages writes a number in place for them.

**Building it** (planned; Sonnet from this section, then the owner's
look on Linux and Windows at a narrow and a wide window):

1. **N1, the frame.** The tokens; the size class in `faraday-starter`
   (replacing `compact`'s single 600 dp test); the rail, the bar and the
   top app bar as `faraday-ui` components (DESIGN §4.1); `Page` gains
   `Make` and `Tools`; the frame draws the rail or the bar on the four
   destinations and the back-arrow bar on task pages.
2. **N2, the pages.** Home's This computer card with the checklist
   inside it, the other cards in the grid; Make and Tools; the ⋮ menu;
   Install and Update in the bar. Strings in `faraday-strings`.
3. **N3, Transfer and Learn** drawn under the top app bar (§2's held
   `Faraday` given the space below it; batch R's way of holding it is
   kept).
4. **Tests** (user-facing): at 400 dp the bar shows the four
   destinations and at 700 dp and 1300 dp the rail does; each opens its
   page; a task page shows neither and Back returns to the destination
   and its scroll; every screen has one primary button; the ⋮ menu
   lists Uninstall only while installed and Remove the boot entry only
   while offered; Install and Update appear in the bar as §3.5 says;
   Larger text (F85) at 700 dp takes the compact layout.

**Questions for the owner** (each with what the spec assumes until
answered):

- Home holds only the cards about this computer (above), not a card per
  destination as in the first sketch: the rail already shows them.
  Assumed: no destination cards on Home.
- Faraday Desktop's own windows keep DESIGN §3's two layouts and the
  240-unit sidebar (F91 says so); Material 3 there would be a separate
  decision. Assumed: the desktop app does not follow.

### 3.2 Make a Faraday stick

0. **For which computer?**, only once the payload carries
   both stick images (MAC A1): "A PC or an Intel Mac" or "A Mac with
   M1 or M2". Nothing is chosen for the person, as with the disk; the
   row that fits this computer says "This computer". Keep data refuses
   a stick of the other kind (MAC §4.8).
1. **Which stick?** The USB disks attached now, each as a
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
2. **What to write?**
   - *New Faraday stick*: the whole stick; everything on it is lost.
   - *Replace Faraday, keep data*: only on a Faraday stick. The boot
     partition is written; the data partition, with its vaults and
     settings, is not read and not written. One line under it states
     the fact the Learn page explains (§4.4). The stick's own upgrade
     rules apply (F2, F5; owner, D15): the same build string as the
     stick already holds is refused with the reason; an older Faraday
     over a newer stick is warned about on Confirm and can still be
     written; anything else is written. On Windows, ticking it on a
     stick whose version was not read asks for administrator permission
     once to read it (§4.1, D22), and Continue waits for the answer;
     *New Faraday stick* needs no version and asks nothing.
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
follow this computer's state.

**Write down** (owner, 2026-10-15 and 2026-10-19, F47),
before step 1: what the person needs once the computer restarts and
the Starter is no longer on screen, as three rows, a label and its
value each, then
**Print**:

- **Boot menu key**: the maker's, to start the stick (`faraday_learn::Maker::boot_key`);
- **BIOS key**: the maker's, to get back into the BIOS afterwards and
  turn Secure Boot on before Windows starts (`Maker::bios_key`, F42);
- **Secure Boot setting**: where the setting is in this maker's BIOS,
  as its guide names it, "Boot Configuration → Secure Boot" on a Dell
  (`Maker::secure_boot`).

The values come from the maker table (§7.6), each agreeing with the
maker's guide (a `faraday-guides` test). A computer whose maker has no
page reads **Not known** on each row; the sheet leaves a line to
write on. On an Intel Mac the rows are the Mac's equivalents (the Mac
steps below): **Boot menu key** "Option, held"; on a T2 Mac
**macOS Recovery key** "Command-R, held" and **Startup Security**
"macOS Recovery → Utilities → Startup Security Utility"; a Mac the
table does not know has those two marked "Only on a Mac with the T2
chip"; a Mac known to have no T2 chip has only the first.

**Print** (owner, 2026-10-19) → the sheet as a PDF made by
`faraday-core`'s PDF writer (`pdf::notes`), written into the system's
temporary folder as `faraday-start-from-the-stick.pdf` and opened at
once in the system's own viewer, exactly as Blank templates' Print
(§3.8): the person prints it, or saves it, from the viewer. Offline;
no printing code or service of the Starter's own, on Windows, Linux
and macOS. A viewer that cannot be opened shows why under the button.
The sheet carries the page's title, the three rows (a line to write on
for a value not known), two lines to circle the key that worked (below),
on Windows with the box ticked the BitLocker recovery key (below), the
steps in order, each with one line of how — **Turn Secure Boot off**,
**Start from the stick**, **Turn Secure Boot back on before starting
Windows** (on Linux, **Turn Secure Boot back on**) — and under the
first, the not-found line below. Nothing else: no guide address, no
computer name, no date, no explanation (CLAUDE.md: labels, values and
actions).

**The keys used** (owner, 2026-10-19; F47): the sheet, not the page,
has a line each to circle the key that worked, so it is plain later
which one did: **Boot menu key used (circle one)** and **BIOS key used
(circle one)**, each the maker's own key from the table first (both
keys, in order, where the table names two: HP's "Esc, then F10"), then
every other single key the maker table's `boot_key` or `bios_key`
names, in a fixed order (Esc, the function keys by number, Fn with a
function key, then the rest as the table has them: Delete, Volume Up),
then **Other:** with a line to write on. For Dell: "F12 · Esc · F9 ·
F11 · Fn+F12 · Volume Down · Other: ________" and "F2 · Esc · F1 · F10
· Fn+F2 · Delete · Volume Up · Other: ________". A maker without a
page has the same lines with no key first. The keys come from the
table, so a maker added to it adds its keys. An Intel Mac's sheet has
neither line: its keys are fixed (Option; Command-R on a T2 Mac), so
there is nothing to circle.

**The BitLocker recovery key on the sheet** (owner, 2026-10-19; F49):
on Windows with BitLocker on or suspended on the system drive (§6),
beside Print a tick box **Include the BitLocker recovery key**,
unticked whenever the Starter starts. Unticked, Print is as above.
Ticked, Print first reads the key through the elevated copy (§6.2, one
elevation prompt; "Waiting for administrator permission" meanwhile),
then the sheet carries, after the rows, **BitLocker recovery key, drive
C:** and the 48 digits as Windows writes them, four groups to a line.
Nothing else of BitLocker: no key ID, no computer name, no QR code. A
key that cannot be read says why under Print and nothing is printed.
The key is never drawn on the page. On an Intel Mac, the Mac's
steps as the page has them, and no not-found line: the Startup
Security Utility is always in macOS Recovery. The PDF has no
document information dictionary (no title, author, producer or
dates) and names no path, user or computer.

**If the Secure Boot setting cannot be found** (owner, 2026-10-15):
step 2 carries "If the Secure Boot setting cannot be found: exit the
BIOS without saving changes. Windows starts as before." (on Linux,
"The computer starts as before."). The sheet carries the same line,
and every guide's firmware step says it too (§6.7), so the phone page
does.

The steps:

1. **Restart into the BIOS** (§7.3). Its confirmation ("Save
   your work first: this restarts the computer") carries the QR code
   of this page's address on the website (§7.4), because that is the
   moment the person needs the steps on a phone; the same code is at
   the end of the page.
2. The firmware steps for this maker: turn Secure Boot off; turn Fast
   Boot off where it hides USB; the not-found line above. When Secure
   Boot is already off, this step says so and has nothing to do.
3. **Start from the stick:** the maker's boot menu key, on the restart
   that saves Secure Boot off, before Windows starts; or, on Windows,
   **Restart to choose a device** (Windows' *Use a device*, §7.3),
   offered only while Windows reports Secure Boot off (F33).
On an Intel Mac the steps are the Mac's, from the `mac-intel` guide
(MAC §3.1, §3.2), and none of them is a button: on a T2 Mac, lower
Startup Security in macOS Recovery; then shut down and hold Option at
power-on, and choose **EFI Boot** (after the firmware password, if
one is set); on a T2 Mac, afterwards, Startup Security back to Full.
A Mac the table does not know, and not ARM, gets the T2 steps marked
as for a Mac with the T2 chip only.

4. **Turn Secure Boot back on**, in the BIOS, before Windows starts
   (F33). The last step, shown always, as the guides' last step is
   (§6.7).

**BitLocker** (F49), on Windows with BitLocker on or suspended on the
system drive, between Write down and step 1: the row **BitLocker, drive
C:** with its state, and the one step that state allows, never required
(§6.2). Nothing waits on it: the restart and the steps are the same
with BitLocker off. Another encryption product not yet confirmed
(§6.4) still closes the restart button.

Tests (F47), user-facing: the page's rows and the sheet's content for
a known maker, a maker without a page and an Intel Mac (a T2 Mac, one
without T2, one the table does not know); Print hands the shell that
PDF to open; the not-found line on the page, on the sheet and in every
guide's firmware step; the maker table's boot menu key, BIOS key and
setting agreeing with each maker's guide (`faraday-guides`); the lines
to circle the keys, the maker's first, for Dell, HP and a maker without
a page, and none on a Mac. F49's: `docs/history/STARTER-LOG.md` §6.5. Built: `faraday-starter`'s
`notes.rs`, `tests/all/notes.rs`; `faraday-core`'s `pdf::notes`; the
shell's `pdf.rs` as for §3.8.

### 3.5 Install and uninstall

**Install** has no check box: **Install now** is one press, then Done,
which shows where it went (F30). When the running copy
is newer than the installed one, the top app bar's action reads
**Update** and Home shows the Update the installed Faraday card
(§3.1d).

**Installed** (the ⋮ menu's Uninstall, §3.1d) opens a Record of what is installed and where, with
**Uninstall**, which asks once. §5 says what each does.

### 3.6 Transfer and Learn

The desktop app's screens (§2). On Windows, Transfer offers Send and
Receive; Receive reads the camera through Media Foundation (§8).

### 3.7 About

Version; each payload file with its build ID and SHA-256; the release
key's fingerprint as the README prints it; how to check the Starter
file itself with `SHA256SUMS` and its signature; on Windows, why the
file is unsigned (§10.2).

### 3.8 Blank templates (F31)

(owner, 2026-10-14; `docs/BACKUP-FLOW.md` §10.2. Built: batch T.) The printable sheets a seed is copied onto by hand, offered
here because the Starter runs on the computer with the printer, and a
blank template holds nothing of any wallet. Faraday's backup checklist
says to print them from here (its **Templates** item) and writes none to
a stick.

A page titled **Blank templates**, opened from Home's row **Blank
templates**: a row each for Words, Copy and Places, its value at the
right, each opening a page of its options (a ring each, filled on the
one chosen) or, for Places, **Fewer** and **More**, with **Continue**
back; then the buttons:

- **Words**: **12** · **24**, 24 chosen;
- **Copy**: **Words** · **SeedQR** · **Both**, Words chosen: the word
  lines, the SeedQR grid, or both on one sheet, as the backup's `copy:`
  line has them (`docs/history/NEW-WALLET.md` §14.7);
- **Places**: a count to fill in by hand, 2 filled in, one or more, no
  upper bound;
- **Print** (primary) → the sheet made by `faraday-core`'s
  `backup::sheet_blank` with no wallet (no name, no shape), the network
  "mainnet", and the choices above (a `copy:` line added to the sheet
  file), then `pdf::sheet`; the PDF is written into the system's
  temporary folder and opened at once in the system's PDF viewer
  (`xdg-open`, `open`, `explorer.exe`: the system's own "open with the
  default program", never a print verb that sends the file to the
  default printer unasked). The person prints from the viewer, to any
  printer or to a PDF. The sheet prints at its true size, so that the
  SeedQR grid's cells stay square and their size;
- **Save the PDF** (secondary) → the same PDF, saved into the Downloads
  folder as `faraday-blank-template-{12|24}-words-{words|seedqr|words-seedqr}.pdf`,
  never over a file ("name (2).pdf", as Transfer's Receive names) →
  "Saved {path}" with **Open it** (the viewer).

Nothing else: no wallet, key or seed is asked for or shown. Tests (batch
T): each of the six choices of Words and Copy makes a PDF whose sheet
text has those `words:` and `copy:` lines and no `name:`, saved under
its name; a second save beside a first is named "(2)"; Print makes the
same PDF bytes as Save and asks the shell to open it. Built:
`faraday-starter`'s `template.rs`, `tests/all/template.rs` and the shell's
`pdf.rs`.

## 4. Writing a stick or a card

### 4.1 What the writer does

As tools/flash-stick/write.sh did (removed in batch L, D21; this
section is the reference): refuse unless
the device is still the stick that was confirmed (serial and size);
unmount what the desktop mounted from it; write; flush; read the whole
write back and compare; ask the system to read the partition table
again. *Keep data* writes the boot partition at its offset after the
same partition-table checks as `faraday/image/stick-boot.sh`, and
checks the partition table is unchanged afterwards: GPT with the disk
UUID `46617261-6461-792D-5374-69636B000000` and exactly two partitions,
`FARADAYBOOT` (an EFI system partition, `…000001`) and `FARADAYDATA`
(`…000002`). A stick made before 2026-10-21, whose partitions are
`OSKBOOT` and `OSKDATA`,
is refused as any other disk is, and nothing is relabelled: its files are
moved to a new stick by hand (owner, 2026-10-21; `docs/DECISIONS.md`
F67). A stick whose `FARADAYBOOT` is smaller than the image's 64 MiB,
such as one made with a 48 MB boot partition before F80, is refused
the same way, with both sizes. The source is
decompressed from the payload while it is written, and the SHA-256 of
the decompressed stream must equal the manifest's before the writer
reports success.

**What the Starter reads from a Faraday stick.** Only the boot
partition, and only to learn which Faraday it holds: the build string
(F5) the stick's own Upgrade a Faraday stick reads from another stick
(`faraday-core/src/upgrade/mod.rs`): the kernel release string in
`EFI/BOOT/BOOTX64.EFI` on a stick, or `faraday-build.txt` on a Pi
card (§1.3). On Linux it is read from the desktop's own mount of
FARADAYBOOT; when FARADAYBOOT is not mounted, the Starter has udisks2 mount it
read-only without asking for a password (`auth.no_user_interaction`),
reads it and unmounts it (batch L: opening the raw partition needs
`OpenDevice`, whose polkit prompt would ask for a password just to list
disks, and `faraday-fat` does not enter subdirectories; it only checks
the boot sector's FARADAYBOOT label). A stick whose version cannot be read
shows as one made before versions were recorded. That version is shown
on Which stick and Confirm and drives the keep-data rules (§3.2).
FARADAYDATA is never mounted, opened, read or written (§4.4).

**On Windows** (owner, 2026-10-19, option (e), D22) FARADAYBOOT
is an EFI system partition, which Windows lets only an elevated process
read, so the Starter's own process cannot read the version. "Could not
read" is kept apart from "no version on the stick": Home and Which stick
say "Faraday stick · version not read", never "0.1.0 or earlier".
*New Faraday stick* needs no version. *Keep data* starts the elevated
copy once with `--read-version` (the disk, its serial and size as
listed, and the pipe to answer on); it checks the disk is still that one,
reads the partition table as keep data checks it and up to the first 64
MiB of FARADAYBOOT, and answers the release string found there
(`faraday_boot::find_release`), or none for a stick made before versions
were recorded. The Starter then applies the same rules as on Linux: the
same build refused, an older Faraday over a newer stick warned about on
Confirm. What was read stands for the session until the stick is
written. A declined prompt says so on What to write and nothing is
written. As a second check, the elevated writer reads the stick's
version again before a keep-data write and refuses the same build.
Linux and macOS read the version as above and never ask; the image and
Upgrade a Faraday stick are unchanged.

### 4.2 Which disks are offered

- A whole disk on USB, removable or hot-pluggable (Linux: udisks2's
  `Drive.ConnectionBus` = `usb`, `Removable` or `Ejectable`); or, for
  a Pi card, an SD card in the computer's own reader (Linux `mmcblk*`;
  Windows bus types SD and MMC).
- Never a disk with a partition the computer uses: on Linux mounted
  anywhere but `/run/media`, `/media` or `/mnt` (udisks2's
  `Filesystem.MountPoints`), or swap (`Swapspace.Active`); on Windows
  the disk holding the Windows, boot or page-file volume.
- Never a disk smaller than the image: 193 MiB and 20 KiB for the
  stick, 196 MiB for the Pi card (F80). The minimum the Starter states
  is a USB stick or SD card of 256 MB or larger: with none attached,
  Which stick? and Which card? say "No USB stick attached · 256 MB or
  larger". Faraday's own boot file takes about
  14 MB of the stick's 64 MiB boot partition, and the Pi
  card's boot files about 16 MB of its 64 MiB; the rest is
  room for later versions to grow. The 128 MiB data partition is
  storage for files (vaults, PSBTs). That explanation is in the README
  and the Learn pages, not on a working screen.
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
given to FARADAYBOOT and FARADAYDATA, folders it writes such as `System Volume
Information`) and reports it. FARADAYDATA keeps its drive letter: nothing
sets the GPT "no drive letter" attribute, and Faraday reads what
Windows writes there and ignores Windows' own files (§4.4; §12, D11).

### 4.4 Keep data

Offered to everyone on a Faraday stick (F11). The writer never reads
or writes FARADAYDATA. The fact the person should know is that plugging the
stick into this computer is what exposes it: the computer can read and
write the data partition whatever the Starter does, and Windows mounts
it with a drive letter. The vaults there are encrypted; the settings
file is not. The Learn page "Replacing Faraday on a stick" says this
and points to Settings → Upgrade a Faraday stick, which upgrades a
stick from another stick without an online computer.

**FARADAYDATA from Windows and macOS** (owner, 2026-10-19;
D11). FARADAYDATA keeps the drive letter Windows gives it, so a person can
put a PSBT, or any file, on a Faraday stick from Windows, and a Mac
mounts it as it mounts any FAT volume. Faraday reads what they write
there as it reads any file a person puts on a stick. What each writes
of its own, at the volume's top level:

| | Folders | Files |
|---|---|---|
| Windows | `System Volume Information\` (`IndexerVolumeGuid`, `WPSettings.dat`), on the first mount; `$RECYCLE.BIN\` (`desktop.ini`, `$I…`/`$R…` pairs) once a file is deleted in Explorer; `RECYCLER\` on older Windows | `desktop.ini` once a folder view is customised; `Thumbs.db` where Explorer caches thumbnails |
| macOS | `.Spotlight-V100\`, `.fseventsd\`, `.Trashes\`, `.TemporaryItems\` | `.DS_Store`; a `._<name>` AppleDouble file beside every file the Mac copies there |

The rule, by name alone: a name beginning with a dot, or one of
`System Volume Information`, `$RECYCLE.BIN`, `RECYCLER`, `desktop.ini`
and `Thumbs.db` compared ignoring case, is the computer's own. It is
never listed, never read and never counted: not in the first look at
the boot stick (`docs/history/PLAN.md` §5.4), not on a visit, not in the Inbox, and
a read that names one is refused. Folders are never entered in any
case. The hidden and system attributes are not trusted alone, either
way: Windows marks its own files with them, but a file is ignored for
its name, and a person's file is read whatever its attributes say.
Everything else a person puts there is read as before. A write leaves
the computer's files where they are: Faraday writes a new file at the
top level under a name no file there has, and goes over only the file
it replaces (a vault over its own, the settings file over the settings
file, a file the person chose Replace for); zeroing the whole
partition, which the person asks for by name, empties it of them too,
and the computer writes them again the next time it mounts the stick.
The rule is `faraday_files::foreign`; the device's disk process, the
desktop app's stick folders and its Downloads listing all list through
it.

## 5. Install, run without installing, uninstall

### 5.1 Running without installing

Opening the downloaded file runs the Starter. It writes nothing to the
computer except what the person asks for (a stick, a card, a received
file). Home's checklist
(§3.1) reads its state live; there is no settings or state file.

### 5.2 Installing

Per user, never needing administrator rights.

| | Linux | Windows |
|---|---|---|
| The Starter | `~/.local/bin/faraday-starter` (the XDG place for a user's executables, and on PATH on most desktops) | `%LOCALAPPDATA%\Programs\Faraday Starter\Faraday Starter.exe` |
| Menu | `~/.local/share/applications/app.faradayoffline.FaradayStarter.desktop` "Faraday Starter" | Start menu shortcut `Faraday Starter` |
| Icon | `~/.local/share/icons/hicolor/256x256/apps/app.faradayoffline.FaradayStarter.png` | in the `.exe` |
| Uninstall entry | — | `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Faraday Starter`: name "Faraday Starter", version, publisher "Faraday", icon, size, `UninstallString` = `"…\Faraday Starter.exe" --uninstall` |

On macOS: the Starter at `~/Applications/Faraday Starter.app`, a copy
of the running bundle made with `ditto` (which keeps its signature);
per user, no administrator. The
bundle is the menu entry and carries the icon. A bundle of that name
whose identifier is not `app.faradayoffline.FaradayStarter` is in the
way, as on Linux. Uninstall removes exactly that.

The menu file names are reverse-DNS, so they never collide with a
developer's own `faraday.desktop`. The executable's name is plain, so
install replaces, and uninstall removes, `faraday-starter` (and the
earlier names) only when the file there answers `--version` as a
Faraday Starter (orchestrator, 2026-10-12); a file of the same name
that does not is left alone, and Install says it is in the way.

The Starter no longer installs, lists or removes the desktop app
(F30). A desktop app an earlier Starter installed
(`~/.local/bin/faraday-desktop` and its menu entry on Linux,
`Faraday Desktop.exe` beside the Starter and its Start menu shortcut on
Windows) is left in place by install, update and uninstall; the README
says how to remove it by hand. A developer's own build there is not
the Starter's.

### 5.3 The desktop app from the Starter

Gone with F30: the Starter no longer runs the desktop app or keeps a
cache of it. F83 (owner, 2026-10-22) brings it back: the Starter
carries, opens and installs Faraday Desktop, and releases stop
publishing it on its own. Planned as batch DS, after the codex32 work.
It is the design batch C1 built and F30 removed (commit `2e807ae0` has
the code and tests it took out, to start from), with the Install check
box gone. When DS is built, the sections below are rewritten to say
what is built, and this section keeps only the running rules.

**The payload** (§1.3). The first staging pass writes each system's
desktop app into its payload folder, compressed by the same `xz -9e
-T1` as the images: `faraday-desktop-VERSION-linux-x86_64.xz` in
`linux/`, from `out/linux/faraday`, and
`faraday-desktop-VERSION-windows-x86_64.exe.xz` in `windows/`, from
`out/windows/faraday.exe`, both built `release=1` as F61's files are now.
Staging refuses either when it does not carry the build ID as a
string, and scans it for the build computer's paths as it scans every
published file. Each gets its `MANIFEST` line like an image. The
manifest stops refusing a `faraday-desktop-` file. A Starter offers
only its own system's app: a `faraday-desktop-` entry counts only when
its name ends `-linux-x86_64.xz` on Linux or `-windows-x86_64.exe.xz`
on Windows. The macOS Starter carries none (F83).

**Releases** (§1.1). The two `faraday-desktop-X.Y.Z-testnet-only-…`
rows leave the table, `SHA256SUMS` and the release folder; staging
refuses a release folder that holds a `faraday-desktop-` file, as it
refuses one in a payload today. The release notes and the README's
Verifying a release and Faraday Desktop sections say Faraday Desktop
is inside the Starter.

**Open Faraday Desktop**, a row on Home (§3.1) between Transfer and
Install, with no line under it; the app itself carries F81's seed line.
It runs the installed copy when that answers `--version` with the
payload's build ID; otherwise it decompresses the payload's copy into
`$XDG_CACHE_HOME/faraday/X.Y.Z/` (default `~/.cache/faraday/X.Y.Z/`;
on Windows `%LOCALAPPDATA%\faraday\cache\X.Y.Z\Faraday Desktop.exe`),
checks its SHA-256 against the manifest, and runs it. Every start
checks the cached copy again and writes it again when the sum is not
the manifest's. The app is started on its own, so closing the Starter
does not end it; a start that fails within 0.4 seconds is reported. If
the Material 3 layout (§3.1d) lands first, it is Home's Faraday Desktop
card (§3.1).

**Install** (§3.5, §5.2) installs both, with no check box: **Install
now** stays one press. Beside §5.2's rows:

| | Linux | Windows |
|---|---|---|
| Faraday Desktop | `~/.local/bin/faraday-desktop` | `%LOCALAPPDATA%\Programs\Faraday Starter\Faraday Desktop.exe`, beside the Starter |
| Menu | `~/.local/share/applications/app.faradayoffline.FaradayDesktop.desktop` "Faraday Desktop" | Start menu shortcut `Faraday Desktop` |
| Icon | `~/.local/share/icons/hicolor/256x256/apps/app.faradayoffline.FaradayDesktop.png` | in the `.exe` |

The installed copy is the decompressed payload file, checked against
its manifest line, written beside its place and renamed in. Install
and update replace `faraday-desktop` only when the file there answers
`--version` as Faraday Desktop; a file that does not is left alone, and
Install says it is in the way. A desktop app an earlier Starter
installed is therefore replaced, and §5.2's paragraph on leaving it in
place goes. **Update the installed Faraday** updates both. Installed's
Record lists the desktop app and its menu entry.

**Uninstall** (§5.4) removes the desktop app, its menu entry and icon,
and the cache, with the rest; never `~/faraday-sticks` or the desktop
app's settings.

**Size.** The Linux desktop app is 14.5 MB, 5.6 MB as `.xz`
(measured 2026-10-23); the Windows one is of the same order. Each
Starter grows by about 6 MB, from about 36 MB to about 42 MB; the owner
accepted that growth on each system (2026-10-23).

**Tests** (§11): staging puts the desktop app in each payload folder
with its manifest line and puts no `faraday-desktop-` file in the
release folder, and refuses one there; a Starter built against another
system's payload offers no desktop app; Open runs the payload's app
from a cache it checked, and writes a damaged cached copy again;
install writes the desktop app's files into a temporary home, replaces
one that answers `--version` as Faraday Desktop, leaves one that does
not, and uninstall removes exactly them.

**Batch DS edits.** `faraday/tools/stage-release.sh` (first pass and
the refusals), `faraday/faraday-payload` (the manifest's refusal goes,
the system filter returns), `faraday/faraday-starter` (the Open row,
Installed's Record) and its strings in
`faraday/faraday-strings/src/en.rs`, `faraday/shells/starter` (Linux
and Windows install, uninstall, the cache, the run), their tests; this
document's §1.1, §1.3, §3.1, §3.5, §5.2 and §5.4, §12's D20;
`docs/DEMO.md` §1 and §9; `docs/MAC.md`'s two F61 mentions; the README.
The website's download page, blog and FAQ follow in its own
repository.

### 5.4 Uninstalling

Removes exactly the files and entries §5.2 lists for this computer.
It never touches anything the person made: received
files, the desktop app's `~/faraday-sticks` and settings. On Windows
the running copy cannot delete itself; the uninstaller finishes by
starting a short command that removes the folder once the Starter has
exited.

Uninstall's BitLocker paragraph (Resume, the logon reminder and the
marker) is removed by F33, with no Resume kept (§6.3).

## 6. Secure Boot back on, and BitLocker offered (Windows)

**The requirement (owner, 2026-10-14, DECISIONS F33):** the step that
keeps Windows from asking for the BitLocker recovery key is turning
Secure Boot back on in the BIOS before Windows starts again. **Since
F49** (owner, 2026-10-19) that holds in the common case only, and the
Starter offers, never requires, three things on the Start page (§3.4):
**Temporarily suspend BitLocker**, **Resume BitLocker**, and the
recovery key on the printed sheet. The steps stay Secure Boot off,
start from the stick, Secure Boot back on.

Until 2026-10-14 this section was the BitLocker guard (F11, F12, F17,
F21): Show the recovery key with its QR code and Save, an extra-safe
suspension with a marker, a logon reminder and Resume, and §6.6's
acceptance runs. F33 removed it (commit 53a715be); F49 brings back,
from that code, the elevated `--bitlocker` verbs and the WMI reading,
with a restart count in place of the marker (§6.2), and nothing of the
rest (§6.3).

### 6.1 Why turning Secure Boot back on is usually enough, and when not

BitLocker, and Windows 11's Device encryption, which turns it on
without asking on many laptops, seals the drive's key to the TPM's
measurement of how the computer started, which on most laptops
includes the Secure Boot state (PCR 7). The TPM is asked only when
Windows starts. Secure Boot off, Faraday from the stick, Secure Boot
back on, then Windows: Windows sees the start it knows and asks for
nothing (F17's observation on a computer of the owner's, and run 5 of
the old acceptance list in a virtual machine).

**When it is not enough** (F49). The key unseals only when the
measurements match those it was sealed to. A firmware that resets its
Secure Boot keys or databases when Secure Boot is turned off and on
(Setup Mode, factory keys, a dbx update lost), BitLocker sealed to
other PCRs, or Windows started while Secure Boot is still off: Windows
asks for the 48-digit recovery key. Faraday never touches the internal
drive; that does not prevent it.

**Suspending first avoids it.** A suspended volume's key is kept in the
clear on the drive, so Windows starts without the TPM's answer; when
protection comes back (by itself after the count, or by Resume), it is
sealed again to the computer's state at that moment, so a firmware
whose Secure Boot keys changed does not ask afterwards either.

### 6.2 What the Starter does on Windows

- **BitLocker is read each time the Start page is shown**, without
  elevation: the shell's `System.Volume.BitLockerProtection` for the
  system drive (the drive of the Windows folder, `C:`), on a thread of
  its own. 1, 3 and 6 read as on, 5 as suspended (the Windows test
  virtual machine read 1 and 5, `tests/windows/vm-on.wmi`,
  `vm-suspended.wmi`); anything else, 2 included, or nothing read, is
  not known, and not known offers nothing. No marker file and no logon
  entry: Windows' own count ends a suspension.
- **On**: the row **BitLocker, drive C:** reads **On**, with
  **Temporarily suspend BitLocker** and under it "For the next 3 times
  Windows starts; then it turns back on by itself". The button starts
  the elevated copy (`--bitlocker suspend`, one elevation prompt, as the
  writer's), which calls `DisableKeyProtectors` with a `DisableCount`
  of 3 on every protected operating-system volume, as `manage-bde
  -protectors -disable C: -RebootCount 3` does, reads every volume
  back (one still protected is a refusal), and on a refusal resumes
  those it suspended, so nothing is half done. A refusal reads "This
  computer's BitLocker is managed: its administrator has to suspend
  it."; a declined prompt on an account that is not an administrator's
  reads "An administrator of this computer has to do this step."
- **Suspended**: the row reads **Suspended · turns back on by itself
  after Windows starts 3 times** (the count as the elevated copy read it
  back, `GetSuspendCount`; "once" for 1; "Suspended until resumed" for a
  suspension with no count; "Suspended" when the count was not read,
  as after the Starter restarts, since the reading without elevation
  has none), with **Resume BitLocker** beside it, which turns it back
  on now (`EnableKeyProtectors` on every suspended operating-system
  volume, read back). Whoever suspended it, Resume is the same step.
- **The count counts Windows' starts.** Windows keeps the count and
  lowers it each time Windows starts; the BIOS and the stick do not run
  Windows, so the restart into the BIOS, Faraday's session and the
  restart after it do not use it up. 3 leaves room for a Windows start
  between (an update, a start before the stick is chosen). How Windows
  counts is to be confirmed on the Windows test virtual machine;
  the count is `api::SUSPEND_STARTS`, held at 3 by the
  owner.
- **The recovery key on the sheet** (§3.4): the tick box beside Print;
  ticked, Print starts the elevated copy (`--bitlocker key`), which
  reads the system drive's first numerical password
  (`GetKeyProtectors` type 3, `GetKeyProtectorNumericalPassword`) and
  answers it over the pipe only this account can open, through buffers
  wiped after each line. A drive with no recovery password is not given
  one: "The drive has no BitLocker recovery key." The key is held in
  the Starter only until the PDF is made, and the PDF is wiped once
  written. The PDF goes into its own `faraday-key-…` folder in the
  system's temporary folder; the Starter overwrites and removes it when
  it exits, and at its next start removes any such folder an earlier
  run left (a crash, a power cut, a file Windows' viewer held open).
- **Turn Secure Boot back on** is listed on Home whenever Secure Boot
  is off (§3.1), and the Start page's step 4 (§3.4) is **Turn Secure
  Boot back on**, before Windows starts, and is the last step.
- **Restart to choose a device** and Windows' **Restart from the
  stick** (*Use a device*) are shown only when Windows reports Secure Boot off at that moment (`UEFISecureBootEnabled` 0 under `HKLM\SYSTEM\CurrentControlSet\Control\SecureBoot\State`, read without elevation, as This computer reads it, §7.1), and hidden when it reads on or cannot be read (owner, 2026-10-14:
  decided). The stick then starts with Secure Boot already off, and the
  power-off line applies (§7.5). The guides' main route chooses the
  stick from the boot menu on the restart that saves Secure Boot off
  (§3.4 step 3).
- The stick reminds at power-off (§7.5).
- **No Microsoft account row** (F48). Nothing in the
  Starter names a Microsoft account, so the Starter does not read the
  account type. The guides' pointer (§6.7) names the Microsoft account
  as a condition.
- **Not on a Mac**: an Intel Mac's Start page (Boot Camp included) has
  no BitLocker row; its steps change Startup Security, not Secure Boot.

The elevated copy's verbs, `--bitlocker key|suspend|resume --pipe
\\.\pipe\faraday-starter-…`, take nothing else; `--record-bitlocker`,
run as an administrator, prints this computer's BitLocker as
`tests/windows/*.wmi` records it, with no recovery password.

### 6.3 What F49 does not bring back

The QR code of the key; the key file and Save; "Or take a photo of
this screen"; Show the recovery key on screen; the typed-back groups;
the TV and remote-session hold (it guarded showing the key); adding a
recovery password to a drive with none; the 0.1.2 marker
(`%LOCALAPPDATA%\faraday\bitlocker-suspended`), the logon reminder
(`--remind`, the Run entry) and the boot-time check that went with
"until resumed" (the count ends a suspension now); the Companion's
parts and `faraday_qr::bitlocker` (F34). The suspension is never a
step: nothing waits on it, and Home has no BitLocker item.

### 6.4 What stays: another encryption product, and Linux

- Another encryption product (VeraCrypt system encryption, a vendor's
  own). The Starter says it found an encrypted drive it does not
  manage, and does not offer the firmware steps until the person
  confirms they have that product's rescue disk or password (**I have
  its rescue disk or password**, on Home's card, §3.1). VeraCrypt is
  detected by its driver, as before. Nothing is shown or kept.
- Linux with a disk unlocked by the TPM (`systemd-cryptenroll
  --tpm2-pcrs=7`): the Linux guide says the passphrase will be asked
  for, and to have it. The Starter detects a `tpm2-device` in
  `/etc/crypttab` and shows that line first.

### 6.5 Batches (F33)

Still to be seen on real Windows, which only it can show: the elevation
prompt on an administrator's and a standard account, the shell property
and WMI on Windows Home with Device encryption, a managed computer's
refusal code, how Windows counts the starts, and whether
the viewer holds the key sheet open when the Starter exits.

The F33 batches and their tests, and F49's batch and its tests: moved to `docs/history/STARTER-LOG.md`.

### 6.6 What is removed

Moved to `docs/history/STARTER-LOG.md`.

### 6.7 The guides' new text

Applied in batch F33d, in `docs/learn/` (the guides are compiled into
every app, so the pages change with the code that reads them).

- **The last step, everywhere.** In the Windows, Linux and maker pages
  the last numbered step, under Afterwards, begins **Turn Secure Boot
  back on** and ends "Do it before Windows starts." (Linux: before the
  computer's own system starts). The maker's way stays between: for
  Dell, "8. Turn Secure Boot back on: turn the computer on, tap F2, set
  Secure Boot to Enabled, then Apply and Exit. Do it before Windows
  starts." "unless Faraday Starter suspended BitLocker" goes from every
  page. The Windows guide's step 7 (Resume BitLocker) goes, so step 6
  is its last.
- **Before you start** (maker pages): "On Linux, have the disk's
  passphrase or recovery key ready if it is unlocked by the TPM." The
  BitLocker sentence goes. The Windows guide's opens (F49): "The
  last step turns Secure Boot back on, before Windows starts. With
  Secure Boot back on, Windows usually starts as before; on some
  computers it asks for the BitLocker recovery key, as If something is
  different says."
- **The pointer**, one paragraph under If something is different, in
  the Windows guide and each maker page. Until F48 it read "If Windows
  asks for the BitLocker recovery key: for a computer set up with a
  Microsoft account, the key is on the account's recovery-key page,
  aka.ms/myrecoverykey; otherwise …". F48 (owner, 2026-10-19) framed the key plainly and named the Microsoft account only as a
  condition, but also said "It is not needed here", which F49 corrects:
  Windows may ask for it even with Secure Boot back on (§6.1). Since
  F49 it reads: "The BitLocker recovery key: with Secure Boot off,
  Windows cannot start without it. Windows may ask for it even after
  Secure Boot is back on, on a computer whose BIOS changes its Secure
  Boot keys when the setting is turned off and on. Suspending BitLocker
  first avoids that: Faraday Starter's Temporarily suspend BitLocker,
  before the restart, suspends it for the next 3 times Windows starts,
  after which Windows turns it back on by itself. To see the key on
  this computer: in Windows, Control Panel → BitLocker Drive Encryption
  → Back up your recovery key; on Windows Home, manage-bde -protectors
  -get C: in a terminal run as administrator; Faraday Starter prints it
  on its sheet when asked to. On a computer signed in with a Microsoft
  account it is also at aka.ms/myrecoverykey. If Windows asks for it
  while Secure Boot is still off: turn the computer off and turn Secure
  Boot back on; Windows then starts without it."
- **The firmware step** (F47): in the Windows, Linux and maker pages
  (not the Mac's), the step that turns Secure Boot off is followed by
  "If the Secure Boot setting cannot be found: exit the BIOS without
  saving changes. Windows starts as before." (Linux: "The computer
  starts as before.").
- **The Windows guide**, If something is different: "Other ways to
  choose the stick" stays, conditioned as the Starter is: "Other ways
  to choose the stick, only when Windows is already running with Secure
  Boot off: Faraday Starter's Restart to choose a device, offered only
  then, and Use a device on the Choose an option screen. The stick then
  starts with Secure Boot already off; turn it back on afterwards, as
  the last step says." Its sentence on backing up the key goes;
  "Without the maker's key to turn Secure Boot back
  on" reads "Without the maker's key: the screen at power-on often
  names it, and the page Another maker lists the common ones."; the
  lines on a computer whose Secure Boot was already off when the
  Starter suspended BitLocker, and on a drive with no BitLocker item,
  go.
- **Surface:** step 6's "Or, in Windows, … Use a device" paragraph
  stays, beginning "Or, when Windows is already running with Secure
  Boot off:", and its sentence on the recovery key goes.
- **"What Secure Boot and BitLocker are"** (`secure-boot-bitlocker`)
  stays, with its first two sections unchanged; its BitLocker section's
  last paragraph reads (F49): "If Windows starts while Secure Boot is
  still off, it asks for the drive's 48-digit recovery key. Turn Secure
  Boot back on before the next start. Some computers' BIOS changes its
  Secure Boot keys when the setting is turned off and on; Windows then
  asks for the key even with Secure Boot back on. BitLocker suspended
  first, as Faraday Starter offers, avoids it. The Windows guide says
  where Windows keeps the key.", and the paragraph before it says "on
  most computers" Windows asks for nothing. Its title stays.
- **"Screens that record"**: under Remote desktop, screen sharing, a
  wireless display, the sentence on the Starter checking for them
  before it shows the recovery key goes; the Starter shows no secret.

## 7. Starting a computer from the stick

### 7.1 What the Starter reads about this computer

Read locally, shown only on this computer, sent nowhere.

| | Windows | Linux |
|---|---|---|
| Maker, model, family | `HKLM\HARDWARE\DESCRIPTION\System\BIOS` (`SystemManufacturer`, `SystemProductName`, `SystemFamily`) | `/sys/class/dmi/id/sys_vendor`, `product_name`, `product_family` |
| Started in UEFI mode | `GetFirmwareType` | `/sys/firmware/efi` exists |
| Secure Boot | `HKLM\SYSTEM\CurrentControlSet\Control\SecureBoot\State\UEFISecureBootEnabled` | the `SecureBoot-8be4df61-…` variable under `/sys/firmware/efi/efivars`, readable by every user |
| ARM | `IsWow64Process2`'s native machine | `uname -m` |
| Encryption | another product only (§6.4); BitLocker is not read (F33) | §6.4 |

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
into `faraday-learn` as every Learn page is. The Starter
carries them (`faraday-guides`), and the website publishes the same text
(§7.4); the desktop app and the stick carry only the address of the
guide for this computer, not its text. Pages:

- *Starting a Windows computer from the stick*: Fast Startup (Windows' Shut down is not a full shut down; restart, or
  turn Fast Startup off), the firmware settings through Settings →
  System → Recovery → Advanced startup, the boot menu, then **Turn
  Secure Boot back on** as the last step, and where Windows keeps the
  recovery key (§6.7).
- *Starting a Linux computer from the stick*, with §6.4's TPM line.
- *What Secure Boot and BitLocker are* (§3.1b). *BitLocker and
  starting from a stick* is removed (F33).
- One page per maker family, published several at a time (§7.6,
  D6). Each has the same sections: Keys; Turn Secure Boot off; Turn Fast Boot off
  (where it hides USB); Start from the stick; Turn Secure Boot back on
  (including restoring factory keys where turning it off cleared
  them), the last numbered step (§6.7); Known quirks. Each ends with a line of where it comes from:
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
  many computers without the boot menu key. It is shown only when Windows reports Secure Boot off at that moment (`UEFISecureBootEnabled` 0 under `HKLM\SYSTEM\CurrentControlSet\Control\SecureBoot\State`, read without elevation, as This computer reads it, §7.1), and hidden when it reads on or cannot be read (owner, 2026-10-14, F33).

Each button asks once ("This restarts the computer now. Save your work first.").

### 7.4 On a phone

The website publishes each guide at
`https://faradayoffline.app/start/<page>`; the Starter shows that
address as a QR code beside the guide. The website's build is the
website repository's work, from these Learn pages (its own HANDOFF).
The address is the only one the Starter contains.

### 7.5 The stick reminds

(owner, 2026-10-12, D8; made the step's backstop by F33, 2026-10-14.
Built, with F33's changes: `faraday-core/tests/all/secure_boot_reminder.rs`.)

**The line.** At power-off, a PC stick that started with Secure Boot
off, or whose Secure Boot state was not read, leads its power-off sheet
with: "Turn Secure Boot back on in the BIOS before starting Windows."
(`power_secure_boot_off`, which read "Secure Boot is off. Turn it back
on before starting Windows." until F33).

- **Where.** The power-off sheet (`docs/FLOWS.md`, Power off), the
  first line under its title, above the guide's code and above Kept,
  Wiped from memory and Then (FLOWS decision 28); in the warning tone,
  at the size of the sheet's body text, not the muted note it was.
- **No press of its own.** The sheet's **Power off** (or its hold, FLOWS
  decision 5) is the press; the line adds none. A last screen after
  Power off, held until a press, was weighed and not taken: one more
  press at every power-off is dismissed by habit, and the sheet is
  already read at the moment the person decides to stop.
- **When.** Secure Boot off: shown. Not read: shown, since a PC stick
  starts with Secure Boot on only once a key of the person's own is
  enrolled (§7.7, later), which reads on. On: not shown.
- **Only the PC stick.** The Pi has no BIOS; Faraday Desktop never
  reads the state; a Mac (maker "Apple Inc.") follows `docs/MAC.md`
  §3.2: a T2 Mac's Startup Security line, and none on another Intel
  Mac, whose firmware has no Secure Boot.
- **The rule on working screens.** A working screen carries labels,
  values and actions. This line is the action the person takes next,
  said at the moment of the press, in one sentence with no reason, as
  F1's line of fact on the boot import sheet is. The reason is in Learn
  (§3.1b) and in the guide behind the code.

**How it is read.** The stick reads the Secure Boot state from
`/sys/kernel/boot_params/data` (byte `0x1ec`: 2 off, 3 on; readable by
every user on x86 and needing no efivarfs, which the stick's kernel
does not build) and the maker from `/sys/class/dmi/id/sys_vendor`.
Checked on a laptop 2026-10-12: byte `0x1ec` = 2 with the SecureBoot
variable 0. The maker's key for the firmware settings is shown beside
the line once the maker has a page.

**The guide's code.** Under the line the power-off sheet shows the QR
code of the guide for this computer (owner, 2026-10-12): one code per
computer, the same the Starter shows on its Start page and its restart
confirmation (§3.4, §7.4), so a person scans one code for the whole
procedure; its last step is Turn Secure Boot back on (§6.7). It is the
maker's page when one matches the firmware's maker, family and model
(`learn::maker`, as the Starter chooses it, read on the stick from
`/sys/class/dmi/id/`), the general Windows page otherwise:
`https://faradayoffline.app/start/<page>`, with no section anchor. Only
on the stick, only when the line is shown; the code is an address, so
the stick reaches nothing.

**The idle power-off** (owner, 2026-10-14: decided). Unchanged, and not
withheld when Secure Boot is off: after a lock and 10 more idle minutes
the stick powers itself off (`docs/FLOWS.md`, Lock). It shows no sheet,
so its reminder is the guides' last step, Turn Secure Boot back on
(§6.7). A long press of the computer's own power
button cuts the power with no sheet; nothing can be shown then.

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

The table the code reads is `faraday_learn::MAKERS`, one row per maker
page: its vendors and families, and for §3.4's Write down (F47) and
the stick's power sheet (F42) the maker's **boot menu key**
(`boot_key`), **BIOS key** (`bios_key`) and the **Secure Boot
setting**'s place (`secure_boot`, the menu path as the page names it;
"business:" or "desktops:" before a second path of the same maker).
The text is `faraday-strings`'. A test in `faraday-guides` checks each
value against the maker's page: every key named in one numbered step
(a held key as "Hold" it), every part of the setting's path in one
paragraph of the Steps.

### 7.7 Keeping Secure Boot on (later)

Faraday can make its own Secure Boot keys and sign `BOOTX64.EFI`. On a
firmware that lets a db key be added beside Microsoft's, both Windows
and Faraday start with Secure Boot on and there is nothing to turn back
on. A later batch writes that guide, per maker, never suggesting the
Microsoft keys be removed (on some laptops that stops the graphics
firmware loading). Surface has no custom keys. Not in the first
release (§12, D9).

Enrolling a key does not escape BitLocker: the db's contents are part
of the TPM's PCR 7 measurement, so adding a key changes it exactly as
turning Secure Boot off does, and Windows asks for the recovery key at
its next start unless BitLocker is suspended. Until F33 the guard ran
once before the enrolment and Resume after it. F33 removed the guard
(owner, 2026-10-14: decided): when batch S enrols Faraday's key, its
guide tells the person to have the recovery key ready, with the same
pointer as §6.7's; nothing more. When S is scheduled is decided then,
by the owner. What the guide
removes is the second firmware visit every time, not the first.

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
  desktop app's, `Faraday::held`).
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

Transfer is the desktop app's (`faraday-core/src/transfer/mod.rs`): Send
shows a file as animated codes for the device's Scan; Receive reads the
device's codes with the camera and saves each whole file into the
Downloads folder. On Linux the camera is `faraday-v4l2`, as the
desktop app's. On Windows it is `faraday-mediafoundation`
(`faraday/shells/mediafoundation`), built as `faraday-avfoundation`
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
faraday/panels/ on its boot partition, and `config.txt` ends with
`include faraday-panel.txt`. Making a card writes `faraday-panel.txt`
as a copy of the chosen profile into the boot partition image before
it is written (`faraday-fat` edits the in-memory image); the writer
reads back what it wrote. A profile holds the firmware lines (timings,
overlays, pins) and the shell's three facts that `panel.conf` bakes
into `/etc/faraday/args` today (dpi, touch controller name, touch
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
(docs/history/PLAN.md §3); a camera module works on the legacy camera stack, which
reads Camera Module v1 and v2, not v3. The Starter's Pi card page says
which. HDMI with no touch needs a USB keyboard and mouse, which the Pi
forbids today (§12, D2). *Replace Faraday, keep data* for a Pi card
follows when the Pi has its upgrade (docs/history/PLAN.md §5.5).

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
download that asks for elevation and writes raw disks matches
heuristics that change monthly, and the guard's design (no boot-time task)
existed partly to give them less to match; F33 removed the suspension. So the plan for the day it
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
saved file, a restart. Since F33 nothing touches BitLocker (§6). Uninstall takes back what install did (§5.4).
On an Apple Silicon Mac (MAC §4.5a), the Faraday start-up volume and
the shrunk macOS container, which the Starter's *Remove Faraday from
this Mac* takes back.

## 11. Tests

Tests, by batch, state what a person sees or can do: the disks offered,
refused and their order (recorded udisks2 answers on Linux, as a
stand-in `/sys` serves `faraday-grant`'s tests; recorded answers on
Windows); no disk chosen until tapped; a stick that is not the
confirmed one refused; keep data offered only for a Faraday stick,
refusing the same build string and warning on an older one, and never
writing past the boot partition (against a regular file holding a
stick image, as `stick-boot.sh` allows); Home's checklist showing the
next step for each state (no stick, stick attached, Windows with
Secure Boot off; F33); install writing
exactly §5.2's files into a temporary home, and uninstall removing
exactly them; the firmware steps closed by another encryption product
until its rescue is confirmed; F33's and F49's tests
(`docs/history/STARTER-LOG.md` §6.5); the
guide chosen for a maker string; a payload file that does not match
its manifest refused; the stick's power-off line for each Secure Boot
byte.

## 12. Decisions

All taken by the owner on 2026-10-12 in discussion, except D11, taken
on 2026-10-19 (#30). Where a decision changed v0.1, `docs/history/STARTER-LOG.md` §13 says
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
- **D7 BitLocker:** the guard: the key shown and confirmed, then
  suspended until resumed; since F21 an optional backup. Superseded by
  F33: no BitLocker step (§6); since F49 offered, never required: a
  suspension for three times Windows starts, Resume, and the key on the
  printed sheet.
- **D8 The stick's power-off reminder:** yes, as §7.5; since F33 the
  backstop for turning Secure Boot back on.
- **D9 Keeping Secure Boot on:** a later batch.
- **D10 Names:** "Faraday Starter" to people (F16, which changed v0.2's
  "Faraday"), "Faraday Desktop" for the desktop app; `faraday-starter` in the
  code.
- **D11 FARADAYDATA on Windows:** decided (owner, 2026-10-19, #30;
  `docs/DECISIONS.md` F50). FARADAYDATA keeps its drive letter, so a person
  can put a PSBT or any file on a Faraday stick from Windows; Faraday
  reads what Windows writes there and never lists, reads or counts
  Windows' or macOS's own files, told by name (§4.4).
- **D12 Home:** a checklist that leads with this computer's next step,
  then the rows (§3.1); since F91 the checklist is inside the This
  computer card and the rows become destinations and cards (§3.1d).
- **D13 Linux writer:** udisks2 over D-Bus, no root in the Starter;
  the pkexec path only if a tried distribution lacks udisks2 (§2).
- **D14 Linux download:** a `.tar.gz`, so the executable bit survives
  and nobody runs `chmod` (§1.2).
- **D15 Keep data reads the stick's version** from the boot partition
  and applies the stick's own upgrade rules (§3.2, §4.1).
- **D16 Guard order:** the key in hand first, then suspend.
  Superseded by F33.
- **D17 Resuming:** the person's act through the Starter, with a logon
  reminder for an installed copy; no boot-time task. Superseded by
  F33, with no Resume kept (§6.3).
- **D18 The key on a phone:** a plain-text QR code and "Or take a
  photo of this screen"; Save to a file only when a drive for it is
  attached. Superseded by F33.
- **D19 Acceptance for Windows Home:** on a Home computer of the
  owner's, with the owner, after Pro passes in a virtual machine.
  Superseded by F33 (`docs/history/BITLOCKER-ACCEPTANCE.md`'s header).
- **D20 The desktop app on Windows:** yes, from batch C, carried by
  the Windows Starter and offered by its Install (§2, §3.5). Built in
  C1: the payload has a folder per system, each Starter offers only its
  own system's desktop app, and Windows install, Try and uninstall
  mirror Linux's (§5.2, §5.3). Its Starter parts are superseded by F30
  (2026-10-14): the Starter carries, installs and runs no desktop app,
  and the desktop app on Windows is a developer's build.
  F83 (2026-10-23) brings C1's payload, Open and install back, without
  the check box (§5.3, batch DS).
- **D21 tools/flash-stick:** removed once the Starter writes a
  developer's own image (`--image`, §1.1), in batch L.
- **D22 The stick's version on Windows** (owner, 2026-10-19, option
  (e)): an elevated read when keep data is chosen, one
  administrator prompt, and "version not read" kept apart from "0.1.0 or
  earlier" (§4.1). A copy of the version on FARADAYDATA or in the GPT was
  rejected: what Windows reads without elevation is not updated by keep
  data or Upgrade a Faraday stick.
