# Convert a seed

**Kind:** Reference
**Status:** Checked against the app's code and strings (`faraday/faraday-core/src/convert/screen.rs`, `faraday/faraday-core/src/convert/mod.rs`); no picture yet
**Screen:** `Convert`
**Group:** Tools

**Opened from:** **Tools → Convert a seed**; **Write it in another form**
on a key's sheet and in [Explore](explore.md), with that key chosen.

A loaded key written in another standard, the same wallet. A value is
given only when it reads back as the key's wallet. A column of three
cards: **From**, **To** and **Result**.

## The cards

### From

Closed, the key chosen, or **Not chosen**.

| Control | What it does |
|---|---|
| a key's button | one per key loaded, with what it was made from: "BIP-39 words, 12", "codex32, 48 characters" or **Master seed** |
| a vault's key | a key in an open vault, **In vault 1**; a press loads it |
| **Add a key** | [Add a key](add-a-key.md) |
| **An LND aezeed** | the 24 words and passphrase of an LND aezeed, read and held for this flow and not loaded as a key; once read, its button is **LND aezeed, 24 words** |

### To

The forms this key can be written in. What is not offered stays on the
card, dimmed, with its reason.

| Form | Restores in |
|---|---|
| **Master seed, hex** | any BIP-32 library and Faraday; no common wallet takes a raw seed |
| **xprv**, or **tprv** on a test network | Sparrow, Bitcoin Core in a descriptor, Faraday |
| **The words**, a key of words only | every BIP-39 wallet |
| **SeedQR**, **CompactSeedQR**, a key of 12 or 24 English words | SeedSigner, Krux, Faraday |
| **Hex entropy**, a key of words only | Ian Coleman's BIP-39 page, Faraday's New key |
| **Word numbers**, a key of words only | any BIP-39 wallet, once turned back into words by the list |
| **4-letter starts**, a key of English words | a wallet that completes a word from its start, and any, written out in full |
| **Seed XOR parts**, a key of English words only | Coldcard and Faraday; each part is itself valid BIP-39 words |
| **SLIP-39, 1 of 1**, a key of seed bytes of 16 or 32 bytes | Trezor and Faraday; no settings, the same words every time |
| **SLIP-39, k of n**, a key of seed bytes of 16 or 32 bytes | Trezor and Faraday; one group of k of n shares, or several groups; fresh randomness |
| **codex32**, a key of seed bytes | Faraday and any codex32 tool |

A key of 64 bytes of seed lists **SLIP-39** dimmed, with "SLIP-39
backups of 64 bytes are read by no wallet"; of 20, 24 or 28 bytes it has
no SLIP-39 row. In Practice, Seed XOR and SLIP-39, k of n are dimmed and codex32's secret goes
straight to Result.

### Settings

Between To and Result, for Seed XOR, SLIP-39, k of n and codex32.

| Control | What it does |
|---|---|
| **Parts** | a Stepper, 2 to 4, for Seed XOR |
| **Shares needed (0: the secret alone)** | codex32's k, the shares needed: 0 (the secret alone, and a minus there stays at 0), then 2 to 9; each set of 2 or more takes a new identifier |
| **Shares made** | codex32's n, the shares made, k to 31; none while k is 0 |
| **Groups** | SLIP-39's groups, 1 to 16; a plus that would leave a shape the system's randomness one press can be given cannot pay for shows disabled and does nothing |
| **Groups needed** | with several groups, how many are needed, 2 up to the groups |
| **Shares needed**, **Shares made** | with one group, its k and n, each a Stepper, 1 to 16; a threshold of 1 only for one share |
| **Group 1 needs**, **Group 1 shares** | with several groups, the same for each group, by its number |
| **Fresh randomness** | shown, with the value Faraday's, where the set draws randomness |
| **Make** | works out the set; with no randomness ready a toast says so and the card stays |

### Result

The value is hidden until it is shown, and dropped when the card is left.

| Line or control | What it does |
|---|---|
| the value, or **Show the code** | shows the text, or the code on a SeedQR form; a set has one eye over all its parts, numbered (the codex32 secret alone is not), several SLIP-39 groups as **Group 1, share 2:** |
| **Standard**, **Restores in**, **Fingerprint** | what the value is, where it restores, and the key's fingerprint, the same as the source (an aezeed's is that of its seed, with no source to match) |
| **Word list** | on a form with no list of its own, for a key on another list than English |
| **Paths** | the paths of the loaded wallets holding this key, or **No loaded wallet holds this key**; for an aezeed, the paths LND uses |
| **Birthday**, **Channels** | for an aezeed: the date its wallet was made, and that it restores on-chain coins only |
| **Passphrase** | on a key with a passphrase: whether the value holds the passphrase's wallet or needs the passphrase to restore it |
| **Check my copy** | a copy typed back: **Compare** says **Your copy matches** or **Your copy differs**, **Clear** empties it; on a set it is compared with every part and says which part matched, or for groups **Your copy matches group 1, share 2** |
| **Save as a file…** | the value to a stick (a set: every part, one per line, in group order), through [This is a secret](files.md#this-is-a-secret) |
| **Done** | back where the screen was opened from |
