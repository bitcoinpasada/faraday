# Back up

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Backup`
**Key:** backup

A backup starts from a plan of where the wallet's keys and files are kept, and the plan becomes a checklist with one item per thing to make. Each item opens once the one before it is done, and the last item writes the sticks.

## Places

**Key:** backup.board

The board lists the places that will keep the backup. Each place that keeps a seed says how close it brings someone to spending: **Can spend alone**, or **Spends with 1 more place** when one other place adds enough seeds. A change that brings a place closer to spending, or lets it see the balance, asks first: **Keep it** keeps the change and **Take it off** undoes it.

The board offers ready plans: **Paper only**, **Paper and a vault** and, for a single key, **Two vaults**. A vault in one is new, made in the checklist.

**Keeps a copy of the plan** leaves a vault out.

**Add partial descriptors to this backup** gives every place a share of the descriptor, which is a multisig descriptor with some keys left off; **Remove partial descriptors** takes the shares off again. A place with only one share cannot work out an address.

## The plan

**Key:** backup.plan

The plan is checked three ways, by trying each place lost or found in turn. A place is a paper place, a stick of files, or a vault's stick that no paper place keeps.

**Any one place lost: the rest rebuild the wallet** is **Yes** when the other places always hold a quorum of seeds with their passphrases and, for several keys, the whole descriptor or shares that hold every key. **Yes, with the vault's passphrase** means a loss is covered only by a vault. **No** names a single point of failure.

**One place found: can spend** is **Yes** when one place holds a quorum of seeds; **Only with the vault's passphrase** means the vault holds what is missing. **One place found: sees the balance** is **Yes** when one place holds the whole descriptor, or the seed of a single key. **Make the checklist** approves the plan.

## Print the blank templates

**Key:** backup.blank

Print the blank templates from Faraday Starter on your online computer. They hold no secret; Faraday itself prints nothing, and a seed never goes to a printer. In the next item you copy each seed onto one by hand.

**Words on the seed** is **12** or **24**. **Done** marks the templates as printed.

## Copy by hand

**Key:** backup.copy

One item per seed. Copy the words onto a template in order, then type them back; the first word that differs is named.

**Show the seed** shows the words, each with its number in the word list, and the SeedQR grid, **Standard SeedQR** or **Compact SeedQR**. **Check my copy** compares the words you type with the seed. **Scan your copy** reads a SeedQR copy. **Keep the words only** drops a SeedQR copy and checks the words instead.

**Seed XOR, 2 parts** and **Seed XOR, 3 parts** are other paper forms. Each part on its own is valid words for a different, empty wallet. The section lists the places the parts go to, and the checklist has an item for them. **Check my copy of part 1** compares the words you type with that part. **Put them away** needs every part checked, or a second press.

## Codex32 strings

**Key:** backup.codex32

For a key made as codex32 shares or loaded from strings. Copy each string by hand, with its place, and check it.

**Check my copy** compares your copy with the string. A string made here reads **Checked** once it matches. A string of a set that already exists reads **Checked** once you have typed as many strings as the set needs and together they make the key; until then it reads **Not checked**. A string you skip reads **Skipped**. If fewer strings than that are on paper, they can only be skipped.

## Passphrase on paper

**Key:** backup.passphrase

Write the seed's passphrase by hand exactly as it is, capitals and spaces included, then type it back; the whole of it is compared.

**Show the passphrase** shows it and **Hide the passphrase** hides it. **Check my copy** opens the field **Your copy's passphrase**, and **Compare** checks it.

## Krux code

**Key:** backup.kef

A seed's Krux code is shown for you to copy by hand onto a Krux template, and then checked.

A code of 37 × 37 squares or less shows **Size**, **By hand** with its sheet in Faraday Starter, Blank templates, Krux code, and a grid in place of the picture, to copy block by block. A larger code cannot be copied by hand.

**Show the code** shows the code. **Scan my copy** reads your copy with the camera. **Check my code** takes the code's passphrase and **Compare** checks it. **Make another** makes a further code. **Not on Paper** takes the code off every place.

## Vault

**Key:** backup.vault

One item per vault in the plan. The vault keeps a further copy of the seed, sealed under its passphrase.

**Into the vault** saves the seed and the descriptor into the open vault. **Load at unlock** has the seed loaded whenever the vault is unlocked. **Make a new vault** makes one where none is open. The vault is sealed and written to its stick at **Lock and write**, the last item.

## The seeds as files

**Key:** backup.files

Each seed is written as a file for the stick. The file is unprotected: anyone who copies the stick can spend with it. It never holds the passphrase. **Make it** makes the file for a seed.

## The wallet sheet

**Key:** backup.sheets

One sheet with the wallet's name, every xpub and the checksum. It cannot spend; it is what rebuilds the wallet. **Make it** makes a PDF to print.

## The shares

**Key:** backup.sheets-shares

For a multisig whose description is split, the item is **The shares**. Each share holds the description with some keys left off: one alone cannot watch the wallet, and enough together rebuild it. **Make them** makes the sheets, text files and pictures.

**Keys left off each share** is a slider from 0 to the most the quorum allows: one fewer than the signatures needed, and at most one fewer than the keys. At 0 every share is the whole descriptor. The table lists each share with its keys' fingerprints. The lines under it say whether any quorum's worth of shares rebuilds the wallet or only some, the fewest shares that rebuild it, how many shares each key is on, and **No single share holds every key** or **Each share holds every key**. A share holds account keys and cannot sign.

## The public files

**Key:** backup.public

One file for each kind of software you chose: **Descriptor** for any wallet software, **Wallet file** for Sparrow and Specter, **Multisig config** for Coldcard, Keystone and Passport. They hold xpubs only: they cannot spend, but they put the keys back together into the wallet. **Make the file** makes one, and **Make them** makes them all.

## Show the descriptor to the software

**Key:** backup.show

**Show as QR** shows the descriptor as a QR code for your watch-only software to scan. It holds xpubs only, so it can watch the wallet but not spend.

When every item is done, **Backup done** shows the plan's checks and the wallet's chart. **Show wallet QR** is its main button; **Open the wallet**, **Lock**, **Power off** and **Change the plan** are beside it.

## One envelope per place

**Key:** backup.envelope

Lists what goes into each place's envelope. Store the envelopes apart. A copy in a vault is one more copy, not a replacement for paper. **Done** closes the item.

## Write to sticks

**Key:** backup.sticks

**Lock and write** locks the session, then writes each stick of the plan in turn, place by place, in the backup flow. **Kept** lists what the lock keeps and **Wiped from memory** what it wipes.

A copy made outside the plan is not tracked, and the plan's three checks do not count it. To copy a vault to another stick, use **Copy the vault to another stick** from the vault on the wallet's chart; the copy opens with the same passphrase, and needs copying again after a later change. Words are copied by hand onto a new sheet and checked word by word. Each copy is as sensitive as what it copies.
