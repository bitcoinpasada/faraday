# Coordinator files

## What a coordinator is

A coordinator is the wallet software on your online machine: Sparrow, Liana, Nunchuk, Specter, Bitcoin Core and others. It holds no keys. It watches addresses, shows balances and history, picks which coins to spend, sets the fee, builds the transaction, and broadcasts it once it is signed.

Faraday is the other half: it holds the keys and signs. Everything that crosses between the two is one of a small number of files, by QR code or on a stick (on the Pi, the SD card). This page names them.

## What the coordinator sends

A descriptor is one line of text describing a whole wallet: the keys, the script type and the derivation paths, with an eight-character checksum. It is the most portable of these, and the one to keep with your backup.

A wallet policy is the same wallet split into a template and a list of keys, which is BIP-388's form. Some coordinators and signers use it because the template is short enough to read on a small screen and compare by eye.

A multisig configuration file is a plain text list of the keys and the threshold, in the shape Coldcard introduced and several coordinators now export. It carries the same facts as a descriptor in a different layout.

A BSMS record does the same job with a setup ceremony around it, below.

A PSBT is the transaction itself, partially signed: the inputs, the outputs, the amounts, and the key information needed to sign, with room for each signer to add a signature.

## What it gets back

An xpub, with the fingerprint and derivation path that say where it came from. That is what a coordinator needs before it can watch anything, and it is all you should ever send.

A signed PSBT: the same transaction with your signature in it. The coordinator collects the signatures it needs, finalises the transaction and broadcasts it.

Nothing else leaves. No seed words, no private key, and no passphrase.

## BSMS

Bitcoin Secure Multisig Setup, BIP-129, is a procedure for building a multisig wallet without trusting the coordinator to report the keys honestly. Each signer produces its key record, the coordinator assembles them into a descriptor record, and the record goes back to every signer to be confirmed and stored before any coins are sent.

It can be run with a shared secret token, so that a coordinator that swapped a key in would produce a record the signers cannot confirm. The point of the ceremony is that every signer ends up holding the same wallet description, checked on its own screen, rather than trusting one machine's word for it.

Faraday writes a key record for each multisig key held here, under a wallet's public files, For the cosigners: the signed xpub record of BIP 129. The record is signed by the account key itself, and it leaves as text and as a code.

An encrypted record is a different file, hex rather than text, and Faraday writes none and reads none. A record with a token in it is still plain text, so carry it the way you would carry any other file the coordinator must not be able to change: by QR code or memory card, not through the coordinator.

Faraday reads a descriptor record wherever it reads a wallet: Scan, Files, and Restore an existing wallet. It checks the wallet's first address against the one the record states, and shows the quorum, the derivation paths the record restricts the wallet to, and which keys are loaded here. A multisig wallet's public files write the descriptor record back out, as text and as a code. Encrypted records are refused by name.

A BSMS record is worth keeping for the same reason a descriptor is: it is what recovery needs when the coordinator is gone.

## Bitcoin Core

Bitcoin Core is not a coordinator with a wallet screen of its own, but it watches a wallet and builds transactions for it once it has been told the descriptor. A wallet's public files offer the Bitcoin Core import format, which is the JSON that `importdescriptors` takes: the wallet's descriptor with its checksum, marked active, over the first thousand addresses of each chain. Bitcoin Core 29 or later takes it as written; an older Core takes the file under More options, Bitcoin Core before 29.

The file asks Core to scan the whole chain from the first block, which finds coins the wallet already holds and takes a while.

Save the file, carry it to the machine running Core, and hand it over with `bitcoin-cli -rpcwallet=<name> importdescriptors "$(cat <file>)"` on a wallet created with `createwallet` and private keys disabled. Core keeps the private keys nowhere: this file is the public wallet and nothing else.

## Why the device checks anyway

The coordinator is on an internet-connected machine, which is the machine most likely to be compromised. A coordinator that has been tampered with can show you one address and build a transaction paying another. Nothing the device receives is trusted for that reason.

So the device verifies. A wallet is shown to you in full before it is accepted, keys and threshold and script type, and you accept it on this screen rather than on the coordinator's. A transaction is shown with every output and its amount, and every output the device believes is your change is verified by deriving that address from a wallet you accepted. An output the device cannot account for is shown as a payment.

That is the whole reason the two programs are separate. Check the address on this screen against the one you meant to pay, and check it on the coordinator's screen too, and a compromise of either machine alone cannot move your coins.
