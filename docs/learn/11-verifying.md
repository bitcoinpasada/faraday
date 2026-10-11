# Verifying

## Why you must check addresses

Every address you pay to, and every address you receive on, reaches you through software on an internet-connected machine: your coordinator. Software that has been tampered with can show you one address and send the funds to another. Malware does exactly this.

An address shown on OpenSigner was derived on OpenSigner, from your key, on a device with no network. It is the one thing in the process you can trust.

## What a bad coordinator can do

The coordinator builds the transaction. It chooses the inputs, the outputs, the amounts, and the change address. A coordinator that lies, or that is simply misconfigured, can point your change at an address that is not yours. Change is often most of the money in a transaction, and the loss looks like an ordinary payment until it is too late.

## Receiving

Before you give out a receiving address, show the same address on OpenSigner and compare it character by character with the one on your coordinator. Check the first group, the last group, and several groups in the middle. Attackers make addresses that match at the start and the end.

You can also check an address the other way round: open the wallet on Home, tap "Check an address", then scan the address, paste it or type it. A pasted address is checked against the wallet exactly as a scanned one is, and the answer means the same thing. What it does not tell you is where the address came from: the screen you copied it from is still the screen to compare against.

## Sending

Before you sign, read every output. Confirm that the destination address and amount match what you intended, and that the change output is marked as derived from your key. An output that OpenSigner cannot confirm as yours is not yours.

## Warning cards

When OpenSigner cannot confirm something, it shows a warning card. A red card is a danger: change that cannot be derived from your key, a fee out of all proportion to the amount, an address on the wrong network. A red card stops the flow until you acknowledge it.

DO NOT TAP PAST A WARNING YOU CANNOT EXPLAIN. Go back to your coordinator and find out why.
