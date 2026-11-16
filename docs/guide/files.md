# Files

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Files`
**Key:** files

Files shows the files copied in from the stick and the files waiting to be written to it, along with what the open vaults and the Inbox can load. Both lists are held in memory and emptied at power-off.

## The stick

**Key:** files.stick

The top line gives the state of the stick. **Scan a QR code** reads a code in place of a file.

With no stick attached, the line says it is waiting for one. With a stick attached while a secret is held, it says the stick is attached and that you must lock to use it; pressing the line locks. Otherwise the line is the stick's label.

## Formatting a stick

**Key:** files.format-stick

A plain USB stick carries files on a stick visit while no vault is open. It needs a partition table (MBR or GPT) with a FAT32 or FAT16 partition: a filesystem on the whole device is not seen, and exFAT, NTFS, ext4, APFS and FAT12 are not read. Sticks over 32 GB usually come as exFAT and need reformatting. A boot partition (labelled `EFI`, `bootfs` or `FARADAYBOOT`, or named `EFI system partition`) is never opened. Files are read at the top level only, up to 18 MiB each.

On Linux, with the stick at `/dev/sdX` (see `lsblk`; this erases it):

```
sudo parted --script /dev/sdX mklabel msdos mkpart primary fat32 1MiB 100%
sudo mkfs.fat -F 32 -n FARADAY /dev/sdX1
```

On macOS, with the stick at `/dev/diskN` (check with `diskutil list`):

```
diskutil eraseDisk FAT32 FARADAY MBRFormat /dev/diskN
```

On Windows, Explorer formats FAT32 only up to 32 GB; for a larger stick, make a FAT32 partition of 32 GB or less in Disk Management.

The Raspberry Pi reads sticks alike; its screens say "SD card" for "stick".

## Delete files

**Key:** files.delete-files

**Delete files** on the stick pane, and a file's **Delete from the stick**, open a sheet that writes only to the stick's files partition. Its other partitions and its partition table are not touched; Faraday only ever writes to that partition, so zeroing it reaches everything Faraday wrote to the stick.

**Chosen files** writes zeros over each file and removes it by name. **The free space** zeroes what earlier deletes left. **The whole partition** deletes every file and zeroes all of it, in **One pass** or **Two passes**. The sheet gives the time it takes on a typical stick. Deleting a vault, or zeroing the free space or the partition, needs a held press. No zeroing is certain, because a stick can keep old blocks: if a secret was ever saved to it unencrypted, the only sure way to remove it is to destroy the stick.

## Open vaults

**Key:** files.open-vaults

One panel per open vault, with **Wallets with their seeds** first. With several wallets, tick the ones you want; **Select all** ticks every one. A wallet shows how many of its keys are in this vault, and **Ready to sign once loaded** when it has enough.

**Load** is named for the choice, such as **Load wallet with keys**, **Load 1 wallet with keys** or **Load 2 wallets with keys**; with nothing chosen it reads **Nothing chosen**. **Each seed and wallet** lists every item of the vault, and **Hide each seed and wallet** closes the list. Once loaded, the panel reads **Everything is loaded** and **Go to Wallets** opens Wallets.

## What the Inbox gives

**Key:** files.inbox-gives

This panel shows when the Inbox holds something that can be loaded: **Wallets**, **Keys with no wallet**, **Xpubs with no wallet**, and backups that are **Not complete or sealed**.

A wallet row names its keys held here, with how many files agree or that they come from split sheets. **Load the wallet** or **Load the wallet with its seeds** loads the chosen ones; **Keys load only with no stick attached** stands in while a stick is attached. **Make it a wallet** makes a wallet of a key, and **Make it a watch-only wallet** of an xpub; an xpub without its origin shows **no origin**.

## Inbox

**Key:** files.inbox

One card per file copied in from the stick, headed **From the stick** with the count; each card has the file's name, its kind and a menu. **Empty** shows until files are copied in on a stick visit.

## Inbox buttons

**Key:** files.inbox-buttons

Each card's button depends on the kind of file.

**Load the wallet** is a wallet file, **Load this key** a words file and **Read the wallet** a Bitcoin Core wallet. **Sign** is a PSBT, or **Add to signatures** when it is the open spend's own transaction. **Decode** is a transaction. **Nonce check** is a finished transaction, or a PSBT with a signature and no key here left to sign. **Check the signature** is a signed message, **Open in QR vault** a QR vault file and **Make PDF** a sheet, online only. **Unlock** or **Contents** is a vault. **Import into the vault** is an entries file and **Add to vault** a text file, with a vault open. **Restore the wallet** is a set of shares, with a count; until the set is whole the card reads how many keys are in hand.

## Inbox menu

**Key:** files.inbox-menu

The card's menu holds the choices that apply to the file: **Add to vault** for a wallet or words file with a vault open, **Restore from its paperkey** for a GPG public key, **Check a signer** for a PSBT, **Details** and **Remove**.

## Outbox

**Key:** files.outbox

The files waiting to be written to the stick, headed **For the stick** with the count and grouped by who may read them. Each card has the file's name, its state and a menu.

**Public** files may be read by anyone. **Written sealed** files are sealed under their passphrases before they reach the stick. **Unprotected secrets** can be used by anyone who has the stick; their state reads **Unprotected**. An unprotected secret stays in memory until it is sent or removed, or until power-off: Lock does not drop it. A file's state reads **Ready**, or **Written after Lock** for an open vault. The card's menu has the choices the file has: **Make PDF** for a sheet, online only, **Show as QR** where the file fits one, **Check a signer** for a PSBT, **Rename**, **Details** and **Remove**, which reads **Delete**, held, for a file no stick holds.
