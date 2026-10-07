# Transactions

## Inputs and outputs

A transaction takes coins you control, called inputs, and creates new coins from them, called outputs. Those new coins can be owned by you, or you can transfer ownership to someone else.

## Change

You cannot spend part of an input. If you control a 10,000 sat input and spend 5,000 sats, the whole 10,000 sat input is broken up into two new coins: the 5,000 sat output (sent to someone else) and your change less the miner fee.

Check the change output of every transaction before you sign. If the device cannot derive a change output from your wallet, treat that output as a payment to somebody else.

## Fees

The fee is simply the inputs less the outputs. If you spend 10,000 sats as 5,000 to somebody else plus 4,900 in change to yourself, the fee is 100 sats.

## Setup and PSBT

To set up a transaction for OpenSigner, you need a coordinator that can create Partially-Signed Bitcoin Transactions, or PSBTs. You can transfer the PSBT to OpenSigner via QR code or the SD card.

## Signing

Before you sign, OpenSigner lists every input and output and marks the amounts you control. The Signatures row on the inputs page lists every signature the transaction already carries, with the key it is under and whether it verifies. Read every address and amount in full and compare them with what your coordinator shows. If anything differs, do not sign; go back to the coordinator and find out why.
