# Replacing Faraday on a stick

## What it does

Make a Faraday stick → Replace Faraday, keep data writes a new Faraday over the stick's boot partition, reads it back and compares. The data partition, with its vaults and settings file, is not read and not written.

The rules are the stick's own Upgrade a Faraday stick's. A stick that already holds the same build is not written. Writing an older Faraday over a newer one is marked on Confirm and can still be done. The stick's boot partition has to be at least as large as the new one's; a stick whose boot partition is smaller has to be written whole, which empties its data partition.

## What the computer can see

Plugging the stick into this computer is what exposes its data partition, whatever the Faraday app does. The computer can read and write it, and Windows gives it a drive letter. The vaults there are encrypted with their passphrases. The settings file is not encrypted. A program on this computer could read the settings file, and could change or delete anything on the data partition.

## Without a computer

Settings → Upgrade a Faraday stick, on a stick that already holds the newer Faraday, copies that Faraday onto the stick that holds vaults. The stick with vaults then never goes into a computer other than the one Faraday runs on.

Write the new Faraday onto a spare stick here with New Faraday stick, start from the spare, and upgrade the stick with vaults from it. The page "Upgrading a Faraday stick" says how.

## Where this comes from

Faraday's own design: the hub app's specification (sections 3.2 and 4.4) and decisions F2, F5 and F11, in Faraday's repository, as of 2026-10-12.
