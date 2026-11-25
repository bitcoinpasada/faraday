# This session as Faraday's hub: one pane of what is loaded, everywhere

**Kind:** Spec
**Status:** Partly built: Home's card with its groups (F323, `docs/SESSION-PANE.md`), the flow pair as one card (F324), a change of page at the Speed and in the Motion style (F318) and the flat section cards of the menu pages (F317) are on main; the rest is not built: the session pane as the one view of what is loaded and going on, on the menu pages and in every flow, the page as panes, and the rest of the motion (§6).
**Open:** To build: the session pane's form in Restore and Spend from a backup (only their rows are the pane's; the pane itself is not drawn there), in Save to a vault, the backup board and Silent, and the checkbox row for loaded keys where a step takes several (built for Restore's share files and for the loaded keys of its open slot; Who signs fills one key at a time, §2; Home's card in groups, the modes of Tools, Check and Wallets and the pick of the other flows, which declare what they take and refuse into the pane, are built, `docs/SESSION-PANE.md`), the pick becoming Your choices at the left in a flow with wide content (the wallet chip as the first row is built where the pair stands) and the backup board, the flow pair's one card there (§4, §6), row buttons on the wallet rows and the shares' file row, and details on Vaults (§2, §7; Tools, Check, Wallets, Home and Files have them), the guide as a pane (§7), the rest of the pane container (§14: the guide as the third pane and the glide on it), Load in Vaults' pane (§2), Tools' and Check's pane at the flow's place at 1920 wide and wider (§4; its drawing cost is not measured), Files first in a pick opened from Check (§2), a pick with no seed held, a wallet already holding the key and a flow's pick leaving out the keys it cannot take (every builder answers for every key), the heights (F331: one resting level, the flow's card raised alone with its own fill, §3, §6), Mode inline with its one confirmation (§9), the Scan button under Settings, the viewfinder in the pane of the flows other than Check an address, its form over a folded strip and its popout (§4), the backup board's insides and codex32 from dice (§7), the flow's one card for the one-page tools and checks other than Units (§2), the timings on the Pi that §6 and §8 name, the take-away in the flows other than Sign a message (§15), flows in progress and the way out of a flow (§16), Tools lit in the sidebar by a tool opened from it (§16), starting with files on the boot stick (§17) addresses shown whole (§18) and one sign for each kind of press (F332, §19); what of these is built is in the sections. Not decided: the Pi's default Motion, until its timings (§8); how the flow's card's fill looks (§3).

"This session", Home's raised card, becomes the session pane, Faraday's hub: everything about
what is loaded and going on lives in it, and it works with every part of
Faraday the same way, at every size. Material 3 has no "persistent context"
component; the pane is built from Material's parts (a card holding a list,
single and multiple selection rows) and differs from Material only where a
decision says so (F316 to F320, F322, F323, F324, F331, F332).

This Spec builds on Home as it is built: F313 (Home's page stays still, only
its pane scrolls), F314 (Home's top row) and F315 (the pane raised at level
3, the lead inside it). Where they differ from this Spec, F323 and F331 (the
heights) say what changes.

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
F315 point 2's "no sub-headings"). As built, Home has them in this order:

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

**Keys first** (owner, 2026-11-24). The side pane has one order: Keys,
Wallets, Files, flows. Home leads with Keys, then Wallets. Check keeps Files
first (§2). The picks' order is not yet changed (Open).

**Waiting is a band at the pane's foot** (owner, 2026-11-24), on Home
as on Tools, Check and Wallets, at the same place in the pane on each. It is
across the card, outside the part that scrolls, with a hairline over it, the
lead first in it with its button. It takes at most about a third of the pane's
height and scrolls inside beyond that. Its rows stand in columns on a wide
card. The lead's button is tonal on Tools and Check. With nothing else in the
session the band is still at the pane's foot; with nothing waiting it is not
drawn. The phone-size window and the Pi do not change (§8).

**What a vault loads has no group of its own** (owner, 2026-11-24): a GPG
key stands with the keys, after all the others, a note and the Secure Boot
keys with the files, each marked with its kind.

A group with no rows is not drawn. A menu page's pane is named by its
**mode**: the kinds of the user's own data its tools take (keys, wallets,
files, flows), in the one order above (§2). The source is
the same; the mode only says which kinds are drawn and which leads.

**The PSBT** is a one-column item as the others are. Its Sign button stands
under its two lines, at the text's start, never at the card's far edge, and
so does every other lead's button (Carry on, Finish, Unlock, Write, Open); each
waiting PSBT is one such item (owner, 2026-11-23). A locked vault file is one
row each too. Any lead is one of Waiting's rows, one column wide. A waiting
PSBT is listed only under Waiting, on every page with the pane; a pick lists it
only where the flow takes PSBTs (owner, 2026-11-24).

**No All keys row** on Home, Tools or Check. It stays only where keys are
cut: the phone-size window and the Pi (§8).

**Nothing is shown twice on one screen**: an item in the pane is not also
in the page's tile. Files and Vaults are the exceptions (§2).

## 2. The pane's forms

Each form is a mode of the one session pane (§14), not a component of its
own.

- **Home.** Every group of §1, everything in the session (F323). On a wide
  card the groups stand in columns: as many columns of at least 336 dp as
  fit, 24 dp apart, each group whole, the next group into the shortest
  column. The card grows as long as its content; where the window is too
  short the card's own box scrolls (F313), its heading, the Waiting band at its
  foot (§1) and the top bar staying. Nothing is dropped. Title L heading, level 1 (§3). With nothing
  loaded the card still stands at its place, so nothing moves when the
  first item loads: it holds what is real (a locked vault file with Unlock,
  an import put off with Later, the last write), and with none of these the
  one line "Nothing loaded"; it has no Make or Restore row (owner,
  2026-11-23).
- **A menu page's pane.** On Tools, Check and Wallets (§4 says where). One
  list, 360 dp wide, each group's small heading then its rows, and Waiting the band at its
  foot, the lead with its button (§1). Heading "This session" in Title M. A list
  too long for the pane scrolls within it, the heading and the band staying.

  | Page | Groups, in order | Not shown | Why |
  |---|---|---|---|
  | Tools | Keys, Wallets, Files (whenever any is held), In progress; Waiting pinned at the foot | Backups, files to write, vaults open | the most tools take a key, and the picks have the same order (owner, 2026-11-24) |
  | Check | Files (transactions, and PSBTs that do not wait, first, then wallet and setup files, then the other kinds a Check tool takes), Keys (every key), Wallets, In progress; Waiting pinned at the foot | Backups, files to write, vaults open | most Check tools start from a file; Check an address and Convert an xpub take a watch-only key |
  | Wallets | Keys; Waiting pinned at the foot (a waiting PSBT with Sign, held wallet and setup files, flows in progress) | the wallets: the page's tile lists them | the pane holds what Wallets' actions use |

