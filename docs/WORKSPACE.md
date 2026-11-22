# This session as Faraday's hub: one pane of what is loaded, everywhere

**Kind:** Spec
**Status:** Not built: the design of Home's "This session" pane as the one view of what is loaded and going on, reused on the menu pages, in a flow's key step and on Files, with Check's and Tools' section cards, side-by-side panes and slower motion.
**Open:** Everything: nothing here is built. The take-away (§15) is part of it: its container, the arrival emphasis and the one written-and-checked state shared with Files. The take-away stands in the session pane at a flow's end (the owner chose it in the mock, 2026-11-22, §15). The session pane and its forms, Your choices moved to the left as the pane's side in every flow, the pane's place on each page and its setting, the section cards of Check and Tools, the layout container that places the pane and moves it, the panes beside the pane, the Motion and Speed settings with the motion they set, and the timings on the Pi that §6 and §8 name are all still to make.

"This session", Home's raised card, becomes the session pane, Faraday's hub: everything about
what is loaded and going on lives in it, and it works with every part of
Faraday the same way, at every size. Material 3 has no "persistent context"
component; the pane is built from Material's parts (a card holding a list,
single and multiple selection rows, a collapsed summary row) and differs
from Material only where a decision says so (F316, F317, F318, F319, F320).

This Spec builds on two pieces of Home not yet on main, which land with
the build: F314 (Home's top row: the 48 dp search bar and three tonal
buttons, the pane at the row's width) and F315 (the pane in two columns,
the lead inside it, raised at level 3). F313 (Home's page stays still, only
its pane scrolls) is on its way to main as well. Where this Spec says "the
pane on Home", it means the pane as F315 draws it.

## 1. What the pane holds, and in which order

