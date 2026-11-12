# Vaults

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Vaults`
**Key:** vaults

The vaults in Files, each a file sealed under its passphrase, open or locked. A vault is copied in on a stick visit and sealed again when the session locks with changes.

## The list

**Key:** vaults.list

One row per vault: its name, its file name, where it is and its size, with its state at the right. A vault not opened since power-on is named **Locked vault**, or **New vault** while it is not yet written to a stick; once opened it goes by its own name.

An open vault offers **View contents** and **Lock**; **Lock** seals it and every other open vault. A locked vault offers **Unlock**. **Create a vault** makes a new one. With no vault in Files the list reads **No vault in Files**. The state reads **Open**, **Open** with its count of unsaved changes, **Locked**, or **Locked · not written yet**.

## Unlock

**Key:** vaults.unlock

Opens one locked vault. The vault is chosen from its rows, each with the unlock cost it was made with and the time that cost takes on this computer; then its passphrase is typed.

**Passphrase** is typed once. **Add a key file** takes a file from Files that is not a vault, and **Add a YubiKey secret** takes the 40 hexadecimal digits of the secret; a vault made with either needs it to open. **Unlock** runs the unlock; the free memory of this computer is shown beside it. With a stick attached the passphrase cannot be typed: the screen asks for the stick to be removed instead.
