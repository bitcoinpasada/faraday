# Flows

**Status:** draft v0.1 · 2026-10-04 · every flow, where it starts, and
where it leaves the person. Not built.

The rule this file checks: **no flow ends on a dead end.** Each one
finishes on a screen that shows what happened and offers the next thing a
person would do. Where a flow cannot go on (no vault open, no file in the
Inbox, a stick still attached), it says what is missing and offers the way
to get it.

`PLAN.md` holds the decisions; `docs/VAULT.md` the vault format;
`docs/QR.md` the scanner.

## Decisions this pass made

1. **One Files screen holds the Inbox and the Outbox.** Files read from a
   stick had nowhere to be seen until a screen asked for one. The sidebar's
   status row reads "Files · 2 in · 1 out" and opens it. Every Inbox file
   carries the action that uses it: a PSBT **Sign in Wallets**,
   `BOOTX64.EFI` **Sign in Secure Boot**, an entries file **Import into a
   vault**, a vault file **Unlock**, a received file **Show as QR**.
   Inbox and Outbox are both emptied at power-off.
2. **A new vault opens as soon as it is created**, on its first
   passphrase's slot, because a new vault is empty and the next thing is
   putting something in it. Create vault lands on the Vaults overview with
   the new vault open and **Add contents** beside it.
3. **After any lock cycle, the Inbox decides what is offered first.**
   Locking restarts the app, so the screen a person was on is gone. When
   unlocking again finds a new PSBT in the Inbox, Home leads with "Sign
   unsigned.psbt" instead of the generic tiles.
4. **A stick visit can delete a vault file.** Changing a passphrase means
   making a new vault; the old file, still openable with the old
   passphrase, stays on the stick until deleted. The visit offers **Delete
   vault-2.ofv** (a hold) and states that deleting on flash media removes
   the file, not every copy of its bytes.
5. **Power off warns when something would be lost.** If the Outbox has
   files or a vault has unsaved changes, Power off lists them and needs a
   hold.
6. **Public and secret leave by different doors** (2026-10-06). Every file
   on its way out is public (anyone may read it: descriptors, wallet files,
   account keys, PSBTs, certificates), sealed (a vault or an encrypted
   backup) or secret (anyone holding it can use what is in it). Public
   files go to the Outbox as they are. A secret goes into the open vault by
   default and leaves only inside the vault file; the one way into the
   Outbox unprotected is the secret sheet, which names the file, what it
   is and who can use it, and enables **Put it in the Outbox unprotected**
   only once the person ticks that anyone with the stick can read it. The
   Outbox lists the three apart, Public, Sealed and Unprotected secrets, a
   stick visit tags each row, and an unprotected secret is not ticked for
   writing until the person ticks it on the visit. A secret let out this
   way is marked as one whatever its text reads as (a node key, a child
   seed's words), and the mark is kept across a lock. A seed's words file
   is a secret by its kind. The code refuses a
   secret on the plain Outbox path, so a new flow cannot skip the sheet.
   Today's secret is the FROST carry (the next share's secret nonce): into
   the vault it is a signing-round record (`docs/VAULT.md` §7, type 9),
   its public PSBT goes to the Outbox, the next device finds the round by
   the transaction when it opens that PSBT with the vault unlocked, and
   the round leaves the vault once it has signed. Create a wallet says the
   same in its cards: after Check come **Secrets into a vault** (the keys
   held here and the wallet), then **Public files** (descriptor, wallet
   file, multisig config, backup sheet, each key's account key, the
   descriptor QR), then **Paper backup**.

7. **Words for keys** (2026-10-06). A **key** or **seed** is what signs:
   words, a SeedQR, a share of one. An account's public key with its
   origin, `[fingerprint/path]xpub`, is an **xpub**, and a wallet's
   description is a **descriptor** (or its split sheets). No file name,
   label or sentence calls an xpub or a descriptor a key; the test
   stick's files are named `seed-…` and `xpub-…` accordingly.

## Session

**Boot.** Firmware loads the system → the boot medium's data partition
copied into memory: vault files into the Inbox, every other file held
apart for the import, a PNG as what its QR codes hold → the import sheet
over Home: what was copied, **Remove the stick** (Pi: the card) → the
moment no removable partition remains, the same sheet:
- each vault from the medium → **Unlock** → the Unlock screen → back to
  the sheet, with the vault's wallets and keys added;
