# One flow container

**Kind:** Spec
**Status:** Specified (F296); built. Every step flow is drawn by the container (`faraday/faraday-core/src/flow.rs`, `flow/`, `flow::contained`): only the current step's card, Your choices at the right or on the bar's button, the fade through between steps (`hub_layout::STEP_MS`). The old column that stacked finished steps above the open one is gone from the code.
**Open:** the live fade of a step that holds a secret (§3), built, awaiting the owner's approval after testing and a timing on the Pi

Every step flow (Make a new vault, Make a new wallet, Restore an existing
wallet, Spend from a backup, Make a new key, Back up's flows, the tools
drawn as step flows, and any new flow) is drawn by one container. Only the
current step's card shows, at one fixed place; moving between steps fades
through; Your choices stands at the right with each answer, and a press
on one goes back to its step. The decision is F296; this document is how
it is drawn and what each flow gives it.

## 1. What a flow declares

A flow hands the container, each frame, what only the flow knows:

- **Its steps**, in order: each step's title, and whether the count is
  known before the first (`Contained::count_fixed`) or grows as answers are
  given (Restore, Sign a transaction, Spend from a backup, the backup
  flow); Your choices then lists the steps reached so far and those the
  flow already knows will come, each "Not chosen yet". A flow may have steps before step 1, read by their title
  alone (`Contained::heads`); a GPG key's restore has none: its public
  key heads the plan card, which is Step 1 (GPG-PAPERKEY §4.2). A step's
  title and a row's label may be built by the flow (`Contained::title`,
  `Row::label`). Make a new key's steps include codex32's: its Scheme,
  Identifier, Entropy, entry, Worksheets, Strings and made cards are steps
  of the same flow, with step numbers of their own in Make a new key's
  order.
- **Which step is current**, and which are answered.
- **Each step's card**: the drawing of its controls in a given width,
  returning their height, as the `body` closure does today.
- **Each answer's line in Your choices**: a label and its value, in the
  mono face where it is data; a step not yet answered reads "Not chosen
  yet". A line never shows a secret (no word, no passphrase, no share):
  what it shows is what a step's summary may show.
- **Which step a row opens**: usually its own; one row may stand for
  several steps and open one of them (Make a new vault's size row stands
  for Opens on and Unlock cost until Customise, and opens the size).
- **What still fits.** When an earlier answer changes, the flow says, for
  each later answer, whether it still holds; the container keeps those
  that do and clears the rest. A flow that says nothing clears every later
  answer. Each flow's `still_fits` describes what its own code already
  keeps and clears; the container added no clearing to any flow.
- **Which steps are full width** (§5).
- **Where Back stops**: the step after a point that made something the
  flow cannot take back, and whether the current step is running (F104).
- **What the pane holds under the rows**, where a flow has more to show
  there: Make a new vault's summary of what will be made stays in the
  pane, and its Create vault button ends the last card, not the pane.
  Back up's map stands under its rows; Sign a transaction's review goes
  under the rows too, its facts plain lines with no press (Your choices'
  rows are the one way back to a step). The backup board's list is the
  exception: it stays in its own supporting pane at the right of the
  full-width step (§5), not under the rows.
- **A foot pinned in the pane** (`Page::pin`: a height and a drawing): a flow
  may keep one thing at the foot of Your choices' pane, outside what scrolls,
  under a hairline, with the rows and what is under them scrolling above it.
  Sign a transaction's signatures box and its primary stand there on the wide
  layout where the primary does not sign (Lock and write, and what else
  Finish shows); where it signs, the box stands after the facts in the
  pane's scroll, so no way to sign is in view before them. Where the region
  above the foot would hold under three rows, the pane draws the foot in the
  scroll after what is under the rows. The small panel has no pane and keeps
  its own foot.
