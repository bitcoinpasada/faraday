# Starting a Lenovo IdeaPad, Yoga or Legion from the stick

## BitLocker first

On Windows, have the drive's BitLocker recovery key in hand before changing any firmware setting: Faraday Starter's BitLocker step on this computer shows it. Its extra-safe choice also suspends BitLocker until it is resumed. On Linux, have the disk's passphrase or recovery key first if the TPM unlocks the disk. The page "BitLocker and starting from a stick" says why.

## Keys

Firmware settings (BIOS): F2, or Fn+F2, pressed repeatedly right after turning the computer on. Boot menu: F12, or Fn+F12. It can take several tries.

The Novo button opens both from a computer that is off: a small button or pinhole marked with a curved arrow, on the side or near the power button, pressed with the tip of an unfolded paper clip. Its Novo Button Menu has BIOS Setup and Boot Menu.

Faraday Starter's Restart into firmware settings opens the BIOS without a key.

## Turn Secure Boot off

In the BIOS, open the Security tab and set Secure Boot to Disabled. Press F10, or Fn+F10, and choose Yes to save and exit.

On Windows, choose the stick from the boot menu on the restart that follows the save, before Windows starts, and turn Secure Boot back on before Windows starts again. Windows started with Secure Boot off asks for the BitLocker recovery key, unless Faraday Starter suspended BitLocker.

## Turn Fast Boot off where it hides USB

Lenovo's IdeaPad articles do not describe a setting that hides USB disks on computers from 2020 on. On computers from 2019 or before, the Boot tab has USB Boot, which has to be Enabled.

## Start from the stick

Plug the stick in, restart, and press F12, or Fn+F12, during power-on. Or, with the computer off, press the Novo button and choose Boot Menu. Choose the stick with the arrow keys and press Enter. Where it is listed twice, choose the entry that begins with UEFI.

## Turn Secure Boot back on

Turn the computer off from Faraday and pull the stick out. Open the BIOS with F2 or the Novo button, set Security → Secure Boot to Enabled, and save with F10.

Lenovo's IdeaPad articles do not describe restoring the Secure Boot keys. If Secure Boot does not stay on, the step on the general Windows page applies.

## Known quirks

Lenovo's article for booting from USB loads the BIOS defaults with F9 before changing Secure Boot. That sets every other BIOS setting back to its default as well.

The same article turns BitLocker off, which decrypts the drive. Faraday Starter's BitLocker step suspends it instead.

## Where this comes from

Lenovo: "Recommended way to enter BIOS - ideapad" (HT500216), "How to boot from USB disk in the BIOS (Boot Menu) - ideapad, Lenovo" (HT500207) and "Introduction to NOVO button - ideapad" (HT062552), on support.lenovo.com; "Get to know UEFI/BIOS" in a Legion user guide, on download.lenovo.com. All read 2026-10-12.