- **Wallets' right tile** lists only the wallets, then, pinned at the tile's
  foot and neutral, none tonal (owner, 2026-11-24), the rows that the pills do
  not repeat: with the pills on screen, or the **Start** button that stands for
  them, one row, Add a key. Where there is neither, all four stand, in two columns, two rows, in Home's order: Make a new
  wallet, Restore an existing wallet, Spend from a backup, Add a key.
  "Keys without a wallet" leaves the tile, since the keys are in the
  pane. Make a new key is reached through New wallet's key pick.
- **Vaults' right tile** ends with a neutral row (owner, 2026-11-24), "Make a new
  vault", at the list's foot where the list fits and pinned at the tile's
  foot where the list scrolls.
- **Vaults** is two panes (owner, 2026-11-23), built as the pair
  (`screens/vaults_pair.rs`, Vaults' mode of the pane: Keys, then Wallets,
  Waiting at the foot). At the left stand this session's keys and wallets.
  While exactly one vault is open each is marked "In a vault" where that
  vault holds it and "Not saved" where it does not; a key with a BIP-39
  passphrase is "In a vault" only where the vault holds it with that
  passphrase, and where the vault holds its words alone it reads "Words in
  a vault" (owner, 2026-11-25), with no Save. A row not saved
  carries Save, one press (the vault's own Save: a key by itself, with no
  passphrase, which stays Save with passphrase on the key's page, so a key
  with a passphrase saved from the pane reads "Words in a vault"; every
  check of that Save stays). With no vault open, two open, or the open one
  read-only, a row has no Save, and a read-only vault's rows only the mark;
  with none or two open there is no mark. Its tile at
  the right holds the vault files, each a card with View contents and Lock or
  Unlock, ending in Make a new vault; an open vault's contents are still a
  full page, with its own Save panel. Load is still Open.
  Load and Save are one press per item, with no ticks and no drag and drop.
  An item both in the vault and in the session is shown in both places,
  marked in both.
- **Files** has no This session. Its left pane is In Faraday, its heading
  kept (`screens/visit.rs`, `in_faraday`), listing the files held with
  their ticks, Write its last row: a tonal row of two lines, "Write", then
  "The checked files, to the stick" (owner, 2026-11-23); Write stays tonal, an
  action on that page (owner, 2026-11-24). Where the list goes
  on below, it fades at its foot, a line "N more · scroll to see them" ("1
  more · scroll to see it") stands whole above the Write row, and a scroll
  bar runs at the list's right (owner, 2026-11-24). The stick stands at the
  right. A file
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
| Vault file, locked | Unlock | Unlock |
| Message file | Check | Check a signed message |
| Text | none | Sign a message (a message wallet loaded); Check an address; Add to vault |
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
  takes one, a checkbox where it takes several. The groups are Keys,
  Wallets, Files, flows, then Other ways in ("Ways in" when nothing loaded is
  taken). A pick opened from Check (Check an address, say) keeps Check's order,
  Files first; opened from anywhere else, Keys first (owner, 2026-11-24). Rules (owner, 2026-11-22):
  - A row of the right kind that the step cannot take stays, dimmed, with
    its reason as its line ("In slot 2", "No seed held", "Not signed yet",
    "Single-key wallets only"). A held file of a kind the flow never takes
    is hidden. A key of a kind the flow never takes is left out too, both
    ways (owner, 2026-11-24): no GPG key is offered to a bitcoin flow, and no
    bitcoin key to a flow that takes only a GPG key.
  - A watch-only key (with its origin) is taken wherever a public key does
    the job: wallets, multisig, Vanity address, Convert an xpub, Check an
    address. In seed tools it is dimmed "No seed held".
  - A flow that takes nothing loaded lists its ways in as rows to pick. A
    tool with a single way in (Units) opens straight on it.
  - A key pick that takes several (Restore's and Spend from a backup's own
    among them) is checkbox rows with one button to use the ticked keys; a
    key already in the wallet is dimmed with its reason (owner, 2026-11-24).
  - **Every tool and check stands on the flow's one card** (owner,
    2026-11-23, 2026-11-24), those of one page too: Units, Hashes and the
    like, and Decode a transaction, Check an address, Nonce check, Check a
    signed message and Invoice, their ways in at the left. Explore and
    Derive, Lightning node key and the seed set page have a left pane as a
    tool has. Lightning's holds two rows per key (ldk-node and Core
    Lightning); LND's aezeed and Core Lightning's `hsm_secret` are buttons at
    the top of the card.
  - Units (built, on a window that is not compact) is one card with no Next
    step: the unit, the typed amount and its conversions stand together and
    change as you type. The left pane lists nothing but an "Add an amount"
    button at its foot, dimmed at 6. The amounts are Material 3's primary
    tabs over the card, each a conversion of its own with its own unit and
    text; a tab's cross removes it, the last cannot go, and a long amount
    is cut before the cross. Opened, there is one amount. The amounts last
    as the one amount did: they stay while Tools is open and go on leaving
    Tools or at Lock. The small panel is the page as before.
  - The pick grows into the room below it before it scrolls.
  - A key added or scanned from a way in returns to the pick, ticked.
  - A key file brought in from Add a key's stick button does the same where
    the flow has a pick for it: Restore's seeds, Spend from a backup, Child
    seeds, and Make a new wallet's slot that asked (the slot is kept only
    while Add a key or its Files visit stands, dropped by any way off Files,
    the pull or a lock). On the device no stick is read while a slot waits; in
    Faraday Demo the slot fills while no key or vault is open. The flow is
    back at its step with the slot filled and the toast "Key {fingerprint}
    added", or the last refusal when a key was refused. A key file opened with no pick waiting,
    or from Home, ends on Wallets with the key listed.

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
  from a page, Home, the sidebar or a search (from Check, in Check's order);
  (c) the rows left become Material selection-list rows, in the same
  order and at the same scroll (owner, 2026-11-24): the radio or checkbox in
  the place of the row's icon, at the start, a tonal container behind each row, the chosen one
  in the secondary container, the state layer on hover and press;
  (d) the pair is one card, raised alone with its own fill (§3, §4), and the
  pane has no outline of its own (owner, 2026-11-23, 2026-11-24);
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


A typed address is never cut to its end: the field it is typed in (Check an
address, Explore's Find an address) grows to the lines it needs, the address
in groups of four, and the page scrolls where the window is too short. The
spaces are drawn only; the text, the caret, a selection and a paste stay the
characters typed (`docs/TEXT-CURSOR.md` §3).

## 3. Its look per form

- **Heights (F331; owner, 2026-11-24).** Everything raised on a menu page
  rests at one level, level 1 (`ELEVATION_SHEET`): the search, the pills (the
  **Start** button where it stands for them), This session, the sidebar and
  the tiles, on Home as on every page. In a flow only the flow's card (the
  pick or Your choices and the step) has a height: level 3; the sidebar, the
  search and the pills are flat, at none. They change in the movement in
  which the two panes become one card, and change back on leaving it (§6).
  Nothing that goes flat is dimmed. Menus, dialogs, sheets, the toast, the
  notice and the phone's Scan button keep their levels, a menu being drawn over
  the flow's card all the same; the section cards stay
  flat (F317); the phone-size window and the Pi do not change. A High
  contrast theme draws no shadow (DESIGN, Elevation): what has a height there
  has an outline in its place, and what goes flat in a flow keeps it. A flow
  with one column and no pair, on a window with a rail, raises its one card to
  level 3. The backup board's places and the guide's pane are flat in a flow
  (owner, 2026-11-24).
- **Home:** level 1, neutral surface-container-low, Title L.
- **Off Home, beside a page's tile:** the same family (fill, corners, row
  design), the heading Title M, the same level. The stacked page's card is
  Home's own (§4).
