# Seed XOR

## What a part is

Seed XOR splits one key into two, three or four parts. Each part is a full set of seed words of the same length as the original: 24 words split into 24-word parts, 12 into 12-word parts. Combining the parts with the XOR operation gives the original key back.

XOR is addition without carrying, done bit by bit. Because it is its own inverse, the order of the parts does not matter and no part is more important than another. You make all but the last part yourself, from the same sources Create a key offers — dice, coin flips, hex digits, playing cards, camera noise, this device's generator, or a mix of them — and the device works out the last one, so that the parts XOR back to your key. You choose the source because if a random part could be guessed, the last part alone would give the key.

The scheme is Coldcard's. Parts made here combine on a Coldcard, and parts made on a Coldcard combine here.

On this device, a key is split from its Backup menu, under "Split with Seed XOR". Parts are combined by Load a key, under "Seed XOR parts": what combining gives back is a key that already existed.

## Every part is needed

All of the parts are needed to recover the key. Lose one and the key is gone, exactly as if you had lost the words themselves. Splitting into three parts and keeping them in three places makes theft harder and loss easier.

Any part on its own tells you nothing about the key. Two parts of a three-part split tell you nothing either. It is all of them or none.

To check a part later, load it as a key and compare its fingerprint with the one the split screen showed for it. That identifies the part without putting the key back together.

## A part is a real key

Each part is a valid set of seed words, so each part is a working wallet with addresses of its own. Coldcard warns about this and it is worth repeating: if you send coins to an address derived from a part, those coins are held by that part alone, not by your key.

Treat every part with the same care as the original words. A part in a photograph, in cloud storage, or in a drawer with the other parts puts the key at risk.

## This is not Shamir

Shamir's Secret Sharing, which SLIP-39 uses, makes shares with a threshold: any 2 of 3, any 3 of 5. Seed XOR has no threshold. Every part is required.

Seed XOR is far simpler in exchange. A part is ordinary BIP-39 words that any wallet can read, the arithmetic is one XOR that you can do with pencil and paper from the wordlist numbers, and no special software is needed to put the key back together.
