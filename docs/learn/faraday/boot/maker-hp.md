# Starting an HP computer from the stick

## Before you start

On Linux, have the disk's passphrase or recovery key ready if it is unlocked by the TPM.

HP's business computers are EliteBook, ProBook and Z workstations. Its consumer computers are OmniBook, OMEN, Victus, Spectre, Pavilion, ENVY and the numbered HP 14, 15 and 17 notebooks. Their menus differ.

## Steps

1. Plug the stick in.

2. Turn the computer off, then on, and press Esc repeatedly until the Startup Menu opens. Press F10 for BIOS Setup.

On an HP desktop, F10 at power-on opens Computer Setup directly. Faraday Starter's Restart into the BIOS opens the Startup Menu without a key.

3. Consumer notebooks: open the Boot Options tab, set Secure Boot to Disabled, then press F10 and choose Yes to save and exit.

::sketch hp-secure-boot

::sketch hp-save

On older models Boot Options is under System Configuration. Exit → Save Changes and Exit also saves. Leave Legacy Support as it is: the stick does not need it.

Business notebooks and workstations: choose Security → Secure Boot Configuration and clear the Secure Boot box. Then Main → Save Changes and Exit, then Yes.

::sketch hp-business-secure-boot

::sketch hp-business-save

Older business models: Advanced → Secure Boot Configuration → Configure Legacy Support and Secure Boot, set to Legacy Support Disable and Secure Boot Disable.

Desktops: Security → Secure Boot Configuration, press F10 at its message, set Secure Boot to Disable, and press F10 to save.

4. At the next start, type the four-digit code shown and press Enter. The screen does not show what is typed.

Consumer notebooks and desktops show it in an Operating System Boot Mode Change message.

::sketch hp-code

Business computers show it on a Secure boot page.

::sketch hp-business-code

5. Press Esc repeatedly, then F9 for Boot Device Options. On Windows, do it on this start, before Windows starts.

Pressing F9 repeatedly at power-on also opens it.

6. Choose the stick. Consumer notebooks list it as USB Hard Drive followed by the stick's name. If it is listed twice, choose the one among the UEFI entries.

::sketch hp-boot-menu

If the stick is not listed: see "If something is different" below.

## Afterwards

7. Turn the computer off from Faraday and pull the stick out.

8. Turn Secure Boot back on: open BIOS Setup as in step 2 and set Secure Boot back to Enabled, or tick the Secure Boot box, in the same menu. Save and exit. Do it before Windows starts.

If Secure Boot does not stay on, on business computers: Security → Secure Boot Configuration, tick Reset Secure Boot keys to factory defaults, save, restart into BIOS Setup, and turn Secure Boot on. HP Sure Start can protect the keys from that reset: first clear Sure Start Secure Boot Keys Protection under Security → BIOS Sure Start, and type the code the next start asks for. HP's consumer documents do not describe restoring the keys.

## If something is different

If Windows asks for the BitLocker recovery key: for a computer set up with a Microsoft account, the key is on the account's recovery-key page, aka.ms/myrecoverykey; otherwise it is wherever it was saved when BitLocker was set up. Then turn Secure Boot back on before the next start.

Stick not in the F9 list: on consumer notebooks, set USB Boot on the Boot Options tab to Enabled. On business computers, tick USB Storage Boot under Advanced → Boot Options; the Fast Boot box beside it can stay as it is. HP's documents do not describe a Fast Boot setting that hides USB disks.

::sketch hp-business-usb-boot

Clear All Secure Boot Keys, under Boot Options on consumer notebooks: do not use it. The stick needs Secure Boot off, not its keys removed.

No Legacy Support setting: newer HP computers have none, and the stick starts in UEFI mode.

## Sources

Where Faraday shows a picture of a firmware screen, it is a mock-up: the real screen may look different.

HP: "HP PCs - Secure Boot (Windows 11)", "HP Consumer Notebook PCs - BIOS Setup Information and Menu Options", and "HP Commercial Products - Commercial Products (2017-2023) with Sure Start May Experience a Secure Boot Error Message at Boot After a Microsoft Cumulative Monthly OS Update", all on support.hp.com, read 2026-10-12. HP's interactive BIOS simulators for its consumer and business notebooks, on hp.com, read 2026-10-13.
