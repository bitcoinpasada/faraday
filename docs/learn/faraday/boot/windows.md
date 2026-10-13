# Starting a Windows computer from the stick

## Before you start

Do the BitLocker step in Faraday Starter on this computer before changing any firmware setting. On a drive encrypted with BitLocker or Device encryption, Windows started with Secure Boot off asks for the drive's 48-digit recovery key. The step is to have that key in hand: Faraday Starter shows it and says where else it is kept. Then I have my recovery key opens the firmware steps; nothing on the computer changes.

The other choice, Extra safe: suspend BitLocker, also asks for two of the key's groups once the key is kept, and suspends BitLocker until it is resumed. Windows then starts without the key whatever the Secure Boot setting, at the cost of a Resume step afterwards. The page "BitLocker and starting from a stick" says why.

A computer whose drive is not encrypted has no BitLocker step, and Faraday Starter says so.

## Steps

1. Plug the stick in.

2. Open the firmware settings with Faraday Starter's Restart into firmware settings. It works on any maker's computer started in UEFI mode.

By hand, Windows 11: Settings → System → Recovery, and Restart now beside Advanced startup. Windows 10: Settings → Update & Security → Recovery, and Restart now under Advanced startup. On either, holding Shift while choosing Power → Restart does the same. On the Choose an option screen, choose Troubleshoot → Advanced options → UEFI Firmware Settings, then Restart.

No UEFI Firmware Settings entry: Windows started in legacy BIOS mode. Restart and press the maker's key at power-on, often Esc, Delete, F1, F2, F10, F11 or F12; the screen at power-on often names it.

Use Restart, not Shut down. With Fast Startup, which is on by default, Shut down does not close Windows: it saves the running Windows kernel to the disk and resumes it at the next power-on. Restart always closes Windows fully and starts again.

3. Find Secure Boot, usually on the Security, Boot or Authentication tab, and set it to Disabled. Save changes and exit.

The Faraday on the stick is not signed with Microsoft's keys, so a computer starts it only with Secure Boot off.

Leave the Secure Boot keys as they are. Do not delete or clear them.

4. Saving restarts the computer. On that restart, press the maker's boot menu key and choose the stick, before Windows starts. If it is listed twice, choose the one that starts with UEFI.

Without a boot menu key: choose the stick in the boot override list in the firmware settings.

Faraday Starter's Restart to choose a device, offered once Secure Boot is off, and Use a device on the Choose an option screen also choose the stick. Both are reached from Windows, so Windows has to start with Secure Boot off, and with BitLocker not suspended that start asks for the recovery key. Use a device is offered only on computers started in UEFI mode, and not on every one.

If the stick is not listed: see "If something is different" below.

## Afterwards

5. Turn the computer off from Faraday and pull the stick out.

6. Open the firmware settings with the maker's key at power-on, set Secure Boot to Enabled, save and exit. Do it before Windows starts: with Secure Boot off, Windows asks for the recovery key unless Faraday Starter suspended BitLocker.

Without the maker's key: start Windows, use Restart into firmware settings again, and turn Secure Boot on there. Windows asks for the recovery key on that start unless BitLocker is suspended, and the key in hand opens it.

If the firmware does not let Secure Boot be enabled, or Windows does not start with it on: load the computer's own Secure Boot keys. On some firmware this is Restore Factory Keys or a Custom setting that loads the keys built into the computer.

A computer whose Secure Boot was already off before the BitLocker step is not asked to turn it on.

7. If BitLocker was suspended with the extra-safe choice: open Faraday Starter in Windows and choose Resume BitLocker. It is offered only while Secure Boot is on.

An installed Faraday Starter offers it at sign-in; one run without installing offers it when opened again.

## If something is different

Stick not in the list: the firmware's own Fast Boot, which is not Windows' Fast Startup, makes some computers skip USB disks at power-on. Set Fast Boot to Disabled, or to Thorough where that is the choice, and save.

Fast Startup: Microsoft recommends leaving it on. To turn it off anyway, on Windows 11 search for "Choose a power plan", then Choose what the power button does, Change settings that are currently unavailable, clear Turn on fast startup, and Save changes. On Windows 10: Power & sleep settings → Additional power settings, then the same steps.

## Sources

Microsoft: "Windows recovery environment" (support.microsoft.com); "Windows Recovery Environment (Windows RE)", "Boot to UEFI Mode or Legacy BIOS mode" and "Disabling Secure Boot" (learn.microsoft.com, Windows hardware manufacturing); "Fast startup causes hibernation or shutdown to fail in Windows 10 or Windows 8.1" (learn.microsoft.com, Windows client troubleshooting); "shutdown" (learn.microsoft.com, Windows commands). ASUS: "[Windows 11/10] How to disable Fast Startup in Windows" (FAQ 1045548) and booting from a USB flash drive (FAQ 1013017). Dell: booting an external device over USB Type-C (knowledge base 000131221). All read 2026-10-12. No maker's computer has been tried with these steps yet.
