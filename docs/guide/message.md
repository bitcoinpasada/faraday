# Sign a message

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Message`
**Key:** message

One card per step: the address, the message, the format and the signature. A signed message is text plus a signature made with the key behind an address; it moves no funds.

## Address

**Key:** message.address

Choose which wallet and receive address signs. Only single-key wallets whose key this device holds are offered, by name. A wallet whose key was made here is offered once it is backed up. Until then its line reads "Its key waits for its backup", and nothing is signed or made with its address.

The address starts at 0/0; **-** and **+** step its index. Check an address offers **Sign a message with this address** for an address of such a wallet, or of a standard account of a loaded key. A wallet's receive address opens here with that wallet and index chosen. A change address, or one from a key with no wallet, shows its path and no **-** or **+**; choosing a wallet leaves it. Choosing another wallet or index clears the signature. Choosing a wallet opens the next card.

## Message

**Key:** message.text

This is the text to be signed, exactly as typed or scanned. The signature fits these characters and no others. A single changed character, a space or a line break, makes a different message and the signature will not check.

**Type** starts typing into the box and **Stop typing** ends it. **Scan** reads a QR code of plain text into the box; a code that holds a key is refused. The count of characters is shown beside the buttons. **Continue** opens the next card.

## Format

**Key:** message.format

Choose the signature's format; the person checking it must be able to read that format.

**BIP-137** is read by most exchanges and older tools. **BIP-322** is the newer format. A legacy or nested SegWit address signs in BIP-137 only, a native SegWit address in either, and a Taproot address in BIP-322 only; a format the address cannot sign is greyed. Choosing a format opens the next card.

## Signature

**Key:** message.sign

**Sign the message** makes the signature and the file for the stick. The file is named `message-` and the last six characters of the address, with the address on the first line, the signature on the second and the message from the third.

**Make the file** writes it again. **Show as QR** shows the same three lines as a code, and the PNG button saves that code as an image. An address, message or format changed afterwards clears the signature.

The card ends with **Check it**, which checks the signature just made on Check a signed message, **Another message**, which starts again empty, and **Done**, which goes back to where Sign a message was opened: Tools, a wallet's card or Home.
