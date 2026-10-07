# Randomness

## Entropy is the whole key

A key is only as strong as the randomness it came from. If the process that produced your seed words was predictable, then someone who knows that process can produce the same words, and there is nothing else protecting the funds. A seed made from a favourite quote, a keyboard pattern, or a "random" phrase you typed yourself will be cracked.

## Dice and coins

Dice and coins are physical randomness that you can watch happen. Each roll or flip is a real event in front of you, you can count them, and no software took part. That is the whole reason OpenSigner asks for dice rolls or coin flips instead of generating a seed for you.

A 6-sided die gives about 2.58 bits per roll, so 50 rolls give 128 bits. A coin gives 1 bit per flip, so 128 flips give 128 bits. Use a real die and a real coin, not a phone app.

A longer key asks for more of both. 12 words take 50 rolls or 128 flips; 15 words take 62 rolls or 160 flips; 18 words take 75 rolls or 192 flips; 21 words take 87 rolls or 224 flips; 24 words take 99 rolls or 256 flips. The screen states the count it is waiting for and counts your entries against it.

While you enter dice rolls, OpenSigner counts how often each face came up and flags long runs and lopsided counts. That check catches a loaded die or a lazy hand. It cannot turn bad rolls into good ones. If it warns you, roll again.

## Computer randomness

Every computer has a random number generator, built from a hardware chip plus the operating system. It is usually fine. The problem is that you cannot see it working. A chip that is faulty, or that was built to be predictable on purpose, produces keys that look perfectly normal and that its maker can recreate. Nobody can tell from the outside.

For this reason OpenSigner lists dice first and marks the device's own generator with the trust it asks for, because you can inspect a die and you cannot inspect a chip.

## From randomness to keys

Your 128 or 256 bits become the seed words. The words are then stretched by an algorithm called PBKDF2 into a 64-byte seed, using the words plus your passphrase (if any) as input. BIP-32 splits that seed into a master private key and a chain code, and every address in every account of the wallet is derived from there.

Nothing further down that chain adds randomness: every key and address in the wallet is computed from the bits you started with.
