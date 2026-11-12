# Convert a seed

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Convert`
**Key:** convert

One card per step: the source, the form to write it in, that form's settings where it has any, and the result. A key held on this device is written in another standard; it stays the same wallet, and each result is read back before it is shown.

## From

**Key:** convert.from

The choice is the key to convert, listed by fingerprint and label: the loaded keys, and the keys of an open vault. Choosing a vault's key loads it first.

**Add a key** opens New key. **An LND aezeed** reads 24 words and a passphrase for this tool and holds them only until it is left. **A SLIP-39 set** takes shares typed one at a time with **Type one share** and **Use this share**; it shows each group as **Enough** or **Not enough yet**, and **Start again** forgets the set. It is not offered in Practice.

## To

**Key:** convert.to

The choice is the form to write the key in. Each row names its standard; only the forms that fit the key are listed.

**The words**, **SeedQR**, **CompactSeedQR**, **Hex entropy**, **Word numbers** and **4-letter starts** need a key with words; SeedQR and CompactSeedQR need 12 or 24 English words, and 4-letter starts the English list. **Seed XOR parts** needs English words. **SLIP-39, 1 of 1** and **SLIP-39, k of n** need a 16 or 32 byte seed. **codex32** takes any key with a seed. **Master seed, hex** and **xprv** (**tprv** on a test network) fit every key. Rows that cannot be made say why, such as BIP-39 words from a seed.

For a set source the rows are **Add a share**, **Replace a lost share** and **Rotate the set**; one that needs more shares typed says how many.

## Settings

**Key:** convert.settings

Shown for Seed XOR parts, codex32, SLIP-39 sets and the actions on a set. **Make** makes the result.

**Parts** is 2 to 4. For codex32, **Shares needed (0: the secret alone)** is 0 or 2 to 9 and **Shares made** is up to 31. For SLIP-39 sets, **Groups**, **Groups needed**, and per group **Shares needed** and **Shares made**. A form that uses randomness says **Fresh randomness**; it is not offered in Practice. For a set, **Group** and **Share number** pick the share to add or replace; Rotating asks how many shares of each group were lost and which shares are **Destroyed**.

## Result

**Key:** convert.result

The value is hidden until shown with the eye. **Standard** names the form and **Restores in** the software that reads it. **Fingerprint** is the source's, so the same wallet can be told. **Paths** lists the loaded wallets that use the key.

**Passphrase** says whether the value holds the passphrase's wallet or must be restored with the passphrase. **Check my copy** compares a typed copy with **Compare**, or a SeedQR scanned back, and says **Your copy matches** or **Your copy differs**. **Save as a file…** offers the value on the secret sheet, which warns before it leaves. **Done** leaves the tool.
