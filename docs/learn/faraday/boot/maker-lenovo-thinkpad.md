# Starting a Lenovo ThinkPad from the stick

## Before you start

On Windows, have the BitLocker recovery key ready: Faraday Starter's BitLocker step shows it. On Linux, have the disk's passphrase or recovery key ready.

## Steps

1. Plug the stick in.

2. Restart the computer. Use Restart, not Shut down: with Windows' Fast Startup on, Shut down can skip the key. Holding Shift while choosing Shut down also shuts down fully.

3. When the Lenovo or ThinkPad logo appears, tap F1 repeatedly.

If it doesn't open: press Enter at the logo, then F1 in the Startup Interrupt Menu, or use Faraday Starter's Restart into firmware settings.

4. Open the Security tab, then Secure Boot, and set Secure Boot to Disabled (Off on some models). Press F10 and choose Yes to save and restart.

::sketch thinkpad-secure-boot

::sketch thinkpad-save

If Secure Boot cannot be changed: Device Guard, on the Security tab, is Enabled, and Secure Boot cannot be changed until it is not.

Leave every other setting on the Secure Boot page as it is: the stick needs Secure Boot off, not its keys changed.

5. The computer restarts. Press F12 as soon as the ThinkPad logo appears. On Windows, do it on this restart, before Windows starts.

6. Choose the stick with the arrow keys and press Enter. If it is listed twice, choose the one that starts with UEFI.

If the stick is not listed: see "If something is different" below.

## Afterwards

7. Turn the computer off from Faraday and pull the stick out.

8. Tap F1 at power-on, set Security → Secure Boot → Secure Boot to Enabled (On), and press F10. Do it before Windows starts: with Secure Boot off, Windows asks for the BitLocker recovery key unless Faraday Starter suspended BitLocker.

If Secure Boot shows Disabled and cannot be changed: Secure Boot Mode is Setup Mode (Platform Mode on older models). Choose Restore Factory Keys on the same Secure Boot page, which returns it to User Mode, then set Secure Boot to Enabled and restart.

## If something is different

Stick not in the F12 list: Lenovo's ThinkPad articles do not describe a setting that hides USB disks from the boot menu.

## Sources

Where Faraday shows a picture of a firmware screen, it is a mock-up: the real screen may look different.

Lenovo: "How to enable Secure Boot on Think branded systems - ThinkPad, ThinkStation, ThinkCentre" (HT509044), "Secure Boot Mode cannot be set - ThinkPad" (HT515493), "Recommended ways to enter BIOS - ThinkPad, ThinkCentre, ThinkStation" (HT500222) and "How to boot from a USB Drive - ThinkPad" (HT118361), all on support.lenovo.com, read 2026-10-12. Lenovo BIOS Simulator Center, on download.lenovo.com, read 2026-10-13.
