# The demo and its public seeds

## What the demo is

Demo is a mode of Faraday on a computer that is online: Faraday on testnet with sample keys. The website runs it in a browser and stays in it. Faraday Desktop starts in it, and Mode in Settings or the pill leaves it. Demo opens every screen and works as Faraday does, with keys that hold nothing.

## The public seeds

Public seeds are each one word written over and over; each is a valid BIP-39 phrase. Of 24 words: bacon, flag, gas, great, slice, solution, summer, they, trade, trap and zebra. Of 12 words: twelve, ocean, piano and silver. Of 15 words: lucky and olive. Of 18 words: glad and school. Of 21 words: brave and sunset. Anyone can load them, so anything sent to their addresses can be taken by anyone.

Demo takes only samples, on testnet. Any other key is refused by its fingerprint.

## New key in the demo

New key runs as on the device: the dice, the coins, the cards, hex digits, the camera or the computer's generator make the words, and the quiz checks them. At Lock in, the demo adds the next sample of the length chosen that is not loaded, in place of the key made, and the Key card says which. There are samples of 12, 15, 18, 21 and 24 words: four of 12, two each of 15, 18 and 21, and eleven of 24. A passphrase there is optional: type one or press its faraday chip, and the sample is added with it. The key made is not kept. With every sample of that length loaded, Lock in is dimmed.

Generate with dice on Create a vault runs the same way: the rolls make the words, and the card shows them, then the demo puts `faraday` in place of them. The vault is made under it; the rolled words are not kept. Every other passphrase and password field in the demo is typed as anywhere else, and also offers a faraday chip that puts `faraday` in the field. The KeePass export takes one you type, of 8 characters or more.

## Samples

Load samples is a pill beside the Demo pill. It opens a list of sample wallets and sample keys, each with a tick box. Load ticked puts the ticked ones in as if they had been typed or scanned. A set loads as one key, from as many shares or parts as it needs. A wallet loads with the sample keys it uses. A sample already loaded is ticked and dimmed. Each row says what the sample is under its name: a wallet its kind and script, a key its form and size. In Demo, the sample passphrase is `faraday`.

Every sample is public, and anything sent to a sample's addresses can be taken by anyone.
## Sample wallets

Savings (2 of 3), Spending, Taproot, Nested SegWit, Legacy, Nested multisig, Legacy multisig, Taproot multisig, Inheritance, Taproot tree, MuSig2, FROST 2 of 3 and 3 of 5. All are on testnet. They use the sample keys bacon, zebra and summer, except 3 of 5, whose five keys are master seeds of 32 bytes, each byte 01, 02, 03, 04 and 05, and which loads them with it.

## Other sample keys

Load samples also lists keys of other forms: the all-zero BIP-39 seed (abandon … about), a Japanese BIP-39 phrase, SLIP-39 shares (one share, 2 of 3, and groups), codex32 secrets and shares, and Seed XOR parts.

The LND cipher seed, which Load samples does not list, gives a Lightning node key: above judge emerge veteran reform crunch system all snap please shoulder vault hurt city quarter cover enlist swear success suggest drink wagon enrich body

## Other samples

Where a screen asks for a vault, a signed message or an address, Use a sample puts one in. Vaults with none offers a sample vault, which opens with the faraday chip. The Check a signed message tile in Tools opens a sample message made by sample Spending's first receive address. Find an address takes that address in its field, and finds it once Spending is loaded. The LND cipher seed field takes the sample cipher seed. They are public, so anything sent to that address can be taken by anyone.

## What the demo does not make

The demo makes no secret a person could take away and use. These actions are dimmed and do nothing: SLIP-39 shares, Seed XOR parts and codex32 shares of two or more; a GPG key and Secure Boot keys; every BIP-85 output; a FROST deal; the vanity passphrase dial; a Lightning node's private key shown, saved into a vault or sent out unprotected; and a silent payments scan key saved into a vault or sent out unprotected. A Lightning cipher seed other than the sample is refused. Their screens open, and everything else on them works. A Faraday stick, booted on a computer with no network, makes all of them.

## Test networks

Faraday Desktop starts in Demo. Testnet, signet and regtest, chosen from the mode pill or Settings, take any seed and show real addresses, for testing with coins that have no value, and every action above works. This computer is online, so a seed that holds real bitcoin is never typed into Faraday Desktop, in any mode: every screen that takes a seed or a secret says so under its field. The website offers no test network.

## Typed words are already exposed

The demo refuses to load a seed that is not public, but by then its words have been typed on an online computer, and anything running there may have read them. A seed that holds funds belongs only on Faraday booted from a stick, on a computer with no network.
