# Tools, every flow

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Catalog`
**Key:** catalog

Every flow of the app on one page, in groups of tiles, each with the standards it follows. **Find a tool: a name, a BIP number** narrows the tiles to those whose name, line, group or standards hold every word typed, and **No tool matches** shows when none does.

## Keys and seeds: making and restoring

**Key:** catalog.keys-and-seeds

A tile that cannot open yet shows in its own line what it needs first; one that needs nothing opens its flow.

**Add a key** makes a key from dice, coins, cards, hex, camera or a mix, or takes its words typed. **Scan a SeedQR** takes a standard or compact code. **SLIP-39 shares** deals a new key as m of n shares, and **Restore from SLIP-39** takes shares typed or scanned back into a key. **codex32** takes a secret or its shares typed into a key. **Seed XOR** combines parts into a key. **Sweep a private key** moves a paper wallet's or a WIF key's coins to a wallet; Practice sweeps no private key.

## Keys and seeds: working with a key

**Key:** catalog.key-tools

**Explore a key**, **Child seeds and passwords**, **Vanity address** and **Silent payments** work from a key and read **Load a key first** until one is loaded. **Explore a key** gives its xpubs and addresses at any path. **Child seeds and passwords** are derived from it by BIP-85. **Vanity address** looks for a first address that begins as chosen. **Silent payments** gives one address to give out.

**Convert a seed** gives the same wallet in another standard. **Krux code** makes a passphrase, note or wallet an encrypted code. **Lightning node key** reads a node's key from a key here or from an LND aezeed.

## Wallets

**Key:** catalog.wallets

**Single-key wallet** makes a native SegWit, Taproot, nested or legacy wallet. **Multisig** is k of n keys, **Taproot multisig** is k of n in one script leaf, and **MuSig2** is all keys with one signature on chain; each opens Make a new wallet at that kind. **Miniscript and inheritance** loads one from its descriptor.

**Restore an existing wallet** takes a descriptor, a wallet file, a BIP 129 record or split sheets. **Back up a wallet** writes seeds by hand, on sheets, in envelopes or as codex32 shares, and reads **Load a wallet first** until one is loaded. **Read a Bitcoin Core wallet** reads and checks a wallet.dat, and reads **Copy a wallet.dat in first** until one is in Files.

## Transactions

**Key:** catalog.transactions

**Sign a transaction** takes a PSBT from Files or a QR code. **Nonce check** recomputes and checks every signature. **Check a signer** compares the sent and returned PSBTs. **Compare two PSBTs** puts two side by side. **Decode a transaction** shows what it spends and pays, field by field. **Decode a Lightning invoice** shows what it asks to be paid, and by whom.

**Check an address** takes a scanned address and searches the loaded wallets, and reads **Load a wallet first** until one is loaded.

## Messages

**Key:** catalog.messages

**Sign a message** signs with a single-key wallet's key here, and reads **Needs a single-key wallet with its key here** otherwise. **Check a signed message** checks one against its address, and reads **Copy a signed message in first** until one is in Files. **Check a timestamp** checks a proof against a block header, and reads **Copy a proof in first** until one is in Files.

## Vaults and other keys

**Key:** catalog.vaults-and-other-keys

**Vaults** holds keys, wallets, entries and notes under passphrases. **QR vault** seals a note or a key in one QR code. **TOTP codes** shows codes from the entries in open vaults. **Diceware passphrase** is rolled with dice in Create a vault. **GPG key** makes, restores, signs, revokes and writes a paperkey. **Secure Boot keys** holds PK, KEK and db, and signs an EFI image. **KeePass export** writes a vault entry as a sealed database.

**TOTP codes**, **GPG key**, **Secure Boot keys** and **KeePass export** read **Make a vault first** or **Unlock a vault first** while no vault is open, and still open, to lead to that.

## Calculators

**Key:** catalog.calculators

**Hashes** gives SHA-256, SHA-256d and HASH160. **Encodings** says what a string is encoded in, and its value. **Descriptor checksum** gives a descriptor's checksum, and whether it holds. **Convert an xpub** gives every spelling of one extended key. **Units** gives one amount in all four units. **Miniscript compiler** compiles a policy into a descriptor. Each opens on the Tools page of calculators.

## Word lists

**Key:** catalog.word-lists

**BIP-39 words** is 2048 words, each with its number and its 11 bits. **EFF large list** is 7776 words, each with its number and its five dice. **EFF short list 1** is 1296 words with their numbers and four dice, and **EFF short list 2** the same with unique three-letter starts.

## QR

**Key:** catalog.qr

**Scan a QR code** reads a single or animated code, as BC-UR or BBQr, into Files.
