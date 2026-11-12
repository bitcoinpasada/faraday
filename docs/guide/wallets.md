# Wallets

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Wallets`
**Key:** wallets

The wallets loaded in this session and the card of the one chosen. On a wide panel the list stands beside the card; on a small one each is its own page.

## Transaction bar

**Key:** wallets.psbt

While a wallet is loaded, a bar above the list names the transaction file in hand. **Signing** with **Continue** returns to a spend in progress; a PSBT file in Files, shown as **in Files**, offers **Review**. The file offered is the one with the fewest signatures.

## The list

**Key:** wallets.list

One row per wallet with its name and shape. A wallet this computer cannot spend from adds **watch only** or the count of keys here, as in **1 of 2 here**, and a spendable one is marked **Can spend**. **Does not match its key** and **Another key can spend** show in the error tone.

With none loaded the card reads **No wallet loaded**. **Add a key** opens a key's entry; with a wallet short of keys chosen it reads as scanning a key for that wallet instead.

## Keys without a wallet

**Key:** wallets.keys

Keys loaded that no wallet uses are listed by fingerprint. With one wallet or more, **Make a wallet from this key** stands under each. With no wallet, the card offers the same, and **Add another key** and **Restore an existing wallet**; with a loose key absent, **Restore an existing wallet** stands in the list.

## The card

**Key:** wallets.card

The card gives the wallet's name with **Rename**, a tag for where it came from, its shape and its key path. Under it a line gives the backup, pressed to open **Backups**: it reads **Not backed up** in the warning tone, or the places held.

**Rename** edits the name in place; Enter keeps it and Esc leaves it. **New wallet from this one** opens Make a new wallet filled from this wallet's kind, quorum and keys, for the kinds that flow makes.

## Actions

**Key:** wallets.actions

The first button starts a spend, **Spend from this wallet**, or **Sign the PSBT from the** stick when a PSBT is in hand, **Carry on the spend** while one runs, or **Finish the backup first** while a key made here has none. **Show wallet QR** shows the wallet for watch-only software. **Back up** opens its backup, and reads **Continue the backup** when one is part way. **Sign a message** appears for a single-key wallet whose key is here. **Remove from session** drops the wallet and keeps its files.

## Signatures

**Key:** wallets.signatures

A line reads, for example, 1 of 2 signatures possible here: the keys here against the quorum, with a pip for each. **Can spend** marks a quorum present. Short of it, **Scan a key** takes another key.

Below, **Spend paths** lists, for a wallet with a relative wait, each path's keys and when it can spend, and **Move each coin within** the days before the shortest wait ends.

## At a glance

**Key:** wallets.glance

**At a glance** draws the wallet, its keys and its backup as a chart in three rows. A line follows for each key here that was loaded from codex32 strings entered with a correction. When the quorum is not here, a further line names the vault that may hold the keys, and **Unlock** opens a locked one.

## First receive address

**Key:** wallets.receive

**First receive address · 0/0** shows the wallet's first address; pressing it shows its code. While a key of the wallet made here is not backed up, **Back up before you receive** stands in its place, in the warning tone, and the address is not shown.

## Silent payments wallet

**Key:** wallets.silent

A silent-payments wallet shows **Key** with its fingerprint and **Here**, **Not here** or **Does not match**, with **Load from vault** or **Add its key** when absent. **Address** gives the wallet's address, which stays unchecked until its key is here; **Labels handed out** counts the labels. **Address, labels and payments**, **Show wallet QR** and **Remove from session** act on it.
