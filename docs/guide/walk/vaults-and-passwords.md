# Vaults, passwords and codes

**Kind:** Spec
**Status:** Written and checked against the app's code and strings; the pictures are named and not yet drawn
**Open:** the pictures, drawn once after the theming has landed

**You end with:** a vault of your own on a stick, holding key
`3dca6805`, a login with its TOTP codes and a note; the vault written,
locked and unlocked again, and the login exported for KeePass.

**Before you start:** [Make a key](new-key.md). [Back it up](back-up.md)
makes a vault inside a backup's checklist
([step 4.8](back-up.md#48-the-vault)); this page makes
one on its own, for keys and for everything else.

**You'll need:**

- Faraday with key `3dca6805` loaded and no stick attached.
- A USB stick other than the Faraday stick, formatted FAT32.
- For a passphrase from dice: five dice, or one die rolled five times.
- For the codes: a site's TOTP setup code (its QR code or its key),
  and a phone or watch showing the time to the minute.

**Time:** about 30 minutes.

A vault is one file, `.ofv`, sealed under one to four passphrases.
Faraday opens it in memory, seals it again at every lock, and writes it
to a stick on the next stick visit. The passphrases on this page are
examples only: never use a passphrase printed anywhere, this page
included.

## 1. Create a vault

On **Home**, press **Create a vault** (**None yet**), or press
**Vaults** in the sidebar, then **Create a vault** at the top right.
With a stick in, the page says **Remove the stick, then type the
passphrases**: pull it first.

*Picture to come: `vaults-and-passwords-create.png`, Create a vault with Size closed and Name and passphrases open.*

**Create a vault** is a column of cards. **Size** is closed on **PCs
and a Raspberry Pi · 64 MiB · 3 passes**, with **Change**:

| Size | Unlock memory | Opens on |
|---|---|---|
| **PCs and a Raspberry Pi** | 64 MiB, 3 passes | any PC and a Raspberry Pi |
| **PCs only** | 512 MiB, 3 passes | PCs only |

Each row gives its unlock time on this computer. **Customise** opens
three cards in place of Size:

| Card | Choices |
|---|---|
| **Where will you open it?** | **This computer**, ticked, and each other kind of machine; the cost suggested next fits the one with the least memory |
| **Unlock cost** | **Light**, **Standard**, **Medium**, **Strong**, **Maximum** and **Custom**, each with its memory, passes and the bits it adds; one reads **Suggested**, the others **Fits** or "Too large for" the weakest machine |
| **Space per passphrase** | the room each passphrase's contents get, and the file's size |

The cost and the space are fixed for the life of the vault. Leave Size
as it is.

Under **Name and passphrases**:

1. **Name**: type `Passwords`. The line under it reads **File
   Passwords.ofv**. The name is what anyone who sees the stick reads;
   leave it empty for `vault.ofv`.
2. **Passphrase 1**: type it in the first field, then again in the
   second, until the line under them reads **Matches**. The eye at the
   end of a field shows what is typed. Or press **Generate with dice**
   (step 2).
3. **Add another passphrase** adds **Passphrase 2**, up to four, each
   with **Remove**. Each passphrase opens contents of its own, and the
   file does not say how many there are. For this walkthrough, keep
   one.

The panel on the right, **Your choices**, repeats each card's choice;
under **Will create** it lists the **File**, its **Size**, the **Unlock
memory**, the time to unlock here (**Unlock here**), what the cost adds
(**Unlock cost adds**), the passphrase's **Strength**, and where it goes
(**Goes to**: **For the stick**). Press **Create vault**.

"Creating" and the time it takes show while it works. A sealed
copy of the vault goes into Files, **For the stick**.

## 2. A passphrase from dice

**Generate with dice** on a passphrase's card opens the dice:

| On the card | What it is |
|---|---|
| **Which list?** | **Long list** (7776 words · five dice), **Short list** (1296 words · four dice), **Short list 2** (1296 words · 3 edits apart) |
| "Roll 5 dice for each word and type the numbers" | the rolls, typed as they land; each word appears as its dice are in |
| the strength line | the words so far, their bits, and the bits with the unlock cost |
| **Use these words** | puts the words in both fields |
| **Cancel** | leaves the field as it was |

*Picture to come: `vaults-and-passwords-dice.png`, the dice card with three words of the long list rolled.*

Six words of the long list are 77.5 bits. Once the words fill the
field, the strength line reads the dice's bits plus the cost's. A typed
passphrase's own bits are not measured, and the line says so.

In Practice ([Faraday Demo](starter-tour.md)), **Practice puts faraday
in place of these words**: the vault's passphrase is `faraday`, the
public one every Practice vault uses.

## 3. Unlock it

Once made, the vault waits on **Unlock**, picked, with **Type the
passphrase once more to open it**. Type the passphrase under
**Passphrase** and press **Unlock**. Under the button, "This
computer:" and how much memory it has free.

*Picture to come: `vaults-and-passwords-unlock.png`, Unlock with Passwords.ofv picked and the passphrase typed.*

"Unlocking" and its time show while it works. A wrong passphrase
leaves the page as it is, with the reason under the field; there is no
limit on tries.

The vault opens on its contents: **Nothing in it yet**, and a button
per thing it can hold:

| Button | Adds |
|---|---|
| **Save a bitcoin key** | a key loaded now (step 4) |
| **Save a wallet** | a wallet loaded now |
| **Add an entry** | a login: title, username, password, URL, notes, TOTP secret (step 5) |
| **Add a note** | text (step 6) |
| **Make a GPG key** | [A GPG key](gpg.md) |
| **Make Secure Boot keys** | [Your own Secure Boot keys](secure-boot-keys.md) |
| **Write it to a stick** | writes the vault as it is now (step 9) |

## 4. The contents, and a key saved

Press **Save a bitcoin key**. **Save a key** lists each key loaded and
not in the vault: `3dca6805` and its name, with **Save**. A key loaded
with a BIP-39 passphrase also has **Save with its passphrase**. Press
**Save**. The line **Every key in this session is in the vault** shows
once all are in.

*Picture to come: `vaults-and-passwords-contents.png`, Vault contents with Bitcoin keys chosen and key 3dca6805 shown.*

The page is now the vault's contents, in three columns:

| Part | What it shows |
|---|---|
| the title | the vault's name, then **Open**, or "Open ·" and the unsaved changes (**Open · 1 unsaved change**); **Rename** and **Lock** on the right; under it the file's name and its cost |
| the first column | each kind the vault holds, with its count: **Bitcoin keys**, **Wallets**, **Entries**, **Notes**, **GPG keys**, **Secure Boot**; then a row per kind to add, each with **+**: **Bitcoin key**, **Wallet**, **Entry**, **Note**, **GPG key**, **Secure Boot** |
| the second column | the items of the kind chosen, with its add button (**Save a key**, **Save a wallet**, **Add an entry**, **Add a note**) |
| the third column | the item chosen |

Key `3dca6805` shows its **Fingerprint**, its **Words** and its SeedQR
behind their eyes, **BIP-39 passphrase** (**Stored** or **Not
stored**), **Load at unlock** (**Yes** or **No**) and **In this
session** (**Loaded** or **Not loaded**). Under them:

| Button | Does |
|---|---|
| **Load into session** | loads the key, when it is not loaded |
| **Load with a passphrase** | loads it with a BIP-39 passphrase typed now |
| **Don't load at unlock**, **Load at unlock** | whether the unlock's load list ticks it (step 8) |
| **Save to a stick** | the key's words or SeedQR as a file for the stick, outside the vault: **Anyone who finds this stick has this seed** |

A key in a wallet of the same vault reads **Load at unlock** "Loads
with" and the wallet's name. A wallet's item shows its backup's chart,
its **Descriptor**, **Open in Wallets**, **Show as QR** and **Save to a
stick**.

A value behind an eye shows after a press on it and hides at the next
press or when the page is left. **Rename** names the vault's contents;
the file keeps its name.

## 5. A login with its TOTP code

Press **Entry**, the row with **+** under the kinds (or **Add an
entry** once **Entries** is listed). Fill in, as an example:

| Field | Example |
|---|---|
| **Title** | `Mail` |
| **Username** | `me@example.com` |
| **Password** | the site's password |
| **URL** | the site's address |
| **Notes** | anything else |
| **TOTP secret** | the site's setup key, or the `otpauth://` link its QR code holds |

Press **Save**, or **Cancel**. An entry needs a title. A TOTP secret
that does not read is refused with **Not a TOTP key**, and the form
keeps what is typed.

*Picture to come: `vaults-and-passwords-entry.png`, the entry Mail with its Code and TOTP secret.*

Under **Entries** two more buttons fill the list:

| Button | Does |
|---|---|
| **Scan a code** | opens the camera; a site's TOTP setup QR code fills a new entry's form with its title, account and secret, to save |
| "Import from Files" and a count | adds every entry of the entries files in Files |

Entries that do not fit the vault's space are not added, and a line
says how many bytes they take and how many are left. With entries in
the list, **Search entries** over it keeps those whose title holds what
is typed; usernames, URLs and notes are not searched.

The entry shows **Username**, **Password**, **URL**, **Code**, **TOTP
secret** and **Notes**, the secrets behind their eyes, then **Edit**,
**Export for KeePass** (step 7) and **Hold to delete from vault**.
**Code** is the code now, with the seconds left; it reads **Set the
time** while the clock is not checked (step 6).

## 6. The codes, and the time

Press **TOTP codes** under **Entries**, or **Tools → TOTP codes**
(**Codes from the entries in open vaults**). The first time after
power-on, **Set the time** opens first:

| On Set the time | What it is |
|---|---|
| **Now** | the clock as Faraday reads it, or **Not known** |
| **Time zone** | the zone, with **-** and **+** |
| **Date**, **Time** | today's date and the next minute, typed |
| **Set** | sets the clock to that minute at its first second |
| **Keep this time** | keeps the clock as it is, where it is right |
| **Back** | goes back without checking the clock |

*Picture to come: `vaults-and-passwords-set-time.png`, Set the time with the date and time typed.*

Type the next minute, and press **Set** when the phone's minute changes
to it. The codes open:

| Row | What it shows |
|---|---|
| "Now · UTC" and the zone | the local time with seconds, and **Set the time** |
| an entry | its title and account, its code, and a bar for the seconds left |
| an entry whose secret is a counter's (HOTP) | a badge, no code |
| an entry whose secret does not read | **Unreadable** |

*Picture to come: `vaults-and-passwords-codes.png`, TOTP codes with the Mail entry's code and its bar.*

A row opens its entry. **Search codes** finds an entry by its title.
Under the list, **Scan a setup code** and **Add an entry** add to the
vault open; with no entry, **No entries with a TOTP secret**. The codes
come from every vault open.

## 7. Export a login for KeePass

On the entry `Mail`, press **Export for KeePass**. Type a passphrase of
8 characters or more under **Passphrase** (as an example,
`correct horse battery staple`), then **Seal and export**; **Cancel**
leaves it.

**Sealing** shows while it works. The file `Mail.kdbx` goes into Files,
**For the stick**, and a line says it waits for the stick, sealed. It
holds that one entry and opens in a KeePass app with that passphrase.
**KeePass export** under **Tools** says the same: an entry leaves this
way, one at a time.

## 8. Lock

Press **Lock** in the vault's title (or on its row in **Vaults**, or
at the foot of the sidebar). With changes in the vault, the **Lock**
sheet lists what it keeps and wipes ([Lock](../screens/lock-and-power.md#lock)):

| Section | Lists |
|---|---|
| **Kept** | the vault, and the files for the stick |
| **Wiped from memory** | key `3dca6805`, "in" the vault; a key or wallet in no vault reads "nowhere else", with **Save** beside it to put it in the vault open |
| **Then** | **Faraday restarts on Unlock** |

Press **Lock**, or **Back to the vault**. The lock seals the vault with
its changes for the stick, wipes every key, and starts Faraday again on
**Unlock**.

## 9. Write it to a stick

Plug the stick in. With nothing loaded, **Stick visit** opens: under
**In Faraday**, `Passwords.ofv` and `Mail.kdbx` are ticked. Press
**Write 2 files**. Each file is written, read back and compared; the
visit's line reads "Wrote Passwords.ofv, read back and matched". Then
pull the stick. [Files, sticks and QR codes](files-and-sticks.md#3-write-files-to-a-stick)
goes through the visit.

*Picture to come: `vaults-and-passwords-written.png`, Vaults with Passwords locked and on its stick, current.*

**Vaults** lists each vault Faraday holds, with where it stands first:

| Line | Meaning |
|---|---|
| **Never written** | sealed for the stick, on no stick yet |
| "On" a stick's name "· current" | the stick holds it as it is now |
| **Changed since written** | changed since the stick was written |
| **Unchanged** | copied in from a stick Faraday no longer knows, and not changed |

A changed vault makes Home lead with "Write Passwords to a stick". A vault on the stick already is written over its own copy;
another file of its name is kept, and this one is written beside it.

## 10. Unlock it again, and load from it

After a power-off the vault is only on its stick. Plug the stick in
with no key loaded: on **Stick visit**, `Passwords.ofv` is ticked under
**On the stick**. Press **Import 1 file**, then pull the stick. A
vault on the Faraday stick itself comes in at start instead, on **Read
from …** ([Read from the boot stick](../screens/prompts.md#read-from-the-boot-stick)).

Then press **Unlock** on the vault's row in **Files** or in
**Vaults**, type the passphrase and press **Unlock**. Files opens, with
**Load from Passwords.ofv** over it:

| On the sheet | What it is |
|---|---|
| a card per wallet | the wallet and its seeds this vault holds, ticked as its **Load at unlock** says |
| **Keys** | keys in no wallet: `3dca6805`, ticked |
| **Loaded** | what is loaded already |
| **Select all**, **Unselect all** | ticks or unticks every card |
| **Load ticked** | loads what is ticked; Wallets opens on a wallet loaded, otherwise Files |
| **Not now** | loads nothing; the vault stays open |

*Picture to come: `vaults-and-passwords-load.png`, Load from Passwords.ofv with the Keys card ticked.*

Press **Load ticked**. A flow that asks for a vault (Spend, Restore,
a backup) unlocks it from its own step and loads what it needs, with
no sheet.

## 11. A new passphrase

Passphrases, the cost and the space are fixed when a vault is made. To
change one, create a new vault, unlock both, and save into the new one
what the old one holds: load the keys and wallets and **Save** them,
and add the entries and notes again. Write the new vault, then delete
the old file from its stick with zeros
([Files, sticks and QR codes](files-and-sticks.md#5-delete-with-zeros)):
until it is deleted, the old file still opens with the old passphrase.

> **With your own key**
>
> - Roll the passphrase with real dice: six words of the long list.
>   Never use a passphrase printed anywhere, or one used for anything
>   else.
> - Write the passphrase down and keep it apart from the stick: a
>   lost passphrase loses what it opens.
> - Name the vault for whoever may see the stick, or leave the name
>   empty.
> - Before you rely on the vault, unlock it from its stick on a fresh
>   start: [Before you rely on a key](before-you-rely.md).
> - A KeePass export is as safe as its passphrase: give it a strong one
>   of its own.
>
> Why: [Vaults](../../learn/faraday/vaults.md),
> [Passphrases](../../learn/05-passphrases.md),
> [Time and TOTP codes](../../learn/faraday/time.md),
> [Where keys are kept](../../learn/faraday/keys-kept.md).

Next: [Files, sticks and QR codes](files-and-sticks.md).
