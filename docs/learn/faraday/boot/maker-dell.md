# Starting a Dell computer from the stick

## BitLocker first

On Windows, have the drive's BitLocker recovery key in hand before changing any firmware setting: Faraday Starter's BitLocker step on this computer shows it. Its extra-safe choice also suspends BitLocker until it is resumed. On Linux, have the disk's passphrase or recovery key first if the TPM unlocks the disk. The page "BitLocker and starting from a stick" says why.

## Keys

Firmware settings (System Setup): F2. Boot menu (One Time Boot): F12. Tap the key about once a second as soon as the Dell logo appears; holding it down can give a keyboard error instead. On a laptop whose top row changes brightness and volume, press Fn+F2 or Fn+F12.

With Windows' Fast Startup on, Shut down does not close Windows and the keys can be missed. Restart instead, then tap the key. Faraday Starter's Restart into firmware settings opens System Setup without a key. The F12 menu also lists BIOS Setup.

## Turn Secure Boot off

Computers made in 2021 or later: in System Setup, choose Boot Configuration in the left menu, turn Secure Boot off, then Apply Changes and exit.

::sketch dell-secure-boot

Computers made before 2021: choose Secure Boot in the left menu, clear Secure Boot Enable, then Apply and exit.

::sketch dell-secure-boot-before-2021

Leave Secure Boot Mode and Expert Key Management as they are.

On Windows, choose the stick from the boot menu on the restart that follows the save, before Windows starts, and turn Secure Boot back on before Windows starts again. Windows started with Secure Boot off asks for the BitLocker recovery key, unless Faraday Starter suspended BitLocker.

## Turn Fast Boot off where it hides USB

Dell's setting is Fastboot, under Pre-boot Behavior, or POST Behavior on older computers, with the choices Minimal, Thorough and Auto. If the stick is missing from the F12 menu, set Fastboot to Thorough. Dell's manuals do not say which choice skips USB disks.

Enable USB Boot Support has to be on: Integrated Devices → USB/Thunderbolt Configuration, or System Configuration → USB Configuration on older computers. For a stick in a Thunderbolt port, Enable Thunderbolt Boot Support has to be on as well.

## Start from the stick

Plug the stick in, restart, and tap F12. Choose the stick in the One Time Boot menu. Where it is listed twice, choose the entry that begins with UEFI. The F12 menu starts the stick once and leaves the boot order as it was.

::sketch dell-boot-menu

## Turn Secure Boot back on

Turn the computer off from Faraday and pull the stick out. Tap F2 at power-on, turn Secure Boot on where it was turned off, then Apply and exit.

If Secure Boot does not stay on, open Expert Key Management: with Enable Custom Mode on, Reset All Keys loads the computer's default keys, and turning Enable Custom Mode off again also restores them.

## Known quirks

The stick starts only in UEFI mode. Boot List Option set to Legacy means this computer starts in legacy BIOS mode, and Dell says changing it to UEFI stops a Windows installed in legacy mode from starting.

The menu names differ between Dell models and BIOS versions. Dell's System Manual for the model, under Support Resources, names each one.

## Where this comes from

Where Faraday shows a picture of a firmware screen, it is a mock-up: the real screen may look different.

Dell: "How to Enter BIOS or UEFI Setup on a Dell Computer" (knowledge base 000126121), "How to Enable Secure Boot on Dell Devices for Windows 11" (knowledge base 000190116), the video "How To Enable Secure Boot", and "BIOS Settings to Allow PXE Boot on Certain Dell Latitude Laptops" (knowledge base 000131551), all on dell.com/support; the Secure Boot and Pre-boot Behavior pages of Dell's Latitude service manuals (dell.com/support/manuals). All read 2026-10-12. The same video frame by frame, knowledge base 000136959, and the System setup and One Time Boot pages of Dell's Latitude owner's manuals, read 2026-10-13.
