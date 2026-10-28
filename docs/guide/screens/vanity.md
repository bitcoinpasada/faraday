# Vanity address

**Kind:** Spec
**Status:** Written from the app's code and strings (`faraday/faraday-core/src/vanity/screen.rs`, `faraday/faraday-core/src/vanity/mod.rs`); not yet read beside the running app
**Open:** the pictures, drawn once after the theming has landed
**Screen:** `Vanity`
**Group:** Tools

**Opened from:** **Tools → Vanity address**, which reads **Load a key
first** and does not open while no key is loaded.

A search for a first receive address that begins with characters of
your choice, by turning a loaded key's account number or adding to its
passphrase. A column of five cards; with one key loaded it opens on
**What turns**, that key chosen, with **Account number** and Native
SegWit.

*Picture to come: `vanity.png`, Vanity address with the Search card open, an address found at an account of key 3dca6805.*

## On this screen

| Control | What it does |
|---|---|
| the way back | at the top left, named after where the page was opened from |
| **Steps only**, **Guided** | at the top right: the short text on each open card, off or on; on **Key** and **What turns** with a link to Learn's **Xpubs and privacy** |
| a card | its title and what was chosen; a press opens it again once the cards before it are done |

## The cards

### Key

Closed, the key's fingerprint, or **Not chosen**.

| Control | What it does |
|---|---|
| a key's row | a box, its fingerprint and its name; a press chooses it and opens **What turns** |

### What turns

Closed, the dial chosen.

| Control | What it does |
|---|---|
| **Account number** | **The same words and passphrase, a different account: fast. …** |
| **Passphrase** | **A few characters added to the key's BIP-39 passphrase: slow, one PBKDF2 a try. …**; grey for a key not loaded from BIP-39 words (**This key was not loaded from BIP-39 words: only its account turns**) and in Faraday Demo |

A press chooses one and moves on.

### Kind of address

Closed, the kind and what every address of it begins with.

| Control | What it does |
|---|---|
| "Legacy · 1…", "Nested SegWit · 3…", "Native SegWit · bc1q…", "Taproot · bc1p…" | the kind of address searched; on a test network legacy reads "Legacy · m… or n…". A press chooses one and moves on |

### Characters

Closed, the characters typed.

| Control | What it does |
|---|---|
| the field | the part every address of the kind shares, greyed, then the characters typed |
| "About 1,024 tries" | how many tries a match takes, on average |
| **Continue** | closes the card and opens **Search**; **Type at least one character after the fixed part** in red while none is typed. A character that cannot stand there says so ("… cannot stand there in this kind of address") |

### Search

Closed, the address found, or "12,345 tried" while it runs.

| Control | What it does |
|---|---|
| "Search for bc1qfa" | starts the search; it runs while this screen is open and stops at the first match |
| "Searching for bc1qfa" | while it runs, with "12,345 tried · 2,000 a second · about 1 s expected" (**Measuring** at first) |
| **Stop** | stops it |
| **Every counter was tried: no address begins that way** | in red, where the search ran out |
| **Found** | the address, in the ok tone |
| "Account 1017 of this key, after 1,018 tries" | for **Account number** |
| **Added to the passphrase:** | for **Passphrase**: the characters, hidden until **Show** (**Hide**), and in the warning tone **These characters are part of the passphrase: write them down with it. …** |
| **Open this wallet** | adds a single-key wallet at the account found, or, for **Passphrase**, loads the key with the characters added ("… · vanity passphrase") and its wallet, and opens it |
| **Search again** | starts the search again from its first try |

## On a narrow window

The top bar holds the way back and the name, **Vanity address**.

## Why

- [Passphrases](../../learn/05-passphrases.md)
- [Kinds of wallets](../../learn/17-wallet-kinds.md)
