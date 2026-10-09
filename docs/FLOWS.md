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
   The FROST carry (the next share's secret nonce) goes into
   the vault as a signing-round record (`docs/VAULT.md` §7, type 9),
   its public PSBT goes to the Outbox, the next device finds the round by
   the transaction when it opens that PSBT with the vault unlocked, and
   the round leaves the vault once it has signed. A wallet's seed, from
   the backup's checklist (2026-10-09), goes in as a key record
   (`kind::KEY`, as Vaults saves it), never a note; as a file it is the
   words or the SeedQR picture, one picked on the sheet, behind its own
   acknowledgement ("anyone who copies the stick or sees this file can
   spend these coins"), and never holds the BIP-39 passphrase. A seed's
   SeedQR never goes through the QR sheet's **PNG to the Outbox**, which
   is for public content. Create a wallet says the
   same in its cards: after Check come **Secrets into a vault** (the keys
   held here and the wallet), then **Public files** (descriptor, wallet
   file, multisig config, backup sheet, BSMS record, Bitcoin Core import,
   each key's account key and its BSMS record; the descriptor, the
   multisig config, the BSMS records and the keys each with **Show as QR**
   and **PNG** beside **To the Outbox**), then **Paper backup**: the seeds copied by hand, each
   copy checked by scanning its drawn SeedQR (by its words' typed numbers
   where there is no camera).

7. **Words for keys** (2026-10-06). A **key** or **seed** is what signs:
   words, a SeedQR, a share of one. An account's public key with its
   origin, `[fingerprint/path]xpub`, is an **xpub**, and a wallet's
   description is a **descriptor** (or its split sheets). No file name,
   label or sentence calls an xpub or a descriptor a key; the test
   stick's files are named `seed-…` and `xpub-…` accordingly.

8. **What loads a key asks for the stick to be pulled** (2026-10-08).
   Keys still load only with no stick attached, but nothing that loads one
   is shown disabled for it: Add a key, Make a new key, Load this key, a
   Tools tile that adds or makes a key, opening a seed from the Inbox or a
   backup. Pressed with a stick in, each opens a sheet, "Pull the stick to
   add a key" (or to make a key, …), the stick's label, and **Cancel**.
   The sheet closes the moment the last stick goes, and what was pressed
   carries on; Cancel closes it with nothing done.

9. **Every scan waits for the stick to be out** (2026-10-09). A camera
   frame may picture a seed, so the scanner opens only with no stick
   attached: Scan, Scan a SeedQR, Scan your copy and a vault entry's Scan,
   pressed with a stick in, open the same sheet, "Pull the stick to scan",
   and the camera turns on once the stick is out. A stick plugged in while
   the camera is on is held back, nothing read from it, and arrives as any
   stick does once the camera is off. Signing by QR alone needs no stick
   and is unchanged.

10. **Whatever needs a vault leads into making or unlocking one, and
    back** (owner, 2026-10-09). Where a flow needs an open vault and none
    is, its button is **Make a vault** when no vault file is in Files,
    **Unlock {name}** when one is, and **Unlock a vault** (the Vaults
    list) when several are; never a disabled button or a line to go to
    Vaults. Create a vault, opened this way, carries a chip at its head,
    "Then: {flow}", and its way back returns to the flow. Making the vault
    goes straight to Unlock with it picked: the passphrase is typed once
    more, which shows the person has it. Unlocking needs the stick out:
    with one in, the Pull sheet asks first ("Pull the stick to unlock").
    Once unlocked, the flow is back on the step it was on, with the
    vault's button live. The places: Tools' **GPG key**, **Secure Boot
    keys** and **KeePass export** ("Make a vault first" / "Unlock a vault
    first" on the tile, pressable; back to that category of the vault
    unlocked, or to Tools when the way back is taken); BIP-85, the
    Lightning node key and the Silent payments scan key (**Save into
    {vault}**); the secret sheet (the FROST carry, a seed from the backup's
    files item), which closes on the way, keeps its secret, and opens
    again on return; the backup's seeds and wallet vault items; Files (an entries file, an
    `.oskb`, a file added to a vault); Vault contents with none open; and
    Restore's seeds route, which offers Unlock only, as a new vault holds
    no seed. Create a wallet's vault step keeps **Make a vault** /
    **Unlock {name}** / **Not now**. Going back anywhere on the way, or
    leaving for another page, ends it: a secret held for it is dropped,
    wiped, as Cancel on the sheet does.

## Home

**Start** leads with **Add a key** (type, scan or bring in a seed) →
the Add a key screen: a form (BIP-39 words, SLIP-39 shares, codex32, Seed
XOR parts), **Scan a SeedQR**, **Make a new key**, and "From a stick ·
plug it in" (with a stick attached, **Copy files in** → Stick visit; a key
file copied in loads when the stick is pulled). **Scan** floats in the
bottom-right corner of Home on every panel → the scanner (with a stick
in, the sheet asks for it to be pulled first). Home's Scan takes a seed
too: a SeedQR, a CompactSeedQR or a seed's words go to Add a key and are
added as **Scan a SeedQR** adds them, never into Files or the Inbox; a
private key or xprv is named, since Add a key does not take one. The
Wallets tab
carries **Add a key** beside Create a wallet and Load or restore a wallet.
A key added with no wallet loaded lands on Wallets, whose card says
"Key 73c5da0a is loaded. No wallet uses it." with **Make a wallet from
this key** (Restore's seeds card with the key in, at the shape: 1 of 1,
Native SegWit, account 0 → **Make the wallet**), **Add another key** (the
same card at 2 keys, for a multisig) and **Load or restore a wallet**.
Several such keys: one line each, the buttons on the one picked. Back
from the seeds card returns to Wallets with the key still loaded.

