# Derive with BIP-85

**Kind:** Spec
**Status:** Written from the app's code and strings (`faraday/faraday-core/src/bip85/screen.rs`, `faraday/faraday-core/src/bip85/mod.rs`, `faraday/faraday-core/src/vault/mod.rs`); not yet read beside the running app
**Open:** the pictures, drawn once after the theming has landed
**Screen:** `Bip85`
**Group:** Tools

**Opened from:** **Tools → Child seeds and passwords**, which reads
**Load a key first** and does not open while no key is loaded.

Child seeds, keys and passwords made from a loaded key the way BIP-85
specifies, so the key's backup covers them all. A column of four
cards; with one key loaded it opens on **Application**, that key
chosen, with **Words**, 24 words and index 0.

*Picture to come: `child-seeds.png`, Derive with BIP-85 with the Value card open, the value hidden.*

## On this screen

| Control | What it does |
|---|---|
| the way back | at the top left, named after where the page was opened from |
| **Steps only**, **Guided** | at the top right: the short text on each open card, off or on; on **Key** with a link to Learn's **Seed words** |
| a card | its title and what was chosen; a press opens it again once the cards before it are done |

## The cards

### Key

Closed, the key's fingerprint and name, or **Not chosen**.

| Control | What it does |
|---|---|
| a key's button | one per key loaded, by fingerprint and name; the chosen one filled |

### Application

Closed, the application and its length ("Words · 24 words").

| Control | What it does |
|---|---|
| **Words**, **WIF**, **Extended private key**, **Hex**, **Password (base64)**, **Password (base85)** | what is made. **WIF** and **Extended private key** have no length and move on when pressed |
| **-**, **+** | the length: "24 words" (12 to 24, three at a time), "32 bytes", "21 characters"; the range beside it ("12 to 24") |
| **Continue** | closes the card and opens **Index** |

### Index

Closed, the index.

| Control | What it does |
|---|---|
| **-**, the index, **+** | the index; digits typed go into it (**Type the digits · Enter continues**) |
| **An index is 0 to 2147483647** | in red, while the number typed is past it; the card does not close |
| **Continue** | closes the card and opens **Value** |

### Value

Closed, the path.

| Control | What it does |
|---|---|
| **Path** | the path BIP-85 derives it at, "m/83696968'/39'/0'/24'/0'" |
| the application's name | the value, hidden as dots until **Show** |
| **Show**, **Hide** | the value on screen, or hidden; it is hidden again once the card is left |
| "Save into Vault 1" | into the open vault: a password as an entry, child words as a key, anything else as a note, each named "BIP-85 Words · 3dca6805 · index 0" |
| **Make a vault**, "Unlock vault.ofv", **Unlock a vault** | in its place while no vault is open: makes or unlocks one, then comes back here |
| **Load as a key** | for **Words**: loads the child words as a key of this session ("Key … added") |
| **Out unprotected…** | the value to the stick unprotected, through [This is a secret](files.md#this-is-a-secret) |

In Faraday Demo the value is not made: it stays dots.

## On a narrow window

The top bar holds the way back and the name, **BIP-85**; the value is
under its name, and the length's **+** at the row's end.

## Why

- [Seed words](../../learn/02-words.md)
- [Passphrases](../../learn/05-passphrases.md)
- [Glossary](../../learn/15-glossary.md)
