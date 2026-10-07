# Backups in other forms

## Why there is more than one

Words on paper or steel is the backup almost everybody should make first. The forms on this page all answer the same two problems that a single written copy has: one copy can be destroyed, and one copy can be found and read by whoever finds it. Each of them trades something away to fix one of those.

Read this page before you split anything. If you do not fully understand a scheme, you are safer with a single written copy.

## SLIP-39

SLIP-39 is Shamir's Secret Sharing over a word list, published by Trezor and used on Trezor devices. It splits a master secret into shares with a threshold, so any 3 of 5 shares rebuild it and two do not, and it can nest that once: groups of shares, with a threshold over the groups as well.

Its words are not BIP-39's. The list is a different one, a share is a different length from a seed phrase, and a share cannot be typed into a wallet that expects seed words. The master secret it protects is the entropy itself, stretched into a seed by SLIP-39's own rules, so a SLIP-39 backup and a BIP-39 backup of the same wallet are not interchangeable: you choose one at the moment the key is made.

Pick it when you want a real threshold and your devices and heirs will have SLIP-39 software. This device reads and writes it, groups and all: Add a key has a row that makes a key whose only written form is shares, and a key loaded from shares can be written into a fresh set of them at any time.

## Codex32

Codex32 is BIP-93: a master seed written as one checksummed string beginning ms1, with optional shares and a threshold, as SLIP-39 has. What is unusual about it is that the checksum and the share arithmetic were designed to be worked by hand, with the printed tables that come with the standard, on paper and without a computer.

That is the point of it. You can create a seed, verify the checksum, and combine shares without trusting any device with the secret, including this one. The cost is time and care: doing it by hand is slow, and a mistake in the arithmetic is silent until the checksum catches it.

Pick it when your objection is to trusting a computer at all.

This device reads and writes it. Type a string, or the shares of a split of one, and the key it holds is loaded. Add a key has a row that makes a key and writes it as codex32: you choose how long a seed to make, whether to split it, and into how many shares of which how many must be present, and the device shows every string in turn and asks you to type it back. Every key already loaded can be written as codex32 from its Backup menu, whatever it was made from, because every key is a seed. What a codex32 backup of a key made of words holds is that seed, with the passphrase already in it, and not the words.

## Seed XOR

Seed XOR splits a key into parts that are themselves ordinary seed phrases, and every part is needed. It has no threshold: two parts of a three-part split are worth nothing. In exchange it is simple enough to do with pencil and paper from the wordlist numbers, and any wallet can read a part. Seed XOR has its own page.

## Encrypted backup

An encrypted backup is a file or QR code holding your words under a passphrase you choose. It is not a split at all: it is one copy that can be stored where a readable copy could not be. Its weak point is the passphrase, which is now a second thing to keep and a second thing to lose. Encrypted backups has its own page.

## Which one

If the problem is that a copy may be destroyed, more copies solve it, and no scheme on this page is needed. Two places, two copies.

If the problem is that a copy may be found, a passphrase or an encrypted backup solves it with one extra secret to keep. A threshold scheme solves it with n places to keep instead of one, and m of them to visit before you can spend.

If the problem is that you want no single person or place to be able to act alone, a threshold over the backup is the wrong tool and a multisig or FROST wallet is the right one: those keep the coins under several keys all the time, rather than putting them back under one key whenever you spend.

## What this device reads today

OpenSigner writes and reads Seed XOR parts and its own encrypted backup, and reads and writes BIP-39 words as every wallet does.

It reads and writes SLIP-39 shares. Type the shares of a backup and the key they hold is loaded, under its passphrase if it has one. Add a key › Create SLIP-39 shares makes a new key and writes it as shares: you choose how many groups, how many of them must be present, and how many shares each group has and needs, and the device shows every share in turn with the quiz. A key already loaded from shares can be written again, under a new plan, from its Backup menu; the old shares still open it.

A key made of BIP-39 words is not backed up as SLIP-39 shares, and never will be. What a set of shares gives back is the seed itself, and a BIP-39 key's seed is 64 bytes, which no SLIP-39 implementation will take.

It reads and writes Codex32. Type the string a Codex32 backup is written as, or the shares of a split of one, and the key it holds is loaded. Add a key › Create Codex32 shares makes a new key and writes it as one string or as a set of shares, and any key already loaded can be written as Codex32 from its Backup menu. A backup of a key made of BIP-39 words holds the 512-bit seed those words come to, which is 127 characters, and a wallet that expects words cannot read it back.
