# Silent payments

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Silent`
**Key:** silent

Shows the silent payments address of a loaded key, with its labels, its record and its scan key, and checks whether a transaction pays it. A silent payments address starts with sp1q and is given out many times; each payment to it lands at an address that is not linked to the others.

## Key

**Key:** silent.key

One button per loaded key, by its fingerprint and name. Choosing one opens **Address**. A wallet opened from Wallets starts with its own key; while that key is not loaded, **Address** shows the address as the wallet's record states it.

## Address

**Key:** silent.address

**-** and **+** step the label: **No label** is the plain address, and each label number gives another address of the same wallet, so payers can be told apart. A label shown is counted as handed out, and the record lists it from then on.

**Show as QR** shows the address, **As a bitcoin: link** as a link. Record writes the public record to Files, and **Record as QR** and **Record as PNG** give it as a code or an image. **Add as a wallet** adds it to Wallets, offered while it is not one. **Continue** opens **Scan key**.

With the key not loaded, the address is marked not checked; labels, the scan key and a check need the key, and **Add its key** loads it. A record the loaded key contradicts shows no address.

## DNS record

**Key:** silent.dns

**DNS record** takes a **Name** and a **Domain** and shows the record line for them, with **DNS record as QR**. **Not a valid DNS name** shows under a field that is not one.


## Scan key

**Key:** silent.scan

The scan key shows every payment to the address and cannot spend any. Whoever holds it sees all of them.

Save into the open vault is offered when a vault is open, and otherwise a vault is made or unlocked first and the step is returned to. **Out unprotected…** writes the key to a file after a warning that it is a secret.

## Check a payment

**Key:** silent.check

Checks one signed transaction. The transaction is copied into Files with the transactions its inputs spend from. One button per file that reads as a transaction, Check followed by its name; **No transaction in Files** shows when there is none.

The result is one of **Pays nothing to this wallet**, **Spends nothing a silent payment is made from**, **Not signed: an input's key is not in it yet** and **Its key is not loaded**, or **Pays this wallet** with a row per output: its number, its amount and whether it went to the address, to a label or to change. When a previous transaction is missing, its transaction id is named and the check is made again after it is copied into Files. Nothing is read from a network.
