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
