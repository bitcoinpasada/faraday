# Home

**Kind:** Spec
**Status:** Written from the app's code and strings (`faraday/faraday-core/src/screens/home.rs`, `faraday/faraday-core/src/screens/sidebar.rs`, `faraday/faraday-core/src/nav/screen.rs`); not yet read beside the running app
**Open:** the pictures, drawn once after the theming has landed
**Screen:** `Home`, `NavMenu`
**Group:** Home

**Opened from:** the sidebar's **Home**; Faraday opens on it after the
self-test, and again after every lock.

*Picture to come: `home-new-stick.png`, Home on a new stick, nothing loaded, at 1280 × 800.*

## On this screen

| Control | What it does |
|---|---|
| **Start** | the heading over the lead and the tiles |
| the lead | the one thing under way or waiting, full width above the tiles; see [The lead](#the-lead). None when nothing is |
| **Make a new wallet** (**Single key or multisig**) | opens Make a new wallet |
| **Restore an existing wallet** (**From its backups**) | opens Restore a wallet |
| **Spend from a backup, step by step** (**A stick, words, or paper**) | opens the Spend tab |
| **Create a vault** (**None yet**) | opens Create a vault. Shown while Files holds no vault and none is open |
| **Vaults** ("2 locked", "1 open") | opens Vaults. Shown in place of **Create a vault** once Files holds a vault or one is open |
| **Stick visit** (**A stick attached**) | opens the stick's files. Shown while a stick is attached and no key is loaded |
| **Backups** ("3 with a plan", "1 not backed up") | opens [Backups](backups.md). Shown once a wallet is known |
| **Scan** | the button at the bottom right: opens the camera. With a stick attached, **Pull the stick to scan** opens first ([Pull the stick](prompts.md#pull-the-stick)) |
| **Practice · Testnet** | the mode pill at the top right, shown in every mode but Mainnet: opens [Mode](mode.md#mode) |
| **?** | at the top right: opens Learn's pages for Home |

The tiles sit in a grid, as many to a row as fit; at 1280 × 800, two
to a row.

### The lead

The first of these that applies is the lead. A lead whose flow already
has a row under its tab is skipped.

| Lead | Shown while | Press |
|---|---|---|
| **Carry on the spend** ("New wallet 1 · Signatures 1 of 2") | a spend of a loaded wallet is under way | goes back to the spend |
| **Continue signing …** ("1 of 2 signatures") | a spend under way of a PSBT whose wallet is not loaded | opens Sign |
| **Continue the backup of …** | a key is made and a place of its backup is not written | opens the backup where it was left |
| **Sign …** (**From Files**) | Files holds a PSBT | starts the spend of the PSBT with the fewest signatures |
| **Import from …** | the boot stick's files wait to be imported | opens [Read from](prompts.md#read-from-the-boot-stick) |
| **Unlock …** ("Read from Boot stick") | a vault the import brought is still locked | opens the import sheet at that vault |
| **Written to …** (the files' names) | the last stick visit of this power-on wrote files, and nothing is loaded | opens Files |
| **Unlock …** (what the vault held when last open, or "… · in Files") | Files holds a locked vault, and nothing is loaded | opens that vault's Unlock |
| **Unlock …** (**Pull the stick to unlock**) | the same, with a stick attached | nothing: the lead is greyed with a warning edge until the stick is pulled |
| **Write … to a stick** (**Changed since written**) | an open vault has changed since it was written | opens Files |

## States

| State | What Home shows |
|---|---|
| a new stick | **Start** with the four tiles, no lead |
| a key loaded | the same; the sidebar lists the key under **Keys** and shows **Lock** |
| a stick attached, no key loaded | the **Stick visit** tile, and **Stick visit** in the sidebar |
| no randomness from the system | the line "No randomness from the system. Faraday has to restart", in the error tone, under the title |

## The sidebar

The sidebar is at the left of every screen on a window this size.

| Control | What it does |
|---|---|
| **Home**, **Vaults**, **Files**, **Wallets**, **Tools** | open their page; the page on show, or the page it was opened from, is on a pill |
| **Vaults** ("1 open") | the count of vaults open, at the row's end |
| **Files** (a number) | the number of files in its Inbox |
| **Learn** | opens Learn's pages for the screen on show |
| **Stick visit** | shown while a stick is attached and no key is loaded: opens the stick's files. A dot: the stick attached has not been visited yet |
| **Transfer** | Faraday Demo only: opens Transfer |
| **Settings** | opens Settings |
| a flow's row ("Make a new wallet", its step at the right) | under the tab it belongs to, one per flow under way: goes back to the flow where it was left |
| **Bring in · Open · Work · Write out** | the stage of the session, the current one in the accent colour: opens Files |
| **Keys** (a count) | the keys loaded, each by its fingerprint and name, **passphrase** at the end of one loaded with a passphrase: a key opens Explore for it |
| **Wallets** (a count) | the wallets loaded, each with **single key**, its quorum ("2 of 3") or **another key**; a green dot where the keys loaded sign for it alone, a grey one where some of its keys are loaded, a red one for a wallet a key not its own can spend alone (**another key**) or a silent payments wallet that a loaded key with its fingerprint contradicts: a wallet opens its card |
| **+ … more** | where a list does not fit: opens the whole list |
| **Files** ("1 from the stick", "2 for the stick", **none**) | opens Files |
| **Sticks** (the stick's name, "2 attached", **none**, **import waiting**) | what is attached. **import waiting** opens the import sheet |
| **Lock** | shown while a key is loaded: locks; see [Lock](lock-and-power.md#lock) |
| the power button | opens [Power](lock-and-power.md#power) |
| **Auto-lock off until backup complete** | shown under **Lock** from Make a new wallet until its backup is done |

## The ⋮ menu

On a tall narrow window, Settings and Power off are in the **⋮** menu
at the top right. It opens **More**:

| Control | What it does |
|---|---|
| **Settings** | opens Settings |
| **Power off** | opens [Power](lock-and-power.md#power) |
| **Close** | closes the menu |

On such a window a page's top bar puts what has no room in its own
**⋮**, which opens **More** with those of these that apply:

| Control | What it does |
|---|---|
| **Learn** | opens Learn's pages for the page |
| **Show keyboard**, **Hide keyboard** | shows or hides the on-screen keyboard |
| **Show sheets**, **Hide sheets** | on Make codex32 shares: shows or hides the worksheets' contents |
| **Close** | closes the menu |

*Picture to come: `home-menu.png`, the ⋮ menu open on a tall narrow window.*

## On a narrow window

Home is one column: chips for the flows under way, the keys, the
wallets, Files and the sticks, each opening what its sidebar row opens;
then what the session holds and its stage, the lead and the tiles. The
pages are a rail of icons at the left, or a bar at the foot on a tall
narrow window.

## Why

- [Start here](../../learn/01-start-here.md)
- [The air gap](../../learn/07-air-gap.md)
- [Where keys are kept](../../learn/faraday/keys-kept.md)
