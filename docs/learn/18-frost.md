# FROST

## What it is

FROST splits one key into n shares. Any m of them can sign; fewer cannot, and no share on its own can do anything. The chain sees one public key and one signature, exactly as it would for a wallet with a single key, and learns nothing about how many shares exist or how many signed.

A 2-of-3 multisig announces itself in every spend; a 2-of-3 FROST wallet is indistinguishable from one person with one key.

## The shares are keys

Each share is 24 words, written down and backed up the way any key is. On this device a share is a key in Keys, with a fingerprint like any other, and the wallet finds its shares by computing each loaded key's public half and matching it against the record. You never type a share number.

A share derives no addresses of its own within the wallet. It is one piece of the group's single key.

## The group record

The record is a short public file that states the threshold, the group's public key, the public half of every share, and the wallet's descriptor. Without it, the device cannot use a share on its own: it cannot tell which group the words belong to, what the threshold is, or which addresses to watch.

So the record travels with every share. Keep a copy wherever you keep a share, and a copy with whoever inherits. It reveals no secret, so make as many copies as you like; if every copy is lost, the wallet cannot be rebuilt even with every share in hand.

## Creating a wallet

A dealer makes the shares. On this device that means choosing how many shares and how many must sign, picking the loaded keys that will be the chosen shares, and letting the device compute the rest; each computed share becomes a loaded key of its own, which you back up like any other key and can then forget.

The shares exist together on one device for the length of that flow: that is what a dealer is.

## Signing in two places

Signing takes two rounds, as MuSig2 does: every signer publishes a nonce, then every signer produces a partial signature over the collected nonces. When the signers are not in the same room, the rounds are carried in a file.

At the first place you choose which other signers will take part, and the device draws every signer's nonce, signs with the share it holds, and saves a carry file. At the next place the carry file is read. The device checks that each stored nonce is the one the transaction already names, that the file is bound to this transaction (its signature hash) and to this set of signers, and that every earlier partial signature verifies. Only then does it sign, add the partials together, and finish the transaction. Any mismatch is refused by name.

The carry file holds the transaction and one secret nonce per signer still to sign. It names no device: any device that holds a signer's share and the group record can continue it. It is a file you keep to yourself and carry between the places, not one you publish or hand to a coordinator.

Writing a secret nonce to a file is safe here because both the nonce and the message were fixed by the device that drew them. Reusing a nonce leaks a share only when the same nonce signs under two different challenges, and any change to the nonce or the transaction breaks a check the device makes by arithmetic. A copy taken in transit is a random number with nothing to pair it with, because the signature made under it never leaves the device.

## A lost share

Lose one share of a 2-of-3 and the funds are still spendable by the other two. Any m shares together also rebuild the whole set, so a lost share can be replaced rather than lived with.

Lose more than n − m shares and the wallet is gone, with every copy of the group record intact and useless.

## The decoy property

A share's 24 words are valid seed words. Loaded as an ordinary key, with no group record in sight, they open a working single-sig wallet that you may keep funded. They act as a member of a FROST wallet only after you have also loaded the group record that lists their public share; nothing in the words shows which wallet they belong to.

## What it is not

FROST is not a backup scheme: shares do not reconstruct your other wallets. It is also newer than multisig and understood by less software. A coordinator that cannot read the group record sees a single-key Taproot wallet, which is enough to watch it and build transactions, and not enough to know who has to sign.
