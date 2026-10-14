# The demo and its public seeds

## What the demo is

Faraday Desktop runs on a computer that is online. On mainnet it is a demo: it works as Faraday does, with keys that hold nothing. The website runs the same demo in a browser, and stays on mainnet.

## The public seeds

Eleven seeds are public. Each is one word written twenty-four times: bacon, flag, gas, great, slice, solution, summer, they, trade, trap and zebra. Each is a valid BIP-39 phrase. Anyone can load them, so anything sent to their addresses can be taken by anyone.

On mainnet the demo takes only these. Any other key is refused by its fingerprint.

## Addresses with a broken checksum

On mainnet every address the demo shows has one character changed, so its checksum fails and no wallet sends to it. A wallet given such an address refuses it. Scripts, transactions, files and signatures keep the real address.

## New key in the demo

New key runs as on the device: the dice, the coins or the computer's generator make the words, and the quiz checks them. At Lock in, the demo adds the next public seed not loaded in place of the key made, and the Key card says which. A passphrase typed there is ignored. The key made is not kept.

## Test networks

On testnet, signet and regtest Faraday Desktop takes any seed and shows real addresses, for testing with coins that have no value. Going to one first shows a warning. The website offers no test network.

## Typed words are already exposed

The demo refuses to load a seed that is not public, but by then its words have been typed on an online computer, and anything running there may have read them. A seed that holds funds belongs only on Faraday booted from a stick, on a computer with no network.
