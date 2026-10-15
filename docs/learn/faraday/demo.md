# The demo and its public seeds

## What the demo is

The demo is Faraday on a computer that is online. The website runs it in a browser and stays on mainnet. A developer's own build of Faraday Desktop starts as the same demo. On mainnet the demo opens every screen and works as Faraday does, with keys that hold nothing.

## The public seeds

Eleven seeds are public. Each is one word written twenty-four times: bacon, flag, gas, great, slice, solution, summer, they, trade, trap and zebra. Each is a valid BIP-39 phrase. Anyone can load them, so anything sent to their addresses can be taken by anyone.

On mainnet the demo takes only these. Any other key is refused by its fingerprint.

## Addresses with a broken checksum

On mainnet every address the demo shows has one character changed, so its checksum fails and no wallet sends to it. A wallet given such an address refuses it. Scripts, transactions, files and signatures keep the real address.

## New key in the demo

New key runs as on the device: the dice, the coins, the cards, hex digits, the camera or the computer's generator make the words, and the quiz checks them. At Lock in, the demo adds the next public seed not loaded in place of the key made, and the Key card says which. A passphrase typed there is ignored. The key made is not kept. With all eleven public seeds loaded, Lock in is dimmed.

Generate with dice on Create a vault runs the same way: the rolls make the words, and the demo puts its public passphrase, brightharborcalmbreezequietshelter, in place of them. The vault is made under the public passphrase; the rolled words are not kept. A passphrase typed by hand is kept as typed.

## What the demo does not make

The demo makes no secret a person could take away and use. These actions are dimmed and do nothing: SLIP-39 shares, Seed XOR parts and codex32 shares of two or more; a GPG key, Secure Boot keys and an APK signing key; every BIP-85 output; a FROST deal; the vanity passphrase dial; a Lightning node's private key shown, saved into a vault or sent out unprotected; a silent payments scan key saved into a vault or sent out unprotected; and typing an LND cipher seed. Their screens open, and everything else on them works. A Faraday stick, booted on a computer with no network, makes all of them.

## Test networks

On testnet, signet and regtest a developer's build of Faraday Desktop takes any seed and shows real addresses, for testing with coins that have no value, and every action above works. Going to one first shows a warning. The website offers no test network.

## Typed words are already exposed

The demo refuses to load a seed that is not public, but by then its words have been typed on an online computer, and anything running there may have read them. A seed that holds funds belongs only on Faraday booted from a stick, on a computer with no network.
