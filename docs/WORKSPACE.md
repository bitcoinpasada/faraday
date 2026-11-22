# This session as Faraday's hub: one card of what is loaded, everywhere

**Kind:** Spec
**Status:** Not built: the design of Home's "This session" card as the one view of what is loaded and going on, reused on the menu pages, in a flow's key step and on Files, with Check's and Tools' section cards, side-by-side panes and slower motion.
**Open:** Everything: nothing here is built. The card's forms, its place on each page and its setting, the section cards of Check and Tools, the panes beside the card, the glide and fade through, the Motion setting, and the mock renders the owner picks from (§12) are all still to make.

"This session", Home's raised card, becomes Faraday's hub: everything about
what is loaded and going on lives in it, and it works with every part of
Faraday the same way, at every size. Material 3 has no "persistent context"
component; the card is built from Material's parts (a card holding a list,
single and multiple selection rows, a collapsed summary row) and differs
from Material only where a decision says so (F316, F317, F318).

This Spec builds on two pieces of Home not yet on main, which land with
the build: F314 (Home's top row: the 48 dp search bar and three tonal
buttons, the card at the row's width) and F315 (the card in two columns,
the lead inside it, raised at level 3). F313 (Home's page stays still, only
its card scrolls) is on its way to main as well. Where this Spec says "the
card on Home", it means the card as F315 draws it.

## 1. What the card holds, and in which order

One source for every form: the session (`app.session.keys`,
`app.session.wallets`), the flows in progress (`in_progress.rs`), Faraday's
files held to write (`app.outbox`) and the vaults open. Every form draws
from it through one component; no page keeps its own list of what is
loaded.

