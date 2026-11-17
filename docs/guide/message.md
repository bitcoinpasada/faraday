# Sign a message

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Message`
**Key:** message

One card per step: the address, the message, the format and the signature. A signed message is text plus a signature made with the key behind an address; it moves no funds.

## Address

**Key:** message.address

Choose which wallet and which receive address signs. Only a single-key wallet whose key is held on this device is offered, each by its name.

The address is a receive address of the wallet, 0/0 at first; **-** and **+** step its index. Check an address offers **Sign a message with this address** on an address it finds in such a wallet, or in a standard account of a key you have loaded. A receive address of a wallet opens this screen with that wallet and index chosen. A change address, or an address of a key with no wallet, opens it showing that address with its path and no **-** or **+**; choosing a wallet then leaves it. Choosing another wallet or index clears a signature already made. Choosing a wallet opens the next card.

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
