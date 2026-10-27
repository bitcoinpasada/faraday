# Coordinator files

## What a coordinator is

A coordinator is the wallet software on your online machine: Sparrow, Liana, Nunchuk, Specter, Bitcoin Core and others. It holds no keys. It watches addresses, shows balances and history, picks which coins to spend, sets the fee, builds the transaction as a PSBT, and broadcasts it once it is signed.

Faraday is the other half: it holds the keys and signs. Everything that crosses between the two is one of a small number of files, by QR code or on a stick (on the Pi, the SD card). This page names them.

## What the coordinator sends

A descriptor is one line of text describing a whole wallet: the keys, the script type and the derivation paths, with an eight-character checksum. It is the most portable of these, and the one to keep with your backup.

A wallet policy is the same wallet split into a template and a list of keys, which is BIP-388's form. A multisig configuration file is a plain text list of the keys and the threshold, in the shape Coldcard introduced. Both carry the same facts as a descriptor in a different layout. A BSMS record does the same job with a setup ceremony around it, below.

A PSBT, a Partially Signed Bitcoin Transaction, is the transaction itself: the inputs, the outputs, the amounts, and the key information needed to sign, with room for each signer to add a signature.

## What it gets back

An xpub, with the fingerprint and derivation path that say where it came from. That is what a coordinator needs before it can watch anything, and it is all you should ever send.

A signed PSBT: the same transaction with your signature in it. The coordinator collects the signatures it needs, finalises the transaction and broadcasts it.

Nothing else leaves. No seed words, no private key, and no passphrase.

## BSMS

Bitcoin Secure Multisig Setup, BIP-129, is a procedure for building a multisig wallet without trusting the coordinator to report the keys honestly. Each signer produces its key record, the coordinator assembles them into a descriptor record, and the record goes back to every signer to be confirmed and stored before any coins are sent. Every signer ends up holding the same wallet description, checked on its own screen.

Faraday writes a key record for each multisig key held here, under a wallet's public files, For the cosigners: the signed xpub record of BIP 129, signed by the account key itself, as text and as a code. It reads a descriptor record wherever it reads a wallet: Scan, Files, and Restore an existing wallet. It checks the wallet's first address against the one the record states, and shows the quorum, the derivation paths the record restricts the wallet to, and which keys are loaded here. A multisig wallet's public files write the descriptor record back out. Encrypted records, which are hex rather than text, are refused by name. A record with a token in it is still plain text, so carry it by QR code or memory card, not through the coordinator.

Keep a BSMS record for the same reason as a descriptor: it is what recovery needs when the coordinator is gone.

## Bitcoin Core

A wallet's public files offer the Bitcoin Core import format, which is the JSON that `importdescriptors` takes: the wallet's descriptor with its checksum, marked active, over the first thousand addresses of each chain. Bitcoin Core 29 or later takes it as written; an older Core takes the file under More options, Core before 29. The file asks Core to scan the whole chain from the first block, which finds coins the wallet already holds and takes a while.

Save the file, carry it to the machine running Core, and run `bitcoin-cli -rpcwallet=<name> importdescriptors "$(cat <file>)"` on a wallet created with `createwallet` and private keys disabled. This file is the public wallet and nothing else.
