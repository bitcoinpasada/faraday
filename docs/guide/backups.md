# Backups

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Backups`
**Key:** backups

Every wallet known and where its backup is. A wallet is listed once, whether it is loaded, in an open vault, or only remembered from a vault now locked.

## Wallets

**Key:** backups.wallets

One row per wallet, with its name and shape; choosing a row shows its card. With no wallet the page reads **No wallet**. On a small panel one wallet is shown at a time, counted **Wallet 1 of 3** (the numbers vary), and **Previous** and **Next** move between them.

## The card

**Key:** backups.card

The card names the wallet and its shape, then lists where its backup is, one line per place. **Back up** opens the backup of a loaded wallet, and reads **Continue the backup** for one left part way. A wallet with no plan reads **No backup plan**. A wallet of a locked vault shows only its plan, with the vault's name and that it is locked.

A line gives the place, what it holds (or **Nothing**), a tag and what was seen of it. The tag is secret, sealed or public: secret holds the key as it is, sealed holds it under a vault's passphrase, public holds nothing that spends.

## What was seen

**Key:** backups.state

A place of paper is named **Place 1**, **Place 2** and on, or by the name given it, and reads **Checked** or **Not checked**. A **Vault** (or **Vault 1**, **Vault 2**) reads unchanged or changed since written, **Not saved yet**, or **A vault is locked**.

Public files read the stick and the time of the last visit that wrote them, or the stick they wait for, or **Not made**. **Watch-only software** reads **Shown** or **Not shown**. **Own devices** reads **Not here**, as nothing there can be seen. A seed file waiting for a stick, or written to one, reads as unprotected, in the error tone. **Not seen** stands for anything else.