- the wallets found, once each, ticked, each with whether it can sign
  here; the keys no wallet uses, ticked; every file for the Inbox, PSBTs
  and vaults ticked;
- **Import** → the chosen wallets, keys and files in; the rest wiped →
  Home;
- **Import later**, or a tap beside the sheet → Home, the files waiting;
  Sticks (sidebar or Home) opens the sheet again;
- nothing on the medium but the settings file → Home: **Create a vault**,
  or the Wallets tab without one (keys typed or scanned for the
  session).

**Unlock.** Choose a vault (its memory need shown first) → passphrase →
- opens → Home, or the Inbox's next step (decision 3);
- wrong passphrase → stays, says so; no lockout (an offline copy can be
  attacked without the device anyway);
- needs more memory than is free → said before the passphrase field;
  choose another vault;
- a stick is attached → "Remove the stick to unlock", which clears itself
  when the stick goes.

**Lock** (sidebar, Home's vault card, a vault's page, or 5 minutes idle) →
vaults sealed into the Outbox → the Wallets session wiped → app restarts → Unlock.
After 10 more idle minutes with an empty Outbox, power off; with anything
in it, stay on Unlock and list what is waiting.

**Power off** → decision 5 → off.

## Sticks and cards

**A stick or card inserted while unlocked** → **Lock to use this stick**
(what is sealed, what is wiped, what waits) →
- **Lock and use stick** → lock → **Stick visit**;
- **Not now** → nothing is handed out; a banner says "Remove the stick to
  keep working" until it goes.

**A stick or card inserted while locked, or with no vault open** → **Stick
visit** directly.

**Stick visit** (one screen, both directions):
- **Write from the Outbox**: chosen files written, read back, compared.
  A sealed vault replaces the file with its identity, or is written
  under its own name (`name.ofv`, `name-2.ofv` when the stick has one;
  `docs/VAULT.md` §6) where there is none. A failed comparison
  leaves the file in the Outbox and says which. The first row is the
  settings, `faraday-settings.txt`, which is not an Outbox file: ticked
  on the boot stick when the settings differ from what the boot stick
  holds, unticked on any other stick, and written over the settings
  file already there.
- **Copy into the Inbox**: chosen files; kinds Faraday does not read
  are shown dimmed. A vault file copied in appears on Vaults, locked. A
  PNG is ticked like any other file, and Select all includes it, but is
  not copied: the disk process decodes it, and each code it finds is
  routed as a scan would be (`docs/QR.md` §2), with a multi-part transfer
  showing its progress across images. A JPEG is not read.
- **Delete a vault file** (decision 4).
- Ends on **Remove the stick** → once gone → **Unlock again**, or Home if
  nothing was locked.

**A stick pulled during a write** → the write is reported failed, the file
stays in the Outbox, and the visit screen says to put the stick back.

## Vaults

**Vaults overview**: each vault in memory with its state (open, locked,
not written yet), memory need and source; **Create a vault** at the top.
- open vault → **Open** → Vault contents;
- locked vault → **Unlock** → passphrase sheet → Vault contents.

**Create a vault** (step cards: where it opens, unlock cost, space per
passphrase, passphrases) → **Create vault** → Vaults overview, new vault
open (decision 2), sealed copy in the Outbox → **Add contents** → Vault
contents, empty, with an Add action per kind.

**Vault contents**: kinds → items → details.
- **Add an entry** → form (title, username, password with **Generate with
  dice**, URL, notes, TOTP secret typed or **Scan**) → saved → the entry
  selected in the list; vault marked unsaved.
- **Import entries** (from an Inbox file) → preview: entries found, lines
  skipped with the reason → choose the vault → import → Entries list.
- **Edit** → the same form → back to the item.
- **Hold to delete from vault** → gone; the next item selected.
- A Bitcoin key or wallet → **Open in Wallets**, **Open in OpenSigner** (a key), or **Don't load at unlock**.
- A GPG key → **Export public key**, **Sign a file**, **Renew**.
- Secure Boot keys → **Open Secure Boot**.

**Change passphrases, cost or space** → **Make a new vault from this one**
→ Create vault, filled in from this vault, with this slot's contents
carried over; the form says that slots whose passphrases are not entered
are not carried (`docs/VAULT.md` §8) → new vault created → the old one
offered for **Remove from this session**, and decision 4 for the stick.

