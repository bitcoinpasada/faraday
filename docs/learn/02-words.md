# Seed words

## Why 12 or 24

The word protocol is BIP-39: a list of 2048 words, each carrying 11 bits. There are five lengths. 12 words carry 128 bits of key, 15 carry 160, 18 carry 192, 21 carry 224 and 24 carry 256, and each length adds a checksum. 128 bits cannot be guessed, so 12 words are enough; 24 give a larger margin. Faraday makes and reads all five, and every wallet that follows BIP-39 reads them.

Each word has a fixed place on the list. Tools › BIP-39 words lists the 2048 words, each with its number and its 11 bits.

Faraday names each loaded key by its fingerprint: eight hex digits computed from the key, the same in every wallet program.

## Keys and other secrets from one key

BIP-85 derives a second secret from a key you already hold, at a path with an application number and an index in it. The same key and the same index always give the same result, so the parent words are the only thing that has to be backed up.

Tools › Child seeds and passwords asks which application. Words derives another BIP-39 key, which Load as a key adds to the keys loaded like any other, named by its own fingerprint. WIF derives one private key in the form Bitcoin Core takes as a wallet seed. Extended private key derives a whole BIP-32 tree. Hex derives 16 to 64 raw bytes, for a program that asks for a key of its own. The two password applications derive a password of the length you ask for, one in base64 and one in base85, for a password manager or a disk.

A derived password or WIF is as secret as the key it came from: it reveals nothing about the parent, but whoever holds it holds whatever it protects, and whoever has the parent words can recompute every child. Faraday saves the value into the open vault, sealed under its passphrase; it leaves the vault only after a warning.

Before child words go into a vault or load as a key, Faraday asks whether to record that they are a child seed, and of which key and index. Recorded, the vault keeps it, and Keys shows it; not recorded, the words are an ordinary seed. Nothing written for a seed's backup, on paper or on a stick, says it is a child seed: whoever finds the backup of a child seed cannot tell from it that another key exists, which leaves the owner able to deny it. To remember where a child seed came from, write it down yourself, apart from its backup.
