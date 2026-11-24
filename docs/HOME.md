# Home at each layout

**Kind:** Reference
**Status:** Built: the wide layout's Home, the phone-size window's and the rail's windows' (one column with the search bar and the box; the rail's has no place tiles), and the Pi's menu. They are three Homes, not one (owner, 2026-11-18).

Home is the first page after the self-test and after every lock, and
the page a person comes back to between jobs. It answers what is loaded,
what is the next thing to do and, on the wide layout and the rail's
windows, which keys and wallets are in it. The ways to start stand
whether or not anything is loaded.

Home is drawn by layout, not by one model:

- **The wide layout** (1280 by 800 and up, the expanded rail):
  `faraday-core`'s `screens/home.rs`. The sidebar carries the places, so
  the page holds the search bar, the first actions and the card of
  what is going on (the lead its first item), wallets and keys, in the column of Tools'
  catalogue (the menu page container, `docs/MENU-PAGE.md`, `docs/LAYOUT.md` §4).
- **The Pi's menu** (the Pi's 480 by 640 panel, no navigation component)
  and **the one-column windows with navigation** (the rail's and the
  phone-size window's bar): `compact/mod.rs`. On the Pi's menu and the
  phone-size window a box of what is loaded, one row for the lead, the
  first actions where there is a search bar, and the places as tiles; on
  the rail's windows the search bar, the first actions, the same
  box of what is going on (the lead its first item), wallets and keys as the wide layout,
  and the places.

The phone-size window differs from the rail's: its search bar is first,
under the box of what is loaded, the first actions stand only while
nothing is loaded and are always stacked, and it has the box instead of
the chips and the lists. The Pi's menu has no search bar and no first
actions.

## 1. The parts, in order

**The wide layout:**

1. **The search bar** (§5), centred, first and still while the page under
   it scrolls; a search typed takes the page.
2. **The lead** (§2) is not a part of its own: it is the first item of
   §3's card (F315).
3. **The three first actions** (§5), always.
4. **What is going on, wallets and keys** (§3): one tinted box of list rows: flows in progress and files waiting, the wallets that need something and the keys no wallet uses, then Backups while something is not backed up, and All keys.
5. **Scan**, the extended FAB at the sidebar's foot, its left edge on the shield's (F312), not on
   the page.

**The Pi's menu, the rail's windows and the phone-size window** (the
rail's windows differ as the list says):

1. On the Pi's menu only, the head (the mark and the name, off mainnet
   the mode pill); on the rail's and the phone-size windows the top app
   bar carries the name.
2. **What is loaded** (§4): one box, not drawn while it has nothing to
   show; not on the rail's windows, where §3's box of list rows
   stand in its place, further down.
3. On the phone-size window and the rail's windows, **the search bar**
   (§5), first on the rail's windows, with a typed search's results taking
   the page.
4. **The lead** (§2): one row.
5. **The three first actions** (§5), only while nothing is loaded (always
   on the rail's windows), and not on the Pi's menu.
6. On the rail's windows, **what is going on, the wallets and Backups,
   then the Keys** (§3), the box scrolling within itself under the fixed
   top.
7. **The places** (§6) as tiles, not on the rail's windows (the rail lists them); then, on the Pi's menu only, Lock (while
   something is held) and Power off; the other windows have them in the top
   app bar's power button's menu (F304).
8. **Scan**: on the rail's windows the FAB at the foot of the rail (F312), on
   the Pi's menu and the phone-size window the floating button at the bottom right.

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

The boot stick at start counts as visited at its first look when it holds no file but the settings file: an empty one gives no tile. A stick put in later, or the boot stick put in again, waits for its visit as any stick does.

A job that would open or start over a flow whose line is in the box of
what is loaded gives way to the next, on the one-column Homes. On the
wide layout and the rail's windows the lead stands in the card (§3), and the
card's row that only repeats its job (the flow it carries on; the wallet
whose PSBT or spend it is) is not drawn while it stands. A job that cannot be
done here yet (a locked vault with the stick still in) shows its button
disabled and its second line, "Pull the stick to unlock", in the warning
tone.

**On the wide layout and the rail's windows** the lead is the first row of
the card's Waiting group (§3): the job's icon, its line and the line under it,
and under them, at the text's start, its one button (Carry on, Finish, Sign,
Unlock, Write, Open). A row with a button is not a press: the button is. The
plain Import row has no button and is pressed whole. The jobs below are not
only the first that applies: each one that applies is a row, one for each
waiting PSBT (fewest signatures first, each with its own Sign) and one for each
locked vault file (each with its own Unlock), and they stay with keys and
wallets loaded (owner, 2026-11-23). Nothing about what is going on is drawn
outside the card; the search row stays above it. With nothing waiting and
nothing loaded the card holds the one line "Nothing loaded".

