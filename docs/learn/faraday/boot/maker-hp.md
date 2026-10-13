# Starting an HP computer from the stick

## BitLocker first

On Windows, do the BitLocker step in Faraday Starter on this computer before changing any firmware setting. On Linux, have the disk's passphrase or recovery key first if the TPM unlocks the disk. The page "BitLocker and starting from a stick" says why.

## Keys

Turn the computer off, turn it on, and press Esc repeatedly until the Startup Menu opens. From it, F10 opens BIOS Setup and F9 opens Boot Device Options. On an HP desktop, F10 at power-on opens Computer Setup directly.

Faraday Starter's Restart into firmware settings opens the Startup Menu without a key.

HP's business computers are EliteBook, ProBook and Z workstations. Its consumer computers are OmniBook, OMEN, Victus, Spectre, Pavilion, ENVY and the numbered HP 14, 15 and 17 notebooks. The two have different menus.

## Turn Secure Boot off

Consumer notebooks: in BIOS Setup, choose System Configuration → Boot Options, set Secure Boot to Disabled, then press F10 and choose Yes to save and exit. Leave Legacy Support as it is: the stick does not need it.

Business notebooks and workstations: in BIOS Setup, choose Security → Secure Boot Configuration and clear the Secure Boot box. On older models it is Advanced → Secure Boot Configuration → Configure Legacy Support and Secure Boot, set to Legacy Support Disable and Secure Boot Disable. Choose Save Changes and Exit, then Yes.

Desktops: Security → Secure Boot Configuration, press F10 at its message, set Secure Boot to Disable, and press F10 to save.

Turning Secure Boot off asks for a code to confirm it: a PIN shown when saving, or an Operating System Boot Mode Change message with a four-digit code at the next start. Type the code and press Enter. On desktops the screen does not show what is typed.

## Turn Fast Boot off where it hides USB

HP's documents do not describe a Fast Boot setting that hides USB disks. On consumer notebooks, System Configuration → Boot Options has USB Boot, which has to be Enabled.

## Start from the stick

Plug the stick in, turn the computer on, press Esc repeatedly, then F9 for Boot Device Options. Choose the stick. Where it is listed twice, choose the entry that begins with UEFI. HP also documents pressing F9 repeatedly at power-on to open the boot menu directly.

## Turn Secure Boot back on

Turn the computer off from Faraday and pull the stick out. Open BIOS Setup as above and set Secure Boot back to Enabled, or tick the Secure Boot box, in the same menu. Save and exit.

If Secure Boot does not stay on, on business computers: Security → Secure Boot Configuration, tick Reset Secure Boot keys to factory defaults, save, restart into BIOS Setup, and turn Secure Boot on. HP Sure Start can protect the keys from that reset: clear Sure Start Secure Boot Keys Protection under Security → BIOS Sure Start first, and type the code the next start asks for. HP's consumer documents do not describe restoring the keys.

## Known quirks

Consumer notebooks list Clear All Secure Boot Keys under Boot Options. Do not use it: the stick needs Secure Boot off, not its keys removed.

Newer HP computers have no Legacy Support setting. That is expected; the stick starts in UEFI mode.

## Where this comes from

HP: "HP PCs - Secure Boot (Windows 11)", "HP Consumer Notebook PCs - BIOS Setup Information and Menu Options", and "HP Commercial Products - Commercial Products (2017-2023) with Sure Start May Experience a Secure Boot Error Message at Boot After a Microsoft Cumulative Monthly OS Update", all on support.hp.com. All read 2026-10-12.
