# The session pane

**Kind:** Reference
**Status:** Built: the component (`screens/session_pane/`) and Home's wide card (F323). The phone-size window and the Pi keep their own box (`docs/HOME.md` §4). Not built: the pane on the menu pages and in the flows (`docs/WORKSPACE.md`).

The session pane is the one component that draws what the session holds as
rows: the keys and wallets loaded (`app.session`), the flows in progress, the
files Faraday holds and what waits for the person. No page keeps a list of its
own (`lint-components`, `session-pane-own-rows`). Today Home's card is its one
form on a wide window; the pane's design for every page is `docs/WORKSPACE.md`.

## 1. The rows as data

`session_pane/rows.rs` builds the rows once, when the section is built, never
while drawing, and holds no geometry, so a page that is not Home can build the
same rows without Home's columns. A row (`SessionRow`) has a leading icon, a
headline, a second line, what a press opens and, for a kind with one obvious
next step, one button.

Three groups stand under small headings (Title Small), in this order; a group
with no rows is not drawn:

1. **Waiting.** The jobs of `docs/HOME.md` §2, each one row, always, with keys
   and wallets loaded too: the spend under way, a backup not finished, one row
   for each PSBT in Files that waits to be signed (fewest signatures first),
   an import put off, the last write ("Written to {stick}"), one row for each
   locked vault file with Unlock, a vault changed since it was written, a stick
   waiting. Then each flow in progress not already one of them, with Resume,
   and the files to write.
2. **Wallets.** Every loaded wallet, a wallet that needs something with the
   warning icon and colour; Backups last, while something is not backed up. A
   wallet's row has no button.
3. **Keys.** Every key of `session.keys.all()`, no cap: its fingerprint, then its
   name in full (wrapping), its passphrase mark, and the wallet that uses it
   ("Used by {wallet}", else "No wallet uses it"). A press opens the key. There
   is no All keys row and no Make a wallet button.

A row with a button carries no chevron: the button stands under the row's
lines at the text's start, as the PSBT's Sign does; the button is the press, and
the row has none of its own. Every lead's button stands so (Carry on, Finish, Unlock, Write,
Open, Sign), in the Waiting group, one column, never across the card.

## 2. Home's card

The card is at most `MENU_PAGE_W` wide, centred, `HOME_CARD_GAP` under the top
bar's row, raised at level 3 on surface-container-low (F315). Its heading is
"This session" (Title Large). The groups stand in as many columns as fit of at
least `SESSION_COL_MIN` (336 dp), `SPACER` (24 dp) apart; each group stands
whole, and the next one goes into the shortest column. The card is as tall as
its groups. Where the window is too short, the card's own box scrolls
(`Slot::HomeBox`; the heading, the search row and the top bar stay) and no row
is dropped.

Nothing loaded: the card stands at its place with what is real of Waiting, and
with none of it the one line "Nothing loaded". It has no Make or Restore row.

### The pane on Tools and Check

On the wide layout Tools and Check are placed by the pane container (`docs/HUB-LAYOUT.md`)
(WORKSPACE §4). Stacked, the page's tile stands under this card, which is
Home's, with Home's rows (`stacked_card`). Beside, the pane is the page's own
mode (`session_pane/page_mode.rs`): "This session" in Title M, then each
group's small heading and its rows in one column, in WORKSPACE §2's order for
Tools with a PSBT waiting, Tools with none and Check, scrolling within
(`Slot::HomeBox`) where the page's height cuts it. Open vaults, files to write
and Backups are not in it, and no row of it is also in the tile.

A file row of the page's own mode carries the row button of WORKSPACE §2's
table: the first entry of the file's shared list (`items::offer(..).button()`),
the button being the row's press as above. The buttons are worked out once a
frame for every file held (`View::inbox_buttons`, the drawing memo, emptied
when the frame ends), so a lock, a load or a change of the files shows in the
next frame; the offer reads the Inbox's and the spend's kept answers. A file
whose kind has no button keeps its row, which opens Files. A button whose
words are made with a count (the shares' Restore) is not drawn, and a PSBT
keeps the rows of Waiting. A wallet's row still has no button.

### Where it stands, for the change of page

The one drawer of the pane's surface (`card_surface`) reports its rectangle
as the pane's place (`Ui::pane`) wherever it draws: Home's card, the stacked
card and the page's own mode. Where a flow's pair stands, the container
(`flow/container.rs`) reports the left part, Your choices or, while the open
step is a pick (`Contained::pick`), the pick's rows, as the pane's place. A page without the pane, a flow with no pair and a measuring draw
(`menu::measure`) report none. The change of page glides the pane between
the two places (`docs/MOTION.md` §3.5); a place is only a rectangle, and a
frame drawn while a secret was typed or shown keeps no image to glide from
(`docs/SECRET-LEAKS.md` §5).

## 3. The phone and the Pi

`loaded_sections` draws the one-column box of `docs/HOME.md` §4 from the same
module through its own rows (`compact_rows`): what needs the person, the keys
no wallet uses with their Make a wallet button, and All keys where keys are cut.
