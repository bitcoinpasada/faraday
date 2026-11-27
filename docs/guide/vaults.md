# Vaults

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Vaults`
**Key:** vaults
**Words:** vault, seal, lock, session, for-the-stick, stick-visit

Vaults lists the vaults in Files, open or locked; each is a file sealed under its passphrase. A vault is copied in on a stick visit, and sealed again when the session locks with changes.

## The list

**Key:** vaults.list

One row per vault: its name, its file name, where it is and its size, with its state on the right. A vault not opened since power-on is called **Vault**, or **New vault** while it has not yet been written to a stick; once opened it goes by its own name.

An open vault offers **View contents** and **Lock**; **Lock** seals it and every other open vault. Its contents page carries **Write it to a stick**, which asks to lock first and then waits for a stick; the sealed file goes out For the stick. A locked vault offers **Unlock**. **Make a new vault** makes a new one. With no vault in Files, Vaults opens **Make a new vault** at once. The state reads **Open**, **Open** with its count of unsaved changes, **Locked**, or **Locked · not written yet**.

## Marks and Save

**Key:** vaults.marks

On a wide screen the keys and wallets this session holds stand beside the list. While exactly one vault is open each is marked **In a vault** or **Not saved**; a key with a BIP-39 passphrase is **In a vault** only where the vault holds it with that passphrase, and a key without one only where the vault holds its words with none; where the vault holds its words otherwise it reads **Words in a vault**. A row that is not saved has a **Save** button that puts it into that vault: a key without its BIP-39 passphrase (**Save with passphrase** is on the key's page in the vault). With no vault open or two open there is no mark and no **Save**; a vault open to read only marks but takes no **Save**.

## Seal count

**Key:** vaults.seals

An open vault shows how many times it has been sealed, as "Sealed 12 times", with the date of the last seal. A vault is sealed when it is locked with changes; opening it and changing nothing does not count. The count is kept inside the encryption.

If the copy you unlock has a lower count than one seen since power-on, or than one its backup plan records, the unlock stops at **Open this copy?** with the line "An older copy: sealed 9 times." **Open** goes on with that copy and **Keep locked** leaves it locked. A copy sealed as many times but with other changes reads "Another copy was sealed 12 times too, with other changes." The count cannot tell an older copy put back by someone with access to the stick from one you kept on purpose: it shows both.

## Load from

**Key:** vaults.load

Unlocking a vault that holds wallets or keys the session does not have opens **Load from** with the vault's name: a card for each wallet with its seeds, and **Keys** for those in no wallet. The filled button says what it loads, as in **Load 2 wallets and 1 key**; **Not now** loads nothing and leaves the vault open. Unlocking from the Vaults list opens the vault's contents with this sheet over them; what loads leaves you there.

A vault of one wallet set to load at unlock, and nothing else, loads it with no answer and opens it.

What is ticked at first follows each record's **Load at unlock**. **Select all** and **Unselect all** tick every card or none. A flow or a tool that asked for the unlock loads only what it needs, with no sheet, and goes back to where it was.