- **In a flow:** the pane and the step card are one card with an inset
  divider (§4, F324), at level 3, filled surface-container-low, a step off the
  page's surface in light and in dark and the fill the pane and the tile have
  already, so nothing changes colour as they join; the value is still to be
  looked at in the build. It has no outline, except at High contrast, where
  the thin one stays.
- **Group headings** are small labels over their rows, the same on Home, a
  menu page and the pick, so the pane reads the same from Home to a flow.
- **The sidebar** (owner, 2026-11-24) is a raised panel with a soft shadow on
  its right edge and no line, in a flow neither shadow nor line: its fill sets
  it off from the page. At High contrast it has its outline in both.
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
3. **A flow's pair and a flow-bound page's pair stand at the same place,
   at the same widths**, except that Tools' and Check's tile is wider (owner,
   2026-11-23, 2026-11-24): at 1920 wide their pane stands at the flow's place
   and the tile takes the rest of the width at its right, its groups in two
   columns or more, so that most of Tools shows without scrolling. The pane
   therefore does not move when a tool opens: it turns into the pick in place
   and the tile turns into the step card, in one movement (§6). A wider window
   keeps the pair at its width at 1920 (`MENU_TILE_WIDE_MAX` becomes the tile's
   width there) and the pane at the flow's place, so the pane does not move
   there either and the pair stands right of the centre (owner, 2026-11-24).
   Narrower than 1920 the same holds (owner, 2026-11-24): the pane stays at the
   flow's place, the tile keeps two columns while both fit beside it and has
   one where they do not. A room that holds one column (1366 × 768) keeps the
   flow's place for both. Wallets
   and the other pairs move nothing sideways, only heights change.
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
   transition (§2).
   **Side by side holds until Home** (owner, 2026-11-23, 2026-11-24): once a
   page or a flow has stood side by side, every page after it stands side by
   side, whether it would fit stacked or not, until Home is shown again. A
   flow always stands side by side, so leaving a tool opened from a stacked
   page shows that page side by side, with every row of its mode (§16). A row's
   details and search results follow the same rule.
6. **Files, Wallets and Vaults never stack.**
7. **Settings has no pane.** Where every one of its cards fits under Home's
   card as it stands, Settings opens there, the card not moving; otherwise
   Home changes to the Settings page. What fits decides, not the window's
   named size. No pane moves to or from Settings.

**The top bar.** The page's name stands at the bar's left end. The search and the three pills (New wallet, Restore, Spend from
backup; text only, with no icons) stand in the top bar on every menu page,
never inside a tile, at one place and one size, at the resting level (§3),
and never redraw on a menu change (owner, 2026-11-24). The search field shrinks to fit
what else the bar shows; there is no second row for the pills. They are
centred where Home has them; the network chip, ? and power stay in the window's corner.
This replaces F314's row above the card. It stands there on every window
with a rail from 600 dp wide (`Nav::search_in_bar`); the bottom bar, the
Pi's panel and a rail window of the compact class (400 x 500) keep the
page's row. Where the room is short the field shrinks to `HOME_SEARCH_MIN`
and then the three pills become one **Start** button beside it (its label ends in a down chevron), which opens
a menu over the page, anchored under the button, with New wallet, Restore and Spend from backup
(Material's menu: rows 48 tall, no title or icons, as wide as its rows need, inside the window, above the button where it does not fit below; a popup at every window size, not a sheet)
(`bar_search::place`, the one rule of the bar and of the page's row: on the
compact rail the row holds the field and the **Start** button). The menu
closes by Escape, Back, a press outside it and a press on a row. About 1020
dp and wider keep the three pills. The search's icon is a magnifying glass at
every size. **The search and the pills stand still when the guide opens or
closes, on every page**, also where the pane slides aside (owner,
2026-11-23).

Built: one search and one text on every menu page with a bar (Home, Tools,
Check, Wallets, Files, Vaults, Settings), at one place and size; a press on a
sidebar page or a pill clears it, a result's press keeps it for the way back.
Wallets' page leaves out the three rows the pills repeat only where the three
pills stand.

**The search and the pills in a flow** (F331; owner, 2026-11-24). They stay
where they are, flat and not dimmed (§3), and work, where they fit beside the
flow's heading: the bar shrinks as above first (the field, then **Start**),
and only where that does not fit either are they left out. They are held off
wherever the sidebar is held off. A press on a pill works as a press in the
sidebar does, the flow kept in progress (§16). The pill of the flow on screen
is off, in the **Start** menu too, so New wallet never starts a second one
inside the first; on any other page New wallet, while a wallet is being made,
goes back to that one (owner, 2026-11-24). A search typed there does not leave the flow: a page with
the results alone takes the flow's card's place (no This session beside them),
its tile, the search, the pills and the sidebar at level 1. Escape or the
field's cross brings back the same step with what was typed in it; only a
press on a result leaves the flow, kept in progress. A search over a step that
shows a secret hides the secret at once (owner, 2026-11-24).

**Scan on a computer** (owner, 2026-11-24). Scan stands in the sidebar
directly under Settings, on every page, as a tonal button (a soft fill), not
at the sidebar's foot and not filled; on the narrow rail it stands directly
under Settings too, as an icon button. It opens no screen: the viewfinder takes
the session pane's content, the page beside it staying. On Home, where This
session is the one card, the viewfinder takes its place; a flow's ways in
show it in the pane in the same way. What was read is the pane's card (for an
address: Check this address, Scan another, Close); Escape or Close puts the
pane back. The session pane's content fades to the viewfinder, at once with
Motion Off (§6). A popout button on the viewfinder's corner enlarges it over the
page, centred, with a button that puts it back (§6). Where the pane is a strip
over a stacked page (Tools, Check), the strip grows to the viewfinder's height
and the page moves down; the Scan box is as wide as the two columns below it
and aligned with them, not the strip's full width (owner, 2026-11-24).

