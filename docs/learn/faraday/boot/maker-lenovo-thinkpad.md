# Starting a Lenovo ThinkPad from the stick

## BitLocker first

On Windows, do the BitLocker step in the Faraday app on this computer before changing any firmware setting. On Linux, have the disk's passphrase or recovery key first if the TPM unlocks the disk. The page "BitLocker and starting from a stick" says why.

## Keys

Firmware settings (BIOS Setup): tap F1 repeatedly when the Lenovo or ThinkPad logo appears. Boot menu: F12 as soon as the ThinkPad logo appears. On some models, press Enter at the logo instead; the Startup Interrupt Menu opens, and F1 there opens BIOS Setup.

With Windows' Fast Startup on, Shut down does not close Windows and the keys can be missed. Restart instead, or hold Shift while choosing Shut down for a full shut down. The Faraday app's Restart into firmware settings opens BIOS Setup without a key.

## Turn Secure Boot off

In BIOS Setup, open the Security tab, then Secure Boot, and set Secure Boot to Disabled (Off on some models). Press F10 to save and restart.

## Turn Fast Boot off where it hides USB

Lenovo's ThinkPad articles do not describe a setting that hides USB disks from the boot menu.

## Start from the stick

Plug the stick in, turn the computer on, and press F12 as soon as the ThinkPad logo appears. Choose the stick with the arrow keys and press Enter. Where it is listed twice, choose the entry that begins with UEFI.

## Turn Secure Boot back on

Turn the computer off from Faraday and pull the stick out. Tap F1 at power-on, set Security → Secure Boot → Secure Boot to Enabled (On), and press F10.

If Secure Boot shows Disabled and cannot be changed, Platform Mode is Setup Mode. Choose Restore Factory Keys on the same Secure Boot page, which returns Platform Mode to User Mode, then set Secure Boot to Enabled and restart.

## Known quirks

Leave every other setting on the Secure Boot page as it is: the stick needs Secure Boot off, not its keys changed.

## Where this comes from

Lenovo: "How to enable Secure Boot on Think branded systems - ThinkPad, ThinkStation, ThinkCentre" (HT509044), "Secure Boot Mode cannot be set - ThinkPad" (HT515493), "Recommended ways to enter BIOS - ThinkPad, ThinkCentre, ThinkStation" (HT500222) and "How to boot from a USB Drive - ThinkPad" (HT118361), all on support.lenovo.com. All read 2026-10-12.
