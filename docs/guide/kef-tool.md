# Krux code

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `KefTool`
**Key:** kef-tool
**Words:** bip39-passphrase, fingerprint, seed, words

One card per step: the contents, the passphrase and its options, and the code. The tool makes an encrypted QR code in Krux's format from text, from a wallet's descriptor or from a loaded key's words; nothing typed is kept once the tool is left.

## Contents

**Key:** kef-tool.contents

Above the steps, **Open a Krux code** reads a code that exists instead: **Scan a Krux code** and **From Files** each ask its passphrase and load the key it holds. Choose what the code holds.

**Type it** takes **Text**, hidden until the eye shows it, and **Continue** goes on once there is some. **A loaded wallet** lists the loaded wallets, each by name and shape, and the code holds the wallet's public descriptor; with none loaded, the card says so. **This key's words** is offered while a key is loaded: the only key that has 12 or 24 English words is chosen at once, and with several each is listed by name and fingerprint. The code holds the words as Krux reads them back, under the same passphrase. The key's own BIP-39 passphrase is not in it: the Code card says so for a key that has one.

## Passphrase

**Key:** kef-tool.make

The passphrase encrypts the code, and the same passphrase opens it again. Type it twice, in **Passphrase** and **Passphrase again**, or made with **Generate with dice**.

**Options** sets the code's form. The defaults are version 20, 100,000 rounds and a random id of 8 bytes. **Size** lists the sizes the contents can take, each with the sheet it can be copied onto by hand. **Version** offers 20 under **AES-GCM** and 5 under **AES-ECB**; an ECB version refuses a code with two equal blocks. Faraday opens all twelve versions and writes these two. **Rounds** is 10,000 to 500,000. **Id** is **Random 8 bytes**, **Random 16 bytes**, **A label** or, where one is offered, **The fingerprint**; the id is printed on the code and is not secret. **Make the code** makes it.

## Size

**Key:** kef-tool.make-size

**Size** in Options lists a row for each sheet size the contents can take: 25 × 25, 29 × 29, 33 × 33 or 37 × 37 squares, each with its sheet in Faraday Starter, Blank templates, Krux code. The size follows the version, the id and the contents; choosing a row sets the options that give it. Where the contents fit none, the card says **By hand** is **No**, **Largest** 37 × 37. **Low error correction** is a switch offered for version 5 only.

## Code

**Key:** kef-tool.code

The code is shown with its version, rounds and id.

**Check my code** opens the code again: type the passphrase in **Passphrase of the code**, **Compare** decrypts the code with it, and a match shows **Checked**, and **Done** closes the check. The card also states **Size** and **By hand**: the sheet of the code's size, or **No** with **Largest** 37 × 37 for a bigger code. **Copy by hand** shows a grid to copy block by block onto the sheet of its size, for a code of 37 × 37 or less, and **Show the code** returns to the code. The grid shows the squares every code of that size shares black; the rest are filled by hand, block by block, columns numbered and rows lettered. **Make another** returns to the passphrase. **Done** goes back to where the tool was opened.
