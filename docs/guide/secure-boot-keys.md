# Secure Boot keys

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `BackupFlow`
**Key:** secure-boot-keys

This flow makes three RSA-2048 keys, the platform key (PK), the key exchange key (KEK) and the signature database key (db), each with a self-signed certificate valid for 30 years. It runs inside the backup flow and keeps the keys at two places or more, because seed words cannot make them again.

## Where it will be kept

**Key:** secure-boot-keys.where

You make the plan of places and vaults as in the backup flow, with at least two places. The private keys are kept in one vault record. Until every place is written, the item offers **Back up first** in place of the enrolment files and nothing is signed.

## Bring in the sticks

**Key:** secure-boot-keys.sticks

As in the backup flow, you bring in each stick that holds a vault before the keys exist, and Faraday reads it here.

## Vaults

**Key:** secure-boot-keys.vaults

As in the backup flow, you make or unlock each vault here so the keys can be saved into it. Keys in a vault unlocked from a stick sign at once.

## Make the key

**Key:** secure-boot-keys.make

**Owner name** names the certificates: the name followed by PK, KEK or db, or Faraday PK, Faraday KEK and Faraday db when the field is empty. The three keys are listed under one owner GUID, made with them. The keys are drawn from the system's randomness and the certificates carry the time they were made, so Faraday asks for the time first. **Make the key** saves the keys into each vault of the plan.

## Lock

**Key:** secure-boot-keys.lock

As in the backup flow, the lock seals every vault with the keys in it and wipes every key from memory.

## Write the sticks

**Key:** secure-boot-keys.write

As in the backup flow, each stick is written in turn, read back and compared.

## Done

**Key:** secure-boot-keys.done

As in the backup flow, each stick written is listed with its place and its files. You use the keys from the vault's Secure Boot item, which has **Hold for enrolment files** and **Sign or check an image**.

## Enrolment files

**Key:** secure-boot-keys.done-enrolment

**Hold for enrolment files** writes nine files to the stick: `PK.auth`, `KEK.auth` and `db.auth`, the signed updates; `PK.esl`, `KEK.esl` and `db.esl`, the signature lists; and `PK.cer`, `KEK.cer` and `db.cer`, the certificates. Different firmware setup screens take different ones of these. Faraday never writes a setting of the computer: the keys are enrolled in the firmware's own setup screen.

A computer's firmware keeps one PK, a KEK list and a db list. An update to the KEK list must be signed by the PK, and an update to db by a KEK. With Secure Boot on, a boot program starts only when its signature verifies with a certificate in db.

## What enrolment adds

**Key:** secure-boot-keys.done-adds

**Windows-compatible** adds four of Microsoft's certificates to the keys: Microsoft Corporation KEK CA 2011 and Microsoft Corporation KEK 2K CA 2023 to the KEK list, Microsoft Windows Production PCA 2011 and Windows UEFI CA 2023 to db. Windows and its updates still start. Microsoft's third-party certificate is not added, so a computer whose graphics card, disk controller or network card firmware Microsoft signed under it may not start. The line under the choice says so in the warning colour. Restore Factory Keys in the BIOS undoes the enrolment.

**Own keys only** adds none: only what the db key signed starts, and Windows does not. With either choice the files are written by a held press, and nothing is written until the hold completes.

## Sign or check an image

**Key:** secure-boot-keys.done-sign

**Sign** signs a release's `BOOTX64.EFI` with the db key: an Authenticode signature over the file's SHA-256 image hash, added at the end of the file. It signs only when `SHA256SUMS` and `SHA256SUMS.asc` are beside the file in the Inbox, the signature verifies with the release key built into Faraday, `SHA256SUMS` lists the file under its release name with its SHA-256, and the release is at least as new as the Faraday signing it. Otherwise the page names the refusal and nothing is signed.

**Check** reads a file's signature against the vault's db certificate: signed by this vault's db key, signed by another key, changed since it was signed, or not signed.
