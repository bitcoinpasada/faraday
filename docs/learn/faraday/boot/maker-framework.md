# Starting a Framework computer from the stick

## Before you start

On Linux, have the disk's passphrase or recovery key ready if it is unlocked by the TPM.

Framework's guides say to turn BitLocker off before turning Secure Boot off. That is not needed: turn Secure Boot back on before Windows starts.

## Steps

1. Plug the stick in.

2. Restart the computer.

3. Tap F2 repeatedly from the moment the computer turns on, while the Framework logo is on screen. A press that comes too late misses the BIOS.

If it doesn't open: use Faraday Starter's Restart into the BIOS.

4. Choose Administer Secure Boot and set Enforce Secure Boot to Disabled with the arrow keys. Press F10 to save and restart.

::sketch framework-front-page

::sketch framework-secure-boot

If the Secure Boot setting cannot be found: exit the BIOS without saving changes. Windows starts as before.

5. The computer restarts. Tap F12 repeatedly as soon as the power light comes on. On Windows, do it on this restart, before Windows starts.

6. Choose the stick: it is listed as EFI USB Device, under EFI Boot Devices.

::sketch framework-boot-menu

With no system on the internal drive, the computer starts from the stick without the menu.

If the stick is not listed: see "If something is different" below.

## Afterwards

7. Turn the computer off from Faraday and pull the stick out.

8. Turn Secure Boot back on: tap F2 at power-on, choose Administer Secure Boot, set Enforce Secure Boot to Enabled, and press F10. Do it before Windows starts.

If Secure Boot does not stay on: Framework's guides do not describe restoring the Secure Boot keys; the step on the general Windows page applies.

## If something is different

The BitLocker recovery key: with Secure Boot off, Windows cannot start without it. It is not needed here: the last step turns Secure Boot back on before Windows starts. To see it on this computer anyway: in Windows, Control Panel → BitLocker Drive Encryption → Back up your recovery key; on Windows Home, manage-bde -protectors -get C: in a terminal run as administrator. On a computer signed in with a Microsoft account it is also at aka.ms/myrecoverykey. If Windows asks for it: turn the computer off and turn Secure Boot back on; Windows then starts without it.

Stick not in the F12 list: Framework's guides do not describe a setting that hides USB disks from the boot menu.

## Sources

Where Faraday shows a picture of a firmware screen, it is a mock-up: the real screen may look different.

Framework: "Arch Linux on the Framework Laptop 13", "Arch Linux on the Framework Laptop 16" and "Bazzite Installation on the Framework Laptop 13", on guides.frame.work, read 2026-10-12. "Enabling Battery Saver Functionality in BIOS" and the first and third again, with their screen images, on guides.frame.work, read 2026-10-13.
