# The session pane

**Kind:** Reference
**Status:** Built: the component (`screens/session_pane/`) and Home's wide card (F323). The phone-size window and the Pi keep their own box (`docs/HOME.md` §4). Wallets' own mode (§1: Waiting, then Keys) stands at the left of Wallets' tile and of a wallet's page on the wide layout. The pick of the flows is a mode of it (§4); The checkbox row (`pick.rs`, `Take::several`: a checkbox in the radio's place, ticked, dimmed with its reason) is built and Restore's share files and Spend from a backup's wallets draw their rows from it (a row that opens ends in the chevron, every row wraps its head and line); not built: the pane in Restore and Spend from a backup (only their rows are shared, in the flows' own pages), in Save to a vault, the backup board, Silent and Vanity (`docs/WORKSPACE.md`).

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

Three groups stand under small headings (Title Small), in this order, Keys
first and Waiting last as the band across the card's foot (`session_pane/band.rs`: a hairline
over it, outside the part that scrolls, at most a third of the pane
where the pane does not stand whole, its rows in columns on a wide card,
scrolling within itself, `Slot::HomeBand`); a group
with no rows is not drawn:

1. **Keys.** Every key of `session.keys.all()`, no cap: its fingerprint, then its
   name in full (wrapping), its passphrase mark, and the wallet that uses it
   ("Used by {wallet}", else "No wallet uses it"). A press opens the key (on Tools, Check, Wallets and Home in the page's second
   pane: `row_open`, `screens/row_details.rs`). There
   is no All keys row and no Make a wallet button.
2. **Wallets.** Every loaded wallet, a wallet that needs something with the
   warning icon and colour; Backups last, while something is not backed up. A
   wallet's row has no button.
3. **Waiting** (the band). The jobs of `docs/HOME.md` §2, each one row, always, with keys
   and wallets loaded too: the spend under way, a backup not finished, one row
   for each PSBT in Files that waits to be signed (fewest signatures first),
   an import put off, the last write ("Written to {stick}"), one row for each
   locked vault file with Unlock, a vault changed since it was written, a stick
   waiting. Then each flow in progress not already one of them, with Resume,
   and the files to write.
**Wallets' mode** (wide layout): two groups, Keys, then Waiting as the band (a waiting PSBT with Sign,
the wallet files held, the flows in progress); the wallets are the
page's tile, so the pane has no Wallets group, and a key in no wallet stands
in its Keys group (`screens/session_pane/page_mode.rs`, `Mode::Wallets`).

A row with a button stands so: the button under the row's lines at the text's
start, as the PSBT's Sign does, its own press. Where the page has the pane's
details (Tools, Check, Wallets on a window that is not compact, Home on the wide layout; Files draws its own rows of the Inbox so, `docs/WORKSPACE.md` §7 point 1) a held file's
row, a PSBT's included, opens its details on a press anywhere else of the row
(on Files the tick box only ticks, the rest of the row opens) and ends in the chevron too; a flow in progress's row has none of its own
press. On the phone and the Pi the button is the row's only press. Every lead's button stands so (Carry on, Finish, Unlock, Write,
Open, Sign), in the Waiting group, one column, never across the card.

## 2. Home's card

The card is at most `MENU_PAGE_W` wide, centred, `HOME_CARD_GAP` under the top
bar's row, raised at level 1 on surface-container-low, with a thin outline at High contrast
(F331). Its heading is
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
Tools and Check, Waiting the band at the foot (the lead with its button, the waiting PSBTs, an import put off and the locked vault files; not backups, files to write, the last write or open vaults) and In progress (each flow in progress that is not the band's lead) a group of its own after the others, scrolling within
(`Slot::HomeBox`) where the page's height cuts it. Vaults' mode is Keys, then Wallets, with Waiting at the foot (a waiting PSBT with
Sign, flows in progress); its rows keep their own presses, as Vaults has no
details yet, and carry the marks of `session_pane/vault_marks.rs` while exactly one vault is open: "In a vault", "Words in a vault" (a key whose words the vault holds without its passphrase, or with one it does not have) or "Not saved" at the row's end, and Save under a row not saved when
that vault can be written (the row's own press moves to the row's chevron).
The press is the vault's own SaveKey or SaveWallet, whose checks stay. Open vaults, files to write
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

## 4. The pick

A flow's pick (`docs/WORKSPACE.md` §2, "The pick") is a mode of the pane
(`session_pane/pick.rs`, `lint-components` `pick-own-rows`). A flow
declares with `Decl`: the question, which kinds it takes (`keys`, `wallets`,
`files`), and for each item a `Take` (chosen or not, what a press does, a
refusal giving its reason or none), and its ways in, the only rows the flow
makes itself. The pane lists the items, never the flow: Keys, Wallets, Files,
then the ways in ("Other ways in" while anything is listed, "Ways in" when
nothing is). A key row's head is its fingerprint and its line is the one
`key_line` says for the pane (name, passphrase, codex32 mark, the wallet that
uses it); a refused row is dimmed, shows its reason as the line and takes no
press, by pointer or keyboard. The row is the Material single-choice list
row, a radio in the icon's place, the chosen one in the secondary container
(`PickRow`, `Kind`). The flow container's Your choices (`flow/choices.rs`) and
the narrow cards of BIP-85, Check a signed message, Decode, Invoice and Convert's keys draw the same groups through `draw_groups` (Make a new wallet, Sign a message, KEF, My seed, Explore and Lightning keep buttons of their own there). A pick that takes
several has a checkbox (`Take::several`; Restore's share files); one that opens an item has the chevron (`Take::opens`; Spend from a backup's wallets); every row wraps its head and line. Restore and Spend from a backup draw these rows in their own pages: the pane's form there is not built.
