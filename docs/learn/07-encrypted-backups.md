# Encrypted backups

## What it is

An OpenSigner backup is one small file, ending in .oskb, that OpenSigner writes: something locked under a passphrase chosen when it was made. Faraday does not write one. It reads one into a vault, where it is kept under the vault's passphrase from then on.

## What it can hold

One of four things. A key's seed words. A key's master seed, which is what a SLIP-39 or a Codex32 key is and which has no words. A note. Or a wallet's recovery sheet: its descriptor, its name and a note.

A key with a wallet passphrase is backed up as its words alone. The wallet passphrase is typed again when the key is loaded.

## Reading one into a vault

Copy the file onto a stick and bring it in, so it is in the Inbox, and unlock the vault it is to go into. Import into the vault on the file in the Inbox, or Import with the file's name on the vault's Bitcoin keys or Notes, asks for the backup's passphrase.

Words and a master seed become a key in the vault. A note becomes a note, and a recovery sheet becomes a recovery sheet. The file in the Inbox is not changed.

A wrong passphrase opens nothing, and Faraday says that passphrase does not open this backup. There is no other way in: without the passphrase the file is a few hundred bytes that look random.

## The memory it needs

The passphrase is stretched with Argon2id at the memory the file states in its header, 64 MiB, 256 MiB or 1 GiB, chosen when it was made. A computer that cannot spare that memory cannot open it, and Faraday says how much it needs.

## Reading one without Faraday

The OSKB format is written down in docs/BACKUP.md, in OpenSigner's source and in Faraday's, and tools/backup/decrypt.py in either opens a file on any computer with Python.
