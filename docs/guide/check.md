# Check

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Check`
**Key:** check
**Words:** key, seed, seedqr, share, seed-xor, codex32

Check lists what you can check a signer against, one tile for each thing, in groups. A tile opens the tool that does it, and **Back** returns to Check; each tool is also on the Tools page.

## The wallet and signing

**Key:** check.signing

**Check an address**, **Decode a transaction**, **Check a signer** and **Nonce check** each open the tool of that name. Check a signer compares the PSBT you sent with the one that came back.

## Setup and backups

**Key:** check.setup

**Check a seed made from dice** opens Make a new key at its dice source. **Check a backup in another form** offers **SeedQR**, **codex32** and **Other forms** (**SLIP-39 shares**, **Seed XOR parts**, **Krux code**), each opening its tool. **Krux code** opens Add a key to read a code. These buttons are tagged **Seed**: they work from the seed.

## Messages and keys

**Key:** check.messages

**Check a signed message** opens the tool of that name. **Child seeds and passwords** opens it, marked **Seed**. **Convert an xpub** opens the calculator of that name.

## Typing a seed here

**Key:** check.seed

A tile tagged **Seed**, **A seed made from dice rolls** and **A backup in another form** take a seed, its shares or its code into this device, to compare what it makes with what the signer shows. That puts the seed on a second device. It is safe within Faraday's design: Faraday is offline, used in a secure place and powered off after use, and the seed is dropped by **Done**, Cancel, leaving the screen, the idle lock and a lock, unless you press **Keep this key**. Type a seed that holds money only on an offline Faraday; Faraday Demo and the Starter run online and are for test seeds. A match does not show that either device is honest.
