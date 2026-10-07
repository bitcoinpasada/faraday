# Nonces

## What a nonce is

Every signature is made with a one-time secret number called the nonce, combined with your private key and the transaction. The nonce must be different for every signature and must never be predictable.

## Why it matters

Anyone who learns the nonce used for a signature can calculate your private key from that signature. Using the same nonce twice gives the key away outright, and a nonce with a pattern in it leaks the key over time. This has happened to real wallets.

A signer that picks its nonce freely could also hide your key inside it on purpose, and a signature made that way looks exactly like an honest one. This is the one way a malicious signing device can steal funds without ever touching a network.

## What OpenSigner checks on a signature that arrived

A transaction can reach the device with signatures another device or another person already made. OpenSigner checks each of them against the key it is under and refuses the transaction when one is not a signature of what is in front of you, because that means the transaction was changed after it was signed. It also looks for two signatures under one key that share a nonce, anywhere in the transaction, and refuses that too: those two signatures are the key. Where a signature is under a key this device holds, it signs the same input again under each nonce rule and says which one produced it, so you can see whether another device signed the way it says it does. The Signatures row on the review opens the list.

## How OpenSigner picks a nonce

As it comes, OpenSigner chooses no nonce with any randomness of its own. Every nonce is calculated from the key and the transaction alone, using RFC 6979 for ECDSA signatures and BIP-340 with no auxiliary randomness for Schnorr signatures. The same transaction signed with the same key always produces exactly the same signature bytes.

As a check on itself, OpenSigner signs every input twice and refuses to continue if the two results differ.

## The Nonce and Schnorr settings

There are two standard ways to derive an RFC 6979 nonce, and OpenSigner offers both under Settings. "Low R" retries the nonce, with a counter added as the RFC's extra data, until the signature comes out in its shortest form. "First" stops at the first nonce.

Low R is what Bitcoin Core, Sparrow, and Electrum produce. First is what the RFC's own test vectors, the BIP-174 test vectors, and Trezor produce. Bitcoin Core signs messages without the retry, so compare a Core message signature against First.

The Schnorr setting covers taproot. "Deterministic" adds no randomness, so the signature can be compared against another implementation. "Fresh randomness" mixes 32 new bytes into every taproot signature, which is what BIP-340 recommends as a defence against fault and side-channel attacks, at the cost of a signature nobody can reproduce.

## Checking OpenSigner against another signer

Because the nonce is deterministic, you can verify that OpenSigner is not hiding anything in it. Load the same seed words into a second implementation on another offline machine, set OpenSigner's Nonce setting to match that software and its Schnorr setting to Deterministic, sign the same transaction on both, and compare the signature bytes on OpenSigner's Signatures screen. They must match exactly. A device that was leaking your key through its nonces would fail this comparison.

## What is not offered yet

There are protocols in which the coordinator contributes randomness to the nonce and then checks that the signer did not choose the nonce alone. OpenSigner does not implement one yet, because no coordinator currently speaks such a protocol over QR codes or PSBT files.

MuSig2 is the other case. Several keys aggregate into one, and each signer's nonce is part of one aggregate nonce, so the nonces are exchanged before anyone signs. OpenSigner signs in either order. When every other signer's nonce is already on the transaction, it derives its own nonce from theirs and from the transaction, signs in one pass, and keeps nothing afterwards. When a nonce is missing, it goes first: it draws its nonce, writes the public half onto the transaction for the coordinator, and holds the secret half in memory as a session. That session is the device's memory of one transaction. It ends when the device signs, when you lock it, and when it loses power, and nothing writes it to storage, because a secret nonce that survives a restart is the classic way to use one twice, and using a MuSig2 nonce twice hands over the private key. If the transaction comes back after the session has ended, the device draws a new nonce, says so, and the other signers sign again.

A FROST wallet is the one place where OpenSigner writes a secret nonce to a file, and the reason it is safe there is that there are not two devices. One device carries the wallet's shares between the places you keep them. At the first place it draws every signer's nonce itself, signs with the share it is holding, and saves one file that holds the transaction and the other signers' secret nonces bound to it. At the next place the same device reads that file back, checks that each stored nonce is the one the transaction already names, that the file is bound to this transaction and to this set of shares, and that the signature already on it verifies, and only then signs. Reusing a nonce leaks a share when the same nonce signs under two different challenges, and the challenge changes only if someone varies a nonce or the message; here both were fixed by the device that drew them, and any change breaks a check it can make by arithmetic. A copy of the file taken in transit is a random number with nothing to pair it with, because the signature made under it never leaves the device. None of this extends to two devices signing together: a file like this is read only by the device that wrote it.
