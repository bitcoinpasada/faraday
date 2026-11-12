# Multisig

**Kind:** History
**Status:** The Learn pages' text as it was edited before the in-app guide (`docs/guide/`) replaced them

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

Faraday signs a spend from a wallet of several keys only when that wallet is registered here, that is, loaded or made here. The keys in a transaction are the coordinator's word, and a coordinator that lied could otherwise have you sign for a wallet whose other keys are someone else's. When a transaction arrives for a wallet not registered here, and it carries the cosigners' xpubs as most coordinators write them, Faraday offers Register this wallet: it shows the threshold, the script type, each cosigner's fingerprint and path and which key is yours, and registers the wallet when you hold the button, once the wallet built from those keys gives the transaction's own addresses. Check those keys against the ones you set the wallet up with. Without the xpubs, load the wallet's descriptor first.

## A cosigner's partial signature in a MuSig2 spend

In a MuSig2 wallet each key signs a part of one signature, and only the parts added together are a signature. A part that someone else put on the transaction can be checked only against the nonces on the transaction, and the coordinator writes all of those, so a part can be built that passes the check without the key behind it having signed. Faraday therefore calls such a part In file, unconfirmed: it is in the file and the file's word, not counted as signed, and it never reads Verified. The signed count says how many parts are in the file and how many more are needed.

Three things count. The finished signature verifying under the wallet's key, which counts every key of the wallet. A part Faraday made in this session, which counts its key. And a secret nonce this session still holds, which shows any part in the file for that key to be false, so that key signs again. A part that does not verify reads Does not verify and its key is still offered to sign again, with a new nonce.

When every key's part is in the file, Faraday adds them and keeps the signature only if it verifies. A transaction opened again after this device was locked or powered off shows this device's own earlier part as In file, unconfirmed too, because the session that made it is gone; it is not signed a second time, which would discard the cosigners' parts.
