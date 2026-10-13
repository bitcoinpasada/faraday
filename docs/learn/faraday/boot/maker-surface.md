# Starting a Microsoft Surface from the stick

## BitLocker first

On Windows, do the BitLocker step in Faraday Starter on this computer before changing any firmware setting. On Linux, have the disk's passphrase or recovery key first if the TPM unlocks the disk. The page "BitLocker and starting from a stick" says why.

## Keys

A Surface has no keys for this; it uses its volume buttons. Shut it down and wait about 10 seconds first.

Firmware settings (Surface UEFI): hold Volume Up, press and release the power button, and keep holding Volume Up until the UEFI screen appears.

Start from a USB disk: hold Volume Down, press and release the power button, and keep holding Volume Down until spinning dots appear under the logo.

Faraday Starter's Restart into firmware settings opens Surface UEFI without the buttons.

## Turn Secure Boot off

In Surface UEFI, choose Security. Under Secure Boot, choose Change configuration, and choose the setting that turns Secure Boot off. Microsoft's pages name the keyset choices, such as Microsoft only, but not the off setting's label. Then choose Exit → Restart now.

## Turn Fast Boot off where it hides USB

Microsoft's Surface UEFI pages list no Fast Boot setting. The Boot configuration page turns USB Storage on and off; it has to be on.

## Start from the stick

Plug the stick in, and unplug any other USB disk: the Surface may start from that instead. Shut down, then hold Volume Down and press and release the power button, as above.

Or, in Windows, Settings → System → Recovery, Restart now beside Advanced startup, then Use a device → USB Storage. Or, in Surface UEFI, Boot configuration: swipe left on USB Storage to start from it once.

## Turn Secure Boot back on

Turn the Surface off from Faraday and pull the stick out. Open Surface UEFI with Volume Up, choose Security, under Secure Boot choose Change configuration, and choose Microsoft only. Then Exit → Restart now.

While Secure Boot is off, the Surface shows a red bar with an open padlock at power-on. Once Secure Boot is back on, it is gone.

## Known quirks

A Surface with a Snapdragon processor is an ARM computer and cannot start the stick. Faraday Starter says so on such a computer.

Surface Go uses a UEFI made by another company, and Surface Pro 3 and earlier have different menus; Microsoft's pages for those name their own settings.

## Where this comes from

Microsoft: "Boot Surface from a USB device", "How to use Surface UEFI" and "Surface turns on but shows a red bar and an unlocked lock icon" (support.microsoft.com); "Manage Surface UEFI settings" (learn.microsoft.com). All read 2026-10-12.
