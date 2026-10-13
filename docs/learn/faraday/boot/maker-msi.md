# Starting an MSI computer from the stick

## BitLocker first

On Windows, do the BitLocker step in Faraday Starter on this computer before changing any firmware setting. On Linux, have the disk's passphrase or recovery key first if the TPM unlocks the disk. The page "BitLocker and starting from a stick" says why.

## Keys

Firmware settings (BIOS): Delete, pressed when the MSI logo appears. Boot menu: F11, pressed when the MSI logo appears.

Faraday Starter's Restart into firmware settings opens the BIOS without a key.

## Turn Secure Boot off

In the BIOS, open Security → Secure Boot and set it to Disable. Press F10 to save and restart.

## Turn Fast Boot off where it hides USB

MSI's notebook article does not describe a setting that hides USB disks from the boot menu.

## Start from the stick

Plug the stick in, restart, and press F11 when the MSI logo appears. Choose the stick. Where it is listed twice, choose the entry that begins with UEFI.

## Turn Secure Boot back on

Turn the computer off from Faraday and pull the stick out. Press Delete when the MSI logo appears, set Security → Secure Boot to Enable, and press F10.

MSI's notebook article does not describe restoring the Secure Boot keys. If Secure Boot does not stay on, the step on the general Windows page applies.

## Known quirks

These steps are from MSI's article for its notebooks, which does not cover desktops or motherboards.

## Where this comes from

MSI: "[Troubleshooting] Secure Boot Violation Error at Startup" (FAQ 11370, which applies to all MSI notebooks), on msi.com/faq. Read 2026-10-12.
