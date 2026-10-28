# Vaults and Unlock

**Kind:** Spec
**Status:** Written from the app's code and strings (`faraday/faraday-core/src/vault/screen.rs`, `faraday/faraday-core/src/vault/panes_screen.rs`, `faraday/faraday-core/src/vault/load_screen.rs`, `faraday/faraday-core/src/vault/mod.rs`); not yet read beside the running app
**Open:** the pictures, drawn once after the theming has landed
**Screen:** `Vaults`, `Unlock`, `UnlockLoad`, `PassphraseSeeds`
**Group:** Vaults

The list of the vaults in Files, a vault's Unlock, and the two sheets
that load from a vault once it is open. A vault is one encrypted file
(`.ofv`); Files holds it while Faraday runs, and a stick visit carries
it in and out.

## Vaults

**Opened from:** the sidebar's **Vaults**, and Home's **Vaults** tile
once Files holds a vault or one is open. A flow that needs a vault (a
backup, a secret on its way out) opens the list with the way back to
the flow above it.

*Picture to come: `vaults-list.png`, Vaults with one vault locked and its card beside the list.*

At 1280 × 800 the page is a list and the vault chosen beside it: the
list on the left, the first vault's card on the right.

| Control | What it does |
|---|---|
| a vault's row | its name and its state; a press opens a locked vault's [Unlock](#unlock) beside the list, or an open vault's [contents](vault.md) |
| **Create a vault** | under the list: opens [Create a vault](create-vault.md). Not shown while the list was opened from a flow |
| the card | the vault chosen: its name, its file, its line, its state, and its buttons |
| **Unlock** | on a locked vault's card: opens its Unlock |
| **View contents** | on an open vault's card: opens its [contents](vault.md) |
| **Lock** | on an open vault's card: locks, sealing every open vault; [Lock](lock-and-power.md#lock) asks first when the lock would end something |
| "A vault holds keys, wallets, entries and notes under one to four passphrases. …" | under the card, in Guided mode |

### A vault's name and state

| Name | State | Shown while |
|---|---|---|
| the name it was given | **Open** | open, no changes |
| the name it was given | "Open · 2 unsaved changes", in the warning tone | open with changes; the next lock seals them |
| **New vault** | **Locked · not written yet** | made this session and in Files, not yet on a stick |
| **Locked vault**, or the name seen when it was last open | **Locked** | locked |
| the same | "Remove the stick to unlock", in the warning tone | a stick is attached |

The card's line says first where the file stands, then what it holds:

| Line | Means |
|---|---|
| **Never written** | made here, not yet written to a stick |
| "On Boot stick · current" | the copy on that stick is this one |
| **Changed since written** | it differs from the copy last written |
| **Unchanged** | read from a stick and not changed since |
| "Savings 2 of 3 · key 9a6a2580 · 12 entries · seen 14:02" | what it held when last open this power-on: its wallets, its keys by fingerprint, its entries, and when |
| **Unlock to see what it holds** | not open this power-on |

An open vault's line gives where it is, its size and its unlock cost.

### States

| State | What Vaults shows |
|---|---|
| no vault in Files | **No vault in Files** in place of the card |
| a vault open | its card with **View contents** and **Lock** |

## Unlock

**Opened from:** a locked vault's **Unlock** on Vaults or on Files,
Home's **Unlock …** lead, and the boot stick's import sheet. Its way
back at the top goes to **Vaults**, to **Import**, or to the flow that
asked for the vault.

*Picture to come: `vaults-unlock.png`, Unlock with the passphrase field and the vault's cost.*

| Control | What it does |
|---|---|
| "Choose the vault and type a passphrase. …" | in Guided mode |
| **Type the passphrase once more to open it** | shown over the vault just made by Create a vault |
| the vault | the vault chosen in the list beside it. On a narrower window, each locked vault is a row: its file, where it is and its size, its unlock cost, and "about 2.4 s here", the time it takes on this computer; a press chooses it |
| **Passphrase** | the field; its eye shows or hides what is typed |
| "Remove the stick, then type the passphrase" | in place of the field while a stick is attached |
| **Unlock** | tries the passphrase. Grey while a stick is attached |
| "This computer: 3.1 GiB free" | the memory free here, or **This computer: memory not reported** |

| State | What Unlock shows |
|---|---|
| unlocking | "Unlocking · about 2.4 s" in place of **Unlock** |
| refused | why, under the button: "That passphrase opens nothing in this vault", "Needs 2048 MiB, which this computer cannot spare", **Type the passphrase** |
| no locked vault | **No locked vault in Files**, and **Create a vault** |

A passphrase opens the contents it was made for; the vault does not say
how many it holds. Once open:

- from Vaults, Files shows with [Load from](#load-from) over it, when
  the vault holds a key or wallet not loaded yet;
- from the boot stick's import, its sheet comes back with the vault's
  wallets and keys in it ([Read from the boot stick](prompts.md#read-from-the-boot-stick));
- from a flow, the flow goes on;
- a vault just made opens on its [contents](vault.md), empty.

## Load from

**Opened from:** an unlock from Vaults or from the boot stick's import,
when the vault holds a key or wallet the session does not hold. Its
title is **Load from …** with the vault's file ("Load from vault.ofv").

*Picture to come: `vaults-load-from.png`, Load from vault.ofv with one wallet ticked.*

| Control | What it does |
|---|---|
| the count | what the vault holds: "1 wallet · 2 keys" |
| **Select all**, **Unselect all** | ticks or unticks every card |
| a wallet's card | its name, its kind, each of its seeds the vault holds by fingerprint, and **Watch-only here** or "1 of 2 seeds kept elsewhere"; a press ticks or unticks the wallet with its seeds. A seed in another wallet too says "also in …" |
| **Keys** | the card of keys in no wallet of the vault, each ticked on its own |
| **Loaded** | at a card's or key's end: the session holds it already; it has no tick |
| **Not now** | closes the sheet and loads nothing. A press beside the sheet does the same |
| **Load ticked** | loads what is ticked. Grey while nothing is, or a stick is attached |

What is ticked at first is what the vault saves with **Load at unlock**
on ([Vault contents](vault.md#a-key)). After the sheet, a wallet loaded
opens Wallets on it, with "… loaded from vault.ofv"; with none, Files
stays.

## Seeds with no passphrase stored

**Opened from:** **Load with a passphrase** on a wallet short of a key
(its card, the Spend tab, a spend), while an open vault keeps a seed
with no passphrase. With one such seed, the sheet opens at its field.

*Picture to come: `vaults-passphrase-seeds.png`, Seeds with no passphrase stored with one seed chosen.*

| Control | What it does |
|---|---|
| a seed's row | the vault it is in, and the seed; a press chooses it |
| **BIP-39 passphrase** | the field for the seed chosen |
| **Another seed** | back to the list, while there is more than one |
| **Load** | loads the seed with the passphrase typed. A passphrase that gives a key the wallet does not hold is refused under the field ("This passphrase gives 5d388376, not a key of New wallet 1"). Grey while the field is empty or a stick is attached |
| **Cancel** | closes the sheet |

## On a narrow window

Vaults is a list of cards, **Create a vault** first, each card with its
buttons under its line. Unlock is its own page, the vaults as rows.

## Why

- [Vaults](../../learn/faraday/vaults.md)
- [Passphrases](../../learn/05-passphrases.md)
