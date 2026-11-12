# Restore an existing wallet

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Restore`
**Key:** restore

One card per step, from the kind of wallet to what to do with it once it is back. The wallet is made at **Make the wallet** on the Keys card from the seeds, and its first addresses are compared on Check.

## Kind

**Key:** restore.kind

The kind is the one written on the backup. A wallet description read here sets the kind, the quorum and every key, and the flow goes on from it with **Continue**.

**I have the wallet description** offers **Scan it**, and **From Files** when a wallet file is in Files. The kinds are the rows of Make a new wallet; choosing one opens the next card, and **More kinds** opens the rest.

## Quorum

**Key:** restore.quorum

Asked for a kind with several keys: how many keys the wallet has and how many must sign, as written on the backup, often 2 of 3.

**Signatures** and **Keys** are set by sliders. **All keys · MuSig2** has no quorum, only the count of keys. **Continue** opens the next card.

## Description

**Key:** restore.description

The description names every key and the wallet's rules. Without one, the wallet is made from the seeds and the cosigners' xpubs at the kind's standard path.

**I have the seed** makes a single-key wallet from its seed alone. **I do not have it** does the same for a multisig. **Scan descriptor** reads a description from the camera. A button named From and a file's name reads a wallet file from Files. **Copy files in** brings in the files of a stick, which is not read while a key is loaded. A button named Unlock and a vault's name opens a locked vault; a wallet in an open vault is offered by name.

**Shares of a split backup** lists the share files in Files; each is ticked, and **Rebuild the wallet** stands out once the ticked shares are enough to rebuild the description.

## Use vaults

**Key:** restore.where

The choice is whether vaults are part of this backup. A vault keeps a further copy of the keys, sealed under a passphrase.

**Yes** adds the card Sticks and vaults, then the vault setup, before Keys. **No** goes to Keys. Vaults already on sticks are brought in here, not later. The answer cannot change once the wallet is made.

## Sticks and vaults

**Key:** restore.how-many

Sets how many vaults the backup uses and which stick holds which vault. Keys stays closed until this card and the vault setup it opens are done.

**Vaults** and **Sticks** are steppers. **Continue** opens the setup, which returns to Keys.

## Keys

**Key:** restore.seeds

One slot per key. A filled slot reads **Here**, or Key 1 of the wallet and on for a key of a description; an empty one reads **Empty**. A seed that is not one of the description's keys is refused, so a wrong word or passphrase shows here.

**Type the words** and **Scan a SeedQR** fill the first empty slot; a passphrase is asked after the seed. A seed in an open vault, or one already loaded, is offered by name. **Its xpub** takes a cosigner's xpub in place of a seed. **Not here** leaves a key of the description for another signer, and **Change** takes it back. **Make the wallet** appears once every slot is filled.

## Check

**Key:** restore.check

Shows the wallet made, to compare with the backup sheet or the software that holds the history. **Make the wallet first** stands in until the wallet exists.

The descriptor and the first addresses are shown. **It matches** closes the card.

## Done

**Key:** restore.done

The wallet is in this session until memory is locked. The card offers what to do with it.

A spend button opens the wallet's spend, or its backup first when a key still lacks one. **Back up this wallet** opens its backup. **Another wallet from these seeds** makes a wallet of another kind from the same keys. **Wallets** opens the wallet.
