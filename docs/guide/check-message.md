# Check a signed message

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `CheckMessage`
**Key:** check-message

Opened from **Check the signature** on a signed message in Files. It reads the file's address, signature and text and says whether the signature holds.

## Verdict

**Key:** check-message.verdict

The verdict is the first line. The line "The signature is valid" with BIP-137 or BIP-322 means the key behind the address signed exactly the text shown below. It says nothing about who presents it or when it was signed.

Where the signature does not hold, the line says so: it does not verify for the address and text, it is not base64 or not a signature of its format, or the kind of address or format is not signed or checked here. A file that is not a signed message for the network in use shows "Not a signed message" for that network instead of a verdict.

## Details

**Key:** check-message.details

**File** is the file's name and **Address** is the address it names. **Wallet** names the loaded wallet that holds the address, as receive or change with its index, or reads **Not an address of a loaded wallet**.

**Message** is the text that was checked. Read it: a valid signature over other text proves something other than what was meant.
