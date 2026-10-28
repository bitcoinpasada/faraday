# Restore an existing wallet

**Kind:** Reference
**Status:** Checked against the app's pictures in docs/guide/pictures/ (drawn by just guide-pictures) and its code and strings (`faraday/faraday-core/src/screens/restore.rs`, `faraday/faraday-core/src/restore.rs`, `faraday/faraday-core/src/seeds/screen.rs`, `faraday/faraday-core/src/screens/parts.rs`)
**Screen:** `Restore`
**Group:** Wallets

**Opened from:** **Restore an existing wallet** on Home, on Wallets'
first page and on Tools; **Make a wallet from this key** and **Add
another key** on Wallets with keys and no wallet, which open it on
**Keys** with the key ticked; "Restore the wallet · 3 shares" on shares
in Files; **Restore its wallet** on the Spend tab.

One card per step, from what the wallet is to the wallet loaded. A
wallet description (a descriptor, a wallet file, a BIP 129 record) fills
the first cards at once.

![Restore an existing wallet with Keys open: key 1 3dca6805 is Key 1 of the wallet, keys 2 and 3 Not here, and Make the wallet](../pictures/restore-keys.png)

*A 2-of-3 wallet read from its description: **Kind**, **Quorum** and **Description** filled from it, **Use vaults** **No**, and on **Keys** key 1 reads **Key 1 of the wallet**, keys 2 and 3 **Not here** with **Change**.*

## On this screen

| Control | What it does |
|---|---|
| the way back | at the top left, the arrow with a name under the title: **Home** or **Wallets**, where it was opened from; once a card is closed, the card before the one open |
| **Steps only**, **Guided** | at the top right: the short text on the open card, off or on |
| a card | its number, its title, what it holds, and **Change**. A card filled from a description has none |
| **Cancel** | at the top right, while the wallet is in progress: asks, then leaves ([Cancel a flow](prompts.md#cancel-a-flow)) |

The cards: **Kind**, **Quorum** (for several keys), **Description**,
**Use vaults**, **Number of vaults** (after **Yes**), **Keys**,
**Check**, **Done**.

## Kind

| Control | What it does |
|---|---|
| **I have the wallet description** | **Scan it** opens the camera; **From Files**, while Files holds one, reads it. Either fills **Kind**, **Quorum** and **Description**, loads the wallet under its file's name, and goes on to **Keys** |
| the kinds | **Single key · native SegWit** and **Multisig · native SegWit**, each with its line; **More kinds** opens the rest. A press chooses one and moves on |
| **Continue** | for a kind read from a description |

## Quorum

**Signatures** and **Keys**, steppers, the line under them ("2
signatures of 3 keys"), and **Continue**.

## Description

The wallet's description, where it is:

| Control | What it does |
|---|---|
| "From new-wallet-1-descriptor.txt" | a description in Files: reads it |
| **Scan descriptor** | the camera |
| **Shares of a split backup** | shares of a description in Files, each a row to press; "2 of 3 keys in hand · …", then **Rebuild the wallet** once they are enough |
| the stick | **Copy files in** opens the [stick visit](stick-visit.md), and the card comes back. Grey with "Insert the stick", or **Not while a key is loaded** |
| **Vault** | "Unlock vault.ofv" for a vault locked in Files; each wallet an open vault holds, "New wallet 1 · Vault 1", which loads it; "Copy the vault in from its stick" while there is neither |
| **I have the seed** | for a single key: no description; the wallet is made from the seed alone |
| **I do not have it** | for several keys: the wallet is made from every key, seed or xpub |

Closed, the card reads the wallet's name, or **None**.

## Use vaults

**Will you use vaults as part of this backup?**: **Yes** or **No**,
then, after **Yes**, **How many vaults will be part of this backup?**
on **Number of vaults**.

## Keys

A row per key of the wallet, by fingerprint where the description
names it. Closed, the card reads "1 of 3 filled".

| Row reads | Means |
|---|---|
| **Empty** | nothing yet |
| **Key 1 of the wallet** | in the ok tone, for a wallet read from its description: a seed loaded is this row's key |
| **Here**, or "Here · New key 1" | for a wallet with no description: a seed loaded fills the row |
| **Not here**, with **Change** | the key stays away; the wallet signs with the others |
| a cosigner's xpub | the key's public part, with **Change** |

The open row's ways in stand over it:

| Control | What it does |
|---|---|
| **Type the words** | opens the [typing page](add-a-key.md); its passphrase is asked there |
| **Scan a SeedQR** | the camera; the passphrase is asked after the scan |
| "Load 3dca6805 from vault.ofv", "Use 3dca6805" | a seed an open vault holds, or one already loaded |
| **Its xpub** | a cosigner's key: **Type or paste an xpub**, or **Scan**. **Not an account key** under one that is not |
| **Not here** | for a wallet of several keys: the key stays away |
| **Make the wallet** | once every row is filled |

A seed that is not the row's key is refused and nothing loads: **Not a
key of this wallet: check the words or the passphrase**, in the error
tone.

## Check

| Control | What it does |
|---|---|
| **Descriptor** | the kind, the keys and the checksum; a press shows the wallet's code |
| **The wallet's first addresses** | **Receive 0/0**, **Receive 0/1**, **Change 1/0**, to compare with the wallet sheet or a watch-only wallet |
| **It matches** | goes on to **Done** |

Closed, it reads the first address, shortened.

## Done

| Control | What it does |
|---|---|
| **Spend from this wallet** | as on the wallet's card: "Sign the PSBT from the stick" with a PSBT for it in Files |
| **Back up this wallet** | opens its backup |
| **Another wallet from these seeds** | opens [Make a new wallet](make-a-wallet.md) filled from this one |
| **Wallets** | opens the wallet's card |

A restored key is not held for a backup: it signs at once.

## On a narrow window

The cards fill the window; the description's sources stand one under
the other, each name over its buttons.

## Why

- [Backups](../../learn/04-backups.md)
- [Inheritance](../../learn/12-inheritance.md)
- [Coordinator files](../../learn/19-coordinators.md)
