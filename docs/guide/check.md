# Check

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Check`
**Key:** check

Check lists what you can check a hardware wallet against, one tile for each thing, in groups. A tile opens the tool that does it, and **Back** returns to Check; each tool is also on the Tools page.

## The wallet and signing

**Key:** check.signing

**An address on its screen** opens Check an address. **A transaction before signing** opens Decode a transaction. **What it signed** opens Check a signer, which compares the PSBT you sent with the one that came back. **Its signatures hide nothing** opens Nonce check.

## Setup and backups

**Key:** check.setup

**A seed made from dice rolls** opens New key at its dice source. **A backup in another form** offers **SLIP-39 shares**, **Seed XOR parts**, **SeedQR**, **codex32** and **Krux code**, each opening its tool. These buttons are tagged **Seed**: they work from the seed.

## Messages and keys

**Key:** check.messages

**A signed message** opens Check a signed message. **Child seeds** opens Child seeds and passwords, marked **Seed**. **An xpub's other spellings** opens Convert an xpub.

## Typing a seed here

**Key:** check.seed

A tile tagged **Seed**, **A seed made from dice rolls** and **A backup in another form** take a seed, its shares or its code into this device, to compare what it makes with what the hardware wallet shows. That puts the seed on a second device. It is safe within Faraday's design: Faraday is offline, used in a secure place and powered off after use, and the seed is dropped by **Done**, Cancel, leaving the screen, the idle lock and a lock, unless you press **Keep this key**. Type a seed that holds money only on an offline Faraday; Faraday Demo and the Starter run online and are for test seeds. A match does not show that either device is honest.
