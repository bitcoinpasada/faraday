# Derive with BIP-85

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Bip85`
**Key:** bip85

One card per step: the key, the application, the index and the value. BIP-85 derives a further secret from a loaded key, and the same key, application and index always give the same value.

## Key

**Key:** bip85.key

The choice is the loaded key the value comes from. Each key is listed by its fingerprint and label. The value is derived again whenever it is wanted, so the backup of this key covers every value taken from it.

## Application

**Key:** bip85.app

The choice is what the value is. **Words** is a child seed, 12 to 24 words, a key of its own. **WIF** is a private key and **Extended private key** an xprv. **Hex** is 16 to 64 secret bytes. **Password (base64)** is 20 to 86 characters and **Password (base85)** 10 to 80.

**-** and **+** set the length of Words, Hex and the passwords; WIF and Extended private key have none. **Continue** opens the next card.

## Index

**Key:** bip85.index

The index numbers the values of one application: 0 is the first, each other number gives another. It is typed as digits, or stepped with **-** and **+**, from 0 to 2147483647. The index is all that is needed, with the key, to make the value again.

## Value

**Key:** bip85.result

**Path** is the derivation path of the value, and the line below it is the value, hidden until **Show**; **Hide** covers it again. The value is a secret.

The Save into button, followed by the open vault's name, puts it in that vault: a password as an entry, child words as a key, any other value as a note. With no vault open, **Make a vault** or **Unlock a vault** takes its place. **Load as a key** is offered for Words and loads the child seed as a key of this session. For child words, either asks whether to record them as a child seed of the parent key: **Record** keeps the parent's fingerprint and index, **Don't record** keeps an ordinary seed. **Out unprotected…** writes the value to a file after a warning.
