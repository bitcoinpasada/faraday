# Check an address

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `CheckAddress`
**Key:** check-address

This page says whether an address belongs to a loaded wallet or to a loaded key's standard accounts. Nothing is signed, paid or kept.

## Opening

**Key:** check-address.opening

From Tools it opens on the file itself when exactly one text file waits in Files, and on a list of the text files to choose from when several do. With none it says **No address to check** and offers **Scan**, **Open Files** and, while a stick is attached, Bring in from that stick; typing into the box also works. A wallet description or a file that is not text does not count as waiting. A result ends with **Another**, which shows the page empty again, and **Done**, which goes back to where it was opened.

## The address

**Key:** check-address.field

Put the address in the **Address** box, typed or pasted, or use **Scan** to read it from a QR code. Enter or Done checks what you typed; pasted or scanned text is checked at once. A `bitcoin:` link is read down to its address. **From a file** appears when Files holds a text file, and pressing a file reads it.

A file with one address is checked as if typed. A file with several lists each, at most 100, with its answer; press a row to see that result, and Back returns. An empty, oversize (over 64 KiB) or non-text file reads **Not a text file**; a wallet description is named, not read.

Under the box, the page names text that is not an address, an address of another network, and a link this device does not read. A seed or private key is refused and the box emptied.

## The result

**Key:** check-address.result

The first 100 receive and change addresses of each loaded wallet and of each loaded key's standard accounts are searched. A find names the wallet or key, the place (**Receive** or **Change** and its index), the wallet's shape and the path, or each key's path in a multisig wallet. A find in a key's account offers **Make a wallet**, then **Open the wallet**; a file's several addresses do not. A find in a loaded key's account or single-key wallet also offers **Sign a message with this address**.

A find says this device makes the address; whether a wallet app or hardware wallet shows the same one is read on its own screen.

A miss names how far it looked. **Look further** goes to deeper indices, and **Stop** ends the search where it is. With no wallet and no key loaded the page says **Load a key or a wallet first**.
