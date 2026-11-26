# Wallets

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Wallets`
**Key:** wallets
**Words:** watch-only, quorum, multisig, fingerprint, session, psbt

Wallets lists the wallets loaded in this session and shows the card of the one you choose. On a wide panel the list stands beside the card; on a small one each is its own page.

## Transaction bar

**Key:** wallets.psbt

While a wallet is loaded, a bar above the list names the transaction file in hand. **Signing** with **Continue** returns to a spend in progress. A PSBT file in Files, shown as **in Files**, offers **Sign**. When there are several, the file offered is the one with the fewest signatures.

## The list

**Key:** wallets.list

One row per wallet, with its name and shape. A wallet this computer cannot spend from adds **watch only**; the card says how many signatures are possible here. One it can spend from is marked **Can spend**. **Does not match its key** and **Another key can spend** show in the error tone.

With no wallet loaded the card reads **No wallet loaded**. **Add a key** opens a key's entry; when the chosen wallet is short of keys, it reads as scanning a key for that wallet instead.

## Keys without a wallet

**Key:** wallets.keys

Loaded keys that no wallet uses are listed by fingerprint. With one wallet or more, **Make a wallet from this key** stands under each. With no wallet, the card offers the same, along with **Start a multisig wallet with this key** and **Restore an existing wallet**; with no loose key either, **Restore an existing wallet** stands in the list. On a wide window a press on a key's row opens its details beside the list, with **Explore** and the same ways on; Back, Escape or the page's tab returns the list.

## The card

**Key:** wallets.card

The card gives the wallet's name with **Rename**, a tag for where it came from, its shape and its key path. The line under it gives the backup, and pressing it opens **Backups**: it reads **Not backed up** in the warning tone, or names the places that hold it.

**Rename** edits the name in place; Enter keeps the change and Esc drops it. **New wallet from this one** opens Make a new wallet filled in from this wallet's kind, quorum and keys, for the kinds that flow makes.

## Actions

**Key:** wallets.actions

The first button starts a spend. It reads **Spend from this wallet**, or **Sign the PSBT from the** stick when a PSBT is in hand, **Carry on the spend** while one is under way, or **Finish the backup first** while a key made here has no backup. **Show wallet QR** shows the wallet for watch-only software. **Back up** opens its backup, and reads **Continue the backup** when one is part done. **Sign a message** appears for a single-key wallet whose key is here. **Check an address** opens that tool, and **Explore its keys** opens Explore on a key of the wallet that is loaded. **Remove from session** drops the wallet and keeps its files.

## Signatures

**Key:** wallets.signatures

A line reads, for example, 1 of 2 signatures possible here: the keys here against the quorum, with a pip for each. **Can spend** marks a quorum that is present. When it is not, **Scan a key** takes another key.

Below it, for a wallet with a relative wait, **Spend paths** lists each path's keys and when it can spend, and **Move each coin within** gives the days left before the shortest wait ends.

## At a glance

**Key:** wallets.glance

**At a glance** draws the wallet, its keys and its backup as a chart in three rows. A line follows for each key here that was loaded from codex32 strings entered with a correction. When the quorum is not here, a further line names the vault that may hold the keys, and **Unlock** opens a locked one.

## First receive address

**Key:** wallets.receive

A key that is not here may be in a vault on a stick, a paper backup or a SeedQR; a locked vault is named with **Unlock**.

**First receive address · 0/0** shows the wallet's first address; press it to see its code. While a key of the wallet that was made here has no backup, **Back up before you receive** stands in its place, in the warning tone, and the address is not shown.

## Silent payments wallet

**Key:** wallets.silent

A silent-payments wallet shows **Key** with its fingerprint and **Here**, **Not here** or **Does not match**, with **Load from vault** or **Add its key** when the key is absent. **Address** gives the wallet's address, which stays unchecked until its key is here; **Labels handed out** counts the labels. **Address, labels and payments**, **Show wallet QR** and **Remove from session** act on it.