One source for every form: the session (`app.session.keys`,
`app.session.wallets`), the flows in progress (`in_progress.rs`), the files
Faraday holds (`app.inbox`, taken in from a stick or a scan, and
`app.outbox`, held to write: Files' In Faraday) and the vaults open. Every
form draws from it through one component, the session pane (§14); no page
keeps its own list of what is loaded.

The rows, in order (HOME.md §3's order, kept):

1. **The lead** (HOME.md §2), the featured first item, across the pane.
2. **Flows in progress**: name, open step; resumes the flow.
3. **Files to write**: their count; opens Files.
4. **Wallets that need something**, ranked as today, with what they need.
5. **Backups**, while something is not backed up.
6. **Keys no wallet uses** (fingerprint, name, "passphrase", "No wallet uses it", Make a wallet).
7. **All keys** with its count, the "more" of the key rows.

A form shows a subset of these rows (§2); the order of Home's never
changes. A menu page's pane is named by its **mode**: the kinds of the
user's own data its tools take (keys, wallets, files, flows), in the order
that page's tools need them (§2's tables). The source is the same; the
mode only says which kinds are drawn and which leads.

## 2. The pane's forms

Each form is a mode of the one session pane (§14), not a component of its
own.

- **Full (Home).** As F315: every row of §1, two columns from 840 dp of
  pane width, Title L heading, level 3.
- **Compact (a menu page's top or left).** The same component on Tools
  and Check, the two menu pages that show it (§4). Its mode holds, on each
  page, the user's own data that page's tools take, whatever its kind, and
  leads with what those tools need most (owner, 2026-11-21; the tables
  below). The lead, if shown, is a plain row (no filled button: the page is
  a launcher). Above the page it is a short strip (§4) with All keys
  standing for the rest; beside the page it shows the whole list, scrolling
  within itself (owner, 2026-11-21). Heading "This session" in Title M.

  What each page's pane holds (the code's tools, read in
  `screens/catalog.rs`, `catalog.rs` and `check/screen.rs`; a kind with
  none of its data held is not drawn):

  **Check** (eleven tools). Seven take a file or a scan: Decode a
  transaction (a PSBT or a transaction file), Check a signer (two of them),
  Nonce check (a signed PSBT or transaction), Check a multisig setup (a
  file that reads as a wallet of several keys), Check an address (a text
  file, or a scan), Check a signed message (a message file), Check a backup
  in another form (shares, parts, a SeedQR, codex32, a Krux code). Three
  take a key with a seed: My seed makes my wallet (also a wallet or key
  file), Check a backup, Child seeds. Two take a wallet: Check an address
  and Check a multisig setup.

  | Order | Kind | Rows | Why |
  |---|---|---|---|
  | 1, leads | Files | the files Faraday holds that a Check tool takes, by kind: PSBTs and transactions, wallet and setup files, message files, shares and words | most Check tools start from a file; the person needs to see what is in Faraday to check it |
  | 2 | Wallets | each loaded wallet, with what it needs | Check an address searches them; Check a multisig setup compares against one |
  | 3 | Keys | keys that hold a seed, with All keys | only three tools take one, and they take one with a seed |
  | 4 | Flows | flows in progress | a check begun and left resumes |

  **Tools** (every flow Faraday has, nine groups). The kinds its tools take,
  and how many tools take each: keys, about fourteen (Keys and seeds'
  Explore, Child seeds, Convert, Krux code, Vanity, Silent payments,
  Lightning, SLIP-39 and Seed XOR parts; the cosigners of every wallet kind
  under Wallets; Sweep); wallets, about eight (Sign a transaction, Back up
  a wallet, Sign a message, Spend from backup, Check a multisig setup,
  Check an address, the KEF tool, Save to a vault); files, about sixteen
  (Sign and Decode a transaction, Check a signer, Nonce check, Check a
  signed message, Check a timestamp, Restore a wallet, Read a Bitcoin Core
  wallet, the shares and words of Add a key, a vault's locked file and its
  entries).

  | Order | Kind | Rows | Why |
  |---|---|---|---|
  | 1, leads while one waits | Files | the files a tool takes, by kind, and Home's lead when a PSBT waits (a plain row) | a file waiting is the one thing a tool is ready for: Sign a transaction (the first tool of Common) opens on it; with none waiting this kind has no rows and the page leads with Keys |
  | 1 otherwise | Keys | every key, "No wallet uses it" with Make a wallet, All keys | the most tools take a key (Keys and seeds, Wallets' cosigners) |
  | 2 | Wallets | each loaded wallet, with what it needs | Sign, Back up, Sign a message and Spend from backup start from one |
  | 3 | Flows | flows in progress | resumes a tool left |
  | not shown | Backups, vaults open | | Backup is a tool's own flow; vaults list on Vaults |

  A page's mode is a row of its own in the menu page container (§14): a
  new menu page names the kinds its tools take and gets the order the same
  way. The Pi's menu and the phone-size window take the same mode in one
  column above the page (§8).
- **Picker (a flow's key step).** When a tool is picked from a page the
  pane stands on, the flow's first screen opens with the pane still where
  it was, beside or above, and the key is picked in it there (owner,
  2026-11-21); the same from every page the pane appears on. The same
  rows, now selectable: a key
  step that takes one key shows each loaded key as a single-choice row
  (Material's list item with a radio button); a step that takes several
  (a multisig's cosigner slots, a seed set, Vanity's keys) shows checkbox
  rows. The step's other ways in ("Add a key", "Load a wallet") are the
  pane's last row, a text button, and return here with the new key ticked.
  A row the step cannot take (a key already in another slot, a key with
  no seed for a tool that needs one) stays, dimmed, with its reason as its
  line ("In slot 2", "No seed held") (owner, 2026-11-21): a person looking
  for a key sees why it cannot be used. Wallet steps (Sign a
  message's wallet, the KEF tool's wallet) take wallet rows the same way.
- **From showing to choosing** (owner, 2026-11-21). When the flow's first
  screen opens beside or above the pane, the pane shows that its keys are
  now the flow's and one is to be picked, in one transition at the Speed
  setting (§9):
  (a) its heading changes from "This session" to the flow's question
  ("Choose a key", "Choose the keys", "Choose a wallet");
  (b) the rows of kinds the flow does not take fade out, and a key it
  cannot take dims with its reason;
  (c) the rows left become Material selection-list rows: a radio button at
  the start (a checkbox where the flow takes several), a tonal container
  behind each row, the chosen one in the secondary container, the state
  layer on hover and press;
  (d) the pane takes an outline in the primary colour, tying it to the
  flow;
  (e) the flow's own first screen reads "Choose a key" with nothing that
  competes with the pane: no other key control, only its Next (off until a
  pick) and the step's own way back.
  The owner chose the selection rows over the keys as pills (chips)
  (2026-11-21): Material keeps chips for short labels and filters, a key
  carries two lines (its fingerprint and name, and what uses it), and pills
  of varying widths are what the pane replaces (§10).
- **Your choices (a flow under way).** Once the pick is made the other
  rows fade out and the pane, where it stands at the left, becomes the
  flow's Your choices (F319, FLOW-CONTAINER §4): one pane, continuous from
  Home to the menu page to the flow. The pick is its first row, a label and
  its value as every row of Your choices is ("Key: Savings", "Keys: 3
  keys", "Wallet: Savings"), and each answer adds a row under it. A press on
  a row goes back to its step on F296's rules; the key row brings the picker
  back in the pane, and the pick folds to its row again after. Where a
  later step needs another key (a multisig's next slot) the picker opens in
  the pane there again. The pane's heading becomes "Your choices". A flow
  not begun beside the pane (Make a new vault from Wallets start, a flow
  opened from Home's lead or a search result) has the same pane at the
  left in this form from its first step. The wallet chip at the flow card's
  head (FLOW-CONTAINER §2) goes: the wallet is a row.
- **Files' In Faraday.** Files' left pane (`screens/visit.rs`, `in_faraday`)
  takes the pane's look and component, listing the files held to write
  with their checks and Write at its foot, as today. Its heading stays "In
  Faraday" (owner, 2026-11-21): it names where the files are, against the
  stick's pane.

## 3. Its look per form

- **Home:** level 3 (`ELEVATION_FAB`), neutral surface-container-low, Title
  L (F315).
- **Off Home (compact, picker, Your choices):** the same family (fill,
  corners, row design), the heading a step smaller, Title M, and lower: level 1
  (owner, 2026-11-21, over level 3 and no shadow): on these pages the pane supports the page
  rather than leads it. In a flow, Your choices is this pane, in place of
  F296's flat pane with a divider at its start (F319).
- **Files:** as off Home; no shadow (it is one of two panes of equal rank).

## 4. Where the pane stands on each page

The pane's place on a menu page is a setting (§9): Automatic (the
default), Above the page or Left of the page. Automatic decides when a page
opens, never while the person is on it: loading a key does not move the
pane under them; the next page opened uses the new place, reached by the
glide (§6). Automatic puts the pane above while the rows it would hold fit
in a short strip (at most `HUB_ABOVE_ROWS` rows, a new token at 3 (owner,
2026-11-21): two files and All keys, or a flow, a wallet and All keys; the
lead counts as two), and to the left once it holds more. The threshold
counts the pane's rows of whatever kinds the page's mode shows (§2), not
keys alone.

On Tools and Check, Automatic keeps the pane at the left whenever it holds
more than a short strip, and a person who wants it Above sets that through
the Place setting (owner, 2026-11-22, on the mock).

The place never depends on motion (owner, 2026-11-21): with motion Off the
pane stands where it would after the glide, and only the moving goes.

**Left fits** where the room beside the navigation holds the pane
(`LIST_PANE`, 360 dp), a `SPACER`, the page's least content width and the
margins (`MARGIN_WIDE` each side): on a menu page the least content is one
section card (`CATALOG_TILE_MIN`). In a flow the pane is Your choices
(F319), so a flow needs the pane and its own column only: the column
(`COLUMN_MAX`, narrowing to `FLOW_CARD_MIN`) and the pane (narrowing from
`LIST_PANE` to `CHOICES_PANE_MIN`, after the column), on the wide layout
(FLOW-CONTAINER §4), at its narrowest about 872 dp of room. Left is
therefore used only on the wide layout (the expanded rail: Large and over,
not low; LAYOUT §2), where the room beside the 240 dp rail is at least
960 dp: there a flow keeps the pane at the left wherever the page it was
opened from had it, and the pane never moves under the person between the
page and the flow. Where Left does not fit, Above is used whatever the
setting; 1366 × 768 is Large and not low, so it takes Left. Guide's pane stays at the right of a flow on
extra-large windows (F319), counted in the room as FLOW-CONTAINER §4 counts
it today; the unit at its narrowest is as wide as before, so Guide's
threshold (1536 dp of window) does not change. On a menu page, Guide's
pane (LAYOUT §6, from 1600 dp) counts in the room when it is open or
pinned as the page opens; opened later with no room left, Guide is the
sheet.

**Settings and the pane** (owner, 2026-11-21). On Settings the pane stands
at the left, and Settings' cards stand in the area beside it (MENU-PAGE §6,
up to two columns). The cards start one gap, `HUB_PANE_GAP` (a new token,
24 dp, 3 × `GRID`), to the right of the pane's edge, not centred in the free
width (owner, 2026-11-22, on the mock; at 3840 × 2160 and wherever Settings
stands beside the pane). On a tall window Settings' cards also stand under the
pane, in the pane's own column:

- **The pane takes only the height it needs**, at most
  `HUB_CARD_HEIGHT_SHARE` of the page area's height, a ratio token at 0.5;
  beyond that its rows scroll inside it, the heading staying.
- **Room under it** is the height left under the pane, the page area's
  height less the pane's and `MENU_GAP`. Where that is at least
  `HUB_UNDER_MIN` (a token, 800 dp: more than any one of Settings' cards,
  which the build checks against them), the column under the pane is a third
  stack of Settings' cards, as wide as the pane, filled as MENU-PAGE §6
  divides cards, by height, a card whole or not at all; a card that does not
  fit the room stays in the area beside. Under 800 dp of room every card is
  in the area beside, as the pane's column ends with the pane.
- **1920 × 1080** (page area about 1016 dp): the pane at most 508 dp, so the
  room under it is under 800 dp unless the pane holds little (an empty
  session: its heading and one row); Settings stays beside the pane, two
  columns.
- **3840 × 2160** (about 2096 dp): the pane at most 1048 dp, the room under
  it at least 1032 dp; Settings' cards stand under the pane as well as in
  the area beside it.
- The layout depends on the window and what the pane holds only, never on
  motion (above), and is worked out as the page opens and as its size
  changes, as MENU-PAGE §6 works the stacks out.

Every page the pane appears on, with its place:

| Page | Place | Why |
|---|---|---|
| Home | centre, full form (not the setting) | the pane is Home's focus (F315) |
| Tools | Automatic (owner, 2026-11-21) | with few rows the pane is above; with many it is beside, the whole list shown |
| Check | Automatic (owner, 2026-11-21) | as Tools; it holds the files Faraday holds first (§2), so it is short until files are loaded |
| A flow's key step | where the pane stood on the page the tool was picked from (owner, 2026-11-21) | the key is picked in the pane the person already sees; a flow opened from Home, the sidebar or a search result has the pane at the left from its first step, the picker and then Your choices (owner, 2026-11-21) |
| A flow under way | at the left, as Your choices (F319) | one pane from Home to the flow; with the pane above, the strip glides to the left at the pick (§6) |
| Files | Left always, as In Faraday (owner, 2026-11-21) | it is already a pane beside the stick's pane |
| Settings | beside the pane, and under it on a tall window (below) | Settings is rarely used with what is loaded; its cards use the room the pane leaves |
| Wallets start, Wallets, Vaults, Backups, Explore | not shown (owner, 2026-11-21) | each lists the loaded wallets, keys or vaults itself; a second list beside it repeats it |
| The Pi's menu, the phone-size window | Above, one column; in a flow, Your choices on the bar's button and sheet (F296 point 4, kept) | no room beside (§8) |

## 5. Check and Tools as section cards

Each section of a menu page (Common, Keys and seeds, The wallet, Signing,
...) becomes one card holding a Material list: rows with an icon, the
title, the line under it and a chevron; hover and press are Material's
state layer across the whole row, not a pill; thin inset dividers between
rows (owner, 2026-11-21). This replaces the grid of one outlined card
per choice (MENU-PAGE §4). The section's label is the pane's heading inside
it, Title S.

The section cards are flat: a filled tint (surface-container), no shadow,
Material's filled card. Home's session pane is the only raised surface, so it stays
the focus. On wide pages the section cards stand two or three to a row,
each as tall as its own list, set as Settings' cards are (MENU-PAGE §6,
columns balanced by height); one column on a page under two
`CATALOG_TILE_MIN`. The phone-size window and the Pi's menu take one
column of the same cards (owner, 2026-11-21: if the mock's renders read
well; a render that reads worse than today's tiles goes back to the owner).
The section card is part of the
menu page container (§14): a page declares its sections (MENU-PAGE §1),
and Tools, Check, Wallets start's ways in and the Pi's places all follow.

Material's lists are a single list in a page; a list inside a card is
Material's own pattern for grouping (Cards and Lists guidelines). The
difference is several such cards side by side: F317 records it.

## 6. Motion

Every motion follows two settings (§9): the **Motion** style (Smooth
motion, Crossfade, Off) and the **Speed** (Quick, Medium, Relaxed; 600 ms
by default). Nothing here changes on its own (owner, 2026-11-21): no
machine steps down from one style to another while it runs, and no
transition picks its own style. A device's default is set ahead of time
from a measurement (§8); after that only the person changes it, in
Settings. The layout never depends on motion (§4): Off puts every screen
where the motion would have ended, at once.

What each style does (Material's Easing and duration and Transitions
pages; all on Material's emphasized curve, `EASE_EMPHASIZED`, at the
Speed's duration unless the row says otherwise):

| What changes | Smooth motion | Crossfade | Off |
|---|---|---|---|
| Home to a menu page, a page to another, into a flow | the pane glides (below); the rest fades through | the whole frame blends from the old screen to the new (today's fade of two kept frames) | at once |
| A step inside a flow | fades through (FLOW-CONTAINER §3) | blends | at once |
| A row arriving in the pane (a PSBT found, a stick plugged in, a key loaded) | fades in while its space opens | blends | at once |
| A row leaving (a flow finished) | fades out; the rows under it slide up | blends | at once |
| Expanding in place (All keys, More kinds in Add a key, any rows opening) | its height changes smoothly, the rows under it moving with it | blends | at once |
| The pane from showing to choosing (§2) | in one transition, the outline and containers fading in | blends | at once |
| A key added in a signing flow | its row arrives (as above), its tonal emphasis eases away, Sign's enabling fades in at the same moment | blends, then the emphasis eases away | at once, no emphasis |
| Hover | the state layer fades in over 150 ms (`MOTION_SHORT3`) and out over 200 ms (`MOTION_SHORT4`), not scaled by Speed | as Smooth motion | at once |

- **The glide.** From Home to a menu page the pane moves from Home's
  centre to its place on the new page, resizing as it goes (Material's
  container transform, a shared element): its bounds and corner radius
  ease from one to the other, its rows cross-fading inside it. Back to
  Home it moves back. From a menu page into a flow's first screen it stays
  where it was; at the pick it becomes Your choices in place (§2), with the
  pane above the strip glides to the left and becomes Your choices (owner,
  2026-11-21: one place for Your choices in every flow).
- **Fade through** for the rest of the page: the old page goes out over
  the first 35% of the eased progress, the new one comes in over the rest,
  as the step fade does today (MOTION.md §3.5).
- **One duration** for every page change and every step inside a flow
  (owner, 2026-11-21): the Speed setting's, 600 ms by default, a
  deliberate difference from Material's 450 ms fade through, approved by
  the owner (F318), against
  today's 200 ms linear crossfade (`motion::FADE_MS`) and 300 ms step fade
  (`motion::STEP_FADE_MS`). A step no longer takes a shorter share of it.
- **"Here is your key."** A key added inside a signing flow (scanned, once
  the scanner's sheet has gone) arrives as its row in Your choices or the
  pane with the arriving row's motion; its tonal emphasis (the added
  line's state layer, `PRESS_EMPHASIS`) eases away over the Speed's
  duration, and Sign's button turns on, fading in, at the same moment, so
  the person sees which key made Sign possible (owner, 2026-11-21, the
  recommendation taken). The mock-up shows it.
- **Hover** is the only motion of hover and press (owner, 2026-11-21): no
  press ripple, no card lift on hover, nothing that moves.
- **Secrets.** A frame drawn while a secret is typed or shown keeps no
  image to fade from (SECRET-LEAKS §5): such a change is at once, and a
  flow step that shows or takes a secret fades drawn live, as now. At 600 ms
  the step's fade out, after which its secret state is released, lasts
  about twice today's 48 ms (about 96 ms against 48). The owner said yes
  to the animation in principle (2026-11-21): the window is that short, and
  reading the secret in it needs code or memory access on the device, both
  outside the security model. It is tested, and reaches main only on the
  owner's yes after that testing (F318 point 7).
- **Bottom sheets** (the Pi and small windows), at once today, slide up
  (owner, 2026-11-21): a sheet rises from the window's foot at the Speed
  under Smooth motion, blends under Crossfade, at once under Off, if the Pi
  keeps up, which is timed with the first test build (§8).
- **Toasts, the chart's side sheet and the theme fade** follow the Motion
  style (they stop at Off, blend under Crossfade) and keep their own
  durations, 200, 400 and 350 ms, not the Speed's (owner, 2026-11-21):
  they are not page changes, and a toast at 600 ms reads as late.
- **What drops motion today.** Nothing, on a slow machine: the gate that
  turned fades off when a fade's steps came too far apart was deleted on
  2026-11-16 (SPEED §12 point 4, MOTION §3.5); `Faraday::fades` reads
  Reduce motion alone, and the gaps between a fade's steps are timed and
  reported, never acted on (`fade.rs`). That stays so. Two things go with
  the build: the doc comment on `Faraday::fades`, which still describes the
  deleted gate, and Reduce motion itself, which becomes Off (§9). The one
  default chosen by the screen is the HDMI screen's own, set ahead of time
  from measurements (§8), replacing F222's Reduce motion on at first start.

## 7. Side-by-side panes with the session pane

Side by side only, never stacked: each pane scrolls within itself and the
window never scrolls as a whole. A pane is worth its room only where a
person uses both sides together; ranked, the strongest first:

1. **A row of the pane and its details.** A key in the pane opens its page
   in Explore beside the pane, a wallet its card from Wallets, a file its
   view: the person goes down the pane and reads each, the pane held
   still (Material's list and detail; LAYOUT §5). Extra-large windows;
   below, the row opens its page as today.
2. **The pane as a flow's picker and Your choices** (§2): the pick in the
   pane, the step beside it, the answers gathering in the same pane. This
   is the reuse itself, and makes most other pairings unneeded.
3. **Signing** (§11's survey): the facts a person compares stay in Your
   choices, now the pane at the left; the change address and the signers
   join it there (no new pane). Sign a transaction's "This transaction"
   (`review_under`) stays under its rows, so it moves to the left with it.
4. **Files** (In Faraday beside the stick): already two panes.
5. **Settings beside and under the pane** (§4): weak, so Settings' cards use only the room the pane leaves.
6. **Tools or Check beside the pane** as a launcher beside a list: weak,
   since a tool starts a full-window flow; covered by the pane's place
   (§4), not a pane.

The backup board's list, a full-width step's own pane (FLOW-CONTAINER §5),
stays at the right: it is the board's, not Your choices, and Your choices
is the bar's sheet on that step as today.

## 8. The phone-size window and the Pi

The same rows in one column, Above always, the same model of moving: the
picker is the same rows with radio or checkbox, and in a flow Your choices
keeps the top app bar's button and its sheet (F296 point 4, kept: F319).
Home's one-column box (HOME.md §4) stays as it is (F315), drawn from the
same source; on Tools and Check the pane is the compact form above the
page.

The Pi's default Motion is set from measurements, made before the glide
is built there (SPEED §7: a late-flow frame timed with the harness of
SPEED §4, on the Pi 3B+): Smooth motion if each step of the glide is drawn
within the screen's frame, else Crossfade if each blend of two kept frames
is, else Off. Its default Speed is set the same way, and may be below
600 ms if the measurement says so. Both are defaults written once; the Pi
never changes them while it runs. Desktops default to Smooth motion at
600 ms.

The Pi's panel and its HDMI screen each have a default of their own, chosen
ahead of time from the measurement at their size, never a step down while
running (owner, 2026-11-21; F222's "Reduce motion on at first start on an
HDMI screen" goes, replaced by this). The HDMI screen is measured at
1920 × 1080 and at 1280 × 800 (WXGA, which the owner shows centred on a
larger screen); its default comes from the two. The owner measures all the
Pi's timings with the first test build, the panel's, both HDMI sizes and
the bottom sheets' slide (§6), in one sitting.

## 9. Settings

On Settings › Appearance (owner, 2026-11-21):

- **The pane's place**: a segmented button, "Automatic", "Above the page",
  "Left of the page"; Automatic by default (§4). Where Left does not fit
  the window, the row shows the value in force beside it, "Above on this
  window". Kept with the other settings (`pane=auto|above|left`).
- **Motion**: a segmented button, "Smooth motion", "Crossfade", "Off";
  Smooth motion by default on desktops, the Pi's from its measurement (§8).
  The row's line says what the chosen one does, as a fact: "The pane moves
  to its place; pages fade", "Screens blend into each other", "Screens
  change at once". **Off is today's Reduce motion** (MOTION.md §3.5,
  `stick/settings.rs`'s `reduce_motion`; owner, 2026-11-21: one setting,
  never two): it replaces that switch and stops everything it stops
  (fades, the glide, a scroll's coast and stretch, the dice tumble, the
  theme fade, a toast's rise, the hover fade). The settings file's
  `motion=reduced` reads as `off` and `motion=full` as `smooth`.
- **Speed**: a segmented button, "Quick" (250 ms, Material's medium 1),
  "Medium" (400 ms, medium 4), "Relaxed" (600 ms, long 4); Relaxed by
  default (owner, 2026-11-21: 600 ms). It applies to Smooth motion and
  Crossfade; with Off the row is shown off. Every page change and every
  step inside a flow takes it (`speed=quick|medium|relaxed`).

Two settings, not one: a style and a speed together are seven values (Off,
and three speeds of each style), over the two to five segments Material
gives a segmented button, and a person who wants another speed keeps the
style. "Standard" goes as a name, since it is no longer the default;
"Relaxed" keeps the name the owner gave 600 ms.

Strings for `faraday-strings` (added with the build): `settings_card_place`
"Session pane", `settings_card_auto` "Automatic", `settings_card_above`
"Above the page", `settings_card_left` "Left of the page",
`settings_card_above_here` "Above on this window"; `settings_motion`
"Motion", `settings_motion_smooth` "Smooth motion",
`settings_motion_crossfade` "Crossfade", `settings_motion_off` "Off",
`settings_motion_smooth_line` "The pane moves to its place; pages fade",
`settings_motion_crossfade_line` "Screens blend into each other",
`settings_motion_off_line` "Screens change at once"; `settings_speed`
"Speed", `settings_speed_quick` "Quick", `settings_speed_medium` "Medium",
`settings_speed_relaxed` "Relaxed"; `hub_choose_key` "Choose a key",
`hub_choose_keys` "Choose the keys", `hub_choose_wallet` "Choose a wallet",
`hub_no_seed` "No seed held", `hub_in_slot` "In slot {n}". Your choices'
rows take each flow's own step labels, as today.
`settings_reduce_motion` goes.

## 10. Survey: every place a loaded key, wallet or file is listed or picked

Today a key step draws the loaded keys as a wrapped row of buttons of
varying widths, `faraday_ui::parts::button_rows` with
`crate::wallet::key_line` labels, the chosen one in the selected style (the
"pill row"). Each place reads `app.session` directly; none shares a
component with Home's pane.

| Place (file) | Shows | Picks | Becomes |
|---|---|---|---|
| Home's pane (`screens/home.rs`, `key_rows`, `wallet_rows`, `going_rows`) | flows, files, wallets, Backups, keys, All keys | opens | the full form |
| Home's one-column box (`compact/mod.rs`, `nav/screen.rs` lines) | vaults open, flows, keys, wallets, files | opens | unchanged (F315) |
| Child seeds and passwords, key step (`bip85/screen.rs`, `pane`) | loaded keys, Add a key | one, pill row | picker, single |
| Explore and Derive (`explore/screen.rs`) | loaded keys | one, pill row | picker, single; Explore itself is All keys |
| Convert a seed (`convert/screen.rs`) | loaded keys, an aezeed | one, pill row | picker, single |
| Silent payments, key step (`silent/screen.rs`) | loaded keys | one, pill row | picker, single | Make another record |
| Lightning node key (`lightning/screen.rs`) | keys, aezeed, Core Lightning | one, pill row | picker, single (its other sources as rows) |
| Create's slots, a multisig's cosigners (`screens/create.rs`, slot rows) | keys not in another slot | one per slot, pill row | picker, one slot at a time, opening again per slot |
| A seed set (`seeds/screen.rs`) | "Use {key}" per key | several, buttons | picker, several (checkboxes) |
| Vanity, key step (`vanity/screen.rs`) | keys, fingerprint and label | several, checkboxes | picker, several |
| KEF tool (`kef_tool/screen.rs`) | wallets, then keys | one, full-width buttons | picker, single (wallet rows, key rows) |
| Sign a message, wallet (`screens/message.rs`, `MWallet`) | wallets that can sign a message | one, button row | picker, single wallet | Sign another message; Check a signed message |
| My seed makes my wallet, holders (`ways_in/mod.rs`, `Way::Holder`; `seed_check/screen.rs`) | loaded keys as ways in | one | picker, single |
| Check a signer, seed rows (`decode/check_screen.rs`, `seed_row`) | the keys here that signed | none (facts) | unchanged |
| Spend from backup (`spend_backup/screen.rs`) | each wallet as a card with readiness | one | picker, single wallet, readiness as the row's line |
| Back up's wallet menu (`screens/backup.rs`, `BWallets`) | wallets, a menu | one | stays a menu (it switches the board) |
| Save to a vault (`vault/screen/save.rs`) | keys and wallets, checkboxes | several | picker, several |
| Wallets, list and keys without (`screens/wallets.rs`) | wallets, loose keys | opens | unchanged; pane not shown (§4) |
| Wallets start (`screens/wallets_start.rs`, `loose_key_rows`) | keys without a wallet | opens | unchanged; pane not shown |
| Files, In Faraday (`screens/visit.rs`, `in_faraday`) | files held to write | several, checks | Files' form (§2) |
| Vaults (`vault/screen.rs`, `list`) | vault files | opens | unchanged; pane not shown |
| Scan's key sheet (`screens/sheets.rs`) | keys a QR names | none | unchanged |
| Sign a transaction (`screens/spend.rs`) | the wallet's keys here | none: `to_sign` is filled from the PSBT (`wallet.rs`) | unchanged; Your choices' first row names the wallet | Sign with another key loaded here (while signatures are missing); Sign another transaction (opens the scanner); Check the signed transaction |

So: about twenty places; twelve picks become the picker form (nine single,
three several), Files takes the pane's look, and the rest are lists that
stay (they are the full lists the pane's "All keys" opens).

## 11. Survey: the signing flows, where a person looks up or down to compare

Two panes already stand in every flow on the wide layout: the pane and
Your choices, which become one, the pane at the left (F319); Sign a
transaction's review also puts "This transaction" (six facts and the
signatures) under Your choices (`screens/spend.rs`, `review_under`).
Candidates, with the screen each is on:

1. **A change output against the wallet it belongs to** (Sign, step 2
   Transaction: the Change row and the change warnings, `review_rows`,
   `review_warnings`; the wallet's own change address is on step 1 Check,
   `step_addresses`, "Change 1/0"). Kept, the strongest: the person goes
   back a step to compare. The change address joins the held pane
   (`review_under`), beside the Change row.
2. **A multisig's cosigners against the wallet's keys** (Sign, steps 4
   Signers, 5 Sign and 6 Signatures for this transaction: `step_keys`, `step_sign`, `step_collect`).
   Kept, moderate: who signs is split over three steps. The pane holds the
   wallet's keys with each one's mark (signs here, signed, waiting) across
   those steps; a key added there gets §6's "here is your key". Check a
   multisig setup (`multisig_check/screen.rs`) has its marks on one card:
   dropped there.
3. **A long review whose summary scrolls away from the outputs** (Sign,
   step 2: rows, other transactions, warnings, then the table). Covered on
   the wide layout by `review_under`; on a narrow window the pinned foot
   (`spend_bar`) keeps the signature count and Continue. Kept only as is.
4. **Check a signer, sent against returned** (`decode/check_screen.rs`):
   the differences are on the result card. Dropped; the result names the
   two files instead.
5. Address against path, a registered wallet against its keys, inputs
   against outputs: on one card each. Dropped.

## 12. (Decided)

§12: every choice was decided by the owner on 2026-11-21; the decisions are
in the sections above.

§12: the flow's end is decided (owner, 2026-11-22, on the mock): the
take-away stands in the session pane, §15.

## 13. What must come first, and which files change

F313, F314 and F315 land first: the pane this Spec reuses is theirs. The
session pane comes out of `screens/home.rs` into a module of its own, `session_pane`, beside
the menu page container (`screens/menu_page.rs`), with its forms as modes;
`screens/home.rs` and `compact/mod.rs` call it. The menu page container
gains the section cards (`screens/catalog.rs`, `check/screen.rs`), and the
hub layout container (§14) the pane's place. Each key step of §10 swaps
its pill row for the picker (`bip85`, `explore`, `convert`, `silent`,
`lightning`, `screens/create.rs`, `seeds`, `vanity`, `kef_tool`,
`screens/message.rs`, `ways_in`, `spend_backup`, `vault/screen/save.rs`);
the flow container (`flow.rs`, `flow/unit.rs`) gives its Your choices rows
to the pane at the left and drops the wallet chip at the pane head. Files'
In Faraday (`screens/visit.rs`) takes the component. The take-away
(§15) is drawn by the flow container (`flow.rs`, `flow/unit.rs`), in the session pane,
fed by the outbox and the visit's receipt (`stick/visit_taken.rs`,
`receipt.rs`); the flows of §15's table end on it in place of their own
Finish, Show as QR and Save buttons, and Files' to-write rows (`screens/
visit.rs`) and the pane draw one row. Motion: `motion.rs`
and `fade.rs` (the glide, the row and height motion, the hover fade,
durations from the Speed setting, the stale comment on `Faraday::fades`),
`stick/settings.rs` (the pane's place, Motion and Speed, Reduce motion's
migration), Settings › Appearance, and `core/faraday-draw/src/tokens.rs`
(`HUB_ABOVE_ROWS`, and `MOTION_LONG4`, 600 ms, Material's long 4, which
the tokens do not have yet), and `HUB_CARD_HEIGHT_SHARE` and
`HUB_UNDER_MIN` (§4). The owner reviews the mock renders (Tools and Check
at 1366 × 768, 1920 × 1080 and 2560 × 1440, Settings at 1920 × 1080 and
3840 × 2160, a key step at 480 × 640) before any of it is built.
HOME.md, MENU-PAGE.md, MOTION.md, LAYOUT.md §5 and §6, FLOW-CONTAINER.md §3
and §4 and F296 follow in the change that builds each part.

## 14. The containers

The owner (2026-11-21): the hub is built as containers, as the flow and
menu page containers are, each drawn by one piece of code, held by
`lint-components`, and described in its own document, written in the
change that builds it. CLAUDE.md's "By area" names each in that change (as
it names FLOW-CONTAINER.md and MENU-PAGE.md); it is not changed before.

- **The session pane** (`session_pane`; its document to be named SESSION-PANE.md). One component, fed by
  §1's one source; its forms (Home, a menu page Above and Left, the key
  picker, Your choices, Files) are modes of it, and a page or flow names
  the mode, never draws rows of its own. It replaces Home's `key_rows`,
  `wallet_rows` and `going_rows` (`screens/home.rs`), every pill row and
  button list of §10's "picker" places, Files' `in_faraday`
  (`screens/visit.rs`), the Your choices pane of the flow container
  (`flow.rs`, `flow/unit.rs`: the container keeps declaring the rows, the
  pane draws them) and the wallet chip at a flow card's head.
  `lint-components` holds that a key step draws no `button_rows` of loaded
  keys and that no screen reads `app.session` to list keys or wallets.
- **The section card** (in `docs/MENU-PAGE.md`). Part of the menu page
  container: a page declares its sections, each with its rows, and the
  container draws each as a section card (§5). It replaces the tile grids
  of `screens/catalog.rs` and `check/screen.rs` and every page's own
  section labels. `lint-components` holds that a menu page draws no tile or
  section card itself.
- **The hub layout** (its document to be named HUB-LAYOUT.md). The page with the pane beside
  or above it: a page or flow declares itself into it, and it owns the
  pane's place (Automatic, Above, Left, §4), the glide between pages (§6)
  and the use of the Motion and Speed settings for page changes. It
  replaces each menu page's own placing of the pane, the flow container's
  centring of the unit and its pane (`flow/unit.rs`), and the screen fade's
  own choice of duration (`fade.rs`, `motion::FADE_MS`,
  `motion::STEP_FADE_MS`). `lint-components` holds that no page places the
  pane or times a page change itself.
- **The take-away** (its document to be named TAKE-AWAY.md). Part of
  the flow container: a flow declares what it produces (QRs, files) and the
  container draws it, in the place §15's choice names. It replaces each flow's own end
  buttons for a file or a QR (§15's table) and gives Files' In Faraday rows
  and the take-away one row drawer. `lint-components` holds that a flow draws no
  "Write to" or "Save" button of its own at its end.

## 15. The take-away

The owner (2026-11-21, F320): every flow that produces something ends with
a take-away, built once, holding what the person takes away: QRs to scan and
files of any kind. This section is the design; F320 holds the calls and the
alternatives passed over (a press on a file queueing it; written
automatically).

**What it is.** One container, at the flow's end in the session pane (below). A plain heading ("Take with you"), then the QR on a light
square with its quiet zone (in Dark too), then the files as Files' In
Faraday rows (leading checkbox, name, kind, size), ticked, and at the foot
one filled "Write to stick": the one high-emphasis action. Every flow that
produces something uses it. Another and Done stay where the flows have them
(FLOW-CONTAINER.md §7). On arrival it carries an emphasis from its place
that eases away, as the key moment does, at the Speed setting (at once
under Off). "Here is your key" (§2) is unchanged.

**The flow's end: in the session pane** (owner, 2026-11-22, on the mock).
At the last step the session pane at the left turns, with the glide (§6), from Your choices into the
  take-away plus next steps: what follows from the result only, never Tools,
  Files or Home, which the sidebar has. The step's card states what
  happened ("Signed: 2 of 3 signatures", the amounts, what is still
  needed) and holds no files or QR. After writing, or on leaving, the pane
  settles back to its session form, the written files green there and in
  Files. On the phone-size window and the Pi the flow's end opens the pane's
  sheet (F319 point 4) showing the take-away and the next steps.

**What this changes.** F319 puts Your choices at the left in
every flow and F296 makes the last step a card in the column: Your
choices stays at the left in every step but the last, which the session pane
takes over (the answers given are not lost: the pane's session form lists
them as the session's rows, and Back returns the pane to Your choices); the
last step is a card of the outcome in the column.

**Elevation and Material.** Level 1 (`ELEVATION_SHEET`), as the session
pane off Home (F316 point 2), not level 3 (the FAB, the toast, the notice).
Checked against Material 3's Elevation, Cards, Lists, Buttons and Color
pages. The one thing outside the scheme is the green: `Role::Ok` is
Faraday's custom role (`docs/THEMES.md` §5), as F306 uses it.

**Writing.** Writing is a press on "Write to stick", never automatic. Each
file is ticked on its own; Write writes the ticked rows through the visit's write, which reads each file
back and compares it (`storage_api.rs`, "Write"; the visit's receipt
records `verified`). Today a flow ends with its own button (Sign's "Write
to the {}" and "Lock and write", Back up's "Done", Wallet export's "As a
file" and "Show wallet QR", Message's "Make the file" and "Show as QR");
those go, the take-away being the one place. The ticked set is the visit's
chosen set (`VisitState.out`); the take-away and Files' to write read and
change the same set. An unticked file is not written: it stays in Files' In
Faraday, held (F320 points 4 and 7).

**Written and checked.** After a write is read back and compared, the
rows in the take-away and the same rows in Files show F306's green: `Role::Ok`
at 12% over the surface as a state layer (`VISIT_SAME_LAYER`), a check in
`Role::Ok` at the row's end. Cobalt Light Standard `Ok` is #006D43 and
Cobalt Dark Standard #76DAA1; F306 already holds their contrast, and the
take-away adds no colour. The state is one: a file's `verified` entry in the
visit's receipt (the same name, the same bytes), which Files' rows already
read. The take-away adds no state of its own (today's `written_and_checked`,
which asks that the whole outbox is empty, becomes a per-file read of the
receipt). No error colours: a file not written reads "Waiting for a
stick" (Files' wording) or its usual failure line in Files' text colours.

**No stick.** The rows stay ticked and read "Waiting for a stick"; the
button returns when a stick is in; meanwhile the files are in Files' to
write (the outbox, `app.outbox`), which is where they already go when a
flow makes them. No file moves: the take-away is a second view of the outbox's
files, never a copy, so nothing is built twice and nothing is lost when the
take-away is left.

**Which flows end with it.** The survey of today's code (screens under
`faraday/faraday-core/src/`):

| Flow | What it produces | Its last step today | The take-away holds | Next steps (from the result) |
|---|---|---|---|---|
| Sign a transaction (`screens/spend.rs`, `spend_backup/screen.rs`) | signed PSBT, finished transaction, a QR | Finish; "Write to the {}", "Lock and write to the {}"; "Show the finished transaction as QR" | the QR, the PSBT and transaction files ticked | Sign with another key loaded here (while signatures are missing); Sign another transaction (opens the scanner); Check the signed transaction (opens Decode a transaction on the signed file) |
| Seed sweep (`seeds/screen.rs`) | `sweep-signed.psbt`, `sweep-final.txn` | the files go to the outbox | both files ticked | Sweep another seed; Check the signed transaction (Decode a transaction, with `sweep-signed.psbt`) |
| A vault saved or changed (`vault/screen`, `vault/seal.rs`) | the sealed vault file; its key words (secret) | the visit's "Write to {}", "Lock, and the visit writes them" | the vault file ticked; key words only under the secrets rule below | none |
| Backups (`screens/backup/quick.rs`, `backups/screen.rs`, `backup_flow/screen.rs`) | seed QR, codex32 strings, PDF, text and picture files | Done, Back up now, Show the SeedQR | the QR, the files ticked | Check a backup in another form (opens its choice of forms; the copy is scanned or typed back, nothing is preloaded); Back up another key |
| Wallet export (`glance/sheet.rs`) | wallet, Electrum, Wasabi and xpub files; wallet and xpub QR | As a file, Xpub file, Show wallet QR, Write it out again | the QR, the files ticked | Export another wallet |
| GPG keys (`gpg.rs`, `backup_flow/gpg_restore.rs`) | certificate, revocation | Make the revocation, Export the kept revocation | the files ticked | Make the revocation (when only the certificate was made) |
| Secure Boot (`secure_boot/mod.rs`) | the signed EFI image | the image goes to the outbox | the image ticked | none |
| KeePass export (`bip85/mod.rs`, `vault/stretch.rs`) | a KDBX file, its key file (secret) | Save as a file | the KDBX file ticked; key file as a secret | Check the export (`vault_export_check`, opened with the KDBX file just written) |
| Sign a message (`screens/message.rs`) | a message file, a QR | Make the file, Show as QR | the QR, the file ticked | Sign another message; Check a signed message (opens with the message file) |
| Derive with BIP-85 (`bip85/screen.rs`) | SeedQR, a words file (secret) | Save as a file, Save into {}, Show the code, Done | secrets rule below | Derive another child (Child seeds and passwords again) |
| Silent payments (`silent/screen.rs`) | a text record, QR | Record as QR, DNS record as QR, Save into {} | the QR, the file ticked | Make another record |
| Krux code (`kef_tool/screen.rs`) | PNG and file pair | its save | the pair ticked | Make another code |
| Chart PDF (`chart_edit.rs`) | a PDF | into the outbox | the PDF ticked | Make another chart |

A next step opens a tool Faraday has, on the result (the same flow again, or a Check or Decode tool with the result loaded); a step with no such tool is dropped, and a flow left with none says "none", so its session pane shows only the take-away.

A flow that produces nothing (Check a signer's result, Convert's shown
result, an address check) has no take-away and ends as FLOW-CONTAINER.md §7 says.
A QR needs no stick: a take-away with a QR and no file has no button.

**Where it meets what exists.**

- *F296 and FLOW-CONTAINER.md §7.* The last step stays the flow's: the step's card, its place, the fade
  through, "Step N of M" and F104's limits on Back. It gives up the flow's
  own end buttons for a file or a QR: it states the outcome, and the take-away
  is in the session pane. F296 point 2's release of secret state
  at the fade's end holds for it.
- *Files' to write (`screens/visit.rs`).* Keeps the outbox, the ticked set
  and the rows; gains the same green row after a write, already F306's.
  Gives up nothing; the row drawer becomes shared (§14).
- *The read-back check.* The visit's write reads each file back and
  compares it, and its receipt keeps the hash and `verified`. The take-away
  reuses it and adds no second check.

**Secrets.** A take-away that holds a secret (a seed backup's QR, a vault's
key words, a KDBX key file, a child seed) keeps F296 point 2 and today's
code: the QR is drawn live from the state the flow holds, so the take-away keeps
no copy, no kept frame (MOTION.md §3.5) and no row text of it; the state is
released when the step's fade out ends and at Lock or idle
(when the pane settles back to its session form, and at Lock or
idle; `secrets_off_screen`: the QR view is closed and each area's `AtLock::lock`
runs). The QR is drawn live in the session pane from the flow's state, never from the pane's own rows, and is released when the pane settles back. A secret file is in the outbox only through `outbox_secret`, after
the person is asked (`secrets.rs`), is dropped at Lock as today
(`inbox.rs`), and its row is ticked only with that asking done: until then
it is not in the take-away's files, only named in a line that says it is asked
for. So the take-away holds a secret on screen and in state no longer than
today's last step does. The receipt keeps hashes only and survives a lock
as it does now. The take-away adds a state of no secret type; a test shows that
leaving the step releases a QR's secret.

**The phone-size window and the Pi.** Where the window is one column, the flow's end opens the pane's sheet (F319
point 4, the top app bar's button's sheet) showing the take-away and the next steps. Your choices keeps the top app bar's button and its
sheet at every other step. The Pi's panel is the same. Its arrival emphasis
follows the Speed setting measured for the Pi (§8).

**Open.** All of it: nothing here is built (the Open line at the top). The owner approved the mock on 2026-11-22.