- **A first card that is a checklist** (`Contained::checklist`): Back up's
  items are the rows, an item pressed fades to its card, Back from an item
  opens the checklist, and Back from the checklist leaves the flow. An
  item that waits on an earlier one takes no press (its row's `opens` is
  not among the flow's steps) and reads "After {the item it waits on}".
- **A note and a chip's press** (`Page::note`, `Page::chip_tap`): Sign a
  transaction's sequence line, and Back up's wallet chip, which is a press.
  Where the pair stands (Your choices at the card's left) the chip that
  names the wallet is not drawn: `Contained::pick_row` gives it as Your
  choices' first row ("Wallet: Savings"), with the chip's press and the
  place it opens its list from. Where no pane stands the chip stays.

## 2. What the container owns

- **The card's place.** The card's head stands where the first step's
  heading stands today: `PAGE_TOP_BAR` under the top app bar, after the
  flow's chip ("Then: …", a step's name) where it has one, and the wallet's
  where no pane stands, in the reading column
  (`COLUMN_MAX`, centred in the room left of the pane, or from the
  navigation's edge plus its margin in a flow whose steps hold wide
  content, LAYOUT §14). The place is the same for every step of a flow,
  so nothing moves when a step changes. The head is no press, except the
  sequence line and the wallet chip above; a press on the current step's
  card does nothing.
- **A tall card** scrolls within the page area under the top app bar, its
  head with it; a step opened starts at its top. The pane does not scroll
  with it.
- **The step's heading**: "Step N of M", or "Step N" while the count is not
  known, then the title, as the small panel's page per step reads today.
- **The fade** (§3) and Reduce motion.
- **Your choices** (§4): the pane on the wide layout, the bar's button and
  the sheet on every other.
- **Going back and forth.** The top app bar's way back, Escape and the
  mouse's back button open the step before the current one in step order,
  with its choices as they were, named after it; from the first step they
  leave the flow (F77, F104). The mouse's forward button opens again the
  step Back left. A row of Your choices opens its step; a row after the
  current one opens it when its answer and every one between still hold,
  so the person goes straight forward to where they were. A running step
  takes no press on a row and no Back; after a point the flow cannot take
  back, the rows before it take no press. A secret shown on a step is
  hidden when the step is left (F104), by any way out (Back, Next, the
  pane, a row of Your choices, the sheet): the container calls the flow's
  `Contained::hide_shown`, which every flow implements, and a step reopens
  with its eye off and what was typed kept. Cancel still leaves the flow
  (F51).
- **F103** stands: a card that asks one thing moves on by itself on the
  choice; the move is the fade.

## 3. The fade through

When the current step changes (a choice, Continue, Back, a row of Your
choices), the old card fades out to the page's surface and the new one
fades in from it, at the same place; where the card is raised (F331), the
card stays and its content fades out to the card's own fill and in from
it. As in Material's own fade through,
the whole fade runs on Material's full emphasized curve
(`tokens::EASE_EMPHASIZED`), the outgoing part taking the first 35% of
the eased progress (about the first 48 ms) and the incoming the rest
(`motion::step_fade`), its clock started as a change of screen's is;
its length, 300 ms (Medium 2), is a named constant in `motion.rs`
(`hub_layout::STEP_MS`), stated with how it was timed on the Pi (F238). Outside the
card's place (the bar's back label, a new row in Your choices) the frame
cross-fades over the same time.

It is built on today's kept-frame fade (`Faraday::fade_begin`,
`Canvas::blend_within`, `Faraday::fade_step`): the frame with the old card
blends to one with the card's place empty, then to the frame with the new
card. A step's blend touches only the bands that differ.

- **Built** for every step of Make a new vault (`Fade` in `fade.rs`): the
  fade runs on the kept frames as above, on a page scrolled it fades only
  the card in view under the top app bar, and the new step starts at its
  top. The incoming card does not scale up from 92% as Material's does:
  that is left out unless the Pi's timing allows it. The 300 ms is
  shorter than Material's 450 ms (F296 point 2).
