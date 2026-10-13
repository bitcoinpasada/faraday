# Starting an MSI computer from the stick

## BitLocker first

On Windows, have the drive's BitLocker recovery key in hand before changing any firmware setting: Faraday Starter's BitLocker step on this computer shows it. Its extra-safe choice also suspends BitLocker until it is resumed. On Linux, have the disk's passphrase or recovery key first if the TPM unlocks the disk. The page "BitLocker and starting from a stick" says why.

## Keys

Firmware settings (BIOS): Delete, pressed when the MSI logo appears. Boot menu: F11, pressed when the MSI logo appears.

Faraday Starter's Restart into firmware settings opens the BIOS without a key.

## Turn Secure Boot off

In the BIOS, open Security → Secure Boot and set Secure Boot Support to Disabled. Press F10, Save & Reset, to save and restart.

::sketch msi-secure-boot

On Windows, choose the stick from the boot menu on the restart that follows the save, before Windows starts, and turn Secure Boot back on before Windows starts again. Windows started with Secure Boot off asks for the BitLocker recovery key, unless Faraday Starter suspended BitLocker.

## Turn Fast Boot off where it hides USB

MSI's notebook article does not describe a setting that hides USB disks from the boot menu.

## Start from the stick

Plug the stick in, restart, and press F11 when the MSI logo appears. Choose the stick. Where it is listed twice, choose the entry that begins with UEFI.

::sketch msi-boot-menu

## Turn Secure Boot back on

Turn the computer off from Faraday and pull the stick out. Press Delete when the MSI logo appears, set Security → Secure Boot → Secure Boot Support to Enabled, and press F10.

MSI's notebook article does not describe restoring the Secure Boot keys. If Secure Boot does not stay on, the step on the general Windows page applies.

## Known quirks

These steps are from MSI's article for its notebooks, which does not cover desktops or motherboards.

## Where this comes from

Where Faraday shows a picture of a firmware screen, it is a mock-up: the real screen may look different.

MSI: "[Troubleshooting] Secure Boot Violation Error at Startup" (FAQ 11370, which applies to all MSI notebooks), on msi.com/faq. Read 2026-10-12, and again, with its screen images, 2026-10-13.
