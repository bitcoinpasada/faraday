# Starting a Windows computer from the stick

## BitLocker first

Do the BitLocker step in Faraday Starter on this computer before changing any firmware setting. On a computer whose drive is encrypted with BitLocker or Device encryption, starting Windows while Secure Boot is off makes Windows ask for the drive's 48-digit recovery key. The step is to have that key in hand: Faraday Starter shows it, and says where else it is kept. Then I have my recovery key opens the firmware steps; nothing on the computer changes.

The extra-safe choice, Extra safe: suspend BitLocker, also asks for two of the key's groups once you have kept it, and suspends BitLocker until you resume it. Windows then starts without the key whatever the Secure Boot setting, at the cost of a Resume step afterwards. The page "BitLocker and starting from a stick" says why.

A computer whose drive is not encrypted has no BitLocker step, and Faraday Starter says so.

## Restart, not Shut down

With Fast Startup, which is on by default, Shut down does not close Windows: it saves the running Windows kernel to the disk and resumes it at the next power-on. Restart always closes Windows fully and starts again. To reach the firmware settings or a boot menu from Windows, restart.

Microsoft recommends leaving Fast Startup on. To turn it off anyway:

Windows 11: search for "Choose a power plan", then Choose what the power button does, Change settings that are currently unavailable, clear Turn on fast startup, and Save changes.

Windows 10: Power & sleep settings → Additional power settings, then the same steps.

## Open the firmware settings

Faraday Starter's Restart into firmware settings does this in one step, on any maker's computer started in UEFI mode. By hand:

Windows 11: Settings → System → Recovery, and Restart now beside Advanced startup.

Windows 10: Settings → Update & Security → Recovery, and Restart now under Advanced startup.

On either, holding Shift while choosing Power → Restart does the same. Windows restarts into a screen titled Choose an option. Choose Troubleshoot → Advanced options → UEFI Firmware Settings, then Restart.

No UEFI Firmware Settings entry means Windows started in legacy BIOS mode. The firmware settings are then opened with the maker's key at power-on, often Esc, Delete, F1, F2, F10, F11 or F12; the screen at power-on often names it.

## Turn Secure Boot off

The Faraday on the stick is not signed with Microsoft's keys, so a computer starts it only with Secure Boot off. In the firmware settings, find Secure Boot, usually on the Security, Boot or Authentication tab, and set it to Disabled. Save changes and exit.

Leave the Secure Boot keys as they are. Do not delete or clear them.

Saving restarts the computer. On that restart choose the stick from the boot menu, before Windows starts, and turn Secure Boot back on before Windows starts again: Windows started with Secure Boot off asks for the recovery key, unless Faraday Starter suspended BitLocker.

## Turn Fast Boot off where it hides USB

The firmware's own Fast Boot, which is not Windows' Fast Startup, makes some computers skip USB disks at power-on, so the stick is not offered. If the stick is missing from the list in the next step, set Fast Boot to Disabled, or to Thorough where that is the choice, and save.

## Start from the stick

Plug the stick in. Faraday Starter's Restart to choose a device, offered once Secure Boot is off, opens Use a device in one step.

By hand: open Advanced startup as above, and on Choose an option choose Use a device, then the stick. Use a device is offered only on computers started in UEFI mode, and not on every one. Without it, use the maker's boot menu key at power-on, or the boot override list in the firmware settings.

Use a device and Restart to choose a device are reached from Windows, so Windows has to have started with Secure Boot off; with BitLocker not suspended, that start asks for the recovery key. Use the maker's boot menu key on the restart that saves Secure Boot off instead.

Where the stick is listed twice, choose the entry that begins with UEFI.

## Afterwards: Secure Boot back on

Turn the computer off from Faraday and pull the stick out. Turn Secure Boot back on before starting Windows: open the firmware settings with the maker's key at power-on, set Secure Boot to Enabled, save and exit. Without the maker's key, start Windows, use Restart into firmware settings again, and turn Secure Boot on there; Windows asks for the recovery key on that start unless BitLocker is suspended, and the key in hand opens it.

If the firmware does not let Secure Boot be enabled, or Windows does not start with it on, load the computer's own Secure Boot keys: on some firmware this is Restore Factory Keys or a Custom setting that loads the keys built into the computer.

If BitLocker was suspended with the extra-safe choice, open Faraday Starter in Windows and choose Resume BitLocker. It is offered only while Secure Boot is on. An installed Faraday Starter offers it at sign-in; one run without installing offers it when opened again.

A computer whose Secure Boot was already off before the BitLocker step is not asked to turn it on.

## Where this comes from

Microsoft: "Windows recovery environment" (support.microsoft.com); "Windows Recovery Environment (Windows RE)", "Boot to UEFI Mode or Legacy BIOS mode" and "Disabling Secure Boot" (learn.microsoft.com, Windows hardware manufacturing); "Fast startup causes hibernation or shutdown to fail in Windows 10 or Windows 8.1" (learn.microsoft.com, Windows client troubleshooting); "shutdown" (learn.microsoft.com, Windows commands). ASUS: "[Windows 11/10] How to disable Fast Startup in Windows" (FAQ 1045548) and booting from a USB flash drive (FAQ 1013017). Dell: booting an external device over USB Type-C (knowledge base 000131221). All read 2026-10-12. No maker's computer has been tried with these steps yet.
