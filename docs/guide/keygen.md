# New key

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `KeyGen`
**Key:** keygen

One card per step, from how the key comes in to its backup quiz. A key is made from randomness, or taken in from words, shares or strings already held. The same cards, headed Add a key, take a key in for a wallet.

## Start

**Key:** keygen.start

The choice is how the key comes in. Under **Make a new key**, the device makes it; under **Add a key you have**, a key already written down is typed or scanned. **Other forms** opens three more.

**BIP-39 words** makes 12 to 24 words. **Codex32** makes codex32 strings by hand, on its own cards. **Type the words**, **Scan a SeedQR** and **Type codex32 strings** take a key held on paper or in a code. **Make SLIP-39 shares** makes shares, **Type SLIP-39 shares** takes them and **Type Seed XOR parts** takes the parts of a Seed XOR key.

## Length

**Key:** keygen.length

The choice is how many words the key has. Each length adds a checksum to its bits.

**12 words** and **24 words** are shown; **Other** opens 15, 18 and 21 words. For SLIP-39 the choice is a share of 20 or 33 words, with **Shares needed** and **Shares dealt** as steppers: that many shares rebuild the key, and fewer reveal nothing of it. Shares restore only in software that reads SLIP-39.

## Randomness

**Key:** keygen.source

The choice is where the key's randomness comes from. Each source asks for trust in something different, and nothing can show afterwards that a source was faulty.

**Your own entropy, words verifiable by hand** lists **Dice · Words chosen by the dice** (six rolls name a word, a 5 or 6 in the first five rolled again), **Dice · Flip mode** (1 to 3 is 0, 4 to 6 is 1) and **Coin flips**. **Your entropy, computer generates words** lists **Dice · Hashed (Coldcard, SeedSigner)**, **Dice · Six as zero, hashed (Keystone)**, **Hex entropy**, **Playing cards** and **Mixed sources**; the same rolls make a different key under each hashing. **Made by this device** lists **This device**.

## Entries

**Key:** keygen.enter

The entries for the source chosen, counted against the number the length needs. Dice and coin flips are **Typed** or pressed as **Buttons**.

**Dice** takes faces 1 to 6. **Coin flips** takes heads or tails, or a die as **Coin** or **Die**. **Playing cards** takes a rank, then a suit, and refuses a card already drawn. **Hex entropy** takes hex digits made elsewhere, which must already be random. **Mixed sources** takes two or more of these in turn. **This device** reads the system's generator once; **Skip** goes on without adding randomness of one's own.

**Undo** takes back the last entry, **Clear** all of them. After the last entry of a source that has counts, a **Randomness check** shows them with cautions, and **Roll again** restarts. **Make the words** follows the last entry, or **Continue** where the words showed as entered; for SLIP-39 it deals the shares.

## Check

**Key:** keygen.check

For SLIP-39 shares only: the counts of the entries behind the shares made.

**Rolls** and **Faces** count what came up, **Longest run** is the longest repeat, and cautions name a long run, a count far from even or a run counting up or down. A caution does not block **Continue**, which opens the shares. **Roll again** starts the entries over. The check cannot turn a poor entry into a good one.

## Key

**Key:** keygen.key

Shows the key the entries made, and **Lock in** adds it to the keys loaded. The fingerprint shows once it is locked in.

**Show words** and **Hide words** toggle the words. For a key typed or made for a wallet's key slot, **Passphrase** and **Passphrase again** take a BIP-39 passphrase: with one, the words make a different key, and both are needed to spend. Left empty, there is none. A key made on its own has no passphrase.

## Words

**Key:** keygen.words

The words of the key, numbered, to write down. A key made for a wallet has its words shown at its backup instead of here.

**Show words** and **Hide words** toggle them. **I wrote them down** opens the quiz. For SLIP-39, one share is shown at a time, with **Next share**, and each share is written down before the next.

## Quiz

**Key:** keygen.quiz

The quiz asks each word's position, in a random order, with four words to pick from; it passes once every word is right.

**Ask it again** repeats a missed word. **Show the words** goes back to the words. **Done** ends the quiz for BIP-39 words, whose key **Lock in** added. SLIP-39 shares are quizzed one share at a time, and **Use this key** or **Add key** adds the key once the quiz passes. **Skip the quiz** adds it without the check; a word copied wrongly then shows only when the backup is needed.

## Codex32

**Key:** keygen.codex32

Make codex32 has its own cards: **Scheme**, **Identifier**, **Entropy**, **Characters**, **Worksheets**, **Strings** and **Strings made**.

**Scheme** sets **Length**, **48 characters** or **74 characters**, and **Split**: **2 of 3**, **3 of 5**, **Other** with a **Threshold**, or **None, the secret alone**. **Identifier** is four characters that name the set on every string. **Entropy** is **Roll dice** or **Type the characters**. **Worksheets** has a tab for each sum, read against the paper. **Strings** has each string written down, then typed back; **Skip** goes on without. **Strings made** has **Check on another device** and **Lock in**, which loads the key.
