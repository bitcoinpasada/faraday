# The hub app

**Status:** spec v0.1 · 2026-10-12 · decided by the owner in discussion
(`docs/DECISIONS.md` F11). Not built. §12 lists what the owner has
not yet confirmed, each with the default this spec assumes.

The problem it solves: getting Faraday onto a stick or a Pi card, and a
laptop to start from it, takes a terminal, `dd`, a firmware menu and a
Secure Boot setting, and on Windows a wrong step can leave the computer
asking for a BitLocker recovery key nobody wrote down. The hub app is
one file a person downloads for their operating system and opens. It
makes a Faraday stick or Pi card, shows how to start this computer from
the stick and puts it back as it was, moves files to and from the device
by QR code, opens the desktop app for testing, and installs itself if
the person wants it in the Start menu or app menu.

"Hub app" is this document's name for it. `docs/DESIGN.md` §5 already
has a reusable screen called Hub (a Navigate screen); the two are
unrelated. People see the app as **Faraday**.

## 0. Rules for the implementing agent

- `CLAUDE.md` and `CLAUDE.local.md` apply in full: every build and test
  through `local/remote-run.sh`; every dimension a token; labels, not
  prose, on working screens (explanations go in Learn pages); tests for
  user-facing behaviour only; `just` once at the end of a batch; no
  `rm`; no commits; no agents.
- Batches in §11's order, one review and one completion report each. A
  batch that needs an owner decision from §12 does not start until the
  owner has confirmed it.
- §6 (the BitLocker guard) is a hard requirement. Nothing in batch W
  ships until §6.6's acceptance run passes in full.
- Nothing in the hub app reaches a network. No HTTP client, no update
  check, no telemetry, no crash reporting. The one link it shows (the
  guides on the website, §7.4) is a QR code a phone opens.
- Completion report: items done, each with its tests and whether they
  pass; what was left out and why; docs changed; renders to look at. No
  restating the spec.

## 1. What ships

### 1.1 Release files

One release carries every file, all listed in one `SHA256SUMS` signed
as today (`README.md`, "Verifying a release"). The release notes and
the website lead with the two hub downloads; the rest are under "Other
downloads".

