# Lightning node key

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Lightning`
**Key:** lightning

Works out a Lightning node's key from a loaded key, an LND aezeed, or Core Lightning's words or hsm_secret file, to confirm which node a backup belongs to. The node id is public; the private key is the node and is a secret. Nothing is signed with it.

## Source

**Key:** lightning.source

Each loaded key has two buttons, ldk-node · and Core Lightning · followed by the key. Choosing one reads the node at once: ldk-node reads the key's BIP-39 seed as ldk-node does and Core Lightning reads its words as lightning-hsmtool does. A key with no BIP-39 seed gives no node.

**LND · type an aezeed** takes the 24 words of an LND cipher seed, counted as typed, with **Passphrase**, which is aezeed (LND's default) when left empty. **Read it** works the node out. A wrong word, a wrong count or a wrong passphrase shows the reason in one line.

## Core Lightning

**Key:** lightning.core-lightning

**Core Lightning · hsm_secret** takes 12 to 24 English BIP-39 words with their passphrase, or a file of Files, one button per file that could be an hsm_secret, Use followed by its name; **No hsm_secret in Files** shows when there is none. A file sealed by a passphrase asks for it, and one that asks for nothing is read when chosen. **Read it** works the node out; a wrong passphrase or a changed file shows the reason in one line.

## Node

**Key:** lightning.node

**Node id** is the node's public key in hex, with **Show as QR** and **PNG**. **Made** is the day the aezeed says it was made and shows for an aezeed only.

**Private key** is hidden until **Show**, and **Hide** hides it again. Save into the open vault writes it to a note in the vault; with no vault open, one is made or unlocked first. **Out unprotected…** writes it to a file after a warning that whoever has it is the node, with its channels and their money.
