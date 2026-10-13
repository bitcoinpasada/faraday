# Starting an Acer computer from the stick

## BitLocker first

On Windows, have the drive's BitLocker recovery key in hand before changing any firmware setting: Faraday Starter's BitLocker step on this computer shows it. Its extra-safe choice also suspends BitLocker until it is resumed. On Linux, have the disk's passphrase or recovery key first if the TPM unlocks the disk. The page "BitLocker and starting from a stick" says why.

## Keys

Firmware settings (BIOS Setup): F2, tapped as the Acer logo appears. Boot menu: F12, once it is turned on in BIOS Setup; on many Acer notebooks it is off until then, and some have none.

Faraday Starter's Restart into firmware settings opens BIOS Setup without a key.

## Turn Secure Boot off

On many Acer computers Secure Boot is greyed out until a supervisor password is set. In BIOS Setup, open the Security tab, choose Set Supervisor Password, and enter a password twice. Keep it written down until it is removed again.

Then open the Boot tab, set Secure Boot to Disabled, and press F10 to save and exit.

On Windows, choose the stick from the boot menu on the restart that follows the save, before Windows starts, and turn Secure Boot back on before Windows starts again. Windows started with Secure Boot off asks for the BitLocker recovery key, unless Faraday Starter suspended BitLocker.

## Turn Fast Boot off where it hides USB

Acer's articles do not describe a setting that hides USB disks from the boot menu.

## Start from the stick

Turn the F12 boot menu on first: in BIOS Setup, open the Main tab, set F12 Boot Menu to Enabled, and press F10 and Enter to save and restart.

Plug the stick in, restart, and press F12 as the Acer logo appears. Choose the stick. Where it is listed twice, choose the entry that begins with UEFI.

## Turn Secure Boot back on

Turn the computer off from Faraday and pull the stick out. Tap F2 at power-on and enter the supervisor password. Set Boot → Secure Boot to Enabled.

Then remove the supervisor password, as Acer recommends once Secure Boot is back on: Security → Set Supervisor Password, enter the current password, leave both new password fields empty, and confirm. Press F10 to save and exit.

Acer's articles do not describe restoring the Secure Boot keys. If Secure Boot does not stay on, the step on the general Windows page applies.

## Known quirks

Menu names and where they are differ between Acer models and BIOS versions; on some, the supervisor password is not needed.

## Where this comes from

Acer: "Enable or disable Secure Boot on an Acer notebook" (knowledge base article 88), "Enable or disable Secure Boot on an Acer desktop" (article 87) and "Why can't I get the F12 Boot Menu to work on my notebook or netbook?" (article 563), Acer's knowledge base on community.acer.com. All read 2026-10-12.
