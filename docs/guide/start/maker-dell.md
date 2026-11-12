# Starting a Dell computer from the stick

**Kind:** Reference
**Status:** Checked against the app

## Before you start

On Linux, have the disk's passphrase or recovery key ready if it is unlocked by the TPM.

## Steps

1. Plug the stick in.

2. Restart the computer. Use Restart, not Shut down: with Windows' Fast Startup on, Shut down can skip the key.

3. When the Dell logo appears, tap F2 about once a second. Don't hold it down.

If it doesn't open: tap Fn+F2, or tap F12 and choose BIOS Setup, or use Faraday Starter's Restart into the BIOS.

4. Look for the menu named Boot Configuration in the list on the left. Under its Secure Boot heading, set Enable Secure Boot to off, then choose Apply Changes and Exit.

Made before 2021: look for the menu named Secure Boot, clear Secure Boot Enable, then choose Apply and Exit. Leave Secure Boot Mode and Expert Key Management as they are.

If the Secure Boot setting cannot be found: exit the BIOS without saving changes. Windows starts as before.

## Choosing the stick

5. The computer restarts. Tap F12 when the Dell logo appears (Fn+F12 on some laptops). On Windows, do it on this restart, before Windows starts.

6. Look for the group named UEFI Boot in the One Time Boot Menu and choose the stick in it. If it is listed twice, choose the one that starts with UEFI. This starts the stick once; the boot order stays as it was. BIOS Setup and Diagnostics are under Other Options.

The stick is listed under the name printed on it (SanDisk, Kingston…), not as Windows Boot Manager, a Linux name such as ubuntu, or the computer's own disk. If it is not listed: see "If the stick is not listed" below.

## Afterwards

7. Turn the computer off from Faraday and pull the stick out.

8. Turn Secure Boot back on: turn the computer on, tap F2, look for Boot Configuration, set Enable Secure Boot back to on, then choose Apply Changes and Exit. Do it before Windows starts.

If Secure Boot does not stay on: Expert Key Management, Enable Custom Mode on, Reset All Keys, then Enable Custom Mode off.

## If something is different

The BitLocker recovery key: Windows asks for it if it starts with Secure Boot off, and on some computers even after Secure Boot is back on. Faraday Starter's Temporarily suspend BitLocker, before the restart, suspends it for the next 3 starts, after which Windows turns it back on by itself. The key is at Control Panel → BitLocker Drive Encryption → Back up your recovery key; on Windows Home, manage-bde -protectors -get C: as administrator; at aka.ms/myrecoverykey with a Microsoft account; or on Faraday Starter's sheet. If Windows asks while Secure Boot is off: turn the computer off and turn Secure Boot back on; Windows then starts without the key.

## If the stick is not listed

Stick not in the F12 list: in F2 setup, look for the menu named Pre-boot Behavior (POST Behavior on older models) and set Fastboot to Thorough. Dell's manuals do not say which Fastboot choice skips USB disks.

Look for the setting Enable USB Boot Support (Integrated Devices → USB/Thunderbolt Configuration, or System Configuration → USB Configuration on older models) and turn it on. In a Thunderbolt port, also turn on Enable Thunderbolt Boot Support.

Look for Boot List Option. If it says Legacy: the stick starts only in UEFI mode, and Dell says changing this to UEFI stops a Windows installed in legacy mode from starting.

Menu names differ between models and BIOS versions: Dell's System Manual for the model, under Support Resources, names each one.

## Sources

Dell: "How to Enter BIOS or UEFI Setup on a Dell Computer" (knowledge base 000126121), "How to Enable Secure Boot on Dell Devices for Windows 11" (knowledge base 000190116), the video "How To Enable Secure Boot", and "BIOS Settings to Allow PXE Boot on Certain Dell Latitude Laptops" (knowledge base 000131551), all on dell.com/support; the Secure Boot and Pre-boot Behavior pages of Dell's Latitude service manuals (dell.com/support/manuals). All read 2026-10-12. The same video frame by frame, knowledge base 000136959, and the System setup and One Time Boot pages of Dell's Latitude owner's manuals, read 2026-10-13.
