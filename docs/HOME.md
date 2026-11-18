# Home at each layout

**Kind:** Reference
**Status:** Built: the wide layout's Home, the phone-size window's and the rail's windows' (one column with the search bar and the places), and the Pi's menu. They are three Homes, not one (owner, 2026-11-18).

Home is the first page after the self-test and after every lock, and
the page a person comes back to between jobs. It answers what is loaded,
what is the next thing to do and, on the wide layout, how to sign for
each wallet loaded. A person who has loaded nothing is shown the ways to
start instead.

Home is drawn by layout, not by one model:

- **The wide layout** (1280 by 800 and up, the expanded rail):
  `faraday-core`'s `screens/home.rs`. The sidebar carries what is loaded
  and the places, so the page holds the lead, the wallets, the first
  actions and the search bar.
- **The Pi's menu** (the Pi's 480 by 640 panel, no navigation component)
  and **the one-column windows with navigation** (the rail's and the
  phone-size window's bar): `compact/mod.rs`. A box of what is loaded,
  one row for the lead, the first actions where there is a search bar,
  and the places as tiles.

The phone-size window differs from the rail's in two things: its search
bar is first, under the box of what is loaded, and a search typed takes
the page; and it always stacks the first actions. The Pi's menu has no
search bar and no first actions.

## 1. The parts, in order

**The wide layout:**

1. **The lead** (§2): a card with one filled button. None when nothing
   waits and something is loaded.
2. **Wallets** (§3): each loaded wallet as a card with **Sign**.
3. **The three first actions** (§5), only while nothing is loaded.
4. **The Backups row** while a wallet is known (§3).
5. **The search bar** (§5), centred.
6. **Scan**, the floating button at the bottom right.

**The Pi's menu, the rail's windows and the phone-size window:**

1. On the Pi's menu only, the head (the mark and the name, off mainnet
   the mode pill); on the rail's and the phone-size windows the top app
   bar carries the name.
2. **What is loaded** (§4): one box, not drawn while it has nothing to
   show.
3. On the phone-size window only, **the search bar** (§5), with a typed
   search's results taking the page.
4. A card for the vaults open, while any is.
5. **The lead** (§2): one row.
6. **The three first actions** (§5), only while nothing is loaded, and not
   on the Pi's menu.
7. On the rail's windows, **the search bar**.
8. **The places** (§6) as tiles; then, on the Pi's menu only, Lock (while
   something is held) and Power off; the rail's windows have them on the
   rail, the phone-size window's Lock in the top app bar and its ⋮ menu.
9. **Scan**, the floating button at the bottom right.

"Nothing is loaded" (`nothing_loaded`) means no key held, no wallet in the
session and no vault open. Files brought in, a stick attached and a vault
file not yet unlocked do not count: they are things waiting, and the lead
names them.

## 2. The lead

The lead is the one next thing, the first that applies of the jobs below.

| When | The card's line | Under it | Its button |
|---|---|---|---|
| A spend under way, its wallet loaded | {wallet} · Signatures {n} of {m} | | Carry on |
| A spend under way, its wallet not loaded | {source} | {n} of {m} signatures | Carry on |
| A backup not finished | {wallet or key} | Backup not finished | Finish |
| A PSBT in Files | {file name} | From Files | Sign |
| An import waiting | Import from {label} | | none: the card is the press |
| An import put off with its vault locked | {vault} | Read from {label} | Unlock |
| Nothing loaded, and the last stick visit wrote | Written to {label} | the files' names | Open |
| Nothing loaded, and a vault file locked | {vault} | what it held when last seen | Unlock |
| A vault changed since it was written | {vault} | Changed since it was written | Write |
| A stick attached and not yet visited | Files on {stick} | A stick attached | Open |
| A key loaded that no wallet uses (wide layout only) | Key {fingerprint} | No wallet uses it | Make a wallet |

