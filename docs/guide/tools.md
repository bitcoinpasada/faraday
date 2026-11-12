# Tools

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Tools`
**Key:** tools

Six calculators on one page: each takes what is typed, works one thing out and keeps nothing. An extended private key is refused. **Clear** empties the field.

## Hashes

**Key:** tools.hashes

**Hashes** gives the **Bytes** count, SHA-256, SHA-256d and HASH160 of the field. **Read as: automatic** reads the field as hex when it is hex and as text otherwise; **Text** and **Hex** force one. Text that is not hex, read as hex, reads **Not hex**.

## Encodings

**Key:** tools.encodings

**Encodings** names the **Encoding** of a string (Base58Check, Base58, Bech32, Bech32m or hex), with its **Prefix**, **Version** and **Checksum** (**holds** or **does not hold**) where it has them, and the **Bytes** as hex.

## Descriptor checksum

**Key:** tools.descriptor

**Descriptor checksum** gives the **Checksum** of a descriptor, **Given** with whether a checksum typed with it **holds**, the descriptor **With it**, and whether it is **Wallet**: one this device loads. A key that appears twice is named under **Keys** in the warning tone.

## Convert an xpub

**Key:** tools.convert-key

**Convert an xpub** gives the key as BIP-32 and in the **Legacy**, **Nested SegWit**, **Native SegWit** and **Taproot** spellings, with its **Depth**, **Fingerprint** and **Child**. An extended private key is refused.

## Units

**Key:** tools.units

**Units** gives one amount in sat, BTC, mBTC and bits. The unit the amount is typed in is chosen in the row above the field.

## Miniscript compiler

**Key:** tools.miniscript

**Miniscript compiler** turns a policy into a descriptor under wsh or tr, the choice in the row above. It gives the **Descriptor**, the **Spend paths** count and whether it is a **Wallet** this device loads, or why it is refused. A key may be named by the fingerprint of a loaded key, which stands for that key's account key.
