# Make a new wallet

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Create`
**Key:** create

Make a new wallet is one card per step, from the kind of wallet to its first addresses. The wallet is made when you press **Make the wallet** on the Keys card, and **Back up wallet** then opens its backup.

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

## Use vaults

**Key:** create.where

Choose whether vaults are part of this backup. A vault keeps a further copy of the keys, sealed under a passphrase.

**Yes** adds the cards Sticks and vaults, Bring in the sticks and Vaults before Keys. **No** goes straight to Keys. Vaults you already have on sticks are brought in here, not later.

## Sticks and vaults

**Key:** create.how-many

Set how many vaults the backup uses and which stick holds which vault.

**Vaults** and **Sticks** are steppers. Each vault is placed on a stick, and one stick may hold several vaults. **Continue** opens the next card.

## Bring in the sticks

**Key:** create.sticks

The vaults are written to their sticks during the backup, so attach each stick that holds a vault here, one at a time. The keys are made only after the stick is removed.

## Vaults

**Key:** create.vaults

Each vault is made or unlocked before the keys exist, so that each new key can be saved into it. Unlocking a vault needs its passphrase; a new vault is made as in Make a new vault.

## Keys

**Key:** create.keys

One row per key, **Key 1**, **Key 2** and so on. Fill each with one of the choices below; **Make the wallet** is offered once every row is filled.

**Make a new key** makes a key for the row. A key already loaded, or a key in an open vault, can fill it too. **Scan its xpub**, or a cosigner's xpub file, fills a row of a multisig with a key from another signer. **Someone else adds it later** leaves the row waiting for that file. **Add a passphrase** adds one to a key from BIP-39 words. Hardware keyboard keys are read as US; **Lock in** then waits for the eye. **Clear** empties the row. **Show xpub QR** shows the key's xpub for the cosigners.

## Check

**Key:** create.check

Shows what was made, for you to compare with your watch-only software.

**Descriptor** gives the kind, the keys and the checksum. **Path** gives the derivation path. **The wallet's first addresses** are its first receive and change addresses; while a key made here has no backup, **Back up before you receive** stands in their place. **It matches** marks the wallet as checked, and **Back up wallet** then opens its backup.
