# What Secure Boot and BitLocker are

## Secure Boot

Secure Boot is a setting in the computer's BIOS (on newer computers called UEFI), the software that runs when the computer is switched on and before any operating system. With it on, the BIOS starts a boot program only if it is signed by a key the BIOS trusts; on most computers sold with Windows that is Microsoft's.

It guards against boot-level malware: a bootkit replaces the program that starts the operating system, so it runs before Windows and before any antivirus, and can hide from both. With Secure Boot on, a replaced boot program is not signed and does not start.

Programs running in Windows cannot turn Secure Boot off. It is changed only in the BIOS, by someone at the computer.

The Faraday on the stick is not signed with Microsoft's keys, so the computer starts it only with Secure Boot off.

## BitLocker and Device encryption

BitLocker encrypts the whole Windows drive. Device encryption is BitLocker turned on by Windows itself, on many laptops, when it is first set up with a Microsoft account or a work or school account.

It guards against someone reading the drive without Windows' sign-in: by taking the drive out and putting it in another computer, or by starting the computer from another system. Without the key, what is on the drive cannot be read.

## Why the key is sealed to how the computer started

The drive's key is kept in the computer's TPM, a security chip, and the TPM releases it only when the computer has started the way it did when the key was sealed. The BIOS records each step of the start in the TPM; one record, PCR 7, holds the Secure Boot state and its keys.

This is what stops someone from starting the computer from a stick of their own and having the key handed over: the start is different, so the TPM keeps the key.

It is also why turning Secure Boot off and then starting Windows makes Windows ask for the drive's 48-digit recovery key. PCR 7 has changed, the TPM does not release the key, and the recovery key is the only other way in.

## How Faraday Starter keeps the recovery key prompt away

The TPM is asked for the key only when Windows starts. A computer whose Secure Boot is turned off, which starts the stick, and whose Secure Boot is turned back on before Windows starts again, is never asked for the recovery key. The guides say to choose the stick from the boot menu on the restart that saves Secure Boot off, and to turn Secure Boot back on before starting Windows.

Before any BIOS step, Faraday Starter's BitLocker step is to have the drive's recovery key in hand: it shows the key, and says where else it is kept. If Windows does start with Secure Boot off, that key opens it.

The extra-safe choice also suspends BitLocker until it is resumed. It shows the key and asks for two of its groups back first, so stopping between the two changes nothing on the computer. Once BitLocker is suspended, Windows starts without asking for the recovery key, whatever the Secure Boot state.

## What suspending does

This applies only to the extra-safe choice. The drive stays encrypted. What is written to it while suspended is encrypted too. Nothing is decrypted.

What changes is the key. While suspended, the key that opens the drive is kept on the drive itself, in the clear, instead of sealed in the TPM. Windows uses it to start without checking how the computer started. Resuming seals the key in the TPM again and removes the clear copy.

## What Faraday on the stick can do to the computer

Nothing to its drives. The Linux kernel on the stick is built without the drivers for a computer's own drives: no SATA (ATA), no NVMe and no eMMC. Without a driver the kernel never sees the drive, so nothing on it can be read, written or mounted, whether it is encrypted or not.

Nothing over a network. The kernel has no network support at all: no network stack, no Wi-Fi and no Bluetooth.

Nothing in the BIOS. The kernel cannot write the BIOS's variables, so the stick changes no setting and no boot entry.

Nothing left behind. Faraday runs in the computer's memory and from the stick. When the computer is turned off, nothing of Faraday is on the computer.

## The risk that remains

While BitLocker is suspended by the extra-safe choice, someone holding the computer could read its drive: by taking it out, or by starting another system, because the key is on the drive in the clear. This is the protection BitLocker normally gives, set aside for as long as the suspension lasts.

While Secure Boot is off, the computer starts boot programs that nobody has signed. A program with administrator rights in Windows could put a bootkit in place in that time, and the BIOS would start it. Someone at the computer could also start any system from a stick.

Both last only until they are turned back on. Keep the computer with you, and do not install or run new programs in Windows, until they are.

## Turning both back on

After every Faraday session Faraday Starter offers these steps, in this order.

1. Turn Secure Boot back on, in the BIOS, before Windows starts. Faraday Starter's Restart into the BIOS opens it.

2. After the extra-safe choice only: Resume BitLocker, in Faraday Starter, once Windows has started again with Secure Boot on. Resuming seals the key to that start, so the next start with Secure Boot on does not ask for the recovery key. Faraday Starter does not offer it in the same Windows session that suspended it.

Then the computer is as it was: Secure Boot on, the drive's key sealed in the TPM, and no copy of it in the clear. Without the extra-safe choice, BitLocker was never suspended, and turning Secure Boot back on is the whole of it.

## Where this comes from

Microsoft: "BitLocker overview", "BitLocker countermeasures" and "Secure the Windows boot process" (learn.microsoft.com, Windows security); "Secure boot" (learn.microsoft.com, Windows hardware design); "Suspend-BitLocker" (learn.microsoft.com, PowerShell). All read 2026-10-12. The drivers the stick's kernel lacks: kernel.forbidden in the Faraday image's x86_64-uefi board (ATA, BLK_DEV_NVME, NVME_CORE, MMC, EFIVAR_FS) and OpenSigner's common kernel.forbidden (NET, WLAN, BT), in Faraday's repository.
