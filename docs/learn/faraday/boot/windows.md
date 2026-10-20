# Starting a Windows computer from the stick

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

4. Saving restarts the computer. On that restart, press the maker's boot menu key and choose the stick, before Windows starts. If it is listed twice, choose the one that starts with UEFI.

The stick is usually listed under its maker's name, the one printed on it (SanDisk, Kingston…), often after "UEFI:". It is not Windows Boot Manager, a Linux name such as ubuntu, or the computer's own disk.

Without a boot menu key: choose the stick in the boot override list in the BIOS.

If the stick is not listed: see "If something is different" below.

## Afterwards

5. Turn the computer off from Faraday and pull the stick out.

6. Turn Secure Boot back on: open the BIOS with the maker's key at power-on, set Secure Boot to Enabled, save and exit. Do it before Windows starts.

If Secure Boot cannot be enabled, or Windows does not start with it on: load the computer's own Secure Boot keys (Restore Factory Keys, or a Custom setting that loads the keys built into the computer).

## If something is different

The BitLocker recovery key: with Secure Boot off, Windows cannot start without it. Windows may ask for it even after Secure Boot is back on, on a computer whose BIOS changes its Secure Boot keys when the setting is turned off and on. Suspending BitLocker first avoids that: Faraday Starter's Temporarily suspend BitLocker, before the restart, suspends it for the next 3 times Windows starts, after which Windows turns it back on by itself. To see the key on this computer: in Windows, Control Panel → BitLocker Drive Encryption → Back up your recovery key; on Windows Home, manage-bde -protectors -get C: in a terminal run as administrator; Faraday Starter prints it on its sheet when asked to. On a computer signed in with a Microsoft account it is also at aka.ms/myrecoverykey. If Windows asks for it while Secure Boot is still off: turn the computer off and turn Secure Boot back on; Windows then starts without it.

Stick not in the list: the BIOS's own Fast Boot, which is not Windows' Fast Startup, makes some computers skip USB disks at power-on. Set Fast Boot to Disabled, or to Thorough where that is the choice, and save.

No UEFI Firmware Settings entry in Windows: Windows started in legacy BIOS mode. Faraday Starter's Restart into the BIOS works on any maker's computer started in UEFI mode, so not on this one; the screen at power-on often names the maker's key. Holding Shift while choosing Power → Restart also opens the Choose an option screen.

Why Secure Boot goes off: the Faraday on the stick is not signed with Microsoft's keys, so a computer starts it only with Secure Boot off.

Why Restart and not Shut down: with Fast Startup, which is on by default, Shut down saves the running Windows kernel to the disk and resumes it at the next power-on, so the key at power-on can be missed. Restart always closes Windows fully and starts again.

Other ways to choose the stick, only when Windows is already running with Secure Boot off: Faraday Starter's Restart to choose a device, offered only then, and Use a device on the Choose an option screen. The stick then starts with Secure Boot already off; turn it back on afterwards, as the last step says. Use a device is offered only on computers started in UEFI mode, and not on every one.

Without the maker's key: the screen at power-on often names it, and the page Another maker lists the common ones.

Fast Startup: Microsoft recommends leaving it on. To turn it off anyway, on Windows 11 search for "Choose a power plan", then Choose what the power button does, Change settings that are currently unavailable, clear Turn on fast startup, and Save changes. On Windows 10: Power & sleep settings → Additional power settings, then the same steps.

## Sources

Microsoft: "Windows recovery environment" (support.microsoft.com); "Windows Recovery Environment (Windows RE)", "Boot to UEFI Mode or Legacy BIOS mode" and "Disabling Secure Boot" (learn.microsoft.com, Windows hardware manufacturing); "Fast startup causes hibernation or shutdown to fail in Windows 10 or Windows 8.1" (learn.microsoft.com, Windows client troubleshooting); "shutdown" (learn.microsoft.com, Windows commands). ASUS: "[Windows 11/10] How to disable Fast Startup in Windows" (FAQ 1045548) and booting from a USB flash drive (FAQ 1013017). Dell: booting an external device over USB Type-C (knowledge base 000131221). All read 2026-10-12. No maker's computer has been tried with these steps yet.
