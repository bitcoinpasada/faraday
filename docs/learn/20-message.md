# Signing a message

## What a signed message is

A signed message is a piece of text plus a signature made with the key behind one of your addresses. It has three parts: the address, the signature, and the text. The file OpenSigner saves and the QR code it shows carry the address on the first line, the signature on the second line, and the message from the third line to the end.

Nothing about it touches the blockchain. It is text, and it is checked by whoever you hand it to.

## What it proves

That whoever made the signature holds the key behind that address, and that this exact text was signed with it. People use this to prove ownership of an address to an exchange, to prove control of funds without moving them, or to sign a statement in a way that cannot be forged.

## What it does not prove

It moves no funds and says nothing about what the address holds.

Anyone can copy the three lines and pass them on. Only the key can make a new one. So if someone wants to prove something to you, ask for a signature over text that you chose, including the date, and treat a ready-made signature you were handed as proof of nothing.

## Formats

OpenSigner produces BIP-137 signatures for legacy and SegWit addresses, and BIP-322 signatures, which work for every address type including Taproot. Choose the format your verifier expects. Most exchanges and older tools expect BIP-137.

## Checking a signed message

Verify on OpenSigner reads a signed message from a QR code or a file, checks whether the signature matches the address and the text, and shows you the text that was checked. Read the text. A valid signature over the wrong text proves the wrong thing.
