# The Faraday boot entry on Linux

## What a boot entry is

A computer's BIOS keeps a list of things it can start, its boot entries. Each has a number, a name and where to start from: Windows Boot Manager, a Linux distribution, a disk. A second list, the boot order, says which entries the BIOS tries at every start, and in which order.

A third setting, BootNext, names one entry to start at the next start only. The BIOS removes BootNext before it starts that entry, so the start after that follows the boot order again.

## The Faraday stick entry

On Linux, Restart from the stick adds one entry named Faraday stick, the first time it is used. It points at the boot program on a Faraday stick's boot partition. Every Faraday stick's boot partition has the same identity, so the one entry starts any Faraday stick.

The entry is never in the boot order. The BIOS starts it only when Faraday Starter sets BootNext to it, which it does just before it restarts the computer. The start after that is the usual one, whether or not a stick is plugged in.

Adding the entry and setting BootNext change the BIOS's settings, so Linux asks for an administrator's password each time.

## Who can see it

Anyone who lists the BIOS's boot entries sees one named Faraday stick: with efibootmgr on Linux, with bcdedit on Windows, and in the boot menu of some computers. It shows that Faraday was used on this computer. It holds nothing else.

## Keeping it or removing it

Kept, the next Restart from the stick uses it again instead of adding another; setting BootNext still asks for the password.

Removed, nothing of Faraday is left in the BIOS. Once the computer is back from a Faraday session, Home offers Remove the Faraday boot entry. Uninstall removes it too. Removing it also asks for an administrator's password.

The stick itself never writes a BIOS setting, and so never adds or removes an entry.

## Where this comes from

efibootmgr(8), the manual page of efibootmgr (man.archlinux.org). The UEFI Specification 2.11, chapter 3, Boot Manager: Load Option Processing (BootNext) and Globally Defined Variables (BootOrder) (uefi.org). Both read 2026-10-12. The stick not writing the firmware's variables: EFIVAR_FS in kernel.forbidden of the Faraday image's x86_64-uefi board, in Faraday's repository.
