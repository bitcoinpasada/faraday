# Where randomness comes from

## Every source asks for trust somewhere

Create offers several ways to produce the bits a key is made of. They are not equally good. The difference between them is what you have to trust for the result to be random at all. This page says, for each source, what that is.

None of them can be checked afterwards: a key from a rigged source looks exactly like a key from a fair one. Choosing the source is the only control you have over this.

## Dice and coins

With a die or a coin, the only thing you trust is the object in your hand. You can see every event, count them, and repeat the whole process with a different die. The device only records what you type.

A die gives about 2.58 bits a roll, so 50 rolls give 128 bits and 99 give 256. A coin gives one bit a flip: 128 or 256 of them. The rolls are hashed with SHA-256 and the flips are packed as bits, which is what other signers do, so you can reproduce either result on a computer you trust.

The counts for the three lengths in between follow the same arithmetic: 62 rolls or 160 flips for 15 words, 75 rolls or 192 flips for 18 words, 87 rolls or 224 flips for 21 words.

## Three ways to read the dice

Other signers turn dice into a key in more than one way, and the same rolls give different keys under each. The device asks which one you are following, so that a key you made elsewhere comes out the same here.

**Hashed** is the first row and what most signers do. The rolls are written out as digits and hashed with SHA-256. Coldcard, SeedSigner and EntropyLab's "Base 10" row all do exactly this, so `echo -n 3246115135… | sha256sum` reproduces the result on any computer.

**Six as zero, hashed** writes every 6 as a 0 first and then hashes the same way. Keystone uses it, and so does the "Dice" mode of iancoleman's page. It is no better or worse than the first; it is a different convention, and a key made under one will not appear under the other.

**Words chosen by the dice** does not hash at all. Five rolls of 1 to 4 and a sixth roll read as a coin — 1 to 3 heads, 4 to 6 tails — pick one word out of 2048 directly, because 4 × 4 × 4 × 4 × 4 × 2 is 2048. A 5 or a 6 among the first five is rolled again. This is the BitBox paper table. It costs more rolls: 72 for 12 words against 50, and 144 for 24 against 99. The last word's final rolls only fill bits the checksum replaces, so a device may stop before them, at 70 for 12 words and 140 for 24. What you get for them is a key you can check by hand against a printed wordlist, without a computer anywhere in it.

The dice name every word, the last one too. The last word also carries the checksum, so only its first bits come from the dice — 7 of its 11 at 12 words, 3 at 24 — and the device writes the checksum into the rest. The last word of your key is usually not the word the table names for the last six rolls. SeedSigner completes a last word the same way.

## Playing cards

With a deck you trust the shuffle. A deck shuffled seven times by hand is thoroughly mixed; a deck out of its box is not shuffled at all, and a deck a card trick has been done with may be in an order somebody knows.

Each draw is worth the log of the cards still in the deck: the first is one of 52, the second one of 51. Twenty-five draws carry 128 bits. 256 bits needs 58 draws, which is more than a deck holds, so the device asks for a second, freshly shuffled deck once the first is spent. It refuses a card that is already out, because a card you enter twice adds nothing and usually means you misread one.

Because each draw is worth less than the one before it, the counts climb faster than the lengths do: 25 draws for 12 words, 31 for 15, 39 for 18, 50 for 21 and 58 for 24.

The entropy is SHA-256 over the cards' numbers, in the order you drew them.

## Camera noise

The sensor in a camera produces a slightly different picture every time, even of the same still scene, because of electrical noise in the sensor itself. The device hashes the pixels of the frames you take. It asks for three frames at 12 words and six at 24, and four or five for the lengths in between.

With the camera you trust the sensor and the software that delivers its frames, which you cannot inspect as you can a die. A camera that returns the same frame twice, or a frame it was given rather than one it took, would produce a key somebody else can produce as well, and nothing on screen would look wrong.

The device refuses a frame with fewer than 32 distinct brightness values, which is what a covered lens or a blown-out picture gives. It states the frame's size, its distinct values, its mean and its variance as facts, not as a verdict: those numbers cannot tell you the sensor is honest.

## This device

Every computer has a random number generator, built from a hardware source plus the operating system. It is usually fine, and it is what this device uses to blind its own calculations.

Using it for a key asks you to trust this device completely. A generator that is faulty, or that was built to be predictable, produces keys that look perfectly normal and that its maker can recreate. You can check a die. You cannot check a chip.

That is the whole caution, and it is why the row carries one. In a browser the generator belongs to the browser and to the page, so the row is not offered at all.

## Mixing

A mix runs two or more sources in turn and combines them. The result is at least as good as the best source in it: an attacker who controls the camera but not your dice learns nothing, because the dice are still in the hash.

Before the words appear, the device lists one commitment per source: the SHA-256 of what that source put in. The key is SHA-256 over those commitments, in that order. Write them down first. If you ever want to check that no source was quietly changed after the fact, recompute that one hash and compare.
