# Starting a Windows computer from the stick

## Before you start

Do the BitLocker step in Faraday Starter first: it shows the drive's recovery key, which Windows asks for if it starts with Secure Boot off. A drive that is not encrypted has no BitLocker step. The page "BitLocker and starting from a stick" says why, and what the extra-safe choice does.

## Steps

1. Plug the stick in.

2. Open the firmware settings with Faraday Starter's Restart into firmware settings.

By hand: Settings → System → Recovery (Windows 10: Settings → Update & Security → Recovery), Restart now beside Advanced startup, then Troubleshoot → Advanced options → UEFI Firmware Settings → Restart.

No UEFI Firmware Settings entry: restart and press the maker's key at power-on, often Esc, Delete, F1, F2, F10, F11 or F12.

Use Restart, not Shut down: with Fast Startup on, Shut down does not fully close Windows.

3. Find Secure Boot, usually on the Security, Boot or Authentication tab, and set it to Disabled. Save changes and exit.

Leave the Secure Boot keys as they are. Do not delete or clear them.

4. Saving restarts the computer. On that restart, press the maker's boot menu key and choose the stick, before Windows starts. If it is listed twice, choose the one that starts with UEFI.

Without a boot menu key: choose the stick in the boot override list in the firmware settings.

If the stick is not listed: see "If something is different" below.

## Afterwards

5. Turn the computer off from Faraday and pull the stick out.

6. Open the firmware settings with the maker's key at power-on, set Secure Boot to Enabled, save and exit. Do it before Windows starts: with Secure Boot off, Windows asks for the recovery key unless Faraday Starter suspended BitLocker.

If Secure Boot cannot be enabled, or Windows does not start with it on: load the computer's own Secure Boot keys (Restore Factory Keys, or a Custom setting that loads the keys built into the computer).

7. Only after the extra-safe choice: open Faraday Starter in Windows and choose Resume BitLocker. It is offered only while Secure Boot is on, at sign-in when Faraday Starter is installed, or when it is opened again.

## If something is different

Stick not in the list: the firmware's own Fast Boot, which is not Windows' Fast Startup, makes some computers skip USB disks at power-on. Set Fast Boot to Disabled, or to Thorough where that is the choice, and save.

No UEFI Firmware Settings entry in Windows: Windows started in legacy BIOS mode. Faraday Starter's Restart into firmware settings works on any maker's computer started in UEFI mode, so not on this one; the screen at power-on often names the maker's key. Holding Shift while choosing Power → Restart also opens the Choose an option screen.

Why Secure Boot goes off: the Faraday on the stick is not signed with Microsoft's keys, so a computer starts it only with Secure Boot off.

Why Restart and not Shut down: with Fast Startup, which is on by default, Shut down saves the running Windows kernel to the disk and resumes it at the next power-on, so the key at power-on can be missed. Restart always closes Windows fully and starts again.

Other ways to choose the stick: Faraday Starter's Restart to choose a device, offered once Secure Boot is off, and Use a device on the Choose an option screen. Both are reached from Windows, so Windows has to start with Secure Boot off, and with BitLocker not suspended that start asks for the recovery key. Use a device is offered only on computers started in UEFI mode, and not on every one.

Without the maker's key to turn Secure Boot back on: start Windows, use Restart into firmware settings again, and turn Secure Boot on there. Windows asks for the recovery key on that start unless BitLocker is suspended, and the key in hand opens it.

A computer whose Secure Boot was already off before the BitLocker step is not asked to turn it on.

A computer whose drive is not encrypted has no BitLocker step, and Faraday Starter says so.

Fast Startup: Microsoft recommends leaving it on. To turn it off anyway, on Windows 11 search for "Choose a power plan", then Choose what the power button does, Change settings that are currently unavailable, clear Turn on fast startup, and Save changes. On Windows 10: Power & sleep settings → Additional power settings, then the same steps.

## Sources

Microsoft: "Windows recovery environment" (support.microsoft.com); "Windows Recovery Environment (Windows RE)", "Boot to UEFI Mode or Legacy BIOS mode" and "Disabling Secure Boot" (learn.microsoft.com, Windows hardware manufacturing); "Fast startup causes hibernation or shutdown to fail in Windows 10 or Windows 8.1" (learn.microsoft.com, Windows client troubleshooting); "shutdown" (learn.microsoft.com, Windows commands). ASUS: "[Windows 11/10] How to disable Fast Startup in Windows" (FAQ 1045548) and booting from a USB flash drive (FAQ 1013017). Dell: booting an external device over USB Type-C (knowledge base 000131221). All read 2026-10-12. No maker's computer has been tried with these steps yet.