**On the one-column Homes** the lead is one row, 44 dp tall, in the
secondary container (the warning container for a job that cannot be done
yet): the job's icon, its line, a chevron at its end, the whole row the
press, the page's full width. It takes the
card's line only, never the line under it or a button. A key that no
wallet uses is no lead on any layout: its row under Keys on Home (§3) and
its card on Wallets offer **Make a wallet from this key**
(`docs/WALLETS.md` §1). **Make a new wallet** is the lead on the Pi's menu while nothing waits,
but on the rail's windows and the phone-size window it is the first of
the three first actions and nothing leads.

## 3. What is going on, wallets and keys (the wide layout and the rail's windows)

Home has no row of chips. What is going on is the first rows of the box
below: the chips over every page beside the rail and the flow rows in the
sidebar stay as they are (F41, `docs/LAYOUT.md` §3). A vault open is not on
Home: the Vaults entry of the sidebar and of the rail carries a badge with
how many are open, and Wallets one with the number of wallets that need
something now, as Files' badge is the Inbox count (F235, F303); none while
the number is zero.

One card, drawn by the session pane (`docs/SESSION-PANE.md`), 24 dp under the
search row (`HOME_CARD_GAP`) on the one-column layouts, and on the wide layout where every menu page's content starts, just under the top bar, as wide as the menu pages
(`MENU_PAGE_W`) at most and centred (the whole page where that is narrower).
Its surface is neutral (surface-container-low) and raised at the Scan button's
elevation (level 3), where Material's elevated card is level 1 (F315); orange
stands only on what needs something. One heading stands inside the card at its
top left, "This session" (Title Large, SemiBold; `home_box_heading`); three
groups stand under small headings (Title Small), in this order, a group with
no rows not drawn (F323):

- **Waiting**: the lead and every other job of §2 as rows with their buttons
  under their lines (one for each waiting PSBT and each locked vault file),
  then a row for each flow in progress (its name, its open step under it,
  Resume), and while files wait to be written, a row with their count,
  opening Files;
- **Wallets**: a row for every loaded wallet, ranked (the wallets the keys here
  sign for alone first, then those with some keys here, then watch-only): its
  name and, under it, the first that applies, "Signatures {n} of {m}" while its
  spend is under way, "A PSBT to sign" while Files holds a PSBT for it, "Not
  backed up" while a key made here waits for its backup, else its shape. A
  wallet that needs something has the warning icon. The row opens the wallet's
  card on Wallets (`Action::OpenWallet`) and has no button; **Backups** is the
  last row, with the shield icon, only while something is not backed up;
- **Keys**: a row for every key held, no cap: its fingerprint, under it its
  name in full (none when the name is the fingerprint), "passphrase" when the
  key was loaded with one and the wallet that uses it ("Used by {wallet}"),
  else "No wallet uses it". The row opens the key (`Action::ExploreKey`). There
  is no All keys row and no Make a wallet button here.

