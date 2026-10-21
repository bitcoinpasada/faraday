# Where keys are kept

## In memory only

A key loaded in Faraday is in the computer's memory and nowhere else. Faraday writes no key to the stick, the SD card or the computer's disk on its own. A lock wipes every key and wallet from memory, and so does turning the computer off.

The lock and power-off sheets list what is wiped and, for each key and wallet, where else it is kept. A key kept nowhere else is listed first, in red.

## What is written

A key is written only in two ways. Sealed in a vault, it is encrypted under the vault's passphrase. Saved as a file, its words or SeedQR picture are written unprotected, and only after a tick that says anyone who copies the file can spend with it.

What Faraday keeps across a lock, until the computer is turned off, holds no secret: each vault's name, its wallets' names and shapes, and its keys' fingerprints.

## Vaults

A vault is one encrypted file on a stick. Unlocking it with its passphrase loads what it holds back into memory. The Vaults page has the rest.

## The idle lock

Faraday locks after 30 minutes without input, and turns the computer off after 60. Settings › Lock after and Power off after change the two times. It does not turn the computer off while files wait to be written to a stick.

From Make a new wallet begun until its keys are sealed in their vaults, or its backup is done, the idle lock does not come, and the sidebar says "Auto-lock off until backup complete". Until then the keys are kept nowhere but in memory. Lock still works, asks first, and ends Make a new wallet.

## No PIN

Faraday has no PIN. Nothing stays on the device between sessions for a PIN to guard: after a lock or a restart, keys come back from a vault and its passphrase, or from the words.
