# Starting an ASUS computer from the stick

## BitLocker first

On Windows, do the BitLocker step in Faraday Starter on this computer before changing any firmware setting. On Linux, have the disk's passphrase or recovery key first if the TPM unlocks the disk. The page "BitLocker and starting from a stick" says why.

## Keys

Firmware settings (BIOS): with the computer fully shut down, hold F2 and press the power button; let go of F2 when the BIOS screen appears. Boot menu: the same with Esc. Inside the BIOS, F8 or Boot Menu opens the boot menu, and F7 switches to Advanced Mode.

Faraday Starter's Restart into firmware settings opens the BIOS without a key.

## Turn Secure Boot off

Notebooks: in the BIOS, press F7 for Advanced Mode (Advanced Settings in MyASUS in UEFI). Open the Security screen, then Secure Boot, and set Secure Boot Control to Disabled. Press F10 and confirm, or choose Save Changes and Exit on the Save & Exit page.

Desktops: Boot → Secure Boot, and set OS Type to Other OS. Save and exit.

## Turn Fast Boot off where it hides USB

If the stick is missing from the boot menu, open the Boot screen in Advanced Mode, set Fast Boot to Disabled, and save with F10. On desktops it is Boot → Boot Configuration → Fast Boot.

## Start from the stick

Plug the stick in. With the computer shut down, hold Esc and press the power button, let go when the boot menu appears, choose the stick, and press Enter. If the menu does not appear, open the BIOS and press F8, or choose Boot Menu. Where the stick is listed twice, choose the entry that begins with UEFI.

## Turn Secure Boot back on

Turn the computer off from Faraday and pull the stick out. Open the BIOS with F2, set Security → Secure Boot → Secure Boot Control to Enabled (on desktops, OS Type to Windows UEFI mode), and save with F10.

If Secure Boot shows Not Active after that: with Secure Boot Control Enabled, open Key Management, choose Restore Factory Keys and confirm with Yes, then save with F10.

## Known quirks

ASUS's restore steps also list Reset To Setup Mode before Restore Factory Keys; Reset To Setup Mode removes the keys. Use Restore Factory Keys alone first.

Leave Launch CSM as it is: the stick starts in UEFI mode, and CSM is for legacy BIOS mode.

## Where this comes from

ASUS: "How to Enable/Disable Secure Boot" (FAQ 1050047), "How to Boot from a USB Flash Drive/CD-ROM (Changing Boot Options)" (FAQ 1013017) and "[Notebook] How to disable Fast Boot in BIOS configuration" (FAQ 1044641), on asus.com/support. All read 2026-10-12.
