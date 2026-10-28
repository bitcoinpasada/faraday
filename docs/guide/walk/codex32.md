# codex32 shares

**Kind:** Spec
**Status:** Text moved from `docs/GUIDE.md` (§4.11), written and checked against the app's screens and strings; the pictures are in `docs/guide/` and keep their names
**Open:** the walkthrough's form (result line, what is needed, time, "With your own key"); the pictures, drawn once after the theming has landed

Instead of steps 4.1 to 4.10, a single-key wallet can have its one key
made as codex32 shares: strings worked from your own dice, any 2 of 3
of which rebuild the key, and 1 alone shows nothing of it. Each string
goes to a place of its own, so no one place holds the key. Section 5
does not need this step.

The test data: BIP 93's published vector 2, identifier `name`. Its
characters make key `fab6868a`. Never keep funds on it.

**Wallets → Make a new wallet**. Kind opens with **Single key, backup
in shares (codex32)** ticked; press it:

| Quorum | Keys |
|---|---|
| ![Quorum, 2 of 3](../4-11-quorum.png) | ![Keys, Make codex32 shares](../4-11-keys.png) |
| **How many to rebuild it** 2 and **How many shares** 3; the line under them reads **Any 2 of the 3 shares rebuild the key.** Press **Continue**, then **No** on **Use vaults**. | **Key 1** offers one button, **Make codex32 shares**. Press it. |

![Make codex32 shares, on the identifier](../4-11-identifier.png)

The page is [**New key**](../screens/new-key.md#make-codex32-shares), open on **Identifier**: **Start** reads **Make
a new key · Codex32** and **Scheme** **48 characters · 2 of 3**, from
Quorum. Type four characters that name the set (**Suggest** fills four),
then **Continue**. **Entropy** offers **Roll dice**, **Type the
characters** (made on paper) and **Type shares made by hand**; dice
can have 4, 6 (the default), 8, 10, 12 or 20 faces, five of one size,
each die's first and second roll giving one bit, as on the booklet's
de-biasing worksheet. Each roll or character drops into its box and the worksheets fill as on
paper; **Worksheets** then shows each sheet whole, and **Strings** each
string to write down and type back. Learn's page **Backups in other
forms** walks through these cards.

![Strings made](../4-11-strings-made.png)

**Strings made** lists **Split**, **Length**, **Identifier**,
**Fingerprint** (`fab6868a` for the test data) and **Entropy**.
**Check on another device** lists sets of strings to load on another
implementation; every set must give the same fingerprint. Press **Lock
in**: the key fills Key 1 and the wallet is made.

| Check | The board, the standard way |
|---|---|
| ![Check, with the Backup row](../4-11-check.png) | ![One string per place](../4-11-board.png) |
| **Descriptor** reads **Single key · native SegWit · fab6868a** and a checksum, and **Backup** **codex32 name · 2 of 3**. Press **It matches**, then **Back up wallet**. | One string per place, on paper, each beside **Whole**, the wallet description: **fab6868a · A** at Place 1, **C** at Place 2, **D** at Place 3. Each place reads **1 of 2 shares of fab6868a · alone: nothing**. **Any one place lost: the rest rebuild the wallet** reads **Yes**; **One place found: can spend**, **No**. |

**Customize backup** changes the board as in steps 4.3 to 4.5: a string
can go into a vault or a stick's files as well. Press **Continue**, then
**Make the checklist**.

![The codex32 strings item](../4-11-strings-item.png)

**Print 3 blank templates** asks for **Codex32, 48 characters** from the
Starter. [**Codex32 strings of seed fab6868a**](../screens/back-up-wallet.md#the-checklist) lists each string and its
place, **Not checked** until it is typed back. **How many strings of
name are there?** counts the strings that exist, if more were made.
Under **The set**, **Add a share** makes one more string of the same
set, **Rotate the set** makes a new set in place of this one, and
**Replace A** replaces a lost string. Press **Check my copy** and type
each string back from its paper; a box copied wrong is named. The rest
of the checklist goes as in steps 4.9 and 4.10.

A string can be kept on more than paper. Here, after **Change the
plan** and **Customize backup**, string A also goes into Vault 1 on a
stick at Place 1, and string C onto a new stick at Place 2, its forms
**Text file** and **QR picture** both on. Each place still reads **1 of
2 shares of fab6868a · alone: nothing**.

![Strings in a vault and in a stick's files](../4-11-board-kept.png)

On the checklist, the vault's item names the set under **Into the
vault**: **1 of 2 shares of fab6868a**, its dot turning green once the
open vault holds that string.

![The vault item, with the set it keeps](../4-11-vault-item.png)

**Save into vault.ofv** (step 4.8) saves each string the plan puts in
the vault as a share record. The vault lists it under **Bitcoin keys**:
**Share A of fab6868a**, **codex32 name · threshold 2**. At unlock,
strings of one set from the vaults open together load the key once
there are enough of them; a string typed in counts with them.

![The vault's share record](../4-11-vault-contents.png)

At **Lock and write** (step 4.10), a string kept in a stick's files is
written as `codex32-fab6868a-c.txt`, and its Codex32QR picture as
`codex32qr-fab6868a-c.png`; the picture is read back against the string
before the stick counts. One such file shows nothing of the key, but
enough of them together are the key: the stick's files are **not
sealed**.

![Insert Stick 3, with the string's files](../4-11-files.png)
