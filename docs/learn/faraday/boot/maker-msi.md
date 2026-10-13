# Starting an MSI computer from the stick

## Before you start

On Windows, have the BitLocker recovery key ready: Faraday Starter's BitLocker step shows it. On Linux, have the disk's passphrase or recovery key ready.

These steps are from MSI's article for its notebooks, which does not cover desktops or motherboards.

## Steps

1. Plug the stick in.

2. Restart the computer.

3. When the MSI logo appears, press Delete.

If it doesn't open: use Faraday Starter's Restart into firmware settings.

4. Open Security → Secure Boot and set Secure Boot Support to Disabled. Press F10, Save & Reset, to save and restart.

::sketch msi-secure-boot

5. The computer restarts. Press F11 when the MSI logo appears. On Windows, do it on this restart, before Windows starts.

6. Choose the stick. If it is listed twice, choose the one that starts with UEFI.

::sketch msi-boot-menu

If the stick is not listed: see "If something is different" below.

## Afterwards

7. Turn the computer off from Faraday and pull the stick out.

8. Press Delete when the MSI logo appears, set Security → Secure Boot → Secure Boot Support to Enabled, and press F10. Do it before Windows starts: with Secure Boot off, Windows asks for the BitLocker recovery key unless Faraday Starter suspended BitLocker.

If Secure Boot does not stay on: MSI's notebook article does not describe restoring the Secure Boot keys; the step on the general Windows page applies.

## If something is different

Stick not in the F11 list: MSI's notebook article does not describe a setting that hides USB disks from the boot menu.

## Sources

Where Faraday shows a picture of a firmware screen, it is a mock-up: the real screen may look different.

MSI: "[Troubleshooting] Secure Boot Violation Error at Startup" (FAQ 11370, which applies to all MSI notebooks), on msi.com/faq. Read 2026-10-12, and again, with its screen images, 2026-10-13.
