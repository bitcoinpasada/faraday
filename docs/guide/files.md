# Files

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Files`
**Key:** files

The files copied in from the stick and the files waiting to be written to it, with what open vaults and the Inbox give. Both lists are held in memory and emptied at power-off.

## The stick

**Key:** files.stick

The top line is the state of the stick. **Scan a QR code** reads a code in place of a file.

With none attached the line reads that it is waiting for a stick. With a stick attached while a secret is held, it reads that the stick is attached and to lock to use it, and a press on it locks. Otherwise the line is the stick's label.

## Delete files

**Key:** files.delete-files

**Delete files** on the stick pane, and a file's **Delete from the stick**, open a sheet that writes only the stick's files partition. Its other partitions and partition table are not touched, so zeroing the partition reaches everything Faraday wrote to the stick.

**Chosen files** writes zeros over each file and removes it by name. **The free space** zeroes what earlier deletes left. **The whole partition** deletes every file and zeroes all of it, in **One pass** or **Two passes**. The sheet gives the time on a typical stick, and deleting a vault, zeroing the free space or the partition is a hold. No zeroing is certain: a stick can keep old blocks, so a secret once saved unencrypted is removed surely only by destroying the stick.

## Open vaults

**Key:** files.open-vaults

One panel per open vault, **Wallets with their seeds** first. With several wallets, each is chosen by its checkbox, and **Select all** chooses every one. A wallet shows how many of its keys are in this vault, and **Ready to sign once loaded** when it has enough.

**Load** is named for the choice, such as **Load wallet with keys**, **Load 1 wallet with keys** or **Load 2 wallets with keys**; with nothing chosen it reads **Nothing chosen**. **Each seed and wallet** lists every item of the vault, and **Hide each seed and wallet** closes the list. Once loaded, the panel reads **Everything is loaded** and **Go to Wallets** opens Wallets.

## What the Inbox gives

**Key:** files.inbox-gives

A panel shown when the Inbox holds something that can be loaded: **Wallets**, **Keys with no wallet**, **Xpubs with no wallet**, and backups that are **Not complete or sealed**.

A wallet row names its keys held here, with how many files agree or that they come from split sheets. **Load the wallet** or **Load the wallet with its seeds** loads the chosen ones; **Keys load only with no stick attached** stands in while a stick is attached. **Make it a wallet** makes a wallet of a key, and **Make it a watch-only wallet** of an xpub; an xpub without its origin shows **no origin**.

## Inbox

**Key:** files.inbox

One card per file copied from the stick, headed **From the stick** with the count, each with its name, its kind and a menu. **Empty** files are copied in on a stick visit.

## Inbox buttons

**Key:** files.inbox-buttons

The button follows the kind of file.

**Load the wallet** is a wallet file, **Load this key** a words file and **Read the wallet** a Bitcoin Core wallet. **Sign** is a PSBT, or **Add to signatures** when it is the open spend's own transaction. **Decode** is a transaction. **Nonce check** is a finished transaction, or a PSBT with a signature and no key here left to sign. **Check the signature** is a signed message, **Open in QR vault** a QR vault file and **Make PDF** a sheet, online only. **Unlock** or **Contents** is a vault. **Import into the vault** is an entries file and **Add to vault** a text file, with a vault open. **Restore the wallet** is a set of shares, with a count; until the set is whole the card reads how many keys are in hand.

## Inbox menu

**Key:** files.inbox-menu

The card's menu has the choices the file has: **Add to vault** for a wallet or words file with a vault open, **Restore from its paperkey** for a GPG public key, **Check a signer** and **Compare** for a PSBT, **Details** and **Remove**.

## Outbox

**Key:** files.outbox

The files waiting to be written to the stick, headed **For the stick** with the count, grouped by who may read them. Each card has its name, its state and a menu.

**Public** files may be read by anyone. **Written sealed** files are sealed under their passphrases before they reach the stick. **Unprotected secrets** may be used by anyone who has the stick; their state reads **Unprotected**. A file's state reads **Ready**, or **Written after Lock** for an open vault, which is sealed and locked at Lock. The card's menu has the choices the file has: **Make PDF** for a sheet, online only, **Show as QR** where the file fits one, **Check a signer** and **Compare** for a PSBT, **Rename**, **Details** and **Remove**. Power off asks first while files wait.
