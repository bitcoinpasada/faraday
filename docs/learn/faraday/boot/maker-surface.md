# Starting a Microsoft Surface from the stick

## Before you start

On Windows, have the BitLocker recovery key ready: Faraday Starter's BitLocker step shows it. On Linux, have the disk's passphrase or recovery key ready.

A Surface has no keys for this; it uses its volume buttons.

## Steps

1. Plug the stick in, and unplug any other USB disk: the Surface may start from that instead.

2. Shut the Surface down and wait about 10 seconds.

3. Hold Volume Up, press and release the power button, and keep holding Volume Up until the Surface UEFI screen appears.

If it doesn't open: use Faraday Starter's Restart into firmware settings, which opens Surface UEFI without the buttons.

4. Choose Security. Under Secure Boot, choose Change configuration, choose None, and choose OK.

::sketch surface-secure-boot

5. Choose Exit → Restart now.

::sketch surface-exit

6. Start the stick before Windows starts: hold Volume Down, press and release the power button, and keep holding Volume Down until spinning dots appear under the logo.

Or, in Surface UEFI, Boot configuration: swipe left on USB Storage to start from it once.

Or, in Windows, Settings → System → Recovery, Restart now beside Advanced startup, then Use a device → USB Storage. Windows asks for the BitLocker recovery key on that start unless Faraday Starter suspended BitLocker.

If the stick does not start: see "If something is different" below.

## Afterwards

7. Turn the Surface off from Faraday and pull the stick out.

8. Open Surface UEFI with Volume Up, choose Security, under Secure Boot choose Change configuration, and choose Microsoft only. Then Exit → Restart now. Do it before Windows starts: with Secure Boot off, Windows asks for the BitLocker recovery key unless Faraday Starter suspended BitLocker.

While Secure Boot is off, the Surface shows a red bar with an open padlock at power-on. Once Secure Boot is back on, it is gone.

## If something is different

Stick not started: Microsoft's Surface UEFI pages list no Fast Boot setting. The Boot configuration page has two USB settings, and both have to be on: USB Storage in the list of boot devices, and Enable Boot from USB devices under Advanced options.

::sketch surface-boot

A Surface with a Snapdragon processor is an ARM computer and cannot start the stick. Faraday Starter says so on such a computer.

Surface Go uses a UEFI made by another company, and Surface Pro 3 and earlier have different menus; Microsoft's pages for those name their own settings.

## Sources

Where Faraday shows a picture of a firmware screen, it is a mock-up: the real screen may look different.

Microsoft: "Boot Surface from a USB device", "How to use Surface UEFI" and "Surface turns on but shows a red bar and an unlocked lock icon" (support.microsoft.com); "Manage Surface UEFI settings" (learn.microsoft.com). All read 2026-10-12; "Manage Surface UEFI settings" read again, with its figures, 2026-10-13.
