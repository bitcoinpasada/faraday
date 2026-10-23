# Xpubs and privacy

## What an xpub is

An xpub, or extended public key, is the public key for one account of your wallet. Your coordinator needs it to find your funds, show your balance, and build transactions. It is what you give a read-only wallet.

Faraday shows it as a QR code or writes it as a file, together with the fingerprint and derivation path the coordinator needs to use it: Show xpub QR and Xpub file, on a key of the wallet's chart. A wallet's public files carry the account keys it is built on. Tools › Explore a key shows a key's extended public key at any path. The same key is written xpub, ypub or zpub to say which script type it is for; Tools › Convert an xpub shows every spelling.

## What it reveals

Every address of that account, the ones already used and every one still to come, and therefore every payment in and out of them, with amounts and dates, forever. That is the account's whole financial history. Once the xpub is tied to your name, the history is tied to your name.

## What it cannot do

An xpub cannot sign, so nobody can spend with it. It does not reveal the seed words. It does not reach your other accounts, and it does not reach any wallet under a passphrase.

## Who should have your xpub

Your own coordinator, on a machine you control. The other members of a multisig, who cannot build the wallet without it. Nobody else, unless you are content for them to read the whole account: not a support chat, a forum post, a block explorer, or a website offering to check your balance. What you hand over cannot be taken back.
