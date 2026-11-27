# Check a signed message

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `CheckMessage`
**Key:** check-message

Open this page with **Check the signature** on a signed message in Files, or from Tools. It reads the file's address, signature and text and says whether the signature holds.

## Opening

**Key:** check-message.opening

From Tools it opens on the message itself when exactly one waits in Files, and on a list to choose from when several do. With none it says **No signed message chosen** and offers **Scan**, **Open Files** and, while a stick is attached, Bring in from that stick. The result of an address in a loaded wallet ends with **Open the wallet**, then **Another**, which shows the list again, and **Done**, which goes back to where it was opened.

## Verdict

**Key:** check-message.verdict

The verdict is the first line. When the line reads "The signature is valid" with BIP-137 or BIP-322, the key behind the address signed exactly the text shown below. It says nothing about who presents the message or when it was signed.

Where the signature does not hold, the line says so: it does not verify for the address and text, it is not base64 or not a signature of its format, or the kind of address or format is not signed or checked here. A file that is not a signed message for the network in use shows "Not a signed message" for that network instead of a verdict.

## Details

**Key:** check-message.details

**File** is the file's name and **Address** is the address it names. **Wallet** names the loaded wallet that holds the address, as receive or change with its index, or reads **Not an address of a loaded wallet**.

**Message** is the text that was checked. Read it, because a valid signature over other text proves something other than what was meant.
