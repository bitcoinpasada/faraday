# Wallets

**Kind:** Spec
**Status:** Written from the app's code and strings (`faraday/faraday-core/src/screens/wallets_start.rs`, `faraday/faraday-core/src/screens/wallets.rs`, `faraday/faraday-core/src/register.rs`, `faraday/faraday-core/src/screens/sheets.rs`, `faraday/faraday-core/src/glance/mod.rs`); not yet read beside the running app
**Open:** the pictures, drawn once after the theming has landed
**Screen:** `WalletsStart`, `Wallets`, `KeyPath`, `Register`
**Group:** Wallets

The Wallets tab's first page, the wallets loaded with the chosen
wallet's card beside them, and the two sheets that add a wallet only
once it is held for: one whose key path another key spends, and one a
spend registers.

## Wallets: the first page

**Opened from:** the sidebar's **Wallets**. The way back at the top of
Make a new wallet, Restore an existing wallet, Spend and Check a signed
message comes here too.

*Picture to come: `wallets-start-empty.png`, Wallets with nothing loaded: the four ways in.*

| Control | What it does |
|---|---|
| "No wallet is loaded. Create one with new keys or keys you load here, or restore one from its backups: …" | in Guided mode, with nothing loaded; "Start from what you have in front of you. …" once something is. Each with a link to Learn's **Start here** |
| **Your wallets** | the wallets loaded, two to a row, at most two rows; **None loaded** while none is. A row's press opens its card on [Wallets](#wallets-the-list-and-the-card) |
| a wallet's row | its name, and how it stands: "Can spend · 2 of 2", "1 of 2 here", **Watch only**, or "1 of 2 here · another key can spend" and **Does not match its key** in the error tone. "+ 2 more" when more are loaded than the rows show |

### With nothing loaded

Four cards, each a press:

| Card | What it opens |
|---|---|
| **Make a new wallet** (**Single key or multisig**) | [Make a new wallet](make-a-wallet.md) |
| **Restore an existing wallet** (**From its backups**) | [Restore an existing wallet](restore.md) |
| **Spend from a backup, step by step** (**A stick, words, or paper**) | [Spend](spend-tab.md) |
| **Add a key** (**Keys first, wallets later**) | [Add a key](new-key.md), on its Start card |