The rows, in order (HOME.md §3's order, kept):

1. **The lead** (HOME.md §2), the featured first item, across the card.
2. **Flows in progress**: name, open step; resumes the flow.
3. **Files to write**: their count; opens Files.
4. **Wallets that need something**, ranked as today, with what they need.
5. **Backups**, while something is not backed up.
6. **Keys no wallet uses** (fingerprint, name, "passphrase", "No wallet uses it", Make a wallet).
7. **All keys** with its count, the "more" of the key rows.

A form shows a subset of these rows (§2); the order never changes.

## 2. The card's forms

- **Full (Home).** As F315: every row of §1, two columns from 840 dp of
  card width, Title L heading, level 3.
- **Compact (a menu page's top or left).** The same component on Tools,
  Check and the other pages of §4. Which of §1's rows it holds there is
  open (§12 choice 9); the lead, if shown, is a plain row (no filled
  button: the page is a launcher). Above the page it is a short strip (§4) with All
  keys standing for the rest; beside the page it shows the whole list of
  keys, scrolling within itself (owner, 2026-11-21). Heading "This
  session" in Title M.
- **Picker (a flow's key step).** When a tool is picked from a page the
  card stands on, the flow's first screen opens with the card still where
  it was, beside or above, and the key is picked in it there (owner,
  2026-11-21); the same from every page the card appears on. The same
  rows, now selectable: a key
  step that takes one key shows each loaded key as a single-choice row
  (Material's list item with a radio button); a step that takes several
  (a multisig's cosigner slots, a seed set, Vanity's keys) shows checkbox
  rows. The step's other ways in ("Add a key", "Load a wallet") are the
  card's last row, a text button, and return here with the new key ticked.
  Whether rows the step cannot take (a key already in another slot, a key
  with no seed for a tool that needs one) leave or stay dimmed is §12
  choice 9. Wallet steps (Sign a
  message's wallet, the KEF tool's wallet) take wallet rows the same way.
- **From showing to choosing** (owner, 2026-11-21). When the flow's first
  screen opens beside or above the card, the card shows that its keys are
  now the flow's and one is to be picked, in one transition at the Motion
  setting's duration:
  (a) its heading changes from "This session" to the flow's question
  ("Choose a key", "Choose the keys", "Choose a wallet");
  (b) the rows the flow cannot use leave or dim, as §12 choice 9 settles;
  (c) the rows left become Material selection-list rows: a radio button at
  the start (a checkbox where the flow takes several), a tonal container
  behind each row, the chosen one in the secondary container, the state
  layer on hover and press;
  (d) the card takes an outline in the primary colour, tying it to the
  flow;
  (e) the flow's own first screen reads "Choose a key" with nothing that
  competes with the card: no other key control, only its Next (off until a
  pick) and the step's own way back.
  Pills (chips) were not taken: Material keeps chips for short labels and
  filters, a key carries two lines (its fingerprint and name, and what uses
  it), and pills of varying widths are what the card replaces (§10).
- **Folded (a flow under way).** Once the pick is made the other rows fade
  out and the card folds to one tonal row, "Using: Savings" (or "Using: 3
  keys"), which glides to the flow card's head where the wallet chip stands
  today (FLOW-CONTAINER §2). A press on it opens the picker again, on the
  same step order rules as a row of Your choices (F296). Where a later step
  needs another key (a multisig's next slot) the picker opens there again,
  and folds again after.
- **Files' In Faraday.** Files' left pane (`screens/visit.rs`, `in_faraday`)
  takes the card's look and component, listing the files held to write
  with their checks and Write at its foot, as today. Its heading: §12,
  choice 6.

## 3. Its look per form

- **Home:** level 3 (`ELEVATION_FAB`), neutral surface-container-low, Title
  L (F315).
- **Off Home (compact, picker):** the same family (fill, corners, row
  design), the heading a step smaller, Title M, and lower: the owner picks
  level 3, level 1 or no shadow from renders (§12, choice 1). The
  recommendation is level 1: on these pages the card supports the page
  rather than leads it.
- **Folded:** a tonal row (secondary container), no shadow, the height of
  a list item.
- **Files:** as off Home; no shadow (it is one of two panes of equal rank).

## 4. Where the card stands on each page

The card's place on a menu page is a setting (§9): Automatic (the
default), Above the page or Left of the page. Automatic decides when a page
opens, never while the person is on it: loading a key does not move the
card under them; the next page opened uses the new place, reached by the
glide (§6). Automatic puts the card above while the rows it would hold fit
in a short strip (at most `HUB_ABOVE_ROWS` rows, a new token proposed at 3:
two keys and All keys, or a flow, a wallet and All keys; the lead counts as
two), and to the left once it holds more. The threshold counts the card's
rows, not keys alone.

**Left fits** where the room beside the navigation holds the card
(`LIST_PANE`, 360 dp), a `SPACER`, the page's least content width and the
margins (`MARGIN_WIDE` each side): on a menu page the least content is one
section card (`CATALOG_TILE_MIN`); in a flow's key step it is the flow's
column (`COLUMN_MAX`) and the Your choices pane (`side_panel_w`). So a menu
page fits Left from about 1150 dp of room (1366 × 768 with the sidebar
fits), a flow's key step with its pane from about 1750 dp (1920 × 1080
fits). Where Left does not fit, Above is used whatever the setting. A key
step opened beside a Left card on a narrower window (1366 × 768) keeps the
card where it was and opens Your choices from the bar as a sheet until the
pick, so the card never moves under the person.
Guide's pane (LAYOUT §6, from 1600 dp) counts in the room when it is open
or pinned as the page opens; opened later with no room left, Guide is the
sheet.

Every page the card appears on, with the proposed place:

| Page | Proposed place | Why |
|---|---|---|
| Home | centre, full form (not the setting) | the card is Home's focus (F315) |
| Tools | Automatic (owner, 2026-11-21) | with few keys the card is above; with many it is beside, the whole list shown |
| Check | Automatic | as Tools; most checks take a PSBT or a file, not a key, so the card is usually short |
| A flow's key step | where the card stood on the page the tool was picked from (owner, 2026-11-21) | the key is picked in the card the person already sees; a flow opened from Home or the sidebar takes Automatic |
| A flow under way | folded line at the card head | the flow keeps the focus (FLOW-CONTAINER §2) |
| Files | Left always, as In Faraday | it is already a pane beside the stick's pane |
| Settings | beside or around the card: §12, choice 2 | Settings is rarely used with what is loaded |
| Wallets start, Wallets, Vaults, Backups, Explore | not shown | each lists the loaded wallets, keys or vaults itself; a second list beside it repeats it |
| The Pi's menu, the phone-size window | Above, one column | no room beside (§8) |

## 5. Check and Tools as section cards

Each section of a menu page (Common, Keys and seeds, The wallet, Signing,
...) becomes one card holding a Material list: rows with an icon, the
title, the line under it and a chevron; hover and press are Material's
state layer across the whole row, not a pill; thin inset dividers between
rows or none (§12, choice 5). This replaces the grid of one outlined card
per choice (MENU-PAGE §4). The section's label is the card's heading inside
it, Title S.

The section cards are flat: a filled tint (surface-container), no shadow,
Material's filled card. Home's card is the only raised card, so it stays
the focus. On wide pages the section cards stand two or three to a row,
each as tall as its own list, set as Settings' cards are (MENU-PAGE §6,
columns balanced by height); one column on a page under two
`CATALOG_TILE_MIN`. The phone-size window and the Pi's menu take one
column of the same cards (the renders show whether the card reads better
there than today's tiles: §12, choice 5). The menu page container holds
it, so Tools, Check, Wallets start's ways in and the Pi's places all follow.

Material's lists are a single list in a page; a list inside a card is
Material's own pattern for grouping (Cards and Lists guidelines). The
difference is several such cards side by side: F317 records it.

## 6. Motion

- **The glide.** From Home to a menu page the card moves from Home's
  centre to its place on the new page, resizing as it goes (Material's
  container transform, a shared element): its bounds and corner radius
  ease from one to the other, its rows cross-fading inside it. Back to
  Home it moves back. From a menu page into a flow's key step it moves to
  the step's place; at the pick it folds and glides to the card head.
- **Fade through** for the rest of the page: the old page goes out over
  the first 35% of the eased progress, the new one comes in over the rest,
  as the step fade does today (MOTION.md §3.5).
- **One duration** for every menu change, set by the Motion setting (§9),
  Standard by default, on Material's emphasized curve
  (`EASE_EMPHASIZED`), against today's 200 ms linear crossfade
  (`motion::FADE_MS`). A step change inside a flow takes three quarters of
  it (300 ms at 400 ms Standard, today's `motion::STEP_FADE_MS`).
- **Secrets.** A frame drawn while a secret is typed or shown keeps no
  image to fade from (SECRET-LEAKS §5): such a change is at once, and a
  flow step that shows or takes a secret fades drawn live, as now.
- **Off** (§9): nothing moves; every change is at once.
- **The Pi.** The glide draws the card again each step over two kept
  frames (the old page and the new without the card), where today's fade
  only blends. It is timed on the Pi as a late-flow frame is (SPEED §7)
  before it is built there; if a step costs more than a frame, the Pi
  keeps a plain fade of the kept frames at the setting's duration, and may
  cap the duration after measuring.

## 7. Panes beside the card

Side by side only, never stacked: each pane scrolls within itself and the
window never scrolls as a whole. A pane is worth its room only where a
person uses both sides together; ranked, the strongest first:

1. **A row of the card and its details.** A key in the card opens its page
   in Explore beside the card, a wallet its card from Wallets, a file its
   view: the person goes down the card and reads each, the card held
   still (Material's list and detail; LAYOUT §5). Extra-large windows;
   below, the row opens its page as today.
2. **The card as a flow's picker** (§2): the pick in the card, the step
   beside it. This is the reuse itself, and makes most other pairings
   unneeded.
3. **Signing** (§11's survey): the facts a person compares stay in the
   Your choices pane, which is already a held pane beside the card; the
   change address and the signers join it there (no new pane).
4. **Files** (In Faraday beside the stick): already two panes.
5. **Settings beside the card**: weak; rendered both ways (§12, choice 2).
6. **Tools or Check beside the card** as a launcher beside a list: weak,
   since a tool starts a full-window flow; covered by the card's place
   (§4), not a pane.

## 8. The phone-size window and the Pi

The same rows in one column, Above always, the same model of moving: the
card folds in a flow, the picker is the same rows with radio or checkbox,
and a press on the folded line opens the picker. Home's one-column box
(HOME.md §4) stays as it is (F315); on Tools and Check the card is the
compact form above the page. The Pi's Motion is measured (§6).

## 9. Settings

Both settled by the owner (2026-11-21), on Settings › Appearance:

- **The card's place**: a segmented button, "Automatic", "Above the page",
  "Left of the page"; Automatic by default (§4). Where Left does not fit
  the window, the row shows the value in force beside it, "Above on this
  window". Kept with the other settings (`card=auto|above|left`).
- **Motion**: a segmented button, "Off", "Quick", "Standard", "Relaxed";
  Standard by default. Quick 250 ms (Material's medium 1), Standard 400 ms
  (medium 4) or 450 ms (long 1, §12 choice 3), Relaxed 600 ms (long 4).
  Every menu change uses it; a flow's step takes three quarters of it.
  **Off is today's Reduce motion** (MOTION.md §3.5, `stick/settings.rs`'s
  `reduce_motion`): it replaces that switch and stops everything it stops
  (fades, the glide, a scroll's coast and stretch, the dice tumble, the
  theme fade, a toast's rise). The settings file's `motion=reduced` reads
  as `off` and `motion=full` as `standard`; F222's default on a Pi's HDMI
  screen becomes Off until the person chooses.

Strings for `faraday-strings` (added with the build): `settings_card_place`
"Session card", `settings_card_auto` "Automatic", `settings_card_above`
"Above the page", `settings_card_left` "Left of the page",
`settings_card_above_here` "Above on this window"; `settings_motion`
"Motion", `settings_motion_off` "Off", `settings_motion_quick` "Quick",
`settings_motion_standard` "Standard", `settings_motion_relaxed`
"Relaxed"; `hub_using` "Using: {}", `hub_using_keys` "Using: {n} keys";
`hub_choose_key` "Choose a key", `hub_choose_keys` "Choose the keys",
`hub_choose_wallet` "Choose a wallet", `hub_no_seed` "No seed held",
`hub_in_slot` "In slot {n}".
`settings_reduce_motion` goes.

## 10. Survey: every place a loaded key, wallet or file is listed or picked

Today a key step draws the loaded keys as a wrapped row of buttons of
varying widths, `faraday_ui::parts::button_rows` with
`crate::wallet::key_line` labels, the chosen one in the selected style (the
"pill row"). Each place reads `app.session` directly; none shares a
component with Home's card.

| Place (file) | Shows | Picks | Becomes |
|---|---|---|---|
| Home's card (`screens/home.rs`, `key_rows`, `wallet_rows`, `going_rows`) | flows, files, wallets, Backups, keys, All keys | opens | the full form |
| Home's one-column box (`compact/mod.rs`, `nav/screen.rs` lines) | vaults open, flows, keys, wallets, files | opens | unchanged (F315) |
| Child seeds and passwords, key step (`bip85/screen.rs`, `card`) | loaded keys, Add a key | one, pill row | picker, single |
| Explore and Derive (`explore/screen.rs`) | loaded keys | one, pill row | picker, single; Explore itself is All keys |
| Convert a seed (`convert/screen.rs`) | loaded keys, an aezeed | one, pill row | picker, single |
| Silent payments, key step (`silent/screen.rs`) | loaded keys | one, pill row | picker, single |
| Lightning node key (`lightning/screen.rs`) | keys, aezeed, Core Lightning | one, pill row | picker, single (its other sources as rows) |
| Create's slots, a multisig's cosigners (`screens/create.rs`, slot rows) | keys not in another slot | one per slot, pill row | picker, one slot at a time, opening again per slot |
| A seed set (`seeds/screen.rs`) | "Use {key}" per key | several, buttons | picker, several (checkboxes) |
| Vanity, key step (`vanity/screen.rs`) | keys, fingerprint and label | several, checkboxes | picker, several |
| KEF tool (`kef_tool/screen.rs`) | wallets, then keys | one, full-width buttons | picker, single (wallet rows, key rows) |
| Sign a message, wallet (`screens/message.rs`, `MWallet`) | wallets that can sign a message | one, button row | picker, single wallet |
| My seed makes my wallet, holders (`ways_in/mod.rs`, `Way::Holder`; `seed_check/screen.rs`) | loaded keys as ways in | one | picker, single |
| Check a signer, seed rows (`decode/check_screen.rs`, `seed_row`) | the keys here that signed | none (facts) | unchanged |
| Spend from backup (`spend_backup/screen.rs`) | each wallet as a card with readiness | one | picker, single wallet, readiness as the row's line |
| Back up's wallet menu (`screens/backup.rs`, `BWallets`) | wallets, a menu | one | stays a menu (it switches the board) |
| Save to a vault (`vault/screen/save.rs`) | keys and wallets, checkboxes | several | picker, several |
| Wallets, list and keys without (`screens/wallets.rs`) | wallets, loose keys | opens | unchanged; card not shown (§4) |
| Wallets start (`screens/wallets_start.rs`, `loose_key_rows`) | keys without a wallet | opens | unchanged; card not shown |
| Files, In Faraday (`screens/visit.rs`, `in_faraday`) | files held to write | several, checks | Files' form (§2) |
| Vaults (`vault/screen.rs`, `list`) | vault files | opens | unchanged; card not shown |
| Scan's key sheet (`screens/sheets.rs`) | keys a QR names | none | unchanged |
| Sign a transaction (`screens/spend.rs`) | the wallet's keys here | none: `to_sign` is filled from the PSBT (`wallet.rs`) | unchanged; folded line names the wallet |

So: about twenty places; twelve picks become the picker form (nine single,
three several), Files takes the card's look, and the rest are lists that
stay (they are the full lists the card's "All keys" opens).

## 11. Survey: the signing flows, where a person looks up or down to compare

Two panes already stand in every flow on the wide layout: the card and
Your choices; Sign a transaction's review also puts "This transaction"
(six facts and the signatures) under Your choices (`screens/spend.rs`,
`review_under`). Candidates, with the screen each is on:

1. **A change output against the wallet it belongs to** (Sign, step 2
   Transaction: the Change row and the change warnings, `review_rows`,
   `review_warnings`; the wallet's own change address is on step 1 Check,
   `step_addresses`, "Change 1/0"). Kept, the strongest: the person goes
   back a step to compare. The change address joins the held pane
   (`review_under`), beside the Change row.
2. **A multisig's cosigners against the wallet's keys** (Sign, steps 4
   Keys, 5 Sign and 6 Collect: `step_keys`, `step_sign`, `step_collect`).
   Kept, moderate: who signs is split over three steps. The pane holds the
   wallet's keys with each one's mark (signs here, signed, waiting) across
   those steps. Check a multisig setup (`multisig_check/screen.rs`) has its
   marks on one card: dropped there.
3. **A long review whose summary scrolls away from the outputs** (Sign,
   step 2: rows, other transactions, warnings, then the table). Covered on
   the wide layout by `review_under`; on a narrow window the pinned foot
   (`spend_bar`) keeps the signature count and Continue. Kept only as is.
4. **Check a signer, sent against returned** (`decode/check_screen.rs`):
   the differences are on the result card. Dropped; the result names the
   two files instead.
5. Address against path, a registered wallet against its keys, inputs
   against outputs: on one card each. Dropped.

## 12. Choices for the owner

Each names the mock render that shows it (`out/mock/workspace/`).

1. **The card's look off Home.** (a) level 1 shadow (recommended); (b)
   level 3 as Home; (c) no shadow, a filled card. Renders
   `look-l1.png`, `look-l3.png`, `look-flat.png`: Tools at 1920 × 1080,
   Cobalt Light and Dark.
2. **Settings and the card.** (a) Settings in the second pane beside the
   card (recommended: the card stays the focus); (b) Settings' cards
   around the card. Renders `settings-beside.png`, `settings-around.png`
   at 1920 × 1080 and 2560 × 1440.
3. **Standard's duration.** (a) 400 ms, Material's medium 4, the owner's
   "about 400" (recommended); (b) 450 ms, long 1, where Material's long
   range starts and the glide sample's default. Strip `glide-400.png`,
   `glide-450.png`: Home to Check at 1920 × 1080.
4. **The card's place on each page** (§4's table).
   - Check: (a) Automatic, as Tools (recommended); (b) always Above,
     since most checks take a file, not a key. Renders `check-auto-few.png`,
     `check-auto-many.png` at 1366 × 768 and 1920 × 1080 (Tools follows
     Automatic, the owner's call: `tools-auto-few.png`,
     `tools-auto-many.png` show it).
   - Wallets start, Wallets, Vaults, Backups, Explore: (a) not shown
     (recommended: each is a list of the same things); (b) Left as on
     Tools. Render `wallets-left.png` at 1920 × 1080.
   - Files: (a) Left, as In Faraday (recommended); (b) by the setting.
     Render `files-card.png` at 1366 × 768 and 1920 × 1080.
   - A flow opened from Home's lead, the sidebar's flow row or a search
     result (no page card to keep): (a) Automatic by the step's rows
     (recommended); (b) the picker inside the flow's card. Renders
     `keystep-left.png`, `keystep-above.png` (Child seeds, five keys) at
     1920 × 1080, and `keystep-pi.png` at 480 × 640.
5. **Section cards' rows.** (a) thin inset dividers (recommended); (b) no
   dividers; and on the phone-size window and the Pi, (a) the cards
   (recommended if the renders read well); (b) today's tiles. Renders
   `check-cards.png`, `tools-cards.png` at 1366 × 768, 1920 × 1080, 2560 ×
   1440, before and after, and `tools-cards-pi.png` at 480 × 640.
6. **Files' pane heading.** (a) "In Faraday" kept (recommended: it names
   where the files are, against the stick's pane); (b) "Loaded files".
   Render `files-card.png`.
7. **Automatic's threshold.** (a) 3 rows (recommended); (b) 2 rows. Renders
   `tools-auto-few.png` with two keys and three.
8. **Do toasts, the chart's side sheet and the theme fade follow Motion?**
   (a) No: they keep their Material durations, and stop only at Off
   (recommended: they are not page changes); (b) they scale too.

9. **What the card holds on Tools and the other menu pages, before and
   while a tool is picked** (open: the owner decides; the rest of the
   picker is settled, §2).
   - Before a pick, on the page: (a) the flows in progress, the wallets
     that need something and the keys, with no files and no Backups, which
     are Home's and Files' (recommended: the page is for acting with keys
     and wallets, and the card stays short enough to stand above); (b) the
     keys alone, with All keys; (c) every row of §1, as on Home.
   - While a tool is picked, a key it cannot take: (a) stays, dimmed, with
     its reason as its line ("No seed held", "In slot 2") (recommended: a
     person looking for a key sees why it cannot be used); (b) leaves with
     the other rows the flow cannot use.
   Renders `menu-holds-a.png`, `menu-holds-b.png`, `menu-holds-c.png` (Tools
   at 1920 × 1080, two wallets, five keys, a flow in progress, a file to
   write), and `choose-dimmed.png`, `choose-gone.png` (Child seeds from
   Tools, five keys of which one has no seed), with a frame strip
   `choose-strip.png` at Standard.

## 13. What must come first, and which files change

F313, F314 and F315 land first: the card this Spec reuses is theirs. The
card's component comes out of `screens/home.rs` into a module of its own
beside the menu page container (`screens/menu_page.rs`), with the full,
compact, picker and folded forms; `screens/home.rs` and `compact/mod.rs`
call it. The menu page container gains the card's place and the section
cards (`screens/catalog.rs`, `check/screen.rs`). Each key step of §10 swaps
its pill row for the picker (`bip85`, `explore`, `convert`, `silent`,
`lightning`, `screens/create.rs`, `seeds`, `vanity`, `kef_tool`,
`screens/message.rs`, `ways_in`, `spend_backup`, `vault/screen/save.rs`);
the flow container (`flow.rs`) draws the folded line at the card head.
Files' In Faraday (`screens/visit.rs`) takes the component. Motion:
`motion.rs` (the glide, durations from the setting), `stick/settings.rs`
(the two settings, Reduce motion's migration), Settings › Appearance, and
`core/faraday-draw/src/tokens.rs` (`HUB_ABOVE_ROWS`). The renders of §12
come before any of it. HOME.md, MENU-PAGE.md, MOTION.md, LAYOUT.md §5 and
FLOW-CONTAINER.md follow in the change that builds each part.
