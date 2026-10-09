# Faraday — plan

(Formerly openfaraday; renamed 2026-10-04.)

**Status:** draft v0.1 · 2026-10-04 · decisions agreed with the owner in
conversation; nothing below is built yet.

Faraday is an offline key appliance that boots from a USB stick on an
x86-64 UEFI PC, or from its card on a Raspberry Pi, and runs entirely
from RAM. It is a fork of
OpenSignerKit / OpenSigner and keeps OpenSigner's trust model: one Rust
binary on a kernel that contains what the device needs and nothing else,
no network stack, no shell, no login, bit-identical builds. It takes
the name of Faraday OS, the owner's amnesic offline Arch desktop, which is
deprecated (2026-10-04): Faraday OS was a general desktop, and Faraday
does a fixed set of jobs. Where this plan borrows from Faraday OS (its QR
transfer, its USB policy, its palette), it says Faraday OS.

The vault format is specified separately in [`docs/VAULT.md`](docs/VAULT.md),
the scanner in [`docs/QR.md`](docs/QR.md), and every flow and where it
leaves the person in [`docs/FLOWS.md`](docs/FLOWS.md).
OpenSigner's own planning, design and UX documents (`docs/PLANNING.md`,
`docs/DESIGN.md`, `docs/UX.md`) remain the reference for everything this
file does not change.

---

## 1. What it does

| Area | Jobs |
|---|---|
| Vaults | Create a vault with one to four passphrases, each opening its own slot; unlock after the boot stick is removed; hold Bitcoin keys, wallets, notes and recovery sheets, entries (passwords and TOTP secrets), GPG keys and Secure Boot keys; seal and write back. |
| Wallets tab | Wallet creation, restore, backup and spending flows, run on OpenSigner's library crates for every piece of cryptography, extended to every wallet kind OpenSigner signs for, with wallet reassembly at the centre: keys and wallets in open vaults load into it, other keys and signatures arrive by QR or file. Design: `docs/WALLETS.md`. |
| GPG | Create an Ed25519 certification key with an Ed25519 signing subkey, export the public certificate, make detached signatures, make a revocation certificate, renew expiry, write a paperkey backup. |
| Secure Boot | Generate PK, KEK and db keys and certificates, write enrolment files, sign and check `BOOTX64.EFI`. |
| QR | One scanner and one sender for the Wallets tab and the QR transfer tab: the union of Faraday OS's file transfer (fountain-coded `ur:bytes`, BBQr, the `faraday-file-v1` envelope) and the Wallets tab's scanner (wallet URs, raw transactions, diagnostics, routing). Feature list and state: `docs/QR.md`. |
| Passphrases | EFF dice tool, reused from OpenSigner. |
| Settings | Keyboard layout, display scale, idle lock, power off. |

**Not in Faraday:** a desktop, a file manager, other applications,
LUKS, encrypted file blobs, image viewing (images are only read for QR
codes, in the disk process: `docs/QR.md` §4), screenshots,
download verification, age, hardware baselines, drive formatting,
partitioning, wiping or cloning, exFAT, a clipboard for secrets, and
writing any decrypted data to USB.

A stick visit does copy files from one stick to another (§5.4): that is
for backup copies of vault, backup and other files, not file
management. Public and sealed files go as they are; a secret, or text
Faraday cannot tell about, only past the secret sheet's warning, one
file at a time.

## 2. Relationship to OpenSigner

- This repository is a full clone of `maxmoney21m/opensignerkit` with its
  history. The original is the git remote `upstream`; upstream changes
  are merged, not copied.
- **No upstream file is edited.** Faraday uses the `osk-*` crates as a
  library; the Wallets tab (`docs/WALLETS.md`) replaces OpenSigner's app, so
  `opensigner-core` and `osk-shell-api` need no changes. The workspace
  manifest is the one shared file.
- Where Faraday needs a variant of an upstream part (the image boards,
  the stick shell, `osk-ui` as `faraday-ui`), it is a copy under
  `faraday/` recording the upstream commit it came from;
  `just upstream-diff` lists upstream changes to the originals since.
- `faraday-core` names OpenSigner's `osk-*` types directly, so an upstream
  change that breaks Faraday breaks there and nowhere else (`docs/WALLETS.md` §1).
- Changes that would help OpenSigner too (a key held outside the core,
  Secure Boot signing of the stick, runtime display density) are offered
  upstream.

## 3. Platforms

Two boards from OpenSigner's image tree, one application.

**PC: `x86_64-uefi`, booted from a USB stick.** The owner has confirmed the
unmodified OpenSigner stick boots on their laptop. The stick is
OpenSigner's layout: an EFI system partition holding one file,
`EFI/BOOT/BOOTX64.EFI`, which is the kernel with the whole root
filesystem inside it, and a data partition (FAT16 as made). The firmware
loads the entire system into RAM; the stick is removed before any vault
is unlocked (§5).

**Raspberry Pi: the `pi3` board, booted from its card.** The Pi firmware
loads the kernel, with the root filesystem inside it, from the card into
RAM, and nothing needs the card after that. The card is therefore treated
as the PC treats its boot stick: it is removed after boot, before any
vault is unlocked, and it or any other SD card can be put back for a
stick visit (§5.4) to save a vault or move files. The card is the Pi's
only storage. It has two partitions: the boot partition, labelled
`OSKBOOT` (OpenSigner's Pi image leaves it unlabelled) so the grant
helper refuses it, and an empty FAT partition for exchanging files. SD
partitions are handed out by the same grant helper as USB partitions,
under the same rules (§4.3). Whether the Pi 3's slot reports removal and
insertion reliably is tested on hardware before the flows rely on it.

The Pi board keeps what OpenSigner's Pi board forbids (decided
2026-10-08): no `USB_STORAGE` or `SCSI`, so no USB sticks, and no `HID`,
so no USB keyboard or mouse. The touch panel is the whole input device,
and the USB ports carry a camera and nothing else. The Pi drives the
panel only, not HDMI.

