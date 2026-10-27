# Verifying

## Why you must check addresses

Every address you pay to, and every address you receive on, reaches you through software on an internet-connected machine: your coordinator. Software that has been tampered with can show you one address and send the funds to another. Malware does exactly this, including by swapping an address as you copy it.

An address shown on Faraday was derived on Faraday, from your key, on a device with no network. It is the one thing in the process you can trust.

## What a bad coordinator can do

The coordinator builds the transaction. It chooses the inputs, the outputs, the amounts, and the change address. A coordinator that lies, or that is simply misconfigured, can point your change at an address that is not yours. Change is often most of the money in a transaction, and the loss looks like an ordinary payment until it is too late.

The device therefore trusts nothing it is sent. A wallet is shown in full before it is accepted, and you accept it on this screen. A transaction is shown with every output and its amount, and an output counts as your change only when the device derives its address from a wallet you accepted. An output it cannot account for is shown as a payment.

## Receiving

Before you give out a receiving address, show the same address on Faraday and compare it character by character with the one on your coordinator. Check the first group, the last group, and several groups in the middle. Attackers make addresses that match at the start and the end.

You can also check an address the other way round: Tools › Check an address scans it and searches the loaded wallets for it. What it does not tell you is where the address came from: the screen you copied it from is still the screen to compare against.

## Sending

Before you sign, Faraday shows what the transaction sends, the change and the fee. Read every output. Confirm that the destination address and amount match what you intended. Change that Faraday rebuilt from the wallet's own keys is marked Verified; an output that Faraday cannot confirm as yours is not yours, and if it cannot derive a change output, treat that output as a payment to somebody else.

Change is the rest of the inputs coming back to the same wallet, not money leaving. Signing is the one step that cannot be taken back. If the transaction is not what you meant to do (a different amount, an address you do not recognise, or a warning of any kind), do not sign. Declining costs nothing.

The Nonce check lists every signature the transaction already carries, with the key it is under and whether it verifies. Compare every address and amount in full with what your coordinator shows. If anything differs, do not sign.

## Warning cards

When Faraday cannot confirm something, the review shows a warning under the transaction. A danger is in red: an input whose value cannot be checked, a signature type that does not cover the outputs, a fee out of all proportion to the amount. Change it cannot rebuild from a loaded wallet is shown as a payment, under Sends. A danger stops the review: Faraday does not sign it. Only a fee this high has a way past, a hold, for a child paying for its parent, a large batch or a consolidation.

A warning you cannot explain is a reason not to sign. Go back to your coordinator and find out why.
