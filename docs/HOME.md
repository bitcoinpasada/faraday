# Home at every size

**Kind:** Spec
**Status:** Specified. Built today, and described in `docs/LAYOUT.md` §4 and `docs/FLOWS.md` (Home): on the one-column layouts the box of what is loaded, the lead and a menu of the places; on the wide layout the lead and four first-run tiles, whatever is loaded.
**Open:** the wide layout's Home is built (the lead as a card with one filled button, each loaded wallet with Sign, the first-run tiles only while nothing is loaded, no Check tile, F275); the one-column Home is not: one Home for the Pi and the navigation bar's and rail's windows with the same lead and wallet cards, the Pi's box of what is loaded held to three lines, the Files tile and the power button in words, the Backups row, the destinations in one order everywhere, and the sidebar's Files row once.

Home is the first page after the self-test and after every lock, and
the page a person comes back to between jobs. It answers three
questions in this order: what is loaded, what is the next thing to do,
and, for each wallet loaded, how to sign for it. A
person who has loaded nothing is shown the ways to start instead.

One model, every size. The Pi's 480 by 640 panel, the navigation bar's
and the rail's windows and the wide layout (1280 by 800 and up) draw the
same parts in the same order; they differ only in how many columns the
parts take and in what the navigation beside them already carries.

## 1. The parts, in order

1. **What is loaded** (§4): one card naming the flows in progress, the
   keys, the vaults open and the files, each line opening its place.
   Only where there is no expanded rail: beside the expanded rail the
   sidebar's lists are this card (`docs/LAYOUT.md` §3).
2. **The lead** (§2): the next thing, one card with one filled button.
   None when nothing waits and something is loaded.
3. **Wallets** (§3): each loaded wallet with **Sign**,
   then the Backups row while a wallet is known.
4. **Start** (§5): the first-run tiles, only while nothing is loaded.
5. **The places** (§6): on the Pi only, which has no navigation
   component, the menu of the destinations; then Lock and Power off.
6. **Scan**, the floating button at the bottom right, on every size.

"Nothing is loaded" means no key held, no wallet in the session and no
vault open. Files brought in, a stick attached and a vault file not yet
unlocked do not count: they are things waiting, and the lead names them.

## 2. The lead

The lead is the one next thing, the first that applies of the jobs
below. It is skipped where it would open or start over a flow whose
line is above it (`docs/FLOWS.md`, Flows in progress), as today.

| When | The card's line | Under it | Its button |
|---|---|---|---|
| A spend under way, its wallet loaded | {wallet} · Signatures {n} of {m} | | Carry on |
| A spend under way, its wallet not loaded | {source} | {n} of {m} signatures | Carry on |
| A backup not finished | {wallet or key} | Backup not finished | Finish |
| A PSBT in Files | {file name} | From Files | Sign |
| An import waiting | Import from {label} | | Import |
| An import put off with its vault locked | {vault} | Read from {label} | Unlock |
| Nothing loaded, and the last stick visit wrote | Written to {label} | the files' names | Open |
| Nothing loaded, and a vault file locked | {vault} | what it held when last seen | Unlock |
| A vault changed since it was written | {vault} | Changed since it was written | Write |
| A stick attached and not yet visited | Files on {stick} | A stick attached | Open |
| A key loaded that no wallet uses | Key {fingerprint} | No wallet uses it | Make a wallet |
| Nothing loaded and nothing waiting | Make a new wallet | Single key or multisig | Make |

The first ten are today's jobs and their order (`docs/FLOWS.md`, Home);
the key with no wallet is new, and is what that key's card on Wallets
already offers (`docs/WALLETS.md` §1, **Make a wallet from this key**).
Make a new wallet is the lead only while nothing is loaded; with a
wallet loaded and nothing waiting there is no lead, and the wallets
head the page.

**Drawn as** Material's filled card: the job's icon in its container,
the line, the line under it in the muted tone, and at the card's end
one filled button, the only filled button on Home. On the Pi the card is
one row as tall as its two lines; on wider windows it is at most
`LEAD_MAX` wide. A job that cannot be done here yet (a locked vault with
the stick still in) shows its button disabled and its second line, "Pull
the stick to unlock", in the warning tone. The whole card is not a
press: the button is, so the lead and the wallets below it read the
same way.

## 3. Wallets on Home

