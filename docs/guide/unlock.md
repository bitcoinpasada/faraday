# Unlock

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Unlock`
**Key:** unlock

Opens one locked vault from Files with its passphrase. It opens from a locked vault's row in Vaults and from the flows that need a vault open.

## The vault

**Key:** unlock.vault

One row per locked vault: its file name, where it is and its size, the unlock cost it was made with and, beside it, the time that cost takes here. The row chosen is the one unlocked. With none locked the screen reads **No locked vault in Files** and offers **Create a vault**.

A vault just made is chosen already and asks for its passphrase once more.

## Passphrase

**Key:** unlock.passphrase

**Passphrase** is typed once and can be shown or hidden. With a stick attached the passphrase cannot be typed: the screen asks for the stick to be removed first, and the field opens once it is out.

**Unlock** runs the unlock; while it runs the screen says so, with the time it takes here. Beside it the memory free on this computer is shown, or **This computer: memory not reported**. A refused passphrase or key shows its reason under the field.

## Key file and YubiKey secret

**Key:** unlock.keys

A vault made with either needs it as well as the passphrase.

**Add a key file** opens **Choose a key file**, a list of the files in Files that are not vaults; with none, it reads **Files holds no file that is not a vault**. **Add a YubiKey secret** opens **YubiKey secret**, the 40 hexadecimal digits, spaces or colons allowed between them; a wrong count is refused with the count found.
