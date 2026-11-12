# Restore a GPG key

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `BackupFlow`
**Key:** gpg-restore

The flow restores a GPG key made by Faraday from the paper copy of its secret (a paperkey) and its public key. The key is saved into each vault of the plan, which are then written to sticks as in the backup flow.

## Restore a GPG key

**Key:** gpg-restore.sheet

The sheet lists each public key in Files that Faraday can restore, by user ID and fingerprint; a row opens the flow on that key. A key that cannot be restored stays in the list, greyed, with the reason.

**Scan the public key** reads the public key from the camera. With no row listed, it is the way on, and the sheet says to copy the public key in on a stick visit first. **Restore from a paperkey** opens the sheet from Tools and from a vault's GPG keys; **Restore from its paperkey** opens the flow from the key's file in Files.

## The public key

**Key:** gpg-restore.head

Shows the key the paperkey is restored against: user ID, fingerprint, signing subkey and dates, and a revocation date when the key has been revoked. A key is restored only if it is an Ed25519 key with one Ed25519 signing subkey, version 4, and a user ID.

**Line 1 of its paperkey** gives the bytes that line must read, so the paper in hand can be matched to this key. **Continue** goes on to the plan.

## Where it will be kept

**Key:** gpg-restore.where

The plan of places and vaults is made as in the backup flow, with two places at least. The public key is kept with the plan while the keys are locked away.

## Bring in the sticks

**Key:** gpg-restore.sticks

Each stick that holds a vault is read here, as in the backup flow, before the key is typed.

## Vaults

**Key:** gpg-restore.vaults

Each vault is made or unlocked here, as in the backup flow, so the restored key can be saved into it.

## Type the paperkey

**Key:** gpg-restore.make

The paperkey is typed one line at a time, in hex digits, on the keyboard at the foot. Each line ends with its CRC and is checked as it ends; a line that fails keeps its digits and shows the reason.

**Scan the paperkey** fills the lines from its QR code while no line is checked. A checked line reads **Line 1 · checked**. **Type line 1 again** clears that line and every line after it. **Next line** ends the line by hand. **Restore the key** appears once every line, the CRC of the whole secret included, is checked, and saves the key into each vault of the plan.

A paperkey for another key, one protected by a passphrase, or a secret that does not make the public key is refused. A key already in an open vault is refused with **This key is in** and the vault's name.

## Lock

**Key:** gpg-restore.lock

As in the backup flow: the lock seals every vault with the key in it and wipes every key from memory.

## Write the sticks

**Key:** gpg-restore.write

As in the backup flow: each stick is written in turn, read back and compared.

## Done

**Key:** gpg-restore.done

As in the backup flow, each stick written is listed with its place and its files.
