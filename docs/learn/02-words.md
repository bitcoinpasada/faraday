# Seed words

## Seed words are a key

A key is a very large random number. So that a person can write it down without error, it is encoded as a list of words: 12, 24, or one of three lengths between.

The words are the key. Anyone who reads them can spend the coins, and there is no bank or company that can reverse the transfer. Do not photograph them, type them into a website, send them in a message, or show them to anyone.

Faraday names each loaded key by its fingerprint: eight hex digits computed from the key, the same in every wallet program.

## Why 12 or 24

The word protocol is BIP-39. It has a list of 2048 words, and 2048 is 2 to the 11th, so each word carries 11 bits. Twelve words carry 132 bits: 128 bits of key and 4 of checksum. No amount of computing can guess a 128-bit number, so 12 words are enough. Twenty-four words carry 256 bits of key and 8 of checksum, for people who want a larger margin.

The three lengths in between are rarer than 12 and 24, and nothing else distinguishes them: they are built by the same rule, entropy plus a checksum of one bit per four bytes of it, and every wallet that follows BIP-39 reads them. 15 words are 160 bits, 18 are 192, and 21 are 224. Faraday makes and reads all five.

## How they are made

The randomness a key is made from is called entropy. A key made from something you remember, such as a sentence or a favourite set of words, is one of a small number of possibilities however long it is, and cracking programs try those possibilities first.

Real entropy comes from a physical process such as rolling dice, flipping coins or shuffling a deck. 128 flips of a coin, or 50 rolls of a die, give 128 bits. New key turns the rolls or flips into words, so no computer has to generate the key for you.

Each word has a fixed place on the BIP-39 list. Tools › BIP-39 words lists the 2048 words, each with its number and its 11 bits.

A key's entropy is sometimes written down as hex digits rather than words. New key takes hex digits made elsewhere as its source: the digits become the key as they are, so they must already be random.

## Keys and other secrets from one key

BIP-85 derives a second secret from a key you already hold, at a path with an application number and an index in it. The same key and the same index always give the same result, so the parent words are the only thing that has to be backed up.

Tools › Child seeds and passwords asks which application. Words derives another BIP-39 key, which Load as a key adds to the keys loaded like any other. WIF derives one private key in the form Bitcoin Core takes as a wallet seed. Extended private key derives a whole BIP-32 tree. Hex derives 16 to 64 raw bytes, for a program that asks for a key of its own. The two password applications derive a password of the length you ask for, one in base64 and one in base85, for a password manager or a disk.

A derived password or WIF is as secret as the key it came from: anyone who has it can compute nothing about the parent, but they hold whatever it protects, and someone who has the parent words can recompute every child. Faraday saves the value into the open vault, sealed under its passphrase; it leaves the vault only after a warning. The same key, application and index give it again when you need it.

## Safe handling

There is no backup except the ones you make, there is no account, and there is nobody who can help you recover. If you lose the words, the coins are gone. If somebody else reads the words, they can take the coins. Backups has its own page; read it before you make a key.
