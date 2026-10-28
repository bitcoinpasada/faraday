# Files

**Kind:** Spec
**Status:** Written from the app's code and strings (`faraday/faraday-core/src/screens/files.rs`, `faraday/faraday-core/src/file_menu.rs`, `faraday/faraday-core/src/screens/sheets.rs`, `faraday/faraday-core/src/secrets.rs`); not yet read beside the running app
**Open:** the pictures, drawn once after the theming has landed
**Screen:** `Files`, `FileMenu`, `Rename`, `Details`, `Potential`, `SecretOut`
**Group:** Files

**Opened from:** the sidebar's **Files**, and its **Files** row at the
foot; **Open Files** on a stick visit; Home's **Written to …** lead; an
empty vault's "Write it to a stick". **Import and load** on a stick
visit opens it with the files just copied in.

Files is what Faraday holds in memory: **From the stick**, the files
copied in, and **For the stick**, the files waiting to be written.
Both are emptied at power-off.

*Picture to come: `files-both.png`, Files with a PSBT from the stick and a vault for the stick.*

## On this screen

| Control | What it does |
|---|---|
| the stick chip | at the top right: "Waiting for a stick"; "Stick attached · lock to use it" while a secret is held, which locks; "Visit Boot stick", which opens the [stick visit](stick-visit.md) |
| **In memory · emptied at power-off** | a label |
| **Scan a QR code** | opens the camera ([Scan](qr-and-scan.md#scan)); what it reads lands in **From the stick** |
| "Remove the stick when you are done" | in the warning tone, after **Import and load**, while the stick is still in: the keys copied in load once it is out ("1 key copied in · remove the stick to load it") |
| an open vault's panel | see [An open vault](#an-open-vault) |
| what the files give | see [What the files give](#what-the-files-give) |
| **From the stick** | the files copied in, with their count; each row its name, what it is and its size, its button, and its **⋮** |
| **For the stick** | the files waiting to be written, with their count, in three groups |
| "Power off asks first while files wait for the stick" | at the foot, while **For the stick** holds a file |

### A file from the stick

Each row has one button for what its kind is for, and **⋮** for the rest.

| Kind | Button | What it does |
|---|---|---|
| a PSBT | **Sign** | starts its spend |
| a PSBT of the spend under way, from another file | **Add to signatures** | adds its signatures to the spend |
| a wallet | **Load the wallet** | loads it and opens its card |
| seed words | **Load this key** | loads the key |
| a vault | **Unlock**, or **Contents** while it is open | opens its [Unlock](vaults.md#unlock) or its [contents](vault.md) |
| entries | **Import into the vault** | adds them to the open vault. Shown while a vault is open |
| a text file | **Add to vault** | adds it to the open vault as a note. Shown while a vault is open |
| a transaction | **Decode** | opens Decode on it |
| a signed message | **Check the signature** | opens Check a signed message on it |
| shares of a wallet | "Restore the wallet · 3 shares" | opens Restore with them; "2 of 3 keys in hand" while some are missing |
| a recovery sheet | **Make PDF** | Faraday Demo only: makes its PDF |

**Empty · files are copied in on a stick visit** while there is none.

### Files for the stick

| Group | Line | A row's state |
|---|---|---|
| **Public** | **Anyone may read these** | **Ready** |
| **Written sealed** | **Sealed under their passphrases before they reach a stick** | **Ready**; **Written after Lock** for an open vault, whose row reads "Vault, open · 2 unsaved changes · sealed and locked at Lock" |
| **Unprotected secrets** | **Anyone who has the stick can use these**, in the error tone | **Unprotected** |

A file there has no button; its **⋮** holds the rest. **Empty** while
there is none. After a stick visit wrote, the receipt follows: "Written
to Boot stick at 14:02", then each file with **verified** or why it was
not.

## An open vault

A panel for each open vault, headed by its file and name ("vault.ofv ·
Vault 1 · open").

| Control | What it does |
|---|---|
| **Select all** | ticks every key and wallet not loaded yet, or none |
| **Wallets with their seeds** | each wallet the vault holds with its seeds: "2 of 3 keys in this vault", and **Ready to sign once loaded** where they are enough; a press ticks it |
| **Load wallet with keys**, **Load 1 wallet with keys**, "Load 2 wallets with keys" | loads the wallets ticked, with their seeds. **Nothing chosen** while none is |
| "Each seed and wallet · 5" | opens the full list, each row ticked to load or **Loaded**; **Hide each seed and wallet** closes it |
| "Load 2 keys and 1 wallet" | loads what the full list ticks. **Everything is loaded** once nothing is left |
| **Go to Wallets** | shown once something has loaded |

## What the files give

Shown while the files from the stick give something to load.

| Control | What it does |
|---|---|
| **Wallets** | each wallet the files describe, once each: its shape, "2 of 3 keys here", "· 2 files say it" or "· from split sheets", and **Ready to sign once loaded**; a press ticks it |
| **Load the wallet with its seeds**, **Load the wallet**, "Load 2 wallets with their seeds" | loads the wallets ticked. **Nothing chosen** while none is. "Keys load once the stick is out" while a stick is in |
| **Keys with no wallet** | each seed no wallet names, by fingerprint, with **Make it a wallet**, which opens [Make it a wallet](#make-it-a-wallet) |
| **Xpubs with no wallet** | each xpub, by its fingerprint or **no origin**, with **Make it a watch-only wallet** |
| **Not complete or sealed** | a backup short of parts, or sealed |

## The ⋮ menu

**Opened from:** a file row's **⋮**: a menu under it, a sheet on a
small window.

*Picture to come: `files-menu.png`, a file's ⋮ menu open on For the stick.*

| Row | Choices |
|---|---|
| a file from the stick | **Add to vault** (a wallet or seed words, while a vault is open), **Details**, **Remove** |
| a file for the stick | **Make PDF** (a recovery sheet, Faraday Demo only), **Show as QR** (where it fits in codes; never a vault), **Rename**, **Details**, **Remove** |
| a file on a stick visited | **Rename**, "Delete from the stick", **Details** |

**Remove** takes the file out of Files. "Delete from the stick" opens
[Delete files](stick-visit.md#delete-files) with that file alone
chosen.

### Rename

Its title is **Rename …** and the file's name.

| Control | What it does |
|---|---|
| the field | the name before its extension; the extension stays as it is |
| the line under it | why a name cannot be used: **Type a name**, **A name cannot start with a dot**, the characters a stick's names cannot hold, **A name is at most 255 characters with its extension**, "Another file for the stick is named …" |
| **Rename** | renames the file; on a stick, the stick visit's line says "… renamed …", or why not |
| **Cancel** | closes the sheet |

### Details

| Field | What it is |
|---|---|
| **Name** | the file's name |
| **Kind** | what it is |
| **Size** | "… bytes" |
| **SHA-256** | its hash in groups of eight; **Reading…** while a stick is read for it; "Not read: …" |
| **Close** | closes the sheet |

## Make it a wallet

**Opened from:** **Make it a wallet** beside a seed with no wallet, and
**Make it a watch-only wallet** beside an xpub. Its title is "A wallet
from 3dca6805", "A watch-only wallet from 3dca6805", or **A watch-only
wallet from an xpub**.

*Picture to come: `files-make-it-a-wallet.png`, A wallet from 3dca6805 with its kinds.*

| Control | What it does |
|---|---|
| "From …" | the files the seed came from |
| **BIP-39 passphrase · none if left empty** | the seed's passphrase field (**SLIP-39 passphrase · none if left empty** for SLIP-39 shares); its eye shows what is typed. **Fingerprint** under it gives the key it makes |
| "A key of New wallet 1: it loads with that wallet" | in the ok tone, where the key is one of a wallet's |
| **Single-key wallet** | otherwise, with the kinds to choose: native SegWit, **Taproot**, nested SegWit, legacy |
| the xpub | for an xpub: the key, and its kind, chosen, or "… · as its path says" |
| **Make the wallet**, or "Load New wallet 1" | makes the wallet, or loads the wallet the key belongs to |
| **Cancel** | closes the sheet |

## This is a secret

**Opened from:** a secret about to leave Faraday: a seed's file from a
backup's checklist, a child seed, a Lightning or silent payments key, a
GPG revocation, and a secret file ticked on a stick visit to be copied
to another stick. Its title is **This is a secret**, or **This may be a
secret** for text Faraday cannot tell about.

*Picture to come: `files-this-is-a-secret.png`, This is a secret for a seed file, with Save into Vault 1.*

| Control | What it does |
|---|---|
| **File**, **What it is**, **Who can use it** | the file's name, what it holds, and who can use it |
| **Sealed in a vault** | in the ok tone, where the secret can go into a vault |
| "Save into Vault 1" | puts the secret into the open vault, sealed. "In Vault 1" in its place where the vault holds it already |
| "Make a vault", "Unlock vault.ofv", **Unlock a vault** | with no vault open: opens [Create a vault](create-vault.md) or [Unlock](vaults.md#unlock); the sheet opens again after |
| "Unprotected, for the stick" | in the error tone: what the other way is; "Unprotected, on the stick" for a file a stick visit copies |
| the forms | where the secret has more than one form (a seed's **Words** or **SeedQR**), a button each |
| "Put it for the stick unprotected" | opens [Save without encryption?](#save-without-encryption) |
| **Cancel** | closes the sheet; the secret goes nowhere |

### Save without encryption?

| Control | What it does |
|---|---|
| the two lines | "… goes on the stick as a plain file, outside any vault. Anyone who has the stick can read it." and "That is as safe as writing it on paper: …" |
| **Hold to save without encryption** | held until it fills: puts the file in Files under **Unprotected secrets**; a file from a stick visit is ticked there instead, and written with the rest |
| **Put it in a vault** | while a vault is open: puts the secret into it instead |
| **Cancel** | closes the sheet |

The same sheet comes before a key's words or SeedQR are saved to a
stick from a vault, before a backup's plan keeps a seed as a plain file
at a place, and before Transfer saves a secret.

## On a narrow window

One column: the stick's status and **Scan a QR code**, the vault panels
and what the files give, then **From the stick**, then **For the
stick**, each row's button under its name.

## Why

- [The air gap](../../learn/07-air-gap.md)
- [Coordinator files](../../learn/19-coordinators.md)
