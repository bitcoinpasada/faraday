# Replacing Faraday on a stick

## What it does

Make a Faraday stick → Replace Faraday, keep data writes a new Faraday over the stick's boot partition, reads it back and compares. The data partition, with its vaults and settings file, is not read and not written.

This is how a Faraday stick is upgraded: Faraday itself writes no boot partition. The new Faraday is checked against the release's manifest before it is written. A stick that already holds the same build is not written. Writing an older Faraday over a newer one is marked on Confirm and can still be done. The stick's boot partition has to be at least as large as the new one's; a stick whose boot partition is smaller has to be written whole, which empties its data partition.

A Faraday stick or Pi card needs a USB stick or SD card of 256 MB or larger. Faraday's own boot file takes about 14 MB of the stick's 64 MiB boot partition, and the Pi card's boot files about 16 MB of its 64 MiB; the rest of that partition is room for later versions to grow. The 128 MiB data partition is storage for files: vaults, PSBTs and anything else brought to the device.

## What the computer can see

Plugging the stick into this computer is what exposes its data partition, whatever Faraday Starter does. The computer can read and write it, and Windows gives it a drive letter. The vaults there are encrypted with their passphrases. The settings file is not encrypted. A program on this computer could read the settings file, and could change or delete anything on the data partition.

## Where this comes from

Faraday's own design: the Starter's specification (sections 3.2 and 4.4) and decisions F2, F5, F11, F80 and F128, in Faraday's repository, as of 2026-10-26.
