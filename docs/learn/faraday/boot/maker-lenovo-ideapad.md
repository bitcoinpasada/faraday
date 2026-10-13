# Starting a Lenovo IdeaPad, Yoga or Legion from the stick

## Before you start

On Windows, have the BitLocker recovery key ready: Faraday Starter's BitLocker step shows it. On Linux, have the disk's passphrase or recovery key ready.

## Steps

1. Plug the stick in.

2. Restart the computer.

3. Right after it turns on, press F2, or Fn+F2, repeatedly. It can take several tries.

If it doesn't open: with the computer off, press the Novo button and choose BIOS Setup, or use Faraday Starter's Restart into firmware settings. The Novo button is a small button or pinhole marked with a curved arrow, on the side or near the power button, pressed with the tip of an unfolded paper clip.

4. Open the Security tab and set Secure Boot to Disabled. Press F10, or Fn+F10, and choose Yes to save and exit.

::sketch ideapad-secure-boot

::sketch ideapad-save

If Secure Boot cannot be changed: Device Guard, on the same tab, is Enabled, and Secure Boot cannot be changed until it is not.

5. The computer restarts. Press F12, or Fn+F12, during power-on. On Windows, do it on this restart, before Windows starts.

From a computer that is off: press the Novo button and choose Boot Menu.

6. Choose the stick with the arrow keys and press Enter. If it is listed twice, choose the one that starts with EFI USB Device; the other is for legacy BIOS mode.

::sketch ideapad-boot-menu

If the stick is not listed: see "If something is different" below.

## Afterwards

7. Turn the computer off from Faraday and pull the stick out.

8. Open the BIOS with F2 or the Novo button, set Security → Secure Boot to Enabled, and save with F10. Do it before Windows starts: with Secure Boot off, Windows asks for the BitLocker recovery key unless Faraday Starter suspended BitLocker.

If Secure Boot does not stay on: Lenovo's IdeaPad articles do not describe restoring the Secure Boot keys; the step on the general Windows page applies.

## If something is different

Stick not in the F12 list: on the Boot tab, set USB Boot to Enabled. Lenovo's IdeaPad articles do not describe a setting that hides USB disks.

::sketch ideapad-usb-boot

Lenovo's article for booting from USB loads the BIOS defaults with F9 before changing Secure Boot. That sets every other BIOS setting back to its default as well.

The same article turns BitLocker off, which decrypts the drive. Faraday Starter's BitLocker step suspends it instead.

## Sources

Where Faraday shows a picture of a firmware screen, it is a mock-up: the real screen may look different.

Lenovo: "Recommended way to enter BIOS - ideapad" (HT500216), "How to boot from USB disk in the BIOS (Boot Menu) - ideapad, Lenovo" (HT500207) and "Introduction to NOVO button - ideapad" (HT062552), on support.lenovo.com; "Get to know UEFI/BIOS" in a Legion user guide, on download.lenovo.com, read 2026-10-12. Lenovo BIOS Simulator Center, on download.lenovo.com, read 2026-10-13.