A job that would open or start over a flow whose line is in the box of
what is loaded gives way to the next, on the one-column Homes (the wide
layout has no such box). A job that cannot be done here yet (a locked
vault with the stick still in) shows its button disabled and its second
line, "Pull the stick to unlock", in the warning tone.

**On the wide layout** the lead is Material's filled card: the job's icon
in its container, the line, the line under it in the muted tone, and at
the card's end one filled button, the only filled button on Home, the card
at most `LEAD_MAX` wide. The card with a button is not a press: the button
is. The plain Import card has no button and is pressed whole. With a
wallet loaded and nothing waiting there is no lead and the wallets head
the page; with nothing loaded and nothing waiting there is no lead and
the first actions stand first.

**On the one-column Homes** the lead is one row, 44 dp tall, in the
secondary container (the warning container for a job that cannot be done
yet): the job's icon, its line, a chevron at its end, the whole row the
press, at most `LEAD_MAX` wide (full width on the Pi's menu). It takes the
card's line only, never the line under it or a button. Two differences
from the wide layout: a key that no wallet uses is no lead (its card on
Wallets offers **Make a wallet from this key**, `docs/WALLETS.md` §1), and
**Make a new wallet** is the lead on the Pi's menu while nothing waits,
but on the rail's windows and the phone-size window it is the first of
the three first actions and nothing leads.

## 3. Wallets on Home (the wide layout)

Under the lead, a section headed **Wallets**, one card for each wallet in
the session, in the sidebar's order (the wallets the keys here sign for
alone first, then those with some keys here, then watch-only), in a grid,
each card at least `TILE_MIN` wide and wide enough for its button.

Each card: the wallet icon, the wallet's name, one line under it, and at
its end **Sign**, Material's outlined button. The card itself opens the
wallet's card on Wallets (`Action::OpenWallet`), as the name in the
sidebar does.

- **The line** says the one thing that matters now, the first that
  applies: "Signatures {n} of {m}" while its spend is under way; "A PSBT
  to sign" while Files holds a PSBT for it; "Not backed up" while a key
  made here waits for its backup; else its shape as the sidebar row gives
  it ("single key", "2 of 3", or the line for a wallet another key
  spends).
- **Sign** does what the wallet card's first button does (`spend_button`):
  finish the backup first, carry on its spend, sign its PSBT in Files, or
  spend from it.

Home has no Receive button: Faraday is an offline signer, and the wallet's
card on Wallets keeps its "First receive address" line.

After the wallets, while any wallet is known, one **Backups** tile:
"Backups" and "{n} not backed up" or "{n} with a plan", opening Backups.

The one-column Homes have no wallet cards: they name each wallet in the
box of what is loaded (§4), and the places carry a Backups tile (§6.2).

## 4. What is loaded (the one-column Homes)

One box, drawn on the Pi's menu, the rail's windows and the phone-size
window, in the order below, laid out as lines side by side and wrapping to
rows; each line opens what its sidebar row opens:

- each flow in progress, its name and "in progress", resuming it;
- each key, its fingerprint and its name when it has one, opening it in
  Explore; past four keys (when more than five are loaded) "+ {n} more",
  opening Explore;
- each wallet by name, opening its card; past four wallets (when more than
  five are loaded) "+ {n} more", opening Wallets;
- the files line, opening Files: the files' count, or, while none are held,
  the stick's label or that none is attached;
- an import the boot stick brought, waiting.

The vaults open are not in the box: a card of their own follows it,
"{n} vaults open", opening the session.

The box is not drawn while it has nothing to show (`loaded_empty`): no
flow, key, wallet, file brought in or waiting, stick or import waiting; a
new person's first card is then the lead or the first actions. The wide
layout has no box: the sidebar's lists are it (`docs/LAYOUT.md` §3).

## 5. The first actions and the search bar

