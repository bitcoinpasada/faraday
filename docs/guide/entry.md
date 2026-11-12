# Add a key

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Entry`
**Key:** entry

A key already written down is typed in here, as BIP-39 words or in another form. The key is added to the keys loaded, under its fingerprint.

## BIP-39 words

**Key:** entry.words

The words are typed in order, one box each. **Words** sets the count: 12, 15, 18, 21 or 24. The status gives the count, and once the words make a key, **checksum correct** and the key's fingerprint; a word not on the list is named.

**Other languages** opens the other word lists. **BIP-39 passphrase** takes the passphrase the key was made with; the same words with another passphrase are another key, and the field left empty means none. **Scan a SeedQR** fills the words from a code. **Add key** adds the key; **Clear** empties the boxes; **Make a new key** opens New key. **Other forms** opens the other forms, and **Copy files in**, with a stick attached, opens Files.

## SLIP-39 shares

**Key:** entry.shares

One share is typed at a time, 20 or 33 words, in any order. Every share typed is listed, headed by how many are in.

**Add this one** takes the share typed. **Scan a QR code** takes a share from a code. **Passphrase** takes the passphrase of the backup, if it has one. **Make the key** needs as many shares as the backup's threshold; fewer rebuild nothing, and shares of different backups are refused.

## Codex32

**Key:** entry.codex32

One string is typed at a time, ms1 and all, and each is listed. A string with threshold 0 is the key at once; shares need as many as the threshold says. A codex32 key has no passphrase.

**Make the key** loads the key. **Check a string** checks one string against its checksum and loads nothing; **Back to adding strings** returns. A string that clashes with one already in is shown as **Held** against **This one**, with **Keep the one held** and **Replace it**. With shares in, **Make the rest** makes every string not typed, and **Add a share** makes one more at an index no other string has.

## Seed XOR parts

**Key:** entry.xor

One part is typed at a time, as BIP-39 words. Every part is needed, and they combine into the words of the key.

**Passphrase** takes the BIP-39 passphrase of the key. **Make the key** combines the parts once all are in.
