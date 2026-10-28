# Replacing Faraday on a stick

## What it does

Make a Faraday stick → Replace Faraday, keep data writes a new Faraday over the stick's boot partition, reads it back and compares. The data partition, with its vaults and settings file, is not read and not written.

This is how a Faraday stick is upgraded: Faraday itself writes no boot partition. The new Faraday is checked against the release's manifest before it is written. A stick that already holds the same build is not written. Writing an older Faraday over a newer one is marked on Confirm and can still be done. The stick's boot partition has to be at least as large as the new one's; a stick whose boot partition is smaller has to be written whole, which empties its data partition.

A Faraday stick or Pi card needs a USB stick or SD card of 256 MB or larger. Faraday's own boot file takes about 14 MB of the stick's 64 MiB boot partition, and the Pi card's boot files about 16 MB of its 64 MiB; the rest of that partition is room for later versions to grow. The 128 MiB data partition is storage for files: vaults, PSBTs and anything else brought to the device.

## With Secure Boot on

The release's boot file, `BOOTX64.EFI`, carries no signature. A computer that starts Faraday with Secure Boot on, with your own db key enrolled, needs the boot file signed with that key. Sign the new release's boot program on Faraday: download the release's `faraday-X.Y.Z-BOOTX64.EFI`, `SHA256SUMS` and `SHA256SUMS.asc` onto a stick's data partition, copy the three in, sign `faraday-X.Y.Z-BOOTX64.EFI` (Vaults, the vault holding the keys, Secure Boot, Owner keys, then Sign or check an image) and write the signed file back. Faraday signs it only when the signature over `SHA256SUMS` verifies with the release key built into Faraday, `SHA256SUMS` lists the file, and the release is at least as new as that Faraday. Put the signed file in this computer's Downloads folder. On What to write, under Boot file, choose One I signed, then the file. Dropping the file on the window chooses it too.

Faraday Starter takes the file only when it is this release's own program with a signature that holds: the program is compared with the release's while the signature is left out, so a file of another release, or with one byte changed, is refused, whoever signed it. It does not judge whose key signed it; the computer does that when it starts. What to write and Confirm show the name on the signing certificate and its SHA-256. The name is whatever the file's maker chose; compare the SHA-256 with your db key's. Only the signature is taken from the file: what is written is the release's own program with that signature, in place of the release's file, and everything else on the boot partition is the release's.

## What the computer can see

Plugging the stick into this computer is what exposes its data partition, whatever Faraday Starter does. The computer can read and write it, and Windows gives it a drive letter. The vaults there are encrypted with their passphrases. The settings file is not encrypted. A program on this computer could read the settings file, and could change or delete anything on the data partition.

## Where this comes from

Faraday's own design: the Starter's specification (sections 3.2, 4.4 and 4.5) and decisions F2, F5, F11, F80, F128 and F133, in Faraday's repository, as of 2026-10-27.
