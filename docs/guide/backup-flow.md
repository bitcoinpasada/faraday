# The backup flow

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `BackupFlow`
**Key:** backup-flow

The flow plans where a key will be kept, reads its sticks and makes or unlocks its vaults before the key exists, then makes the key, locks once and writes each stick. It makes a GPG key, Secure Boot keys, or the vaults and sticks of a wallet.

## Where it will be kept

**Key:** backup-flow.where

One row per place, each place a stick with the vaults it keeps. For a wallet the card is **The plan**, the same board and cards as in Back up.

**A new vault** gives the place a vault of its own. **Also Vault 2** adds a second vault to the place; **One stick for both** puts both on one stick. **Public files here** marks the place whose stick also carries the public files. **Add a place** adds a place; **Remove** removes one, down to the fewest. **I destroyed this copy** takes a written place off the plan. **An existing vault** uses a vault already on a stick. **Continue** is grey with fewer than 2 places.

## Bring in the sticks

**Key:** backup-flow.sticks

Each stick that holds a vault is read here, before any key exists, so every vault has a stick that works. The read checks that the stick can be written, its room and its vaults. Nothing is written to a stick now; the vaults go onto it after the keys are saved and memory is locked.

**Lock and bring in the sticks** locks first, while a key or a vault is open. **Check it later** puts a stick whose vaults are all new off to the write step; its vaults are made now and kept sealed in memory until it is written. **Use another stick** refuses the stick in. On a stick that holds vaults already, a vault is chosen from them or **A new vault beside them** is made. **Continue** waits until every stick is checked or deferred.

## Vaults

**Key:** backup-flow.vaults

Each vault is made or unlocked now, so the key can be saved into it.

**Create Vault 1** opens Create a vault for that vault. **Open Vault 1** asks for its passphrase and unlocks it. **I do not have its passphrase** offers **A new vault beside it** or another stick. A vault cannot be unlocked while a stick is attached.

## Make the key

**Key:** backup-flow.make

Makes the key the flow is for. A wallet's keys are made on Make a new wallet's Keys card instead, so this card for a wallet only returns there.

A GPG key takes **Name**, **Email** and **Expires**. Secure Boot keys take **Owner name**. Once made, the key is saved into each vault of the plan.

## Lock

**Key:** backup-flow.lock

The lock seals every vault with the key in it and wipes every key from memory. **Kept** lists what is kept and where; **Wiped from memory** lists what is wiped. A key kept nowhere else is shown in the error tone. **Lock and write** locks once.

## Write the sticks

**Key:** backup-flow.write

Each stick is written in turn, by place: its files are written, read back and compared. A stick is pulled once its copy is checked.

A pull before the check counts for nothing; the stick is plugged in again. **Use another stick** refuses the stick in.

## Done

**Key:** backup-flow.done

Lists each stick written, with its place, label and time, and the files on it. **Unlock Vault 1** opens the vault with its passphrase. A wallet with no vault has **Home**.
