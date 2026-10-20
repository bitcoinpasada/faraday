# Starting a Microsoft Surface from the stick

## Before you start

On Linux, have the disk's passphrase or recovery key ready if it is unlocked by the TPM.

A Surface has no keys for this; it uses its volume buttons.

## Steps

1. Plug the stick in, and unplug any other USB disk: the Surface may start from that instead.

2. Shut the Surface down and wait about 10 seconds.

3. Hold Volume Up, press and release the power button, and keep holding Volume Up until the Surface UEFI screen appears.

If it doesn't open: use Faraday Starter's Restart into the BIOS, which opens Surface UEFI without the buttons.

4. Choose Security. Under Secure Boot, choose Change configuration, choose None, and choose OK.

::sketch surface-secure-boot

If the Secure Boot setting cannot be found: exit the BIOS without saving changes. Windows starts as before.

5. Choose Exit → Restart now.

::sketch surface-exit

6. Start the stick before Windows starts: hold Volume Down, press and release the power button, and keep holding Volume Down until spinning dots appear under the logo.

Or, in Surface UEFI, Boot configuration: swipe left on USB Storage to start from it once.

Or, when Windows is already running with Secure Boot off: Settings → System → Recovery, Restart now beside Advanced startup, then Use a device → USB Storage.

If the stick does not start: see "If something is different" below.

## Afterwards

7. Turn the Surface off from Faraday and pull the stick out.

8. Turn Secure Boot back on: open Surface UEFI with Volume Up, choose Security, under Secure Boot choose Change configuration, and choose Microsoft only. Then Exit → Restart now. Do it before Windows starts.

While Secure Boot is off, the Surface shows a red bar with an open padlock at power-on. Once Secure Boot is back on, it is gone.

## If something is different

The BitLocker recovery key: with Secure Boot off, Windows cannot start without it. Windows may ask for it even after Secure Boot is back on, on a computer whose BIOS changes its Secure Boot keys when the setting is turned off and on. Suspending BitLocker first avoids that: Faraday Starter's Temporarily suspend BitLocker, before the restart, suspends it for the next 3 times Windows starts, after which Windows turns it back on by itself. To see the key on this computer: in Windows, Control Panel → BitLocker Drive Encryption → Back up your recovery key; on Windows Home, manage-bde -protectors -get C: in a terminal run as administrator; Faraday Starter prints it on its sheet when asked to. On a computer signed in with a Microsoft account it is also at aka.ms/myrecoverykey. If Windows asks for it while Secure Boot is still off: turn the computer off and turn Secure Boot back on; Windows then starts without it.

Stick not started: Microsoft's Surface UEFI pages list no Fast Boot setting. The Boot configuration page has two USB settings, and both have to be on: USB Storage in the list of boot devices, and Enable Boot from USB devices under Advanced options.

::sketch surface-boot

A Surface with a Snapdragon processor is an ARM computer and cannot start the stick. Faraday Starter says so on such a computer.

Surface Go uses a UEFI made by another company, and Surface Pro 3 and earlier have different menus; Microsoft's pages for those name their own settings.

## Sources

Where Faraday shows a picture of a firmware screen, it is a mock-up: the real screen may look different.

Microsoft: "Boot Surface from a USB device", "How to use Surface UEFI" and "Surface turns on but shows a red bar and an unlocked lock icon" (support.microsoft.com); "Manage Surface UEFI settings" (learn.microsoft.com). All read 2026-10-12; "Manage Surface UEFI settings" read again, with its figures, 2026-10-13.
