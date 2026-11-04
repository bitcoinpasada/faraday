# A stick that does not say it is Faraday's

**Kind:** Spec
**Status:** Specified (F174, F182); built: the image, the device, the Starter, the Name field, Learn and the guide (§3 to §8), with a stick Windows leaves whole (§4, §5.1)
**Open:** §12's batch 5: the dev build and disk test, the write timings and the Windows 11 virtual machine run of §10; macOS's `diskutil` keys checked against a Mac, paused until one is available (`docs/MAC.md`)

A Faraday stick or Pi card has partitions with the same names as any
other stick or card, its own IDs, and nothing in Faraday that finds it
by name. The owner settled every choice here on 2026-10-30 (F174), and
those of §2's finding on 2026-10-30 and 2026-10-31 (F182). The
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

**A stick bigger than the image.** Tried again on 2026-10-30
in the same virtual machine, with two sticks of 1 GiB written by the
Starter from an image of 395,304 sectors: no collision, each stick kept
its own IDs, and Windows still rewrote both partition tables, read on
the host afterwards and compared with the image byte by byte:

- The primary header (sector 1): the backup's place set to the disk's
  last sector, the last usable sector to 33 before it, the partition
  entries' place moved from sector 2 to sector 2016, and the header's
  CRC worked out again. The entries' CRC, the disk GUID and every
  other field were left. Nothing was written at sector 2016, which
  holds zeros, so the primary entries' CRC fails there.
- A new backup at the disk's end: the entries, unchanged, in its last
  33 sectors but one, and a header naming them. It is whole and right.
