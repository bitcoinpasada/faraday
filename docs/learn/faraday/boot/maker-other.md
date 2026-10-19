# Starting another maker's computer from the stick

## Before you start

On Linux, have the disk's passphrase or recovery key ready if it is unlocked by the TPM.

Find your computer's two keys: one opens the BIOS, one opens the boot menu. Search online for your computer's model and "boot menu key". The screen at power-on often names them.

BIOS: often F2, Delete, F1, F10 or Esc.

Boot menu: often F12, F11, F9 or Esc.

On some laptops the key is pressed with Fn. On some computers Esc opens a menu that lists the other keys.

## Steps

1. Plug the stick in.

2. Restart the computer. Use Restart, not Shut down: with Windows' Fast Startup on, Shut down can skip the key.

3. When the maker's logo appears, tap the BIOS key about once a second.

If it doesn't open: use Faraday Starter's Restart into the BIOS, or try the next key.

4. Find Secure Boot, usually on the Security, Boot or Authentication tab, set it to Disabled, then save and exit.

Leave the Secure Boot keys as they are. Do not delete or clear them.

If the Secure Boot setting cannot be found: exit the BIOS without saving changes. Windows starts as before.

5. The computer restarts. Tap the boot menu key when the maker's logo appears. On Windows, do it on this restart, before Windows starts.

6. Choose the stick. If it is listed twice, choose the one that starts with UEFI.

The stick is usually listed under its maker's name, the one printed on it (SanDisk, Kingston…), often after "UEFI:". It is not Windows Boot Manager, a Linux name such as ubuntu, or the computer's own disk.

If the stick is not listed: see "If something is different" below.

## Afterwards

7. Turn the computer off from Faraday and pull the stick out.

8. Turn Secure Boot back on: tap the BIOS key at power-on, set Secure Boot to Enabled, then save and exit. Do it before Windows starts.

If Secure Boot does not stay on: load the computer's own Secure Boot keys. On some computers this is Restore Factory Keys or a Custom setting that loads the keys built into the computer.

## If something is different

The BitLocker recovery key: with Secure Boot off, Windows cannot start without it. It is not needed here: the last step turns Secure Boot back on before Windows starts. To see it on this computer anyway: in Windows, Control Panel → BitLocker Drive Encryption → Back up your recovery key; on Windows Home, manage-bde -protectors -get C: in a terminal run as administrator. On a computer signed in with a Microsoft account it is also at aka.ms/myrecoverykey. If Windows asks for it: turn the computer off and turn Secure Boot back on; Windows then starts without it.

Stick not in the boot menu: set Fast Boot to Disabled, or to Thorough where that is the choice. Where there is a USB Boot setting, set it to Enabled.

Secure Boot greyed out: on some computers it can be changed only once a supervisor password is set.

## Sources

The keys are the ones the maker pages here give, each from its maker's own documentation. Microsoft: "Boot to UEFI Mode or Legacy BIOS mode" and "Disabling Secure Boot" (learn.microsoft.com, Windows hardware manufacturing). All read 2026-10-12.
