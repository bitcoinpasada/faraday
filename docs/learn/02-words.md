# Seed words

## Seed words are a key

A key is a very large random number. So that a person can write it down without error, it is encoded as a list of words: 12, 24, or one of three lengths between.

The words are the key. Anyone who reads them can spend the coins, and there is no bank or company that can reverse the transfer. Do not photograph them, type them into a website, send them in a message, or show them to anyone.

Keys lists one row per key you have loaded: the fingerprint that names it, and under that what the key is made of — "12 words", "24 words and a passphrase", "BIP-85 child".

## Why 12 or 24

The word protocol is BIP-39. It has a list of 2048 words, and 2048 is 2 to the 11th, so each word carries 11 bits. Twelve words carry 132 bits: 128 bits of key and 4 of checksum. No amount of computing can guess a 128-bit number, so 12 words are enough. Twenty-four words carry 256 bits of key and 8 of checksum, for people who want a larger margin.

The three lengths in between are rarer than 12 and 24, and nothing else distinguishes them: they are built by the same rule, entropy plus a checksum of one bit per four bytes of it, and every wallet that follows BIP-39 reads them. 15 words are 160 bits, 18 are 192, and 21 are 224. OpenSigner creates and loads all five.

## How they are made

The randomness a key is made from is called entropy. A key made from something you remember, such as a sentence or a favourite set of words, is one of a small number of possibilities however long it is, and cracking programs try those possibilities first.

Real entropy comes from a physical process such as rolling dice, flipping coins or shuffling a deck. 128 flips of a coin, or 50 rolls of a die, give 128 bits. OpenSigner turns the rolls or flips into words on the device, so no computer has to generate the key for you.

A backup can be written as numbers instead of words, because each word has a place from 1 to 2048 on the BIP-39 list; Load a key › Word numbers takes a key that way, one number per word.

A key's entropy is sometimes written down as hex digits rather than words; Load a key › Hex entropy takes those 32 to 64 digits and computes the words from them.

## Keys and other secrets from one key

BIP-85 derives a second secret from a key you already hold, at a path with an application number and an index in it. The same key and the same index always give the same result, so the parent words are the only thing that has to be backed up.

The key page's BIP-85 row asks which application. Words derives another BIP-39 key, which is added to Keys like any other and can be loaded on another wallet. WIF derives one private key in the form Bitcoin Core takes as a wallet seed. Extended private key derives a whole BIP-32 tree. Hex derives 16, 32 or 64 raw bytes, for a program that asks for a key of its own. The two password applications derive a password of the length you ask for, one in base64 and one in base85, for a password manager or a disk.

A derived password or WIF is as secret as the key it came from: anyone who has it can compute nothing about the parent, but they hold whatever it protects, and someone who has the parent words can recompute every child. The device shows the value on a screen that stays masked until you hold it, does not copy it, and keeps nothing: write it down or type it in, and derive it again from the same key and index when you need it.

## Safe handling

There is no backup except the ones you make, there is no account, and there is nobody who can help you recover. If you lose the words, the coins are gone. If somebody else reads the words, they can take the coins. Backups has its own page; read it before you make a key.
