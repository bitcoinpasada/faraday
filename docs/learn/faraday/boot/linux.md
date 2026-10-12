# Starting a Linux computer from the stick

## A disk unlocked by the TPM

If this computer's disk is unlocked by the TPM bound to PCR 7, Linux asks for the disk's passphrase or recovery key at the next start after Secure Boot is turned off. Have it before changing any firmware setting. PCR 7 holds the Secure Boot state: it changes when Secure Boot is turned on or off and when its keys change, and the TPM releases the disk's key only for the state it was bound to.

Such a disk has a line with tpm2-device in /etc/crypttab, and was set up with systemd-cryptenroll --tpm2-pcrs=7 or bound to PCR 7 another way. The Faraday app shows this first when /etc/crypttab names a tpm2-device. With Secure Boot back on and its keys as they were, the TPM unlocks the disk again.

A disk unlocked by a passphrase alone, or not encrypted, has nothing to do here.

## Open the firmware settings

The Faraday app's Restart into firmware settings does this in one step. By hand, in a terminal:

systemctl reboot --firmware-setup

It works only on a computer started in UEFI mode whose firmware offers it. Otherwise open the firmware settings with the maker's key at power-on, often Esc, Delete, F1, F2, F10, F11 or F12; the screen at power-on often names it.

## Turn Secure Boot off

The Faraday on the stick is not signed with Microsoft's keys, so a computer starts it only with Secure Boot off. In the firmware settings, find Secure Boot, usually on the Security, Boot or Authentication tab, and set it to Disabled. Save changes and exit.

Leave the Secure Boot keys as they are. Do not delete or clear them.

## Turn Fast Boot off where it hides USB

The firmware's Fast Boot makes some computers skip USB disks at power-on, so the stick is not offered. If the stick is missing from the boot menu, set Fast Boot to Disabled, or to Thorough where that is the choice, and save.

## Start from the stick

Plug the stick in and power on. Press the maker's boot menu key at power-on and choose the stick, or choose it in the boot override list in the firmware settings. Where the stick is listed twice, choose the entry that begins with UEFI.

## Afterwards: Secure Boot back on

Turn the computer off from Faraday and pull the stick out. Open the firmware settings, set Secure Boot back to Enabled, save and exit. If the firmware does not let Secure Boot be enabled, load the computer's own Secure Boot keys: on some firmware this is Restore Factory Keys or a Custom setting that loads the keys built into the computer.

## Where this comes from

systemd's manual pages systemctl(1), systemd-cryptenroll(1) and crypttab(5), from systemd's repository on GitHub (man/). Microsoft: "Boot to UEFI Mode or Legacy BIOS mode" and "Disabling Secure Boot" (learn.microsoft.com, Windows hardware manufacturing), for the firmware steps every maker shares. ASUS: booting from a USB flash drive (FAQ 1013017). Dell: booting an external device over USB Type-C (knowledge base 000131221). All read 2026-10-12. No maker's computer has been tried with these steps yet.
