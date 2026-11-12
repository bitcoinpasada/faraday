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

## Seal count

**Key:** vaults.seals

An open vault shows how many times it was sealed, as "Sealed 12 times", with the date of the last seal. A vault is sealed when it is locked with changes; opening it and changing nothing does not count. The count is inside the encryption.

When an unlocked copy has a lower count than one seen since power-on, or than one its backup plan records, the unlock stops at **Open this copy?** with the line "An older copy: sealed 9 times." **Open** goes on with that copy and **Keep locked** leaves it locked. A copy sealed as many times with other changes reads "Another copy was sealed 12 times too, with other changes." The count shows an older copy put back by someone with access to the stick, and a copy kept on purpose, without telling them apart.

## Load from

**Key:** vaults.load

Unlocking a vault that holds wallets or keys the session lacks opens **Load from** with the vault's name: a card for each wallet with its seeds, and **Keys** for those in no wallet. **Load ticked** loads what is ticked; **Not now** loads nothing and leaves the vault open.

What is ticked at first follows each record's **Load at unlock**. **Select all** and **Unselect all** tick every card or none. A flow that asked for the unlock loads what it needs instead.