- **Reduce motion**: the new card is there at once.
- **A secret on screen**: a card that shows or takes a secret (words, a
  passphrase, a share) fades like any other, but is drawn live: each step
  draws the outgoing card from the state it still holds at a falling
  opacity over the card's place, so no image of a frame with a secret is
  kept (the kept-frame blend is used only where neither frame held a
  secret). The outgoing step's secret state is released when its fade out
  ends (35% of the eased progress, about 48 ms), not at the press.
  A press that closes a sheet and moves the step together (the hold on a wallet's
  confirmation sheet, which makes the wallet and opens Check) fades the same way.
  `docs/MOTION.md` §3.5's rule that a fade does not start from a frame
  drawn while a secret was typed or shown holds still: there is no kept
  frame, the live draw being the way. This is a security trade-off: the
  secret stays held about 48 ms past the press, and the live draw costs
  drawing time on the Pi, to be timed per `docs/SPEED.md`. It reaches main
  only on the owner's approval after testing.
- **The live fade is built** (`Faraday::fade_begin`, `Fade::live_veil`,
  `flow::container`'s outgoing card): a Lock, the idle lock or Cancel
  during it ends it at the next frame drawn (a Lock clears the form
  itself). The release at the end
  of the fade out (the flow's `Contained::hide_shown` runs)
  runs at the first frame drawn after it, and the fade asks for a frame on
  every tick; the outgoing card takes no press, field or focus.
- No step card grows open and nothing glides to the open card
  (`docs/MOTION.md` §3.5): only the current step is drawn, at one place.

## 4. Your choices

**Wide layout** (the expanded rail: a large or extra-large window that is
not low; `ui.compact` false). A step whose card is a column stands as a
pair, one card: Your choices at the LEFT, `LIST_PANE` (360 dp), one
`SPACER` gap, and the step card at its right, up to `COLUMN_MAX`, the
pair centred in the room beside the navigation (placed by the pane container, `docs/HUB-LAYOUT.md`). The page
is the surface; the pair is one card (F331): `SHAPE_L`, filled
surface-container-low, at level 3, with a thin outline (OutlineVariant) at High
contrast only, a padding outside the pair's left and right edges and foot and a
padding above the content, which starts half a padding lower than a flat page's,
ending a padding above the window's foot where the pair is taller, so the pane
and the step scroll inside it. A full-width step on a window with a rail raises
its one column the same way. The step scrolls in the card's flat part, a
corner's radius inside its top and foot, so a moved frame moves nothing of its
shape. The card is drawn first, to where it ended last; a frame that finds
another end is drawn again before it is shown. In the pair a divider at the middle of the gap, inset at top and
bottom by the card's padding, is the only line between the two, and runs
to where the taller side ends: a pair ends level, tops and bottoms in line,
the pane never stretched to the window's foot. Where the pair does not
fit, and for the wide content that stands at the navigation's edge, the
pane stands at the right as Make a new vault's does: `side_panel_w` (31% of the
page area, 240 to 320 dp), with the same divider at the middle of the gap
before it. Guide's pane is its own card and draws none; a full-width step
and every one-column layout draw neither. The heading **Your choices** is in Label L at `SIDE_PANEL_TOP`, then one row per
step: the label, and under it the answer or "Not chosen yet", each row a
press in its 48 dp target. The current step's row is marked as current.
Explore and Derive is not a step flow: where the pair fits (`docs/HUB-LAYOUT.md`),
its Path page stands as the same pair with the loaded keys as the pick's rows
(`session_pane/pick.rs`, one choice, checked against Material's single-choice list row) in place of Your choices, the same divider and outline,
and the result in the card; the pane scrolls with the page. Addresses and
Find, the one-column layouts and a room too narrow for the pair keep today's
row of keys. Changing the key hides what its eyes show (`prune`).
A flow whose open step is a pick says so (`Contained::pick`: the question, then groups of `session_pane/pick.rs` rows, declared with `Decl`, `docs/SESSION-PANE.md` §4). Where the pair stands the pane draws those rows in place of Your choices, headed by the question and each group by its own heading, and the card keeps its title, Next (off until a pick) and the way back on the top bar; the pane then scrolls within itself like Your choices (the pair grows to the window before the list scrolls). After the pick, Your choices are in place again. On every other size the step's card draws the rows (`flow::pair_left` tells the screen which). The rows are labels and fingerprints only. Child seeds and passwords use it, as do Convert a seed's From step (the loaded keys, the aezeed held and the ways in; the open vaults' keys, the set's entry and Next stay in the card) the Krux code tool's Contents step (keys, then wallets, then typing; the typed text and Next stay in the card) and Sign a message's wallet step (every wallet, one that is not single-key or whose key is not loaded dimmed with its reason; the address and its number stay in the card), and My seed makes my wallet's Seed card, whose rows are the loaded keys, then the ways in (Type the words, Scan a SeedQR, codex32 and Other forms, which opens the rest in the pane) while the card lists its ways in; the card then draws none of them (on every other size the card keeps its ways and the keys under Other forms); Restore's held share files (checkbox rows, ticked as the restore has them, Rebuild below) and Spend from a backup's wallets (a row that opens, the chevron at its end, the keys' readiness and the shape as its line, the places a missing key may be under a wallet that cannot sign yet) draw the pane's rows (`session_pane/pick.rs`) in their own page, not in the pane: they declare no `Contained::pick`; the other flows' picks (Sign a transaction's wallet, the signers, the seed set and the rest of WORKSPACE §10's list) are not built.
A flow may list rows under a step's own (Sign a transaction lists its keys
under Signers, or, where there is no Signers step (one key, or no wallet known and one key
here), under Sign once it is reached, each marked "Signs here" or "Signed" where it is one
of these); they open that step, and take no press while it is the open one. A
key row draws its fingerprint in the mono face, whole, then the name in the
body face, which is what is cut to fit, then the mark whole in the `Primary`
role (the row's `mark`); a key row leaves out a name that could read as a mark.
A row whose answer is an address (the row's `address`) draws it whole in the small mono face under the label, in groups of four wrapped only between groups, the first two and last two in the full ink and the middle in the quieter one; the row grows in height and is never cut to fit.
The Lightning node key page stands as the same pair on the wide layout with
a key loaded: each loaded key's two sources (ldk-node and Core Lightning) are
the pick's rows, pressing what the buttons press, and the aezeed and
`hsm_secret` sources, the words and the node stay in the card; opened from
Convert, on a one-column layout or in a room too narrow for the pair the
sources are the buttons in the card (the rows are checked against Material's
single-choice list row, as Explore's). The page ends in one tail for every
case (the pair's lines and the scroll to a padding below the outline's foot).
The private key is hidden whenever the node it shows changes or goes: another
source, a file, an edit of the words or the passphrase, a read, a stick.
A pane taller than the window scrolls within itself (LAYOUT §6), so a low
window does not drop the pane. The scroll is one offset for every flow
(`Scroll::choices`), set back to the top when a flow is opened and by Back. A flow's
own right-hand panel is under its rows (§1). Its strings are the common
ones.

**Every one-column layout** (the menu, the navigation bar, the rail): the
top app bar carries a **Your choices** button among its trailing actions,
before the ?. It opens the list as a sheet (Faraday's sheet: the window's
width less a margin on compact, at most 560 dp centred otherwise), the
same rows, a press on one closing the sheet and fading to its step;
Escape, the scrim and the way back close it. The small panel's strip of
steps gives way to it. On the small panel the button is
an icon with the tooltip "Your choices"; elsewhere it is labelled **Your
choices**. Your choices stays at a flow's end; rows before a point the flow
cannot take back take no press. On a window too low for the pane, the pane
scrolls within itself.

On the wide layout a step whose card is a column stands as the pair
(the pane container, `screens/hub_layout/`): Your choices 360 dp, a `SPACER`
and the card's column up to 720 dp, centred in the room beside the
navigation. Guide's ? opens its supporting pane at the right (LAYOUT §6),
instead of the sheet, whenever the pair at its narrowest and the pane fit:
1536 dp of window. The pair then moves left only as far as the pane needs
(not at all when it already fits centred), and returns to its own place
when the pane closes. The pane has the pair card's top and foot. The pair slides between the two places as the pane
comes or goes, at the Speed setting and Motion style of a change of page
(`docs/MOTION.md` §3.5). To make
room, the card's column narrows from 720 to `FLOW_CARD_MIN` (560 dp) first
and then Your choices from 360 to `CHOICES_PANE_MIN` (240 dp), each only as
far as the room needs. Below that room Guide is the sheet. The rectangle
the fade treats as the card is the step card alone, not the pair.
The full-width steps and the steps
whose card stands at the navigation's edge (wide content) have their own
pane or none, and Guide is the sheet there.

**The two sizes named:**

- **480 × 640, the Pi's panel**: 268 × 358 dp, compact and short, the
  menu's layout. One card under the bar, its controls below its heading;
  the bar's back, the flow's name, Your choices, the ? and Cancel.
- **1366 × 768, a laptop**: a large window, not low: the expanded rail,
  the card in its column, Your choices at its left (about 1126 dp of page
  area, the pane 360 of it). The same laptop's half-screen window (683 dp)
  is medium: the rail, the card, the bar's button.

**The scanner in the pane.** On the wide layout, Check an address' **Scan an address** puts the scanner's frame (`screens/scan_frame.rs`) in the pair's pane in place of Your choices, the card holding the title and **Cancel** (`address_check/pane.rs`); it is Explore's pair at the same place. The scan is the scanner sheet's own state, so a code read, Cancel, Escape, Back, Lock and any press that leaves the page end it as the sheet does. A read card shows in the sheet, as today.

## 5. The two full-width steps

- **Make codex32 shares**, while its dice rolls, tracks and tree are on
  screen (the input panel, the sheets, and the dice sheet's column, LAYOUT
  §16); where the column stands the step's left side holds the window
  without scrolling, the worksheet's view taking the height left.
- **The backup board** (Back up's plan, a wallet's backup flow).

On these Your choices' pane is hidden and the step takes the whole room
beside the navigation; the bar's Your choices button stands, as on a
narrow window, and opens the sheet. The card's head keeps its place.
Leaving the step brings the pane back with the fade. The backup board's
list stays in its own supporting pane at the right of the step; Your
choices opens as a sheet there. A flow with a full-width step answers
`wide_content` true on every step, so its card does not move when the
step comes and goes. Every other dice worksheet (a seed's rolls in Make
a new key, Make a new vault's passphrase dice, `dice_entry`'s foot panel)
stays in the main column beside Your choices. New key's Dice words, Dice flips and Coins step
holds, in its card under the count, the strip of the word being made
(six slots of a roll's face and its bits, eleven of an entry's one bit for the flips, the 11-bit number, the word;
`keygen/dice_strip.rs`), then the pad, then Words so far
(`keygen/words_so_far.rs`), on the Pi's panel behind a "Words so far · n"
button that opens a page. After the last roll the strip clears and the
pad is greyed. The rule lines are in the Guide ("How dice make your words"), which the step's ?
and the strip's info button open.

## 6. Worksheets inside a flow (LAYOUT §14)

Wide content in a flow's card (a worksheet, a table, Spend's transaction)
takes the work width from the card's start to `SPACER` before the Your
choices pane, not to the window's edge; only §5's two steps run to the
area's edge less `MARGIN_WIDE` (`Contained::wide_content`, `Ui::flow_work`).
No rule of a step's rows runs the work width, since no card is stacked. A
flow whose steps can hold wide content keeps its card at the navigation's
edge plus its margin on every step, so the place does not move.

## 7. The end of a flow

The last step is the flow's result (a key made, a vault made, a wallet
restored, a transaction signed), drawn as a card like the others: what
happened and the next thing a person would do (`docs/FLOWS.md`: no flow
ends on a dead end). Another and Done stay where the flows have them. The
end card counts in "Step N of M" and F104's limits on Back apply to it
(Make a new key's made card included). A flow's foot (My seed makes
my wallet's, Read a Bitcoin Core wallet's) ends its last card.

A flow whose last step makes something names its take-away (`Contained::take_away`: the files made, by name in the outbox with the SHA-256 of their bytes, and the codes to scan). Where the pair stands (`flow::pair_left`) the pane draws it in place of Your choices (`flow/take_away.rs`): the heading "Take with you", each file a tick row of Files' (`screens/visit.rs`, `row`; the ticks are the visit's chosen set; a file no longer in the outbox is a row with Make the file or Make again), each code a row with a Show as QR button, and at the pane's foot one filled button, "Write to stick": off with nothing ticked, "Waiting for a stick" with no stick in. The step's card keeps the outcome and holds no file or code. Back to the step before brings Your choices again. No sheet opens by itself. Every other size keeps the card as it was. Sign a message uses it; the other flows' ends are not built.

For Sign a message the button asks to lock first (signing leaves keys in memory) and the flow ends with the lock, so "Written and checked" (a file of the same name and bytes read back) is not reached in the flow. Not built: that state reached in a flow, the next steps in the pane, the pane settling back after a write, the ticks kept through the lock, and the button writing only the files of its own rows.

## 8. The flows and how each maps

| Flow | Its screen | Its steps in the container | Full-width step | Under Your choices' rows, or beside |
|---|---|---|---|---|
| Make a new vault | `vault/screen.rs` | Size, Opens on, Unlock cost, Name and passphrases; the size row stands for the two after it until Customise | none | its summary and Create vault |
| Make a new wallet | `screens/create.rs` | Kind, then its steps; the count known once Keys is a step | none | a multisig's Keys step is a pick, one slot at a time (the first empty slot, or the filled one its Change opens again): the held keys at the left, one standing in another slot dimmed with its slot, then the open slot's other ways in (Make a new key or New words, Scan its xpub, Type or paste an xpub, Someone else adds it later); the card keeps each slot's name, state, Change and Clear, its xpub outputs and fields, and for the open slot the Inbox's files and the open vaults' keys with Unlock (worked out in the drawing's memo, which a pick cannot reach), and marks the open slot |
| Restore an existing wallet | `screens/restore.rs` | its steps, count not known; a typed word is never in a row | none | none |
| Sign a transaction | `screens/spend.rs` | its steps, count not known | none | the six review facts (Wallet, Sends, To, Fee, Change, Transaction ID), plain lines with no press; the sequence line as the card head's note |
| Spend from a backup | `spend_backup/screen.rs` | its steps, count not known; a card's own line at the top of its body | none | none |
| Make a new key, Make codex32 shares | `keygen/screen.rs`, `codex32/screen.rs` | its steps, codex32's cards among them with numbers of their own in its order; the made card is the end and counts | codex32's entry while it is current | none; the dice sheet's column is the step's own |
| Back up (a wallet's checklist and plan) | `screens/backup.rs`, `screens/backup/quick.rs` | the checklist as the first card, an item pressed fades to its card; Your choices lists the items with their state (Done, "After {item}", To do) | the board | the map under the rows, except on the plan card, which is the map; the wallet chip is a press |
| The backup flow (GPG keys, Secure Boot keys, a wallet's backup) | `backup_flow/screen.rs` | its steps, count not known; a GPG key's restore opens on the plan card (its public key at the top), Step 1 | the board | the board's list, in its own pane at the right |
| Sign a message | `screens/message.rs` | its steps | none | none |
| Derive with BIP-85 | `bip85/screen.rs` | its steps | none | none |
| Convert a seed | `convert/screen.rs` | From, To, Settings where asked, Result | none | none |
| Silent payments | `silent/screen.rs` | its steps | none | none |
| Vanity address | `vanity/screen.rs` | its steps | none | none |
| Check a signer | `decode/check_screen.rs` | its steps | none | none |
| Check a multisig setup | `multisig_check/screen.rs` | its steps | none | none |
| My seed makes my wallet | `seed_check/screen.rs` | its steps, a foot ending the last card | none | none |
| Read a Bitcoin Core wallet | `core_wallet/screen.rs` | its steps, a foot ending the last card | none | none |
| Check a timestamp | `ots_check/screen.rs` | its steps | none | none |
| Krux code | `kef_tool/screen.rs` | its steps | none | none |
| Hashes (Tools), on a window that is not compact | `faraday/faraday-core/src/tools/screen.rs`, `faraday/faraday-core/src/tools/flow.rs` | Input, Result | none | none |
| Encodings (Tools), on a window that is not compact | `faraday/faraday-core/src/tools/screen.rs`, `faraday/faraday-core/src/tools/flow.rs` | Input, Result | none | none |
| Descriptor (Tools), on a window that is not compact | `faraday/faraday-core/src/tools/screen.rs`, `faraday/faraday-core/src/tools/flow.rs` | Input (on the pair, a pick at the left: the held files it takes, Type it, Scan), Result | none | none |
| Miniscript (Tools), on a window that is not compact | `faraday/faraday-core/src/tools/screen.rs`, `faraday/faraday-core/src/tools/flow.rs` | Input (on the pair, a pick at the left: the held files it takes, Type it, Scan), Result | none | none |
| Convert an xpub (Tools), on a window that is not compact | `faraday/faraday-core/src/tools/screen.rs`, `faraday/faraday-core/src/tools/flow.rs` | Input (on the pair, a pick at the left: the held files it takes, Type it, Scan), Result | none | none |
| Decode a Lightning invoice, on a window that is not compact | `faraday/faraday-core/src/invoice/screen.rs`, `faraday/faraday-core/src/invoice/flow.rs` | Input, Result | none | none |
| Decode a transaction, on a window wide enough for the container's pair | `faraday/faraday-core/src/decode/screen.rs`, `faraday/faraday-core/src/decode/flow.rs` | Transaction (the pick at the left, with Type it among its ways in and its field in the card, then Result in the card) | none | none |
| Check a signed message, on a window wide enough for the container's pair | `faraday/faraday-core/src/screens/check_message.rs` | Message (the pick at the left, then Result in the card) | none | none |
| Nonce check, on a window wide enough for the container's pair | `faraday/faraday-core/src/screens/nonce_check.rs`, `faraday/faraday-core/src/decode/nonce_flow.rs` | Nonce check (the file and one row per input at the left, the check in the card; with nothing to check, the ways in at the left) | none | none |
| Units (Tools), on a window that is not compact | `faraday/faraday-core/src/tools/screen.rs`, `faraday/faraday-core/src/tools/flow.rs` | one card (its title alone, no step count) | none | none; the pane lists no rows and its foot holds the Add an amount button |

The screens are under `faraday/faraday-core/src/`.

## 9. What changes when it is built

Built; nothing is left to build but the live fade's Pi timing and the
owner's approval (§3). What the build changed is in git. Two rules remain
for any change that touches a flow:

- `docs/SECRET-LEAKS.md` §5: for a step change inside a flow its check
  reads "after the fade out ends", not at the press; `docs/MOTION.md`
  §3.5's rule that a fade does not start from a frame drawn while a secret
  was typed or shown holds still, no frame being kept.
- Tests, of what a person notices: a pressed row opens its step; a later
  answer that still fits is kept after an earlier one changes and one that
  does not is cleared; Back and the rows keep F104's limits; the bar's
  button opens the sheet on a one-column window and on a full-width step.

## 10. Questions for the owner

The owner answered them on 2026-11-19; sections 1 to 9 hold the answers.