- The protective MBR (its partition still the image's size), the
  entries at sector 2 (whole, and matching the primary header's
  entries CRC) and the image's own backup at its last sector were left
  as the Starter wrote them.

Windows did this because the backup GPT was not at the disk's last
sector, where the UEFI specification (§5.3) puts it: the image's
table ends where the image does. Sector 2016 is the image's first
usable sector (2048) less the 32 sectors the entries take; a table
Windows makes itself has its first usable sector at 34, which gives
sector 2, where the entries are. That Windows works out the entries'
place this way, rather than keeping it, is read from these two sticks
and the collision above (which had the same result); Microsoft
documents neither the rewrite nor the formula.

What each reader makes of a stick in that state (tried on scratch
copies with Windows' change applied, unless it says read):

- **Linux** (`block/partitions/efi.c`, read): a primary header whose
  entries fail their CRC is invalid, and the backup is only looked at
  with `gpt` on the kernel's command line. The disk has no partitions.
  The device's kernel: a stick in that state, plugged in, has no
  partitions, so nothing is handed out and Files sees no stick.
  udisks2 lists the disk with no partitions, so the Starter on Linux
  does not know it as a Faraday stick.
- **The PC's firmware** (OVMF, EDK II's `PartitionDxe`, read and
  tried): it finds the primary invalid, reads the backup at the last
  sector, and writes the primary back from it (entries at sector 2).
  The stick then starts, and the device finds it by its build. OVMF
  does this to every USB disk in at start, not only the one it starts
  from. Other firmware may or may not.
- **macOS** (`IOGUIDPartitionScheme`, read): an entries CRC that fails
  ends the scan; the disk has no partitions.
- **Windows**: `IOCTL_DISK_GET_DRIVE_LAYOUT_EX` gives both partitions
  (Windows reads its own backup), so the Starter on Windows lists the
  stick as a Faraday stick; then its raw read of the table (keep data,
  the version on the stick) fails on the primary and refuses it as not
  a Faraday stick.
- **sfdisk, sgdisk and `faraday/image/stick-boot.sh`**: "the primary
  GPT table is corrupt, but the backup appears OK", and the backup's
  table is used; `stick-boot.sh` accepts the stick.

Every reader that fails, fails closed. But a stick that has been in a
Windows computer once is not seen by Linux or macOS, nor kept by the
Starter's keep data, until a PC's firmware happens to repair it. Every
stick of 0.1.8 and before has this shape.

**What Windows does to the boot partition.** Shut down with a stick in,
Windows writes three values in the boot partition's FAT32: the second
FAT entry, FAT[1], set to `0xFFFFFFFF` in both FATs, and FSInfo's hint
of the next free cluster moved on by one. `fsck.fat` finds the volume
clean before and after: FAT[1]'s top four bits are reserved, and the
hint is advice that Linux's vfat, `fsck.fat`, OVMF and `faraday-fat`
check against the volume or ignore. But the boot partition's SHA-256 is
then not the release's, so a stick that has been in Windows fails the
check by hand of §4.3. Plugging a stick in, copying to it and pulling it
out while mounted wrote nothing to the boot partition.

**What was tried then** (the same virtual machine, 2026-10-30, scratch
sticks of 1 GiB made on the host and read there after each step):

- The image with its first usable sector at 34 in both GPT headers, the
  partitions where they are: Windows rewrites the table as above, and it
  comes out whole, the entries' place being sector 2. The kernel reads
  both partitions after plugging in, copying and shutting down.
- A table for the whole stick (the backup at the disk's last sector,
  the data partition to the end): Windows writes nothing to the table,
  at any step.
- A boot partition with FAT[1] and the FSInfo hint already as Windows
  leaves them, in a table for the whole stick and in the image with its
  first usable sector at 34: its SHA-256 is unchanged after plugging in
  and ejecting, copying to the data partition, five minutes in and a
  shutdown, and pulling the stick out while mounted. The kernel reads
  the stick, OVMF starts from it, and the device finds its data
  partition.
- The GPT attributes read-only (bit 60) and do-not-mount (bit 63) on
  the boot partition: Windows ignores both on an EFI system partition
  and writes the three values at shutdown all the same.

So the image is built as Windows leaves it, with a table that stays
whole if Windows rewrites it (§4.1), and the Starter makes a stick
Windows has no reason to change (§4.2).

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

**Its first usable sector** (F182). Both of the stick image's GPT
headers give sector 34 as the first usable sector, where genimage gives
2048 (the first partition's start); the partitions do not move. 34 is
what a table Windows makes has, and the first usable sector less the
entries' 32 sectors is then sector 2, where the entries are, so a table
Windows rewrites (§2) stays whole: on a stick written with `dd`, Etcher
or GNOME Disks, and on any stick Windows writes for another reason (a
collision, Disk Management). The GPT read-only and do-not-mount
attributes are not used: Windows ignores them on an EFI system
partition (§2).

**Its boot partition as Windows leaves it** (F182). The stick image's
boot FAT32 has FAT[1] set to `0xFFFFFFFF` in both FATs and FSInfo's
next-free hint set to the first free cluster, read from the volume's
FAT as built, never a fixed number (it moves with the boot file's
size). These are the values Windows writes at shutdown (§2), so it has
nothing to write and the boot partition's SHA-256 stays the release's.
`fsck.fat` takes either form as clean.

Both steps run after genimage, in `faraday/image/build.sh` (or the
board's `post-image.sh` it calls), with the headers' and the entries'
CRCs worked out again, the same bytes on every builder. The build then
checks the image with `sgdisk -v` and `sfdisk --verify` (no problem
found) and the boot partition with `fsck.fat -n` (clean), all tools
Faraday did not write, and fails if any of them reports one. The
`-boot.vfat` line of `SHA256SUMS` is the boot partition as it is then.

The Pi card's boot partition is FAT16, which has no FSInfo, and whose
FAT[1] carries the clean-shutdown and no-error bits. Windows gives it a
drive letter on plugging in and adds `System Volume Information`: three
root directory entries and three clusters that were free, with their
FAT entries. Every file and its clusters stay as they were, and the
card's kernel starts from it as from the release's (under QEMU, which
does not run the Pi's firmware), but its SHA-256 is no longer the
release's.

### 4.2 What the Starter changes

When it writes a whole stick or card, the Starter draws new IDs from the
system's randomness (they are not secret), and these are each stick's
and card's own:

- the stick's disk GUID, and each partition's unique GUID, in the
  primary GPT (header and entries) and the backup GPT, with both
  headers' CRCs and both entry arrays' CRCs worked out;
- the Pi card's MBR disk signature (offset 440, four bytes);
- the data partition's FAT serial: in its boot sector (offset 0x43 on
  FAT32, 0x27 on FAT16) and, on FAT32, in the backup boot sector the
  boot sector names.

The boot partition keeps the release's serial, on the stick and on the
Pi card (F182, in place of F174's own serial for each volume): it is
the release's boot partition byte for byte, so `sha256sum` of it is the published value (§4.3). Nothing finds
a volume by its serial in Faraday (§5), the boot partition takes no
drive letter on Windows, so its serial is seldom recorded, and its boot
file names Faraday to anyone who opens it (§8); a serial shared by every
Faraday stick of a release tells no more. Raspberry Pi OS does the same:
a new disk identifier at first start, its boot partition's FAT serial
as released.

**The Pi card** is written from the image, at the image's sizes
(F182): these IDs are the only bytes that differ from it, with the
label of §6 where the person gave one. They are applied as patches on
the stream (`faraday/shells/starter/src/writer.rs`, `Patched`). An MBR
has no backup for Windows to move, so §2 does not reach the card's
table.

**A stick is made for its size** (F182). The Starter does not copy the
image to a stick: it makes the stick from it.

1. **The partition table for the whole stick**: the protective MBR
   with its partition covering the disk (at most `0xFFFFFFFF`
   sectors); the primary header (sector 1) with the backup at the
   disk's last sector, the last usable sector 33 before it, the first
   usable sector 34 (§4.1); the entries at sector 2: the boot
   partition as the image's, the data partition from sector 133120 to
   the last usable sector; the backup entries and header in the disk's
   last 33 sectors. The stick's own GUIDs (above), every CRC worked
   out. Windows then has nothing to rewrite (§2).
2. **The boot partition**: the image's sectors 2048 to 133119 byte for
   byte, its FAT serial the release's and its FAT as Windows leaves it
   (§4.1), with the signed boot file of `docs/STARTER.md` §4.5 where
   there is one.
3. **The data partition**: a FAT32 volume at the partition's size,
   made by Faraday's own formatter in `faraday/faraday-fat` (F180,
   `docs/ESTABLISHED.md` §3.2, batch E1), with Microsoft's FAT
   specification's table of cluster sizes as `mkfs.fat -F 32` follows
   it (4 KiB to 8 GB, 8 KiB to 16 GB, 16 KiB to 32 GB, 32 KiB above),
   the stick's own data serial and the person's name (§6), no files.
   Only its reserved sectors, both FATs and the root directory's cluster
   are written. The image's data partition is an empty FAT32 volume
   (`faraday/image/boards/x86_64-uefi/genimage.cfg`, `empty = true`),
   so nothing of it is carried over.

The source is still the release image: it is decompressed and its
SHA-256 checked against the release's image list as it streams past, as keep data
does now, and only its boot partition reaches the stick. A stick of
32 GB then has about 32 GB for vaults and files, where it had 128 MiB.

What is written falls from the image's 202 MB to the boot partition's
67 MB and the FATs (two of 4 bytes a cluster: 16 MB on a 32 GB stick
at 16 KiB clusters, under 1 MB on a 1 GB one). Write and read-back of
a 32 GB stick, at a USB 2 stick's usual 8 MB/s writing and 25 MB/s
reading: about 14 s where it was about 33 s; at a USB 3 stick's 40 and
150 MB/s, about 3 s where it was about 6 s. These are estimates, timed
in §10.

**A reused stick's free space** is left as it is (F182): the rest of
the data partition holds what the stick held before, as the stick past
the image's end does now. Faraday's vaults are encrypted, and Faraday
writes no key to a stick, but a stick that held other files keeps them
in its free space, where a recovery tool can find them. The Starter
does not say a stick is erased: a discard (TRIM) is a hint that many
USB sticks ignore, and an overwrite does not reach the flash a stick
keeps in reserve.

**Clear the whole stick first** is an option on What to write? under
New Faraday stick (`docs/STARTER.md` §3.2; Material 3, Checkbox),
unticked at first and not offered with keep data. Under it, the time it
adds: "Adds about N min", from the stick's size at 8 MB/s, a USB 2
stick's writing speed, so the estimate errs long (about an hour for
32 GB). Confirm says it again before anything is written. Ticked, the
Starter writes zeros over every sector of the stick before it makes the
stick, reads none of them back, and Writing shows that pass with its
own progress and the time left. The option does not reach the flash a
stick keeps in reserve.

`dd`, Etcher and GNOME Disks still write the image as published, which
stays what anyone can check against `SHA256SUMS` (§7): its data
partition of 128 MiB, its table with the first usable sector at 34
(§4.1).

### 4.3 The check after writing

Everything written is read back and compared, byte for byte, with what
was meant to be written (`docs/STARTER.md` §4.1, step 6).

**A Pi card.** It is compared with the image as decompressed again with
the same patches in place. Then:

- With the patches taken back out of what was read (the image's own
  IDs and label put back in their places), the SHA-256 of the whole is
  the release's image list's. This is the check a person can repeat (§8).
- The screen says what was checked, and names what was expected to
  differ: "Matches the release image, except this card's own IDs, as
  expected", with "its name" named before ", as expected" where it
  applies ("Matches the release image, except this card's own IDs and
  its name, as expected").

**A stick** (F182). The whole-image SHA-256 does not apply to a stick
made for its size: its table and data partition are the Starter's own.
What is checked, and against what:

- **The source**: the image's SHA-256, as it streams past, is the
  list's (as keep data checks it now). A source that does not match
  is refused before the stick's boot partition is complete, and the
  write fails.
- **The boot partition**: read back whole and compared byte for byte
  with the image's, the signed boot file in place where there is one.
  Without one, its SHA-256 is also the boot partition's published value:
  the `-boot.vfat` line `SHA256SUMS` lists (`docs/IMAGES.md`), which the
  release's image list `IMAGES` (`docs/IMAGES.md` §4, built into the
  Starter, `docs/STARTER.md` §1.3) carries as its ninth field on a stick
  image's line (`-` on every other line). The Starter
  checks against a published number and not only against the stream.
- **The table and the data partition's FAT32**: every sector written is
  read back and compared with what the Starter made. No published value
  covers them: they are the stick's own, made from its size, its IDs
  and its name, and the Starter's FAT32 is tested against dosfstools'
  `fsck.fat` and `mkfs.fat`, which Faraday did not write (§10).

The screen says what was checked: "The boot partition matches the
release", or, with a signed boot file, "The boot partition matches the
release, except the boot file you signed, as expected"; the table and
data partition are not named, being made, not copied.
`docs/learn/faraday/checking-written.md` says the hand check is the
boot partition's, one command on Linux: `sha256sum /dev/sdX1`, compared
with the `-boot.vfat` line of `SHA256SUMS` once its signature is
checked (a stick with a signed boot file holds that file, so its sum is
not the release's). It holds for a stick written with `dd` too, and,
with the boot partition built as Windows leaves it (§4.1), after the
stick has been in Windows.
On Windows the Starter reads a stick's boot file, to show its version,
without changing the file's last-access date, which Windows' FAT driver
would otherwise write into the boot partition.

### 4.4 Keep data

Keep data writes only the boot partition (F128) and never the
partition table. It keeps the stick's own disk and partition GUIDs, and
the new boot partition is the release's as it is, its serial included
(§4.2).

Keep data is unchanged by a stick made for its size (§4.2): it reads
the table, finds §5.1's layout (a data partition of 128 MiB or more),
and writes the boot partition, checked as §4.3 checks a stick's: byte
for byte, and against the `-boot.vfat` value where no boot file is
signed; its screen says "The boot partition matches the release". A
stick whose primary table Windows broke (§2) is refused, as now, with
the fault named: the table is damaged, make the stick again. The
Starter reads the primary table only and never writes a table in keep
data (F128); it does not take the backup's word for a table it is about
to write beside.

## 5. Knowing a stick without its names

### 5.1 The layout

A Faraday stick is a GPT disk with exactly two partitions: the first an
EFI system partition (type `C12A7328-F81F-11D2-BA4B-00A0C93EC93B`) at
sector 2048 of 64 MiB, the second a basic data partition (type
`EBD0A0A2-B9E5-4433-87C0-68B6B72699C7`) at sector 133120 of at least
128 MiB, ending at or before the disk's last usable sector (F80,
F182). Everything else is exact: GPT, exactly two partitions, the
types, the boot partition's start and size, the data partition's start.
A stick written with `dd` (128 MiB) and one the Starter made for its
size (to the last usable sector) are both known; any other layout is
not, and every reader fails closed on it. A Pi card is an MBR disk with
two FAT partitions of the image's sizes, exactly (F80, F182). Names,
IDs and labels play no part.

The one rule is read in three places, which agree: in
`faraday/faraday-boot/src/layout.rs` (`is_stick`), read by
`faraday-grant --find-boot`; in `faraday/shells/starter/src/gpt.rs`
(`layout`), read by the Starter on each system and by keep data; and in
`faraday/image/stick-boot.sh`. The looser size admits any disk an
ordinary tool partitions so; the device still takes only the one whose
boot partition holds the running build (§5.2), and keep data writes
only a boot partition (F128).

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
  writing compares, what is expected to differ, and how to repeat
  §4.3's check by hand: on a stick, the boot partition's SHA-256
  against its `-boot.vfat` line; on a Pi card, the image's IDs put back
  and the SHA-256 of the whole.
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

With F182:

- The image: `faraday/image/build.sh` and the stick board's
  `post-image.sh` (the first usable sector, the boot FAT, the checks
  with `sgdisk`, `sfdisk` and `fsck.fat`), `faraday/image/README.md`,
  `faraday/tools/stage-release.sh` (the boot partition's sum in
  `IMAGES`).
- The device and the layout: `faraday/faraday-boot/src/layout.rs`,
  `faraday/faraday-grant/src/lib.rs`, `faraday/faraday-disk/src/lib.rs`,
  `faraday/image/stick-boot.sh`, `faraday/tools/disk-test.py`.
- The Starter: `faraday/faraday-fat/src/lib.rs` (the FAT32 formatter),
  `faraday/faraday-payload/src/images.rs`,
  `faraday/shells/starter/src/gpt.rs`, `writer.rs`, `finder.rs`,
  `faraday/faraday-starter/src/api.rs` and `decide.rs`, What to write?'s
  Clear the whole stick first, the check's lines and Writing's clearing
  pass in `faraday/faraday-strings/src/en.rs`.
- Docs: F80's data partition size, `docs/STARTER.md` §1.3, §3.2, §4.1,
  `docs/IMAGES.md`, `docs/ESTABLISHED.md` §3.2,
  `docs/learn/faraday/checking-written.md`, the guide's Written picture
  (`docs/guide/screens/starter-make.md`) and Get Faraday on a stick
  (`docs/guide/walk/get-faraday.md`).

## 10. Tests

What a person or a wallet would notice, and checks against tools
Faraday did not write:

- Two sticks written by the Starter from one image have different disk
  GUIDs, partition GUIDs and data partition serials, and the same boot
  partition, the release's; two Pi cards likewise, each reading back
  as the image with its IDs, and with the IDs put back its SHA-256 the
  list's.
- The Starter knows a stick written so as a Faraday stick on each
  system's fixture, and refuses a disk with another layout; keep data
  keeps the stick's GUIDs and writes the release's boot partition as it is.
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

For F182:

- The image as built: `sgdisk -v` and `sfdisk --verify` find no
  problem, both headers give the first usable sector 34 and the
  partitions are where genimage put them; `fsck.fat -n` finds the boot
  partition clean, FAT[1] is `0xFFFFFFFF` in both FATs, and FSInfo's
  hint is the first free cluster, for two boot files of different sizes.
  Windows' rewrite of §2, applied to the image on a 1 GiB scratch copy,
  leaves a table the kernel, `sfdisk` and `is_stick` all read.
- A stick made for its size by the Starter from the release image:
  `sfdisk --verify` and `sgdisk -v` find no problem, the kernel lists
  both partitions with no GPT warning, `fsck.fat -n` finds the data
  partition clean at 1 GB, 32 GB and 256 GB (sparse files), and a file
  written there by the device, by Linux's vfat and by Windows reads back
  in the others. The boot partition's SHA-256 is the `-boot.vfat` value
  `SHA256SUMS` lists; a source one byte off is refused; a boot partition
  read back one byte off fails the write.
- The Starter's FAT32 against `mkfs.fat -F 32` with the same size,
  serial and label, at sizes either side of each step of the cluster
  table: the same boot sector fields, FSInfo and FAT starts, and
  `fsck.fat -n` clean.
- The image list `IMAGES` with the boot partition's sum is read; a
  stick image's line without it, or with a sum that is not hex, is
  refused.
- Clear the whole stick first: every sector outside what the Starter
  then writes reads zero, and the time shown before it starts is the
  stick's size at 8 MB/s.
- The device under QEMU (`faraday/tools/disk-test.py`, which gains
  `--no-kvm` for a builder without KVM, as `stick-qemu.py` has): it
  starts from a stick made for its size and finds its data partition; a
  stick in §2's state, plugged in, has no partitions and nothing is
  handed out; one in §2's state at start is repaired by OVMF and found.
- §5.1's rule: a data partition of 128 MiB and one to the disk's last
  usable sector are known; one of less than 128 MiB, one starting
  elsewhere, one past the last usable sector, a third partition, and a
  boot partition of another size are refused, by `is_stick`,
  `gpt::layout` and `stick-boot.sh` alike, on the same fixtures; a table
  Windows rewrote from an image with the first usable sector at 34 is
  known.
- Keep data on a stick made for its size keeps its table and data
  partition byte for byte; on a stick whose primary Windows broke it is
  refused with the damaged-table fault, not "no GPT" (the 4096-byte
  sector try no longer hides the 512-byte one's fault).
- The time to write and check a stick made for its size, on a USB 2
  and a USB 3 stick, against the whole-image write.
- In the Windows 11 virtual machine, with real builds (§12, batch 5): a
  stick made by the Starter, an image written with `dd`, and two of each
  at once, through plugging in, copying to the data partition, a
  shutdown with the stick in, and pulling it out while mounted; after
  each, the table is whole, the boot partition's SHA-256 is the
  release's, and the kernel reads both partitions. The Pi card's FAT16
  boot partition through the same steps, with the difference §4.1 names.

## 11. Choices made here

Within F174, this spec chose the following; each is the owner's to
change:

- Two sticks of the running build in at start: neither data partition
  is mounted (§5.2), rather than taking the first found.
- The Pi card's boot label is `bootfs` (§3), Raspberry Pi OS's own.

Decided by the owner on §2's finding (2026-10-30 and 2026-10-31,
F182):

- The image, for a stick written with any tool: its first usable sector
  at 34 and its boot FAT as Windows leaves it, checked in the build
  (§4.1). The GPT read-only attribute is not used.
- The Starter makes a stick for its size (§4.2): a table for the whole
  disk, the release's boot partition byte for byte, the data partition
  to the last usable sector formatted by Faraday's own FAT32 formatter
  (F180). The others weighed and not taken: the image written whole
  with a table for the stick's size over it (the data partition stays
  128 MiB); readers that take the backup when the primary is broken
  (the kernel only with `gpt` on its command line, macOS's and
  udisks2's readers cannot be changed, and the stick stays broken for
  every other computer).
- The boot partition stays the release's byte for byte, its FAT serial
  included, on the stick, the Pi card and in keep data (§4.2, §4.4);
  the disk and partition GUIDs, the Pi card's disk signature and the
  data partition's serial stay each stick's own. The check by hand is
  then `sha256sum` of the boot partition (§4.3). This replaces F174's
  own serial for each volume.
- The check after writing a stick: its boot partition against the
  release's published value, the rest as the Starter made it (§4.3).
- The data partition takes the whole stick, with no cap (§5.1).
- A reused stick's free space is left as it is, with Clear the whole
  stick first as an option that shows its time before it starts (§4.2).
- The Pi card keeps the image's sizes (§4.2, §5.1).
- Sticks made before get no line in the release notes.

Chosen here, within F182; each is the owner's to change:

- Clear the whole stick first writes zeros and does not read them back,
  and its time is estimated at 8 MB/s (§4.2).
- The boot partition's sum is the ninth field on a stick image's line
  of `IMAGES` (§4.3).

## 12. Order of work

Each batch is its own change, landed when its tests pass; each needs
the one before it, except 2 and 3, which can go in either order.

| Batch | What | Where | Size |
|---|---|---|---|
| 1 | This specification and F182 | §2, §4, §5.1, §10, §11 | Built |
| 2 | The image: first usable sector 34, the boot FAT as Windows leaves it, the build's checks with `sgdisk`, `sfdisk` and `fsck.fat` | §4.1 | Built |
| 3 | The device and the layout: §5.1's rule in `faraday-boot`, `faraday-grant`, `faraday-disk` and `stick-boot.sh`, with fixtures of each shape and one Windows rewrote; `disk-test.py --no-kvm` | §5.1, §5.2 | Built |
| 4 | The Starter: the stick made for its size, the FAT32 formatter (`docs/ESTABLISHED.md` §3.2, batch E1), the boot partition's sum in `MANIFEST` and its check, Clear the whole stick first, the check's lines, keep data's check, Learn's Checking what is written, the guide's Written picture | §4.2 to §4.4 | Built |
| 5 | A dev build and the disk test; the timings; the Windows 11 virtual machine run with real builds, the Pi card's FAT16 boot partition included; what it finds, if anything, is a change of its own | §10's last items | half a day to a day |
