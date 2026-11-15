# Check an address

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `CheckAddress`
**Key:** check-address

Says whether an address belongs to a loaded wallet or to a loaded key's standard accounts. Nothing is signed, paid or kept.

## The address

**Key:** check-address.field

The address goes in the **Address** box, typed or pasted, or **Scan** takes it from a QR code. Enter or Done checks what is typed; pasted or scanned text is checked at once. A `bitcoin:` link is read to its address. **From a file** is there when Files holds a text file: a press on one reads it.

A file with one address is checked as if typed. A file with several lists each, at most 100, with its answer; a press on a row shows that result, and Back returns. An empty, oversize (over 64 KiB) or non-text file reads **Not a text file**; a wallet description is named, not read.

Text that is no address, an address of another network and a link this device does not read are named under the box. A seed or private key is refused and the box emptied.

## The result

**Key:** check-address.result

The first 100 receive and change addresses of each loaded wallet and of each loaded key's standard accounts are searched. A find names the wallet or key, the place (**Receive** or **Change** and its index), the wallet's shape and the path, or each key's path in a multisig wallet. A find in a key's account offers **Make a wallet**, then **Open the wallet**. A find in a single-key wallet whose key is loaded, or in a loaded key's account, also offers **Sign a message with this address**.

A find says this device makes the address; whether a wallet app or hardware wallet shows the same one is read on its own screen.

A miss names how far it looked. **Look further** goes to deeper indices, and **Stop** ends the search where it is. With no wallet and no key loaded the page says **Load a key or a wallet first**.
