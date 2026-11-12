# Starting a Windows computer from the stick

**Kind:** Reference
**Status:** Checked against the app

## Before you start

The last step turns Secure Boot back on, before Windows starts. With Secure Boot back on, Windows usually starts as before; on some computers it asks for the BitLocker recovery key, as If something is different says.

## Steps

1. Plug the stick in.

2. Open the BIOS with Faraday Starter's Restart into the BIOS.

By hand: Settings → System → Recovery (Windows 10: Settings → Update & Security → Recovery), Restart now beside Advanced startup, then Troubleshoot → Advanced options → UEFI Firmware Settings → Restart.

No UEFI Firmware Settings entry: restart and press the maker's key at power-on, often Esc, Delete, F1, F2, F10, F11 or F12.

Use Restart, not Shut down: with Fast Startup on, Shut down does not fully close Windows.

3. Find Secure Boot, usually on the Security, Boot or Authentication tab, and set it to Disabled. Save changes and exit.

Leave the Secure Boot keys as they are. Do not delete or clear them.

If the Secure Boot setting cannot be found: exit the BIOS without saving changes. Windows starts as before.

## Choosing the stick

4. Saving restarts the computer. On that restart, press the maker's boot menu key and choose the stick, before Windows starts. If it is listed twice, choose the one that starts with UEFI.

The stick is listed under the name printed on it (SanDisk, Kingston…), often after "UEFI:", not as Windows Boot Manager, a Linux name such as ubuntu, or the computer's own disk.

Without a boot menu key: choose the stick in the BIOS's boot override list. If the stick is not listed: see "If the stick is not listed" below.

## Afterwards

5. Turn the computer off from Faraday and pull the stick out.

6. Turn Secure Boot back on: open the BIOS with the maker's key at power-on, set Secure Boot to Enabled, save and exit. Do it before Windows starts.

If Secure Boot cannot be enabled, or Windows does not start with it on: load the computer's own Secure Boot keys (Restore Factory Keys, or a Custom setting that loads the keys built into the computer).

## If something is different

The BitLocker recovery key: Windows asks for it if it starts with Secure Boot off, and on some computers even after Secure Boot is back on. Faraday Starter's Temporarily suspend BitLocker, before the restart, suspends it for the next 3 starts, after which Windows turns it back on by itself. The key is at Control Panel → BitLocker Drive Encryption → Back up your recovery key; on Windows Home, manage-bde -protectors -get C: as administrator; at aka.ms/myrecoverykey with a Microsoft account; or on Faraday Starter's sheet. If Windows asks while Secure Boot is off: turn the computer off and turn Secure Boot back on; Windows then starts without the key.

## If the stick is not listed

Stick not in the list: the BIOS's own Fast Boot, which is not Windows' Fast Startup, makes some computers skip USB disks at power-on. Set Fast Boot to Disabled, or to Thorough, and save.

Without the maker's key: the screen at power-on often names it, and the page Another maker lists the common ones.

## Other ways in

No UEFI Firmware Settings entry in Windows: Windows started in legacy BIOS mode. Faraday Starter's Restart into the BIOS works on any maker's computer started in UEFI mode, so not on this one; the screen at power-on often names the maker's key. Holding Shift while choosing Power → Restart also opens the Choose an option screen.

Other ways to choose the stick, only when Windows is already running with Secure Boot off: Faraday Starter's Restart to choose a device, offered only then, and Use a device on the Choose an option screen. The stick then starts with Secure Boot already off; turn it back on afterwards, as the last step says. Use a device is offered only on computers started in UEFI mode, and not on every one.

## Why the steps are as they are

Why Secure Boot goes off: the Faraday on the stick is not signed with Microsoft's keys, so a computer starts it only with Secure Boot off.

Why Restart and not Shut down: with Fast Startup, which is on by default, Shut down saves the running Windows kernel to the disk and resumes it at the next power-on, so the key at power-on can be missed. Restart always closes Windows fully.

Fast Startup: Microsoft recommends leaving it on. To turn it off anyway, on Windows 11 search for "Choose a power plan", then Choose what the power button does, Change settings that are currently unavailable, clear Turn on fast startup, and Save changes. On Windows 10: Power & sleep settings → Additional power settings, then the same steps.

## Sources

Microsoft: "Windows recovery environment" (support.microsoft.com); "Windows Recovery Environment (Windows RE)", "Boot to UEFI Mode or Legacy BIOS mode" and "Disabling Secure Boot" (learn.microsoft.com, Windows hardware manufacturing); "Fast startup causes hibernation or shutdown to fail in Windows 10 or Windows 8.1" (learn.microsoft.com, Windows client troubleshooting); "shutdown" (learn.microsoft.com, Windows commands). ASUS: "[Windows 11/10] How to disable Fast Startup in Windows" (FAQ 1045548) and booting from a USB flash drive (FAQ 1013017). Dell: booting an external device over USB Type-C (knowledge base 000131221). All read 2026-10-12. No maker's computer has been tried with these steps yet.
