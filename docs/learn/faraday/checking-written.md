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
table first: GPT, Faraday's disk identifier, and exactly two
partitions, FARADAYBOOT and FARADAYDATA. A stick with any other layout
is refused and nothing is relabelled.

## While it is written

The image is decompressed as it is written, by an xz decoder written in
Rust, with no temporary file. As it streams, the compressed file is
read a second time and its SHA-256 must match the manifest's again, and
the image written must have the manifest's size and SHA-256. Otherwise
the write is reported as failed, never as done.

After the write the Starter reads the whole of what it wrote back from
the stick and compares it, then has the system read the partition table
again. Keep data then checks that the partition table is unchanged.

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
4.1 and 4.5) and decisions F49 and F133, in Faraday's repository, as of
2026-10-28.