While nothing is loaded, the three first actions stand under the title on
the wide layout and the rail's windows, after the lead when a job waits,
and under the search bar on the phone-size window: **Make a new wallet**,
**Restore an existing wallet** and **Spend from a backup**. They are three
like buttons with their heading only, no small line, no label over them,
no pill: the whole button is the press. On the wide layout they always
stand as one row of three equal columns, a heading too long for its column
wrapping onto a second line, the three buttons one height. On the rail's
windows they stand as a row where three columns leave every heading room
within two lines, else one under the other at full width. The phone-size
window always stacks them. The Pi's menu has none.

The search bar shows the search icon and the word "Search", on every
layout but the Pi's menu. On the phone-size window it is at the top,
directly under the box of what is loaded, above the lead and the three
actions; while something is typed its results take the page and the lead,
the actions and the places are not drawn until the
search is cleared. On the wide layout it is under the first actions, or
under the Backups tile once something is loaded, centred and at most
`LEAD_MAX` wide; on the rail's windows under the first actions at full
width. Typing in it there changes nothing above it, neither what is drawn
nor where: only the results under the bar scroll. That holds where the
content above the bar leaves room under it for about three rows of
results (`home_pinned`); on a window too short for that, the whole page
scrolls as one, the same content in the same order. A press in the bar
shows its caret, which blinks.

Once anything is loaded, the three actions are gone: their ways in stay
on Wallets (Make a new wallet, Restore an existing wallet, Spend from a
backup, Add a key; `docs/WALLETS.md`).

## 6. The places

### 6.1 One order

The destinations stand in one order on the sidebar, the rail and the bar:

**Home, Check, Wallets, Files, Vaults, Tools**, then Guide and Settings
(Transfer, in the desktop app, after Tools on the sidebar and the rail,
above Settings). The bar keeps its five, Home, Wallets, Files, Vaults,
Tools: Check is not on it (F255). Material asks that navigation keep one
order as a window grows from bar to rail to drawer. The Pi's menu keeps
Wallets before Check (§6.2, F255 amended 2026-11-18).

### 6.2 The places as tiles

The Pi's menu, the rail's windows and the phone-size window end the page
with the places as tiles: two to a row on the Pi's menu, as many columns
as `TILE_MIN` leaves room for elsewhere. In order: Check, Wallets, Backups
(while a wallet is known), Files, Vaults, Tools, Guide, Transfer (the
desktop app, online), Settings; on the Pi's menu Wallets and Check change
places. A tile's line under its name: Wallets, the count of wallets, or
nothing; Backups, "{n} not backed up" or nothing; Files, "{inbox} ·
{to write}"; Vaults, "{n} open", "{n} locked" or nothing. Under the tiles
on the Pi's menu, **Lock** while something is
held and **Power off**, labelled outlined buttons side by side, Power off
alone at the full width when nothing is held. The wide layout has no
tiles: the sidebar is its places.

### 6.3 Check on Home

