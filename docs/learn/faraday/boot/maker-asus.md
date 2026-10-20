# Starting an ASUS computer from the stick

## Before you start

On Linux, have the disk's passphrase or recovery key ready if it is unlocked by the TPM.

## Steps

1. Plug the stick in.

2. Shut the computer down fully.

3. Hold F2 and press the power button. Let go of F2 when the BIOS screen appears.

If it doesn't open: use Faraday Starter's Restart into the BIOS.

4. Notebooks: press F7 for Advanced Mode, open Security → Secure Boot, and set Secure Boot Control to Disabled. Press F10 and confirm: Save & Reset on newer notebooks, Save & Exit Setup on older ones.

::sketch asus-secure-boot

::sketch asus-save

In MyASUS in UEFI, Advanced Mode is Advanced Settings.

Desktops: Boot → Secure Boot, and set OS Type to Other OS. Press F10 and choose Ok in the Save Changes & Reset box; the last tab is Exit.

::sketch asus-desktop-secure-boot

::sketch asus-desktop-save

Leave Launch CSM as it is: the stick starts in UEFI mode, and CSM is for legacy BIOS mode.

If the Secure Boot setting cannot be found: exit the BIOS without saving changes. Windows starts as before.

5. Hold Esc as the computer starts again, and let go when the boot menu appears. On Windows, do it on this restart, before Windows starts.

From a computer that is shut down: hold Esc and press the power button. If the menu does not appear: open the BIOS and press F8, or choose Boot Menu.

6. Choose the stick and press Enter. If it is listed twice, choose the one that starts with UEFI.

::sketch asus-boot-menu

The stick is usually listed under its maker's name, the one printed on it (SanDisk, Kingston…), often after "UEFI:". It is not Windows Boot Manager, a Linux name such as ubuntu, or the computer's own disk.

If the stick is not listed: see "If something is different" below.

## Afterwards

7. Turn the computer off from Faraday and pull the stick out.

8. Turn Secure Boot back on: open the BIOS with F2, set Security → Secure Boot → Secure Boot Control to Enabled (on desktops, OS Type to Windows UEFI mode), and save with F10. Do it before Windows starts.

If Secure Boot shows Not Active after that: with Secure Boot Control Enabled, open Key Management, choose Restore Factory Keys and confirm with Yes, then save with F10.

ASUS's steps also list Reset To Setup Mode before Restore Factory Keys. Reset To Setup Mode removes the keys: use Restore Factory Keys alone first.

## If something is different

The BitLocker recovery key: with Secure Boot off, Windows cannot start without it. Windows may ask for it even after Secure Boot is back on, on a computer whose BIOS changes its Secure Boot keys when the setting is turned off and on. Suspending BitLocker first avoids that: Faraday Starter's Temporarily suspend BitLocker, before the restart, suspends it for the next 3 times Windows starts, after which Windows turns it back on by itself. To see the key on this computer: in Windows, Control Panel → BitLocker Drive Encryption → Back up your recovery key; on Windows Home, manage-bde -protectors -get C: in a terminal run as administrator; Faraday Starter prints it on its sheet when asked to. On a computer signed in with a Microsoft account it is also at aka.ms/myrecoverykey. If Windows asks for it while Secure Boot is still off: turn the computer off and turn Secure Boot back on; Windows then starts without it.

Stick not in the boot menu: open the Boot screen in Advanced Mode, set Fast Boot to Disabled, and save with F10. On desktops it is Boot → Boot Configuration → Fast Boot.

## Sources

Where Faraday shows a picture of a firmware screen, it is a mock-up: the real screen may look different.

ASUS: "How to Enable/Disable Secure Boot" (FAQ 1050047), "How to Boot from a USB Flash Drive/CD-ROM (Changing Boot Options)" (FAQ 1013017) and "[Notebook] How to disable Fast Boot in BIOS configuration" (FAQ 1044641), on asus.com/support. All read 2026-10-12; the first two read again, with their screen images, 2026-10-13.
