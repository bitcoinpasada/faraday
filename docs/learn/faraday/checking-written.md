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
partition at sector 133120 of 128 MiB. Names, IDs and labels play no
part: they are an ordinary stick's. A stick with any other layout is
refused and nothing is relabelled.

## While it is written

The image is decompressed as it is written, by an xz decoder written in
Rust, with no temporary file. As it streams, the compressed file is
read a second time and its SHA-256 must match the manifest's again, and
the image written must have the manifest's size and SHA-256. Otherwise
the write is reported as failed, never as done.

After the write the Starter reads the whole of what it wrote back from
the stick and compares it, byte for byte, with the image decompressed
again with this stick's own bytes in place, then has the system read the
partition table again. Keep data then checks that the partition table is
unchanged.

## What differs from the image

A stick written by the Starter is not a copy of the image byte for
byte. These bytes are its own:

- the disk's GUID and each partition's unique GUID, in the primary and
  the backup GPT, with the CRCs of both headers and both entry arrays
  worked out again; on a Pi card, the MBR disk signature (4 bytes at
  offset 440);
- each FAT volume's serial number, in its boot sector and, on FAT32, in
  its backup boot sector;
- the data partition's name, where one was given;
- the boot file, where one you signed was chosen.

The IDs are drawn from the system's randomness. They are not secret;
they make two sticks two disks to a computer. Windows takes a second
disk with the same GUID offline and writes new IDs to it when it is
brought online.

With those bytes put back as the image has them, what was written has
the manifest's SHA-256: the Starter checks that after every write, and
the screen says "Matches the release image, except this stick's own
IDs", naming the name and the boot file where they apply.

To repeat it by hand on Linux: copy the image's length off the stick
(`dd if=/dev/sdX of=copy.img bs=1M count=…`, the length the manifest
names); read the image's own GUIDs with `sgdisk --info` and the disk's
with `sgdisk --print` on the image, and write them into the copy with
`sgdisk --disk-guid=… --partition-guid=1:… --partition-guid=2:…`, which
works the CRCs out again; attach the copy (`losetup --partscan`) and put
each FAT serial back with `fatlabel -i`, and the data partition's label
with `fatlabel`; then `sha256sum copy.img`. A stick written with a boot
file you signed holds that file, so its sum is not the manifest's.

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