Check is a tile of the places wherever they are drawn (the Pi's menu, the
rail's windows, the phone-size window); the wide layout's Home draws none,
the sidebar listing Check (F275).

### 6.4 Files once in the sidebar

The sidebar lists Files once, as a tab. It carries its badge (the Inbox's
count, or the dot for a stick not visited) and, at its end in the muted
tone, "{n} to write" while files wait to be written. The sticks row at the
foot stays.

## 7. At each size

The sketches are to scale in their parts' order, not in their pixels.

### 7.1 The Pi's menu, 480 by 640, nothing loaded

```
+------------------------------------+
| [mark] FARADAY          (Testnet)  |
|        OFFLINE                     |
| +--------------------------------+ |
| | [w] Make a new wallet        > | |  the lead, one row
| +--------------------------------+ |
| +---------------+ +--------------+ |
| | [w] Wallets   | | [v] Check    | |  the places
| +---------------+ +--------------+ |
| +---------------+ +--------------+ |
| | [f] Files     | | [l] Vaults   | |
| +---------------+ +--------------+ |
|  ...Tools, Guide, Settings         |
|  [ Power off ]           [ Scan ]  |
+------------------------------------+
```

With a key, a wallet or a file the box of what is loaded stands above the
lead, a Backups tile follows Check once a wallet is known, and Lock stands
beside Power off.

### 7.2 The wide layout, 1280 by 800, nothing loaded

```
+-------------+----------------------------------------------------------+
| FARADAY     | Home                                      (Testnet)  ?   |
| OFFLINE     |                                                          |
| Home        | +-----------------+ +-----------------+ +--------------+ |
| Check       | | [w] Make a new  | | [r] Restore an  | | [s] Spend    | |
| Wallets     | |     wallet      | |     existing    | |     from a   | |
| Files       | |                 | |     wallet      | |     backup   | |
| Vaults      | +-----------------+ +-----------------+ +--------------+ |
| Tools       |                                                          |
| Guide       |        ( search  Search                           )      |
| Settings    |                                                          |
|             |                                                [ Scan ]  |
| [Power off] |                                                          |
+-------------+----------------------------------------------------------+
```

A job waiting (an import, say) stands as its card above the row; the
plain Import card has no pill and is pressed whole. Typing in the search
bar leaves the card, the row and the bar where they are; the results show
and scroll under the bar.

### 7.3 The wide layout, 1280 by 800, keys and wallets loaded

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

### 7.4 The phone-size window, nothing loaded

```
+------------------------------+
| FARADAY   (Testnet)  ?  ⋮    |  the top app bar
| ( search  Search            ) |  the search bar; the box of what
| +--------------------------+ |  is loaded above it only once
| | [w] Make a new wallet    | |  something is loaded
| +--------------------------+ |
| +--------------------------+ |
| | [r] Restore an existing  | |  the three first actions,
| |     wallet               | |  stacked at full width
| +--------------------------+ |
| +--------------------------+ |
| | [s] Spend from a backup  | |
| +--------------------------+ |
| the places                   |
|                     [ Scan ] |
+------------------------------+
```

While something is typed the results alone fill the page under the bar.
Only what is under the search bar scrolls, until the window is too short
to leave room under the bar for about three rows of results; then the
whole page scrolls (§5). The rail's windows draw the same page with the search
bar under the first actions, the rail beside it carrying Lock and Power
off.

## 8. Strings

Home's strings are in `faraday/faraday-strings/src/en/screens_home.rs`,
the one-column Homes' in `compact.rs`, the sidebar's in
`screens_sidebar.rs`.

## 9. What drawing reads

Drawing reads and does not work things out (`docs/SPEED.md` §3.3,
`docs/COMPUTE.md`). Sign's action and the wallet's line come from the
frame's memo (`View::spend_button`); the lead's ranking is a lookup
(`home_jobs`). No new costly work is drawn: a wallet card on Home reads
what the wallet card on Wallets reads.

## 10. Against Material 3

Checked against Material 3's pages: Cards (filled and outlined cards,
actions at a card's end), Lists (the box of what is loaded), Buttons (one
filled button on a page; the outlined Sign), Search (the search bar),
Floating action button (Scan), Navigation bar, Navigation rail and
Navigation drawer (one order of destinations across them, at most five on
the bar), and the canonical Feed layout (the wide Home's grid).

- No difference from Material is added. The lead on the wide layout is a
  filled card with a filled button; the wallets are outlined cards with
  one button at their end.
- The Pi's Home as the menu of destinations is the one-column short
  window's rule (`docs/LAYOUT.md` §3).
- Check's tile where the navigation does not carry it, the bar's windows,
  is not a difference from Material (F275).

## 11. Tests

What a person would notice: with nothing loaded the three first actions
show as headings only, each pressed as a whole, and a job waiting leads;
the wide Home's first actions and search bar stay where they were while a
search is typed, and on the phone-size window a search typed takes the
page; a Home taller than its window scrolls to its places; Home offers
Scan, Backups once a wallet is known and the lead for each of the jobs of
§2; the Pi's menu puts Wallets left of Check and the phone keeps Check
first. The destinations' order is a design rule, held by review and the
renders, not by a test.
