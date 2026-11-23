# This session as Faraday's hub: one pane of what is loaded, everywhere

**Kind:** Spec
**Status:** Partly built: Home's card with its groups (F323, `docs/SESSION-PANE.md`), the flow pair as one card (F324), a change of page at the Speed and in the Motion style (F318) and the flat section cards of the menu pages (F317) are on main; the rest is not built: the session pane as the one view of what is loaded and going on, on the menu pages and in every flow, the page as panes, and the rest of the motion (§6).
**Open:** To build: the session pane's modes and its form on the menu pages and in the flows (§2; Home's card in groups is built, `docs/SESSION-PANE.md`), the pairs and stacked pages of §4, the pick becoming Your choices at the left in a flow with wide content and the backup board, the flow pair's one card there (§4, §6), row buttons and details on the pane (§2, §7; Files' row menu already shows the shared list), search results and the guide as panes (§7), the pane container and its lint (§14), the top bar's search and pills (§4), the session pane's place handed to the glide, which moves it (§6), Settings as one tile (§9), the timings on the Pi that §6 and §8 name, and the take-away (§15). Not decided: what Vaults' pane holds (§2), whether the flow's end is one card or two (§15), whether a sheet is centred on the window or on the step card, whether the page's name stays in the top bar, where the Scan button stands on a computer, whether prose is a block (the guide's text), and the backup board's insides.

"This session", Home's raised card, becomes the session pane, Faraday's hub: everything about
what is loaded and going on lives in it, and it works with every part of
Faraday the same way, at every size. Material 3 has no "persistent context"
component; the pane is built from Material's parts (a card holding a list,
single and multiple selection rows) and differs from Material only where a
decision says so (F316 to F320, F322, F323, F324).

This Spec builds on Home as it is built: F313 (Home's page stays still, only
its pane scrolls), F314 (Home's top row) and F315 (the pane raised at level
3, the lead inside it). Where they differ from this Spec, F323 says what
changes.

## 1. What the pane holds, and in which order

