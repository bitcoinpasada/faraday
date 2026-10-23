# Kinds of wallets

## What a wallet is here

A key is words. A wallet is a rule about which keys may spend, together with the addresses that rule produces. One key can be in several wallets, and the same key in two wallets with different script types gives two sets of addresses and two balances.

In Faraday the sidebar lists them apart: Keys are the secrets, and Wallets are the rules, whether you made them here or a coordinator sent them. The kinds below are the rules the device can read. Which one suits you depends on what you are protecting against, so all of them are offered.

## Single-sig

One key spends. The wallet still has a choice in it: the script type, which decides the address prefix and the derivation path. Legacy, nested SegWit, native SegWit and Taproot are four different wallets over the same words.

It is the cheapest arrangement to hold and to recover: seed words, a script type, and a derivation path. It is also the one where a single stolen backup is a total loss.

## Multisig

The wallet names n keys and requires m of them to sign. A 2-of-3 survives one stolen key and one lost backup. Spending reveals the script and puts m signatures on the chain, so anyone reading the block sees that it was a 2-of-3 and sees the three keys. Every coordinator supports this arrangement. The Multisig page covers this kind in full.

## Taproot multisig

The same m-of-n written as a Taproot script path. Unspent, the address looks like any other Taproot address and says nothing. Spending reveals the one script that was used and nothing else, and the transaction is smaller than the equivalent classic multisig.

When the wallet has no way to spend with a single key, the Taproot internal key is set to a NUMS point: a public key chosen so that nobody knows a private key for it. The device shows this in the review, because an internal key that has not been shown to be unspendable is a way to spend the wallet that you never agreed to.

Make a new wallet builds this kind: Taproot multisig under More kinds. Fewer coordinators write it than write classic multisig.

## MuSig2

All n keys together produce one public key and one signature. The chain sees a single-key Taproot spend: no threshold, no key count, nothing about the arrangement. It is BIP-327, written in a wallet with the musig() key expression of BIP-390.

Signing is interactive and takes two rounds. Every signer publishes a nonce, then every signer produces a partial signature over the collected nonces, and the partials add up to one signature. Every participant must take part; there is no m of n. A nonce must never be used twice, which is why the device holds one only in memory and only for the one transaction it was drawn for.

Use it when all the signers are reachable and what the chain reveals matters; not where one signer may be unavailable.

## FROST

Any m of n shares of one key sign, and the chain sees one key and one signature, as with MuSig2. The shares are made by a dealer at creation and each one is 24 words. FROST has its own page.

## Recovery and inheritance

A wallet can name one key that spends at any time and another that spends alone after a wait. That is two spend paths in one script, and it is how Liana's wallets are shaped. It gives you a recovery that needs no second person on the day, and an inheritance an heir can execute by waiting. It costs a timelock whose clock restarts every time the coins move, so the wallet has to be refreshed or the recovery path opens while you are still using it. Spend paths and timelocks has the details.

Make a new wallet does not offer this kind; Faraday reads one from its descriptor, including one with more than one recovery path, and names every path and every wait. Tools › Miniscript compiler writes a policy of your own as a descriptor.

## Silent payments

A silent payments wallet publishes one address that never changes and never reuses a payment. The payer works out a fresh taproot output that only you can find, so nothing on the chain links two payments to the same address. It is built over one key, and a wallet has to scan the blocks to find its payments. This device checks one transaction at a time and exports the scan descriptor; it does not send to a silent payment address. Silent payments has the details.

## Wallets with none of your keys

A wallet made from public keys alone derives addresses, checks change and exports itself, and cannot sign. Faraday calls it watch-only. Loading a friend's descriptor to check an address they gave you is a legitimate use of the device, and so is keeping a watch-only copy of a wallet whose keys are elsewhere.

## Choosing

Four questions decide it. How many people are involved, and must more than one of them agree? How many devices and backups will actually exist, in how many places? Does it matter what the chain shows about the arrangement? And must a coordinator, a recovery tool, or an heir's software understand the wallet years from now?

More parts mean the wallet survives one failure, and also that the plan has more ways to go wrong. A 2-of-3 kept in three places by a person who has never tested a recovery is worse than a single key on steel in two places. Pick the arrangement you will actually maintain, write it down, and test the recovery before it is needed.
