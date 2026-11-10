# Tools

## What the tools are

The standalone calculators in Tools take what you type, work one thing out from it, and keep nothing. They hold no key of their own, and an extended private key typed into one is refused. Two go further: Decode reads the wallets you have loaded, and the Lightning tool reads a node's seed and can save the node's private key into the open vault.

## Hashes

SHA-256, SHA-256 applied twice, and RIPEMD-160 of SHA-256, which Bitcoin calls HASH160. These three are what Bitcoin hashes with: a transaction id is SHA-256d of the transaction, and the 20 bytes inside a legacy address are HASH160 of a public key.

The field is read as hex when every character is a hex digit and there is an even number of them, and as text otherwise. The mode row forces one or the other.

## Encodings

Bitcoin writes bytes in a handful of alphabets. Base58Check carries a version byte and a four-byte checksum, and spells legacy addresses and extended keys. Bech32 and bech32m carry a human-readable prefix and their own checksum, and spell segwit and Taproot addresses.

Give it a string and the tool says which one it is, what bytes it holds, and whether the checksum holds. Give it hex and it offers the same bytes in the other spellings.

## Descriptor checksum

A descriptor is the text a wallet is described by. The eight characters after the # are a checksum over it, and a coordinator that reads a descriptor with a broken checksum refuses it rather than deriving the wrong addresses.

The tool computes the checksum of what you type, says whether the one that came with it holds, and says whether the descriptor parses as a wallet this device could use.

## Convert an xpub

One extended public key has several spellings. BIP-32 writes it as xpub or tpub; SLIP-132 writes the same key as ypub, zpub, upub or vpub to say which script type it is meant for: xpub for legacy, ypub for nested SegWit, zpub for native SegWit. The key material is identical; only the four version bytes differ.

Give it any of them and the tool shows all of them, with the network, the depth, the fingerprint and the child number the key states about itself.

## Miniscript compiler

Tools → Miniscript compiler compiles a policy into a descriptor, and says whether this device loads it. Where a policy names a key, the fingerprint of a key this device already holds stands for that key's account key, so pk(73c5da0a) is accepted and needs no xpub typed. The Keys row above the field lists the fingerprints it will take; with no key loaded there is no such row and every key must be written out.

## Units

One bitcoin is 100 000 000 sats. Type an amount in bitcoin, millibitcoin, bits or sats and the other three follow.

## Decode a transaction

Decode shows the review that Sign a transaction shows, for a transaction that is only being read: what it spends and pays, field by field. The wallets you have loaded are used to read it, so change of a wallet in use is recognised as it is when you sign. Nothing is signed.

Use it to read what a coordinator produced before you sign it anywhere, or to see what a raw transaction from a block explorer actually pays.

## QR vault

Tools → QR vault seals one small thing under a passphrase into a single QR code. Make takes a note typed in, or the words of a loaded key of 12 or 24 English words, then a passphrase typed twice. It shows the code, and offers it as a labelled picture and as a .pgp file, both to the Outbox. Open takes a scanned code or a .pgp file and the passphrase, and says what the code holds.

The code is a standard OpenPGP message under that passphrase, not a format of Faraday's own. Sequoia opens it without Faraday: save the code's bytes with zbarimg --raw -Sbinary code.png > vault.pgp, then run sq decrypt vault.pgp and type the passphrase. GnuPG does not open it.

The passphrase is the only thing protecting the code. Anyone who has the code and guesses the passphrase reads what it holds, and a lost passphrase loses the contents: nothing else opens it. The tool warns that the code is only as strong as its passphrase.

A QR vault is not a file vault. Nothing is kept on a stick, it is not listed under Vaults, and it is never open: what was typed, opened or made is cleared when the tool is left. To change what a code holds, make a new one; the old code still opens to what it held, so destroy it. The code can be printed. The picture carries the words QR vault, a name and the date it was sealed, and never what it holds. It is too large to copy by hand, so carry the picture on a stick to a computer with a printer.

## Krux code

Makes an encrypted QR code in Krux's format from text you type or from a loaded wallet's descriptor, under a passphrase. Options offers the sizes the contents can take, each naming its Starter sheet, and sets the version, the rounds and the id; the defaults are version 20, 100,000 rounds and a random id. The code comes with Check my code, which opens it again with the passphrase to show it holds what you gave. A code that fits the Starter's blank Krux sheet can also be copied onto it by hand, block by block, with Copy by hand. The text and the passphrase are wiped when you leave the tool. What a Krux code is, and what its passphrase has to carry, is on the page about other backups.

## Lightning node key

A Lightning node has an identity key, and a backup of that node is an LND cipher seed — twenty-four words that are not a BIP-39 phrase —, the BIP-39 words an ldk-node wallet was built from, or Core Lightning's hsm_secret file. The tool reads any of them and says which node it is: the public key the network knows the node by, and for a cipher seed the version and the birthday it carries.

Core Lightning keeps its node's root secret in one file, hsm_secret. It is 32 bytes, or 73 bytes when a passphrase seals it, or, in a node made since Core Lightning began to write words, a hash followed by the 12 to 24 English words of the node. Pick the file from Files: a file sealed by a passphrase asks for it, and the empty passphrase is accepted when that is the one that was set. Core Lightning's node is also given from English BIP-39 words typed here, with their passphrase. A file of 32 bytes cannot say it is an hsm_secret, so compare the node id with the one your node reports in lightning-cli getinfo. The file is opened here and never decrypted anywhere else.

Use it to confirm that a backup in your hand belongs to the node you think it does, before you restore it anywhere. A node key is hot by nature, on a machine that is online and in use all the time, so Faraday signs nothing with it. The node's private key is a secret: it goes into the open vault, or out unprotected only after a warning.
