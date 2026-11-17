# The backup flow

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `BackupFlow`
**Key:** backup-flow

This flow plans where a key will be kept, reads its sticks and makes or unlocks its vaults before the key exists, then makes the key, locks once and writes each stick. It is used for a GPG key, Secure Boot keys, or the vaults and sticks of a wallet.

## Where it will be kept

**Key:** backup-flow.where

Each row is a place: a stick and the vaults it keeps. For a wallet the card is **The plan**, the same board and cards as in Back up.

**A new vault** gives a place a vault of its own. **Also Vault 2** adds a second vault to the place, and **One stick for both** puts both on one stick. **Public files here** marks the place whose stick also carries the public files. **Add a place** adds a place; **Remove** removes one, down to the fewest. **I destroyed this copy** takes a written place off the plan. **An existing vault** uses a vault that is already on a stick. **Continue** is grey while there are fewer than 2 places.

## Bring in the sticks

**Key:** backup-flow.sticks

Every stick that holds a vault is read in this step, before any key exists, so that each vault has a stick that works. The read checks that the stick can be written, how much room it has and which vaults it holds. Nothing is written to a stick yet; the vaults go onto it after the keys are saved and memory is locked.

**Lock and bring in the sticks** locks first, if a key or a vault is open. **Check it later** postpones a stick whose vaults are all new to the write step; its vaults are made now and kept sealed in memory until it is written. **Use another stick** refuses the stick in. On a stick that already holds vaults, choose one of them or make **A new vault beside them**. **Continue** waits until every stick is checked or postponed.

## Vaults

**Key:** backup-flow.vaults

Each vault is made or unlocked in this step, so that the key can be saved into it.

**Create Vault 1** opens Create a vault for that vault. **Open Vault 1** asks for its passphrase and unlocks it. If you do not have the passphrase, **I do not have its passphrase** offers **A new vault beside it** or another stick. A vault cannot be unlocked while a stick is attached.

## Make the key

**Key:** backup-flow.make

This step makes the key the flow is for. A wallet's keys are made on Make a new wallet's Keys card instead, so for a wallet this card only sends you back there.

A GPG key takes **Name**, **Email** and **Expires**. Secure Boot keys take **Owner name**. Once made, the key is saved into each vault of the plan and the Lock card opens by itself.

## Lock

**Key:** backup-flow.lock

Locking seals every vault that has the key in it and wipes every key from memory. **Kept** lists what is kept and where, and **Wiped from memory** lists what is wiped. A key kept nowhere else is shown in the error tone. The card names the key and its vault. **Lock and write** locks once.

## Write the sticks

**Key:** backup-flow.write

The sticks are written one at a time, by place: its files are written, read back and compared. Pull a stick only once its copy is checked.

A stick pulled before the check does not count, and you plug it in again. **Use another stick** refuses the stick in.

## Done

**Key:** backup-flow.done

This lists each stick written, with its place, label and time, and the files on it. **Unlock Vault 1** opens the vault with its passphrase; for Secure Boot keys it reads **Unlock Vault 1 for the enrolment files**. A wallet with no vault has **Home**.
