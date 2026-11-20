# Make a new vault

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `CreateVault`
**Key:** create-vault
**Words:** vault, vault-passphrase, unlock-cost, seal

A vault is one encrypted file for a stick, opened with its passphrase. Two steps make it, Opens on and Name and passphrase, one shown at a time; **Your choices** lists each answer (at the right of a wide window, from the top bar's **Your choices** button elsewhere), and pressing one opens its step; **Customize** replaces Opens on with Where will you open it? and Unlock cost.

## Opens on

**Key:** create-vault.preset

This step sets the unlock cost from the machines the vault must open on. The cost is the memory and passes of each guess at the passphrase. Pressing a row chooses it and moves on.

**PCs only** is 512 MiB and 3 passes. **PCs and a Raspberry Pi** is 256 MiB and 3 passes and opens on a Raspberry Pi 3 as well; it is chosen at first. A Pi Zero 2 W opens only a vault of 64 or 128 MiB: tick it under **Where will you open it?** or choose **Light** under **Customize**. The rows name the sizes only; a row says how much memory this computer has free only when it has too little. **Customize** opens the two cards below on the chosen row's values, and with them the summary's unlock memory, unlock time and unlock cost.

## Where will you open it?

**Key:** create-vault.where

Tick every machine the vault must open on. The cost suggested next fits the ticked machine with the least memory.

**This computer**, **Other PCs**, **Raspberry Pi 5**, **Raspberry Pi 4**, **Raspberry Pi 3** and **Pi Zero 2 W** each tick or untick a kind of machine, with its memory shown. **Continue** opens Unlock cost.

## Unlock cost

**Key:** create-vault.cost

A higher cost makes each guess at the passphrase more expensive (each guess costs about as much as 2^N plain hashes, N shown on the form), and the vault takes longer to unlock and needs more memory.

**Light**, **Standard**, **Medium** and **Strong** are 64 MiB and 3 passes, 256 MiB and 3, 512 MiB and 3, and 1 GiB and 3. A row's note reads **Suggested**, **Fits**, or too large for the weakest machine ticked. **Custom** sets **Memory**, 64 MiB to 1 GiB, then **Continue**; a vault is always written with 3 passes.

## Name and passphrase

**Key:** create-vault.phrases

**Name** says what the vault holds, for anyone who sees the stick; the file's name follows it. Left empty, the file is `vault.kdbx`. The strength line gives the passphrase's bits and what each guess costs at the unlock cost.

Type the **Passphrase** twice; **Make a new vault** appears once the two match. Each field's eye shows or hides what is typed. **Add a key file** takes **Make a key file** or **Choose a file**; you need the key file to open the vault, so keep it apart from the vault. **Generate with dice** opens a panel of words chosen by typed dice rolls, so their bits are known: **Long list** (7776 words, five dice), **Short list** (1296 words, four dice) or **Short list 2** (1296 words, 3 edits apart). **Use these words** puts them in both fields. Hardware keyboard keys are read as US, Caps Lock ignored.
