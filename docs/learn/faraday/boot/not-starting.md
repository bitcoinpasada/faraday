# When the stick does not start

## Not in the boot menu

Fast Boot. The firmware's Fast Boot makes some computers skip USB disks at power-on. Set it to Disabled, or to Thorough where that is the choice.

USB boot turned off. Some firmware has a setting that allows starting from USB at all, such as Enable USB Boot Support. Turn it on.

A USB-C or Thunderbolt port. On some computers these ports start nothing until the firmware's Thunderbolt or USB-C boot support is turned on. Use a USB-A port, or turn that setting on.

Legacy mode or CSM. The stick starts in UEFI mode only. A computer set to start in legacy BIOS or CSM mode does not offer it: set the firmware's boot mode to UEFI. Turning CSM on does not help. Where the stick is listed twice, choose the entry that begins with UEFI.

Not a PC with an Intel or AMD processor. A computer with an ARM processor, such as a Windows laptop with a Snapdragon processor, cannot start the stick.

## Chosen, but nothing starts

If choosing the stick returns to the menu or shows a Secure Boot message, Secure Boot is still on. Turn it off as the Windows or Linux page says.

## A black screen

Faraday shows its logo a few seconds after it starts, then its first screen. A screen that stays black with no logo means the firmware did not start the stick: check that Secure Boot is off, and try another USB port. A logo followed by a screen that stays black for more than a minute means Faraday did not start its screen on this computer: hold the power button until the computer turns off.

## A firmware password

If the firmware settings ask for a password that you do not have, set by an employer or a previous owner, only whoever set it can open them. Without it Secure Boot stays on, and this computer cannot start the stick.

## Where this comes from

Microsoft: "Boot to UEFI Mode or Legacy BIOS mode" and "Disabling Secure Boot" (learn.microsoft.com, Windows hardware manufacturing); "BitLocker countermeasures" (learn.microsoft.com, Windows security). ASUS: booting from a USB flash drive (FAQ 1013017). Dell: booting an external device over USB Type-C (knowledge base 000131221). All read 2026-10-12. No maker's computer has been tried with these steps yet.
