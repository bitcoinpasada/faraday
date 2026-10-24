# Backups in other forms

## Why there is more than one

Words on paper or steel is the backup almost everybody should make first. The forms on this page answer the two problems a single written copy has: it can be destroyed, and it can be found and read. Each trades something away. If you do not fully understand a scheme, you are safer with a single written copy.

## SLIP-39

SLIP-39 is Shamir's Secret Sharing over a word list, published by Trezor. It splits a master secret into shares with a threshold, so any 3 of 5 shares rebuild it and two do not, and it can nest that once: groups of shares, with a threshold over the groups as well.

Its words are not BIP-39's. The list is a different one, a share is a different length from a seed phrase, and a share cannot be typed into a wallet that expects seed words. A SLIP-39 backup and a BIP-39 backup of the same wallet are not interchangeable: you choose one at the moment the key is made.

Pick it when you want a real threshold and your devices and heirs will have SLIP-39 software. Faraday reads it, groups and all, and writes it. Add a key › Type SLIP-39 shares loads the key a backup's shares hold, under its passphrase if it has one. Add a key › Make SLIP-39 shares makes a new key and writes it as one group of shares: you choose how many shares there are and how many are needed, and Faraday shows every share in turn with the quiz. A key made of BIP-39 words is not backed up as SLIP-39 shares: a BIP-39 key's seed is 64 bytes, which no SLIP-39 implementation takes.

## Codex32

Codex32 is BIP-93: a master seed written as one checksummed string beginning ms1. Box 4 of the string is the threshold, boxes 5 to 8 the identifier, box 9 the share index, and the last 13 boxes (15 at 127 characters) the checksum. A string of 48 characters holds a 128-bit seed, one of 74 characters a 256-bit seed, and one of 127 characters a 512-bit seed.

A codex32 seed can stand alone, with threshold 0, or be split into shares, k of n, with k from 2 to 9 and up to 31 shares. Any k shares of one set rebuild the seed; fewer than k tell nothing about it. The shares of one set carry the same threshold, identifier and length, and each its own index.

The checksum and the share arithmetic were designed to be worked by hand. The codex32 booklet, Shamir Secret Sharing Codex, has the worksheets, tables and paper wheels to make a seed from dice, put its checksum on it, derive shares and recover the secret, on paper and without a computer. The booklet and its single worksheets are at https://secretcodex32.com/docs/index.html. The cost is time and care: a mistake in the arithmetic is silent until the checksum catches it.

The guide to read beside this page is the codex32 user flow guide of SeedSigner's codex32 branch, which reads codex32 strings and Codex32QR codes, checks them and offers corrections: https://github.com/FractalEncrypt/FractalEncrypt_seedsigner/blob/codex/codex32-error-correction/docs/codex32_qr/Codex32_User_Flow_Guide.md

## Making codex32 shares in Faraday

New key › Codex32 makes a key as the booklet does, every step on screen. Scheme sets the length, 48 or 74 characters, and the split: 2 of 3, 3 of 5, Other (a threshold of 2 to 9 and up to 31 strings), or None, the secret alone. Identifier takes four characters, typed before any entropy as the booklet orders it; Suggest fills four from this device's randomness. The identifier is not secret. One already used by the strings typed in Add a key is refused.

The entropy is your own. Roll dice follows the booklet's dice de-biasing worksheet: five dice are rolled twice; a die whose second value is lower than its first gives 0, one whose second value is higher gives 1, and a die with two equal values is rolled twice again. The five bits are one character, die 1's bit first. Type the characters takes characters made on paper, from dice or a worksheet. Characters a person makes up are not random, and a share built from them can give the seed away to someone holding fewer than k strings. Faraday's own randomness does not make the seed here.

The k initial shares are the entropy: their data boxes are the characters rolled or typed, unchanged. The other shares and the secret are derived from them. As each character is entered it drops into its box, and the checksum worksheet and the translation worksheets fill with it. Every cell opens to its working: the rolls that made it, the two cells added, the table row looked up.

After the last character, Worksheets shows each sheet whole, one tab per sheet, in the booklet's order: the dice and the checksum of each initial share, each derived share, the secret. Strings shows each string to write down and asks for it typed back; a string typed back that differs names the first box that differs. Strings made lists the split, the length, the identifier and the fingerprint. Check on another device lists sets of k strings to load on another implementation: every set must give the same fingerprint. Lock in loads the key.

On the stick and the Pi the sheets are shown while the flow is open and masked when it is left; the eye masks and shows them at any time. Faraday Desktop does not mask them: it is online, and a seed that holds real bitcoin is never made there.

## The codex32 worksheets

The checksum worksheet adds the string two characters at a time and looks up the next 13 symbols in the checksum table, row by row, as the booklet prints it. Making a string, its final row is fixed and the checksum boxes are worked from the bottom up. Checking a string, every box is given and the final row is worked out: the string is valid when that row reads SECRETSHARE32, or SECRETSHARE32EX at 127 characters.

