# Vanity address

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Vanity`
**Key:** vanity

Searches for a first receive address of a loaded key that begins with chosen characters, by trying one account number or one passphrase suffix after another. A find is a fact about the key and the number: any device that turns the same dial gives the same address.

## Key

**Key:** vanity.key

One row per loaded key, by fingerprint and name. The key itself does not change.

## What turns

**Key:** vanity.dial

**Account number** keeps the key's words and passphrase and tries other accounts; it is fast, and the wallet found is the key's own, at that account. **Passphrase** adds a few characters to the key's BIP-39 passphrase; it is slow, and the words with that passphrase are another wallet that needs the passphrase kept with them. Only a key loaded from BIP-39 words can turn the passphrase, and Practice does not offer it.

## Kind of address

**Key:** vanity.script

**Legacy**, **Nested SegWit**, **Native SegWit**, **Taproot** and **Silent payment code** each show the characters every address of the kind begins with. A character that cannot stand in that place in that kind is refused.

## Characters

**Key:** vanity.prefix

The characters are typed after the fixed part. An estimate of tries (About, then a count) is shown; each character more takes 32 times as long for a bech32 address and 58 times for a legacy one. At least one character is needed. **Continue** opens the search.

## Search

**Key:** vanity.run

Search for followed by the characters starts the search. While it runs, the tries so far, the rate and the expected time show, and **Stop** ends it.

A find shows **Found**, the address, and for the account dial the account number and the tries it took. For the passphrase dial it shows **Added to the passphrase:** with the characters hidden until **Show**; these are part of the passphrase and are written down with it. Pressing the address opens its code, unless the key waits for its backup.

**Open this wallet** opens the wallet at the account found, and **Open in Silent payments** opens a silent payment code found on the passphrase dial; an account found for a silent payment code has no wallet to open. **Search again** starts over. When every counter has been tried with no match, the search says so.
