# Starting a Framework computer from the stick

## Before you start

On Windows, have the BitLocker recovery key ready: Faraday Starter's BitLocker step shows it. On Linux, have the disk's passphrase or recovery key ready.

Framework's guides say to turn BitLocker off before turning Secure Boot off. Faraday Starter's BitLocker step suspends it instead.

## Steps

1. Plug the stick in.

2. Restart the computer.

3. Tap F2 repeatedly from the moment the computer turns on, while the Framework logo is on screen. A press that comes too late misses the BIOS.

If it doesn't open: use Faraday Starter's Restart into the BIOS.

4. Choose Administer Secure Boot and set Enforce Secure Boot to Disabled with the arrow keys. Press F10 to save and restart.

::sketch framework-front-page

::sketch framework-secure-boot

5. The computer restarts. Tap F12 repeatedly as soon as the power light comes on. On Windows, do it on this restart, before Windows starts.

6. Choose the stick: it is listed as EFI USB Device, under EFI Boot Devices.

::sketch framework-boot-menu

With no system on the internal drive, the computer starts from the stick without the menu.

If the stick is not listed: see "If something is different" below.

## Afterwards

7. Turn the computer off from Faraday and pull the stick out.

8. Tap F2 at power-on, choose Administer Secure Boot, set Enforce Secure Boot to Enabled, and press F10. Do it before Windows starts: with Secure Boot off, Windows asks for the BitLocker recovery key unless Faraday Starter suspended BitLocker.

If Secure Boot does not stay on: Framework's guides do not describe restoring the Secure Boot keys; the step on the general Windows page applies.

## If something is different

Stick not in the F12 list: Framework's guides do not describe a setting that hides USB disks from the boot menu.

## Sources

Where Faraday shows a picture of a firmware screen, it is a mock-up: the real screen may look different.

Framework: "Arch Linux on the Framework Laptop 13", "Arch Linux on the Framework Laptop 16" and "Bazzite Installation on the Framework Laptop 13", on guides.frame.work, read 2026-10-12. "Enabling Battery Saver Functionality in BIOS" and the first and third again, with their screen images, on guides.frame.work, read 2026-10-13.