| File | What it is |
|---|---|
| `Faraday-X.Y.Z-Windows.exe` | the hub app for Windows (x86_64) |
| `Faraday-X.Y.Z-Linux` | the hub app for Linux (x86_64) |
| `faraday-desktop-X.Y.Z-linux-x86_64` | the desktop app alone (today's `faraday-X.Y.Z-linux-x86_64`, renamed) |
| `faraday-stick-X.Y.Z-x86_64-uefi.img.xz` | the stick image |
| `faraday-pi-X.Y.Z-….img.xz` | the Pi image; its name follows §9's board and panel decision |

- Images are published as `.xz` only. `SHA256SUMS` lists each `.xz` and
  also the `.img` inside it, under the `.img` name, so `sha256sum -c
  --ignore-missing SHA256SUMS` checks a download, and after
  decompressing, the image; the hub app shows the `.img` line for what
  it writes.
- No dev image is published, as a file or inside the hub app. Developers
  build dev images (`just dev=1 faraday-stick-image`) and write them
  with `tools/flash-stick`, which stays the developer's tool.
- The README's "Verifying a release" gains: verifying on Windows
  (`Get-FileHash -Algorithm SHA256` for the sums, Gpg4win for the
  signature), and why the Windows hub app is unsigned (§10.2).
- Writing a plain image without the hub app: Raspberry Pi Imager and
  balenaEtcher take the `.xz` as it is; other tools after decompressing
  it (7-Zip on Windows, `xz -d` on Linux). The README says each in one
  line.

### 1.2 One Linux file for every distribution

The 0.1.1 desktop app needs glibc 2.43 (checked 2026-10-12: `objdump
-T` on the release binary shows `GLIBC_2.43`), which
Ubuntu 24.04 (2.39) and Debian 12 (2.36) do not have. The hub app and
the desktop app for release are built in a pinned container against
glibc 2.31 (Debian 11), so one x86_64 file runs on Ubuntu 20.04 and
later, Debian 11 and later, Fedora 32 and later, and Arch. This is the
`just faraday-linux-bin` the README says is not set up yet. Every other
library the apps use at run time (X11 or Wayland, xkbcommon) is loaded
from the desktop the app runs on, as today.

No `.deb`, `.rpm`, Flatpak, Snap or AppImage. A browser drops the
executable bit from a download, so on Linux the person marks the file
executable once (the website and the README show the file manager's
"Allow executing" and `chmod +x`). Formats for software centres are a
later decision, if people get stuck there.

### 1.3 What is inside the hub app

The hub app carries its payload: the bytes of the release's `.xz`
files, unchanged, and a manifest naming each with its two SHA-256 sums
(of the `.xz` and of the `.img` or binary inside) and the build ID it
carries. The payload is built in by the hub's build from `out/release/`
and is the same bytes the release publishes beside it.

| Hub app | Carries |
|---|---|
| Linux | stick image, Pi image, desktop app (as `.xz`) |
| Windows | stick image, Pi image |

At start the hub app checks every payload file against its manifest and
refuses to write one that does not match. That catches corruption, not
tampering: the defence against tampering is `SHA256SUMS` and its
signature over the hub app file itself, which About (§3.7) explains.

Decompression is a pure-Rust xz decoder with an entry in `docs/deps/`
(the choice is batch R's, after reading the candidates). Faraday does
not link liblzma: xz-utils 5.6.0 and 5.6.1 carried a backdoor in 2024,
and a C decompressor is the wrong place to take that risk again. The
decoder streams: an image is decompressed as it is written, never into
a temporary file.

Estimated size: Linux about 40 MB (11 + 12 MB images, 4 MB desktop app,
the hub itself about 13 MB), Windows about 36 MB.

## 2. How it is built

- `faraday/faraday-hub`: the hub app's state, screens and decisions, no
  I/O, tested as `faraday-core` is, through actions in and commands out.
  It draws with `faraday-core::ui` and `osk-ui`, so it looks like
  Faraday and obeys the same tokens.
- `faraday/shells/hub`: the window (winit and softbuffer, as the
  desktop shell; both run on Windows), the payload, and one module per
  operating system for what differs: listing disks, writing them,
  installing, reading this computer's facts, restarting into firmware.
- **Transfer and Learn are the desktop app's own screens.** The hub
  app holds a `Faraday` in online mode, limited to Transfer and Learn,
  and draws it when one of them is open; Back returns to the hub's
  Home. The desktop app's `--transfer` already opens it this way. Batch
  R checks that `faraday-core` can be held this way and adds what is
  missing, without changing what the desktop app does.
- **The privileged writer is the same binary.** `--write` with the
  mode, the payload file, the device and the confirmed serial and size,
  as `tools/flash-stick/write.sh` takes them. On Linux the hub app runs
  `pkexec <its own path> --write …`; polkit asks for the password. On
  Windows it starts itself elevated (`ShellExecuteExW` with `runas`)
  and the elevated copy reports progress over a named pipe only the
  person's account can open. The writer checks the device again before
  it writes (§4.3) and never takes a path from anything but its
  arguments.
- On Windows, winit already brings in `windows-sys`; the Win32 calls
  the shell needs (disks, volumes, registry, shortcuts, elevation) go
  through it, and no other Windows crate is added.
- Builds: Linux in the §1.2 container; Windows cross-compiled for
  `x86_64-pc-windows-gnu` in a pinned container on the build machine.
  No hosted CI (`CLAUDE.md`). Batch W tries the Windows hub app on a
  Windows computer, natively.
- Strings are in `opensigner/opensigner-core/src/strings/en.rs` with a
  `hub_` prefix, as `CLAUDE.md` requires. `CLAUDE.md`'s screen rule
  gains: the hub app's screens are this document's §3, built from
  DESIGN §4's components.

## 3. Screens

Every screen uses DESIGN §4's components and §5's intents. Titles and
labels are the defaults; the build may tighten wording, not change
what a screen does.

### 3.1 Home (Navigate)

Title: the wordmark. A card for this computer, then the rows.

- **This computer:** maker and model (§7.1); on Windows and Linux,
  whether Secure Boot is on; on Windows, whether the system drive is
  encrypted. On a computer that cannot start a Faraday stick (§7.1:
  an ARM computer, or one that started in legacy BIOS mode), one line
  says so; Make a stick stays, for another computer.
- **Make a Faraday stick** → §3.2
- **Make a Pi card** → §3.3
- **Start this computer from the stick** → §3.4
- **Transfer** → the desktop app's Transfer (§2)
- **Try the desktop app** (Linux only) → opens it (§5.3). Its row says
  "Test seeds only".
- **Install** or **Installed** → §3.5
- **About** → §3.7
- **Learn** → the hub's Learn pages (§7)

### 3.2 Make a Faraday stick

1. **Which stick?** (Navigate) The USB disks attached now, as
   `tools/flash-stick` lists them: maker and model, size, device,
   serial, partitions with names and sizes, what is mounted, and
   "Faraday stick" for one whose partition table is a Faraday stick's.
   The list follows sticks plugged in and pulled out. A disk the
   computer uses (§4.2) is shown dimmed with the reason and cannot be
   chosen. With one stick it is chosen; with more, none is until one is
   tapped.
2. **What to write?** (Choose)
   - *New Faraday stick*: the whole stick; everything on it is lost.
   - *Replace Faraday, keep data*: only on a Faraday stick. The boot
     partition is written; the data partition, with its vaults and
     settings, is not read and not written. One line under it states
     the fact the Learn page explains (§4.4).
3. **Confirm** (Record) the stick (maker, model, size, device, serial),
   what is written (the build ID and the `.img` SHA-256), and the
   action button: *Erase and write* or *Write*. Then the system's own
   prompt (§2).
4. **Writing** (Progress): written of total, then *Reading back*.
5. **Done** (Result): "The stick can be pulled out." and the next
   step: **Start this computer from the stick** (§3.4). On failure the
   reason the writer gave, and nothing claims success.

### 3.3 Make a Pi card

As §3.2, with two choices before the card: **Which Pi?** and **Which
screen?** (§9). The card list also offers the computer's own SD card
reader (§4.2). *Replace Faraday, keep data* is offered once the Pi
image can be replaced that way (§9.5).

### 3.4 Start this computer from the stick

The guide for this computer (§7), opened at its maker's page, with the
steps in order and the buttons the operating system allows:

1. On Windows, the BitLocker step (§6), before anything else. Until it
   is done, steps 2 and on are shown and their buttons are not offered.
2. **Restart into firmware settings** (§7.3).
3. The firmware steps for this maker: turn Secure Boot off; turn Fast
   Boot off where it hides USB.
4. **Start from the stick:** the maker's boot menu key, or Windows'
   *Use a device*.
5. **Afterwards:** turn Secure Boot back on before starting Windows.
6. A QR code of this page's address on the website (§7.4), to read the
   steps on a phone while the computer is in its firmware.

### 3.5 Install and uninstall

**Install** (Choose): one check box, Linux only, *Also install the
desktop app, for testing with test seeds only*, unchecked; then
**Install**. Done shows where it went. When the running copy is newer
than the installed one, the row reads **Update the installed Faraday**.

**Installed** opens a Record of what is installed and where, with
**Uninstall**, which asks once. §5 says what each does.

### 3.6 Transfer and Learn

The desktop app's screens (§2). On Windows, Transfer offers Send until
batch C adds the camera (§8).

### 3.7 About

Version; each payload file with its build ID and SHA-256; the release
key's fingerprint as the README prints it; how to check the hub app
file itself with `SHA256SUMS` and its signature; on Windows, why the
file is unsigned (§10.2).

## 4. Writing a stick or a card

### 4.1 What the writer does

As `tools/flash-stick/write.sh`, which is the reference: refuse unless
the device is still the stick that was confirmed (serial and size);
unmount what the desktop mounted from it; write; flush; read the whole
write back and compare; ask the system to read the partition table
again. *Keep data* writes the boot partition at its offset after the
same partition-table checks as `faraday/image/stick-boot.sh`, and
checks the partition table is unchanged afterwards. The source is
decompressed from the payload while it is written, and the SHA-256 of
the decompressed stream must equal the manifest's before the writer
reports success.

### 4.2 Which disks are offered

- A whole disk on USB, removable or hot-pluggable; or, for a Pi card,
  an SD card in the computer's own reader (Linux `mmcblk*`; Windows bus
  types SD and MMC).
- Never a disk with a partition the computer uses: on Linux mounted
  anywhere but `/run/media`, `/media` or `/mnt`, or swap; on Windows
  the disk holding the Windows, boot or page-file volume.
- Never a disk smaller than the image.

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
write the data partition whatever the hub app does, and Windows mounts
it with a drive letter. The vaults there are encrypted; the settings
file is not. The Learn page "Replacing Faraday on a stick" says this
and points to Settings → Upgrade a Faraday stick, which upgrades a
stick from another stick without an online computer.

## 5. Install, run without installing, uninstall

### 5.1 Running without installing

Opening the downloaded file runs the hub app. It writes nothing to the
computer except what the person asks for (a stick, a card, a received
file, the desktop app's cache under §5.3).

### 5.2 Installing

Per user, never needing administrator rights.

| | Linux | Windows |
|---|---|---|
| The hub app | `~/.local/share/faraday/faraday` | `%LOCALAPPDATA%\Programs\Faraday\Faraday.exe` |
| The desktop app, if ticked | `~/.local/share/faraday/faraday-desktop` | not offered |
| Menu | `~/.local/share/applications/app.faradayoffline.Faraday.desktop`, and `…FaradayDesktop.desktop` "Faraday desktop (testing)" | Start menu shortcut `Faraday` |
| Icon | `~/.local/share/icons/hicolor/256x256/apps/app.faradayoffline.Faraday.png` | in the `.exe` |
| Uninstall entry | — | `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Faraday`: name, version, publisher "Faraday", icon, size, `UninstallString` = `"…\Faraday.exe" --uninstall` |

The menu file names are reverse-DNS, so they never collide with a
developer's own `faraday.desktop`.

### 5.3 The desktop app from the hub app

Try the desktop app runs the installed desktop app if there is one;
otherwise the hub app decompresses it from its payload into
`$XDG_CACHE_HOME/faraday/X.Y.Z/` (default `~/.cache/faraday/X.Y.Z/`),
checks its SHA-256 against the manifest, and runs it. Every start
checks the cached copy again. The desktop app opens on its own; it is
the same program as the plain release file, and can also be opened
without the hub app.

### 5.4 Uninstalling

Removes exactly the files and entries §5.2 lists for this computer, and
the §5.3 cache. It never touches anything the person made: received
files, the desktop app's `~/faraday-sticks` and settings. On Windows
the running copy cannot delete itself; the uninstaller finishes by
starting a short command that removes the folder once the hub app has
exited.

## 6. The BitLocker guard (Windows)

**The requirement (owner, 2026-10-12):** nobody who follows the hub
app's steps is ever asked for a BitLocker recovery key. This section is
how, and §6.6 is how it is proved.

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

Before §3.4 shows its firmware steps or offers its buttons, the hub app
finds out whether any Windows volume on this computer is protected, and
if one is, two things must both be done, in this order:

1. **Protection is suspended.** The hub app suspends BitLocker on every
   protected operating-system volume, with no restart count, so it
   stays suspended until resumed (`Win32_EncryptableVolume.
   DisableKeyProtectors` with a count of 0; batch W confirms against
   Microsoft's documentation and §6.6 that 0 means until resumed, as
   `Suspend-BitLocker -RebootCount 0` does). While suspended, Windows starts without
   the key whatever the firmware and the TPM report: Secure Boot off,
   Secure Boot on with other keys, a cleared TPM.
2. **The recovery key is in hand.** The hub app reads the volume's
   recovery password (`GetKeyProtectorNumericalPassword`) and shows it,
   the way Windows' own "Back up your recovery key" does. The person
   writes it down or prints it, or saves it to a file on a drive other
   than the encrypted one, and then types two of its eight groups back,
   chosen by the hub app. A volume with no recovery password gets one
   added first (`ProtectKeyWithNumericalPassword`), and that is the one
   shown. The key is never written anywhere by the hub app unless the
   person chooses Save, and never to the Faraday stick.

Suspending alone already prevents the prompt; the key in hand covers
what the guard cannot see (a firmware update, a repair shop, a
protector resumed by someone else).

All of this is done by the elevated writer (§2) through WMI's
`root\CIMV2\Security\MicrosoftVolumeEncryption`, which answers in
numbers in every Windows language and exists on Home editions with
Device encryption. Nothing parses `manage-bde`'s localised text.

Knowing whether the guard is needed does not need elevation: Home
reads the volume's BitLocker protection state through the shell
property `System.Volume.BitLockerProtection`. If that cannot be read,
the hub app treats the drive as protected.

If the person declines the elevation prompt, the guard stays open: the
firmware steps are not shown and the buttons are not offered. The page
says why in one line and offers the step again.

### 6.3 Resuming, only when it is safe

The hub app registers one startup task, as SYSTEM, named "Faraday:
resume BitLocker", at the moment it suspends. At each Windows start the
task checks that Secure Boot is on (`Confirm-SecureBootUEFI`) and only
then resumes protection (`EnableKeyProtectors`) and deletes itself.
Resuming seals the key to how this start was measured, so a start with
Secure Boot on as before resumes cleanly. If Secure Boot is still off,
the task does nothing and protection stays suspended, which costs
protection, never access. The hub app's Home shows "BitLocker is
suspended" while it is, with **Resume BitLocker** once Secure Boot is
on again.

Secure Boot turned back on with different keys (a firmware that
cleared its keys and needed "restore factory keys") also resumes
cleanly, because the task seals to whatever this start measured.

### 6.4 Where the guard cannot reach

- A person who changes firmware settings without the hub app. Every
  other place the steps appear (the website's guides, the stick's
  power-off screen, §7.4–§7.5) puts "Do the BitLocker step in the
  Faraday app first" before any firmware step.
- Another encryption product (VeraCrypt system encryption, a vendor's
  own). The hub app says it found an encrypted drive it does not
  manage, and does not offer the firmware steps until the person
  confirms they have that product's rescue disk or password. VeraCrypt
  is detected by its driver; anything else falls under "drive not
  readable as BitLocker".
- Linux with a disk unlocked by the TPM (`systemd-cryptenroll
  --tpm2-pcrs=7`): the Linux guide says the passphrase will be asked
  for, and to have it. The hub app detects a `tpm2-device` in
  `/etc/crypttab` and shows that line first.

### 6.5 What the person sees

One page, before anything else on Windows, when a drive is protected:
the state ("Drive C: is encrypted with BitLocker"), **Suspend BitLocker
and show the recovery key**, then the key and the two groups to type,
then "Done: Windows will start without the key until Secure Boot is
back on." Learn: "BitLocker and starting from a stick".

### 6.6 Acceptance, before batch W ships

In a Windows 11 virtual machine with a virtual
TPM and Secure Boot on (QEMU with OVMF's Microsoft-key variables and
`swtpm`, or Hyper-V), with BitLocker on the system drive, for Home and
Pro:

1. Through the hub app: guard, then Secure Boot off in the firmware,
   start the Faraday stick image, power off, Secure Boot on, start
   Windows. **No recovery prompt**; protection resumed (the task ran and
   is gone).
2. As 1, but start Windows with Secure Boot still off. **No recovery
   prompt**; protection still suspended; Home says so; turning Secure
   Boot on and starting again resumes it.
3. As 1, with the firmware's keys reset to default before turning
   Secure Boot on. **No recovery prompt.**
4. The elevation declined: the firmware steps are not shown.
5. Without the hub app, Secure Boot off and Windows started: the
   recovery prompt appears. This shows the virtual machine reproduces
   the problem, so 1–3 prove something.

Each run is recorded in the completion report: the steps, and a
screenshot of Windows' first screen after each start.

## 7. Starting a computer from the stick

### 7.1 What the hub app reads about this computer

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

### 7.2 The guides are Learn pages

Each guide is a Learn page in `docs/learn/faraday/boot/`, round-tripped
into `osk-learn` as every Learn page is, so the hub app, the desktop
app and the stick all carry them, and the website publishes the same
text (§7.4). Pages:

- *Starting a Windows computer from the stick*: BitLocker first (§6),
  Fast Startup (Windows' Shut down is not a full shut down; restart, or
  turn Fast Startup off), the firmware settings through Settings →
  System → Recovery → Advanced startup, *Use a device*, then Secure
  Boot back on.
- *Starting a Linux computer from the stick*, with §6.4's TPM line.
- *BitLocker and starting from a stick* (§6).
- One page per maker family. Each has the same sections: Keys; Turn
  Secure Boot off; Turn Fast Boot off (where it hides USB); Start from
  the stick; Turn Secure Boot back on (including restoring factory keys
  where turning it off cleared them); Known quirks. Each ends with a
  line of where it comes from: the maker's support page and the date it
  was read, and the model it was tried on, or "Not yet tried on a
  computer".
- *When the stick does not start*: not in the boot menu (Fast Boot,
  USB boot off, a USB-C port that does not boot, CSM), a black screen,
  a firmware password set by an employer.

### 7.3 Buttons that remove steps

- **Restart into firmware settings**: Windows `shutdown /r /fw /t 0`
  (elevated); Linux `systemctl reboot --firmware-setup`. Both work on
  any maker in UEFI mode, so nobody has to catch a key at power-on to
  reach Secure Boot.
- **Restart to choose a device** (Windows): `shutdown /r /o /t 0`,
  Windows' advanced startup, whose *Use a device* starts the stick on
  many computers without the boot menu key. Offered once Secure Boot is
  off.

Each button asks once ("Save your work first: this restarts the
computer").

### 7.4 On a phone

The website publishes each guide at
`https://faradayoffline.app/start/<page>`; the hub app shows that
address as a QR code beside the guide. The website's build is the
website repository's work, from these Learn pages (its own HANDOFF).
The address is the only one the hub app contains.

### 7.5 The stick reminds

At power-off, a stick that started with Secure Boot off says so, with
the maker's key for the firmware settings when the maker has a page:
"Secure Boot is off. Turn it back on before starting Windows." The
stick reads the Secure Boot state from `/sys/kernel/boot_params/data`
(byte `0x1ec`: 2 off, 3 on; readable by every user on x86 and needing
no efivarfs, which the stick's kernel does not build) and the maker
from `/sys/class/dmi/id/sys_vendor`. Checked on a laptop 2026-10-12:
byte `0x1ec` = 2 with the SecureBoot variable 0.

### 7.6 Maker families and where to start

The first release covers these families (§12, D6). The keys are
starting points from common knowledge, **every one to be checked
against the maker's own documentation** before its page is written,
and the page says which were tried on a computer.

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

A maker not in the table, or not read, gets the general Windows or
Linux page, whose firmware route (§7.3) needs no key.

### 7.7 Keeping Secure Boot on (later)

Faraday can make its own Secure Boot keys and sign `BOOTX64.EFI`. On a
firmware that lets a db key be added beside Microsoft's, both Windows
and Faraday start with Secure Boot on and there is nothing to turn back
on. A later batch writes that guide, per maker, never suggesting the
Microsoft keys be removed (on some laptops that stops the graphics
firmware loading). Surface has no custom keys. Not in the first
release (§12, D9).

## 8. Transfer

Transfer is the desktop app's (`faraday-core/src/transfer.rs`): Send
shows a file as animated codes for the device's Scan; Receive reads the
device's codes with the camera and saves each whole file into the
Downloads folder. On Linux the camera is `opensigner-v4l2`, as the
desktop app's. On Windows, batch C adds a camera module on Media
Foundation, written as `opensigner-v4l2` is (the calls declared in the
module, every `unsafe` block with its `SAFETY` comment, a `docs/deps`
note for anything added); until then Windows Transfer offers Send only.

## 9. Pi compatibility

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
serves every panel. A card made from the plain image (without the hub
app) starts with the 2.8 inch profile, and the README says which one
line to change on its boot partition for each other screen.

Batch P starts by checking each mechanism on the Pi 3 (does the
firmware take `include` as this needs; does the shell get the command
line) and writes down what it found before building.

### 9.4 Boards (§12, D3)

1. Pi 3B, 3B+ and Zero 2 W: one 32-bit image; the Zero 2 W board is in
   the tree already.
2. Pi 4: its own kernel and firmware, the same mechanism.
3. Pi 5: later, if at all. It is 64-bit only, its display goes through
   KMS (`DRM` is forbidden on every Faraday image), and its cameras
   need libcamera.

### 9.5 Screens and cameras

The first profiles are the screens tried on a Pi (§12, D1). A USB
webcam works on every Pi, because the Pi's USB already allows cameras
(PLAN.md §3); a camera module works on the legacy camera stack, which
reads Camera Module v1 and v2, not v3. The hub app's Pi card page says
which. HDMI with no touch needs a USB keyboard and mouse, which the Pi
forbids today (§12, D2). *Replace Faraday, keep data* for a Pi card
follows when the Pi has its upgrade (PLAN.md §5.5).

## 10. Security

### 10.1 What the hub app is trusted with

The hub app runs on an online computer and writes the system a person
will trust with their keys. It carries the release's own bytes, checks
them against its manifest, shows their sums, reads every write back,
and has no network code. The writer runs elevated for one write and
takes its device, serial and size as arguments it checks again. The
hub app never reads a Faraday stick's data partition.

### 10.2 Unsigned on Windows

The Windows hub app is not code-signed: a code-signing certificate
names a verified person or company, and Faraday publishes under a
pseudonym (owner, 2026-10-12, for privacy). Windows shows "Windows
protected your PC" on first run; *More info* → *Run anyway* opens it.
The website, the README and About say this, and how to check the file
with `SHA256SUMS` and its signature instead. Batch W runs the hub app
under Microsoft Defender on a Windows computer and reports any
detection.

### 10.3 What it does to the computer

Only what the person asks for: a written stick or card, an install, a
saved file, the BitLocker suspension and its startup task (§6), a
restart. Uninstall takes back what install did (§5.4).

## 11. Batches

| Batch | What | Needs |
|---|---|---|
| R | Release groundwork: the glibc 2.31 container for the desktop app; `.xz` images; the new names; `SHA256SUMS` with the `.img` lines; the hub's payload build; the xz decoder and its `docs/deps` note; README "Verifying a release" for Windows | no decision |
| L | The hub app on Linux: crates (§2), Home, Make a stick, Install and uninstall, Try the desktop app, Transfer and Learn, About; the writer through `pkexec` | no decision |
| G | Guides: the Learn pages of §7.2 with their sources; detection on Linux; Restart into firmware settings on Linux; the stick's power-off reminder (§7.5); the website item in its own repository | D6, D8 |
| W | The hub app on Windows: window, install and uninstall, disks and writing, detection, the BitLocker guard with §6.6's acceptance, the firmware buttons, Transfer Send; Defender run | D7, D11 |
| C | Windows camera for Transfer Receive | D4 |
| P | Pi: profiles, boards, Make a Pi card, layout checks at the new sizes; tried on hardware | D1, D2, D3 |
| S | Keeping Secure Boot on (§7.7) | D9 |

Tests, by batch, state what a person sees or can do: the disks offered
and refused (a stand-in `/sys` on Linux, as `faraday-grant`'s tests
do; recorded answers on Windows); a stick that is not the confirmed one
refused; keep data offered only for a Faraday stick and never writing
past the boot partition (against a regular file holding a stick image,
as `stick-boot.sh` allows); install writing exactly §5.2's files into a
temporary home, and uninstall removing exactly them; the BitLocker
guard's order (no firmware step before both parts are done; resume only
with Secure Boot on) against a recorded WMI; the guide chosen for a
maker string; a payload file that does not match its manifest refused;
the stick's power-off line for each Secure Boot byte.

## 12. Decisions the owner has not confirmed

Each has the default this spec assumes. The owner confirms or changes
them before the batch that needs them (§11).

- **D1 First Pi screens:** the Waveshare 2.8 inch DPI and the Raspberry
  Pi Touch Display (7 inch DSI, first version), then others as they are
  tried on a Pi.
- **D2 HDMI on the Pi:** not in the first round; it needs USB keyboard
  and mouse on the Pi, with the stick's new-keyboard approval code.
- **D3 Boards:** Pi 3B, 3B+ and Zero 2 W first, Pi 4 next, Pi 5 later.
- **D4 Windows Transfer:** ships with Send; Receive follows batch C.
- **D5 Where the screens are specified:** here, built from DESIGN §4,
  with `CLAUDE.md`'s screen rule naming this document.
- **D6 Makers:** every family in §7.6 in the first release, each marked
  tried or not.
- **D7 BitLocker:** the guard as §6 describes, including suspending,
  showing the key, and the self-deleting startup task.
- **D8 The stick's power-off reminder:** yes, as §7.5.
- **D9 Keeping Secure Boot on:** a later batch.
- **D10 Names:** "Faraday" to people; `faraday-hub` in the code.
- **D11 OSKDATA on Windows:** decided after batch W reports what
  Windows writes to a stick.