## Session

**Boot.** Firmware loads the system → the boot medium's data partition
copied into memory: vault files into the Inbox, every other file held
apart for the import, a PNG as what its QR codes hold → the import sheet
over Home: what was copied, **Remove the stick** (Pi: **Remove the SD
card**) → the
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

On the Pi every medium is an SD card and every screen says so: "SD card"
for "stick", **SD card visit** for **Stick visit**, **Lock and use SD
card**, "Remove the SD card" (`docs/PLANNING.md` §16.144). The flows
below use the PC's words.

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
- **Copy into the Inbox**: chosen files, any of them; a file Faraday
  reads as nothing it knows comes in as a File, for **Sign a file** or
  QR **Send**, and is dropped at a lock. A vault file copied in appears
  on Vaults, locked. A
  PNG is ticked like any other file, and Select all includes it, but is
  not copied: the disk process decodes it, and each code it finds is
  routed as a scan would be (`docs/QR.md` §2), with a multi-part transfer
  showing its progress across images. A JPEG's codes are not read; it
  comes in as a File.
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
- opened from a flow that needs a vault (decision 10): **Unlock** →
  the flow, on the step it was on; the way back returns to it.

**Create a vault** (step cards: where it opens, unlock cost, space per
passphrase, passphrases) → **Create vault** → Vaults overview, new vault
open (decision 2), sealed copy in the Outbox → **Add contents** → Vault
contents, empty, with an Add action per kind. Opened from a flow that
needs a vault (decision 10), it shows "Then: {flow}" by its title, and
**Create vault** goes to Unlock with the new vault picked, then back to
the flow.

**Vault contents**: kinds → items → details.
- **Add an entry** → form (title, username, password with **Generate with
  dice**, URL, notes, TOTP secret typed or **Scan**) → saved → the entry
  selected in the list; vault marked unsaved.
- **Import entries** (from an Inbox file) → preview: entries found, lines
  skipped with the reason → choose the vault → import → Entries list.
