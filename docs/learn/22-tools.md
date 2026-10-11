# Tools

## What the tools are

Each tool is a calculator. It takes something you scan, paste or type, works one thing out from it, and keeps nothing. No tool touches a key, and none of them stores anything, so nothing on these screens is a secret.

Every tool but Units opens on the scanner. Under the viewfinder are Read a file, Paste and Type: the string reaches the tool the same way whichever of them you use, and the answer opens as soon as the tool can work one out. Words, a seed code, an extended private key and a private key are refused here, because a secret is never pasted.

## Hashes

SHA-256, SHA-256 applied twice, and RIPEMD-160 of SHA-256, which Bitcoin calls HASH160. These three are what Bitcoin hashes with: a transaction id is SHA-256d of the transaction, and the 20 bytes inside a legacy address are HASH160 of a public key.

The field is read as hex when every character is a hex digit and there is an even number of them, and as text otherwise. The mode row forces one or the other, which is how you hash the four characters "dead" rather than the two bytes 0xde 0xad.

## Encodings

Bitcoin writes bytes in a handful of alphabets. Base58Check carries a version byte and a four-byte checksum, and spells legacy addresses and extended keys. Bech32 and bech32m carry a human-readable prefix and their own checksum, and spell segwit and Taproot addresses.

Give it a string and the tool says which one it is, what bytes it holds, and whether the checksum holds. Give it hex and it offers the same bytes in the other spellings.

## Descriptor checksum

A descriptor is the text a wallet is described by. The eight characters after the # are a checksum over it, and a coordinator that reads a descriptor with a broken checksum refuses it rather than deriving the wrong addresses.

The tool computes the checksum of what you type, says whether the one that came with it holds, and says whether the descriptor parses as a wallet this device could use.

## Convert key

One extended public key has several spellings. BIP-32 writes it as xpub or tpub; SLIP-132 writes the same key as ypub, zpub, upub or vpub to say which script type it is meant for. The key material is identical; only the four version bytes differ.

Give it any of them and the tool shows all of them, with the network, the depth, the fingerprint and the child number the key states about itself. A key already loaded is a row of its own on the scanner, Use a loaded key, which fills the field with that key's account key. Extended private keys are refused: the key explorer is where those are seen.

## Units

One bitcoin is 100 000 000 satoshi. A millibitcoin is a thousandth of a bitcoin, and a bit is a millionth. Type an amount in any of the four and the other three follow as you type.

## Decode a transaction

The same review the Sign flow shows, over a transaction you are reading rather than signing. Whatever keys and wallets you have loaded are used to read it, so change of a wallet in use is recognised here as it is when you sign; with nothing loaded the change row says so. Nothing is signed either way, there is no confirm, and the last page is Done.

Use it to read what a coordinator produced before you sign it anywhere, or to see what a raw transaction from a block explorer actually pays.

## Lightning node key

A Lightning node has an identity key, and a backup of that node is either an LND cipher seed — twenty-four words that are not a BIP-39 phrase — or the BIP-39 words an ldk-node wallet was built from. The tool reads either one and says which node it is: the public key the network knows the node by, and for a cipher seed the version and the birthday it carries.

Use it to confirm that a backup in your hand belongs to the node you think it does, before you restore it anywhere. Nothing is loaded and nothing is signed. A node key is hot by nature — it is on a machine that is online, in use, all the time — so it is not a key this device holds or signs with, and what the tool derives is gone when you leave the screen.
