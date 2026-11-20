# Files

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Files`
**Key:** files
**Words:** inbox, for-the-stick, stick-visit, vault, psbt, lock

Files shows the files copied in from the stick and the files waiting to be written to it, along with what the open vaults and the Inbox can load. Both lists are held in memory and emptied at power-off.

## The two cards

**Key:** files.cards

Files is one page whether a stick is attached or not: **In Faraday** holds everything Faraday holds (the open vaults, the settings, the files waiting to be written, and the Inbox) and **On the stick** holds the stick's files. Side by side on a wide window; on a small one they are the tabs **In Faraday** and **On the stick**. The stick's error line shows on **On the stick**, and on a small window above the **In Faraday** list too.

**Write 3 files** at the foot of **In Faraday** writes the ticked files to the stick. At the foot of **On the stick**, **Import 3 files** copies the ticked files in and **Import and load** copies them in and loads them once the stick is pulled; a line under the list says what pulling the stick will do next. Pulling the stick keeps you on Files.

## The stick

**Key:** files.stick

The top bar gives the state of the stick in a chip. With no stick attached, **On the stick** says it is waiting for one. With a stick attached while a secret is held, it says to lock to use the stick, with a **Lock** button; the secrets leave memory. Otherwise the card lists the stick's files, and its heading has the stick's label as a chip and **Rename stick**. A stick holding one PSBT offers **Import and sign** and the file's name: only that file comes in, and it lands on its review.

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

**Delete files** on the **On the stick** card, and a file's **Delete from the stick**, open a sheet that writes only to the stick's files partition. Its other partitions and its partition table are not touched; Faraday only ever writes to that partition, so zeroing it reaches everything Faraday wrote to the stick.

**Chosen files** writes zeros over each file and removes it by name. **The free space** zeroes what earlier deletes left. **The whole partition** deletes every file and zeroes all of it, in **One pass** or **Two passes**. The sheet gives the time it takes on a typical stick. Deleting a vault, or zeroing the free space or the partition, needs a held press. No zeroing is certain, because a stick can keep old blocks: if a secret was ever saved to it unencrypted, the only sure way to remove it is to destroy the stick.

## Open vaults

**Key:** files.open-vaults

An open vault has a row in the **In Faraday** list, with **Contents**, which opens it in Vaults. Its ⋮ menu holds **Load wallet with keys**, which loads the vault's wallets together with their seeds; with several it reads **Load 2 wallets with keys**, and with none left to load the menu offers nothing. Choosing which wallets, and listing each seed and wallet, are done in Vaults.

## What the Inbox gives

**Key:** files.inbox-gives

This panel shows when the Inbox holds something that can be loaded: **Wallets**, **Keys with no wallet**, **Xpubs with no wallet**, and backups that are **Not complete or sealed**.

A wallet row names its keys held here, with how many files agree or that they come from split sheets. **Load the wallet** or **Load the wallet with its seeds** loads the chosen ones; **Keys load only with no stick attached** stands in while a stick is attached. **Make it a wallet** makes a wallet of a key, and **Make it a watch-only wallet** of an xpub; an xpub without its origin shows **no origin**.

## Inbox

**Key:** files.inbox

One row per file copied in from the stick, under the heading **From the stick** with the count; each row has the file's name, its kind, a button and a menu. A vault file that is open reads **Vault, open**. **Empty** shows until files are copied in on a stick visit. Where a row is too narrow to keep the name readable, its button is left out.

## Inbox buttons

**Key:** files.inbox-buttons

A row's button by kind.

**Load the wallet** is a wallet file, **Load this key** a words file and **Read the wallet** a Bitcoin Core wallet and **Open** a Krux encrypted code (a `.kef` file the Krux tool wrote): it asks the code's passphrase and loads the key, as **From Files** on Add a key's Krux code form does. **Sign** is a PSBT, or **Add to signatures** when it is the open spend's own transaction. **Decode** is a transaction. **Nonce check** is a finished transaction, or a PSBT with a signature and no key here left to sign. **Check the signature** is a signed message and **Make PDF** a sheet, online only. **Unlock** (not while a stick is attached) or **Contents** is a vault. **Import into the vault** is an entries file and **Add to vault** a text file, with a vault open. **Restore the wallet** is a set of shares, with a count; it opens Restore on its Description card with the shares ticked, which reads how many keys are in hand.

## Inbox menu

**Key:** files.inbox-menu

The card's menu holds the choices that apply to the file: **Add to vault** for a wallet or words file with a vault open, **Restore from its paperkey** for a GPG public key, **Decode** and **Check a signer** for a PSBT or a finished transaction, **Sign this message** and **Check an address** for a text file, **Details** and **Remove**. A PSBT that Sign refuses opens its Details, which say why under **Not opened**.

## For the stick

**Key:** files.outbox

The files waiting to be written to the stick, in groups by who may read them, each heading with its count. Each row has the file's name, its state (**Ready**, **Written after Lock** for an open vault's file, or **Unprotected**) and a menu. An open vault's row says how many changes are not yet saved.

**Public** files may be read by anyone. **Written sealed** files are sealed under their passphrases before they reach the stick. **Unprotected secrets** can be used by anyone who has the stick. An unprotected secret stays in memory until it is sent or removed, or until power-off: Lock does not drop it. The card's menu has the choices the file has: **Make PDF** for a sheet, online only, **Show as QR** where the file fits one, **Check a signer** for a PSBT, **Rename**, **Details** and **Remove**, which reads **Delete**, held, for a file no stick holds.
