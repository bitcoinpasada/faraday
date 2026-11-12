# Starting an MSI computer from the stick

**Kind:** Reference
**Status:** Checked against the app

## Before you start

On Linux, have the disk's passphrase or recovery key ready if it is unlocked by the TPM.

These steps are from MSI's article for its notebooks, which does not cover desktops or motherboards.

## Steps

1. Plug the stick in.

2. Restart the computer.

3. When the MSI logo appears, press Delete.

If it doesn't open: use Faraday Starter's Restart into the BIOS.

4. Look for the tab named Security, then the entry Secure Boot, and set Secure Boot Support to Disabled. Press F10, Save & Reset, to save and restart.

If the Secure Boot setting cannot be found: exit the BIOS without saving changes. Windows starts as before.

## Choosing the stick

5. The computer restarts. Press F11 when the MSI logo appears. On Windows, do it on this restart, before Windows starts.

6. In the list headed Please select boot device, choose the stick. If it is listed twice, choose the one that starts with UEFI.

The stick is listed under the name printed on it (SanDisk, Kingston…), often after "UEFI:", not as Windows Boot Manager, a Linux name such as ubuntu, or the computer's own disk. If it is not listed: see "If the stick is not listed" below.

## Afterwards

7. Turn the computer off from Faraday and pull the stick out.

8. Turn Secure Boot back on: press Delete when the MSI logo appears, set Security → Secure Boot → Secure Boot Support to Enabled, and press F10. Do it before Windows starts.

If Secure Boot does not stay on: MSI's notebook article does not describe restoring the Secure Boot keys; the step on the general Windows page applies.

## If something is different

The BitLocker recovery key: Windows asks for it if it starts with Secure Boot off, and on some computers even after Secure Boot is back on. Faraday Starter's Temporarily suspend BitLocker, before the restart, suspends it for the next 3 starts, after which Windows turns it back on by itself. The key is at Control Panel → BitLocker Drive Encryption → Back up your recovery key; on Windows Home, manage-bde -protectors -get C: as administrator; at aka.ms/myrecoverykey with a Microsoft account; or on Faraday Starter's sheet. If Windows asks while Secure Boot is off: turn the computer off and turn Secure Boot back on; Windows then starts without the key.

## If the stick is not listed

Stick not in the F11 list: MSI's notebook article does not describe a setting that hides USB disks from the boot menu.

Menu names differ between models and BIOS versions; MSI's article covers notebooks only.

## Sources

MSI: "[Troubleshooting] Secure Boot Violation Error at Startup" (FAQ 11370, which applies to all MSI notebooks), on msi.com/faq. Read 2026-10-12, and again, with its screen images, 2026-10-13.
