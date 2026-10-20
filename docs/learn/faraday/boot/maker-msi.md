# Starting an MSI computer from the stick

## Before you start

On Linux, have the disk's passphrase or recovery key ready if it is unlocked by the TPM.

These steps are from MSI's article for its notebooks, which does not cover desktops or motherboards.

## Steps

1. Plug the stick in.

2. Restart the computer.

3. When the MSI logo appears, press Delete.

If it doesn't open: use Faraday Starter's Restart into the BIOS.

4. Open Security → Secure Boot and set Secure Boot Support to Disabled. Press F10, Save & Reset, to save and restart.

::sketch msi-secure-boot

If the Secure Boot setting cannot be found: exit the BIOS without saving changes. Windows starts as before.

5. The computer restarts. Press F11 when the MSI logo appears. On Windows, do it on this restart, before Windows starts.

6. Choose the stick. If it is listed twice, choose the one that starts with UEFI.

::sketch msi-boot-menu

The stick is usually listed under its maker's name, the one printed on it (SanDisk, Kingston…), often after "UEFI:". It is not Windows Boot Manager, a Linux name such as ubuntu, or the computer's own disk.

If the stick is not listed: see "If something is different" below.

## Afterwards

7. Turn the computer off from Faraday and pull the stick out.

8. Turn Secure Boot back on: press Delete when the MSI logo appears, set Security → Secure Boot → Secure Boot Support to Enabled, and press F10. Do it before Windows starts.

If Secure Boot does not stay on: MSI's notebook article does not describe restoring the Secure Boot keys; the step on the general Windows page applies.

## If something is different

The BitLocker recovery key: with Secure Boot off, Windows cannot start without it. Windows may ask for it even after Secure Boot is back on, on a computer whose BIOS changes its Secure Boot keys when the setting is turned off and on. Suspending BitLocker first avoids that: Faraday Starter's Temporarily suspend BitLocker, before the restart, suspends it for the next 3 times Windows starts, after which Windows turns it back on by itself. To see the key on this computer: in Windows, Control Panel → BitLocker Drive Encryption → Back up your recovery key; on Windows Home, manage-bde -protectors -get C: in a terminal run as administrator; Faraday Starter prints it on its sheet when asked to. On a computer signed in with a Microsoft account it is also at aka.ms/myrecoverykey. If Windows asks for it while Secure Boot is still off: turn the computer off and turn Secure Boot back on; Windows then starts without it.

Stick not in the F11 list: MSI's notebook article does not describe a setting that hides USB disks from the boot menu.

## Sources

Where Faraday shows a picture of a firmware screen, it is a mock-up: the real screen may look different.

MSI: "[Troubleshooting] Secure Boot Violation Error at Startup" (FAQ 11370, which applies to all MSI notebooks), on msi.com/faq. Read 2026-10-12, and again, with its screen images, 2026-10-13.
