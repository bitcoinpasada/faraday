# Silent payments

**Kind:** Spec
**Status:** Written from the app's code and strings (`faraday/faraday-core/src/silent/screen.rs`, `faraday/faraday-core/src/silent/mod.rs`); not yet read beside the running app
**Open:** the pictures, drawn once after the theming has landed
**Screen:** `Silent`
**Group:** Tools

**Opened from:** **Tools → Silent payments**, which reads **Load a key
first** and does not open while no key is loaded; **Address, labels and
payments** on a silent payments wallet's card on
[Wallets](wallets.md#a-silent-payments-wallet), which opens it on
**Address**, whether or not the wallet's key is loaded.

A loaded key's silent payments address, its scan key, and a check of
whether a transaction pays it. A column of four cards; with one key
loaded it opens on **Address**, that key chosen.

*Picture to come: `silent-payments.png`, Silent payments with the Address card open on key 3dca6805, no label.*

## On this screen

| Control | What it does |
|---|---|
| the way back | at the top left, named after where the page was opened from |
| **Steps only**, **Guided** | at the top right: the short text on each open card, off or on, with a link to Learn's **Silent payments** |
| a card | its title and what was chosen; a press opens it again |

## The cards

### Key

Closed, the key's fingerprint, or **Not chosen**.

| Control | What it does |
|---|---|
| a key's button | one per key loaded, by fingerprint and name; the chosen one filled |

### Address

Closed, the address, shortened.

| Control | What it does |
|---|---|
| **-**, **No label**, **+** | the label: **No label**, or "Label 1" and on. Each label makes another address of the same wallet; a label shown counts as handed out, and the wallet's record lists it from then on |
| the address | in full, for the label shown |
| **Show as QR** | the address as a [QR code](qr-and-scan.md#a-qr-code) |
| **As a bitcoin: link** | the same as a `bitcoin:` link in a code |
| **Record for the stick** | the wallet's record as a file, `silent-3dca6805.txt`, for the stick |
| **Record as QR**, **Record as PNG** | the record as a code, or as a picture file for the stick |
| **Add as a wallet** | while the key has no silent payments wallet loaded: adds one |
| **Continue** | closes the card and opens **Scan key** |

For a wallet whose key is not loaded, the card shows the wallet's
address, "Not checked: load key 3dca6805 to check this address" in the
warning tone, **Its key is not loaded: labels, the scan key and a check
need it**, **Show as QR**, and **Add its key**, which opens
[Add a key](add-a-key.md).

### Scan key

Closed, **A secret: shows every payment**.

| Control | What it does |
|---|---|
| **Secret · shows every payment, cannot spend** | a tag in the warning tone |
| "Save into Vault 1" | the scan key into the open vault |
| **Make a vault**, "Unlock vault.ofv", **Unlock a vault** | in its place while no vault is open: makes or unlocks one, then comes back here |
| **Out unprotected…** | the scan key to the stick unprotected, through [This is a secret](files.md#this-is-a-secret) |

### Check a payment

Closed, what the last check came to, **Needs a previous transaction**,
or **A transaction from Files**.

| Control | What it does |
|---|---|
| "Check payment.txn" | one per file in Files that reads as a transaction: checks it |
| **No transaction in Files** | where none does |
| the file checked | its name, then what it came to |
| "Pays this wallet: 1 output" | in the ok tone; under it each output, "Output 0 · 0.01000000 BTC · to the address" (**change**, or "label 2") |
| **Pays nothing to this wallet** | none of its outputs is this wallet's |
| **Spends nothing a silent payment is made from** | none of its inputs counts |
| **Not signed: an input's key is not in it yet** | in the warning tone |
| **Its key is not loaded** | in the warning tone |
| "Needs the transaction it spends from, …: copy it into Files and check again" | in the warning tone, with that transaction's id |

Where a loaded key contradicts the wallet's record, every card after
**Key** shows only, in red, "This wallet's record does not match key
3dca6805: its scan key or spend key is not that key's. Do not use its
address".

## On a narrow window

The top bar holds the way back and the name, **Silent payments**; the
buttons wrap.

## Why

- [Silent payments](../../learn/20-silent-payments.md)
- [Xpubs and privacy](../../learn/11-xpubs.md)
