# What Secure Boot and BitLocker are

## Turning Secure Boot off to start Faraday is safe

Secure Boot is off only while the computer starts Faraday from the stick, and back on before Windows starts again. Windows always starts with it on, as before.

Faraday cannot touch the computer. The system on the stick has no drivers for the computer's own drives and no network, so it cannot read, change or send anything on them, encrypted or not. It changes no BIOS setting. When the computer is turned off, nothing of Faraday is left on it.

## Secure Boot

Secure Boot is a setting in the computer's BIOS (on newer computers called UEFI). With it on, the computer starts only boot programs signed by a key it trusts: on most computers sold with Windows, Microsoft's. It guards against malware that replaces the program that starts Windows.

The Faraday stick is not signed with Microsoft's key, so the computer starts it only with Secure Boot off. Turned back on before Windows starts, the computer is as it was.

## BitLocker

BitLocker encrypts the Windows drive. Device encryption is BitLocker turned on by Windows itself, on many laptops. The drive's key is kept in the computer's security chip, the TPM, which gives it to Windows only when the computer has started the way it always does, Secure Boot included.

The TPM is asked only when Windows starts. Secure Boot off, Faraday, Secure Boot back on, then Windows: on most computers Windows sees the start it knows and asks for nothing.

If Windows starts while Secure Boot is still off, it asks for the drive's 48-digit recovery key. Turn Secure Boot back on before the next start. Some computers' BIOS changes its Secure Boot keys when the setting is turned off and on; Windows then asks for the key even with Secure Boot back on. BitLocker suspended first, as Faraday Starter offers, avoids it. The Windows guide says where Windows keeps the key.

## Where this comes from

Microsoft: "BitLocker overview", "BitLocker countermeasures" and "Secure the Windows boot process" (learn.microsoft.com, Windows security); "Secure boot" (learn.microsoft.com, Windows hardware design). All read 2026-10-12. The drivers the stick's kernel lacks: kernel.forbidden in the Faraday image's x86_64-uefi board (ATA, BLK_DEV_NVME, NVME_CORE, MMC, EFIVAR_FS) and the image's common kernel.forbidden (NET, WLAN, BT), in Faraday's repository.
