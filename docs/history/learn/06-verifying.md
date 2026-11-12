# Verifying

**Kind:** History
**Status:** The Learn pages' text as it was edited before the in-app guide (`docs/guide/`) replaced them

## Why you must check addresses

Every address you pay to, and every address you receive on, reaches you through software on an internet-connected machine: your coordinator. Software that has been tampered with can show you one address and send the funds to another. Malware does exactly this, including by swapping an address as you copy it.

An address shown on Faraday was derived on Faraday, from your key, on a device with no network. It is the one thing in the process you can trust.

## What a bad coordinator can do

The coordinator builds the transaction. It chooses the inputs, the outputs, the amounts, and the change address. A coordinator that lies, or that is simply misconfigured, can point your change at an address that is not yours. Change is often most of the money in a transaction, and the loss looks like an ordinary payment until it is too late.

The device therefore trusts nothing it is sent. A wallet is shown in full before it is accepted, and you accept it on this screen. A transaction is shown with every output and its amount, and an output counts as your change only when the device derives its address from a wallet you accepted. An output it cannot account for is shown as a payment.

## Receiving

Before you give out a receiving address, show the same address on Faraday and compare it character by character with the one on your coordinator. Check the first group, the last group, and several groups in the middle. Attackers make addresses that match at the start and the end.

You can also check an address the other way round: Tools → Check an address scans it and searches the loaded wallets for it. What it does not tell you is where the address came from: the screen you copied it from is still the screen to compare against.

## Sending

Before you sign, Faraday shows what the transaction sends, the change and the fee. Read every output. Confirm that the destination address and amount match what you intended. Change that Faraday rebuilt from the wallet's own keys is marked Verified; an output that Faraday cannot confirm as yours is not yours, and if it cannot derive a change output, treat that output as a payment to somebody else.

Change is the rest of the inputs coming back to the same wallet, not money leaving. Signing is the one step that cannot be taken back. If the transaction is not what you meant to do (a different amount, an address you do not recognise, or a warning of any kind), do not sign. Declining costs nothing.

The Nonce check lists every signature the transaction already carries, with the key it is under and whether it verifies. Compare every address and amount in full with what your coordinator shows. If anything differs, do not sign.

## Checking a signer

Tools → Check a signer compares two PSBTs: the one sent to a signing device and the one that came back. It compares the unsigned transaction (version, locktime, every input and output), every signature and final field, the key origins and scripts, any other field, unknown and proprietary pairs included, and the bytes of the returned file.

A clean verdict, "Only signatures were added", says that the transaction is the one sent and that the only differences are signatures added under keys the sent PSBT names, which verify over this transaction, and inputs finalized from such signatures. "Only nonces and signatures were added" is the same for a MuSig2 PSBT, which also gains public nonces. A clean verdict says nothing more: it does not say the signatures are the ones you wanted, or that the coordinator's transaction was right. Read the transaction in Spend before signing.

Anything else is a finding, with its place (Transaction, an input or an output) and its level: Blocked, the transaction is not the one sent or a signature commits to no output; Danger, a field added or changed; Caution, something removed or not checked. A signature that cannot be checked is a finding, never a pass. A signature under a key the sent PSBT does not name is a finding even when it verifies. With no input in common the verdict is "Not the same transaction".

Stated lists what the check saw and accepted without calling it a finding: a MuSig2 nonce added, an input finalized, a field a signer adds that Faraday checks in full against the transaction.

The Nonces recomputed line needs the seed of the key that signed: a loaded key signs the input again here, and the line counts the signatures it reproduces ("1 of 2"). A key not loaded is named ("None: 0f1a2b3c not loaded"); its signature's nonce is not recomputed, which is not the same as cleared. A nonce chosen by a signer can hide its key, and only the seed shows it. Load the key (the Load button on the result) and the check runs again.

## Comparing two PSBTs and the fee bump

Tools → Compare two PSBTs lists how two PSBTs differ, without judging them: the fee and fee rate of each, the inputs and outputs they share or have alone, and what differs in the version, the locktime, the inputs' sequences and the outputs' amounts. It reads only what moves money and what signs it. Where the two are one unsigned transaction, it offers Check a signer, which compares every field.

When the two are an original and its replacement, Compare also gives the fee bump result. "Only the fee is raised" says that the replacement spends the same inputs, pays the same amounts to the same addresses, and takes the extra fee from the person's own money: from change, or from an input added from a loaded wallet. It says nothing about a payment added or removed: "More than the fee changed" lists what moved. "Cannot tell whether only the fee is raised" is the answer where an amount is not stated, or where change belongs to a key that is not loaded; load the key and the result is worked out again. Faraday's rules are stricter than the network's: the fee and the fee rate must both be higher.

The original must be your own copy: the file you made or signed, kept in Files. A coordinator that shows you a replacement can show you a different original too, and "only the fee is raised" then compares the replacement to what the coordinator wants you to believe you sent.

The result does not stop signing. A person may mean to add a payment to a stuck transaction, or to cancel one by sending it back to themselves. The review of the replacement, its warnings and its holds, stands as it is for any transaction; the fee bump result states what it changes from the original.

## Warning cards

When Faraday cannot confirm something, the review shows a warning under the transaction. A danger is in red: an input whose value cannot be checked, a signature type that does not cover the outputs, a fee out of all proportion to the amount. Change it cannot rebuild from a loaded wallet is shown as a payment, under Sends. A danger stops the review: Faraday does not sign it. Only a fee this high has a way past, a hold, for a child paying for its parent, a large batch or a consolidation.

A warning you cannot explain is a reason not to sign. Go back to your coordinator and find out why.
