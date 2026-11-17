# Make a new wallet

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Create`
**Key:** create

Make a new wallet is one card per step: Kind, Keys, Check and then Back up. The wallet is made when you press **Make the wallet** on the Keys card, and its end card then shows **Back up now**, which opens its backup; new vaults are added on the backup plan.

## Kind

**Key:** create.kind

Choose the kind of wallet: it sets how the wallet locks its coins and what its addresses look like. Choosing a row opens the next card.

**Single key · native SegWit** is one key signing, with bc1q addresses, the kind most wallets make. **Multisig · native SegWit** needs a quorum of several keys. **More kinds** opens the rest. **Single key · Taproot** gives bc1p addresses. **Single key · nested SegWit** and **Multisig · nested SegWit** give 3 addresses, for services that cannot pay bc1. **Single key · legacy** (1 addresses) and **Multisig · legacy** (3 addresses) are for old software only. **Multisig · Taproot** is a quorum on one Taproot script leaf. **All keys · MuSig2** needs every key to sign and appears on chain as one Taproot key. **Start over** forgets a wallet part made.

## Backup

**Key:** create.backup-form

Asked for a single key of any address type: choose how the key is written on paper.

**Words (BIP-39)** is 12 to 24 words in one list. **Shares (codex32)** splits the key into shares: a set number of them rebuild it and fewer reveal nothing of it, so you can keep each in its own place. The Keys card stays closed until you answer this one.

## Quorum

**Key:** create.quorum

For a kind with several keys. A quorum of m keys out of n must sign to spend; **All keys · MuSig2** has no quorum, only the count of keys. Keep the keys apart, on different devices or with different people: together in one place, the quorum protects nothing.

**Signatures needed** and **Keys** are set by steppers. **Path** is **BIP-48** or **BIP-87** for the multisig kinds that are a descriptor, and cannot change once the wallet is made. **Account** is the account the keys are taken at, 0 at first.

## Keys

**Key:** create.keys

One row per key, **Key 1**, **Key 2** and so on. Fill each with one of the choices below; **Make the wallet** is offered once every row is filled.

**Use a vault I already have on a stick** is a row at the head of Keys, off until pressed, shown only before any key exists. On, it brings in that one stick and unlocks its vault for the keys. It has no counts and no **Check it later**.

**Make a new key** makes a key for the row. A key already loaded, or a key in an open vault, can fill it too. **Scan its xpub**, or a cosigner's xpub file, fills a row of a multisig with a key from another signer. **Someone else adds it later** leaves the row waiting for that file. **Add a passphrase** adds one to a key from BIP-39 words. Hardware keyboard keys are read as US; **Lock in** then waits for the eye. **Clear** empties the row. **Show xpub QR** shows the key's xpub for the cosigners.

## Check

**Key:** create.check

Shows what was made, for you to compare with your watch-only software.

**Descriptor** gives the kind, the keys and the checksum. **Path** gives the derivation path. **The wallet's first addresses** are its first receive and change addresses; while a key made here has no backup, **Back up before you receive** stands in their place. **It matches** marks the wallet as checked, and the end card shows **Wallet made**, **Backup** as **not made**, and **Back up now**, which opens its backup, and **Open the wallet**. **Lock** and **Power off** wait for the backup.
