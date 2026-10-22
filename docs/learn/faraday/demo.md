# The demo and its public seeds

## What the demo is

Demo is a mode of Faraday on a computer that is online: Faraday on testnet with sample keys. The website runs it in a browser and stays in it. Faraday Desktop starts in it, and Mode in Settings or the pill leaves it. Demo opens every screen and works as Faraday does, with keys that hold nothing.

## The public seeds

Eleven seeds are public. Each is one word written twenty-four times: bacon, flag, gas, great, slice, solution, summer, they, trade, trap and zebra. Each is a valid BIP-39 phrase. Anyone can load them, so anything sent to their addresses can be taken by anyone.

Demo takes only samples, on testnet. Any other key is refused by its fingerprint.

## New key in the demo

New key runs as on the device: the dice, the coins, the cards, hex digits, the camera or the computer's generator make the words, and the quiz checks them. At Lock in, the demo adds the next sample of the length chosen that is not loaded, in place of the key made, and the Key card says which. There are samples of 12, 15, 18, 21 and 24 words: four of 12, two each of 15, 18 and 21, and eleven of 24. A passphrase typed there is ignored. The key made is not kept. With every sample of that length loaded, Lock in is dimmed.

Generate with dice on Create a vault runs the same way: the rolls make the words, and the demo puts its public passphrase, brightharborcalmbreezequietshelter, in place of them. The vault is made under the public passphrase; the rolled words are not kept. A passphrase typed by hand is kept as typed.

## What the demo does not make

The demo makes no secret a person could take away and use. These actions are dimmed and do nothing: SLIP-39 shares, Seed XOR parts and codex32 shares of two or more; a GPG key and Secure Boot keys; every BIP-85 output; a FROST deal; the vanity passphrase dial; a Lightning node's private key shown, saved into a vault or sent out unprotected; a silent payments scan key saved into a vault or sent out unprotected; and typing an LND cipher seed. Their screens open, and everything else on them works. A Faraday stick, booted on a computer with no network, makes all of them.

## Test networks

Faraday Desktop starts in Demo. Testnet, signet and regtest, chosen from the mode pill or Settings, take any seed and show real addresses, for testing with coins that have no value, and every action above works. This computer is online, so a seed that holds real bitcoin is never typed into Faraday Desktop, in any mode: every screen that takes a seed or a secret says so under its field. The website offers no test network.

## Typed words are already exposed

The demo refuses to load a seed that is not public, but by then its words have been typed on an online computer, and anything running there may have read them. A seed that holds funds belongs only on Faraday booted from a stick, on a computer with no network.
