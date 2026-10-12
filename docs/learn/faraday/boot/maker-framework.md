# Starting a Framework computer from the stick

## BitLocker first

On Windows, do the BitLocker step in the Faraday app on this computer before changing any firmware setting. On Linux, have the disk's passphrase or recovery key first if the TPM unlocks the disk. The page "BitLocker and starting from a stick" says why.

## Keys

Firmware settings (BIOS): tap F2 repeatedly right after turning the computer on, at the Framework logo. Boot menu: tap F12 repeatedly as soon as the power light comes on.

The Faraday app's Restart into firmware settings opens the BIOS without a key.

## Turn Secure Boot off

In the BIOS, choose Administer Secure Boot and set Enforce Secure Boot to Disabled with the arrow keys. Press F10 to save and restart.

## Turn Fast Boot off where it hides USB

Framework's guides do not describe a setting that hides USB disks from the boot menu.

## Start from the stick

Plug the stick in, turn the computer on, and tap F12 as soon as the power light comes on. Choose the stick. Where it is listed twice, choose the entry that begins with UEFI. With no system on the internal drive, the computer starts from the stick without the menu.

## Turn Secure Boot back on

Turn the computer off from Faraday and pull the stick out. Tap F2 at power-on, choose Administer Secure Boot, set Enforce Secure Boot to Enabled, and press F10.

Framework's guides do not describe restoring the Secure Boot keys. If Secure Boot does not stay on, the step on the general Windows page applies.

## Known quirks

F2 is tapped while the Framework logo is on screen, from the moment the computer is turned on; a press that comes too late misses the BIOS.

Framework's guides say to turn BitLocker off before turning Secure Boot off. The Faraday app's BitLocker step suspends it instead.

## Where this comes from

Framework: "Arch Linux on the Framework Laptop 13", "Arch Linux on the Framework Laptop 16" and "Bazzite Installation on the Framework Laptop 13", on guides.frame.work. All read 2026-10-12.
