# Multisig

## What m-of-n means

A multisig wallet is controlled by several keys, and a transaction is valid only when a set number of them have signed. In a 2-of-3 wallet there are three keys, any two of them can move the funds, and the third is not needed. Each key has its own seed words, ideally generated on a different device and stored in a different place.

## What it protects against

One stolen key cannot spend. One destroyed backup cannot lose the funds. A single-key wallet fails on either of those, and a multisig survives both.

## What it costs

Every key needs its own backup, so a 2-of-3 wallet means three sets of seed words in three places. On top of that, recovery needs the wallet description: which keys are in the wallet, the script type, and how many signatures are required. The seed words alone will not find the funds. If the description is lost, the funds are lost, even with all three keys in hand.

More parts means more to keep, more to explain to an heir, and more ways to lock yourself out of your own arrangement.

## When multisig is not the answer

One backup kept badly does not become safer as three backups kept worse. For most people, a passphrase, a better hiding place, or a second copy of the words solves more problems than multisig does.

Multisig protects against a stolen key and a destroyed backup. It does not protect against a wrong address, a scam, or a wallet description that nobody wrote down.

## Using multisig with Faraday

Make a new wallet makes a multisig from keys loaded here and cosigners' xpubs. Restore an existing wallet reads one a coordinator made: a descriptor, a wallet file, a multisig config or a BIP 129 record, by QR code or from a file. Faraday shows every key in the wallet and which of them are loaded here.

Once loaded, Faraday recognises the addresses and change of that wallet, so a transaction you sign is checked against the wallet you loaded. A wallet is kept for the session; saved into a vault, it comes back when the vault is unlocked.

## FROST and MuSig2

Both put the same arrangement under one key, so the chain sees one signature and nothing about how many keys exist; Kinds of wallets compares them with multisig, and FROST has its own page.