The groups stand in as many columns as fit of at least 336 dp
(`SESSION_COL_MIN`), 24 dp apart, each group whole and the next into the
shortest column. Nothing loaded: the card stands at its place with what is real
of Waiting, and with none of it holds "Nothing loaded"; it has no Make or
Restore row. The card is as tall as its groups; the page does not scroll, on
the sidebar windows and on the rail's windows: the search bar stays, and where
the card would run past a margin (`MARGIN_WIDE`) above the window's foot it
scrolls within itself, all columns as one (its own offset, `Slot::HomeBox`), and
no row is dropped. Where the fixed part leaves the box under two rows
(`HOME_BOX_MIN_ROWS`) the page scrolls as a whole instead, the box with it. The
rail's windows draw no place tiles (§6.2): the rail lists every place, so their
box is the last thing on the page.

Home has no Sign button: the card on Wallets and the lead carry what signs
next. Home has no Receive button either: Faraday is an offline signer, and
the wallet's card on Wallets keeps its "First receive address" line. The
sidebar lists neither wallets nor keys. The page does not scroll, on the sidebar windows and on the rail's windows: the search bar and the first actions
stay, and the card grows with its rows down to a margin (`MARGIN_WIDE`) above
the window's foot, then scrolls within itself, both columns as one (its own offset, `Slot::HomeBox`),
as a list pane does (owner, 2026-11-21). Where the fixed part leaves the
box under two rows (`HOME_BOX_MIN_ROWS`) the page scrolls as a whole instead,
the box with it. The rail's windows draw no place tiles (§6.2): the rail
lists every place, so their box is the last thing on the page.

The Pi's menu and the phone-size window have no such lists: they name each
wallet and key in the box of what is loaded (§4), and the places carry a
Backups tile (§6.2).

## 4. What is loaded (the Pi's menu and the phone-size window)

One box, drawn on the Pi's menu and the phone-size window (the rail's
windows have §3's box of list rows instead, with no files line), in the order below, laid out as lines side by side and wrapping to
rows; each line opens what its sidebar row opens:

- the vaults open, first: the vault's name and "open" for one, "{n} vaults
  open" for more, opening the session;
- each flow in progress, its name and "in progress", resuming it;
- each key, its fingerprint and its name when it has one, opening it in
  Explore; past four keys (when more than five are loaded) "+ {n} more",
  opening Explore;
