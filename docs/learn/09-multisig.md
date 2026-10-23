# Multisig

## What m-of-n means

A multisig wallet is controlled by several keys, and a transaction is valid only when a set number of them have signed. In a 2-of-3 wallet any two of the three keys can move the funds. Each key has its own seed words, ideally generated on a different device and stored in a different place.

## What it protects against

One stolen key cannot spend. One destroyed backup cannot lose the funds. A single-key wallet fails on either of those, and a multisig survives both. It does not protect against a wrong address, a scam, or a wallet description that nobody wrote down.

## What it costs

Every key needs its own backup, so a 2-of-3 wallet means three sets of seed words in three places. On top of that, recovery needs the wallet description: which keys are in the wallet, the script type, the derivation path, and how many signatures are required. With every key in hand and the script type and path known, a lost description can be rebuilt; Faraday's Restore does this. With only a quorum of the seeds, two of the three, it cannot: the third key's xpub is in the description and nowhere else, so the addresses cannot be worked out and the funds cannot be found.

More parts means more to keep, more to explain to an heir, and more ways to lock yourself out of your own arrangement. Kinds of wallets says when a different arrangement fits better.

## Using multisig with Faraday

Make a new wallet makes a multisig from keys loaded here and cosigners' xpubs. Restore an existing wallet reads one a coordinator made: a descriptor, a wallet file, a multisig config or a BIP 129 record, by QR code or from a file. Faraday shows every key in the wallet and which of them are loaded here.

Once loaded, Faraday recognises the addresses and change of that wallet, so a transaction you sign is checked against the wallet you loaded. A wallet is kept for the session; saved into a vault, it comes back when the vault is unlocked.