- **Edit** → the same form → back to the item.
- **Hold to delete from vault** → gone; the next item selected.
- A Bitcoin key or wallet → **Open in Wallets**, **Open in OpenSigner** (a key), or **Don't load at unlock**.
- A GPG key → **Export public key**, **Revocation certificate**, each with
  **Show as QR** and **PNG** on its row; **Sign a file**, whose list has
  **Show as QR**, **PNG** and **Sign** on each file's row; **Renew**.
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
- **The backup starts with a plan, then a checklist of only what it
  needs** (owner, 2026-10-09, proposal A; `docs/WALLETS.md` §5). Three
  presets (**Paper only**, **Paper and vault**, **Paper, vault and
  watch-only software**) fill the plan's questions, each a multi-choice
  list (DESIGN §4.2): where the seeds go, the places (how many, share or
  whole sheet, which keep the vault's stick, their names with a vault
  open), where the wallet description goes, the software and form, each
  passphrase's places. Beside them the map (one box per place, the
  vault, a stick of files, the software, the seeds on their own
  devices, each tagged secret, sealed or public) and the check: "Any one
  place lost: the rest rebuild the wallet", "One place found: can
  spend", "One place found: sees the balance", qualified "with the
  vault's passphrase" where the vault is what decides it. **Make the
  checklist** replaces the questions with only the items the plan needs,
  each done by what it does (a file in the Outbox, a seed or the wallet
  in the vault, a copy matched, the descriptor shown); the envelopes
  alone by **Done**. A place's name lives only in the vault (record
  type 11, `docs/VAULT.md` §7), which the next backup of the wallet
  starts from; no file, sheet, PDF, PNG or Outbox item holds one, and
  the blank template carries empty "Place ____ holds ____" lines
  instead. On a small panel: one question per page, the map as the
  plan's last page and in the envelopes item.
- **A seed's backup: by hand, into a vault, a file only past the secret
  sheet** (owner, 2026-10-09; `docs/WALLETS.md` §5 step 3). Paper stays
  first: a copy item per seed; the vault item has **Save into {vault}**
  (and "with its passphrase" when loaded with one) under each seed; the
  files item **Save as a file…**, which opens the secret sheet for the
  seed. The same on a small panel.
- **The backup says where each seed is kept** (owner, 2026-10-09;
  `docs/WALLETS.md` §5). The map on the side panel marks each line
  with its item's state; the copy page on a small panel lists the wallet (in which open vault, or
  not in one) and every key of the wallet by fingerprint: in which open
  vault, whether its copy by hand was checked, whether its file is in
  the Outbox unprotected, or not here (backed up on its own device). With
  vault files and none open, it says the vaults were not checked.
- **Any loaded wallet backs up again, with every file on offer**
  (owner, 2026-10-09; `docs/WALLETS.md` §5). Entries: the wallet's card
  (**Back up**, also on the small panel), Start's **Back up a wallet**,
  the Tools tile, Create's last step and Restore's **Back it up
  again**. With more than one wallet loaded the head is "Back up a
  wallet" with the wallet's name as a chip, which lists the loaded
  wallets; a press on one starts the backup on it. A watch-only wallet
  (no seeds here) is asked nothing about seeds. The checklist's public
  files item and Create's Public files card are one list drawn from the
  wallet (descriptor, wallet file, multisig config, BSMS record, Bitcoin
  Core import, each key held here and its BSMS key record), the item
  showing the rows the plan's software picks. **Save the wallet into
  {vault}** is its own item (or the way to make or unlock one, back to
  the item); the seeds' vault item has the seeds'.
- **The descriptor and its shares as pictures** (2026-10-09). Create's
  Public files card and the backup's public files item (**PNG** on the
  descriptor's row) write `{name}-descriptor.png`: the
  checksummed descriptor as one static code, with the wallet's name, its
  shape (and the network when not mainnet), the keys' fingerprints, the
  descriptor's checksum and "Public: watch only, spends nothing" under
  it. Past one code (version 25 at ECL M) it goes as the BBQr parts the
  QR view makes, `{name}-descriptor-1-of-3.png` and on, each labelled
  with its part. **Shares to the Outbox** writes each split share's
  `{name}-share-k-of-n.png` beside its PDF and text: the share's text as
  one code, labelled with the share's number and quorum, the keys it
  holds and leaves off, and "Not a wallet on its own". A scan of a share
  picture comes back as the share, and Restore takes any quorum of them.