Under the lead, a section headed **Wallets**, one card for each wallet
in the session, in the sidebar's order (the wallets the keys here sign
for alone first, then those with some keys here, then watch-only).

Each card: the wallet icon, the wallet's name, one line under it, and at
its end **Sign**, Material's outlined button. The card
itself opens the wallet's card on Wallets (`Action::OpenWallet`), as the
name in the sidebar does.

- **The line** says the one thing that matters now, the first that
  applies: "Signatures {n} of {m}" while its spend is under way; "A PSBT
  to sign" while Files holds a PSBT for it; "Not backed up" while a key
  made here waits for its backup; else its shape as the sidebar row
  gives it ("single key", "2 of 3").
- **Sign** does what the wallet card's first button does
  (`spend_button`, read through the frame's memo): finish the backup
  first, carry on its spend, sign its PSBT in Files, or spend from it.

Home has no Receive button: Faraday is an offline signer, and the
wallet's card on Wallets keeps its "First receive address" line.

On the Pi at most three wallets are listed; past three, two and a row
"+ {n} more" that opens Wallets. Wider windows list them all in the
tile grid, each card at least `TILE_MIN` wide and wide enough for its
two buttons.

**Backups** follows the wallets while any wallet is known, as one row:
"Backups" and "{n} not backed up" or "{n} with a plan", opening Backups.
It takes the place of today's Backups tile on every size.

## 4. What is loaded

One card, a line for each of these that applies, in this order, each
opening what its sidebar row opens:

- each flow in progress, its name and "in progress";
- the keys, on one line: "{n} keys" or "1 key", then the first
  fingerprints in the muted tone as far as they fit, opening Keys;
- the vaults open, "{n} vaults open", opening Vaults (today's separate
  vaults card under the box goes into it);
- the files, only while Files holds a file or a stick is attached: what
  the sidebar's Files row says today ("8 files from the stick", "2 files
  to write to the stick"), or the stick's label;
- an import the boot stick brought, waiting.

The wallets are not in this card: §3 lists them with their buttons.
With nothing loaded, no file and no stick, there is no card: a new
person's first card is the lead, never "0 files · no stick".

**On the Pi** the card holds at most three lines: past three it shows the
first two and "+ {n} more", which opens the card in place to every line,
ending "Show less". Wider windows show every line.

## 5. Start, while nothing is loaded

While nothing is loaded, Start follows the lead (which is then Make a
new wallet, or a job waiting): **Restore an existing wallet**, "From its
backups"; **Spend from a backup, step by step**, "A stick, words, or
paper"; and, on the navigation bar's windows only, **Check a hardware
wallet** (§6.3). Make a new wallet is not a tile while it is the lead.
When a job waiting leads, Make a new wallet is Start's first tile.

Once anything is loaded, Start is gone: its ways in stay on Wallets
(Make a new wallet, Restore an existing wallet, Spend from a backup,
Add a key; `docs/WALLETS.md`), and Check is a destination.

## 6. The places

### 6.1 One order

The destinations stand in one order on the expanded rail, the rail, the
bar and the Pi's menu:

**Home, Check, Wallets, Files, Vaults, Tools**, then Guide and Settings
(and Transfer in the desktop app, after Tools).

The reason: Material asks that navigation keep one order as a window
grows from bar to rail to drawer, and that destinations stand by how
much they are used. Check stays right under Home (F255). Wallets is where
signing and receiving start; Files is where the PSBTs and transactions
to sign arrive and where what is written waits, which a person visits
at every signing; Vaults is opened at the start of a session and seldom
after; Tools is the long list of everything. It is also the order the
rail and the bar already have, so only the sidebar (today Check, Vaults,
Files, Wallets) and the Pi's menu (today Check, Wallets, Vaults, Files)
change.

The bar keeps its five, Home, Wallets, Files, Vaults, Tools: Check is
not on it (F255).

### 6.2 The Pi's menu

The Pi has no navigation component: its Home ends with the places as
tiles two to a row, in §6.1's order: Check, Wallets, Files, Vaults,
Tools, Guide, Transfer in the desktop app, Settings. Backups is no tile
(§3). A tile's line under its name is in words:

- Wallets: how many are loaded, "{n} loaded", or nothing;
- Files: "{n} in", "{n} to write", or both, "8 in · 2 to write", or
  nothing while Files is empty (today "0 · 0");
- Vaults: "{n} open" or "{n} locked", as today.

Under the tiles, **Lock** while something is held and **Power off**,
both labelled outlined buttons side by side, Power off alone and at its
own width when nothing is held (today an unlabelled icon). The expanded
rail's foot labels its power button the same way; the rail labels it
already.

### 6.3 Check on Home

On the rail's, the expanded rail's and the Pi's windows Check is a
destination right under Home, so Home carries no Check tile. On the
bar's windows Check is not on the bar, so Home keeps **Check a hardware
wallet**, "Its seed, addresses and signatures": among Start while
nothing is loaded, and as a tile after the Backups row once something
is. This narrows F255's second point ("Home's fourth tile is Check,
always") to the windows whose navigation lacks Check (F275). The wide
layout's Home is built so: no Check tile.

### 6.4 Files once in the sidebar

The sidebar draws Files twice today: as a tab, and as the row above
Lock that says what the files are. The row goes. The tab carries its
badge as now (the Inbox's count, or the dot for a stick not visited)
and, at its end in the muted tone as Vaults carries "{n} open", "{n} to
write" while files wait to be written. The sticks row stays.

## 7. At each size

The sketches are to scale in their parts' order, not in their pixels.

### 7.1 The Pi, 480 by 640, nothing loaded

```
+------------------------------------+
| [mark] FARADAY          (Testnet)  |
|        OFFLINE                     |
| +--------------------------------+ |
| | [w] Make a new wallet  [ Make ]| |  the lead, filled button
| |     Single key or multisig     | |
| +--------------------------------+ |
| +---------------+ +--------------+ |
| | Restore an    | | Spend from a | |  Start
| | existing      | | backup, step | |
| | wallet        | | by step      | |
| +---------------+ +--------------+ |
| +---------------+ +--------------+ |
| | [v] Check     | | [w] Wallets  | |  the places
| +---------------+ +--------------+ |
| +---------------+ +--------------+ |
| | [f] Files     | | [l] Vaults   | |
| +---------------+ +--------------+ |
|  ...Tools, Guide, Settings         |
|  [ Power off ]           [ Scan ]  |
+------------------------------------+
```

### 7.2 The Pi, 480 by 640, a key and four wallets loaded, a PSBT in

```
+------------------------------------+
| [mark] FARADAY          (Testnet)  |
|        OFFLINE                     |
| +--------------------------------+ |
| | [k] 1 key   784f9c44           | |  what is loaded
| | [f] 1 file from the stick      | |
| +--------------------------------+ |
| +--------------------------------+ |
| | [s] inheritance.psbt  [ Sign ] | |  the lead
| |     From Files                 | |
| +--------------------------------+ |
| Wallets                            |
| +--------------------------------+ |
| | Savings     (Sign)             | |  a wallet card
| | A PSBT to sign                 | |
| +--------------------------------+ |
| +--------------------------------+ |
| | Spending    (Sign)             | |
| | single key                     | |
| +--------------------------------+ |
|  + 2 more                       >  |
|  Backups · 1 not backed up      >  |
|  ...the places, Lock, Power off    |
|                          [ Scan ]  |
+------------------------------------+
```

### 7.3 The wide layout, 1280 by 800, nothing loaded

```
+-------------+----------------------------------------------------------+
| FARADAY     | Home                                      (Testnet)  ?   |
| OFFLINE     |                                                          |
| Home        | +----------------------------------------------+         |
| Check       | | [w] Make a new wallet              [ Make ]  |         |
| Wallets     | |     Single key or multisig                   |         |
| Files       | +----------------------------------------------+         |
| Vaults      | Start                                                    |
| Tools       | +-------------------------+ +-------------------------+  |
| Guide       | | Restore an existing     | | Spend from a backup,    |  |
| Settings    | | wallet  From its backups| | step by step            |  |
|             | +-------------------------+ +-------------------------+  |
|             |                                                          |
| [Power off] |                                                [ Scan ]  |
+-------------+----------------------------------------------------------+
```

### 7.4 The wide layout, 1280 by 800, keys and wallets loaded

```
+-------------+----------------------------------------------------------+
| FARADAY     | Home                                      (Testnet)  ?   |
| OFFLINE     |                                                          |
| Home        | +----------------------------------------------+         |
| Check       | | [s] inheritance.psbt               [ Sign ]  |         |
| Wallets     | |     From Files                               |         |
| Files    77 | +----------------------------------------------+         |
| Vaults open | Wallets                                                  |
| Tools       | +--------------------------+ +--------------------------+|
| Guide       | | Savings                  | | Spending                 ||
| Settings    | | A PSBT to sign           | | single key               ||
| ----------- | |                  (Sign)  | |                  (Sign)  ||
| Keys      3 | +--------------------------+ +--------------------------+|
|  9a6a2580   | +--------------------------+ +--------------------------+|
|  5d388376   | | Taproot                  | | Legacy multisig          ||
| Wallets  11 | | single key               | | 2 of 3 · watch only      ||
|  ...        | |                  (Sign)  | |                  (Sign)  ||
| [Lock]      | +--------------------------+ +--------------------------+|
| [Power off] | Backups · 1 not backed up                     >  [ Scan ]|
+-------------+----------------------------------------------------------+
```

The rail's and the bar's windows draw the Pi's column (§7.1, §7.2)
under the top app bar, without the head and without the places (the
navigation carries them), Lock and Power off being on the rail and in
the ⋮ menu; on the bar's windows the Check tile of §6.3 follows Backups.

## 8. Strings

New, in `faraday/faraday-strings/src/en/screens_home.rs` beside Home's
others:

- the lead's buttons: "Carry on", "Finish", "Sign", "Import", "Unlock",
  "Open", "Write", "Make a wallet", "Make";
- the lead's lines that change: "Backup not finished", "Changed since it
  was written" (today the vault's currency line is reused; kept if it
  reads so), "Key {}", "No wallet uses it";
- Wallets: the section's head "Wallets", "Sign", "A PSBT to
  sign", "Not backed up", "Signatures {} of {}";
- what is loaded: "1 key", "{} keys", "Show less".

New, in `faraday/faraday-strings/src/en/compact.rs`: the Pi's tile
lines "{} loaded", "{} in", "{} to write".

New, in `faraday/faraday-strings/src/en/screens_sidebar.rs`: the Files
tab's "{} to write".

Kept as they are: "+ {} more" (`sidebar_more`), "in progress", "{} {}
open", "Backups", "{} not backed up", "{} with a plan", "Restore an
existing wallet" and the other Start tiles, "Power off" (`nav_power_off`).
Left unused once built: `home_job_sign`'s "Sign {}" and the Pi's
"{} · {}" files line (`home_files_line`).

## 9. What drawing reads

Drawing reads and does not work things out (`docs/SPEED.md` §3.3,
`docs/COMPUTE.md`). Sign's action and the wallet's line come from the
frame's memo (`View::spend_button`, `crate::glance::Reads`); the
lead's ranking stays the lookup `home_jobs` is. No new costly work is
drawn: a wallet card on Home reads what the wallet card on Wallets
reads.

## 10. Against Material 3

Checked against Material 3's pages: Cards (filled and outlined cards,
actions at a card's end), Lists (the box of what is loaded, the Backups
and "+ N more" rows), Buttons (one filled button on a page; outlined
buttons for the wallets' two actions), Floating action button (Scan),
Navigation bar, Navigation rail and Navigation drawer (one order of
destinations across them, at most five on the bar), and the canonical
Feed layout (the wide Home's grid).

- No difference from Material is added. The lead is a filled card with
  a filled button; the wallets are outlined cards with two buttons at
  their end; the destinations keep one order on every navigation.
- The Pi's Home as the menu of destinations is the one-column short
  window's rule already (`docs/LAYOUT.md` §3), not new.
- §6.3, Check's tile only where the navigation lacks Check, is not a
  difference from Material; it changes F255's second point, and so
  needs a decision.

## 11. Tests

What a person would notice: with a wallet loaded, Home on the Pi and on
the wide layout offers its Sign, and Sign opens what the
wallet card's first button opens; with nothing loaded, Start shows and
the first card is the lead, not a files line; with a key and no wallet,
the lead makes a wallet from it; once something is loaded Start is gone;
with four or more wallets on the Pi, two and "+ N more" show, and "+ N
more" opens Wallets; the Pi's Files tile and the box name files in
words; Power off is labelled and powers off. The destinations' order is
a design rule, held by review and the renders, not by a test.
