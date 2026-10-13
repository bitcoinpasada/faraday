# Starting a Linux computer from the stick

## Before you start

If this computer's disk is unlocked by the TPM bound to PCR 7, Linux asks for the disk's passphrase or recovery key at the next start after Secure Boot is turned off. Have it before changing any firmware setting.

Such a disk has a line with tpm2-device in /etc/crypttab, and was set up with systemd-cryptenroll --tpm2-pcrs=7 or bound to PCR 7 another way. Faraday Starter shows this first when /etc/crypttab names a tpm2-device.

PCR 7 holds the Secure Boot state: it changes when Secure Boot is turned on or off and when its keys change, and the TPM releases the disk's key only for the state it was bound to. With Secure Boot back on and its keys as they were, the TPM unlocks the disk again.

A disk unlocked by a passphrase alone, or not encrypted, has nothing to do here.

## Steps

1. Plug the stick in.

2. Open the firmware settings with Faraday Starter's Restart into firmware settings.

By hand, in a terminal:

systemctl reboot --firmware-setup

That works only on a computer started in UEFI mode whose firmware offers it. Otherwise restart and press the maker's key at power-on, often Esc, Delete, F1, F2, F10, F11 or F12; the screen at power-on often names it.

3. Find Secure Boot, usually on the Security, Boot or Authentication tab, and set it to Disabled. Save changes and exit.

The Faraday on the stick is not signed with Microsoft's keys, so a computer starts it only with Secure Boot off.

Leave the Secure Boot keys as they are. Do not delete or clear them.

4. Saving restarts the computer. Press the maker's boot menu key at power-on and choose the stick. If it is listed twice, choose the one that starts with UEFI.

Without a boot menu key: choose the stick in the boot override list in the firmware settings.

If the stick is not listed: see "If something is different" below.

## Afterwards

5. Turn the computer off from Faraday and pull the stick out.

6. Open the firmware settings, set Secure Boot back to Enabled, save and exit.

If the firmware does not let Secure Boot be enabled: load the computer's own Secure Boot keys. On some firmware this is Restore Factory Keys or a Custom setting that loads the keys built into the computer.

## If something is different

Stick not in the boot menu: the firmware's Fast Boot makes some computers skip USB disks at power-on. Set Fast Boot to Disabled, or to Thorough where that is the choice, and save.

## Sources

systemd's manual pages systemctl(1), systemd-cryptenroll(1) and crypttab(5), from systemd's repository on GitHub (man/). Microsoft: "Boot to UEFI Mode or Legacy BIOS mode" and "Disabling Secure Boot" (learn.microsoft.com, Windows hardware manufacturing), for the firmware steps every maker shares. ASUS: booting from a USB flash drive (FAQ 1013017). Dell: booting an external device over USB Type-C (knowledge base 000131221). All read 2026-10-12. No maker's computer has been tried with these steps yet.
