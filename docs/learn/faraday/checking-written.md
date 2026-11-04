# Checking what is written

## What the Starter carries

Faraday Starter carries the release's stick and Pi images as the
release publishes them, compressed with xz, and a manifest naming each
with two SHA-256 sums and a size: the compressed file's sum, and the sum
and size of the image inside it. The manifest is built into the Starter
with the images.

These sums catch a file damaged on this computer. They do not show that
the Starter itself is the release's: that is the release's
`SHA256SUMS` and its signature, checked over the Starter's own file
(About).

## Before anything is written

The Starter reads the compressed image whole and compares its SHA-256
with the manifest's. On a difference nothing is written.

The stick must still be the one confirmed, by its serial number and
size. Replace Faraday, keep data also checks the stick's partition
table first: a Faraday stick's layout, GPT with exactly two partitions,
an EFI system partition at sector 2048 of 64 MiB and a basic data
partition at sector 133120 of at least 128 MiB. Names, IDs and labels play no
part: they are an ordinary stick's. A stick with any other layout is
refused and nothing is relabelled.

## Its name

The data partition has no name in the image, so each computer shows it
by its own word for a drive with none. A name typed on What to write is
written as FAT keeps one, in capitals: in the data partition's first
sector, in its backup at sector 6, and as the volume-label entry in its
top folder, which is what Windows shows. Keep data leaves the data
partition, and its name, as they are.

## While it is written

The image is decompressed as it is written, by an xz decoder written in
Rust, with no temporary file. As it streams, the compressed file is
read a second time and its SHA-256 must match the manifest's again, and
the image written must have the manifest's size and SHA-256. Otherwise
the write is reported as failed, never as done.

After the write the Starter reads back from the stick what it wrote
and compares it, byte for byte, with what it meant to write, then has
the system read the partition table again. Keep data then checks that
the partition table is unchanged.

## Clearing the whole stick first

What to write offers Clear the whole stick first for a new stick: zeros
are written over every sector of the stick before it is made, and
Writing shows that pass with its own progress and the time left. None
of the zeros is read back. It takes about as long again as writing the
stick (an hour for 32 GB, at the 8 MB/s of a USB 2 stick), so it is
unticked at first. It removes what the stick held from the sectors a
computer can read; it does not reach the flash a stick keeps in
reserve, and the Starter does not say a stick is erased. A stick not
cleared keeps what it held in the free space of its data partition, as
a stick written with `dd` does.

## What a stick has of its own

A stick written by the Starter is made for its size, not copied from
the image byte for byte. Its boot partition is the image's; its
partition table and its data partition are the Starter's own:

- the table holds the disk's GUID and each partition's unique GUID, new
  for each stick, with the data partition filling the stick from sector
  133120 to its last usable sector, and the backup table at the stick's
  end;
- the data partition is a FAT32 volume of the stick's size, with its
  own serial number and the name you gave it, where one was given, so
  a 32 GB stick has about 32 GB for files where the image has 128 MiB;
- the boot file is the one you signed, where one was chosen.

The IDs are drawn from the system's randomness. They are not secret;
they make two sticks two disks to a computer. Windows takes a second
disk with the same GUID offline and writes new IDs to it when it is
brought online.

## What is compared on a stick

- The image, as it is read, has the manifest's SHA-256, and the boot
  partition's SHA-256 is the one the release publishes: the
  `-boot.vfat` line of `SHA256SUMS`.
- The boot partition read back from the stick is compared with the
  image's, byte for byte.
- The table and the data partition read back are compared with what the
  Starter made. No published number covers them: they are the stick's
  own. The Starter's FAT32 is checked against `fsck.fat` and
  `mkfs.fat`, which Faraday did not write.

The screen says "The boot partition matches the release", or, with a
boot file you signed, "The boot partition matches the release, except
the boot file you signed, as expected".

To repeat the boot partition's check by hand on Linux, once the
signature of `SHA256SUMS` is checked: `sha256sum /dev/sdX1` is the
`-boot.vfat` line's sum. It holds for a stick written with `dd` too,
and for a stick that has been in Windows. A stick written with a boot
file you signed holds that file, so its sum is not the release's.

## What differs on a Pi card

A Pi card is a copy of the image, except these bytes, which are its
own: the MBR disk signature (4 bytes at offset 440), the data
partition's FAT serial number in its boot sector and its backup boot
sector, and its name, where one was given. With those bytes put back
as the image has them, what was written has the manifest's SHA-256: the
Starter checks that after every write, and the screen says "Matches the
release image, except this card's own IDs, as expected", naming the
name where it applies.

To repeat it by hand on Linux: copy the image's length off the card
(`dd if=/dev/sdX of=copy.img bs=1M count=…`, the length the manifest
names); put the image's own disk signature back with `sfdisk
--disk-id`, attach the copy (`losetup --partscan`) and put each FAT
serial back with `fatlabel -i` and the data partition's label with
`fatlabel`; then `sha256sum copy.img`. This holds until the card has
been in a Windows computer, which adds a `System Volume Information`
folder to its boot partition; the files on it stay as they were.

An image given to the Starter by name (`--image`) has no manifest:
Confirm shows its SHA-256, and the write must give that sum.

## A boot file one signed

A boot file chosen under One I signed is checked when it is chosen and
again by the writer. Its Authenticode image hash, which leaves the
signature out, must equal the hash of the release's own `BOOTX64.EFI`,
read from the image the Starter carries after that image is checked
against its manifest. Its signature must be one alone, at the end of
the file. Otherwise it is refused by name.

Only the signature is taken from the file. What is written is the
release's own program with that signature added, so every byte the
signature does not cover is the release's. The signature must verify
over what is built, with the certificate it carries.

The Starter does not judge whose key signed it: the computer's db does
that when it starts the stick. What to write and Confirm show the
certificate's name, which the file's maker chose, and its SHA-256, to
compare with the db key's.

The writer reads the file again before the stick is opened and refuses
it when its SHA-256 differs from the one it had when chosen.

## The BitLocker recovery key

On Windows, with Include the BitLocker recovery key ticked, Print reads
the system drive's recovery key through the Starter's elevated copy,
after one administrator prompt. The Starter checks only its shape:
eight groups of six digits separated by hyphens, for a drive letter.
Nothing checks that it opens the drive.

The key is held in memory only until the sheet is made, and that memory
is wiped when it is dropped. It is never drawn on the page. The sheet
carries the drive and the 48 digits, four groups to a line, and nothing
else of BitLocker. The PDF has no document properties and names no
path, user or computer.

## Where this comes from

Faraday's own design: the Starter's specification (sections 1.3, 3.4,
4.1 and 4.5), the specification of a stick that does not say it is
Faraday's (sections 4 and 5), and decisions F49, F133 and F174, in
Faraday's repository, as of 2026-10-30. The GPT and its CRCs are the
UEFI specification's; the FAT serial is Microsoft's FAT
specification's.
