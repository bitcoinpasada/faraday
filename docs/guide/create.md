# Make a new wallet

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Create`
**Key:** create
**Words:** key, multisig, quorum, descriptor, xpub, vault, share

Make a new wallet is one card per step: Kind, Keys, Check and then Back up. The wallet is made when you press **Make the wallet** on the Keys card, and its end card then shows **Back up now**, which opens its backup; new vaults are added on the backup plan.

## Kind

**Key:** create.kind

Choose the kind of wallet: it sets how the wallet locks its coins and what its addresses look like. Choosing a row opens the next card.

**Single key** is ticked when the page opens: one key signing, with bc1q addresses, the kind most wallets make; pressing it takes it and opens the next card. **Multisig** needs a quorum of several keys. **More kinds** opens the rest. **Single key · Taproot** gives bc1p addresses. **Single key · nested SegWit** and **Multisig · nested SegWit** give 3 addresses, for services that cannot pay bc1. **Single key · legacy** (1 addresses) and **Multisig · legacy** (3 addresses) are for old software only. **Multisig · Taproot** is a quorum on one Taproot script leaf. **All keys · MuSig2** needs every key to sign and appears on chain as one Taproot key. **Start over** forgets a wallet part made.

## Backup

**Key:** create.backup-form

Asked for a single key of any address type: choose how the key is written on paper.

**Words (BIP-39)** is ticked when the card opens: 12 to 24 words in one list. **More ways** shows **Shares (codex32)**, which splits the key into shares: a set number of them rebuild it and fewer reveal nothing of it, so you can keep each in its own place. The key made on the Keys card is made the way chosen here, and its Start card is already answered, with **Change**.

## Quorum

**Key:** create.quorum

For a kind with several keys. A quorum of m keys out of n must sign to spend; **All keys · MuSig2** has no quorum, only the count of keys. Keep the keys apart, on different devices or with different people: together in one place, the quorum protects nothing.

**Signatures needed** and **Keys** are set by steppers. **Path** is **BIP-48** or **BIP-87** for the multisig kinds that are a descriptor, and cannot change once the wallet is made. **Account** is the account the keys are taken at, 0 at first.

## Keys

**Key:** create.keys

One row per key. **Make the wallet** is offered once every row is filled.

**Use a vault I already have on a stick** brings in its vault's keys.

On a single key backed up as words, and on a multisig's empty row (**New words**), the row makes its key in place: pick **12 words** or **24 words**, roll dice (**Typed** or **Buttons**, **Undo**, **Clear**) to the count, then **Make the words**. **Show words** reveals them, **Make again** drops them, **Add a passphrase** adds one; hardware keys read as US; **Lock in** waits for the eye. **Make the wallet** or **Lock in** takes the key. **Other ways** opens **Make a new key**. Leaving the card drops the rolls and words.

**Scan its xpub** fills a multisig row with another signer's key. **Someone else adds it later** leaves the row waiting. **Clear** empties the row. **Show xpub QR** shows the key's xpub.

## Check

**Key:** create.check

Shows what was made, for you to compare with your watch-only software.

**Descriptor** gives the kind, the keys and the checksum. **Path** gives the derivation path. **The wallet's first addresses** are its first receive and change addresses; while a key made here has no backup, **Back up before you receive** stands in their place. **It matches** marks the wallet as checked and opens its backup in the same press. For a wallet made to move another's money, Check stays open, with **Back up wallet** and the spend of the old wallet's coins.
