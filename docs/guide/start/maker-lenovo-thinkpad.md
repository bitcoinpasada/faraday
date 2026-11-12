# Starting a Lenovo ThinkPad from the stick

**Kind:** Reference
**Status:** Checked against the app

## Before you start

On Linux, have the disk's passphrase or recovery key ready if it is unlocked by the TPM.

## Steps

1. Plug the stick in.

2. Restart the computer. Use Restart, not Shut down: with Windows' Fast Startup on, Shut down can skip the key. Holding Shift while choosing Shut down also shuts down fully.

3. When the Lenovo or ThinkPad logo appears, tap F1 repeatedly.

If it doesn't open: press Enter at the logo, then F1 in the Startup Interrupt Menu, or use Faraday Starter's Restart into the BIOS.

4. Look for the menu named Security in the list, then the page named Secure Boot, and set Secure Boot to Disabled (Off on some models). Press F10 and choose Yes to save and restart; Restart → Exit Saving Changes does the same.

If the Secure Boot setting cannot be found: exit the BIOS without saving changes. Windows starts as before.

## The Secure Boot page

If Secure Boot cannot be changed: Device Guard, on the Security tab, is Enabled, and Secure Boot cannot be changed until it is not.

Leave every other setting on the Secure Boot page as it is: the stick needs Secure Boot off, not its keys changed.

## Choosing the stick

5. The computer restarts. Press F12 as soon as the ThinkPad logo appears. On Windows, do it on this restart, before Windows starts.

6. Look for the stick in the Startup Interrupt boot list, choose it with the arrow keys and press Enter. If it is listed twice, choose the one that starts with UEFI.

The stick is listed under the name printed on it (SanDisk, Kingston…), often after "UEFI:", not as Windows Boot Manager, a Linux name such as ubuntu, or the computer's own disk. If it is not listed: see "If the stick is not listed" below.

## Afterwards

7. Turn the computer off from Faraday and pull the stick out.

8. Turn Secure Boot back on: tap F1 at power-on, set Security → Secure Boot → Secure Boot to Enabled (On), and press F10. Do it before Windows starts.

If Secure Boot shows Disabled and cannot be changed: Secure Boot Mode is Setup Mode (Platform Mode on older models). Choose Restore Factory Keys on the same Secure Boot page, which returns it to User Mode, then set Secure Boot to Enabled and restart.

## If something is different

The BitLocker recovery key: Windows asks for it if it starts with Secure Boot off, and on some computers even after Secure Boot is back on. Faraday Starter's Temporarily suspend BitLocker, before the restart, suspends it for the next 3 starts, after which Windows turns it back on by itself. The key is at Control Panel → BitLocker Drive Encryption → Back up your recovery key; on Windows Home, manage-bde -protectors -get C: as administrator; at aka.ms/myrecoverykey with a Microsoft account; or on Faraday Starter's sheet. If Windows asks while Secure Boot is off: turn the computer off and turn Secure Boot back on; Windows then starts without the key.

## If the stick is not listed

Stick not in the F12 list: Lenovo's ThinkPad articles do not describe a setting that hides USB disks from the boot menu.

Menu names differ between models and BIOS versions. The menu named Startup holds the boot settings; Secure Boot is under Security.

## Sources

Lenovo: "How to enable Secure Boot on Think branded systems - ThinkPad, ThinkStation, ThinkCentre" (HT509044), "Secure Boot Mode cannot be set - ThinkPad" (HT515493), "Recommended ways to enter BIOS - ThinkPad, ThinkCentre, ThinkStation" (HT500222) and "How to boot from a USB Drive - ThinkPad" (HT118361), all on support.lenovo.com, read 2026-10-12. Lenovo BIOS Simulator Center, on download.lenovo.com, read 2026-10-13.
