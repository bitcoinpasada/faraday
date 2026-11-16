# Unlock

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Unlock`
**Key:** unlock

Unlock opens one locked vault in Files with its passphrase. You reach it from a locked vault's row in Vaults, and from any flow that needs a vault open.

## The vault

**Key:** unlock.vault

One row per locked vault: its file name, where it is and its size, the unlock cost it was made with and, beside that, how long that cost takes on this computer. The row you choose is the one unlocked. With no locked vault the screen reads **No locked vault in Files** and offers **Create a vault**.

A vault you have just made is already chosen, and asks for its passphrase once more.

## Passphrase

**Key:** unlock.passphrase

Type the **Passphrase** once; it can be shown or hidden. It cannot be typed while a stick is attached: the screen asks you to remove the stick first, and the field opens once it is out.

**Unlock** runs the unlock, and while it runs the screen says so, with the time it takes here. Beside it is the memory free on this computer, or **This computer: memory not reported**. If the passphrase or key is refused, the reason shows under the field.

## Key file

**Key:** unlock.keys

A vault made with a key file needs that file as well as the passphrase.

**Add a key file** opens **Choose a key file**, a list of the files in Files that are not vaults; with none, it reads **Files holds no file that is not a vault**.
