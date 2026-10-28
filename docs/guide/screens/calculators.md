# Tools: the calculators

**Kind:** Spec
**Status:** Written from the app's code and strings (`faraday/faraday-core/src/tools/screen.rs`, `faraday/faraday-core/src/tools/mod.rs`, `faraday/faraday-core/src/catalog.rs`); not yet read beside the running app
**Open:** the pictures, drawn once after the theming has landed
**Screen:** `Tools`
**Group:** Tools

**Opened from:** a tile under **Calculators** on [Tools](tools.md):
**Hashes**, **Encodings**, **Descriptor checksum**, **Convert an xpub**,
**Units** or **Miniscript compiler**, each opening the page on that
calculator.

The page's title is **Tools**, like the page of tiles it is opened
from. Each calculator takes what is typed, works one thing out and
keeps nothing; none of them touches a key.

*Picture to come: `calculators.png`, Tools on Hashes, abc typed, its three hashes.*

## On this screen

| Control | What it does |
|---|---|
| the way back | at the top left, named after where the page was opened from (**Tools**); <kbd>Escape</kbd> does the same and empties the field |
| "Calculators: each takes what you type, works one thing out and keeps nothing. …" | under the title, with a link to Learn's **Tools** |
| **Hashes**, **Encodings**, **Descriptor checksum**, **Convert key**, **Units**, **Miniscript** | a button per calculator; the one on show filled |
| the mode row | for **Hashes**, **Units** and **Miniscript** only: how what is typed is read |
| the field | what is typed goes into it; its hint names what it takes |
| **Clear** | at the field's end: empties it |

What is worked out shows under the field, a row per value, each in
mono; what cannot be read shows in red instead.

## The calculators

| Calculator | Mode row | Field | Rows |
|---|---|---|---|
| **Hashes** | **Read as: automatic**, **Text**, **Hex** | **Text or hex** | **Bytes**, then SHA-256, SHA-256d and HASH160 in hex. **Not hex** when **Hex** is chosen and the text is not |
| **Encodings** | none | **Base58, bech32 or hex** | **Encoding**, **Prefix** (bech32's), **Version**, **Checksum** (**holds** or **does not hold**), **Bytes**. **Not Base58, bech32, bech32m or hex** |
| **Descriptor checksum** | none | **A descriptor** | **Checksum**; **Given**, the checksum typed after `#` and whether it **holds**; **With it**, the descriptor with its checksum; **Wallet**: **one this device loads** or **not one this device loads**. **Not a descriptor** |
| **Convert key** | none | **An xpub, ypub, zpub, tpub…** | the key in BIP-32's spelling, then as **Legacy**, **Nested SegWit**, **Native SegWit** and **Taproot**; **Depth**, **Fingerprint**, **Child**. **That is an extended private key: this tool takes public keys only**; **Not an extended key** |
| **Units** | **In sat**, **In BTC**, **In mBTC**, **In bits** | **An amount**: digits only, a whole number of the unit chosen | the amount in sat, BTC, mBTC and bits |
| **Miniscript** | "wsh", "tr": the script it is compiled for | **A policy, e.g. or(pk(9a6a2580),and(pk(…),older(1000)))**; a loaded key may be named by its fingerprint | **Descriptor**, the policy compiled; **Spend paths**, how many; **Wallet**: **one this device loads**, or **keys need origins and both chains to load**. A policy that does not compile shows the compiler's message |

## On a narrow window

The top bar holds the way back to Tools and the name, **Tools**; each
value is under its name, and the explanation is at the foot.

## Why

- [Tools](../../learn/16-tools.md)
- [Glossary](../../learn/15-glossary.md)
