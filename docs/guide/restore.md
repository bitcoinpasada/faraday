# Restore an existing wallet

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Restore`
**Key:** restore
**Words:** seed, descriptor, quorum, xpub, share, vault, multisig

Restore rebuilds a wallet from its backup, one card per step, from the kind of wallet to what to do with it once it is back. The wallet is made from the seeds when you press **Make the wallet** on the Keys card, and you compare its first addresses on Check.

## Kind

**Key:** restore.kind

Choose the kind written on the backup. If you have the wallet's description, read it here instead: it sets the kind, the quorum and every key, and Keys opens next.

**I have the descriptor** offers **Scan it**, and **From Files** when a wallet file is in Files (a button named From and a file's name for each one when there are several). This is the only card that reads a description. The kinds are the same rows as in Make a new wallet; choosing one opens the next card, and **More kinds** opens the rest.

## Quorum

**Key:** restore.quorum

Asked for a kind with several keys: how many keys the wallet has and how many must sign, as written on the backup, often 2 of 3.

**Signatures** and **Keys** are set by sliders. **All keys · MuSig2** has no quorum, only the count of keys. **Continue** opens the next card.

## Description

**Key:** restore.description

The description names every key and the wallet's rules. Without one, the wallet is made from the seeds and the cosigners' xpubs at the kind's standard path; the Keys card asks for the account.

**I have the seed** makes a single-key wallet from its seed alone. **I do not have it** does the same for a multisig. **Copy files in** brings in a stick's files; a stick is not read while a key is loaded. A button named Unlock and a vault's name opens a locked vault, and a wallet in an open vault is offered by name.

**Shares of a split backup** lists the share files in Files. Tick the ones to use; **Rebuild the wallet** stands out once they are enough to rebuild the description.

## Keys

**Key:** restore.seeds

One row per key. A filled row reads **Here**, or Key 1 of the wallet and so on for a key of a description; an empty one reads **Empty**. A seed that is not one of the description's keys is refused.

**Type the words** and **Scan a SeedQR** fill the first empty row; the passphrase is asked for after the seed. **Use a vault I already have on a stick** is off until pressed, and shown only before any seed is typed; it brings in that stick and unlocks its vault. A seed in an open vault, or one already loaded, is offered by name. **Its xpub** takes a cosigner's xpub in place of a seed. **Not here** leaves a key of the description for another signer, and **Change** takes that back. **Make the wallet** appears once every row is filled.

## Check

**Key:** restore.check

Shows the wallet that was made, for you to compare with the backup sheet or the software that holds its history. **Make the wallet first** stands in until the wallet exists.

The descriptor and the first addresses are shown. **It matches** closes the card. A wallet made from the seeds alone also says the kind, account and path it assumed, and the button reads **It matches my record**, beside **I have nothing to compare**.

## Done

**Key:** restore.done

The wallet stays in this session until memory is locked. The card offers what to do with it next.

The restore ends on the same end card as Make a new wallet and Back up: **Wallet restored** with the wallet's name, its kind line and where it was restored from (a wallet made from a loaded key reads **Wallet made**, with no source line), and the Backup line reads "not recorded here". **Done** is the filled button, back to where Restore was opened from; **Show wallet QR** sits beside it, for the watch-only app. A spend button opens the wallet's spend, or its backup first when a key still has none. **Back up this wallet** opens its backup while it has none. **Open the wallet** opens its card. **Another wallet from these seeds** makes a wallet of another kind from the same keys. **Lock** and **Power off** end the visit.
