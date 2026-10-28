# Sign a message and Check a signed message

**Kind:** Reference
**Status:** Checked against the app's pictures in docs/guide/pictures/ (drawn by just guide-pictures) and its code and strings (`faraday/faraday-core/src/screens/message.rs`, `faraday/faraday-core/src/screens/check_message.rs`, `faraday/faraday-core/src/state.rs`, `faraday/faraday-core/src/guide.rs`)
**Screen:** `Message`, `CheckMessage`
**Group:** Tools

A message signed with a single-key wallet's key at one of its receive
addresses, and a signed message checked against its address. The
walkthrough is [Sign and check a message](../walk/messages.md).

## Sign a message

**Opened from:** **Sign a message** on a single-key wallet's card on
[Wallets](wallets.md#the-card); **Tools → Sign a message** (**With a
single-key wallet's key here**), which reads **Needs a single-key
wallet with its key here** and does not open while no such wallet is
loaded; the sidebar's row for the signing while it is under way.

![Sign a message on New wallet 1, the Address card open at receive address 0/0](../pictures/messages-sign.png)

*Sign a message on New wallet 1: **Address** open at "Receive address 0/0", **Message**, **Format** and **Signature** closed below.*

A column of four cards, each closed to one line once it is done. It
opens on **Address**, with the first single-key wallet whose key is
loaded chosen and BIP-322 as the format.

| Control | What it does |
|---|---|
| the way back | at the top left, its name under the title: on **Address**, where the page was opened from (the wallet, "New wallet 1"), on a later card the card before it; <kbd>Escape</kbd> does the same |
| **Steps only**, **Guided** | at the top right: the short text on each open card, off or on; on **Address** and **Format** with a link to Learn's **Signing a message** |
| **Cancel** | at the top: ends the signing ([Cancel a flow](prompts.md#cancel-a-flow)) |
| a card | its title and what was chosen; a press opens it again |

### Address

Closed, the address chosen, shortened, or **None chosen**.

| Control | What it does |
|---|---|
| a wallet's button | one per single-key wallet whose key is loaded, by its name; the chosen one filled |
| "Receive address 0/0" | the receive address the message is signed at; **-** and **+** step its number |
| the address | in full; **Back up before you receive**, in the warning tone, in its place while a key made here waits for its backup |
| **Continue** | closes the card and opens **Message** |

### Message

Closed, the text's first line in quotes, or **Not written yet**.

| Control | What it does |
|---|---|
| the box | the text; **Press Type to write the message** while it is empty, **Type the message** while typing goes to it |
| **Type**, **Stop typing** | typing goes to the box, or stops; <kbd>Enter</kbd> starts a new line, <kbd>Escape</kbd> stops typing |
| "38 characters" | the count of every character, spaces and line breaks among them |
| **Continue** | closes the card and opens **Format** |

### Format

Closed, "BIP-322" or "BIP-137".

| Control | What it does |
|---|---|
| "BIP-322", "BIP-137" | the format; one the address cannot carry is grey. A press closes the card and opens **Signature** |

### Signature

Closed, **Signed** or **Not signed yet**.

| Control | What it does |
|---|---|
| **Sign the message** | signs; a message that cannot be signed says why in red under the card |
| **Signature** | the signature, in base64 |
| `message-` and the address's last six characters, `.txt` | the file, **Signed message**, made as the message is signed and put **For the stick** |
| **Remove** | takes the file off **For the stick**; its button then reads **Make the file**, or **Make again** once a stick visit has written it |
| **Show as QR** | the signed message as a [QR code](qr-and-scan.md#a-qr-code) |
| **PNG for the stick** | the same code as a picture file, for the stick |

The file and the code hold three lines: the address, the signature,
and the message. A change to the address, the message or the format
after signing drops the signature.

![The Signature card open, the signature in base64 and message-rtc6am.txt for the stick](../pictures/messages-signed.png)

*The message signed: the **Signature** card open, the signature in base64, `message-rtc6am.txt` **For the stick**, **Show as QR** and **PNG for the stick**.*

## Check a signed message

**Opened from:** **Check the signature** on a signed message in
[Files](files.md#a-file-from-the-stick); **Tools → Check a signed
message** (**Against its address**), which opens the first signed
message in Files and reads **Copy a signed message in first** while
there is none. In Faraday Demo's Practice, with none in Files, the tile
checks the sample `sample-message.txt`.

![Check a signed message: The signature is valid (BIP-322), its file, address, wallet and message](../pictures/messages-check.png)

*message-rtc6am.txt, signed on Sign a message and brought back into Files, opened with **Check the signature**: "The signature is valid (BIP-322)", the address bc1q cfk6 qv40 … rtc6 am, "New wallet 1 · receive 0/0" and the text "Faraday guide example, 27 October 2026". The way back reads **Back to Files**.*

| Control | What it does |
|---|---|
| the way back | at the top left, named after where the page was opened from, under the title |
| "A valid signature proves that whoever holds this address's key signed exactly this text. …" | under the title with **Guided** on, with a link to Learn's **Signing a message** |
| the verdict | in a card: "The signature is valid (BIP-322)" in the ok tone, or why it does not ("The signature does not verify for this address and this message", "The signature is not base64 or not a signature of its format"), in red |
| **File** | the file's name |
| **Address** | the address it was signed for |
| **Wallet** | the loaded wallet whose address it is: "New wallet 1 · receive 0/0" ("change 1/…" for a change address); or **Not an address of a loaded wallet** |
| **Message** | the text that was signed |

A file that is not a signed message for the network in use reads, in
red, "Not a signed message for a mainnet address", and nothing else;
with no file to check, **No signed message chosen**.

## On a narrow window

Each page's way back and title are in the top bar (**Sign a message**,
**Check a message**); Check a signed message puts each value under its
name and its explanation after the rows.

## Why

- [Signing a message](../../learn/14-message.md)
