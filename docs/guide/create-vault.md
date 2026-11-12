# Create a vault

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `CreateVault`
**Key:** create-vault

A vault is one encrypted file for a stick, opened with its passphrase. Two cards make it, Opens on and Name and passphrase; **Customise** replaces Opens on with Where will you open it? and Unlock cost.

## Opens on

**Key:** create-vault.preset

Sets the unlock cost from the machines the vault must open on. The cost is the memory and passes of each guess at the passphrase. A press chooses a row and moves on.

**PCs only** is 512 MiB and 3 passes. **PCs and a Raspberry Pi** is 64 MiB and 3 passes and opens on a Raspberry Pi 3 as well; it is chosen at first. Each row's line gives the unlock time on this computer. **Customise** opens the two cards below on the chosen row's values.

## Where will you open it?

**Key:** create-vault.where

Each machine ticked is one the vault must open on; the cost suggested next fits the machine with the least memory ticked.

**This computer**, **Other PCs**, **Raspberry Pi 5**, **Raspberry Pi 4**, **Raspberry Pi 3** and **Pi Zero 2 W** each tick or untick a kind of machine, with its memory shown. **Continue** opens Unlock cost.

## Unlock cost

**Key:** create-vault.cost

A higher cost makes each guess at the passphrase dearer and adds bits to its strength, and takes longer to unlock and needs more memory.

**Light**, **Standard**, **Medium**, **Strong** and **Maximum** are 64 MiB and 3 passes, 256 MiB and 3, 512 MiB and 3, 1 GiB and 4, and 2 GiB and 2. A row's note reads **Suggested**, **Fits**, or too large for the weakest machine ticked. **Custom** sets **Memory**, 64 MiB to 4 GiB, and **Passes**, 2 to 10, then **Continue**.

## Name and passphrase

**Key:** create-vault.phrases

**Name** is what the vault holds, for anyone who sees the stick; the file's name follows it. Left empty, the file is `vault.kdbx`. The strength line gives the passphrase's bits and the bits the unlock cost adds.

**Passphrase** is typed twice and the two must match before **Create vault** is offered. **Show passphrase** and **Hide passphrase** toggle the fields. **Add a key file** takes **Make a key file** or **Choose a file**; the key file is needed to open the vault and is kept apart from it. **Generate with dice** opens a panel of words chosen by typed dice rolls, so their bits are known: **Long list** (7776 words, five dice), **Short list** (1296 words, four dice) or **Short list 2** (1296 words, 3 edits apart). **Use these words** puts them in both fields.