Under them, a vault locked in Files: "vault.ofv is in Files, locked",
and **Unlock**, which opens its [Unlock](vaults.md#unlock).

### With something loaded

Under **Your wallets**: **Spend from a backup, step by step** and **Add
a key**. Making and restoring a wallet stay on Home and Tools; the
wallet card has the rest.

## Wallets: the list and the card

**Opened from:** a wallet's row on the first page, a wallet loaded from
Files or a vault, and the end of Add a key, Make a new wallet and
Restore an existing wallet.

*Picture to come: `wallets-card.png`, Wallets with New wallet 1 chosen and its card beside the list.*

At 1280 × 800 the page is the list on the left and the chosen wallet's
card on the right.

| Control | What it does |
|---|---|
| the lead bar | over the list, while a PSBT waits: its file's name, then **Signing** and **Continue** for the spend under way, or "in Files" and **Review** for a PSBT in Files. A press opens the spend. Shown while a wallet is loaded |
| **Wallets** | the list's heading; each wallet's row has its name and kind, "· watch only" with no key here or "· 1 of 2 here" short of keys, and at its end **Can spend**, or **Another key can spend** or **Does not match its key** in the error tone. A press shows its card |
| **Keys without a wallet** | each key loaded that no wallet uses, by fingerprint and name, and **Make a wallet from this key** under it |
| **Restore an existing wallet** | under the list, while nothing is loaded |
| **Add a key** | the list's last button: opens [Add a key](new-key.md). It reads "Scan a key for New wallet 1" while the wallet shown is short of keys, and opens the camera for that wallet's key |

### The card

| Control | What it does |
|---|---|
| the name | the wallet's name, and **Rename** beside it: the name becomes a field, "Enter keeps it · Esc leaves it" |
| **New wallet from this one** | beside **Rename** where it fits, else with the actions: opens [Make a new wallet](make-a-wallet.md) filled from this wallet: its kind, quorum and keys, at the lowest account they do not use yet. Only for a kind Make a new wallet makes |
| the source | at the top right: where the wallet came from, "From Vault 1", or a statement such as "Made here" |
| the kind | "2 of 3 · native SegWit multisig" |
| the key path | a Taproot wallet's: **Key path: none (unspendable)**, "Key path: 3dca6805 alone", or, in the error tone, "A key that is not one of this wallet's keys can spend everything alone, without the other signatures: …" |
| the backup line | **Not backed up**, or where its backup is ("Backup: paper ×2 checked · vault.ofv"); a press opens [Backups](backups.md) on the wallet |
| the signatures | "1 of 2 signatures possible here", a dot per signature; then **Can spend** (**Another key can spend** in the error tone), or **Scan a key**, which opens the camera for a missing key |
| **At a glance** | the wallet, each key and the backup: where each is, **Can sign here**, **Cosigner · xpub only**, **Not here**, **In this vault**, with **Load from vault** or **Load with a passphrase** where an open vault holds the key |
| the codex32 mark | under a key loaded from codex32 strings that went in by a correction: "fab6868a · Share entry corrected", or "… · Share repairs unchecked" |
| where the missing keys may be | short of keys: "vault.ofv is locked · it may hold them" with **Unlock**; "Not in vault.ofv · perhaps another vault, a paper backup or a SeedQR"; or "A vault on a stick, a paper backup or a SeedQR may hold them" |
| **First receive address · 0/0** | the wallet's first address; **Back up before you receive**, in the warning tone, in its place while a key made here waits for its backup |

The actions, along the card's foot:

| Control | What it does |
|---|---|
| **Spend from this wallet** | opens [Spend from New wallet 1](spend-tab.md#spend-from-a-wallet). It reads "Sign the PSBT from the stick" while a PSBT for the wallet is in Files, "Carry on the spend · Signatures 1 of 2" while its spend is under way, and **Finish the backup first** while a key made here waits for its backup, which opens the backup |
| **Show wallet QR** | the wallet's description as a [QR code](qr-and-scan.md#a-qr-code) |
| **Back up** | opens the wallet's [backup](back-up-wallet.md); **Continue the backup** while one is left part way |
| **Sign a message** | for a wallet whose key signs messages: opens Sign a message |
| **Remove from session** | at the right: takes the wallet out of memory |

### No wallet loaded

| What is loaded | What the card shows |
|---|---|
| nothing | **No wallet loaded** |
| one key | "Key 3dca6805 is loaded. No wallet uses it." |
| more keys | "2 keys are loaded. No wallet uses them.", each key a row to tick |
| then | **Make a wallet from this key** (Restore, on its Keys card, as a single-key wallet with the key ticked), **Add another key** (Restore, as a 2-key multisig with it), **Restore an existing wallet** |

### A silent payments wallet

Its card has no signatures and no spend.

| Control | What it does |
|---|---|
| **Key** | the key's fingerprint, and **Here**, **Not here** or **Does not match** |
| **Load from vault**, **Add its key** | while the key is not here: loads it from an open vault, or opens Add a key for it |
| **Address** | the address, checked against the key when it is loaded; "Not checked: load key 3dca6805 to check this address" while it is not. A record whose keys are not the key's shows no address, and "This wallet's record does not match key …" in the error tone |
| **Labels handed out** | how many |
| **Address, labels and payments** | opens the wallet's silent payments page |
| **Show wallet QR**, **Remove from session** | as on any card |

## Another key spends this wallet alone

**Opened from:** a Taproot wallet coming in, by any way (Files, a scan,
a vault, Restore), whose key path is a key that is not one of the
wallet's own. The wallet waits, and the sheet opens over what is on
screen.

*Picture to come: `wallets-key-path.png`, Another key spends this wallet alone over Files.*

| Control | What it does |
|---|---|
| "New wallet 1: this key is not one of its keys and can spend everything alone, without the other signatures" | in the error tone, then the key in groups of four |
| **Hold to add this wallet** | held until it fills: adds the wallet. The press that brought it in goes on as it would have; the same wallet is not asked about again this session |
| **Cancel** | the wallet is not added |

Once added, the card's key path line names the key, and **Can spend**
reads **Another key can spend**.

## Register this wallet

**Opened from:** **Register this wallet** on a spend's review, when an
input spends from a wallet of several keys that is not loaded and the
PSBT carries every cosigner's xpub.

*Picture to come: `wallets-register.png`, Register this wallet with its shape, three cosigners and Yours.*

| Control | What it does |
|---|---|
| the shape | "2 of 3, P2WSH, sorted": the quorum, the script and the key order ("keys in this order") |
| **Most coordinators sort the keys** | where both orders fit the transaction: **Sorted** (chosen at first) and **In this order**, one to choose |
| **Name** | the field: the wallet's name, the PSBT's file name at first ("savings-unsigned.psbt" gives "Savings Unsigned") |
| the cosigners | a line each: fingerprint and path; "· Yours" in the primary tone where a loaded key is that cosigner |
| **Hold to register** | held until it fills: checks the wallet against the spend, adds it as a wallet file is added, and reviews the spend again |
| **Cancel** | adds nothing |

The hold adds nothing when the wallet does not fit; the sheet stays,
with why under the name:

| Line | Means |
|---|---|
| **These keys do not make a wallet** | the keys and the order make no wallet |
| "These keys do not give input 0's address" | the wallet is not the one the input spends from |
| "Output 1 names this wallet but is not its address" | an output claims to be the wallet's and is not |
| **None of these keys is yours** | no key loaded is one of them |
| "Key 3dca6805 at 48h/0h/0h/2h has your fingerprint but is not your key" | a key under your fingerprint that your key does not give |

## On a narrow window

The first page is a list of rows, one per way in. Wallets shows the
list, and a wallet's card as its own page, its actions under it.

## Why

- [Kinds of wallets](../../learn/17-wallet-kinds.md)
- [Start here](../../learn/01-start-here.md)
- [Multisig](../../learn/09-multisig.md)
- [Spend paths and timelocks](../../learn/10-spend-paths.md)
- [Coordinator files](../../learn/19-coordinators.md)
