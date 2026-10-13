# Starting an Acer computer from the stick

## Before you start

On Windows, have the BitLocker recovery key ready: Faraday Starter's BitLocker step shows it. On Linux, have the disk's passphrase or recovery key ready.

## Steps

1. Plug the stick in.

2. Restart the computer.

3. When the Acer logo appears, tap F2.

If it doesn't open: use Faraday Starter's Restart into firmware settings.

4. Open the Security tab, choose Set Supervisor Password, and enter a password twice. Write it down: step 10 removes it again.

::sketch acer-supervisor

On many Acer computers Secure Boot is greyed out until a supervisor password is set; on some, the password is not needed.

5. Open the Main tab and set F12 Boot Menu to Enabled.

::sketch acer-boot-menu-key

On many Acer notebooks the F12 boot menu is off until it is turned on here, and some have none.

6. Open the Boot tab, set Secure Boot to Disabled, and press F10 and Enter to save and exit.

::sketch acer-secure-boot

7. The computer restarts. Press F12 as the Acer logo appears. On Windows, do it on this restart, before Windows starts.

8. Choose the stick. If it is listed twice, choose the one that starts with UEFI.

If the stick is not listed: see "If something is different" below.

## Afterwards

9. Turn the computer off from Faraday and pull the stick out.

10. Tap F2 at power-on and enter the supervisor password. Set Boot → Secure Boot to Enabled. Then remove the supervisor password, as Acer recommends once Secure Boot is back on: Security → Set Supervisor Password, enter the current password, leave both new password fields empty, and confirm. Press F10 to save and exit. Do it before Windows starts: with Secure Boot off, Windows asks for the BitLocker recovery key unless Faraday Starter suspended BitLocker.

If Secure Boot does not stay on: Acer's articles do not describe restoring the Secure Boot keys; the step on the general Windows page applies.

## If something is different

Stick not in the F12 list: Acer's articles do not describe a setting that hides USB disks from the boot menu.

Menu names and where they are differ between Acer models and BIOS versions.

## Sources

Where Faraday shows a picture of a firmware screen, it is a mock-up: the real screen may look different.

Acer: "Enable or disable Secure Boot on an Acer notebook" (knowledge base article 88), "Enable or disable Secure Boot on an Acer desktop" (article 87) and "Why can't I get the F12 Boot Menu to work on my notebook or netbook?" (article 563), Acer's knowledge base on community.acer.com. All read 2026-10-12.
