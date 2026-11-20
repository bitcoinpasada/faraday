# Spend from a backup

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `SpendBackup`
**Key:** spend-from-backup
**Words:** psbt, descriptor, vault, seal, share, seed

Spend from a backup is one card at a time that takes you from what is in the envelope to a transaction sent and the device locked, with three stops on this computer and three on an online one; nothing moves until the last. The wallet is worked out from what you bring in, and the steps are those of Sign a transaction.

## How to spend bitcoin

**Key:** spend-from-backup.map

The map lists the six stops: what you have, open the wallet here, check the balance, write the payment, sign it here, send it. Each is marked offline or online. **Start** opens the first card.

## What do you have?

**Key:** spend-from-backup.holding

Bring in everything from the envelope here, in any order; each new thing asks only the next question. The card also holds the start from the stick.

**Unlock** with a vault's name opens a vault found in Files, using its passphrase. **Scan the QR codes you have** and **Type the words you have** take seeds, descriptors and split sheets. **Or bring in what is on paper** is offered beside an open vault. **Rebuild the wallet** joins split sheets that add up. **Not sure what you have?** shows what each thing looks like. **Start over** clears what you brought in.

## Check the money is really there

**Key:** spend-from-backup.check

You do this on the online computer, to confirm the backup is the wallet you expect. **Show the wallet as a QR code** gives the wallet's descriptor, which you load into Sparrow; then compare the Receive and Change addresses with those Sparrow shows. A match means two programs read the same backup. **Show more addresses** lists more of them. **Yes, my money shows** goes on.

For words alone, the wallet shown is of the most common kind. **No money shows** makes the next kind. When none shows money, use **Scan more** or **Type a passphrase** to go on. From a wallet's card, this card is Load the wallet in Sparrow: **Show the wallet QR** and Receive address 0, then **It is loaded**.

## Write the payment

**Key:** spend-from-backup.write

You do this on the online computer, in Sparrow's Send tab: enter the address, the amount and a fee, then use Create Transaction, Finalize Transaction for Signing and Show QR. **I have the PSBT** goes on. From a wallet's card, this card is Bring the transaction here.

## Bring the transaction here

**Key:** spend-from-backup.bring

The PSBT comes in by camera or from a stick. **Scan** (**Scan the PSBT** from a wallet's card) reads Sparrow's QR code, and reads an animated code as it loops. From a stick, the PSBT is copied in as a file. PSBTs already in Files are listed, each with **Use this one**. **Read it** opens the transaction. Faraday reads a stick only while no vault or key is open, so lock first if one is in use. Moving every coin to a new wallet shows its address and the Max amount to enter in Sparrow.

## Read the transaction

**Key:** spend-from-backup.transaction

This is the same card as Transaction in Sign a transaction. It shows what the transaction sends, the change, the fee and the warnings, with **Show the table**. A transaction that no loaded wallet matches is read without verified change.

## The transaction

**Key:** spend-from-backup.txid

This is the transaction's id, which you compare after sending. Write down its first and last few characters; Sparrow shows the same id.

## Which way it spends

**Key:** spend-from-backup.path

This card is shown for a wallet with several ways to spend. Each path lists the keys it needs and its wait, and a line says whether the transaction's sequence allows it. **Continue** goes on.

## Nonces

**Key:** spend-from-backup.nonces

This card is shown for an **All keys · MuSig2** wallet. Every key must add a nonce before anyone signs. Add the others' nonces from their copies, or let this device go first and share its own.

## Add the seeds that can sign

**Key:** spend-from-backup.signers

This card is titled Add the seeds that can sign on the paper route and Who signs otherwise. Each key reads **Signs here** or **Not here**. **Load from vault**, **Scan a key** and **Unlock** with a vault's name bring a key here; **Collect a signature** takes a cosigner's. A key from another wallet is refused. **Continue** goes on.

## Sign it

**Key:** spend-from-backup.sign

Signing adds the signature of every key loaded here. If the wallet needs signatures from elsewhere, the next card lists who has still to sign.

## The other signatures

**Key:** spend-from-backup.collect

This card is shown for a wallet of several keys. Scan or copy in each cosigner's signed copy; each signature is checked against its key and this transaction before it counts.

## Send it

**Key:** spend-from-backup.finish

**Show the finished transaction as QR** shows the signed PSBT; Sparrow scans it on the same transaction and broadcasts it. Instead, you can carry the finished transaction in Files on a stick. Whatever sends it should report the id shown earlier; a different id means that what was sent is not what was signed.

## Put everything away

**Key:** spend-from-backup.away

Files still waiting for the stick are listed. **Lock** seals the vault again with its keys inside, and no key is left in memory. **Power off** switches the device off. **Done** ends the spend and goes back to where it was opened from. **Start over** returns to the first card, and asks first once a step of the spend is done. When Faraday signs with a vault open, locking seals a changed copy of the vault to write back, which keeps the record of the amounts signed.