- each wallet that needs something now (§3's lines) by name, opening its
  card, at most four; the wallets not named are one line, "+ {n} more"
  after those named, else "All wallets ({n})", opening Wallets;
- the files line, opening Files: the files' count, or, while none are held,
  the stick's label or that none is attached;
- an import the boot stick brought, waiting.

A line is as wide as its text and is never cut short while room is left
beside it.

The box is not drawn while it has nothing to show (`loaded_empty`): no
vault open, flow, key, wallet, file brought in or waiting, stick or import waiting; a
new person's first card is then the lead or the first actions. The wide
layout has no box: §3's box of list rows is it.

## 5. The first actions and the search bar

The three first actions stand on the wide layout and the rail's windows
always, whether or not anything is loaded, in the top app bar with the
search (F314, F323 point 4), as text-only tonal pills (no icon): **New
wallet**, **Restore** and **Spend from backup**, after the search field,
which shrinks to what the page's name, the pills and the bar's actions leave.
Group and field are centred over the page's centre line where the bar has
room, else packed from the name's end; there is no second row (`screens/bar_search.rs`). The
bar never scrolls. On the rail's windows the row of F314 stays as it was. On the phone-size window the three stand only while nothing is
loaded, under the search bar, as three like buttons with their full heading
and no small line, always stacked: **Make a new wallet**, **Restore an
existing wallet** and **Spend from a backup**. The Pi's menu has none.

The search bar shows the search icon and the word "Search", on every
layout but the Pi's menu. It stands in the top app bar, 48 dp tall, on every window with a rail from 600 dp wide, with the three first actions as pills beside it from about 1020 dp and as one **Start** button (a down chevron after its label) that opens a menu of the three under it, a popup at every window size, below that (on a rail window of the compact class the field and the button stand in the page's row), and stays where it is while the page under it scrolls. On the phone-size
window it is directly under the box of what is loaded and, where the
window leaves room for about three rows of results (`home_pinned`), stays
too; else the whole page scrolls as one. On every layout with the bar,
while something is typed its results take the page: the lead, the first
actions, the wallets, the keys, Backups and the places are not drawn until
the search is cleared. On a window with the search in the top bar the card
of what is loaded (This session) stays and the results stand in a tile
under it, or, where the window is too short for that, in a tile at its right
with the card at the left (`docs/MENU-PAGE.md` §4). The results are the matching tools, then every
Settings card with a word typed on it (its title, labels, options' names
and values' words, taken from the strings the card draws), each shown as
the whole card, drawn and pressable as on Settings, so a change made in it
is the same change; then the glossary words. "appearance", "contrast" and
"orange" (a theme's name) give the Appearance card, "time" the Time card,
"about" the About card. Tools' bar shows the same. A press in the bar
shows its caret, which blinks.

## 6. The places

### 6.1 One order

The destinations stand in one order on the sidebar, the rail and the bar:

**Home, Check, Wallets, Files, Vaults, Tools**, then Settings
(Transfer, in the desktop app, after Tools on the sidebar and the rail,
above Settings). The bar keeps its five, Home, Wallets, Files, Vaults,
Tools: Check is not on it (F255). Material asks that navigation keep one
order as a window grows from bar to rail to drawer. The Pi's menu keeps
Wallets before Check (§6.2, F255 amended 2026-11-18).

### 6.2 The places as tiles

The Pi's menu and the phone-size window end the page
with the places as tiles (the rail's windows draw none: the rail lists every
place): two to a row on the Pi's menu, as many columns
as `CATALOG_TILE_MIN` leaves room for elsewhere. In order: Check, Wallets, Backups
(while a wallet is known), Files, Vaults, Tools, Guide (the Pi's menu only), Transfer (the
desktop app, online), Settings; on the Pi's menu Wallets and Check change
places. A tile's line under its name: Wallets, the count of wallets, or
nothing; Backups, "{n} not backed up" or nothing; Files, "{inbox} ·
{to write}"; Vaults, "{n} open", "{n} locked" or nothing. Under the tiles
on the Pi's menu, **Lock** while something is
held and **Power off**, labelled outlined buttons side by side, Power off
alone at the full width when nothing is held. The wide layout has no
tiles: the sidebar is its places.

Home has no Guide tile except on the Pi's menu, whose Home has no top bar:
everywhere else the top bar's ? is the way into the guide, the phone-size
windows included (F307 amended 2026-11-21).

### 6.3 Check on Home

Check is a tile of the places where the navigation lacks Check (the Pi's
menu, the phone-size window); the rail's windows and the wide layout's Home
draw none, the rail and the sidebar listing Check (F275).

### 6.4 Files once in the sidebar

The sidebar lists Files once, as a tab. It carries its badge (the Inbox's
count, or the dot for a stick not visited), and no second count. The sticks row at the
foot stays. The sidebar lists no keys and no wallets: they are on Home (§3).

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
| Home        | ( search  Search         ) (New wallet)(Restore)(Spend.) |
| Check       |                                                          |
| Wallets     |                                                          |
| Files       |                                                          |
| Vaults      |                                                          |
| Tools       |                                                          |
|             |                                                          |
| Settings    |                                                          |
|             |                                                [ Scan ]  |
| [Power off] |                                                          |
+-------------+----------------------------------------------------------+
```

A job waiting (an import, say) stands as its card under the top row; the plain Import card has no pill and is pressed whole. Typing in the
search bar leaves the bar where it is and takes the page: the card
and everything under it are not drawn, the results show and scroll
under the bar.

### 7.3 The wide layout, 1280 by 800, keys and wallets loaded

```
+-------------+----------------------------------------------------------+
| FARADAY     | Home                                      (Testnet)  ?   |
| OFFLINE     |                                                          |
| Home        | ( search  Search         ) (New wallet)(Restore)(Spend.) |
| Check       | +------------------------------------------------------+ |
| Wallets   1 | | [s] inheritance.psbt                     [ Sign ]    | |
| Files    77 | |     From Files                                       | |
| Vaults    1 | +------------------------------------------------------+ |
|             | +------------------------------------------------+     |
|             | | [w] Spending                                 > |     |
|             | |     Signatures 1 of 2                          |     |
|             | | [!] Savings                                  > |     |
| [Lock]      | |     A PSBT to sign                             |     |
| [Power off] | | [s] Backups                                  > |     |
|             | |     1 not backed up                            |     |
|             | | [k] All keys                           3     > |     |
|             | | [k] c3a1f20e               Make a wallet     > |     |
|             | |     savings · No wallet uses it                |     |
|             | +------------------------------------------------+     |
|             |                                                          |
+-------------+----------------------------------------------------------+
```

Many rows scroll within the box, not the page. On the rail's windows the same box of list rows stands under the
top row at full width, in place of the box of what is loaded, and
scrolls within itself by the same rules; no place tiles follow it.

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

While something is typed the results alone fill the page under the bar
(on a window with the search in the top bar, in a tile under This session's
card or at its right, as above).
Only what is under the search bar scrolls, until the window is too short
to leave room under the bar for about three rows of results; then the
whole page scrolls (§5). The rail's windows draw the search bar first and
the lead, the first actions always and the
box of what is going on, wallets and keys, the rail beside it carrying Lock and Power off.

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
actions at a card's end), Lists (the rows of wallets and keys, and the box of what is loaded), Buttons (one
filled button on a page; the outlined Sign), Search (the search bar),
Floating action button (Scan), Navigation bar, Navigation rail and
Navigation drawer (one order of destinations across them, at most five on
the bar), and the canonical Feed layout (the wide Home's grid).

- The box scrolls within itself while the page stays still, as a list
  pane does; a page that does not scroll differs from Material's feed
  layout, where the page scrolls (owner, 2026-11-21, F313).
- No other difference from Material is added. The lead on the wide layout is the card's first item, filled, with a
  filled button; a wallet that needs something is
  a two-line list item.
- The Pi's Home as the menu of destinations is the one-column short
  window's rule (`docs/LAYOUT.md` §3).
- Check's tile where the navigation does not carry it, the bar's windows,
  is not a difference from Material (F275).

## 11. Tests

What a person would notice: with nothing loaded the three first actions
show as headings only, each pressed as a whole, and a job waiting leads;
with a wallet loaded the three first actions still stand and each opens
its flow; a wallet with a PSBT to sign, a spend under way or a backup waiting is a
row that opens its card, with no Sign, and one with nothing to do has no
row; Wallets carries a badge with how many need something, and the box has no All wallets row; Backups is a row only while one is not backed up; All keys opens Explore; a vault open is
no chip; a key no wallet uses is a row offering a wallet from it, with no
lead, and one loaded with a passphrase says so; the wide Home's search bar stays where it was while a search is typed,
the typed search takes the page and clearing it brings the rest back; with many keys and a short window no key row shows and All keys counts them, with room at most three show under All keys, and the box scrolls within itself while the first actions keep their place; and
on the phone-size window a search typed takes the page; a Home taller than
its window keeps the bar still or scrolls to its places; Scan is the sidebar's and the rail's FAB and floats on the Pi's menu and the phone-size window,
Backups once a wallet is known and the lead for each of the jobs of §2;
the Pi's menu puts Wallets left of Check and the phone keeps Check first.
The destinations' order is a design rule, held by review and the renders,
not by a test.
