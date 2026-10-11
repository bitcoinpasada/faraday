# Glossary

## Account

One branch of a key with its own addresses and its own history. A coordinator usually watches one account at a time.

## Address

Where a payment goes. It is derived from a key, so a wallet can make as many as it needs. Every address should be used once.

## Air gap

No network connection of any kind. Data crosses as QR codes or as files that you carry.

## BSMS

Bitcoin Secure Multisig Setup, BIP-129: a procedure in which every signer confirms the same multisig description on its own screen before the wallet is used.

## Carry file

The file that carries a part-signed FROST transaction from one signer to the next, holding the transaction and the secret nonce of each signer still to sign.

## Change

The part of an input that comes back to you, at an address derived from your own key.

## Checksum

Bits at the end of the seed words that catch a typo. Passing the checksum does not prove the words are yours.

## Codex32

BIP-93: a master seed written as one checksummed string, optionally split into shares, with arithmetic that can be done by hand.

## Coordinator

The internet-connected wallet software that watches your balances, builds transactions, and sends them to the network. It has no keys.

## Delay

A wait written into a wallet's script, after which a spend path that was closed becomes usable.

## Derivation path

The route from a master key to one of its child keys, level by level, starting at the root, m. An h marks a hardened level. The levels are purpose, coin, account, change, and index.

## Descriptor

One line of text that describes a whole wallet: which keys, which script type, which paths. Your coordinator can export one.

## Diceware

A way of picking words at random by throwing dice. Each throw of four or five dice names one word of a published list, and the words together are a passphrase. OpenSigner carries the three lists the Electronic Frontier Foundation publishes.

## Entropy

Randomness, measured in bits. 128 bits of real entropy is unguessable. Entropy from a memorable phrase is far lower than it looks.

## Eye glyph

The mark at the start of a row that names a wallet or one of its keys: this device holds the public key only. You can derive and compare its addresses; you cannot spend from it. On a screen with a secret panel the eye is a button instead, which reveals the panel; the two never appear on one screen.

## Fee

The inputs less the outputs. It goes to the miner of the block.

## Fingerprint

Four bytes that identify a key, so a coordinator and a signer can confirm they mean the same one.

## FROST

A scheme in which one key is split into n shares, any m of which sign, and the chain sees one key and one signature.

## Group record

The public file that states a FROST wallet's threshold, its group key, the public half of every share, and its descriptor. A share without it opens nothing.

## Hot wallet

Wallet software that holds keys on an internet-connected device.

## Input

A coin being spent. An input is always spent whole, and the remainder returns as change.

## Key

The secret a wallet is built from, held as seed words. Everything else is derived from it.

## Key glyph

The mark at the start of a row that names a wallet or one of its keys: this device holds the private key, so it can sign for that wallet. A 2-of-3 with one of your keys in it carries the key glyph.

## Locktime

The earliest block or time at which a transaction may confirm.

## Multisig

A wallet with several keys that spends only when a set number of them sign.

## Network

Which chain a key and its addresses belong to: mainnet, testnet, signet, or regtest. Funds on one network do not exist on another.

## Node

Software that keeps a full copy of the blockchain and validates every transaction. A coordinator connects to one.

## NUMS point

A public key chosen so that nobody knows a private key for it. It is used as a Taproot internal key when a wallet is meant to be spendable only by its scripts.

## Output

A coin being created: an address and an amount.

## Passphrase

Extra text added to the seed words that produces a different key. It is stored nowhere.

## PIN

The number that opens a session on OpenSigner, or a key kept on an Android device.

## Policy

The rule a wallet spends by: which keys, how many of them, and any wait or spend path. A descriptor is one way of writing it down.

## PSBT

Partially Signed Bitcoin Transaction. What a coordinator hands to a signer, and gets back with a signature in it.

## Recovery path

A spend path that becomes usable only after a delay, so that a wallet can be recovered later by keys that cannot spend from it today.

## Sat

The smallest unit of bitcoin. One bitcoin is 100,000,000 sats.

## Script type

The form of a wallet's addresses: legacy, nested SegWit, native SegWit, or Taproot. It decides the derivation path and the address prefix.

## Seed

The 64 bytes that the seed words and passphrase are stretched into. Every key of the wallet comes from it.

## Seed words

The 12 or 24 words that are the key. Whoever reads them controls the funds.

## Share

One of the n pieces a FROST wallet's key is split into, held as 24 words. Any m of them sign; one alone can do nothing.

## Signature

Proof that a key agreed to a transaction or a message. On its own it moves nothing.

## Signer

The offline software or device that holds the keys and signs transactions. OpenSigner is a signer.

## SLIP-39

Trezor's backup scheme: Shamir's Secret Sharing over a word list of its own, with a threshold. Its shares are not BIP-39 seed words.

## Xpub

An account's extended public key. It reveals every address and payment of that account and can sign nothing.