**In a flow** the pane (the pick, then Your choices) and the step card are
**one card with an inset divider** (F324): the divider stands at the middle
of the gap and is inset at its top and bottom by the card's padding; the
outer edges and everything inside stay exactly where two cards would have
them. The one card has a height and a fill of its own and no outline, except at
High contrast (§3). The pair is centred, as above.

Every page, with its place:

| Page | Place |
|---|---|
| Home | centred, every group (§2) |
| Tools, Check | stacked under This session where it all fits; else the pair, the pane at the flow's place at 1920 and wider (rule 3), the tile taking the rest of the width at 1920 in more columns |
| Wallets | the pair |
| Vaults | the pair (built; marks and Save built, Load Open): this session's keys and wallets at the left, the vault files at the right, an open vault's contents on a page of their own |
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
tonal group with its title, its list rows and a 1 dp outline-variant line at its
edge (Material's outlined card; owner, 2026-11-23), as on every tile of group
cards, 16 dp apart, 16 dp from the
tile's edge, and as many columns as `CATALOG_TILE_MIN` leaves room for
(§4 rule 3). The tile is stacked under This session or beside it, and
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
| Into a flow, from Tools' or Check's pair where the pane stands at the flow's place (at 1920 and wider, §4) | the pane does not move and turns into the pick; the tile turns into the step card, in one movement | blends | at once |
| Into a flow, from another pair | the pane stays at its place and turns into the pick; the tile turns into the step card; heights change; the two become one card, the join animated, the heights changing with it (below) | blends | at once |
| Into a flow, from Home or a stacked page | the pane glides into the flow's pair as it turns into the pick; then as from a pair | blends | at once |
| The pick becoming Your choices | in place, the other rows fading out, the pick folding to its row | blends | at once |
| The search and the pills, menu page to menu page | no change | no change | no change |
| The heights, into a flow and out of it (§3) | in the movement in which the two panes become one card: the card takes its fill and rises to level 3, and the shadows of the sidebar, the search and the pills go; back on leaving | blends | at once |
| The search and the pills, into a flow and out of it | stay; the field shrinks, or they become **Start**, only where the flow's heading needs the room | blends | at once |
| A search typed in a flow, and cleared (§4) | the results fade through in the flow's card's place, the heights changing in the same movement | blends | at once |
| A step inside a flow | fades through (FLOW-CONTAINER §3) | blends | at once |
| A row's details, search results | fades through in the pane's place | blends | at once |
| The guide opening and closing | slides in from the right edge; in the same movement the pair slides left where the guide needs the room, or the tile of Tools or Check draws in to its narrower form, This session staying; closing is the reverse, the pair going back to its place, in a flow too; the search stands still (owner, 2026-11-23) | blends | at once |
| The session pane's content becoming the viewfinder, and back (§4) | fades | blends | at once |
| The viewfinder's popout | the frame grows from its own place to the centre in one movement, and goes back | blends | at once |
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
  once (owner, 2026-11-23). The guide's tab stays as it is until
  decided; its pane slides in from the right edge and out again
  (MOTION §3.5).
- **One duration** for every page change and every step inside a flow
  (owner, 2026-11-21): the Speed setting's, 600 ms by default, a
  deliberate difference from Material's 450 ms fade through, approved by
  the owner (F318), against
  the 200 ms linear crossfade it replaced and the 300 ms step fade
  (`hub_layout::STEP_MS`). A step no longer takes a shorter share of it.
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
  image to fade from (SECRET-LEAKS §5): such a change is at once (a search over
  such a step too, §4), and a
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
   Opening an item is not leaving the page (owner, 2026-11-24), from
   Tools as from any page with the pane: the page's tab stays selected,
   the pressed row is marked in the pane, and the top bar shows a back
   arrow and the item's name. One Back, or a press on the page's tab,
   returns the page where it was scrolled; a press on another row swaps
   the details in place, with no history of rows. A wallet's details are
   the content of its page on Wallets. A flow begun from the details has
   the item already picked, so the pane turns straight into Your choices,
   and leaving it returns to the details. Where the item goes away (a key
   unloaded, a file taken away) the page returns; Lock clears as anywhere.
   Built on Tools, Check and Wallets where the window is not compact, and on
   Home on the expanded rail only, the rail's windows keeping Home as it is (`row_open`, `screens/row_details.rs`): the open row by what it is (a key
   by its fingerprint, a wallet by its identity, a file by its name and
   size) in its own area; a key's details offer Explore and, where no wallet
   uses it, Make a wallet from this key, Add another key and Restore; a
   wallet's are its page; a file's are its shared list, then its kind and
   size as Files says them. A row with a button opens them on a press beside
   the button. The offers end in the chevron where they open a page or flow
   and not where they act at once (F332). A flow begun from the details
   returns to them when it is left by Back; a press on another tab clears
   them. On Home they stand in the second pane's place as a search's results
   do (point 3): under the card where they fit (a wallet's page never stacks:
   it stands beside), else beside it; a held file's row is a waiting PSBT's;
   a typed search shows its results and the row stays open behind it. A file's
   offers carry the icon their action has elsewhere (Sign, Check an address,
   Back up, Make a new wallet, Restore a wallet, Open a vault); an action with
   none has none, and a group with none has no gap for one. On Files, on the
   expanded rail only (compact, the rail's windows and the Pi unchanged), a
   file of In Faraday's Inbox opens its details in the stick's place, beside
   the list, and the stick's card is not drawn while they stand (its state is
   untouched, the chip stays in the top bar). The owner's rule (2026-11-25):
   the box ticks, the rest of the row opens the details. Only the tick box
   ticks, in a press box of its own at the box; a press anywhere else of the
   row (the name, the line, the chevron) opens them and leaves the tick as it
   was; a row's button and its menu stay their own presses. Rows with no
   stick in (no box then, the whole row opens them) and rows already on the
   stick open them too. A file that asks before it ticks (a seed) asks at
   its box only. The files waiting to be written and the stick's own
   rows are untouched. Not built: Vaults' details.
