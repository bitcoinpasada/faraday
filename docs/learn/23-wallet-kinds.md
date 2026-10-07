# Kinds of wallets

## What a wallet is here

A key is words. A wallet is a rule about which keys may spend, together with the addresses that rule produces. One key can be in several wallets, and the same key in two wallets with different script types gives two sets of addresses and two balances.

In OpenSigner these are two screens: Keys lists the secrets, and Wallets lists the rules, whether you built them here or a coordinator sent them. The kinds below are the rules the device can read and build. Which one suits you depends on what you are protecting against, so all of them are offered.

## Single-sig

One key spends. The wallet still has a choice in it: the script type, which decides the address prefix and the derivation path. Legacy, nested SegWit, native SegWit and Taproot are four different wallets over the same words.

It is the cheapest arrangement to hold, to explain and to recover: seed words, a script type, and a derivation path. It is also the one where a single stolen backup is a total loss, and a single destroyed backup with no second copy is a total loss the other way.

## Multisig

The wallet names n keys and requires m of them to sign. A 2-of-3 survives one stolen key and one lost backup. The script lists all n public keys, and spending reveals the script and puts m signatures on the chain, so anyone reading the block sees that it was a 2-of-3 and sees the three keys.

Every coordinator supports this arrangement, and it has the most software behind it. It needs the wallet description as well as the keys: without it, the keys alone do not find the funds. The Multisig page covers this kind in full.

## Taproot multisig

The same m-of-n written as a Taproot script path. Unspent, the address looks like any other Taproot address and says nothing. Spending reveals the one script that was used and nothing else, and the transaction is smaller than the equivalent classic multisig.

When the wallet has no way to spend with a single key, the Taproot internal key is set to a NUMS point: a public key chosen so that nobody knows a private key for it. The device shows this in the review, because an internal key that has not been shown to be unspendable is a way to spend the wallet that you never agreed to.

Add a wallet builds this kind: pick Taproot multisig, choose the keys, choose how many must sign, and the review states the quorum, the keys and that the key path cannot spend. Fewer coordinators write this than write classic multisig, and recovery software for it is younger.

## MuSig2

All n keys together produce one public key and one signature. The chain sees a single-key Taproot spend: no threshold, no key count, nothing about the arrangement. It is the standard in BIP-327, and a wallet is written with the musig() key expression of BIP-390.

Signing is interactive and takes two rounds. Every signer publishes a nonce, then every signer produces a partial signature over the collected nonces, and the partials add up to one signature. Every participant must take part; there is no m of n. A nonce must never be used twice, which is why the device holds one only in memory and only for the one transaction it was drawn for.

Use it when all the signers are yours or reachable, and when what the chain reveals matters. Do not use it where one signer may be unavailable.

## FROST

Any m of n shares of one key sign, and the chain sees one key and one signature, as with MuSig2. The shares are made by a dealer at creation and each one is 24 words. FROST has its own page.

## Recovery and inheritance

A wallet can name one key that spends at any time and another that spends alone after a wait. That is two spend paths in one script, and it is how Liana's wallets are shaped: a primary key, and one or more recovery paths that become usable when a coin has sat untouched for a set number of blocks.

It gives you a recovery that needs no second person on the day, and an inheritance an heir can execute by waiting. It costs a timelock whose clock restarts every time the coins move, so the wallet has to be refreshed or the recovery path opens while you are still using it. Spend paths and timelocks has the details.

Add a wallet builds one: the keys that can sign now, then up to three recovery paths, each with its own keys and its own wait. Each wait has to be longer than the one before it, and you either pick one of the four offered or type a number of days, up to 455. Last comes whether the wallet pays to SegWit or to Taproot. The review names every path and every wait, in days and in blocks. A wallet Liana or another coordinator built arrives as a descriptor and is read the same way, including one with more than one recovery path.

## Silent payments

A silent payments wallet publishes one address that never changes and never reuses a payment. The payer takes the address, does the arithmetic with the keys of the inputs they are spending, and pays a fresh taproot output that only you can find. Nothing on the chain links two payments to the same address, and nothing has to pass between you and the payer beforehand.

It is built over one key: the device derives a scan key and a spend key at BIP-352's paths, and the address carries both public halves. Labels give the one address several forms, so a payer can be told apart from another without publishing a second address. Silent payments has the details.

What it costs is that a wallet has to look for the payments. There is no chain of addresses to hand a coordinator, so something has to scan the blocks with the scan key. This device does the arithmetic for one transaction at a time, under Check a payment, and exports the scan descriptor for a wallet that scans continuously. Sending to a silent payment address is not built here.

## Wallets with none of your keys

A wallet made from public keys alone derives addresses, checks change and exports itself, and cannot sign. Its rows carry the eye rather than the key. Loading a friend's descriptor to check an address they gave you is a legitimate use of the device, and so is keeping a watch-only copy of a wallet whose keys are elsewhere.

## Choosing

Four questions decide it. How many people are involved, and must more than one of them agree? How many devices and backups will actually exist, in how many places? Does it matter what the chain shows about the arrangement? And must a coordinator, a recovery tool, or an heir's software understand the wallet years from now?

More parts mean the wallet survives one failure, and also that the plan has more ways to go wrong. A 2-of-3 kept in three places by a person who has never tested a recovery is worse than a single key on steel in two places. Pick the arrangement you will actually maintain, write it down, and test the recovery before it is needed.
