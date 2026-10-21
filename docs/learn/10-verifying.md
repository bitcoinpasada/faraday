# Verifying

## Why you must check addresses

Every address you pay to, and every address you receive on, reaches you through software on an internet-connected machine: your coordinator. Software that has been tampered with can show you one address and send the funds to another. Malware does exactly this.

An address shown on Faraday was derived on Faraday, from your key, on a device with no network. It is the one thing in the process you can trust.

## What a bad coordinator can do

The coordinator builds the transaction. It chooses the inputs, the outputs, the amounts, and the change address. A coordinator that lies, or that is simply misconfigured, can point your change at an address that is not yours. Change is often most of the money in a transaction, and the loss looks like an ordinary payment until it is too late.

## Receiving

Before you give out a receiving address, show the same address on Faraday and compare it character by character with the one on your coordinator. Check the first group, the last group, and several groups in the middle. Attackers make addresses that match at the start and the end.

You can also check an address the other way round: Tools › Check an address scans it and searches the loaded wallets for it. What it does not tell you is where the address came from: the screen you copied it from is still the screen to compare against.

## Sending

Before you sign, read every output. Confirm that the destination address and amount match what you intended, and that the change output is marked as derived from your key. An output that Faraday cannot confirm as yours is not yours.

## Warning cards

When Faraday cannot confirm something, the review shows a warning under the transaction. A danger is in red: an input whose value cannot be checked, a signature type that does not cover the outputs, a fee out of all proportion to the amount. Change it cannot rebuild from a loaded wallet is marked Unverified.

A warning you cannot explain is a reason not to sign. Go back to your coordinator and find out why.
