# Upgrading a Faraday stick

## What it does

Settings → Upgrade a Faraday stick copies the Faraday that is running onto another Faraday stick. The boot partition of the stick Faraday started from is written over the other stick's boot partition, read back and compared. The other stick's data partition, with its vaults and settings file, is not written. A stick that holds vaults then never needs a computer other than the one Faraday runs on.

The upgrade runs only in a fresh session with nothing unlocked. If anything is unlocked, Faraday locks first and opens the upgrade in the fresh session.

## Which stick is copied

Faraday copies the stick it started from. It knows that stick by what it holds: the kernel on its boot partition carries this Faraday's version and commit. It does not matter when the stick went in, whether it was taken out and put back, or whether both sticks are in at once.

This identifies a version. It is not a signature: a stick made to carry the same version would be copied too. Copy only from a stick you made from a release you checked.

## Versions

The upgrade shows the version on the stick to upgrade and the version to be written. A stick made by Faraday 0.1.0 or earlier carries no version and shows as 0.1.0 or earlier. A stick that carries a newer Faraday than the one running is marked: writing it puts an older Faraday on it. A stick that already carries this Faraday is not written.

## Limits

The stick to upgrade needs a boot partition at least as large as this one's: 48 MB on a PC. A release whose boot partition grew cannot be copied onto an older stick. That stick has to be written again from the image on a computer, which empties its data partition.

A stick removed during the write does not start until it is upgraded again. The write does not touch its data partition, so its vaults are still there.

## With Secure Boot

The copy is byte for byte, so a stick whose BOOTX64.EFI carries your db signature passes it on. Faraday does not sign inside the upgrade. Sign each new release once, through a spare stick, so that the stick holding your vaults never meets an online computer:

1. On a computer, write the new release onto a spare stick, and copy its EFI/BOOT/BOOTX64.EFI onto the spare's data partition as well.

2. Start from your signed vault stick. Read BOOTX64.EFI from the spare into the Inbox, unlock the vault that holds your db key, sign the file, lock, and write only the signed file back to the spare.

3. On the computer, copy the signed file over EFI/BOOT/BOOTX64.EFI on the spare's boot partition.

4. Start from the spare, and upgrade the vault stick from it.