2. **The pane as a flow's pick and Your choices** (§2), beside the step, as
   one card (§4).
3. **Search results sit in a pane**, stacked first by the same rule: under
   This session's card where the window has the height, on Home and on any
   stacked page; where it is too short, beside it at the right, This
   session at the left. A search finds tools, then Settings' cards, then
   the guide's glossary words (`faraday/faraday-core/src/tools/search.rs`). In
   a flow the results stand alone, in the flow's card's place (§4).
4. **The guide is always the third pane; a sheet where there is no room.**
   It has the height of the other two panes, tops and bottoms level, and is
   flat in a flow (§3). At
   1920 × 1080 and over three panes fit: the pair slides left to make room
   and the guide stands at the right. Where the centred pair already leaves
   the room at its right, the pair does not move. The guide stays open
   across pages and flows, following to each page's or step's section;
   there is no pin. It has no pop-up and no over-the-page form. Beside
   Tools' or Check's wider tile at 1920 the tile shrinks to its narrower form
   while the guide is open. When the guide closes the pair goes back to its
   place, at the Speed, also in a flow (owner, 2026-11-23). This would
   replace LAYOUT §6's and F310's forms.
5. **Signing** (§11's survey): the facts a person compares stay in Your
   choices; the change address and the signers join it there (no new pane).
   Sign a transaction's "This transaction" (`review_under`) stays under its
   rows, so it moves to the left with it.
6. **Files** (In Faraday beside the stick): two panes, as wide as each other (`Tile::Equal`, `docs/HUB-LAYOUT.md`).

**The backup board** is a step of the backup flow and has the flow pair's
form: one card with an inset divider, its list of things to place at the
left as the step's Your choices, Places at the right as the step's work,
the checks and Done at the step's foot. The pane does not swap sides
entering the board. On the phone and the Pi the list is the Your choices
button and sheet, Places the screen.

Its insides (owner, 2026-11-22 to 2026-11-24):

- **The plan choices** (Paper only, Paper and a vault, Two vaults) stand
  before the steps, each on one line in its button.
- **Your choices is sequential**: one column, one choice at a time: how many
  places; which places hold a sealed vault; which seed goes to each place,
  into a vault where there is one; where the wallet description goes; shares
  (a multisig's keys left off each share); watch-only software; then the
  checks and Done, pinned at the foot. A step that does not apply is skipped.
  A step passed shows nothing of what was chosen; a press on it goes back.
  **Show all** at its foot opens everything at once, in two columns where
  the content needs them, balanced: Seeds and the wallet description left;
  Shares, Vaults and watch-only right.
- **Width**: the flow pair's width for up to two places and a short list; with
  three or more places, or a list longer than one column, the card widens to
  the whole work area. Places are a grid, 1 in a row at 1366, 2 at 1920, 3 at
  2560 and wider (3 at 3840 too), the rest scrolling.
- **A place** is a card, flat in the flow's card (§3), with no coloured bar at its left; cards side by
  side end level. Inside it is a flat list: one plain row per thing kept, its
  leading icon for paper, stick or vault, the format and where in the
  supporting text, one ⋮ per row; no box in a box. Colour stands only as a
  small tag (sealed, not sealed). A vault with nothing in it is shown, with
  "Nothing kept".
- **Nothing twice**: "+ new vault" stands only in Your choices' Vaults group;
  "Add a stick" is in a place's menu, with no dashed box; "+ place" is one
  empty card at the grid's end; an empty box shrinks to its header. A
  row's formats are switched in its menu, the row listing only those on.
  Removing a place, a stick or a vault is in its menu, two taps, no X. "Made in
  the checklist" and "linked" show only while a vault's tag is hovered. "Add
  watch-only software" has its arrow.

Settled as drawn (owner, 2026-11-24): Show all's two columns and which group
stands in which, removal in a menu, the formats in a menu, the two hover
facts, "Add a stick" in a menu, three columns at 3840, and the phone's steps
as one sheet over Places.

**codex32 from dice** (owner, 2026-11-24) shows every part on one screen: the
worksheet at the left, the dice sheet in a right column, its tree over the
dice tracks.

## 8. The phone-size window and the Pi

The same rows in one column, the pane above the page, the same model of
moving: the pick is the same rows with radio or checkbox, and in a flow
Your choices keeps the top app bar's button and its sheet (F296 point 4,
kept: F319). Where the strip cuts the keys, an All keys row stands for the
rest. Home's one-column box (HOME.md §4) stays as it is (F315), drawn from
the same source. Waiting stands there as it does now, the lead one row at the
top (owner, 2026-11-24).

At a flow's end no sheet opens by itself (owner, 2026-11-22): the take-away
is the one pane, as a step card; a sheet opens only when asked for (Show QR,
Write out).

The Pi's default Motion is decided by the owner after timing the first test
build there (owner, 2026-11-23; SPEED §7: a late-flow frame timed with the
harness of SPEED §4, on the Pi 3B+). It replaces the rule that took the
fullest style whose step fits the screen's frame, which would give Off: a
fade step takes about 34 ms there (MOTION.md). Its default Speed is set the same way, and may be below
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

**The navigation** (owner, 2026-11-23, 2026-11-24). The phone's navigation bar
is hidden on every flow step and on the backup board, and no swipe brings it
back: a step is left only by its Back or Close. Faraday Demo's host strip is
drawn at computer sizes only, not in the phone's or the Pi's frame.

## 9. Settings

**One tile of tonal group cards** (owner, 2026-11-22), set out as Tools is:
Appearance, Mode, Session, Time, Randomness, Display, What is remembered
and About, each a tonal group card with its title, in balanced columns.
Every setting's choices show on screen with no press to see them: inline
segmented buttons, switches and buttons, never a row that opens a sheet.
Appearance keeps the theme colour chips (the ten themes, DESIGN §4's filter
chip, THEMES §12) beside Light or dark, Contrast and the rest. Where
Settings stands is §4 rule 7; it has no session pane.

**Mode** (owner, 2026-11-24) is inline in its card, a segmented button of the
modes. Pressing another asks first in one sheet: "Change to Testnet?", with
Not now and Lock and use Testnet, worded by the mode pressed.

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
| Explore and Derive (`explore/screen.rs`, `explore/pane.rs`) | loaded keys | one | built: on the wide layout the keys stand as the picker's rows in a pane at the left of the card and the result stays in the card; the one-column layouts and a window too narrow for the pair keep the pill row, and the Addresses and Find pages have no pane |
| Convert a seed (`convert/screen.rs`) | loaded keys, an aezeed | one, pill row | picker, single |
| Silent payments, key step (`silent/screen.rs`) | loaded keys | one, pill row | picker, single | Make another record |
| Lightning node key (`lightning/screen.rs`, `lightning/pane.rs`) | keys, aezeed, Core Lightning | one | built for the keys: on the wide layout each loaded key's two sources stand as the picker's rows in a pane at the left of the card; the aezeed and Core Lightning `hsm_secret` sources stay buttons in the card, and the one-column layouts, a window too narrow for the pair and the page opened from Convert keep the pill row |
| Create's slots, a multisig's cosigners (`screens/create.rs`, slot rows) | keys not in another slot | one per slot, pill row | picker, one slot at a time, opening again per slot |
| A seed set (`seeds/screen.rs`) | "Use {key}" per key | several, buttons | picker, several (checkboxes) |
| Vanity, key step (`vanity/screen.rs`) | keys, fingerprint and label | one, radios | built: the loaded keys stand as the picker's radio rows in a pane at the left of the card; a press chooses the key and goes on; the one-column layouts keep the same rows in the card |
| KEF tool (`kef_tool/screen.rs`) | keys, then wallets | one, full-width buttons | picker, single (wallet rows, key rows) |
| Sign a message, wallet (`screens/message.rs`, `MWallet`) | wallets that can sign a message | one, button row | picker, single wallet | Sign another message; Check a signed message |
| My seed makes my wallet, holders (`ways_in/mod.rs`, `Way::Holder`; `seed_check/screen.rs`) | loaded keys as ways in | one | picker, single |
| Decode a transaction (`decode/screen.rs`, `decode/flow.rs`) | held transactions, Type it, Scan, Open Files; the shown one marked | one | built: where the container's pair stands the rows are the picker's, at the left of the card, and Result stands in the card; where it does not stand the pick is drawn in the card on the Transaction step. Type it opens a field on the Transaction step (on a small panel, among the page's ways in); Next reads the text through the classification and reading a file of the same bytes gets, so it takes a transaction or a PSBT, in hex or base64, and refuses what such a file is refused; the text is not put in Files, and it is dropped at a Lock and on leaving, as the invoice's is |
| Check a signed message (`screens/check_message.rs`) | held signed-message files, Scan, Open Files; the shown one marked | one | built: where the container's pair stands the rows are the picker's, at the left of the card, and Result stands in the card; where it does not stand the pick is drawn in the card; a small panel is the page |
| Nonce check (`screens/nonce_check.rs`, `decode/nonce_flow.rs`) | the file or scan it opened from, All inputs and a row per input (its number, its signatures, its verdict) | none | built where the container's pair stands: the pane's rows choose which input's signatures the card shows (it opens on All inputs); every warning stays at the card's top whichever row is chosen, and the buttons and the ending stay at its foot for the whole transaction; the choice is dropped on leaving the check and at a Lock; a small panel keeps the page |
| Check a signer, seed rows (`decode/check_screen.rs`, `seed_row`) | the keys here that signed | none (facts) | unchanged |
| Spend from backup (`spend_backup/screen.rs`) | each wallet as a card with readiness | one | picker, single wallet, readiness as the row's line |
| Back up's wallet menu (`screens/backup.rs`, `BWallets`) | wallets, a menu | one | stays a menu (it switches the board) |
| Save to a vault (`vault/screen/save.rs`) | keys and wallets, checkboxes | several | picker, several |
| Wallets, list and keys without (`screens/wallets.rs`) | wallets, loose keys | opens | the wallets in the page's tile, ending in its pinned rows (§2); the loose keys leave it; the pane beside it (§2) |
| Wallets start (`screens/wallets_start.rs`, `loose_key_rows`) | keys without a wallet | opens | unchanged |
| Files, In Faraday (`screens/visit.rs`, `in_faraday`) | files held to write | several, checks | Files' form (§2) |
| Vaults (`vault/screen.rs`, `list`) | vault files | opens | the page's tile, ending in Make a new vault (§2); its pane this session's keys and wallets (built, `screens/vaults_pair.rs`) |
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
   (`review_under`), beside the Change row: the payment's own change address
   with its path, not the wallet's first. A row each where the payment has
   several, in output order, and only for an output the device has verified as
   change: no row where there is none (owner, 2026-11-23, 2026-11-24). Sign
   and Create a wallet list it and the signers at the left once chosen.
2. **A multisig's cosigners against the wallet's keys** (Sign, steps 4
   Signers, 5 Sign and 6 Signatures for this transaction: `step_keys`, `step_sign`, `step_collect`).
   Kept, moderate: who signs is split over three steps. The pane holds the
   wallet's keys with each one's mark (signs here, signed, waiting) across
   those steps; a key added there gets §6's "here is your key". Built for
   Sign a transaction: once Signers is reached, one row per key under its
   row in Your choices (fingerprint, name, then "Signs here" where the key is
   still to sign (not complete, no warning that stops Sign, the key one the
   spend uses and not yet signed here, and no key the spend uses owing its backup), or "Signed"
   while a verified signature of it is in the file; no mark for any other
   key. The mark follows the inspection and what Sign is to sign, not
   what the signer then does: a refusal or skip found only while signing
   (a key mismatch, an unsupported input, a leaf not carried, amounts
   refused, an input already final) is not seen, so a key can read "Signs
   here" and sign nothing; once the key has signed here its mark goes), a press opening Signers until Sign is done,
   after which it takes none, as the steps before Sign. The mark is drawn
   whole after the fingerprint and name, which is what is cut. "Waiting" is
   not drawn. A single-key wallet has no Signers step: its key's row stands
   under Sign's own step, with the same mark. The fingerprint is in the mono face
   and the mark in its own ink, the primary role; a mark drawn as a pill has its word centred
   (owner, 2026-11-24). Check a
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
places the panes. The top bar takes the search and the text-only pills, on every menu page and, where they fit, in a flow (§4).
The "What you can do" list is one module (§2), read by the pane's details
and Files' row menu (`file_menu.rs`). Each key step of §10 swaps
its pill row for the picker (`bip85`, `explore`, `convert`, `silent`,
`lightning`, `screens/create.rs`, `seeds`, `vanity`, `kef_tool`,
`screens/message.rs`, `ways_in`, `spend_backup`, `vault/screen/save.rs`);
the flow container (`flow.rs`) gives its Your choices rows
to the pane at the left and drops the wallet chip at the pane head. Files'
In Faraday (`screens/visit.rs`) takes the component. The take-away
(§15) is drawn by the flow container (`flow.rs`), in the session pane,
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
  (`flow.rs`: the container keeps declaring the rows, the
  pane draws them) and the wallet chip at a flow card's head.
  `lint-components` holds that a key step draws no `button_rows` of loaded
  keys and that no screen reads `app.session` to list keys or wallets.
  Built for the pick of Make a new wallet, BIP-85, Convert, KEF, Sign a
  message, Check a signed message, Decode, Invoice, the Seed check, Explore
  and Lightning: each declares what it takes, chooses and refuses
  (`screens/session_pane/pick.rs`, `Decl`) and the pane lists the rows, in
  Keys, Wallets, Files order, with the key row's line; `lint-components`
  (`pick-own-rows`) holds the count of files that list `session.keys` or
  `session.wallets` for a row.
- **The pane container** (`screens/hub_layout/`, `docs/HUB-LAYOUT.md`; built for Tools, Check, search results, the flows, Explore, Files and Wallets' pair; the guide as the third pane and the glide are not on it). Every
  page is made of panes (F322). A page or flow declares its panes into it,
  and it owns where they stand (§4: centred, the pair, stacked), that a
  pair ends level, the heights (§3), the guide as the third pane, the glide between pages (§6)
  and the use of the Motion and Speed settings for page changes. It
  replaces each menu page's own placing of the pane, the flow container's
  centring of the unit and its pane, and the screen fade's
  own choice of duration (`fade.rs`). `lint-components` holds that no page places a
  pane or times a page change itself, as it holds the menu page and flow
  containers, and that no card stands outside a pane.
- **The section card** (built, `docs/MENU-PAGE.md`): a group of a pane.
- **The scanner frame** (built for Check an address' **Scan an address** on the wide layout, `screens/scan_frame.rs`, `address_check/pane.rs`; the scanner sheet draws the same frame): the camera's picture, the found code's outline, brackets, state line and parts bar in a pane at the left of the card where the pair stands. The scan state is the sheet's own, so there is one camera and every way the sheet ends a scan ends this one (`docs/SCANNING.md` §7.1). The popout, the other scans and the sidebar's Scan are not built.

**What a page may draw** (owner, 2026-11-22; F322). A page draws only
chrome, up to two panes plus the guide, sheets, and the toast and the
tooltip, the only passing overlays.

- **Chrome:** the rail or bar (destinations, badges, the Scan button in its
  one place), the top bar (back or close, title, the search or its icon,
  the pills (on the menu pages, and in a flow where they fit), the network chip only off Mainnet and pressable, the phone's
  Your choices button, the keyboard button, ?, one power button that opens
  the sheet with Lock and Power off), the phone's navigation bar, the
  keyboard docked at the window's foot, the step card's one action bar, and
  the Demo's host strip. No Stick chip; no Lock or Power at the rail's foot.
- **A pane holds groups.** It has no buttons in its head: back is the top
  bar's. It has no action bar at its foot: the action is a row, or ends a
  small flow.
- **A group holds rows, then at most one named block:** field, chart, QR,
  scanner frame, progress, chips, pad, worksheet, running text (the
  guide's only, owner, 2026-11-23; working screens keep labels, values and
  actions), or canvas (the one block that fills a pane, for the backup
  board's Places and codex32's dice and tree).
- **A row carries one trailing button or one trailing control.** Settings'
  inline controls are such controls. A row that opens ends in the chevron
  (§19), on the session pane and on the menu pages alike.
- **Sheets stand over panes:** dialogs, menus and Your choices are sheets.
  On a wide window a sheet opened during a flow stands centred on the
  window, the same place on every page and flow (owner, 2026-11-23).
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
  needs, the pair centred (§4), as one card with an inset divider like the
  flow's other steps (owner, 2026-11-24); "Write to stick" is one filled
  button at the pane's foot, in three states: off with nothing ticked,
  "Waiting for a stick" with no stick in, "Written and checked" after the
  file is read back (owner, 2026-11-24).

**Built.** Sign a message's last step, where the pair stands
(`flow/take_away.rs`, `Contained::take_away`): the heading, the file as a
tick row, the signed message's code as a row with Show as QR, and the button
at the foot, which asks to lock first (signing leaves keys in memory, so
the write is never from a clean process) and the flow ends with the lock.
The file's row is green and the button reads "Written and checked" only for
a file of the same name and the same bytes (the SHA-256 kept when the file
was made, compared with the receipt's; nothing is hashed while drawing); a
file renamed or removed in Files shows a row with Make the file or Make again
instead. Files resets the chosen set when it is opened, as it always did, so
the tick does not outlast a visit to Files.

Not built: the other flows (the table below); "Written and checked" reached
in the flow (the lock ends it first); the tick kept on opening Files and
through the lock; the next steps in the pane; the pane settling back to its
session form after a write; the rows' own "Waiting for a stick"; the emphasis
on arrival; the file's green in the session pane after writing; and the code
drawn inline on a light square: its row opens the QR sheet when asked, as the
flow's finish did, and the sheet still makes the PNG. The pane's write counts
and writes only its own rows' files, but the visit's write sends everything
chosen; this must be closed before a second flow uses the pane, or before
a flow ends in a clean process.

**What this changes.** F319 puts Your choices at the left in
every flow and F296 makes the last step a card in the column: Your
choices stays at the left in every step but the last, which the session pane
takes over (the answers given are not lost: the pane's session form lists
them as the session's rows, and Back returns the pane to Your choices); the
last step is a card of the outcome in the column.

**Elevation and Material.** It stands in the flow's one card and has that
card's height, level 3 (§3, F331), none of its own.
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
settles back. The owner said yes to this trade-off (2026-11-22, and again
2026-11-24, knowing that the QR stays drawn until the pane settles back, Lock
or idle); it is built
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

## 16. Flows in progress, and the way out of a flow

Owner, 2026-11-23 and 2026-11-24.

- **A tool or check left before its end keeps its place, no secrets**: which
  key was picked, the step and the choices made. What was typed in and any
  result shown go when the tool is left. It stands in This session as a flow in
  progress, with its words and its Resume button, as the wallet, spend,
  message, vault and backup flows do.
- **A tool opened from Tools lights Tools** in the sidebar (owner, 2026-11-24).
- **No row under a tab** names a flow. A mark on the tab stands for it, where
  Files', Wallets' and Vaults' badges stand: the number of flows in progress
  there. Back up's is under Wallets.
- **A press on the tab** goes to its flow while it has one (with several, the
  one last left). Pressed while that flow is on screen it opens the tab's menu,
  the flow kept and its mark staying; pressed once more it returns to the flow.
- **The phone and the rail**: the chips that name the flows over every page stay
  until the session pane stands at those sizes. On the phone's bar a tab's own
  badge and its flow mark are two pills side by side at the icon's corner, the
  flow's in the primary colour. A check in progress has no mark there (the bar
  has no Check); Home's session rows show it.
- **Leaving a flow** shows the page it was begun from (Home, a search's results,
  a wallet's card, Files, a vault, a scan), and the tab's menu otherwise. A
  resumed flow goes to the page it was first begun from. §4 rule 5 says how that
  page then stands.
- **The search and the pills** leave a flow only by a press on a pill or on a
  search result, as a press on a tab does: the flow stays in progress. New
  wallet, pressed while one is being made, goes back to it (§4). Typing a
  search does not leave it (§4). Leaving puts every height back (§3).

## 17. Starting with files on the boot stick

Owner, 2026-11-23 and 2026-11-24. F130 stands: names and sizes until Import.

- Start opens on **Files**, the stick's files ticked. A stick with only the
  settings file goes straight to Home, as today, with no row for it. The rows
  show a guess from the name ("PSBT, by its name · 1.2 KB"), the contents only
  after Import.
- **Import works with the stick in**: the ticked files come into Files. A key
  whose file was on the stick loads as the stick is pulled, and the sheet says
  to remove it. Pulled before Import, it says "Put the stick back in to import".
- **One button**: "Import and load any keys" when a ticked file's name suggests
  that it may hold a key, else "Import" (owner, 2026-11-24): only names are known
  before the import. It does what today's one button does. Built.
- **A vault among the ticked files**: its passphrase is asked in the same tile,
  in place of the list, one vault after another, the second shown once the first
  is done.
- After the import, Home. **Later** goes to Home with the import waiting there;
  Files keeps its badge.
- **A second stick** while an import waits on Files: the import closes, Files shows
  the second stick, and the import waits on Home as "Import from FARADAY-STICK",
  which opens Files with the import again.
- **A small panel** (480 x 640, 320 x 480): Later and the button stand in a bar kept
  at the foot; the list scrolls above it.

## 18. Addresses are shown whole

Owner, 2026-11-23 and 2026-11-24. An address is never cut on screen, anywhere,
and a hover does not count. It is drawn in mono in groups of four, wrapped only
between groups and only where the room requires it.

- **Your choices and the other narrow places** (the pane, the phone's sheet, the
  Pi): the small mono type, the row growing in height, the first two groups and the last two in the
  full ink, the middle in the quieter ink, every character there.
- **A list row that names an address** (Check an address's list from a file, Sign
  a message's, Silent payments'): the grouped address is the headline; the row's
  name and detail stand under it.
- **A sentence that names an address** (a PSBT check's finding, a comparison's
  line): the sentence, then the grouped address on its own lines under it.
- **A typed address field** grows to the lines it needs, in groups of four.
- Decode's wide table wraps an address wider than its column (built).
- **Built:** Your choices' address row (the pane and the sheet; Sign a message's chosen address) the Spend panel's first recipient (`Ui::address_whole`), and Explore's Address row and Addresses list and Find results, in groups of four at every size. Not built: Sign a message's list row, the lists of Check an address and Silent payments.

## 19. What can be pressed, and its sign

F332 (owner, 2026-11-24) gives each kind one job: what opens ends in a chevron,
what chooses has a radio or a box, what acts has an icon and a verb and no
chevron, a fact reacts to nothing. DESIGN §4.13 has the kinds. The rows of the
session pane (§2), of the menu pages and of Settings follow it: one row part
draws the chevron, from whether the row opens. A row with a button keeps the
button and has the chevron too once its press opens its details (§2). Built: the wallet card's
backup line and chart boxes, the wallet's source tag, the sidebar's lock tag, the
phone's Settings rows, the network pill's arrow (wide windows only), "What you can do", the one
chevron token (`CHEVRON`, drawn by `Ui::row_chevron`) and the error tone of
Remove every key, Remove from session and Delete from the stick. **Open:**
the phone's Home rows (the loaded card in `nav/`), Files' rows, the session
pane's own chevron, Who signs' one filled button, Delete files' first press,
and the accent text of `plan/cards.rs` and `screens/entry.rs`; the three lints
below count them and only let them fall.

Where it changes what is drawn today:

- **A wallet's card:** the backup line, which opens Backups, has a chevron, on
  the phone too.
- **The wallet chart:** a box that opens has a chevron; the others have none.
- **Facts in a button's outline:** a wallet's "From Vault" and the sidebar's
  "Auto-lock off until backup complete" are plain tags.
- **The phone's Settings and Home:** a row that opens has the chevron, drawn by
  the shared row part.
- **The network pill** has the small menu arrow on wide windows only; the
  phone-size window and the Pi keep the pill as it was. The fact, "Testnet" on a
  spend's step, stays a plain tag.
- **Files:** the Write row keeps its tonal fill and has no chevron; "Written
  after Lock", now text in the accent, is a neutral tag.
- **"What you can do":** a row that opens a page or a flow has the chevron, at
  the one size that Tools, Check and the session pane have; a row that acts at
  once has none (owner, 2026-11-24).
- **Spend from a backup, Who signs:** one filled button, Continue; **Scan a
  key** is tonal.
- **Dangerous acts:** the first press of Delete files, Remove every key, Remove
  from session and Delete from the stick is in the error tone.

What a lint is to hold (`lint-components`):

- **The calls of `pill`.** `Ui::pill` and `Ui::pill_in` draw an outline and take
  no press, so a call outside the stick's pill (F142) is a fact in an outline.
  Today it would also find the locked Key card's Passphrase, "Set" or "Not set"
  (`stat_pill`). The sidebar's pill draws its own outline and goes with it.
- **Accent text in a status.** A status's words are drawn by one part, which
  takes a status and no accent; a status in `Role::Primary` outside it fails.
- **The chevron's size.** One token in place of `ROW_CHEVRON`,
  `MENU_ROW_CHEVRON` and the sizes written in place, taken by the row part
  alone. The one size is 20 (owner, 2026-11-25), the session pane's, so Tools
  and Check's chevrons are 20 too, not 24.

One filled button a step is not seen by counting lines: a test of a step's
presses holds it, Who signs' first. Review holds the rest.
