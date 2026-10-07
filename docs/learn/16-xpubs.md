# Xpubs and privacy

## What an xpub is

An xpub, or extended public key, is the public key for one account of your wallet. Your coordinator needs it to find your funds, show your balance, and build transactions. It is what you give a read-only wallet.

OpenSigner exports it as a QR code, together with the fingerprint and derivation path the coordinator needs to use it.

Any account of a key is exported from the key's own page, under Account key: the four single-signature accounts and the two multisig accounts BIP 48 defines, each with its path. A single-sig wallet's Export hands over the one account that wallet is built on.

## What it reveals

Every address of that account, the ones already used and every one still to come, and therefore every payment in and out of them, with amounts and dates, forever. That is the account's whole financial history. Once the xpub is tied to your name, the history is tied to your name.

## What it cannot do

An xpub cannot sign, so nobody can spend with it. It does not reveal the seed words. It does not reach your other accounts, and it does not reach any wallet under a passphrase.

## xpub, ypub, zpub

You will see the same key written with different prefixes. The prefix is a convention called SLIP-132 that tells the coordinator which script type the account uses: xpub for legacy, ypub for nested SegWit, zpub for native SegWit. The key inside is identical either way. OpenSigner can show it with whichever prefix your coordinator expects.

## Who should have your xpub

Your own coordinator, on a machine you control. The other members of a multisig, who cannot build the wallet without it. Nobody else, unless you are content for them to read the whole account.

## Where never to put it

Not in a support chat, a forum post, a block explorer, or a website offering to check your balance. What you hand over cannot be taken back.

## Vanity addresses

A key's page offers Vanity address: the device tries one candidate after another until the first address of the account begins with the characters you asked for. Each free character costs 32 tries on a bech32 address and 58 on a base58 one, so four characters past bc1q is about a million tries, and the device states the rate it is managing and how long that is.

There are two dials. The passphrase counter appends characters to the key's BIP-39 passphrase, which means every candidate is a different key and costs a full derivation; taking the find opens that key beside the one you started from. The account index steps through the accounts of the key you already have, which is far cheaper and adds a wallet rather than a key.

A vanity address is not safer. It is the same kind of address, with the same key behind it, and a string that looks familiar is a string an attacker can imitate: never check an address by its first characters alone. What the passphrase dial leaves you with is a passphrase to back up, since the words alone no longer reach the funds, and a counter written down nowhere is a key lost. The account dial leaves you with a wallet at an account no other software will look for unless you tell it, which its descriptor does.

The counter is the one other tools use, in the same order, so a find here is a find there.
