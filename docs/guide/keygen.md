# Make a new key

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `KeyGen`
**Key:** keygen

Make a new key makes a key from randomness, or takes in one you already hold as words, shares or strings. It is one card per step, from how the key comes in to the quiz on its backup; headed Add a key, the same cards take a key in for a wallet.

## Start

**Key:** keygen.start

First choose how the key comes in. Under **Make a new key** the device makes one; under **Add a key you have** you type or scan a key that is already written down. **Other forms** opens three more.

**BIP-39 words** makes a key of 12 to 24 words. **Codex32** makes codex32 strings worked out by hand, on its own cards. **Type the words**, **Scan a SeedQR** and **Type codex32 strings** take in a key you hold on paper or in a code. **Make SLIP-39 shares** makes shares, **Type SLIP-39 shares** takes them in, and **Type Seed XOR parts** takes in the parts of a Seed XOR key.

## Length

**Key:** keygen.length

Choose how many words the key has. **12 words** and **24 words** are shown, and **Other** opens 15, 18 and 21 words. Twelve words carry 128 bits and 24 carry 256: either is beyond guessing, and most wallets and signers read every length. Each length ends in a checksum, which catches a word copied wrongly.

For SLIP-39 you choose shares of 20 or 33 words instead, and set **Shares needed** and **Shares dealt**: that many shares rebuild the key, and fewer reveal nothing of it. Shares restore only in software that reads SLIP-39.

## Randomness

**Key:** keygen.source

Choose where the key's randomness comes from. Each source asks you to trust something different, and nothing can show afterwards that a source was faulty.

Under **Your own entropy, words verifiable by hand**, your rolls or flips choose the words directly, so the result can be checked on paper: **Dice · Words chosen by the dice**, **Dice · Flip mode** and **Coin flips**. Six rolls name a word, and a 5 or 6 among the first five is rolled again; in flip mode 1 to 3 is 0 and 4 to 6 is 1.

Under **Your entropy, computer generates words**, your entries are hashed into the words: **Dice · Hashed (Coldcard, SeedSigner)**, **Dice · Six as zero, hashed (Keystone)**, **Hex entropy**, **Playing cards** and **Mixed sources**. The same rolls make a different key under each hashing. Under **Made by this device** there is one source, **This device**.

## Entries

**Key:** keygen.enter

Enter what you rolled, flipped or drew; the count shows how many entries the length still needs. Dice and coin flips are **Typed** or pressed as **Buttons**.

**Dice** takes faces 1 to 6. **Coin flips** takes heads or tails, or a die read as **Coin** or **Die**. **Playing cards** takes a rank, then a suit, and refuses a card already drawn: past 52, shuffle again. **Hex entropy** takes hex digits that are already random. **Mixed sources** takes two or more sources in turn. **This device** reads the system's generator once. Use a real die, and never pick numbers in your head.

**Undo** takes back the last entry and **Clear** all of them. After the last entry a **Randomness check** shows the counts, and **Roll again** starts over. **Make the words** comes next, or **Continue** where the words showed as you entered them; for SLIP-39 it deals the shares.

## Check

**Key:** keygen.check

For SLIP-39 shares only: the counts of the entries behind the shares just made.

**Rolls** and **Faces** count what came up, and **Longest run** is the longest repeat. The counts catch a biased die, or entries typed rather than rolled: a caution names a long run, a count far from even, or a run counting up or down. A caution does not stop you. **Continue** opens the shares, and **Roll again** starts the entries over. The check cannot turn poor entries into good ones.

## Key

**Key:** keygen.key

Shows the key your entries made. **Lock in** adds it to the loaded keys, and its fingerprint shows once it is locked in.

**Show words** and **Hide words** show and hide the words. For a key typed in, or made for a wallet's key slot, **Passphrase** and **Passphrase again** take a BIP-39 passphrase. With a passphrase the same words make a different key, and you need both to spend; left empty, there is none. A key made on its own has no passphrase.

The eye of a passphrase field shows what is typed. Once a hardware keyboard has typed one, **Keys are read as a US keyboard. Caps Lock is ignored.** shows and **Lock in** waits until the eye has been opened.

## Words

**Key:** keygen.words

The key's words, numbered, for you to write down. A key made for a wallet shows its words at its backup instead of here.

**Show words** and **Hide words** show and hide them. **I wrote them down** opens the quiz. SLIP-39 shows one share at a time: write each down before pressing **Next share**.

## Quiz

**Key:** keygen.quiz

The quiz checks your written copy. It asks for the word at each position, in a random order, with four words to pick from, and passes once every word is right.

**Ask it again** repeats a missed word. **Show the words** goes back to the words. For BIP-39 words **Done** ends the quiz; **Lock in** has already added the key. Opened from Tools, a finished key ends on a card with its name and fingerprint: **Make a wallet with this key**, **Another** for a new key from the start, or **Done** back to Tools. SLIP-39 shares are quizzed one share at a time, and **Use this key** or **Add a key** adds the key once the quiz passes. **Skip the quiz** adds the key unchecked, and a word copied wrongly then shows only when you need the backup.

## Codex32

**Key:** keygen.codex32

Make codex32 has its own cards: **Scheme**, **Identifier**, **Entropy**, **Characters**, **Worksheets**, **Strings** and **Strings made**.

**Scheme** sets **Length**, **48 characters** or **74 characters**, and **Split**: **2 of 3**, **3 of 5**, **Other** with a **Threshold**, or **None, the secret alone**. **Identifier** is four characters that name the set on every string. **Entropy** is **Roll dice** or **Type the characters**. **Worksheets** has a tab for each sum, read against the paper. **Strings** has each string written down, then typed back; **Skip** goes on without. **Strings made** has **Check on another device** and **Lock in**, which loads the key.

The codex32 booklet has worksheets to make the strings on paper: https://secretcodex32.com/. The codex32 user flow guide of SeedSigner's codex32 branch reads and checks strings and Codex32QR codes: https://github.com/FractalEncrypt/FractalEncrypt_seedsigner/blob/codex/codex32-error-correction/docs/codex32_qr/Codex32_User_Flow_Guide.md

Copyright (c) 2022 Blockstream
