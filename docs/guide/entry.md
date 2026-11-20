# Add a key

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Entry`
**Key:** entry
**Words:** seed, key, words, bip39-passphrase, seedqr, share, codex32, seed-xor

Type in a key you have already written down, as BIP-39 words or in another form. It is added to the keys loaded, under its fingerprint.

## BIP-39 words

**Key:** entry.words

Type the words in order, one box each. **Words** sets the count: 12, 15, 18, 21 or 24. The status gives the count, and once the words make a key, **checksum correct** and the key's fingerprint; a word not on the list is named.

**Other languages** opens the other word lists. **BIP-39 passphrase** takes the passphrase the key was made with. The same words with another passphrase are another key, an empty field none. **Scan a SeedQR** fills the words from a code. **Add a key** adds the key; **Clear** empties the boxes; **Make a new key** opens Make a new key. **Other forms**, under Scan, opens the other forms, and **Import a key file** loads one at the pull.

A passphrase field has an eye that shows what is typed. Hardware keyboard keys are read as US; **Add a key** then waits for the eye.

## SLIP-39 shares

**Key:** entry.shares

Type one share at a time, 20 or 33 words, in any order. Every share you have typed is listed, headed by how many are in.

**Add this one** takes the share you typed. **Scan a QR code** takes a share from a code. **Passphrase** takes the passphrase of the backup, if it has one. **Make the key** needs as many shares as the backup's threshold. Fewer rebuild nothing, and shares of different backups are refused.

## Codex32

**Key:** entry.codex32

Type one string at a time, ms1 and all; each is listed. A string with threshold 0 is the key at once, and shares need as many as the threshold says. A codex32 key has no passphrase.

**Make the key** loads the key. **Check a string** checks one string against its checksum and loads nothing; **Back to adding strings** returns. A string that clashes with one already in is shown as **Held** against **This one**, with **Keep the one held** and **Replace it**. With shares in, **Make the rest** makes every string not typed, and **Add a share** makes one more at an index no other string has.

## Seed XOR parts

**Key:** entry.xor

Type one part at a time, as BIP-39 words. Every part is needed, and together they combine into the words of the key.

**Passphrase** takes the BIP-39 passphrase of the key. **Make the key** combines the parts once all are in.

## Krux code

**Key:** entry.krux

A key held as a Krux encrypted code is read here, not typed. **Scan a Krux code** reads it with the camera; **From Files** takes a code saved in Files, one button for each file that may be one. Either asks the code's passphrase, and a wrong one says so and can be tried again. The words the code holds go on to the passphrase of the key, as a scanned SeedQR does. A scan or file that is not a Krux code is said not to be one.
