# Randomness

## Every source asks for trust somewhere

A key is only as strong as the randomness it came from. If the process that made your seed words was predictable, whoever knows that process can make the same words, and nothing else protects the funds. A seed made from a favourite quote, a keyboard pattern or a phrase you typed yourself will be cracked.

New key offers several ways to produce the bits a key is made of: dice or coins, playing cards, hex digits, this device's generator, and a mix of several, camera noise among them. They are not equally good. The difference is what you have to trust for the result to be random at all. This page says, for each source, what that is.

None of them can be checked afterwards: a key from a rigged source looks exactly like a key from a fair one. Choosing the source is the only control you have over this. Nothing after the entropy adds randomness: the seed, the master key and every address are computed from the bits you started with.

## Dice and coins

With a die or a coin, the only thing you trust is the object in your hand. You can see every event, count them, and repeat the whole process with a different die. The device only records what you type. Use a real die and a real coin, not a phone app.

A die gives about 2.58 bits a roll and a coin one bit a flip. For 12, 15, 18, 21 and 24 words the screen asks for 50, 62, 75, 87 and 99 rolls, or 128, 160, 192, 224 and 256 flips, and counts your entries against the number. The rolls are hashed with SHA-256 and the flips are packed as bits, which is what other signers do, so you can reproduce either result on a computer you trust.

A die can also stand in for a coin: each roll is one flip, 1, 2 or 3 for tails and 4, 5 or 6 for heads.

While you enter rolls or flips, Faraday counts how often each face came up and flags long runs, sequential runs such as 1-2-3-4-5-6, and lopsided counts. That check catches a loaded die or a lazy hand. It cannot turn bad rolls into good ones. If it warns you, roll again.

## Three ways to read the dice

Other signers turn dice into a key in more than one way, and the same rolls give different keys under each. The device asks which one you are following, so that a key you made elsewhere comes out the same here.

Hashed is what most signers do, including Coldcard and SeedSigner. The rolls are written out as digits and hashed with SHA-256, so `echo -n <rolls> | sha256sum` reproduces the result on any computer.

Six as zero, hashed writes every 6 as a 0 first and then hashes the same way. Keystone uses it. It is a different convention, not a better one, and a key made under one does not appear under the other.

Words chosen by the dice does not hash at all. Five rolls of 1 to 4 and a sixth roll read as a coin pick one word out of 2048 directly, because 4 × 4 × 4 × 4 × 4 × 2 is 2048. A 5 or a 6 among the first five is rolled again. This is the BitBox paper table. It costs more rolls: Faraday asks for 72 for 12 words and 144 for 24, against 50 and 99. The result can be checked by hand against a printed wordlist, with no computer in it.

The last word also carries the checksum, so only its first bits come from the dice, and the device writes the checksum into the rest. The last word of your key is usually not the word the table names for the last rolls.

## Playing cards

With a deck you trust the shuffle. A deck out of its box is not shuffled at all, and a deck a card trick has been done with may be in an order somebody knows. The count below assumes a well-shuffled deck, about seven riffle shuffles; the device cautions when five cards in a row come in the deck's own order, counting up or down, as a deck out of its box does.

Each draw is worth the log of the cards still in the deck: the first is one of 52, the second one of 51. The counts climb faster than the lengths do: 25 draws for 12 words, 31 for 15, 39 for 18, 50 for 21 and 58 for 24. More than 52 needs a second, freshly shuffled deck, which the device asks for. It refuses a card that is already out, because a card entered twice adds nothing and usually means a misread one.

The entropy is SHA-256 over the cards' numbers, in the order you drew them. A card's number is its suit times 13 plus its rank, the suits in the order spades, hearts, diamonds, clubs and the ranks ace to king, counting from 0. This numbering is Faraday's own: another wallet's card mode may make a different key from the same cards.

## Camera noise

The sensor in a camera produces a slightly different picture every time, even of the same still scene, because of electrical noise in the sensor itself. The device hashes the pixels of the frames you take: three frames for 12 words, four for 15, five for 18 and six for 21 and 24. Camera noise is offered only inside a mix or added to the device's own randomness, never as a key's only source.

With the camera you trust the sensor and the software that delivers its frames, which you cannot inspect as you can a die. A frame the camera was given rather than one it took would produce a key somebody else can produce as well, and nothing on screen would look wrong.

The device refuses a frame with fewer than 32 distinct brightness values, which is what a covered lens or a blown-out picture gives, and a frame identical to one already taken. It states the frame's size, distinct values, mean and variance as facts, not as a verdict.

## Hex

A key's entropy is sometimes written down as hex digits rather than words. New key takes hex digits made elsewhere as its source: the digits become the key as they are, so they must already be random.

## This device

Every computer has a random number generator, built from a hardware source plus the operating system. It is usually fine. Using it for a key asks you to trust this device completely: a generator that is faulty, or built to be predictable, produces keys that look perfectly normal and that its maker can recreate. You can check a die. You cannot check a chip.

The row names what the device reads. The stick and the Pi read /dev/random, the Linux kernel's generator, which mixes the board's hardware generator with its other sources. On the Pi the board's hardware generator feeds the kernel; the device does not read it directly, so a faulty chip is one source among several. If the read fails, the device makes no key and says so on Home.

Faraday Demo and the Starter read the operating system's generator: getrandom(2) on Linux, ProcessPrng on Windows and getentropy(2) on a Mac. Each waits until the generator is ready.

The same generator also seeds everything else the device draws at random, whichever source made the key: a vault's keys and seal nonces, GPG and Secure Boot keys, MuSig2 nonces, a new keyboard's code and the blinding of the device's own calculations. In the website demo, the key made is replaced by one of its public demo keys, whichever source made it.

## Mixing

A mix runs two to five sources in turn and combines them, dice or this device among them. The result is at least as good as the best source in it: an attacker who controls the camera but not your dice learns nothing, because the dice are still in the hash.

Before the words appear, the device lists one commitment per source: the SHA-256 of what that source put in. The key is SHA-256 over those commitments, in that order. Write them down first. If you ever want to check that no source was quietly changed after the fact, recompute that one hash and compare.