One source for every form: the session (`app.session.keys`,
`app.session.wallets`), the flows in progress (`in_progress.rs`) and the
files Faraday holds (`app.inbox`, taken in from a stick or a scan, and
`app.outbox`, held to write: Files' In Faraday). Every form draws from it
through one component, the session pane (§14); no page keeps its own list
of what is loaded.

The pane holds only what is in the session (owner, 2026-11-22). It has no
row to make or restore something: no "Make a new wallet" or "Restore an
existing wallet" lead row, no "Make a wallet" button on a key row, no "Make
a new key" row. A thing being made, or made, is in the session and shows.
It has no row for a plugged stick (the lead covers it) and none for an open
vault (the badge on Vaults stands for it); a locked vault file Faraday can
open has a row under Waiting, with Unlock (owner, 2026-11-23).

The rows stand in groups under small headings (owner, 2026-11-22; reverses
F315 point 2's "no sub-headings"), in this order on Home:

1. **Waiting:** the lead (HOME.md §2), a row for each waiting PSBT (fewest
   signatures first), flows in progress (name, open step; a press resumes),
   files to write, the last write ("Written to {stick}"), a locked vault
   file with Unlock, an import put off with Later, and a vault changed since
   it was written. These show always, with keys and wallets loaded too
   (owner, 2026-11-23).
2. **Wallets:** every loaded wallet, its need in orange where it has one;
   Backups is the group's last row, while something is not backed up.
3. **Keys:** every key, those a wallet uses included, the line naming the
   wallet ("No wallet uses it" otherwise). A key's name shows in full,
   wrapping where needed.

A group with no rows is not drawn. A menu page's pane is named by its
**mode**: the kinds of the user's own data its tools take (keys, wallets,
files, flows), in the order that page's tools need them (§2). The source is
the same; the mode only says which kinds are drawn and which leads.

**The PSBT** is a one-column item as the others are. Its Sign button stands
under its two lines, at the text's start, never at the card's far edge; each
waiting PSBT is one such item, and the card scrolls inside when long
(owner, 2026-11-23). Any lead stands in the Waiting group, not across the card's columns.

**No All keys row** on Home, Tools or Check. It stays only where keys are
cut: the phone-size window and the Pi (§8).

**Nothing is shown twice on one screen**: an item in the pane is not also
in the page's tile. Files is the one exception (§2).

## 2. The pane's forms

Each form is a mode of the one session pane (§14), not a component of its
own.

- **Home.** Every group of §1, everything in the session (F323). On a wide
  card the groups stand in columns: as many columns of at least 336 dp as
  fit, 24 dp apart, each group whole, the next group into the shortest
  column. The card grows as long as its content; where the window is too
  short the card's own box scrolls (F313), its heading and the top bar
  staying. Nothing is dropped. Title L heading, level 3. With nothing
  loaded the card still stands at its place, so nothing moves when the
  first item loads: it holds what is real (a locked vault file with Unlock,
  an import put off with Later, the last write), and with none of these the
  one line "Nothing loaded"; it has no Make or Restore row (owner,
  2026-11-23).
- **A menu page's pane.** On Tools, Check and Wallets (§4 says where). One
  list, 360 dp wide, each group's small heading then its rows. The lead is
  a plain row on Tools and Check (no filled button: the page is a
  launcher). Heading "This session" in Title M. A list too long for the
  pane scrolls within it, the heading staying.

  | Page | Groups, in order | Not shown | Why |
  |---|---|---|---|
  | Tools, a PSBT waiting | Files (the waiting PSBT first), Keys, Wallets, In progress | Backups, files to write, vaults open | a file waiting is what a tool is ready for: Sign a transaction opens on it |
  | Tools, none waiting | Keys, Files (whenever any is held), Wallets, In progress | as above | the most tools take a key |
  | Check | Files (PSBTs and transactions first, then wallet and setup files, then the other kinds a Check tool takes), Wallets, Keys (every key), In progress | Backups, files to write, vaults open | most Check tools start from a file; Check an address and Convert an xpub take a watch-only key |
  | Wallets | Waiting (a waiting PSBT with Sign, held wallet and setup files, flows in progress), Keys | the wallets: the page's tile lists them | the pane holds what Wallets' actions use |

- **Wallets' and Vaults' right tile** ends with a row to make one: a tonal,
  highlighted row, "Make a new wallet" on Wallets and "Make a new vault" on
  Vaults. It stands at the list's foot where the list fits, and is pinned
  at the tile's foot where the list scrolls. Wallets' tile lists only the
  wallets: "Keys without a wallet" leaves it, since the keys are in the
  pane. Make a new key is reached through New wallet's key pick.
- **Vaults** is two panes, its tile at the right holding the vault files,
  then an open vault's contents. What its pane at the left holds is not
  decided.
- **Files** has no This session. Its left pane is In Faraday, its heading
  kept (`screens/visit.rs`, `in_faraday`), listing the files held with
  their ticks, Write its last row; the stick stands at the right. A file
  both in Faraday and on the stick is listed in both and highlighted in
  green in both (owner, 2026-11-22): Files shows where a file exists.
- **Settings** has no pane (§4, §9). **The backup board** has no This
  session: it is a step of its flow (§7).

**Row buttons and what a row opens** (owner, 2026-11-22). A row carries one
button at most, only for a kind with one obvious next step. A press on the
row opens its details (§7), led by "What you can do": every flow that
applies, likeliest first, then the facts. Each item's "What you can do" is
one shared list, used by this pane's details and by Files' row menu alike.
A long button keeps its words; the file's name then holds to one line, cut
with an ellipsis. "Add to vault" is never a row button and shows in the
details only while a vault is open.

| Item | Row button | What you can do |
|---|---|---|
| PSBT to sign, its wallet loaded | Sign (Collect where it belongs to the spend under way) | Sign or Collect; Check a signer; Decode |
| PSBT fully signed | Check a signer | Check a signer; send back (QR, stick); Decode |
| PSBT no loaded wallet can sign | Check a signer | Check a signer; Decode |
| Finished transaction | Decode | Decode; Check a signer |
| Wallet or descriptor file, not loaded | Load | Load; Add to vault; Check a multisig setup; Check an address; Back up; Spend from backup |
| Wallet file, loaded | none | Check a multisig setup; Check an address; Back up; Spend from backup |
| Account key file; key line (xprv, hex seed) | Use key | Use key, then the flows that take it |
| Seed words | Add a key | Add a key; Restore; Spend from backup; Check a backup; Add to vault |
| SLIP-39, codex32 or Seed XOR shares, enough to restore | Restore | Restore; Check a backup; Add a key; Add to vault |
| The same, not enough | none | Check a backup; Add a key; Add to vault |
| Split sheet of a setup | Restore | Restore |
| Vault file, locked | Open | Open |
| Message file | Check | Check a signed message |
| Text | none | Sign a message (a message wallet loaded); Check an address |
| Krux code | Open | Open |
| GPG public key that can be restored | Restore | Restore a GPG key |
| Bitcoin Core wallet file | Read | Read a Bitcoin Core wallet |
| Key with a seed | none | Explore; the flows that take a key with a seed |
| Key no wallet uses | none | Make a wallet from this key; Explore; the flows that take it |
| Key, watch-only | none | Explore; multisig; single-key wallet; Vanity address |
| Wallet | none | Sign (while a PSBT of it waits); Back up; Spend from backup; Check an address; Sign a message |
| Wallet needing a key | Add a key | Add a key to the slot; as a wallet |
| Flow in progress | Resume | Resume |

- **The pick (the picker; a flow's first step, and each later step that takes one).**
  The flow opens with the pane where its pair has it (§4) and the choice is
  made in the pane. Rows are selectable: a radio button where the flow
  takes one, a checkbox where it takes several. The groups are Files,
  Wallets, Keys, then Other ways in ("Ways in" when nothing loaded is
  taken). Rules (owner, 2026-11-22):
  - A row of the right kind that the step cannot take stays, dimmed, with
    its reason as its line ("In slot 2", "No seed held", "Not signed yet",
    "Single-key wallets only"). A held file of a kind the flow never takes
    is hidden.
  - A watch-only key (with its origin) is taken wherever a public key does
    the job: wallets, multisig, Vanity address, Convert an xpub, Check an
    address. In seed tools it is dimmed "No seed held".
  - A flow that takes nothing loaded lists its ways in as rows to pick. A
    tool with a single way in (Units) opens straight on it.
  - The pick grows into the room below it before it scrolls.
  - A key added or scanned from a way in returns to the pick, ticked.

  What each flow's pick holds, where more than "what it takes, then its ways
  in":

  | Flow | The pick |
  |---|---|
  | Make a new wallet | the first step is the kind of wallet, the pane still This session; each key slot then offers the loaded keys (those in other slots dimmed), Make a new key, a key from a vault, Scan its xpub, Someone else adds it later |
  | Sign a transaction | held PSBTs; Scan a transaction; A PSBT on the stick |
  | Add a key | Words, A SeedQR, A Krux code, codex32 shares; a "More kinds" row opening the rest of the ways-in list |
  | Back up a wallet | wallets; "A wallet file (from the stick)", which runs Restore's steps (pick the file, check it) and returns to Back up with that wallet loaded and picked |
  | Spend from a backup | loaded wallets, their readiness as the line; A stick, Words, Paper |
  | Check a signer | step 1 the PSBT sent; step 2 the pane lists held PSBTs again, the first dimmed, and Scan a PSBT |
  | My seed makes my wallet | keys with a seed, loaded wallets, held wallet and key files; the ways in |
  | Check an address | wallets, keys; Type a descriptor, Scan an address, A text file: the last two search every loaded wallet and key |
  | Sign a message | a wallet; a multisig one dimmed "Single-key wallets only"; a single-key wallet is picked at once where there is one |
  | Restore from SLIP-39, codex32 or Seed XOR; Check a backup in another form | held share, part and SeedQR files of the right kind where the flow takes a backup; the ways in |
  | Read a Bitcoin Core wallet; Check a signed message | held files of that kind; the ways in |
  | GPG key, Secure Boot keys | an open vault, or Make a new vault: they make their keys from fresh randomness and take no loaded key |

- **From showing to choosing.** Choosing a tool goes straight to the flow's
  one card (§4), the pick at the left and the step at the right, in one
  transition at the Speed setting (§9), from a stacked page and from a page
  beside its pane alike, with no side-by-side stage between:
  (a) the heading changes from "This session" to the flow's question
  ("Choose a key", "Choose the keys", "Choose a wallet");
  (b) the pick lists what the flow takes, made from the flow alone and not
  from what the page was showing: rows it does not take fade out, and rows
  it takes that the page's own mode hid (an open vault, a file to write, a
  backup) fade in, so a flow begins with the same rows whether it is reached
  from a page, Home, the sidebar or a search;
  (c) the rows left become Material selection-list rows: the radio or
  checkbox at the start, a tonal container behind each row, the chosen one
  in the secondary container, the state layer on hover and press;
  (d) the pane takes an outline in the primary colour, tying it to the flow;
  (e) the flow's own first step has nothing that competes with the pane: no
  other key control, only its Next (off until a pick) and its way back.
- **Your choices (a flow under way).** Once the pick is made the other rows
  fade out and the pane becomes the flow's Your choices, in place, at the
  left (F319): never at the right and never in the middle of the screen.
  The pick is its first row, a label and its value as every row of Your
  choices is ("Key: Savings", "Keys: 3 keys"), and each answer adds a row
  under it. A press on a row goes back to its step on F296's rules; the key
  row brings the pick back, and the pick folds to its row again after. A
  later step that needs another choice (a multisig's next slot, Check a
  signer's second PSBT) opens the pick in the pane again. The heading
  becomes "Your choices". A flow begun from Home's lead, the sidebar or a
  search result has the same pane at the left from its first step. The
  wallet chip at the flow card's head (FLOW-CONTAINER §2) goes: the wallet
  is a row.

## 3. Its look per form

- **Home:** level 3 (`ELEVATION_FAB`), neutral surface-container-low, Title
  L (F315).
- **Off Home, beside a page's tile:** the same family (fill, corners, row
  design), the heading Title M, level 1. The stacked page's card is Home's
  own (§4).
- **In a flow:** the pane and the step card are one card with an inset
  divider (§4, F324).
- **Group headings** are small labels over their rows, the same on Home, a
  menu page and the pick, so the pane reads the same from Home to a flow.
- **Orange** only on a need. **Green** only on a written and checked file.

## 4. Where the pane stands on each page

The place is fixed by the page and the room; there is no setting for it
(owner, 2026-11-22), and it never depends on how much is loaded or on
motion: with motion Off the pane stands where it would after the glide.

**The rules.**

1. **On Home the pane is centred**, directly under the top bar. The search
   and the three pills are in the top bar, so This session starts at the
   same height on Home as on every other page.
2. **On every other page with a pane, the pane stands at the left and the
   page's content in one wide tile at its right**, the content starting in
   the tile's top-left corner. The pane is `LIST_PANE` (360 dp), the tile
   takes the flow step card's width, up to `COLUMN_MAX`, the gap between is
   `HUB_PANE_GAP` (24 dp, 3 × `GRID`), and the pair is centred in the room
   beside the navigation.
3. **The page's pair and the flow's pair stand at the same place, at the
   same widths**, at any window size where they would otherwise differ. A
   flow entered from a menu page moves nothing sideways: the pane turns into
   the pick in place and the tile turns into the step card; only heights
   change. The tile's groups take the columns that fit its width and it
   grows taller. On a window too narrow for the pair at those widths the
   tile takes the room left (1366 × 768).
4. **A pair always ends level**: tops and bottoms line up, at the height the
   taller pane's content needs. A pane is never longer than its pair's
   content needs and is stretched to the window's foot only where the
   content needs it; then it scrolls inside.
5. **Stacked first, on Tools and Check.** Where the window has the height,
   the page's tile stands under the This session card, which stays exactly
   where Home has it, at Home's width and with Home's rows (not the page's
   own mode, §2), so opening the page does not change it. The tile is as
   wide as the card, 24 dp (rule 2's gap, not `MENU_GAP`) under it, and ends at least a
   page margin above the window's foot. Where it does not all fit, the pair
   of rule 2. Nothing is cut back until a tool is chosen. Entering a flow
   from a stacked page turns the layout into the flow's pair in one
   transition (§2), and Back returns to the page the person came by with
   every row it had.
6. **Files, Wallets and Vaults never stack.**
7. **Settings has no pane.** Where every one of its cards fits under Home's
   card as it stands, Settings opens there, the card not moving; otherwise
   Home changes to the Settings page. What fits decides, not the window's
   named size. No pane moves to or from Settings.

**The top bar.** The search stands in the top bar on every page that has
one, never inside a tile. The three pills (New wallet, Restore, Spend from
backup) are text only, with no icons, and stand beside it on Home only. The
search field shrinks to fit what else the bar shows; there is no second row
for the pills. The search, with Home's pills, is centred over This
session's pane; the network chip, ? and power stay in the window's corner.
This replaces F314's row above the card.

**In a flow** the pane (the pick, then Your choices) and the step card are
**one card with an inset divider** (F324): the divider stands at the middle
of the gap and is inset at its top and bottom by the card's padding; the
outer edges and everything inside stay exactly where two cards would have
them. The pair is centred, as above.

Every page, with its place:

| Page | Place |
|---|---|
| Home | centred, every group (§2) |
| Tools, Check | stacked under This session where it all fits; else the pair |
| Wallets | the pair |
| Vaults | two panes; the left pane's content not decided |
| Files | In Faraday at the left, the stick at the right |
| Settings | no pane; under Home's card where it all fits, else its own page |
| A flow's pick, and a flow under way | the flow's pair, one card with an inset divider |
| The backup board | one card with an inset divider: its list at the left as Your choices, Places at the right |
| The phone-size window, the Pi | one column (§8) |

The pairs are for the wide layout (the expanded rail: Large and over, not
low; LAYOUT §2). In a flow the column narrows to `FLOW_CARD_MIN`, then the
pane from `LIST_PANE` to `CHOICES_PANE_MIN` (FLOW-CONTAINER §4).

## 5. Check and Tools as tiles

The section cards are built (F317, `docs/MENU-PAGE.md` §4). What changes:
on Tools and Check every section stands inside **one tile**, the page's
second pane (§4), in columns that fit the tile's width, each section a
tonal group with its title and its list rows, 16 dp apart, 16 dp from the
tile's edge. The tile is stacked under This session or beside it, and
carries no search bar: Tools' search is in the top bar (§4). The
phone-size window and the Pi's menu keep one column of the same groups.

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
| Home to a page with a pane, a page to another | the pane glides (below); the rest fades through | the whole frame blends from the old screen to the new (today's fade of two kept frames) | at once |
| Home to Tools, Check or Settings where it stacks | the card does not move; the tile fades in under it | blends | at once |
| Home to Settings, its own page | no pane moves; fades through | blends | at once |
| Into a flow, from a pair | the pane stays at its place and turns into the pick; the tile turns into the step card; heights change; the two become one card, the join animated | blends | at once |
| Into a flow, from Home or a stacked page | the pane glides into the flow's pair as it turns into the pick; then as from a pair | blends | at once |
| The pick becoming Your choices | in place, the other rows fading out, the pick folding to its row | blends | at once |
| A step inside a flow | fades through (FLOW-CONTAINER §3) | blends | at once |
| A row's details, search results, the guide opening | fades through in the pane's place; where the guide needs room the pair slides left | blends | at once |
| A row arriving in the pane (a PSBT found, a key loaded) | fades in while its space opens | blends | at once |
| A row leaving (a flow finished) | fades out; the rows under it slide up | blends | at once |
| Expanding in place (More kinds in Add a key, any rows opening) | its height changes smoothly, the rows under it moving with it | blends | at once |
| A key added in a signing flow | its row arrives (as above), its tonal emphasis eases away, Sign's enabling fades in at the same moment | blends, then the emphasis eases away | at once, no emphasis |
| Hover | the state layer fades in over 150 ms (`MOTION_SHORT3`) and out over 200 ms (`MOTION_SHORT4`), not scaled by Speed | as Smooth motion | at once |

- **The glide.** The pane moves from its place on the old page to its place
  on the new one, resizing as it goes (Material's container transform, a
  shared element): its bounds and corner radius ease from one to the other,
  its groups cross-fading inside it. Back to Home it moves back. Where the
  page's pair and the flow's pair share a place it does not move at all
  (§4 rule 3). Because the pair is centred, it moves less than to the
  window's edge.
- **Fade through** for the rest of the page: the old page goes out over
  the first 35% of the eased progress, the new one comes in over the rest,
  as the step fade does today (MOTION.md §3.5).
- **Sheets and dialogs on a computer** blend over 200 ms, whatever the
  Speed setting, the page staying visible under them; Off shows them at
  once (owner, 2026-11-23). The guide's tab and pane stay as they are
  until decided.
- **One duration** for every page change and every step inside a flow
  (owner, 2026-11-21): the Speed setting's, 600 ms by default, a
  deliberate difference from Material's 450 ms fade through, approved by
  the owner (F318), against
  the 200 ms linear crossfade it replaced and the 300 ms step fade
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
  outside the security model. The owner said yes to it being built
  (2026-11-22); it is built and tested on its own and its results are shown
  to the owner before it reaches main (F318 point 7).
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

A page is one pane, or two side by side or one above the other, plus the
guide as a third (owner, 2026-11-22). Each pane scrolls within itself; the
window never scrolls as a whole.

1. **A row's details are the page's second pane.** They follow the
   stacked-first rule: where the window has the height they stand under
   This session, in the second pane's place; where not, beside it at the
   right, in the page's tile's place. There is no third card. Back, in the
   top bar, returns the tile. On Home they stand under Home's card where
   they fit, and beside This session where they do not. On Wallets, Files
   and Vaults, which never stack, they stand beside. "What you can do"
   leads (§2), then the facts. A wallet's own actions are a "What you can
   do" group of rows in place of its buttons.
2. **The pane as a flow's pick and Your choices** (§2), beside the step, as
   one card (§4).
3. **Search results sit in a pane**, stacked first by the same rule: under
   This session's card where the window has the height, on Home and on any
   stacked page; where it is too short, beside it at the right, This
   session at the left. A search finds tools, then Settings' cards, then
   the guide's glossary words (`faraday/faraday-core/src/tools/search.rs`).
4. **The guide is always the third pane; a sheet where there is no room.**
   It has the height of the other two panes, tops and bottoms level. At
   1920 × 1080 and over three panes fit: the pair slides left to make room
   and the guide stands at the right. Where the centred pair already leaves
   the room at its right, the pair does not move. The guide stays open
   across pages and flows, following to each page's or step's section;
   there is no pin. It has no pop-up and no over-the-page form. This would
   replace LAYOUT §6's and F310's forms.
5. **Signing** (§11's survey): the facts a person compares stay in Your
   choices; the change address and the signers join it there (no new pane).
   Sign a transaction's "This transaction" (`review_under`) stays under its
   rows, so it moves to the left with it.
6. **Files** (In Faraday beside the stick): two panes.

**The backup board** is a step of the backup flow and has the flow pair's
form: one card with an inset divider, its list of things to place at the
left as the step's Your choices, Places at the right as the step's work,
the checks and Done at the step's foot. The pane does not swap sides
entering the board. On the phone and the Pi the list is the Your choices
button and sheet, Places the screen. Its insides are not settled here.

## 8. The phone-size window and the Pi

The same rows in one column, the pane above the page, the same model of
moving: the pick is the same rows with radio or checkbox, and in a flow
Your choices keeps the top app bar's button and its sheet (F296 point 4,
kept: F319). Where the strip cuts the keys, an All keys row stands for the
rest. Home's one-column box (HOME.md §4) stays as it is (F315), drawn from
the same source.

At a flow's end no sheet opens by itself (owner, 2026-11-22): the take-away
is the one pane, as a step card; a sheet opens only when asked for (Show QR,
Write out).

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

**One tile of tonal group cards** (owner, 2026-11-22), set out as Tools is:
Appearance, Mode, Session, Time, Randomness, Display, What is remembered
and About, each a tonal group card with its title, in balanced columns.
Every setting's choices show on screen with no press to see them: inline
segmented buttons, switches and buttons, never a row that opens a sheet.
Appearance keeps the theme colour chips (the ten themes, DESIGN §4's filter
chip, THEMES §12) beside Light or dark, Contrast and the rest. Where
Settings stands is §4 rule 7; it has no session pane.

There is no setting for the session pane's place.

On Settings › Appearance (owner, 2026-11-21):

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

Strings for `faraday-strings` (added with the build): `settings_motion`
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
| Home's pane (`screens/home.rs`, `key_rows`, `wallet_rows`, `going_rows`) | flows, files, wallets, Backups, keys, All keys | opens | the Home form, in groups, without All keys |
| Home's one-column box (`compact/mod.rs`, `nav/screen.rs` lines) | vaults open, flows, keys, wallets, files | opens | unchanged (F315) |
| Child seeds and passwords, key step (`bip85/screen.rs`, `pane`) | loaded keys, Add a key | one, pill row | picker, single |
| Explore and Derive (`explore/screen.rs`) | loaded keys | one, pill row | picker, single |
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
| Wallets, list and keys without (`screens/wallets.rs`) | wallets, loose keys | opens | the wallets in the page's tile, ending in Make a new wallet; the loose keys leave it; the pane beside it (§2) |
| Wallets start (`screens/wallets_start.rs`, `loose_key_rows`) | keys without a wallet | opens | unchanged |
| Files, In Faraday (`screens/visit.rs`, `in_faraday`) | files held to write | several, checks | Files' form (§2) |
| Vaults (`vault/screen.rs`, `list`) | vault files | opens | the page's tile, ending in Make a new vault (§2); its pane not decided |
| Scan's key sheet (`screens/sheets.rs`) | keys a QR names | none | unchanged |
| Sign a transaction (`screens/spend.rs`) | the wallet's keys here | none: `to_sign` is filled from the PSBT (`wallet.rs`) | unchanged; Your choices' first row names the wallet | Sign with another key loaded here (while signatures are missing); Sign another transaction (opens the scanner); Check the signed transaction |

So: about twenty places; twelve picks become the picker form (nine single,
three several), Files takes the pane's look, and the rest are lists that
stay.

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

F313, F314, F315 and the section cards (F317) are built; this builds on them. The
session pane comes out of `screens/home.rs` into a module of its own, `session_pane`, beside
the menu page container (`screens/menu_page.rs`), with its forms as modes;
`screens/home.rs` and `compact/mod.rs` call it. The menu page container
puts its section cards inside one tile on Tools and Check
(`screens/catalog.rs`, `check/screen.rs`), and the pane container (§14)
places the panes. The top bar takes the search and Home's text-only pills.
The "What you can do" list is one module (§2), read by the pane's details
and Files' row menu (`file_menu.rs`). Each key step of §10 swaps
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
`stick/settings.rs` (Motion and Speed, Reduce motion's migration), Settings › Appearance, and `core/faraday-draw/src/tokens.rs`
(`HUB_PANE_GAP`, 24 dp, and `MOTION_LONG4`, 600 ms, Material's long 4, which
the tokens do not have yet).
HOME.md, MENU-PAGE.md, MOTION.md, LAYOUT.md §5 and §6, FLOW-CONTAINER.md §3
and §4 and F296 follow in the change that builds each part.

## 14. The containers

The owner (2026-11-21): the hub is built as containers, as the flow and
menu page containers are, each drawn by one piece of code, held by
`lint-components`, and described in its own document, written in the
change that builds it. CLAUDE.md's "By area" names each in that change (as
it names FLOW-CONTAINER.md and MENU-PAGE.md); it is not changed before.

- **The session pane** (`session_pane`; its document to be named SESSION-PANE.md). One component, fed by
  §1's one source; its forms (Home, a menu page, the key
  picker, Your choices, Files) are modes of it, and a page or flow names
  the mode, never draws rows of its own. It replaces Home's `key_rows`,
  `wallet_rows` and `going_rows` (`screens/home.rs`), every pill row and
  button list of §10's "picker" places, Files' `in_faraday`
  (`screens/visit.rs`), the Your choices pane of the flow container
  (`flow.rs`, `flow/unit.rs`: the container keeps declaring the rows, the
  pane draws them) and the wallet chip at a flow card's head.
  `lint-components` holds that a key step draws no `button_rows` of loaded
  keys and that no screen reads `app.session` to list keys or wallets.
- **The pane container** (its document to be named HUB-LAYOUT.md). Every
  page is made of panes (F322). A page or flow declares its panes into it,
  and it owns where they stand (§4: centred, the pair, stacked), that a
  pair ends level, the guide as the third pane, the glide between pages (§6)
  and the use of the Motion and Speed settings for page changes. It
  replaces each menu page's own placing of the pane, the flow container's
  centring of the unit and its pane (`flow/unit.rs`), and the screen fade's
  own choice of duration (`fade.rs`, `motion::STEP_FADE_MS`). `lint-components` holds that no page places a
  pane or times a page change itself, as it holds the menu page and flow
  containers, and that no card stands outside a pane.
- **The section card** (built, `docs/MENU-PAGE.md`): a group of a pane.

**What a page may draw** (owner, 2026-11-22; F322). A page draws only
chrome, up to two panes plus the guide, sheets, and the toast and the
tooltip, the only passing overlays.

- **Chrome:** the rail or bar (destinations, badges, the Scan button in its
  one place), the top bar (back or close, title, the search or its icon,
  Home's pills, the network chip only off Mainnet and pressable, the phone's
  Your choices button, the keyboard button, ?, one power button that opens
  the sheet with Lock and Power off), the phone's navigation bar, the
  keyboard docked at the window's foot, the step card's one action bar, and
  the Demo's host strip. No Stick chip; no Lock or Power at the rail's foot.
- **A pane holds groups.** It has no buttons in its head: back is the top
  bar's. It has no action bar at its foot: the action is a row, or ends a
  small flow.
- **A group holds rows, then at most one named block:** field, chart, QR,
  scanner frame, progress, chips, pad, worksheet, or canvas (the one block
  that fills a pane, for the backup board's Places and codex32's dice and
  tree).
- **A row carries one trailing button or one trailing control.** Settings'
  inline controls are such controls.
- **Sheets stand over panes:** dialogs, menus and Your choices are sheets.
  Locked is a sheet over the empty page. Self-test failed is the one
  full-window state.

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
  Files. The take-away and the step's card are level, as tall as the taller
  needs, the pair centred (§4). Whether they are one card with an inset
  divider, as the flow's other steps are, or two cards a gap apart is not
  decided; nor whether "Write to stick" is a button at the pane's foot or a
  row (§14).

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
for. A secret QR is drawn a little longer than today's last step draws it, until the pane
settles back. The owner said yes to this trade-off (2026-11-22); it is built
and tested on its own, and its results are shown to the owner before it
reaches main. The receipt keeps hashes only and survives a lock
as it does now. The take-away adds a state of no secret type; a test shows that
leaving the step releases a QR's secret.

**The phone-size window and the Pi.** Where the window is one column, the
take-away is the one pane, as a step card, with the next steps under it. No
sheet opens by itself; a sheet opens only when asked for (Show QR, Write
out). Your choices keeps the top app bar's button and its sheet at every
other step. The Pi's panel is the same. Its arrival emphasis follows the
Speed setting measured for the Pi (§8).

**Open.** All of §15 is to build (the Open line at the top).
