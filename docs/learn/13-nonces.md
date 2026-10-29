# Nonces

## What a nonce is

Every signature is made with a one-time secret number called the nonce, combined with your private key and the transaction. The nonce must be different for every signature and must never be predictable.

## Why it matters

Anyone who learns the nonce used for a signature can calculate your private key from that signature. Using the same nonce twice gives the key away outright, and a nonce with a pattern in it leaks the key over time. This has happened to real wallets.

A signer that picks its nonce freely could also hide your key inside it on purpose, and a signature made that way looks exactly like an honest one. This is the one way a malicious signing device can steal funds without ever touching a network.

## What Faraday checks on a signature that arrived

A transaction can reach the device with signatures another device or another person already made. Faraday checks each of them against the key it is under and refuses the transaction when one is not a signature of what is in front of you, because that means the transaction was changed after it was signed. It also looks for two signatures under one key that share a nonce, anywhere in the transaction, and refuses that too: those two signatures are the key. Where a signature is under a key this device holds, it signs the same input again under each nonce rule and says which one produced it, so you can see whether another device signed the way it says it does. The Nonce check, on Spend's last card once the transaction is finished (Tools' Nonce check tile opens that card), lists them.

## How Faraday picks a nonce

For a single key, Faraday adds no randomness of its own. The nonce is calculated from the key and the transaction alone, using RFC 6979 for ECDSA signatures and BIP-340 with no auxiliary randomness for Schnorr signatures. The same transaction signed with the same key always produces exactly the same signature bytes. As a check on itself, Faraday signs every input twice and refuses to continue if the two results differ.

A MuSig2 round that goes first is different: its nonces are drawn from the session seed, which comes from the system's generator, and are never reused across signing passes.

## Which nonce rule

There are two standard ways to derive an RFC 6979 nonce. "Low R" retries the nonce, with a counter added as the RFC's extra data, until the signature comes out in its shortest form. "First" stops at the first nonce. Faraday signs with Low R, for transactions and messages alike, and has no setting for it.

Low R is what Bitcoin Core, Sparrow, and Electrum produce. First is what the RFC's own test vectors, the BIP-174 test vectors, and Trezor produce. Bitcoin Core signs messages without the retry, so a Core message signature does not match Faraday's byte for byte.

For taproot, Faraday adds no randomness to a single-key signature, so it can be compared against another implementation that does the same. BIP-340 recommends 32 fresh bytes in every signature as a defence against fault and side-channel attacks; a signature made that way cannot be reproduced.

## Checking Faraday against another signer

Because a single-key nonce is deterministic, you can verify that Faraday is not hiding anything in it. Load the same seed words into a second implementation that signs with Low R, and for taproot with no auxiliary randomness, on another offline machine, sign the same transaction on both, and compare the signature bytes. They must match exactly. A device that was leaking your key through its nonces would fail this comparison.

## What is not offered yet

There are protocols in which the coordinator contributes randomness to the nonce and then checks that the signer did not choose the nonce alone (anti-exfil). Faraday does not implement one. The only one that works over QR codes is an experimental alpha that no published standard carries yet. Faraday plans it for developer builds only, both to check another signer and to sign so that a coordinator can check Faraday, until a published standard carries it.

MuSig2 exchanges nonces before anyone signs, and Faraday signs in either order. When every other signer's nonce is already on the transaction, it derives its own from theirs and from the transaction, signs in one pass, and keeps nothing. When a nonce is missing, it goes first: it draws its nonce, writes the public half onto the transaction for the coordinator, and holds the secret half in memory as a session for that one transaction. The session ends when the device signs, when you lock it, and when it loses power, and nothing writes it to storage, because a secret nonce that survives a restart is the classic way to use one twice, and using a MuSig2 nonce twice hands over the private key. If the transaction comes back after the session has ended, the device draws a new nonce, says so, and the other signers sign again. Faraday writes no secret nonce to a file or a vault.
