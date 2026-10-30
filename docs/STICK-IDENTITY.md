# A stick that does not say it is Faraday's

**Kind:** Spec
**Status:** Specified (F174); built: the image, the device, the Starter, the Name field, Learn and the guide (§3 to §8)
**Open:** the Windows 11 virtual machine test with two sticks written by the Starter (§2, §10's last item), and macOS's `diskutil` keys checked against a Mac (`docs/MAC.md`)

A Faraday stick or Pi card has partitions with the same names as any
other stick or card, its own IDs, and nothing in Faraday that finds it
by name. The owner settled every choice here on 2026-10-30 (F174). The
threat is a casual glance: someone who sees the stick in a file
manager, in AutoPlay, or in a computer's history of drives. Someone who
opens its files can still tell what it is (§8).

Faraday has no users yet (owner). Nothing here converts a stick made
before; §5.4 says what still works with one.

## 1. What says "Faraday" now

- **FAT labels** `FARADAYBOOT` and `FARADAYDATA`, on the stick and the
  Pi card (`faraday/image/boards/x86_64-uefi/genimage.cfg`,
  `faraday/image/boards/pi3/genimage.cfg`). Windows shows the data
  partition as "FARADAYDATA (E:)", in Explorer and in AutoPlay, and
  keeps the label in the computer's record of the drives it has seen.
- **GPT names**, the same two words (the stick; the Pi card is MBR).
- **GPT UUIDs** that spell "Faraday-Stick" in ASCII hex, the same on
  every stick (F67; `faraday/shells/starter/src/gpt.rs`,
  `faraday/image/stick-boot.sh`).
- **FAT volume serials** made from the commit date
  (`faraday/image/fat-serial.sh`), the same on every stick of a release.
- **Files**: `faraday-settings.txt` and `faraday-timings.txt` at the
  data partition's top, and Faraday's name and version inside the boot
  file. These stay (§8).

## 2. Two sticks with the same IDs on Windows

Tried on 2026-10-30 in a Windows 11 virtual machine, with two copies of
one stick image: the first attached at start, the second plugged in
while signed in.

- The second disk was **Offline**, with the reason **Collision** (it
  had the same disk GUID). Its partitions had no drive letter, and
  Windows said nothing to the person.
- Brought online (Disk Management's Online, `Set-Disk -IsOffline
  $false`), Windows **wrote a new disk GUID and new partition GUIDs to
  that stick**. Its data partition then had a drive letter.
- That stick no longer had F67's GUIDs, so the Starter's keep data and
  `faraday/image/stick-boot.sh` refuse it as not a Faraday stick.
- After the machine was shut down, that stick's primary GPT header
  pointed its partition entries at sectors holding zeros; the backup GPT
  was whole and right. Whether this happens on real hardware is not
  known.

So every stick needs its own IDs (§4), and nothing may know a Faraday
stick by fixed IDs (§5).

## 3. The names

The names an ordinary stick or card has:

| | Stick (GPT) | Pi card (MBR) |
|---|---|---|
| Boot partition, GPT name | `EFI system partition` | (none) |
| Boot partition, FAT label | `EFI` | `bootfs` |
| Data partition, GPT name | `Basic data partition` | (none) |
| Data partition, FAT label | none | none |

`EFI system partition` and `Basic data partition` are the names Windows
gives the partitions it makes. `bootfs` is the label of Raspberry Pi
OS's boot partition. A FAT volume with no label is written as FAT asks
for one (`NO NAME` in the boot sector and no volume-label entry in the
root directory), and each system shows it by its own word for a drive
with no name. To check in the build, on each system: what it shows for
the data partition of a stick written this way (expected: Windows "USB
Drive", macOS "NO NAME", Linux file managers the size and "Volume").

The device's Files and Stick screens show a volume with no label the
way they show any stick with none.

## 4. Each stick's own IDs

### 4.1 The image

The image keeps fixed IDs, so a build is still reproducible: a disk
GUID and two partition GUIDs on the stick, an MBR disk signature on the
Pi card, and the FAT volume serials, each made from the commit date and
its role as `faraday/image/fat-serial.sh` makes the serials. They spell
nothing, and differ from one release to the next. The GUIDs are version
4 in form (RFC 9562), like any random one.

### 4.2 What the Starter changes

When it writes a whole stick or card, the Starter draws new IDs from the
system's randomness (they are not secret):

- the stick's disk GUID, and each partition's unique GUID, in the
  primary GPT (header and entries) and the backup GPT, with both
  headers' CRCs and both entry arrays' CRCs worked out again;
- the Pi card's MBR disk signature (offset 440, four bytes);
- each FAT volume's serial: in the boot sector (offset 0x43 on FAT32,
  0x27 on FAT16) and, on FAT32, in the backup boot sector the boot
  sector names.

These are the only bytes that differ from the image, with the label of
§6 where the person gave one and the signed boot file of
`docs/STARTER.md` §4.5 where there is one. They are applied as the
writer applies a signed boot file now: as patches on the stream
(`faraday/shells/starter/src/writer.rs`, `Patched`), which take a list
of patches in place of one.

### 4.3 The check after writing

The check stays whole: everything written is read back and compared,
byte for byte, with the image as decompressed again with the same
patches in place (`docs/STARTER.md` §4.1, step 6). Then:

- With the patches taken back out of what was read (the image's own
  IDs, label and boot partition put back in their places), the SHA-256
  of the whole is the release manifest's. This is the check a person can
  repeat (§8).
- The screen says what was checked, and names what was expected to
  differ: "Matches the release image, except this stick's own IDs, as
  expected", with "its name" and "the boot file you signed" named
  before ", as expected" where they apply ("Matches the release image,
  except this stick's own IDs, its name and the boot file you signed,
  as expected").

### 4.4 Keep data

Keep data writes only the boot partition (F128) and never the partition
table. It keeps the stick's own disk and partition GUIDs, and gives the
new boot partition the FAT serial the old one had, read from the
stick's boot sector before the write. Its check is §4.3's for the boot
partition's range.

## 5. Knowing a stick without its names

### 5.1 The layout

A Faraday stick is a GPT disk with exactly two partitions: the first an
EFI system partition (type `C12A7328-F81F-11D2-BA4B-00A0C93EC93B`) at
sector 2048 of 64 MiB, the second a basic data partition (type
`EBD0A0A2-B9E5-4433-87C0-68B6B72699C7`) at sector 133120 of 128 MiB
(F80). A Pi card is an MBR disk with two FAT partitions of those sizes.
Names, IDs and labels play no part.

### 5.2 The device

**Its boot medium.** At start, before the app, init finds the disk it
started from:

- On the Pi: the board's own card, `mmcblk0`, as now.
- On a PC: among the USB disks with §5.1's layout (read from `/sys`:
  partition count, starts and sizes), the one whose boot partition holds
  this build: its boot file's release string (`faraday_boot::find_release`)
  is the running kernel's. Where exactly one does, its second partition
  is the data partition. Where none or more than one does, no data
  partition is mounted, and Files says that two sticks of this build are
  in, or that the stick it started from was not found.

Init records the data partition's node where it records the exchange
partition now (`/etc/faraday/exchange`, written by
`faraday/image/build.sh` from `EXCHANGE` in each board's `board.conf`),
and `LABEL=FARADAYDATA` goes from the PC's `board.conf`.

**What is kept back.** `faraday-grant` keeps back the first partition of
the boot medium, and every partition whose GPT name is `EFI system
partition`, and, on the Pi, `mmcblk0p1` (`faraday/faraday-grant/src/lib.rs`).
`faraday-disk` keeps back the same, and any FAT volume labelled `EFI`
or `bootfs`, as its second lock (`faraday/faraday-disk/src/lib.rs`).
This keeps back more than before: any stick's EFI system partition. It
fails closed, and Faraday writes no boot partition (F128). The backup
flow's refusal of a boot partition (`docs/BACKUP-FLOW.md`, Usable
sticks; `faraday/faraday-core/src/backup_flow/steps.rs`) reads the same
rule.

### 5.3 The Starter

The Starter knows a Faraday stick by §5.1's layout, on each system from
what it reads now without a password (udisks2 on Linux, `diskutil` on
macOS, `IOCTL_DISK_GET_DRIVE_LAYOUT_EX` on Windows, which gives each
partition's type, start and size), and by the release string on its
boot partition where that can be read (`docs/STARTER.md` §4.3). Keep
data checks the layout before and after, in place of F67's names and
GUIDs (`faraday/shells/starter/src/gpt.rs`, `linux/udisks.rs`,
`macos/diskutil.rs`, `windows/disks.rs`). The image's boot partition is
checked as a FAT32 volume labelled `EFI` where it was `FARADAYBOOT`
(`writer.rs`).

`faraday/image/stick-boot.sh`, the developer's keep data, checks the
same layout.

### 5.4 Sticks made before

A stick made before has §5.1's layout, so the Starter and the device
still know it. It keeps its old names and F67's GUIDs until it is made
again, or until its data partition is named (§6). Nothing converts it.

## 6. Naming the data partition

The person may give the data partition a name, or take it away:

- **In the Starter**, when making a stick or card: an optional Name
  field on Make a Faraday stick and Make a Pi card (`docs/STARTER.md`
  §3.2, §3.3), empty by default. It is the Starter's first text field;
  it follows Material 3's text field (m3.material.io, Text fields) and
  `docs/TEXT-CURSOR.md`.
- **On the device**, for the data partition of the boot medium and for
  any FAT stick it has been handed: Rename stick, in Files, a dialog
  with one text field (Material 3, Dialogs and Text fields).

A FAT label is at most 11 characters: letters, digits, spaces and the
characters FAT allows (Microsoft's FAT specification), kept in capitals
as Windows keeps them; a name typed in lower case is written in
capitals, and the field says so as it is typed. An empty name takes the
label away (§3). The label is written to the boot sector, to the backup
boot sector on FAT32, and to the root directory's volume-label entry,
which is what Windows shows. `faraday-fat` gains this write, checked
against the Microsoft FAT specification and against dosfstools'
`fatlabel`, which Faraday did not write.

## 7. A stick written by another tool

A stick or card written with `dd`, Etcher or Raspberry Pi Imager has the
image's IDs (§4.1), so two of them can still clash on Windows (§2). At
its first start, the device gives the data partition its own FAT serial
(where the serial is still the image's), written as §4.2 writes one. It
does not change the partition table (F128). Learn says to write a stick
with the Starter for it to have its own IDs.

## 8. Learn and the guide

- `docs/learn/faraday/checking-written.md` says what the check after
  writing compares, what is expected to differ, and how to repeat the
  manifest check of §4.3 by hand: put the image's IDs back and take the
  SHA-256.
- `docs/learn/faraday/boot/replace-faraday.md` says that a stick has
  ordinary names, that a computer still sees its files, and that
  someone who opens them can tell it is Faraday's: the settings file's
  name, and Faraday's name in the boot file.
- The guide's Make a stick walk and the Files screen's page name the
  Name field and Rename stick.

## 9. What changes with it

- The image: both `genimage.cfg`, `faraday/image/fat-serial.sh` (or a
  sibling for the GUIDs and the signature), both `board.conf`,
  `faraday/image/boards/x86_64-uefi/busybox.fragment` (no lookup by
  label), `faraday/image/stick-boot.sh`, `faraday/image/README.md`.
- The device: `faraday/faraday-grant/src/lib.rs`,
  `faraday/faraday-disk/src/lib.rs`, `faraday/faraday-fat/src/lib.rs`
  (labels), `faraday/shells/stick/src/files.rs` and `main.rs`,
  `faraday/faraday-core/src/backup_flow/steps.rs`, Files (Rename stick).
- The Starter: `faraday/shells/starter/src/gpt.rs`, `writer.rs`,
  `linux/udisks.rs`, `macos/diskutil.rs`, `windows/disks.rs`,
  `faraday/faraday-starter/src/api.rs` and `decide.rs`, the Name field.
- Strings: `faraday/faraday-strings/src/en.rs` (`starter_why_boot_label`,
  the `starter_fault_*` lines that name FARADAYBOOT and FARADAYDATA, the
  check's line, the Name field, Rename stick, Files' lines of §5.2).
- Docs: F67 (amended by F174) and F80's partition names, `docs/STARTER.md` §4.1, §4.3,
  §4.4, `docs/MAC.md`, `docs/BACKUP-FLOW.md`, `docs/DEVELOPING.md`,
  `README.md`, `docs/guide/walk/secure-boot-keys.md`, the Learn pages of
  §8, and the tests and fixtures that name the old labels
  (`faraday/faraday-disk/tests/disks.rs`, `faraday/faraday-grant/tests/grant.rs`,
  `faraday/shells/starter/tests/`, `faraday/faraday-fat/tests/judged.rs`).

## 10. Tests

What a person or a wallet would notice, and checks against tools
Faraday did not write:

- Two sticks written by the Starter from one image have different disk
  GUIDs, partition GUIDs and FAT serials; each reads back as the image
  with its IDs, and with the IDs put back its SHA-256 is the manifest's.
- The Starter knows a stick written so as a Faraday stick on each
  system's fixture, and refuses a disk with another layout; keep data
  keeps the stick's GUIDs and boot serial.
- The grant keeps back the boot medium's first partition and any `EFI
  system partition`; init picks the one stick of this build, and none
  where two are in.
- A label written by `faraday-fat` reads back the same in dosfstools'
  `fatlabel` and `fsck.fat`, and a label `fatlabel` wrote reads back the
  same in `faraday-fat`; an empty name leaves no label.
- A stick written with `dd` gets its own data serial at its first start,
  and keeps it at the next.
- Again in the Windows 11 virtual machine (§2): two sticks written by
  the Starter are both online, each with a drive letter, and neither is
  rewritten.

## 11. Choices made here

Within F174, this spec chose the following; each is the owner's to
change:

- Two sticks of the running build in at start: neither data partition
  is mounted (§5.2), rather than taking the first found.
- Keep data keeps the stick's boot serial (§4.4), rather than a new one.
- The Pi card's boot label is `bootfs` (§3), Raspberry Pi OS's own.
