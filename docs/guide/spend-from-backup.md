# Spend from a backup

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `SpendBackup`
**Key:** spend-from-backup

One column of cards from what is in the envelope to a transaction sent and the device locked, three stops on this computer and three on an online one; nothing moves until the last. The wallet is worked out from what comes in, and the steps of a spend are the ones of Spend.

## How to spend bitcoin

**Key:** spend-from-backup.map

The map of the six stops: what you have, open the wallet here, check the balance, write the payment, sign it here, send it. Each is marked offline or online. **Start** opens the first card.

## What do you have?

**Key:** spend-from-backup.holding

Everything in the envelope goes in here, in any order, and each new thing asks only the next question. The card also holds the start from the stick.

**Unlock** with a vault's name opens a vault found in Files, with its passphrase. **Scan the QR codes you have** and **Type the words you have** take seeds, descriptions and split sheets. **Or bring in what is on paper** is offered beside an open vault. **Rebuild the wallet** joins split sheets that add up. **Not sure what you have?** shows what each thing looks like. **Start over** clears what came in.

## Check the money is really there

**Key:** spend-from-backup.check

Done on the online computer. **Show the wallet as a QR code** gives the wallet's description, to load into Sparrow, and the Receive and Change addresses are compared with those Sparrow shows. A match means two programs read the same backup. **Show more addresses** lists more of them. **Yes, my money shows** goes on.

For words alone, the wallet shown is of the most common kind. **No money shows** makes the next kind. When none shows money, **Scan more** or **Type a passphrase** are the ways on. From a wallet's card, the card is Load the wallet in Sparrow: **Show the wallet QR** and Receive address 0, then **It is loaded**.

## Write the payment

**Key:** spend-from-backup.write

Done on the online computer, in Sparrow's Send tab: the address, the amount and a fee, then Create Transaction, Finalize Transaction for Signing and Show QR. **I have the PSBT** goes on. From a wallet's card, this card is Bring the transaction here.

## Bring the transaction here

**Key:** spend-from-backup.bring

The PSBT comes in by camera or from a stick. **Scan** (**Scan the PSBT** from a wallet's card) reads Sparrow's QR code, an animation being read as it loops. The stick copies it in as a file. PSBTs already in Files are listed, each with **Use this one**. **Read it** opens the transaction. Faraday reads a stick only while no vault or key is open, so one in use locks first. Moving every coin to a new wallet shows its address and the Max amount to enter in Sparrow.

## Read the transaction

**Key:** spend-from-backup.transaction

The same card as Transaction in Spend: what the transaction sends, the change, the fee and the warnings, with **Show the table**. A transaction no loaded wallet matches is read without verified change.

## The transaction

**Key:** spend-from-backup.txid

The transaction's id, to compare after sending. Write down the first and last few characters; Sparrow shows the same id.

## Which way it spends

**Key:** spend-from-backup.path

Shown for a wallet with several ways to spend. Each path lists the keys it needs and its wait. A line says whether the transaction's sequence allows it. **Continue** goes on.

## Nonces

**Key:** spend-from-backup.nonces

Shown for an **All keys · MuSig2** wallet. Every key adds a nonce before anyone signs; the others' are added from their copies, or this device goes first and shares its own.

## Add the seeds that can sign

**Key:** spend-from-backup.signers

Shown on the paper route as Add the seeds that can sign, otherwise as Who signs. Each key reads **Signs here** or **Not here**. **Load from vault**, **Scan a key** and **Unlock** with a vault's name bring a key here; **Collect a signature** takes a cosigner's. A key from another wallet is refused. **Continue** goes on.

## Sign it

**Key:** spend-from-backup.sign

Signing adds the signature of every key loaded here. A wallet that needs signatures from elsewhere goes on to the next card, which lists who has still to sign.

## The other signatures

**Key:** spend-from-backup.collect

Shown for a wallet of several keys. Each cosigner's signed copy is scanned or copied in, and each signature is checked against its key and this transaction before it counts.

## Send it

**Key:** spend-from-backup.finish

**Show the finished transaction as QR** shows the signed PSBT, which Sparrow scans on the same transaction and broadcasts. The finished transaction, in Files, can instead be carried on a stick. Whatever sends it should report the id shown earlier; a different id means what was sent is not what was signed.

## Put everything away

**Key:** spend-from-backup.away

Files still waiting for the stick are listed. **Lock** seals the vault again with its keys inside, and none is left in memory. **Power off** switches the device off. **Start over** returns to the first card. When Faraday signs with a vault open, locking seals a changed copy of the vault to write back, which keeps the record of the amounts signed.
