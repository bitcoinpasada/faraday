# Lightning node key

**Kind:** Reference
**Status:** Checked against the app's pictures in docs/guide/pictures/ (drawn by just guide-pictures) and its code and strings (`faraday/faraday-core/src/lightning/screen.rs`, `faraday/faraday-core/src/lightning/mod.rs`)
**Screen:** `Lightning`
**Group:** Tools

**Opened from:** **Tools → Lightning node key**; a scanner's read card,
**Use in Lightning**, for an LND aezeed read, its words typed in and
**LND · type an aezeed** chosen.

A Lightning node's key, made the way ldk-node makes it from a loaded
key, or the way LND makes it from an aezeed typed in. Its public key is
the node id; its private key is the node.

![Lightning node key by ldk-node from key 3dca6805, the node id shown and the private key hidden](../pictures/lightning.png)

*Lightning node key with "ldk-node · 3dca6805" chosen: the **Node id** in hex, the **Private key** hidden, **Show**, **Make a vault** and **Out unprotected…**.*

## On this screen

| Control | What it does |
|---|---|
| the way back | at the top left, named after where the page was opened from, under the title |
| "A Lightning node's key, made the way ldk-node or LND makes it. …" | under the title with **Guided** on, with a link to Learn's **Tools** |
| "ldk-node · 3dca6805" | one per key loaded, the chosen one filled: the node key ldk-node makes from that key. **This key has no BIP-39 seed**, in red, for a key with no seed to make it from |
| **LND · type an aezeed** | the node key LND makes from its aezeed: opens the fields below |

### The aezeed

| Control | What it does |
|---|---|
| the words field | **The 24 words** until typed; the count at its end ("12 of 24") |
| **Never type a seed that holds real bitcoin here.** | under the field, in Faraday Demo |
| **Passphrase** | the aezeed's passphrase, shown as dots; **aezeed (LND's default)** while empty |
| **Read it** | reads the words with the passphrase |

What cannot be read says why in red: "An aezeed is 24 words; 12 are
typed", "Word 7 is not on the list", "Not an aezeed for this
passphrase: …".

### The node

| Control | What it does |
|---|---|
| **Node id** | the node's public key, in hex |
| **Made** | for an aezeed: the date it was made |
| **Private key** | hidden as dots until **Show** |
| **Show**, **Hide** | the private key on screen, or hidden |
| "Save into Vault 1" | the node key into the open vault (**The node key is in the vault**) |
| **Make a vault**, "Unlock Vault-1.fvlt", **Unlock a vault** | in its place while no vault is open: **Make a vault** while Files holds none, "Unlock" and the file's name for one locked vault in Files, **Unlock a vault** for several; each comes back here |
| **Out unprotected…** | the node key to the stick unprotected, through [This is a secret](files.md#this-is-a-secret) |

## On a narrow window

The top bar holds the way back to Tools and the name, **Lightning**;
each value is under its name, **Read it** under the passphrase, and the
explanation at the foot.

## Why

- [Tools](../../learn/16-tools.md)
- [Glossary](../../learn/15-glossary.md)
