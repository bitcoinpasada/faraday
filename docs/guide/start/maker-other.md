# Starting another maker's computer from the stick

**Kind:** Reference
**Status:** Checked against the app

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

4. Look for a setting named Secure Boot, usually on a tab named Security, Boot, Startup, Boot Options or Authentication. Set it to Disabled, then save and exit (often F10).

Some computers name it Secure Boot Control, Secure Boot Support or Secure Boot State, or have a Boot Mode or OS Type choice instead; names vary. Leave the Secure Boot keys as they are; do not delete or clear them.

If the Secure Boot setting cannot be found: exit the BIOS without saving changes. Windows starts as before.

## Choosing the stick

5. The computer restarts. Tap the boot menu key when the maker's logo appears. On Windows, do it on this restart, before Windows starts.

6. In the boot menu, choose the stick. The menu may be named Boot Menu, Boot Device Options or Boot Option Menu. If it is listed twice, choose the one that starts with UEFI.

The stick is listed under the name printed on it (SanDisk, Kingston…), often after "UEFI:", not as Windows Boot Manager, a Linux name such as ubuntu, or the computer's own disk. If it is not listed: see "If the stick is not listed" below.

## Afterwards

7. Turn the computer off from Faraday and pull the stick out.

8. Turn Secure Boot back on: tap the BIOS key at power-on, set Secure Boot to Enabled, then save and exit. Do it before Windows starts.

If Secure Boot does not stay on: load the computer's own Secure Boot keys. On some computers this is Restore Factory Keys or a Custom setting that loads the keys built into the computer.

## If something is different

The BitLocker recovery key: Windows asks for it if it starts with Secure Boot off, and on some computers even after Secure Boot is back on. Faraday Starter's Temporarily suspend BitLocker, before the restart, suspends it for the next 3 starts, after which Windows turns it back on by itself. The key is at Control Panel → BitLocker Drive Encryption → Back up your recovery key; on Windows Home, manage-bde -protectors -get C: as administrator; at aka.ms/myrecoverykey with a Microsoft account; or on Faraday Starter's sheet. If Windows asks while Secure Boot is off: turn the computer off and turn Secure Boot back on; Windows then starts without the key.

## If the stick is not listed

Stick not in the boot menu: look for a setting named Fast Boot, Fastboot or Quick Boot, usually on the Boot tab, and set it to Disabled, or to Thorough where that is the choice. Look for a setting named USB Boot, USB Boot Support or Boot from USB devices and set it to Enabled. Boot Order or Boot Sequence can stay as it is: the boot menu starts the stick once. A setting named Legacy or CSM can stay as it is: the stick starts in UEFI mode.

Secure Boot greyed out: on some computers it can be changed only once a supervisor password is set.

## Sources

The keys are the ones the maker pages here give, each from its maker's own documentation. Microsoft: "Boot to UEFI Mode or Legacy BIOS mode" and "Disabling Secure Boot" (learn.microsoft.com, Windows hardware manufacturing). All read 2026-10-12.
