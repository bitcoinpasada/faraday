# Vanity address

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Vanity`
**Key:** vanity

Vanity address searches for a first receive address of a loaded key that begins with characters you choose, by trying one account number or one passphrase suffix after another. A find belongs to the key and the number: any device that turns the same dial gives the same address.

## Key

**Key:** vanity.key

Choose which loaded key to search with; there is one row per key, by fingerprint and name. The key itself does not change.

## What turns

**Key:** vanity.dial

This chooses what the search changes. **Account number** keeps the key's words and passphrase and tries other accounts; it is fast, and the wallet found is the key's own, at that account. **Passphrase** adds a few characters to the key's BIP-39 passphrase; it is slow, and the words with that passphrase make another wallet, so the passphrase must be kept with them. Only a key loaded from BIP-39 words can change the passphrase, and Practice does not offer it.

## Kind of address

**Key:** vanity.script

**Legacy**, **Nested SegWit**, **Native SegWit**, **Taproot** and **Silent payment code** each show the characters every address of that kind begins with. A character that cannot appear in that place in that kind is refused.

## Characters

**Key:** vanity.prefix

Type the characters you want after the fixed part. An estimate of tries (About, then a count) is shown; each extra character takes 32 times as long for a bech32 address and 58 times as long for a legacy one. You need at least one character. Search for followed by the characters starts the search.

## Search

**Key:** vanity.run

Search for followed by the characters starts the search. While it runs, the tries so far, the rate and the expected time show, and **Stop** ends it.

A find shows **Found**, the address, and for the account dial the account number and the tries it took. For the passphrase dial it shows **Added to the passphrase:** with the characters hidden until **Show**; write them down with the passphrase. Pressing the address opens its code, unless the key waits for its backup.

**Open this wallet** opens the wallet at the account found, and **Open in Silent payments** opens a silent payment code found on the passphrase dial; an account found for a silent payment code has no wallet. **Another** starts the tool again, empty, on the same key; **Done** goes back to where the tool was opened. A search with no match says so.
