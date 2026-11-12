# QR vault

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `QrTool`
**Key:** qr-tool

One card per step: the contents, the passphrase and the code. A QR vault seals one small thing under a passphrase into a single QR code, and the same tool opens a scanned code or a .pgp file; nothing typed, opened or made is kept once the tool is left.

## Contents

**Key:** qr-tool.contents

The choice is what the code holds.

**Type a note** takes a **Note**, hidden until the eye shows it, with the characters left counted below; **Continue** goes on once the note is not empty. **A loaded key** lists the loaded keys, and a key that is not 12 or 24 English words says **Not 12 or 24 English words** and cannot be chosen. A vault item saved with **Save as a QR vault** opens the tool at the next card, with the item named by its kind and vault; a seed whose passphrase is kept in the vault does not carry it into the code.

## Passphrase

**Key:** qr-tool.passphrase

The passphrase seals the code and is the only thing that opens it: a lost one loses the contents. It is typed in **Passphrase** and **Passphrase again**.

**Memory** is the memory the sealing takes, 64 to 1,024 MiB; opening the code needs the same memory, and a computer without room for it says so. A row adds randomness of one's own. **Name** is printed on the picture of the code beside the date; what the code holds never is. **Make the code** makes it.

## Code

**Key:** qr-tool.code

**Show as QR** shows the code. **PNG to the Outbox** and **To the Outbox** save the picture and the .pgp file. **Make another** returns to the first card.

The code is a standard OpenPGP message under the passphrase, so software that reads one opens it without Faraday.

## Open

**Key:** qr-tool.open

A scanned QR vault or a .pgp file opens the tool here, with its size in bytes. The passphrase is typed in **Passphrase** and **Open** decrypts it; a wrong passphrase, a damaged code or a message that is not a QR vault is named.

What the code holds is named: A seed of 12 or 24 words offers **Load as a key**; A codex32 share names its key and offers **Add the share**; A passphrase names its key and shows it, with **Add to key** where that key is loaded and its words were loaded without a passphrase; A note is shown and offers **Save as a note** into the open vault, or says **Open a vault first**.