A translation worksheet makes one string from k others. Each input string has a translation symbol, a number from 0 to 31 that the booklet writes in a second alphabet of Greek letters and signs. Each input is multiplied by its symbol, character by character, and the products are added. For a derived share the symbols come from the booklet's derivation table. For the secret, each string's recovery symbols, read off the recovery wheel or table, are multiplied together, which the booklet calls fusion.

The booklet publishes checksum worksheets for 48 and 74 characters, and translation worksheets only for 48 characters at k = 2 and k = 3. For k of 4 or more, and at 74 characters, Faraday's worksheets on screen are the only worked form; each cell can still be checked by hand with the wheels or tables. No sheet exists for 127 characters.

## Codex32 strings in

Add a key › codex32, also New key › Type codex32 strings, takes one string at a time, typed or scanned, and checks each at once. A string whose checksum fails is not taken, and is named by its index. A string with another identifier is refused. The same string again is already in. A different string at an index already in is shown beside the one held, with the boxes that differ, to keep one or the other. Once k strings of one set are in, the key loads.

Check a string, on the same form, says whether one string's checksum holds, what the string is, and whether it is of the set typed in. It loads and keeps nothing.

Suggest corrections, off whenever the form opens, adds ? to the keyboard for a box that cannot be read. A failing string is then decoded as BIP-93 allows: up to 4 changed boxes, or 8 unreadable ones, or a mix in which twice the changes plus the unreadable boxes come to 8 at most. A correction is shown only when it is the only one within that bound, and it is never taken as it stands. I mistyped clears the changed boxes, to be typed again from the paper. The paper is damaged gives the repairs to write on the paper, then the same typing. Unreadable on the paper takes the correction for boxes marked ?. From then on the key carries Share entry corrected or Share repairs unchecked beside its fingerprint.

A Codex32QR code is one 48-character string in a QR code, as SeedSigner's codex32 branch defines it. Faraday reads one wherever a codex32 string is scanned, and shows one for each 48-character string it makes. No code is defined for longer strings.

## Codex32 backups

A backup's copies by hand write a key held as codex32 strings: none, the secret alone, or a threshold of 2 to 9 of up to 31 strings, the random shares drawn from this device. For a key made of BIP-39 words the strings hold the 512-bit seed those words come to, with the passphrase already in it, at 127 characters: a wallet that expects words cannot read them back, and they are not for checking by hand. A key loaded from a master seed gets strings of its own seed's length.

In the backup plan a seed can be kept as codex32 strings, each string placed on its own. A place with k strings of one set holds the seed. A key made as a codex32 set keeps the set's facts (threshold, identifier, length, indices; no strings), and its backup places that set rather than splitting the seed again. Add a share, Replace a lost share and Rotate the set are the set's actions.

## Single key, backup in shares

Make a new wallet opens with Single key, backup in shares (codex32) ticked, above Single key and Multisig. Its Quorum is the codex32 scheme, how many shares rebuild it and how many there are. Its one key is made by Make codex32 shares.

On the backup side it gives a single-key wallet much of what multisig gives: no one paper or place loses the wallet, and fewer than k papers reveal nothing. At spending it does nothing multisig does. The wallet is single-key on chain, native SegWit, one xpub to the coordinator, and to sign, the whole key is on one device. Whoever holds k shares can spend alone.

Codex32 shares are made only for single-key wallets. A slot of a multisig, Taproot multisig, MuSig2 or recovery wallet offers BIP-39 words to make a key. A codex32 key already loaded can fill any slot.

## Codex32 sheets from the Starter

Blank templates in Faraday Starter print codex32 sheets with Form set to codex32. Length is 48, 74 or 127 characters. Copy is String, Codex32QR or Both, the last two at 48 only. Worksheets is None, or Dice, checksum and translation: at 48 characters the dice sheet, the checksum sheet and the translation sheets for k = 2 and k = 3; at 74 the dice sheet and the 256-bit checksum sheet; none at 127. A share sheet has ms1 printed in boxes 1 to 3, boxes for the threshold, the identifier and the share index, every box numbered, a box for the fingerprint, and the alphabet with 0, 7, S and Z slashed as handwriting should be. The sheets print at true size and hold nothing until written on.

Faraday draws these sheets itself, in the booklet's layout and numbering, from BIP-93's arithmetic. The booklet's notice, at the foot of this page, is kept here and is not printed on the sheets.

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

If the problem is that you want no single person or place to be able to act alone, a threshold over the backup is the wrong tool and a multisig wallet is the right one: it keeps the coins under several keys all the time, rather than putting them back under one key whenever you spend.

## The codex32 booklet's notice

The codex32 booklet, whose worksheets Faraday's codex32 sheets follow, carries this notice.

MIT License

Copyright (c) 2022 Blockstream

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
