# Back up

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Backup`
**Key:** backup

The plan of where a wallet's keys and files are kept becomes a checklist of one item per thing to make. An item opens once the one before it is done, and the last item writes the sticks.

## Places

**Key:** backup.board

The board lists the places that keep the backup. Each place that keeps a seed says how near it brings a spend: **Can spend alone**, or **Spends with 1 more place** when one other place adds enough seeds. A change that brings a place nearer a spend, or lets it see the balance, asks first: **Keep it** keeps the change and **Take it off** undoes it.

**Add partial descriptions to this backup** gives every place a share of the wallet description, a multisig description with some keys left off; **Remove partial descriptions** takes the shares off. A place with one share alone cannot work out an address.

## The plan

**Key:** backup.plan

The map of the places is checked three ways, by trying every place lost or found in turn. A place is a paper place, a stick of files, or a vault's stick that no paper place keeps.

**Any one place lost: the rest rebuild the wallet** is **Yes** when the other places always hold a quorum of seeds with their passphrases and, for several keys, the whole description or shares that hold every key. **Yes, with the vault's passphrase** means a loss is covered only by a vault. **No** names a single point of failure.

**One place found: can spend** is **Yes** when one place holds a quorum of seeds; **Only with the vault's passphrase** means the vault holds what is missing. **One place found: sees the balance** is **Yes** when one place holds the whole description, or the seed of a single key. **Make the checklist** approves the plan.

## Print the blank templates

**Key:** backup.blank

Blank templates are printed from Faraday Starter on the online computer; they hold no secret, and Faraday prints nothing. Each seed is copied onto one by hand in the next item.

**Words on the seed** is **12** or **24**. **Done** marks the templates printed.

## Copy by hand

**Key:** backup.copy

One item per seed. The words are copied onto a template in order, then typed back; the first word that differs is named.

**Show the seed** shows the words, each with its number in the word list, and the SeedQR grid, **Standard SeedQR** or **Compact SeedQR**. **Check my copy** compares the typed words with the seed. **Scan your copy** reads a SeedQR copy. **Keep the words only** drops a SeedQR copy and checks the words instead. **Seed XOR, 2 parts** and **Seed XOR, 3 parts** are other paper forms.

## Codex32 strings

**Key:** backup.codex32

For a key made as codex32 shares or loaded from strings. Each string is copied by hand, with its place, and checked.

**Check my copy** compares the copy with the string. The item reads **Checked** once it matches.

## Passphrase on paper

**Key:** backup.passphrase

A seed's passphrase is written by hand exactly as it is, capitals and spaces included, then typed back and compared whole.

**Show the passphrase** shows it; **Hide the passphrase** hides it. **Check my copy** opens the field **Your copy's passphrase**, and **Compare** checks it.

## Krux code

**Key:** backup.kef

A seed's Krux code is shown, copied by hand onto a Krux template and checked.

A code of 37 × 37 squares or less shows **Size**, **By hand** with its sheet in Faraday Starter, Blank templates, Krux code, and a grid in place of the picture, to copy block by block; a larger code cannot be copied by hand.

**Show the code** shows the code. **Scan my copy** reads the copy with the camera. **Check my code** takes the passphrase of the code and **Compare** checks it. **Make another** makes a further code. **Not on Paper** takes the code off every place.

## Vault

**Key:** backup.vault

One item per vault the plan has. The vault keeps a further copy of the seed, sealed under its passphrase.

**Into the vault** saves the seed and the wallet description into the open vault. **Load at unlock** has the seed loaded when the vault is unlocked. **Make a vault** makes one where none is open. The vault is sealed and written to its stick at **Lock and write**, the last item.

## The seeds as files

**Key:** backup.files

Each seed is written as a file for the stick. The file is unprotected: anyone who copies the stick can spend with it, and it never holds the passphrase. **Make it** makes the file for a seed.

## The wallet sheet

**Key:** backup.sheets

One sheet with the wallet's name, every xpub and the checksum. It cannot spend; it rebuilds the wallet. **Make it** makes a PDF to print.

For a multisig whose description is split, the item is **The shares**: each share holds the description with some keys left off, one alone cannot watch the wallet and enough together rebuild it. **Make them** makes the sheets, text files and pictures.

**Keys left off each share** is a slider from 0 to the most the quorum allows, one fewer than the signatures needed and at most one fewer than the keys; with 0 every share is the whole description. The table lists each share with its keys' fingerprints. Under it, lines say whether any quorum's worth of shares rebuild the wallet or only some, the fewest shares that rebuild it, how many shares each key is on, and **No single share holds every key** or **Each share holds every key**. A share holds account keys and cannot sign.

## The public files

**Key:** backup.public

One file per kind of software chosen: **Descriptor** for any wallet software, **Wallet file** for Sparrow and Specter, **Multisig config** for Coldcard, Keystone and Passport. They hold xpubs only; they cannot spend, but put the keys back together into the wallet. **Make the file** makes one and **Make them** all.

## Show the descriptor to the software

**Key:** backup.show

**Show as QR** shows a QR code of the descriptor for the watch-only software to scan. It holds xpubs only and can watch the wallet, not spend.

## One envelope per place

**Key:** backup.envelope

Lists what goes into the envelope of each place. The envelopes are stored apart. A copy in a vault is one more copy, not a replacement for paper. **Done** closes the item.

## Write to sticks

**Key:** backup.sticks

**Lock and write** locks the session, then writes each stick of the plan in turn, by its place, in the backup flow. **Kept** lists what the lock keeps and **Wiped from memory** what it wipes.

## Extra copies

**Key:** backup.extra-copies

The last card, in Guided only. A copy made outside the plan is not tracked, and the plan's three checks do not count it.

**Extra copies** opens the page on them. A vault is copied to another stick, from the vault on the wallet's chart with **Copy the vault to another stick**; the copy opens with the same passphrase and is copied again after a later change. Words are copied by hand onto a new sheet and checked word by word. Each copy is as sensitive as what it copies.
