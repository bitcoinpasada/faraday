# Starting a Dell computer from the stick

## Before you start

On Windows, have the BitLocker recovery key ready: Faraday Starter's BitLocker step shows it. On Linux, have the disk's passphrase or recovery key ready.

## Steps

1. Plug the stick in.

2. Restart the computer. Use Restart, not Shut down: with Windows' Fast Startup on, Shut down can skip the key.

3. When the Dell logo appears, tap F2 about once a second. Don't hold it down.

If it doesn't open: tap Fn+F2, or tap F12 and choose BIOS Setup, or use Faraday Starter's Restart into the BIOS.

4. Choose Boot Configuration, turn Secure Boot off, then Apply Changes and Exit.

::sketch dell-secure-boot

Made before 2021: choose Secure Boot, clear Secure Boot Enable, then Apply and Exit.

::sketch dell-secure-boot-before-2021

Leave Secure Boot Mode and Expert Key Management as they are.

5. The computer restarts. Tap F12 when the Dell logo appears (Fn+F12 on some laptops). On Windows, do it on this restart, before Windows starts.

6. Choose the stick. If it is listed twice, choose the one that starts with UEFI. This starts the stick once; the boot order stays as it was.

::sketch dell-boot-menu

The stick is usually listed under its maker's name, the one printed on it (SanDisk, Kingston…), often after "UEFI:". It is not Windows Boot Manager, a Linux name such as ubuntu, or the computer's own disk.

If the stick is not listed: see "If something is different" below.

## Afterwards

7. Turn the computer off from Faraday and pull the stick out.

8. Turn it on, tap F2, turn Secure Boot back on, then Apply and Exit. Do it before Windows starts: with Secure Boot off, Windows asks for the BitLocker recovery key unless Faraday Starter suspended BitLocker.

If Secure Boot does not stay on: Expert Key Management, Enable Custom Mode on, Reset All Keys, then Enable Custom Mode off.

## If something is different

Stick not in the F12 list: in F2 setup, set Fastboot to Thorough (under Pre-boot Behavior, or POST Behavior on older models). Turn on Enable USB Boot Support (Integrated Devices → USB/Thunderbolt Configuration, or System Configuration → USB Configuration on older models). In a Thunderbolt port, also turn on Enable Thunderbolt Boot Support. Dell's manuals do not say which Fastboot choice skips USB disks.

Boot List Option says Legacy: the stick starts only in UEFI mode, and Dell says changing this to UEFI stops a Windows installed in legacy mode from starting.

Menu names differ between models and BIOS versions: Dell's System Manual for the model, under Support Resources, names each one.

## Sources

Where Faraday shows a picture of a firmware screen, it is a mock-up: the real screen may look different.

Dell: "How to Enter BIOS or UEFI Setup on a Dell Computer" (knowledge base 000126121), "How to Enable Secure Boot on Dell Devices for Windows 11" (knowledge base 000190116), the video "How To Enable Secure Boot", and "BIOS Settings to Allow PXE Boot on Certain Dell Latitude Laptops" (knowledge base 000131551), all on dell.com/support; the Secure Boot and Pre-boot Behavior pages of Dell's Latitude service manuals (dell.com/support/manuals). All read 2026-10-12. The same video frame by frame, knowledge base 000136959, and the System setup and One Time Boot pages of Dell's Latitude owner's manuals, read 2026-10-13.
