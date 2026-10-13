# BitLocker and starting from a stick

## Why Windows asks for the recovery key

BitLocker encrypts the Windows drive and keeps its key in the computer's TPM, sealed to measurements of how the computer started. Device encryption is BitLocker turned on by Windows itself, on many laptops, when it is first set up with a Microsoft account or a work or school account.

One of those measurements, PCR 7, is the Secure Boot state. Turning Secure Boot off and starting Windows changes it, and so does turning it back on with other keys than before. The TPM then does not release the key, and Windows asks for the drive's 48-digit recovery key. Clearing or turning off the TPM, and some firmware updates, do the same.

Starting Faraday from the stick does not, on its own: Windows is not running then.

## Why only when Windows starts

The TPM is asked for the key only when Windows starts: Windows' boot manager asks for it, and the TPM checks the measurements of that start before it releases it. Changing a BIOS setting asks nothing, and neither does starting Faraday. So a computer whose Secure Boot is turned off, which then starts the stick, and whose Secure Boot is turned back on before Windows starts again, is never asked for the key: Windows only ever sees the start it was sealed to.

The prompt comes when Windows starts while Secure Boot is off. Most often that is the restart that saves the BIOS setting, when the stick is not chosen from the boot menu in time and Windows starts instead. It also comes when Secure Boot is turned back on with other keys than before.

## What Faraday Starter does

On a Windows computer with an encrypted drive, Faraday Starter's BitLocker step comes before any BIOS step. By default it is the recovery key in hand.

Show the recovery key. Windows asks for an administrator's permission, then the app shows the drive's recovery key; a drive without one gets one added first, and that is the one shown. Keep it: write it down or print it, scan its QR code with a phone, or take a photo of the screen. Save writes it to a file, offered only when a drive other than the encrypted ones is attached, and never onto a Faraday stick.

I have my recovery key. This opens the BIOS steps, for as long as Faraday Starter stays open. Nothing on the computer changes: BitLocker stays on, and a slip that starts Windows with Secure Boot off asks for the key that is in hand.

Extra safe: suspend BitLocker. This choice covers those slips. The key is shown as above; then it is hidden and the app asks for two of its eight groups, typed from what you kept. Then the app suspends BitLocker on the encrypted Windows drives until it is resumed. While it is suspended, Windows starts without the recovery key whatever the BIOS reports: Secure Boot off, Secure Boot on with other keys, or a cleared TPM. The key comes first, so stopping between the two changes nothing on the computer.

## What suspended means

This applies only to the extra-safe choice.

The drive stays encrypted, and what is written to it is encrypted. The key that opens it is kept on the drive in the clear, so anyone holding the computer can read the drive until BitLocker is resumed. Suspending costs protection while it lasts, never access.

Suspend protection in Windows' Control Panel ends at the next restart, which would be the restart into the BIOS. Faraday Starter's suspension has no restart count: it lasts until BitLocker is resumed.

## Resume, with Secure Boot on

After the extra-safe choice, BitLocker is resumed once Secure Boot is back on. Resuming seals the key again to how this start was measured. Resumed with Secure Boot off, the next start with Secure Boot on would ask for the recovery key. Faraday Starter offers Resume BitLocker only while Secure Boot is on, and says why when it is off. On a computer whose Secure Boot was already off when BitLocker was suspended, it offers Resume BitLocker with Secure Boot off, as it was.

Turn Secure Boot back on, start Windows, and choose Resume BitLocker in Faraday Starter. An installed Faraday Starter offers it at sign-in. One run without installing offers it when it is opened again.

Windows' own route, on Windows Pro, Enterprise and Education: Start, type BitLocker, open Manage BitLocker, and choose Resume protection on the operating system drive. Windows Home has Device encryption without that Control Panel; resume there with Faraday Starter.

## Where Faraday Starter cannot help

BIOS settings changed without Faraday Starter. Have the recovery key in hand, with Faraday Starter's BitLocker step, before any BIOS step.

Another encryption product, such as VeraCrypt system encryption or a maker's own. Faraday Starter does not manage it. Have its rescue disk or password before changing any BIOS setting.

An account that is not an administrator. An administrator of this computer has to do the BitLocker step.

A computer whose BitLocker an employer manages. Its recovery key is kept by the employer: its IT department has it. Suspending may be refused, or undone at the next start. Its administrator has to suspend it. Faraday Starter shows the drive as protected again if that happens.

A BIOS update, a repair shop, or someone resuming BitLocker in between. The kept recovery key is what opens the drive then.

Windows started while Secure Boot is off, without the extra-safe choice. Windows asks for the recovery key. Type the kept key; Windows starts. Then turn Secure Boot back on before the next start.

## Where the recovery key is

The one kept in the BitLocker step. A drive encrypted by Device encryption under a Microsoft account also has its key in that account, at aka.ms/myrecoverykey, which a phone can open. A computer an employer manages keeps it with the employer. The recovery screen shows a key ID, which names the key it wants.

## Where this comes from

Microsoft: "BitLocker recovery overview", "BitLocker countermeasures" and "BitLocker operations guide" (learn.microsoft.com, Windows security); "Suspend-BitLocker" (learn.microsoft.com, PowerShell); "DisableKeyProtectors method of the Win32_EncryptableVolume class" (learn.microsoft.com, Win32); "Device encryption in Windows" (support.microsoft.com). All read 2026-10-12.
