# Words Faraday uses

**Kind:** Reference
**Status:** Checked against the app
**Key:** glossary

The words Faraday uses in its own way, each with what it means in the app. A page that uses some of them lists them under Words on this page, and a press of one opens its definition here.

## BIP-39 passphrase

**Key:** glossary.bip39-passphrase

An optional extra phrase added to a seed's words when the key is made. The same seed with another passphrase makes another key, and Faraday never keeps the passphrase.

## Boot stick

**Key:** glossary.boot-stick

The USB stick Faraday starts from. It holds the kernel and the application, and it is not where files are kept.

## Checklist

**Key:** glossary.checklist

What a backup plan becomes: one item for each thing to make, each opening once the one before it is done. The last item writes the sticks.

## codex32

**Key:** glossary.codex32

A paper backup that splits a key into shares written as short strings of letters and digits. As many shares as the threshold rebuild the key.

## Descriptor

**Key:** glossary.descriptor

A line of text that says which keys make a wallet's addresses and how they spend. It holds public keys only, so it can rebuild the wallet but cannot spend.

## Fingerprint

**Key:** glossary.fingerprint

Eight characters that name a key, such as 9a6a2580. The same seed with another passphrase has another fingerprint, so it is how one key is told from another.

## For the stick

**Key:** glossary.for-the-stick

The half of Files that holds what a stick visit writes: sealed vault files, signed transactions and other files Faraday has made. They wait there until a stick is attached after the session is locked.

## Inbox

**Key:** glossary.inbox

The list of files and codes Faraday has taken in and can load, whether copied in from a stick, scanned or read from an open vault. It is held in memory and emptied at power-off.

## Key

**Key:** glossary.key

What is loaded in Faraday and signs, named by its fingerprint. A key is made from a seed, and from a BIP-39 passphrase if one is used.

## Lock

**Key:** glossary.lock

Ends the session: every open vault is sealed into Files, For the stick, every key, seed and passphrase is dropped from memory and the application restarts. Home is the first screen after it.

## Multisig

**Key:** glossary.multisig

A wallet that needs several keys to sign: k of n keys, such as 2 of 3. The number needed is its quorum.

## Part

**Key:** glossary.part

One piece of a seed split by Seed XOR. Each part on its own is valid words for a different, empty wallet, and all the parts together make the seed. An old part is one still kept after Make new parts, until it is destroyed.

## Place

**Key:** glossary.place

Somewhere a backup is kept: a sheet of paper, an envelope, a stick or a vault. A backup plan lists the places and what each one holds.

## Plan

**Key:** glossary.plan

A backup's list of the places that keep a wallet's keys and files, and what each place holds. The plan becomes the checklist of things to make.

## Practice

**Key:** glossary.practice

A mode in which Faraday runs on testnet with public sample keys, on an online computer. Every screen opens and works as it does in Faraday, but no secret a person could take away is made.

## PSBT

**Key:** glossary.psbt

A partially signed Bitcoin transaction: the file or code a wallet app gives Faraday to sign and gets back signed. Faraday shows what it spends and pays before it signs.

## Quorum

**Key:** glossary.quorum

The number of keys that must sign to spend from a multisig wallet, such as 2 of 3. A place holds a quorum when the seeds it keeps are enough to spend alone.

## Seal

**Key:** glossary.seal

To lock a file under a passphrase so that it can be read only by opening it again with the passphrase. A vault is sealed when it is written and again at every lock.

## Seed

**Key:** glossary.seed

The 12 or 24 words that make a key, written down, scanned as a SeedQR or kept in a vault. One seed with a BIP-39 passphrase makes another key than the same seed without one.

## Seed XOR

**Key:** glossary.seed-xor

A way to split a seed into two or three parts, each a valid set of words, so that all the parts together make the seed. No part alone holds the wallet.

## SeedQR

**Key:** glossary.seedqr

A QR code that holds a seed's words, in a standard or a compact form, so that a seed is scanned and not typed.

## Session

**Key:** glossary.session

The time from loading a key to the lock. Keys and seeds are held in memory for the session only.

## Share

**Key:** glossary.share

One piece of a key split into several, a set of which rebuilds it, as in codex32 and SLIP-39. A share of a descriptor is a multisig descriptor with some keys left off.

## Stick visit

**Key:** glossary.stick-visit

A time a plain USB stick is attached to copy files in from it or write files to it, while no vault is open.

## Unlock cost

**Key:** glossary.unlock-cost

The memory and passes Argon2id takes for each guess at a vault's passphrase, shown as a time and as bits added to the passphrase's strength. It is fixed when the vault is made. A higher cost slows every guess, an attacker's and yours, and needs a computer with more memory to open the vault.

## Vault

**Key:** glossary.vault

A file sealed under a passphrase that holds keys, wallets, entries and notes. It is copied in on a stick visit, opened with its passphrase and sealed again at the lock.

## Vault passphrase

**Key:** glossary.vault-passphrase

The passphrase a vault is sealed under. It opens the vault file and is not a seed's BIP-39 passphrase: it never changes a key.

## Wallet

**Key:** glossary.wallet

The addresses made from one key or several by a descriptor. Faraday signs for a wallet; a wallet app watches it and makes the transactions.

## Watch only

**Key:** glossary.watch-only

A wallet this computer holds no key of, so it cannot spend from it. Its public keys and descriptor are enough to follow it.

## Words

**Key:** glossary.words

A seed written as words from the BIP-39 list of 2048 words, the usual paper backup of a key.

## xpub

**Key:** glossary.xpub

An extended public key: the public key from which every address of one account of a key is made. It cannot spend, and Faraday takes it only with its origin, the key's fingerprint and path.