## Wallets tab

Every flow, entry point and step is in `docs/WALLETS.md`. What it
guarantees for this file's rule:

- **Every way in reaches a wallet card** (`docs/WALLETS.md` §2) that says what
  the wallet is, which keys can sign here, how many signatures the
  transaction has and needs, and the one thing still to do.
- **A transaction never stops at "can't sign"**: a missing key opens **Add
  its key** on its slot; a missing signature opens **Collect**; a missing
  wallet opens **Load the wallet**.
- **Signing ends on two results**, the signed PSBT for the wallet that wrote
  the transaction and the finished transaction with its txid, each shown as
  a code and offered to the Outbox ("Insert a stick to write it").
- **Anything added in a session offers Save to vault**, and locking lists
  what was not saved.
- **Creating a wallet ends in Backup and Save**, never on the descriptor
  alone.

## Spend tab

Family mode for someone spending for the first time
(`docs/FAMILY.md`): the map → the stick → **What are you holding?** (a
Faraday stick and its passphrase, words only, or words and a
description) → **Open the wallet** (the vault's Unlock and load, word
entry, or the description by camera or from Files) → check the money in
Sparrow → write the payment in Sparrow → bring the PSBT (camera, or a
stick: the lock cycle, after which the tab opens on the same page) → the
spend's own steps → **Put everything away** (lock, the Outbox to a
stick, power off). Every page that cannot be reached yet sends the
person to the one that comes first.

## OpenSigner tab (kept for now)

OpenSigner's own screens and flows, unchanged. From a vault, **Open in
OpenSigner** on a Bitcoin key runs OpenSigner's load flow with the key
already supplied, ending on its key page. Its encrypted-backup export
offers **Import into a vault**; its PSBTs and public exports go to the
Outbox ("Insert a stick to write it").

## GPG

Every GPG action needs an open vault (the secret key lives there). With
none open: "Unlock or create a vault first", with both buttons.

**Make a key** (step cards: name and email, expiry, vault) → created →
result: fingerprint, **Export public key** (code or Outbox),
**Revocation certificate** (Outbox, with the note that it should be kept
apart from the key), **Paperkey backup** (shown; a hold to reveal) → the
key's page in Vault contents.

**Sign a file** → choose an Inbox file → choose the key → detached
signature to the Outbox → "Insert a stick to write it". No Inbox file:
"Copy the file in on a stick visit first."

**Renew** → new expiry → updated public key to the Outbox.

## Secure Boot

Needs an open vault for the keys.

**Make keys** → PK, KEK, db generated into the vault → Secure Boot screen.

**Enrolment files** → policy (Windows-compatible or own keys only) → **Put
in Outbox** → Learn page: enrolling in the firmware's setup screen.

**Sign `BOOTX64.EFI`** → needs the file in the Inbox (none: "Copy
BOOTX64.EFI in from a stick first", with what a stick visit is) → build
record match → fingerprint compared through another channel → **Sign with
db** → signed file to the Outbox → Learn page: copying it onto the boot
partition on another computer.

**Check a signature** → result against the vault's db certificate.

## QR transfer tab

**Receive** → scanner (any payload) → routed by `docs/QR.md` §2; a file
lands in the Inbox and Files opens on it.

**Send** → choose: an Outbox file, an Inbox file, or a public item from an
open vault (an account key, a descriptor, a GPG public key) → format
(UR, BBQr, numbered parts where they apply) → animated code with frame
rate and progress → **Done**. A secret item is never offered here; seed
codes are shown only from the Wallets tab's backup step.

## New devices and settings

**A keyboard or pointer arrives** → it does nothing until it types the
code shown, or clicks the target drawn → **Ignore this device** lists it
in Settings until unplugged.

**No camera, or the camera is refused** → the scanner says which, and
offers Type and Read a file.

**Settings**: keyboard layout (US first), display scale, idle lock and
power-off times, reduce motion, light or dark, ignored devices, About
(version, build hash). They are kept across a lock in RAM, and across a
power-off in `faraday-settings.txt` on the boot stick, read at boot
before the passphrase (`PLAN.md` §5.2). Never for an idle time, the
signed-amount memory's seal and the network are not read from the file.
