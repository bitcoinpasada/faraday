# Glossary

## Account

One branch of a key with its own addresses and its own history. A coordinator usually watches one account at a time.

## BSMS

Bitcoin Secure Multisig Setup, BIP-129: a procedure in which every signer confirms the same multisig description on its own screen before the wallet is used.

## Carry file

The file that carries a part-signed FROST transaction from one signer to the next, holding the transaction and the secret nonce of each signer still to sign.

## Checksum

Bits at the end of the seed words that catch a typo. Passing the checksum does not prove the words are yours.

## Codex32

BIP-93: a master seed written as one checksummed string, optionally split into shares, with arithmetic that can be done by hand.

## Coordinator

The internet-connected wallet software that watches your balances, builds transactions, and sends them to the network. It has no keys.

## Delay

A wait written into a wallet's script, after which a spend path that was closed becomes usable.

## Derivation path

The route from a master key to one of its child keys, level by level, starting at the root, m. An h, or an apostrophe, marks a hardened level. The levels are purpose, coin, account, change, and index.

## Descriptor

One line of text that describes a whole wallet: which keys, which script type, which paths. Your coordinator can export one.

## Diceware

A way of picking words at random by throwing dice. Each throw of four or five dice names one word of a published list, and the words together are a passphrase. Faraday carries the three lists the Electronic Frontier Foundation publishes.

## Fingerprint

Four bytes that identify a key, so a coordinator and a signer can confirm they mean the same one.

## FROST

A scheme in which one key is split into n shares, any m of which sign, and the chain sees one key and one signature.

## Group record

The public file that states a FROST wallet's threshold, its group key, the public half of every share, and its descriptor. A share without it opens nothing.

## Identifier

Four characters in boxes 5 to 8 of a codex32 string, the same in every share of one set. They name the set and are not secret.

## Key

The secret a wallet is built from, held as seed words. Everything else is derived from it.

## Locktime

The earliest block or time at which a transaction may confirm.

## Multisig

A wallet with several keys that spends only when a set number of them sign.

## Network

Which chain a key and its addresses belong to: mainnet, testnet, signet or regtest. Funds on one network do not exist on another. Settings chooses it as the Mode: Demo, Mainnet, Testnet, Signet or Regtest.

## NUMS point

A public key chosen so that nobody knows a private key for it. It is used as a Taproot internal key when a wallet is meant to be spendable only by its scripts.

## Passphrase

Extra text added to the seed words that produces a different key. Nothing stores it unless you keep it in a vault, which is best a different vault from its seed.

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

## Share

One of the pieces a key or a description is split into. A FROST wallet's key is split into n shares held as 24 words, any m of which sign; SLIP-39 and codex32 split a seed into shares; a backup can split a multisig's description into shares. One share alone can do nothing.

## Share index

The character in box 9 of a codex32 string: s for the secret itself, another character for each share of the set.

## SLIP-39

Trezor's backup scheme: Shamir's Secret Sharing over a word list of its own, with a threshold. Its shares are not BIP-39 seed words.

## Translation symbol

In codex32, the number from 0 to 31 an input string is multiplied by, character by character, when a share or the secret is made from k strings. The codex32 booklet writes it in a second alphabet of Greek letters and signs.

## Vault

One file on a stick holding keys, wallets, entries and notes, encrypted under a passphrase. It has four slots, and each passphrase opens one of them.

## Watch-only

A wallet whose keys are not loaded here. Its addresses can be derived and compared; nothing can be spent from it.

## Xpub

An account's extended public key. It reveals every address and payment of that account and can sign nothing.
