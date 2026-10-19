# Starting a Lenovo IdeaPad, Yoga or Legion from the stick

## Before you start

On Linux, have the disk's passphrase or recovery key ready if it is unlocked by the TPM.

## Steps

1. Plug the stick in.

2. Restart the computer.

3. Right after it turns on, press F2, or Fn+F2, repeatedly. It can take several tries.

If it doesn't open: with the computer off, press the Novo button and choose BIOS Setup, or use Faraday Starter's Restart into the BIOS. The Novo button is a small button or pinhole marked with a curved arrow, on the side or near the power button, pressed with the tip of an unfolded paper clip.

4. Open the Security tab and set Secure Boot to Disabled. Press F10, or Fn+F10, and choose Yes to save and exit.

::sketch ideapad-secure-boot

::sketch ideapad-save

If Secure Boot cannot be changed: Device Guard, on the same tab, is Enabled, and Secure Boot cannot be changed until it is not.

If the Secure Boot setting cannot be found: exit the BIOS without saving changes. Windows starts as before.

5. The computer restarts. Press F12, or Fn+F12, during power-on. On Windows, do it on this restart, before Windows starts.

From a computer that is off: press the Novo button and choose Boot Menu.

6. Choose the stick with the arrow keys and press Enter. If it is listed twice, choose the one that starts with EFI USB Device; the other is for legacy BIOS mode.

::sketch ideapad-boot-menu

If the stick is not listed: see "If something is different" below.

## Afterwards

7. Turn the computer off from Faraday and pull the stick out.

8. Turn Secure Boot back on: open the BIOS with F2 or the Novo button, set Security → Secure Boot to Enabled, and save with F10. Do it before Windows starts.

If Secure Boot does not stay on: Lenovo's IdeaPad articles do not describe restoring the Secure Boot keys; the step on the general Windows page applies.

## If something is different

The BitLocker recovery key: with Secure Boot off, Windows cannot start without it. It is not needed here: the last step turns Secure Boot back on before Windows starts. To see it on this computer anyway: in Windows, Control Panel → BitLocker Drive Encryption → Back up your recovery key; on Windows Home, manage-bde -protectors -get C: in a terminal run as administrator. On a computer signed in with a Microsoft account it is also at aka.ms/myrecoverykey. If Windows asks for it: turn the computer off and turn Secure Boot back on; Windows then starts without it.

Stick not in the F12 list: on the Boot tab, set USB Boot to Enabled. Lenovo's IdeaPad articles do not describe a setting that hides USB disks.

::sketch ideapad-usb-boot

Lenovo's article for booting from USB loads the BIOS defaults with F9 before changing Secure Boot. That sets every other BIOS setting back to its default as well.

The same article turns BitLocker off, which decrypts the drive. That is not needed: turn Secure Boot back on before Windows starts.

## Sources

Where Faraday shows a picture of a firmware screen, it is a mock-up: the real screen may look different.

Lenovo: "Recommended way to enter BIOS - ideapad" (HT500216), "How to boot from USB disk in the BIOS (Boot Menu) - ideapad, Lenovo" (HT500207) and "Introduction to NOVO button - ideapad" (HT062552), on support.lenovo.com; "Get to know UEFI/BIOS" in a Legion user guide, on download.lenovo.com, read 2026-10-12. Lenovo BIOS Simulator Center, on download.lenovo.com, read 2026-10-13.
