# BitLocker and starting from a stick

## The recovery key is not needed

BitLocker's key is checked only when Windows starts. Turn Secure Boot off, start Faraday from the stick, and turn Secure Boot back on before Windows starts: Windows does not ask for the recovery key.

It asks only when Windows starts while Secure Boot is off, most often on the restart that saves the setting, when the stick is not chosen from the boot menu in time. Type the recovery key and Windows starts. Then turn Secure Boot back on before the next start.

## A backup of the key, if you want one

Show the recovery key, on Faraday Starter's BitLocker page. Windows asks for an administrator's permission first. Write the key down, scan its QR code with a phone, or take a photo of the screen. Save writes it to a file on another drive, never onto a Faraday stick.

A computer set up with a Microsoft account also keeps the key at aka.ms/myrecoverykey. A computer an employer manages keeps it with the employer's IT department.

## Extra safe: suspend BitLocker

For Windows to start without the key even with Secure Boot still off. Faraday Starter shows the key, asks for two of its groups back, then suspends BitLocker until it is resumed.

The drive stays encrypted, but while suspended the key that opens it is kept on the drive in the clear: someone holding the computer could read the drive. Resume BitLocker in Faraday Starter once Secure Boot is back on and Windows has started again; an installed Faraday Starter offers it at sign-in. On Windows Pro, Enterprise and Education, Manage BitLocker's Resume protection does the same.

## When Faraday Starter cannot show the key

Another encryption product, such as VeraCrypt: have its rescue disk or password before changing any BIOS setting. An account that is not an administrator's: an administrator has to show the key. A computer whose BitLocker an employer manages: the employer has the key, and may refuse a suspension.

## Where this comes from

Microsoft: "BitLocker recovery overview", "BitLocker countermeasures" and "BitLocker operations guide" (learn.microsoft.com, Windows security); "Suspend-BitLocker" (learn.microsoft.com, PowerShell); "Device encryption in Windows" (support.microsoft.com). All read 2026-10-12.
