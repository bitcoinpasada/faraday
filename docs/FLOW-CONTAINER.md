# One flow container

**Kind:** Spec
**Status:** Specified (F296); built. Every step flow is drawn by the container (`faraday/faraday-core/src/flow.rs`, `flow/`, `flow::contained`): only the current step's card, Your choices at the right or on the bar's button, the fade through between steps (`motion::STEP_FADE_MS`). The old column that stacked finished steps above the open one is gone from the code.
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
- **A first card that is a checklist** (`Contained::checklist`): Back up's
  items are the rows, an item pressed fades to its card, Back from an item
  opens the checklist, and Back from the checklist leaves the flow. An
  item that waits on an earlier one takes no press (its row's `opens` is
  not among the flow's steps) and reads "After {the item it waits on}".
- **A note and a chip's press** (`Page::note`, `Page::chip_tap`): Sign a
  transaction's sequence line, and Back up's wallet chip, which is a press.

## 2. What the container owns

- **The card's place.** The card's head stands where the first step's
  heading stands today: `PAGE_TOP_BAR` under the top app bar, after the
  flow's chip ("Then: …") where it has one, in the reading column
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
fades in from it, at the same place. As in Material's own fade through,
the whole fade runs on Material's full emphasized curve
(`tokens::EASE_EMPHASIZED`), the outgoing part taking the first 35% of
the eased progress (about the first 48 ms) and the incoming the rest
(`motion::step_fade`), its clock started as a change of screen's is (`motion::FADE_MS`);
its length, 300 ms (Medium 2), is a named constant in `motion.rs` beside
`FADE_MS`, stated with how it was timed on the Pi (F238). Outside the
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
not low; `ui.compact` false). The pane stands at the right of the page
area, as Make a new vault's does today: `side_panel_w` (31% of the page
area, 240 to 320 dp), on the surface with a divider at its start (a line from the top to a page
margin above the window's foot, and the same line the same spacer left of
the card's column, so the card is framed on both sides, drawn only where it
stands at least a spacer clear of the page's left edge, nearer which the
navigation's edge frames the card; Guide's pane is its own card and draws
none; a full-width step and every one-column layout draw neither), its
heading **Your choices** in Label L at `SIDE_PANEL_TOP`, then one row per
step: the label, and under it the answer or "Not chosen yet", each row a
press in its 48 dp target. The current step's row is marked as current.
A pane taller than the window scrolls within itself (LAYOUT §6), so a low
window does not drop the pane. The scroll is one offset for every flow
(`Scroll::choices`), set back to the top when a flow is opened. A flow's
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

On the wide layout a step whose card is a column stands with Your choices as
one unit: the card's `COLUMN_MAX` column, a `SPACER` and the pane, centred in
the room beside the navigation (`flow/unit.rs`). Guide's ? opens its
supporting pane at the right (LAYOUT §6), instead of the sheet, whenever the
unit at its narrowest and the pane fit: 1536 dp of window. The unit then
moves left only as far as the pane needs (not at all when it already fits
centred; about 140 dp at 1920 dp), and stays there when the pane closes,
until the flow ends; a new flow starts centred. The move shows through the
frame's cross-fade that a pane coming or going has. To make room, the card's
column narrows from 720 to `FLOW_CARD_MIN` (560 dp) first and then Your choices
from 320 to `CHOICES_PANE_MIN` (240 dp), each only as far as the room needs.
Below that room Guide is the sheet. The full-width steps and the steps
whose card stands at the navigation's edge (wide content) have their own
pane or none, and Guide is the sheet there.

**The two sizes named:**

- **480 × 640, the Pi's panel**: 268 × 358 dp, compact and short, the
  menu's layout. One card under the bar, its controls below its heading;
  the bar's back, the flow's name, Your choices, the ? and Cancel.
- **1366 × 768, a laptop**: a large window, not low: the expanded rail,
  the card in its column, Your choices at the right (about 1126 dp of page
  area, the pane 320 of it). The same laptop's half-screen window (683 dp)
  is medium: the rail, the card, the bar's button.

## 5. The two full-width steps

- **Make codex32 shares**, while its dice rolls, tracks and tree are on
  screen (the input panel, the sheets, and the dice sheet's column, LAYOUT
  §16).
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
stays in the main column beside Your choices.

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

## 8. The flows and how each maps

| Flow | Its screen | Its steps in the container | Full-width step | Under Your choices' rows, or beside |
|---|---|---|---|---|
| Make a new vault | `vault/screen.rs` | Size, Opens on, Unlock cost, Name and passphrases; the size row stands for the two after it until Customise | none | its summary and Create vault |
| Make a new wallet | `screens/create.rs` | Kind, then its steps; the count known once Keys is a step | none | none |
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