- **Every public file a flow makes as a code and a picture**
  (2026-10-09; `docs/QR.md` §4 item 4). Beside a file's **To the
  Outbox** sit **Show as QR** and **PNG** where a wallet or a person
  reads it from a code: Create's Public files card (descriptor, multisig
  config, BSMS descriptor record, each key held here and its BSMS key
  record), Create's Keys card (**Show xpub QR**, **Xpub PNG to the
  Outbox**, **Xpub file to the Outbox** under each key held here), the
  backup's public files item (the same rows as Create's card), Sign a message
  (**Put in the Outbox**, **Show as QR**, **PNG to the Outbox**) and
  Silent payments (**Record to the Outbox**, **Record as QR**, **Record
  as PNG**). The QR sheet carries **PNG to the Outbox** (**PNG** beside
  the format on a small panel) whenever it shows one code of public
  content; a row's **PNG** is that button without the sheet, and says
  "{name} is in the Outbox". Labels under the code: `Key {fp} · {kind}
  · {path}` for a key, `Silent payment address · {fp}` for the silent
  payments address and record, `Signed by {address}` for a message, the
  wallet's shape and keys for its files. A file past one code says so
  instead of writing parts; a PSBT, a transaction, a file shown from
  Files and a secret offer no picture.
- **A key in no wallet has a way on**: the card's buttons above with no
  wallet loaded; with wallets loaded, the key's row in the left column
  carries **Make a wallet from this key**. The left column runs top
  down, wallets, keys in no wallet, then its buttons, and scrolls when
  taller than the screen.
- **Restore** takes the wallet's description (a stick, a vault, paper) or
  **Type the seeds**: each seed through Add a key and back, **Add another
  key**, then **Make the wallet**: M of N on two sliders, the cosigners'
  xpubs, the kind and the path, the first address, Check.

## Spend tab

Family mode for someone spending for the first time
(`docs/FAMILY.md`): the map → the stick → **What are you holding?** (a
Faraday stick and its passphrase, words only, or words and a
description) → **Open the wallet** (the vault's Unlock and load, word
entry with **Add another seed** for a wallet of several seeds: M of N, the
cosigners' xpubs, the kind and the path; or the description by camera or
from Files) → check the money in
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
none open, Tools' **GPG key** tile reads "Make a vault first" (no vault
file) or "Unlock a vault first" and leads into Create a vault or Unlock,
which come back to the vault's GPG keys (decision 10).

**Make a key** (step cards: name and email, expiry, vault) → created →
result: fingerprint, **Export public key** (code or Outbox),
**Revocation certificate** (Outbox, with the note that it should be kept
apart from the key), **Paperkey backup** (shown; a hold to reveal) → the
key's page in Vault contents. The public key and the revocation each
have **Show as QR** and **PNG** on their row: `{name}-{fp8}.png` with the
user ID and fingerprint under the code, and
`{name}-{fp8}-revocation.png` labelled "Revokes {user ID}" (an Ed25519
certificate is one code, about 700 bytes armoured).

**Sign a file** → choose an Inbox file → choose the key → detached
signature to the Outbox → "Insert a stick to write it". Each file's row
also shows the signature as a code (**Show as QR**) or writes it as
`{file}-signature.png` (**PNG**). No Inbox file: "Copy the file in on a
stick visit first."

**Renew** → new expiry → updated public key to the Outbox.

## Secure Boot

Needs an open vault for the keys. With none open, Tools' **Secure Boot
keys** tile leads into Create a vault or Unlock and back, as GPG's does;
**KeePass export** the same, to the vault's entries.

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
