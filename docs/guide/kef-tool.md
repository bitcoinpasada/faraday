# Krux code

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `KefTool`
**Key:** kef-tool

One card per step: the contents, the passphrase and its options, and the code. The tool makes an encrypted QR code in Krux's format from text or from a wallet's descriptor; nothing typed is kept once the tool is left.

## Contents

**Key:** kef-tool.contents

The choice is what the code holds.

**Type it** takes **Text**, hidden until the eye shows it, and **Continue** goes on once there is some. **A loaded wallet** lists the loaded wallets, each by name and shape, and the code holds the wallet's public descriptor; with none loaded, the card says so.

## Passphrase

**Key:** kef-tool.make

The passphrase encrypts the code, and the same passphrase opens it again. It is typed twice, in **Passphrase** and **Passphrase again**, or made with **Generate with dice**.

**Options** sets the code's form. The defaults are version 20, 100,000 rounds and a random id of 8 bytes. **Size** lists the sizes the contents can take, each with the sheet it can be copied onto by hand. **Version** groups the versions by **AES-GCM**, **AES-CTR**, **AES-CBC** and **AES-ECB**, and **Legacy** opens the oldest two; a compressed version takes at most 1,024 bytes, and an ECB version refuses a code with two equal blocks. **Rounds** is 10,000 to 500,000. **Id** is **Random 8 bytes**, **Random 16 bytes**, **A label** or, where one is offered, **The fingerprint**; the id is printed on the code and is not secret. **Make the code** makes it.

## Code

**Key:** kef-tool.code

The code is shown with its version, rounds and id.

**Check my code** opens the code again: the passphrase is typed in **Passphrase of the code**, **Compare** decrypts the code with it, and a match shows **Checked**. **Done** returns. **Copy by hand** shows a grid to copy block by block onto the sheet of its size, for a code small enough, and **Show the code** returns to the code. **Make another** returns to the passphrase.
