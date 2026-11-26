# Tools

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Tools`
**Key:** tools
**Words:** descriptor, fingerprint, xpub

Tools holds six calculators on one page: each takes what you type, works one thing out and keeps nothing. An extended private key is refused, and **Clear** empties the field.

## Hashes

**Key:** tools.hashes

**Hashes** gives the **Bytes** count, SHA-256, SHA-256d and HASH160 of the field. **Read as: automatic** reads the field as hex when it is hex and as text otherwise; **Text** and **Hex** force one. Text that is not hex, read as hex, reads **Not hex**.

## Encodings

**Key:** tools.encodings

**Encodings** names the **Encoding** of a string (Base58Check, Base58, Bech32, Bech32m or hex), with its **Prefix**, **Version** and **Checksum** (**holds** or **does not hold**) where it has them, and the **Bytes** as hex.

## Descriptor checksum

**Key:** tools.descriptor

**Descriptor checksum** reads a descriptor from the field, **Scan** or **From Files**, giving the **Checksum** of a descriptor, **Given** with whether a checksum typed with it **holds**, the descriptor **With it**, and whether it is **Wallet**: one this device loads. **Take to Wallets** then puts it in Wallets after the wallet is confirmed. A key that appears twice is named under **Keys** in the warning tone. On a wide window with a pane at the left, the pane holds the ways in to this field, to Convert an xpub's and to the Miniscript compiler's: the held files the field takes, **Type it** and **Scan**.

## Convert an xpub

**Key:** tools.convert-key

**Convert an xpub** reads a key typed, scanned with **Scan** or read from **From Files**, and gives the key as BIP-32 and in the **Legacy**, **Nested SegWit**, **Native SegWit** and **Taproot** spellings, with its **Depth**, **Fingerprint** and **Child**. An extended private key is refused.

## Units

**Key:** tools.units

**Units** gives one amount in sat, BTC, mBTC and bits. Choose the unit you are typing in from the row above the field.

## Miniscript compiler

**Key:** tools.miniscript

**Miniscript compiler** takes a policy typed, scanned with **Scan** or read from a file with **From Files**, and turns it into a descriptor under wsh or tr, chosen in the row above. It gives the **Descriptor**, the **Spend paths** count and whether it is a **Wallet** this device loads, or why it is refused. A key may be named by the fingerprint of a loaded key, which stands for that key's account key.
