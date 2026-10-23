# Backups in other forms

## Why there is more than one

Words on paper or steel is the backup almost everybody should make first. The forms on this page answer the two problems a single written copy has: it can be destroyed, and it can be found and read. Each trades something away. If you do not fully understand a scheme, you are safer with a single written copy.

## SLIP-39

SLIP-39 is Shamir's Secret Sharing over a word list, published by Trezor. It splits a master secret into shares with a threshold, so any 3 of 5 shares rebuild it and two do not, and it can nest that once: groups of shares, with a threshold over the groups as well.

Its words are not BIP-39's. The list is a different one, a share is a different length from a seed phrase, and a share cannot be typed into a wallet that expects seed words. A SLIP-39 backup and a BIP-39 backup of the same wallet are not interchangeable: you choose one at the moment the key is made.

Pick it when you want a real threshold and your devices and heirs will have SLIP-39 software. Faraday reads it, groups and all, and writes it. Add a key › Type SLIP-39 shares loads the key a backup's shares hold, under its passphrase if it has one. Add a key › Make SLIP-39 shares makes a new key and writes it as one group of shares: you choose how many shares there are and how many are needed, and Faraday shows every share in turn with the quiz. A key made of BIP-39 words is not backed up as SLIP-39 shares: a BIP-39 key's seed is 64 bytes, which no SLIP-39 implementation takes.

## Codex32

Codex32 is BIP-93: a master seed written as one checksummed string beginning ms1, with optional shares and a threshold, as SLIP-39 has. The checksum and the share arithmetic were designed to be worked by hand, with the printed tables that come with the standard, on paper and without a computer. You can create a seed, verify the checksum, and combine shares without trusting any device with the secret, including this one. The cost is time and care: a mistake in the arithmetic is silent until the checksum catches it.

Faraday reads and writes it. Add a key › Type a codex32 secret takes a string, or the shares of a split of one, and loads the key it holds. A backup's copies by hand can write a key made of words as codex32 shares of its seed, 2 of 3 or 3 of 5. What a codex32 backup of a key made of words holds is the 512-bit seed those words come to, with the passphrase already in it, which is 127 characters; a wallet that expects words cannot read it back.

## Seed XOR

Seed XOR splits one key into two or more parts. Each part is a full set of seed words of the same length as the original, and combining the parts with the XOR operation gives the original key back. It is Coldcard's scheme: parts made here combine on a Coldcard, and parts made on a Coldcard combine here.

All but the last part are random, and the last is worked out so that the parts XOR back to your key. Every part is needed: it has no threshold, two parts of a three-part split are worth nothing, and losing one loses the key. In exchange it is simple enough to do with pencil and paper from the wordlist numbers, and any wallet can read a part.

Each part is also a valid set of seed words, so each is a working wallet with addresses of its own. Coins sent to an address derived from a part are held by that part alone, not by your key. Treat every part with the same care as the original words.

In Faraday, a backup splits a seed among its copies by hand into two or three parts, with the random parts drawn from the device's own generator. Add a key › Type Seed XOR parts combines them into the key that already existed.

## A vault

A vault is a file on a stick holding your words encrypted under a passphrase you choose. It is not a split: it is one copy that can be stored where a readable copy could not. Its weak point is the passphrase, now a second thing to keep and a second thing to lose. Vaults has its own page.

## Which one

If the problem is that a copy may be destroyed, more copies solve it, and no scheme on this page is needed. Two places, two copies.

If the problem is that a copy may be found, a passphrase or a vault solves it with one extra secret to keep. A threshold scheme solves it with n places to keep instead of one, and m of them to visit before you can spend.

If the problem is that you want no single person or place to be able to act alone, a threshold over the backup is the wrong tool and a multisig or FROST wallet is the right one: those keep the coins under several keys all the time, rather than putting them back under one key whenever you spend.