On a PC, SD cards are read through USB card readers. The x86 kernel keeps
`MMC` forbidden, as OpenSigner has it, because `MMC` is also how many
laptops attach their internal eMMC disk.

Differences a person sees on the Pi:

- The 2.8" panel is the `small` class, so every flow is paged (§9.1).
- The Pi 3 has no verified boot. Secure Boot keys can be made and PC
  images signed on a Pi, but nothing signs or checks the Pi's own boot;
  Faraday opens on OpenSigner's existing statement that only a card
  the person flashed should be used.
- RAM is 1 GB on a Pi 3, which bounds the Argon2id cost of any vault it
  creates or opens (`docs/VAULT.md` §3.1).
- Every screen says "SD card" where the PC says "stick" ("SD card
  visit", "Lock and use SD card", "no SD card" on Home), and draws the SD
  card glyph where the PC draws a drive (`docs/PLANNING.md` §16.144). The
  stick shell tells the app which at start.

The Android, iOS and desktop shells stay in the tree for upstream merges
and are not Faraday targets.

## 4. Trust model and processes

### 4.1 What runs

| Process | User | Privilege | Reads untrusted input |
|---|---|---|---|
| BusyBox init (PID 1) | root | full; idle after boot | no |
| `faraday-grant` | own user | `CAP_CHOWN` only | `/sys` attributes, nothing read from a stick |
| `faraday-disk` | `ofdisk` (uid 201) | none | FAT16 and FAT32 filesystems on USB and SD partitions; PNG and JPEG files, read for QR codes |
| `faraday` (the app) | `opensigner` (uid 200) | none | camera frames (QR), input events, file contents passed by the disk process, vault files |
| `faraday-boot` (not built yet, §5.5) | `ofboot` (uid 203) | none | `/sys` attributes; a boot partition's release string, bounded; reads no FAT |

`rcS` runs once as root at boot: it mounts `proc` and `sysfs`, starts
`faraday-grant` and the disk process's restart loop, and mounts no
storage; init then starts the app loop. Three root processes remain
after boot, and none reads input: init, waiting to run `poweroff -f`
when the app loop ends; the app loop's shell (`inittab`), which starts
the app as `opensigner` with no new privileges and starts it again when
it exits for a lock; and the disk process's restart loop (`rcS`), which
starts `faraday-disk` as `ofdisk` the same way and again a second after
it ends.

### 4.2 The kernel

OpenSigner's x86 and Pi configurations, with these changes on both, each
enforced by the board's `kernel.forbidden` and `kernel.required` lists
(the Pi's further changes are in §3):

- **Forbidden:** `FAT_FS`, `VFAT_FS`, `MSDOS_FS`, `EXFAT_FS` and every
  other filesystem except the in-memory ones the initramfs needs. No
  filesystem on a stick is ever parsed by the kernel.
- **Required:** `EFI_PARTITION` and `MSDOS_PARTITION`, so partitions
  appear as their own device nodes and the whole-disk node is never
  handed out (§4.3).
- **Command line:** `init_on_free=1 init_on_alloc=1` added, so memory a
  process frees is zeroed before reuse (§5.3).
- **Forbidden:** `COREDUMP`, so no process's memory is written to a file
  (§5.3).
- **Command line:** `usbcore.authorized_default=2`, so only devices on
  hard-wired internal ports are authorised without the helper (§4.6).
- **USB drivers:** storage, HID, UVC and hubs only on x86; UVC and hubs
  only on the Pi (§3, §4.6).
- `RTC_HCTOSYS` is not needed: Faraday displays no time.
- **Hardening** from Faraday's review, taken upstream as
  `docs/PLANNING.md` §16.140 and carried into Faraday's board copies:
  memory zeroed on allocation and free by default, heap and copy
  hardening, stopping on corruption and on an oops, Yama (`ptrace_scope`
  3) and Lockdown forced to confidentiality; io_uring, AIO, System V
  IPC, the keyring, cross-memory attach and GPIO from userspace out; on
  x86 the 32-bit and 16-bit system calls, raw HID, virtio, the SCSI
  CD-ROM driver and the vendor HID drivers out. `rcS` mounts `/proc`,
  `/sys` and `/dev` `nosuid,noexec` with hidden pids and sets
  `kptr_restrict` 2. Not yet: Landlock for the app and the disk process,
  and `efi=disable_early_pci_dma` (HANDOFF item 63).

### 4.3 Storage access (option B)

Device nodes the kernel creates for a new disk belong to root. Exactly one
privileged action happens per partition:

1. `faraday-grant` polls `/sys/class/block` (no netlink, no uevent
   helper, as OpenSigner does).
2. A new partition is considered only if its disk is on the USB bus, or
   is an SD card on the Pi, and it is a partition node, never a
   whole-disk node.
3. The helper decides from `/sys` alone and reads nothing from the
   stick. A partition whose GPT name (`PARTNAME`, parsed by the kernel)
   is `OSKBOOT` is the boot partition, and so is the first partition of
   the Pi's own card (`mmcblk0p1`), which is MBR and has no names but a
   fixed layout. The boot partition stays root's and is never handed
   out. The disk process checks again: it refuses any partition whose
   FAT label is `OSKBOOT`, which also covers an older stick, named
   `esp`, plugged in as a second stick.
4. Any other partition is handed to `ofdisk`, mode `0600`, but only while
   the app has published that it is in the clean state (§5.1). On removal
   the node goes away with the device.

`faraday-grant` starts as root, keeps `CAP_CHOWN`, drops to its own
user, empties its bounding set and sets no-new-privileges. `CAP_CHOWN`
lets a process take ownership of any file, so the helper's safety rests
on what it reads: `/sys` attributes the kernel wrote, compared as
strings. It has no seccomp filter (§12, decision 8). It also applies the USB device
policy (§4.6), using the same capability on the device's authorisation
files.

`faraday-disk` reads and writes FAT16 and FAT32 in Rust, unprivileged, and
speaks to the app over a pipe with a small request format: list a
partition, read a file, write a file. It writes by creating a new file,
reading it back, comparing, and renaming it into place; it never modifies
a file in place. A compromised disk process can hand the app a doctored
file (the app already treats file contents as hostile) and can read the
Outbox (public) and sealed vaults (ciphertext). It never holds a secret.

The boot partition is never handed out, so the running system cannot
rewrite its own kernel. OpenSigner has the same property by never mounting
it. The one exception, not built yet, is upgrading a stick (§5.5): while
the app is in that flow, a partition named `OSKBOOT` goes to
`faraday-boot`, never to the disk process, and only for a raw copy of
the Faraday that is running. This requires both images' `genimage.cfg` to name or label the boot
partition `OSKBOOT`: the ESP's GPT name and FAT label on the PC, the boot
partition's FAT label on the Pi, where it is also the card's first
partition.

### 4.4 Untrusted input, the complete list

Camera frames (OpenSigner's vendored `rqrr`, fuzzed), file contents read
from sticks (PSBTs and other OpenSigner inputs, entry import text, the
boot stick's settings file, §5.2),
vault files (only the fixed header is read before an authentication tag
is checked; Argon2id cost is capped before allocation), FAT metadata (in
the unprivileged disk process), GPT and MBR partition tables (in the
kernel), `/sys` attributes (in the grant helper), USB descriptors and input events (in the kernel and the
app, §4.6), and, once §5.5 is built, a boot partition's release string
(in `faraday-boot`, bounded).

### 4.5 What this does not defend against

A malicious Faraday binary: it could hide a secret in the ciphertext
or the random-looking empty slots it writes. The defence is the binary
itself: reproducible builds checked by independent builders, as
OpenSigner requires. Firmware, hardware implants and cold-boot attacks on
RAM are out of scope, as they are for OpenSigner and Faraday.

### 4.6 USB devices and input

Faraday OS's USBGuard policy allows a device whose interfaces are all in an
allowed set, which includes both HID and storage. A stick that also
reports a keyboard is therefore accepted there, and Faraday OS's README says
the policy checks what a device says it is, not its firmware. No policy
can tell a real keyboard from a device built to imitate one. Faraday
uses three layers instead, and the last is what stops an imitation.

1. **No drivers.** The x86 kernel carries drivers for four kinds of USB
   interface and nothing else: mass storage, HID keyboards and pointers,
   UVC cameras and hubs. The Pi's carries only UVC and hubs (§3). A network adapter, serial adapter, audio device,
   printer or vendor device has no driver to bind to, whatever it
   reports.
2. **Interface authorisation.** The kernel's own USB authorisation, the
   mechanism USBGuard is built on, with no daemon: `usbcore.authorized_default=2`
   on the command line authorises only devices on hard-wired internal
   ports (a laptop's own keyboard, touchpad and camera), and `rcS` sets
   each root hub's `interface_authorized_default` to 0. For every other
   device, `faraday-grant` authorises interfaces one by one: storage,
   HID, video and hub, and no others. **A device with a storage interface
   has only that interface authorised; its HID interfaces never are.** A
   device that re-enumerates is a new device and is decided again.
3. **New input must prove a person is at the screen.** The shell reads
   each input device separately (OpenSigner's evdev code already does), so
   the app knows which device every key and click came from. Trusted
   without asking: the laptop's built-in keyboard (PS/2 or a hard-wired
   USB port), its touchpad, and the Pi's touch panel. Any other keyboard,
   including one plugged in before boot, has its input ignored until it
   types a short random code shown on screen; any other pointer, until a
   person approves it with input already believed (**Use this pointer**,
   pressed on the built-in keyboard or touchpad, or on a keyboard that has
   typed its code). A held-back device's clicks and keys never reach the
   app as input, so only a person at the screen can approve it. A device
   cannot see the screen, so it cannot pass. A laptop's
   built-in touchscreen is on I2C and trusted as its touchpad is; its
   positions are scaled by the ranges its HID report descriptor declares,
   read from sysfs (`faraday/shells/stick/src/hid.rs`), since the shell
   makes no `EVIOCGABS` ioctl.

Devices that appear after boot (a keyboard, once authorised; a webcam)
get the owners `/etc/mdev.conf` gives those present at boot, from
`faraday-grant`: no hotplug helper runs, and without this the app could
not read a keyboard plugged in later.

The screen that asks names the device as it describes itself, says that
the description is a claim, and offers **Ignore this device**. Ignored
devices are listed in Settings until they are unplugged.

## 5. The stick rule and the lock cycle

### 5.1 The rule

**No removable storage (USB, or SD on the Pi) is accessible while any
secret is in memory.** A secret
is an unlocked vault slot, a key loaded in OpenSigner, a GPG or Secure
Boot private key, or a passphrase being typed. The app is in the *clean
state* only after a fresh start with none of these; only then does it
publish the clean marker that `faraday-grant` checks.

Applies to every stick and card, not only the boot medium. A reader with
no media in it (a built-in USB card reader with no card) has no
partitions and does not count.

### 5.2 Inbox and Outbox

- **Inbox:** files read from a stick while clean — PSBTs, unsigned
  `BOOTX64.EFI`, entry import text, vault files. Held in RAM.
- **Outbox:** results that are public or encrypted — signed PSBTs, public
  keys and certificates, GPG signatures and revocation certificates,
  Secure Boot enrolment files, the signed `BOOTX64.EFI`, sealed vaults.
  Held in RAM and written on the next stick visit.
- **Settings** are not an Outbox file. `faraday-settings.txt` on the
  boot stick, plain text so the theme, scale and keyboard layout apply
  before the passphrase, is read once a session, at boot. A line is
  taken only if its key is known and its value is one Settings offers;
  never for an idle time, the signed-amount memory's seal and the
  network are not read from it, so a file someone edited cannot make a
  fresh boot less protected than the defaults. A stick visit offers it
  as its first row, ticked on the boot stick when the settings changed,
  and writes it over the settings file there
  (`faraday-core/src/stick_settings.rs`).

Both live under `/run/faraday`, a RAM directory readable only by
uid 200. Nothing decrypted is ever placed in the Outbox. Both are shown on one
**Files** screen, where each Inbox file carries the action that uses it
(`docs/FLOWS.md`, decision 1).

### 5.3 Locking

Lock seals every unlocked vault (re-encrypts the opened slots, copies the
others byte for byte; `docs/VAULT.md` §5), places the sealed vaults in the
Outbox, and exits the app with a restart code. The app loop starts a
fresh process; with `init_on_free=1` the old process's memory is zeroed
by the kernel. A forgotten copy of a secret cannot outlive the process
that held it.

The app loop runs as uid 200 under `setpriv --nnp`:

```
while /usr/bin/Faraday "$@"; [ $? -eq 75 ]; do :; done
```

and init runs `poweroff -f` when it ends.

**What a lock wipes and what it keeps (audited 2026-10-06).** The lock
button on a vault's screen is this same session lock; there is no lock of
one vault alone.

- Wiped: the process's whole memory. Before it exits, the app drops the
  session (keys, wallets, every flow in progress, open vault contents);
  seed words, key material and decrypted records are zeroized as they
  drop (`Zeroizing`, `osk_crypto::Secret`). Then the process ends and the
  kernel zeroes each page it frees (`init_on_free=1`). The kernel has no
  `SWAP`, `HIBERNATION`, `DEVMEM`, `PROC_KCORE`, `KEXEC` or `DEBUG_FS`
  (`kernel.forbidden`), so no page reaches storage and nothing reads
  another process's memory. `faraday-disk` runs as uid 201 and can read
  neither the app's memory nor `/run/faraday`.
- Kept, in `/run/faraday` for the next process: the Inbox, the Outbox,
  and the `kept` set (settings, the signed-amount memory of
  `docs/WALLETS.md` §3.3, the Spend tab's page, the idle time, and which
  Outbox files are secrets let out after the warning). The signed-amount
  memory is a privacy trace (txids and amounts), not a key.
- Wiped as they pass, since they are not in the app's memory: the bytes
  of every file `faraday-disk` reads or writes and every frame on its
  pipes, at both ends (`faraday-files::proto`), because that process
  lives on across locks; and the shell's raw input buffers once decoded.
  Typed words and passphrases are `SecretText`
  (`faraday-core/src/secret_text.rs`), which wipes its old buffer when it
  grows, where a `Zeroizing<String>` would leave it behind.
- Dropped from the Inbox at Lock, before the boxes are saved: a seed's
  words file (copied in, or `<picture>-words.txt` read from a SeedQR
  picture), a FROST carry file (`.osk`), an entries file and any plain
  text file (the kind added to a vault as a note). They stay on the stick
  they came from. Without this they would reach the next, clean, process,
  where a flaw in how the app reads a file from a later stick would run
  as uid 200 and could send them out to it. Kept: PSBTs, descriptors and
  other public files, and vault, `.oskb` and KeePass files (ciphertext).
- The Outbox keeps what the person put there, an unprotected secret
  included, since writing it is what they asked for; a visit leaves it
  unticked (`docs/FLOWS.md` decision 6).
- `COREDUMP` is forbidden on every board (`kernel.forbidden`), so no
  crash writes a process's memory to a file.
- The framebuffer holds the last frame until the next process draws, at
  once; only the kernel and the app read it.
- The desktop app (online, for testing) restarts inside one process: there
  only the app's own zeroizing applies. It starts on testnet; reaching
  mainnet, chosen or by loading a mainnet wallet or transaction, first
  shows **Not air-gapped**, which stays until **I understand** (once per
  session).
- **Panic key.** Super and S held together for two seconds, on any
  believed keyboard, end the stick shell at once: the panel is painted
  black, the app is dropped (its zeroizing runs), nothing is saved, and
  init powers off. Settings shows the chord beside Power off; the
  desktop app has none.

### 5.4 Flows

**Boot.** The first time the app sees the boot medium in a power-on, it
reads every file on its data partition into memory: vault files into the
Inbox, encrypted; every other file into a holding area apart from the
Inbox, a PNG as what its QR codes hold (a SeedQR as its words), a file
Faraday reads as nothing it knows as a File, to sign or send as codes.
A keyboard or pointer waiting to be believed (§4.6) is asked about
first, whether it was seen before the stick or after; then a sheet over
Home says
how many files were copied and what they are, and **Remove the stick to
start the import** (on the Pi, the card). Once no removable partition
remains, the same sheet lists the medium's vaults, each with **Unlock**
(the Unlock screen, which comes back to the sheet); every wallet found in
the files, the pictures and the open vaults, once each by descriptor,
ticked, with its shape, whether the keys present can sign for it ("Can
sign", "k of n keys here · m more needed", "Watch-only") and the files
that carry it and its keys; the keys no listed wallet uses, ticked; and
every file for the Inbox, PSBTs and vaults ticked, an open vault's file
always kept. A line states that files not chosen are wiped from memory
and that bringing one in later means inserting the stick again.
**Import** loads the chosen wallets with their keys and the chosen keys,
moves the chosen files into the Inbox, wipes the rest, and leaves Home.
**Import later**, or a tap beside the sheet, leaves the files waiting;
the sidebar's Sticks row and Home's Sticks and import cards open the
sheet again. A lock wipes what was not imported, and a later insertion
of the boot medium is an ordinary stick visit. Unlocking, loading keys
and scanning are never offered while a stick or card is present: a
camera frame may picture a seed. The camera on withdraws the clean
marker as a secret does, so a stick inserted while it is on is held
back, nothing read, and arrives once the camera is off.

**A stick inserted while unlocked.** Nothing is handed out. The app sees
the disk in `/sys` and shows a sheet: what will be sealed (vaults with
unsaved changes), what will be wiped (keys loaded in OpenSigner), what is
waiting in the Outbox. **Lock and use stick** locks (§5.3); the fresh
process, now clean, lets the stick through. **Not now** leaves the stick
untouched and shows "Remove the stick to keep working".

**A stick visit.** One screen does both directions: write the Outbox
(each file read back and compared), save sealed vaults over the files they
came from (`docs/VAULT.md` §6), and pick files into the Inbox. On the Pi
the boot card or another SD card is the one visited, and the screen is
**SD card visit**. It ends on **Remove the stick** ("Remove the SD card"
on the Pi), then **Unlock again**.

**Copying from one stick to another.** The visit's write list, **Write
to the stick**, also lists the Inbox's files under **From the Inbox**,
except those the stick shown already holds (same name and size), all
unticked. So stick A → visit, copy in → stick B → visit, write; with
both sticks in at once the visit shows one and then the other. A
public or sealed file (PSBTs, wallets, descriptors, certificates,
vaults, `.oskb`, KeePass) is written as it is, and Select all ticks it.
A secret kind (words, seed parts, entries, a FROST carry file) is
ticked only through the secret sheet, never by Select all: it may go
into the open vault, or, once its line is ticked, is ticked to write
unprotected; it never enters the Outbox, which outlives a lock. Text
and a file Faraday does not read go the same way, the sheet saying
"Faraday cannot tell whether this is a secret". A PNG copied in is
kept as it came, beside what its codes hold, and written out as the
same file; it takes the exposure of what its codes hold (a SeedQR or
words a secret, an encrypted backup sealed, text unknown, no code read
like a file Faraday does not read). A lock drops from the Inbox every
file, picture or not, that may hold a secret. A file over 18 MiB is
listed and not read. The boot import keeps only what a picture's
codes hold, and its holding area is wiped unless imported, so copying
from the boot stick takes Import or a later visit.

**File-based PSBT signing.** Visit: pick the PSBT into the Inbox. Remove.
Unlock. Sign in OpenSigner; Save puts the signed PSBT in the Outbox. Lock.
Visit: the Outbox is written. QR signing needs no stick.

### 5.5 Upgrading a stick

Not built yet (decided 2026-10-08, `docs/PLANNING.md` §16.142). Faraday
copies itself onto another Faraday stick: the boot partition of the
stick it started from is written over the other stick's, and that
stick's data partition, with its vaults and settings file, is not
touched. The stick holding vaults then never needs a computer other than
the one Faraday runs on.

**The flow.** Settings → **Upgrade a Faraday stick** (**Upgrade a
Faraday SD card** on the Pi, whose medium is named as §3 says). If anything is
unlocked, the sheet for a stick inserted while unlocked (§5.4) comes
first, so the copy happens in the clean state. Then:

1. "Insert the stick Faraday started from." Its boot partition is read
   whole into memory, and kept only if it holds this Faraday (below).
2. "Remove it. Insert the stick to upgrade." The sheet shows the version
   on that stick and the one to be written. **Upgrade** writes, reads
   back and compares.
3. Done; the stick is removed.

With both sticks in at once there is no swap: the one holding this
Faraday is the source. Taking the boot stick out after boot and putting
it back for step 1 changes nothing. Every stick has the same partition
UUIDs (`genimage.cfg`), so the source is told by what it holds, never by
when it was inserted.

**Which stick is the source.** The image build writes the Faraday
version and commit into the kernel's release string
(`CONFIG_LOCALVERSION`), which the bzImage carries uncompressed in its
setup header. A boot partition is accepted as the source only if it
contains the running kernel's release string, as `/proc/version` gives
it. This identifies the version; it is not a signature, and a stick made
to carry the same string would pass. Such a stick, present at the
machine, could as well have been the stick it booted from. The target's
version is read the same way and shown: a stick that carries no such
string (0.1.0 and earlier) shows as an earlier version, and a target
newer than the running Faraday is warned about.

**Who writes.** `faraday-boot`, user `ofboot` (uid 203), started by
`rcS` as the disk process is. It copies the partition raw and reads no
FAT on either stick; apart from `/sys`, the only bytes it reads from a
stick are the release strings, bounded and printable only. The app asks
it two things over a pipe: read the source, and write the copy to a
partition. It never hands it bytes, so a compromised app can at most
write the Faraday that is running. `faraday-grant` hands a partition
named `OSKBOOT` to `ofboot`, never to `ofdisk`, and only while the app
publishes an upgrade marker beside the clean marker (§5.1). Outside the
flow no boot partition is handed out, as before (§4.3).

**Limits.** The target's boot partition must be at least the source's
size: 48 MB on the PC, about 11 MB of it used now. A release whose boot
partition has to grow cannot be copied onto an older stick; that needs
repartitioning, which moves the data partition, and is not planned. A
stick pulled during the write does not boot; its data partition is
intact, and the upgrade is run again.

**Secure Boot.** The copy is byte for byte, so a source whose
`BOOTX64.EFI` carries the owner's db signature passes it on. Faraday
does not sign inside the upgrade (owner, 2026-10-08): the app would then
hand `faraday-boot` bytes of its own making. A new release is signed
once, through a spare stick, so the vault stick never meets an online
computer:

1. On a computer: write the release onto a spare stick, and put its
   `BOOTX64.EFI` on the spare's data partition too.
2. Boot the current, signed vault stick. Read the file from the spare
   into the Inbox, unlock the vault holding the db key, sign (§8), lock,
   and write only the signed file back to the spare.
3. On the computer: copy the signed file onto the spare's boot
   partition.
4. Boot the spare and upgrade the vault stick from it.

A Learn page and the README take the person through these steps; the
working screens do not explain them.

**The Pi.** The same flow over the card's first partition
(`mmcblk0p1`, 32 MB), swapping cards in the one slot. After the PC.

**Test.** QEMU boots the new image with an older stick attached,
upgrades it, boots it, and checks the version and that its data
partition is byte for byte what it was.

**For development.** `faraday-stick-image` also leaves the boot
partition's own image (`esp.vfat`) as
`out/stick/faraday-x86_64-uefi[-dev]-boot.vfat`, and
`just faraday-stick-boot dev=/dev/sdX` writes it over the partition
named `OSKBOOT` alone, so a test stick keeps its vaults and settings
across builds. It refuses unless the disk is on USB, its first partition
is named `OSKBOOT` and is large enough, and the device is typed back; it
reads back and compares. It puts the whole stick on the build computer,
so it is not offered to users.

## 6. Vaults and the Wallets tab

### 6.1 Vaults

Format, slots, passphrases, sizes and write-back: `docs/VAULT.md`.

A vault holds typed records, not files, so there is no file explorer. The
**vault contents** screen of an open vault lists its kinds (Bitcoin keys,
wallets, entries, notes, GPG keys, Secure Boot keys) with counts; a kind
opens its list, and an item opens its details. Secret fields (words, a
password, a TOTP secret, a note's text) sit in a hold-to-reveal panel and
show only while held. Each kind has its own Add action; each item has its
actions (open in the Wallets tab, export a public key, sign a file) and **Hold
to delete from vault**.

### 6.2 The OpenSigner and Wallets tabs

The **OpenSigner tab** stays until the Wallets tab is robust enough to
deprecate it. It needs no change to OpenSigner: faraday-core acts as
its shell, opens a vault key in it by sending the key as a scanned payload
(`Event::Scanned`, which runs OpenSigner's own load flow), and imports a
key it exports as an `.oskb` encrypted backup into a vault. Its PSBTs and
other public results go to the Outbox through `Command::WriteFile`; its
plain note export is refused there, since nothing decrypted leaves.

### 6.3 The Wallets tab

The tab, its architecture, wallet reassembly and every flow are in
`docs/WALLETS.md`. What concerns the vault:

- `faraday-core` reads and writes `faraday-vault` directly; both are
  Faraday crates inside the secret boundary. No upstream interface is
  involved.
- At unlock, every wallet and every key marked "load at unlock" in the open
  slots loads into the Wallets tab, and keys are matched to the wallets that use them.
- Keys and wallets added in a session offer **Save to vault**; locking lists
  what was not saved.
- **Remove from session** (the default) is separate from **Delete from
  vault** (a hold naming the vault).
- BIP-39 passphrases are not stored unless the person turns on storing for
  that key.
- A MuSig2 or FROST round that must survive a lock is sealed into the vault,
  bound to its transaction (`docs/WALLETS.md` §4.2).
- `.oskb` encrypted backups from OpenSigner devices import into a vault.

### 6.4 Entries

An entry holds a title, username, password, URL, notes and an optional
TOTP secret (`otpauth://` URI). Faraday stores entries; it does not
generate TOTP codes. Entries arrive by typing, by scanning a QR code with
the camera (TOTP setup codes are QR), or from an Inbox text file of
`otpauth://` URIs and `field: value` lines. CSV and KeePass import are not
supported. Passwords are shown only in OpenSigner's hold-to-reveal secret
panel; there is no clipboard.

## 7. GPG (write-only subset)

Nothing parses OpenPGP data from outside. Inputs are the person's own keys
from a vault.

| Job | Notes |
|---|---|
| Create | Ed25519 certification primary, Ed25519 signing subkey, one or more user IDs, expiry. Secret material goes to a vault slot. |
| Export public certificate | ASCII-armoured, v4 packets. Outbox. |
| Sign | Detached signatures over Inbox files, SHA-256 or SHA-512. Outbox. |
| Revocation certificate | Made at creation; also on request. Outbox. |
| Renew | New self-signatures with a new expiry; export the updated certificate. |
| Paperkey | The secret parts in `paperkey`'s text format, checked against `paperkey` output. Shown as text or QR, or written to the Outbox only as part of a sealed vault. |

Not built: encryption subkeys, keyrings, web of trust, smartcards,
importing GnuPG secret keys, exporting a subkey-only secret key for an
online machine.

Tests: every packet Faraday writes is checked by `gpg --import` and
`gpg --verify` (and `sqv`) against vectors in `tools/vectors/pgp/`.
Version-4 fingerprints need SHA-1, which is used for fingerprints only.

## 8. Secure Boot

- Generate PK, KEK and db as RSA-2048 keys with self-signed X.509
  certificates and an owner GUID. Private keys go to a vault slot.
- Write enrolment files: EFI signature lists and signed `.auth` updates
  for PK, KEK and db, in the Outbox.
- Two enrolment policies, as Faraday has. **Windows-compatible** (the
  default, for a shared computer) adds Microsoft Corporation KEK CA 2011
  and KEK 2K CA 2023 to KEK, and Microsoft Windows Production PCA 2011
  and Windows UEFI CA 2023 to db. It does not add the third-party UEFI CA
  (2011 or 2023) or the Option ROM UEFI CA 2023. **Own keys only** adds
  nothing. The Microsoft certificates are public; the image carries them
  with the SHA-256 of each recorded in the source. Under either policy, a
  graphics card or storage controller whose firmware is signed by the
  third-party authority may not start, and the Secure Boot screen says
  so. Enrolment itself happens in the
  firmware's setup screen; Faraday never writes EFI variables
  (efivarfs is not in the kernel).
- Sign `BOOTX64.EFI` (Authenticode, SHA-256) from the Inbox. The first
  signing of an image requires its SHA-256 to match a build record whose
  fingerprint the person checked independently; OpenSigner's builds are
  bit-identical, so the record is reproducible. The signed file goes to
  the Outbox and is copied onto the boot partition on another computer —
  the running system cannot write its own boot partition (§4.3), and the
  firmware checks the signature, so an untrusted copy step changes
  nothing. The stick that goes to that computer is a spare, never the
  one holding vaults; the vault stick is then upgraded from the spare
  (§5.5).
- Check an existing signature against the db certificate in a vault.
- Test: `tools/stick-qemu.py` extended to enrol generated keys in OVMF,
  boot the signed image, and confirm an unsigned or altered image is
  refused. This is OpenSigner's §15 item 52. Built 2026-10-07 as
  `faraday/tools/sb-ovmf-check.py` (`just faraday-sb-ovmf`), over
  systemd-boot rather than the stick: for each policy, OVMF in Setup
  Mode takes the three `.auth` updates (systemd-boot writes them, as a
  firmware setup screen does), then with Secure Boot enforced runs the
  image signed with the db key and refuses it unsigned and altered by one
  byte. In Setup Mode the firmware may take KEK and db without checking
  their signatures; those are checked by `openssl` in
  `faraday-sb/tests/judged.rs`. Signing the stick's own `BOOTX64.EFI` is
  the same code path over a larger image.

## 9. Interface

### 9.1 Two layouts from one flow

Each Faraday flow is declared once in faraday-core as an ordered
list of field groups and a summary of what it will do. Two layouts render
the declaration:

- **`wide`** (laptop and desktop): each group is a step card on one
  screen. One card is open at a time; **Continue** closes it to a one-line
  summary of the choice and opens the next. Clicking a closed card, or
  its row in the summary panel, reopens it. The summary panel beside the
  cards lists each choice as it is made ("Your choices"), with "Will
  create" and the primary action at its foot. The summary is the review;
  there is no separate confirm screen. Actions that cannot be undone keep
  OpenSigner's hold. (Settled on the prototype, 2026-10-04: showing every
  group open at once was too crowded.)
- **`small` and `mobile`**: one group per page, ending on the summary as a
  review page.

Content, wording and validation are identical in both. A layout changes
placement, never content, as OpenSigner's design rule has it. One-line
field hints are allowed on every class.

Desktop forms add what OpenSigner's components lack: text fields with a
caret, selection and Tab order; passphrase pairs with a live match check
and show/hide; segmented choices, dropdowns and an "Advanced" disclosure
on `wide` only; a destination picker built from the Outbox and visited
sticks.

**Unlock cost.** The Create vault form asks where the vault will be
opened (this computer, other PCs, Raspberry Pi 5, 4, 3, Zero 2 W) and
offers the largest Argon2id preset that fits the weakest choice. The
summary panel shows the memory needed to open and the unlock time on this
computer, measured at boot; warnings name the machine a cost may not fit.
The form states that passphrase length matters more than cost. Presets,
limits, warnings and the reference timing table: `docs/VAULT.md` §3.1.
A Learn page, "Unlock cost", explains memory as a hard limit, time as a
soft one, the bits arithmetic, and why the cost cannot change later. The on-screen keyboard appears only when no physical keyboard is
found. **Generate with dice** fills both passphrase fields directly.

### 9.2 Look and feel

- **Typography:** Inter for text, JetBrains Mono for data. The font baker
  (`tools/fontbake`) gains kerning pairs and tabular figures, and text is
  positioned at fractional pixels.
- **Scale:** a display-scale setting saved on the boot stick, with a
  first-boot guess from the resolution, replacing the fixed
  `opensigner.dpi=160` (OpenSigner §15 item 54). Layouts fill the screen
  with a capped form column, instead of a fixed 960×640 frame.
- **Palette:** Faraday OS's approved palette (`#11161c` background, `#83d8ef`
  accent), dark and light, as tokens.
- **Depth:** two or three surface levels and 1 px hairlines; soft shadows
  only on popovers, pre-blurred and cached as nine-slice images.
- **Icons:** one line icon set (Lucide, Phosphor or Tabler), baked at
  build time.
- **Motion:** 120–180 ms ease-out on focus, press and disclosure; a page
  slide on small classes; only changed regions redrawn (the firmware
  framebuffer has no vsync); a reduce-motion setting.
- **Data:** fingerprints in grouped mono; the Argon2id cost as a bar
  against free RAM; the Secure Boot chain (PK → KEK → db → `BOOTX64.EFI`)
  as a diagram; QR on a white card with square modules.
- **Review:** every screen rendered at about six reference sizes by
  `shells/snapshot` and reviewed as images.

`osk-ui` is forked into `faraday-ui` for this: its layout engine and
tiny-skia renderer are kept; forms components, the `wide` layout, kerning,
shadows and motion are added. The Wallets tab is drawn with it like every
other tab.

## 10. Crates

| Crate | Role |
|---|---|
| `faraday/faraday-core` | State machine: tabs, flows, the lock cycle, Inbox and Outbox; names `osk-*` items directly; the Wallets tab's steps, per-wallet-kind flows, wallet reassembly and non-cryptographic logic and wording (`docs/WALLETS.md`). Events in, commands out, no I/O. |
| `faraday/faraday-vault` | The vault format (`docs/VAULT.md`). `no_std` + `alloc`. |
| `faraday/faraday-pgp` | The write-only OpenPGP subset. `no_std` + `alloc`. |
| `faraday/faraday-sb` | Secure Boot keys, signature lists, `.auth` files, Authenticode. |
| `faraday/faraday-ui` | Fork of `osk-ui` (§9). |
| `faraday/shells/stick` | The x86 stick shell, from `shells/pi`: framebuffer, evdev, V4L2, the pipe to the disk process. |
| `faraday/faraday-disk` | FAT16 and FAT32 in Rust, unprivileged. |
| `faraday/faraday-grant` | The `CAP_CHOWN` helper. The only new crate with `unsafe` (ownership and capabilities). |
| `faraday/faraday-boot` | Not built yet (§5.5): copies the running Faraday's boot partition raw onto another stick's. |

New dependencies, each needing a note in `docs/deps/` under OpenSigner's
policy: an Ed25519 implementation (`ed25519-dalek`), `sha1`, `hkdf`,
`rsa`, the RustCrypto DER, X.509 and CMS crates, `miniz_oxide` in the app
(BBQr `Z`), and a PNG and a JPEG decoder plus `rqrr` in the disk process
only (QR from images). FAT is in-house
(§12 item 5). Expected cost: roughly 15–25 crates on top of OpenSigner's
device graph, to be measured with `cargo tree` once a prototype exists.

## 11. Order of work

1. **Interface prototype.** Home, Create vault, Unlock, the Wallets tab
   and Secure Boot as rendered screens at the reference sizes, to settle
   the look before the flows are built.
2. **Image and processes.** Kernel changes (§4.2), `faraday-grant`,
   `faraday-disk`, the app loop, the ESP named `OSKBOOT`. QEMU test:
   a stick inserted while clean is listed and read; inserted while a
   secret is held, it is not handed out.
3. **Vaults.** `faraday-vault` with vectors and a reference reader;
   create, unlock, lock, write-back; entries. `tools/argon2-bench` run on
   the reference machines to fill `docs/VAULT.md` §3.1's timing table,
   including whether the 32-bit Pi 3 image allocates 1 GiB.
4. **The Wallets tab**, in `docs/WALLETS.md` §9's order.
5. **GPG.**
6. **Secure Boot**, with the OVMF enrolment test.
7. **Forms and look:** `faraday-ui`, kerning, scale, motion, the
   snapshot gallery.
8. **Upgrading a stick** (§5.5): the boot-partition-only flash for
   development first, then `faraday-boot` and the flow on the PC, the
   Learn page and README section, then the Pi.

## 12. Decided 2026-10-04

1. **Vault file names** are fixed: `vault.ofv`, then `vault-2.ofv`
   (`docs/VAULT.md` §6). Reversed by the owner 2026-10-07: a vault is
   named when it is made, and `vault.ofv` is the name left empty.
2. **Write-back replaces** the previous copy; no `.bak`.
3. **Idle** (owner, 2026-10-06): at 5 minutes without input a warning
   comes up with a countdown and what the lock wipes, seals and keeps;
   any key or touch closes it and does nothing else. The session locks
   at 10 minutes. The machine powers off at 20 minutes without input,
   counted across the lock, with a countdown on the Locked sheet, but
   only when the Outbox is empty; with
   anything waiting in it (a sealed vault with changes, a signed PSBT),
   the app stays on the lock screen and shows what is waiting, because
   powering off would lose it and, once locked, no secret is left in
   memory to protect. Both times are settings.
4. **Keyboard:** US English only in the first build.
5. **FAT:** in-house, in `faraday-disk` (§4.3). The `fatfs` crate's
   last release is 0.3.6 from January 2023, with 0.4 unreleased, and its
   `no_std` path rests on `core_io`. What is needed is a subset: FAT16
   and FAT32 (the stick's own 32 MB data partition is FAT16 as
   `mkfs.fat` makes it), long file names, list, read, create, rename,
   delete; no formatting, no in-place edits. It runs in an unprivileged
   process. Tests check every write with `fsck.fat -n` and read it back
   through the host kernel's `vfat`, cross-check reads against `fatfs` as
   a dev-dependency, and fuzz the parser with the repository's `fuzz/`
   setup.
6. **QR from images:** PNG only, read for QR codes in the disk process,
   which passes the app the decoded text and the picture's bytes as they
   are (`docs/QR.md` §4). The app keeps those bytes to write the same
   `.png` to another stick and never decodes them (owner, 2026-10-08).
   JPEG is not read (owner, 2026-10-05): it would need a decoder crate.
7. **BBQr `Z`:** read, with the decompressed size bounded; never written
   (`docs/QR.md` §4).
8. **No seccomp filter on the grant helper** (owner, 2026-10-05). Linux
   builds seccomp filters only with `NET`, which OpenSigner's
   `kernel.forbidden` excludes for a reason this plan keeps. A filter
   could not stop the one call that matters, `chown` on any path, and
   Landlock cannot either: it has no right for changing ownership, and its
   rules cannot tell a stick's node from an input device's under `/dev`.
   Instead the helper's input is made as small as it can be: `/sys`
   alone, nothing read from a stick (§4.3). The kernel stays OpenSigner's.

## 13. Open questions

1. **The Pi Zero 2 W.** Wi-Fi and Bluetooth hardware on the board, which
   OpenSigner notes must be disabled in the kernel or removed. (Decided
   2026-10-08: the Pi is the `pi3` board with the Waveshare 2.8" panel,
   with no HDMI and no USB keyboard, mouse or storage, §3.)
