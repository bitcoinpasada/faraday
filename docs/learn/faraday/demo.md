# The demo and its public seeds

## What the demo is

Demo is a mode of Faraday on a computer that is online: Faraday on testnet with sample keys. The website runs it in a browser and stays in it. Faraday Desktop starts in it, and Mode in Settings or the pill leaves it. Demo opens every screen and works as Faraday does, with keys that hold nothing.

## The public seeds

Eleven seeds are public. Each is one word written twenty-four times: bacon, flag, gas, great, slice, solution, summer, they, trade, trap and zebra. Each is a valid BIP-39 phrase. Anyone can load them, so anything sent to their addresses can be taken by anyone.

Demo takes only samples, on testnet. Any other key is refused by its fingerprint.

## New key in the demo

New key runs as on the device: the dice, the coins, the cards, hex digits, the camera or the computer's generator make the words, and the quiz checks them. At Lock in, the demo adds the next sample of the length chosen that is not loaded, in place of the key made, and the Key card says which. There are samples of 12, 15, 18, 21 and 24 words: four of 12, two each of 15, 18 and 21, and eleven of 24. A passphrase typed there is ignored. The key made is not kept. With every sample of that length loaded, Lock in is dimmed.

Generate with dice on Create a vault runs the same way: the rolls make the words, and the demo puts its public passphrase, brightharborcalmbreezequietshelter, in place of them. The vault is made under the public passphrase; the rolled words are not kept. A passphrase typed by hand is kept as typed.

## Samples

Load samples is a pill beside the Demo pill. It opens a list of sample wallets and sample keys, each with a tick box. Load ticked puts the ticked ones in as if they had been typed or scanned. A set loads as one key, from as many shares or parts as it needs. A wallet loads with the sample keys it uses. A sample already loaded is ticked and dimmed. Each row says what the sample is under its name: a wallet its kind and script, a key its form and size. In Demo, every passphrase is `faraday`.

Every sample is public, and anything sent to a sample's addresses can be taken by anyone. The words, shares and strings below can be typed where a screen asks for them.

## Sample wallets

Savings (2 of 3), Spending, Taproot, Nested SegWit, Legacy, Nested multisig, Legacy multisig, Taproot multisig, Inheritance, Taproot tree, MuSig2, FROST 2 of 3 and 3 of 5. All are on testnet. They use the sample keys bacon, zebra and summer, except 3 of 5, whose five keys are master seeds of 32 bytes, each byte 01, 02, 03, 04 and 05, and which loads them with it.

## Sample keys of one repeated word

Each is one word written the number of times shown, and each is a valid BIP-39 phrase. Of 24 words: bacon, flag, gas, great, slice, solution, summer, they, trade, trap and zebra. Of 12 words: twelve, ocean, piano and silver. Of 15 words: lucky and olive. Of 18 words: glad and school. Of 21 words: brave and sunset.

## Other sample keys

abandon … about: abandon written 11 times, then about; BIP-39, 12 words, the all-zero seed.

Japanese words: あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あおぞら; BIP-39 Japanese, 12 words.

SLIP-39, one share: duckling enlarge academic academic agency result length solution fridge kidney coal piece deal husband erode duke ajar critical decision keyboard

SLIP-39, 2 of 3, two shares: shadow pistol academic always adequate wildlife fancy gross oasis cylinder mustang wrist rescue view short owner flip making coding armed and shadow pistol academic acid actress prayer class unknown daughter sweater depict flip twice unkind craft early superior advocate guest smoking

SLIP-39, groups, three shares: wildlife deal decision scared acne fatal snake paces obtain election dryer dominant romp tactics railroad marvel trust helpful flip peanut theory theater photo luck install entrance taxi step oven network dictate intimate listen and wildlife deal beard romp alcohol space mild usual clothes union nuclear testify course research heat listen task location thank hospital slice smell failure fawn helpful priest ambition average recover lecture process dough stadium and wildlife deal decision smug ancestor genuine move huge cubic strategy smell game costume extend swimming false desire fake traffic vegan senior twice timber submit leader payroll fraction apart exact forward pulse tidy install

codex32 secret: ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw

codex32, 2 shares: MS12NAMEA320ZYXWVUTSRQPNMLKJHGFEDCAXRPP870HKKQRM and MS12NAMECACDEFGHJKLMNPQRSTUVWXYZ023FTR2GDZMPY6PN

Seed XOR, 2 parts: float vendor rabbit upgrade alter globe right toddler window roof black rule and icon weasel expose grace stand spike plastic high runway betray prevent axis

Encrypted backup: a file, in Load samples only, opened with the passphrase `faraday`; it holds a master seed of 32 bytes, each byte 42.

LND cipher seed: above judge emerge veteran reform crunch system all snap please shoulder vault hurt city quarter cover enlist swear success suggest drink wagon enrich body; it gives a Lightning node key, and Load samples does not list it.

## What the demo does not make

The demo makes no secret a person could take away and use. These actions are dimmed and do nothing: SLIP-39 shares, Seed XOR parts and codex32 shares of two or more; a GPG key and Secure Boot keys; every BIP-85 output; a FROST deal; the vanity passphrase dial; a Lightning node's private key shown, saved into a vault or sent out unprotected; a silent payments scan key saved into a vault or sent out unprotected; and typing an LND cipher seed. Their screens open, and everything else on them works. A Faraday stick, booted on a computer with no network, makes all of them.

## Test networks

Faraday Desktop starts in Demo. Testnet, signet and regtest, chosen from the mode pill or Settings, take any seed and show real addresses, for testing with coins that have no value, and every action above works. This computer is online, so a seed that holds real bitcoin is never typed into Faraday Desktop, in any mode: every screen that takes a seed or a secret says so under its field. The website offers no test network.

## Typed words are already exposed

The demo refuses to load a seed that is not public, but by then its words have been typed on an online computer, and anything running there may have read them. A seed that holds funds belongs only on Faraday booted from a stick, on a computer with no network.
