# The caret in every text field: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/TEXT-CURSOR.md` on 2026-10-30, when the last
of its work was built. Each keeps its original number; the source keeps
a one-line stub under the same heading.

## The header (from docs/TEXT-CURSOR.md)

**Kind:** Spec
**Status:** Specified, partly built: every field of §2 but two (§4, batches C1 to C4; `faraday/faraday-core/src/edit.rs`, `Ui::typed_lines`, `Ui::draw_typed`)
**Open:** the words of a list typed on its own keyboard (`screens/parts.rs` `keyed_cells`) and Make codex32 shares' string typed back (`codex32/screen.rs` `typed_back`) take no caret yet (§6 item 1, planned).

## 2. Survey (2026-10-30)

Read from the code at main `2be654675`. "Keys" is Left, Right, Home and
End with Backspace, Delete and a character at the caret; "Press" is a
press placing the caret.

| Field | Where it is drawn | Keys | Press | Why not |
|---|---|---|---|---|
| Vault passphrase, Unlock, Import, Create vault's phrases, its name, an entry's one-line fields, the prompt, the dice box | `vault/screen.rs` `text_box`, `secret_box` | yes | yes | |
| BIP-39 passphrase in New key and Create, Entry's passphrase | `screens/parts.rs` `dots_field` | yes | yes | |
| Cosigner keys and a custom path (Restore, Family) | `seeds/screen.rs` | yes | yes | |
| A wallet's new name (Wallets) | `screens/wallets.rs` | yes | yes | |
| Read a seed's field | `seeds/read.rs` via `seeds/screen.rs` | yes | yes | |
| A vault's search bar | `vault/screen/search.rs` | yes | no | not in `field_action` |
| A place's name on the backup board | `board/screen.rs` | yes | no | not in `field_action` |
| A file's new name (Files, Rename) | `file_menu.rs` | yes | no | not in `field_action` |
| A wallet's name at Register | `register.rs` | yes | no | not in `field_action` |
| Sweep's key | `seeds/sweep.rs` | yes | no | not in `field_action` |
| The roller's count | `fresh/roller` via `vault/screen.rs` | yes | no | not in `field_action` |
| Set time's date and time | `screens/set_time.rs` via `text_box` | no | no | not a `typing_target`, not in `field_action` |
| Tools' Find a tool | `screens/catalog.rs` | no | no | drawn as text and an end caret; `catalog_key` first |
| A word list's find | `wordlist/screen.rs` | no | no | drawn with `caret_char` at the end |
| Explore's path | `explore/screen.rs` | no | no | drawn with `caret_char` at the end |
| Explore's pages' field | `explore/pages.rs` | no | no | drawn with `caret_char` at the end |
| BIP-85's index | `bip85/screen.rs` | no | no | drawn with `caret_char` at the end |
| Vanity's prefix | `vanity/screen.rs` | no | no | drawn as text and an end caret |
| The Inbox's potential passphrase (masked) | `screens/sheets.rs` | no | no | drawn as text and an end caret; `potential_key` first |
| Lightning's aezeed passphrase (masked) | `lightning/screen.rs` | no | no | drawn with `caret_char`; a dot per byte, not per character |
| A chart sheet's field | `glance/sheet.rs` `field` | no | no | drawn as text and an end caret; `chart_key` first |
| Sign a message's text (several lines) | `screens/message.rs` | no | no | `caret_moves` is false: wrapped, `caret_char` |
| A vault entry's notes and other fields of several lines | `vault/screen.rs` `text_area` | no | no | `caret_moves` is false: wrapped, `caret_char` |
| Entry's share or key typed (not words) | `screens/entry.rs`, `compact/screen.rs` | no | no | wrapped, `caret_char`; not a `typing_target` |
| Tools' input | `faraday/faraday-core/src/tools/screen.rs` | no | no | drawn with `caret_char` at the end |
| Lightning's 24 words | `lightning/screen.rs` | no | no | drawn with `caret_char` at the end |
| A GPG key's paperkey line | `backup_flow/gpg_restore_screen.rs` | no | no | wrapped, `caret_char`; ends itself at its last digit (§6) |
| Seed words in a grid (Entry's words, Restore, Check my copy) | `screens/parts.rs` word grid, `keyed_cells` | no | no | cells, not one line of text (§6) |
| A word typed on a list's own keyboard | `compact/screen.rs` `keyed_head` | no | no | a prefix of a few keys (§6) |
| Codex32's boxes | `codex32/screen.rs` | no | no | a box per character (§6) |
| Make codex32 shares' string typed back | `codex32/screen.rs` `typed_back` | no | no | drawn whole, in groups of four |
| The Starter | | | | has no text field |

## 4. Batches

- **C1, the fields that draw with `Ui::typed` already.** `field_action`
  names a vault's search, the backup board's place name (a first press
  still opens it with all of it selected, `docs/BACKUP-FLOW.md` §19.18;
  a press after places the caret), Files' rename, Register's name,
  Sweep's key and the roller's count; Set time's date and time are
  `typing_target`s. Test: in Set time, the arrow keys and a press
  between two digits put the caret where the next digit goes.
- **C2, single-line fields drawn as text.** Tools' Find a tool, a word
  list's find, Explore's path, BIP-85's index, Vanity's prefix, the
  Inbox's potential passphrase, Lightning's aezeed passphrase and a chart
  sheet's field are drawn with `Ui::typed`, are `typing_target`s read by
  `typing_text`, named by `field_action`, and reached by `edit_key`
  before their own handlers. The fields that take typing with no press
  to focus them are pressed with `Action::TypeHere` (`edit::Here`),
  which does nothing but place the caret; the potential passphrase and
  the aezeed's keep their own presses. Explore's `m/` and Vanity's fixed
  characters are drawn before the field and are not in it. Explore's
  first index on Addresses stays typed at its end: it is a number
  stepped a page at a time, and each key derives a page of addresses.
  Tests: type, Left twice, type: inserted there, in Find a tool, and a
  press then Delete; in the aezeed's masked passphrase an edit at the
  caret, and two passphrases of one length with the caret at one place
  drawn the same.
- **C3, fields of several lines, and the rest typed at their end.**
  `Ui::typed_lines` in `faraday-ui` wraps as `wrap_as` does and notes
  each line's edges with its rows (`FieldAt::band`); a press finds its
  line, then the edge; Up and Down in `edit_key` go to the nearest edge
  on the line above or below, and on the first or last line the key
  goes on as before. Sign a message's text (pressed with
  `Here::Message`, which starts typing as **Type** does, so **Stop
  typing** is a button again) and a vault entry's fields of several
  lines draw with it, and Add a key's string on a small panel; Add a
  key's string on a wider window (`Here::Entry`, which on a small panel
  moves typing off the passphrase), Tools' input (`Here::Tools`) and the aezeed's words
  (`Here::Aezeed`, which moves typing off its passphrase) are one line
  and draw with `Ui::typed`. `Ui::typed` now counts each edge from the
  advances in one pass, as `Ui::measure` adds them, where it measured
  every start of the text: a long descriptor in Tools costs a pass, not
  a pass a character. Add a key's string takes no press to place the
  caret, on any window, and Vanity's prefix none on a small panel: the
  string's field is 44 dp tall (40 on a small panel), under Material's
  48 dp target, and the prefix's card head reaches over its field
  there (§6 item 4). On a small panel the page keeps the string, now
  a focused field, above the keyboard; when codex32's correction opens
  in its place the page goes back to its top. Tests: a message wraps, a press on a later line
  and Up land the next key where expected; a codex32 string takes an
  edit at the caret by the keys.
- **C4, the owner's answers (§6).** Add a key's string field and the
  small panel's Vanity prefix field are 48 dp tall (`tokens::TOUCH`),
  the Vanity field standing under its card head's own 48 dp, and the
  small panel's form chips keep their room above the string, so a press
  places the caret in both; the string's field on a small panel is a
  touch target tall at the least and wraps once (`Ui::wrapped_as`, then
  `Ui::draw_typed`). A GPG key's paperkey line is a field like the rest
  (`edit::GPG_LINE`): the caret moves in it, Up and Down a line where it
  wraps, and an edit inside it holds the line from ending part way
  (`RestoreLive::held`); the moment the line is whole and its CRC holds
  it is taken and the next asked for, typed straight through or fixed
  in the middle; a line not yet right stays. Add a key's words and Check
  my copy's words typed back take the caret as a place in the words
  typed: Left and Right move it within a word and on past its edge into
  the next cell, a press on a cell puts it there (each cell's characters
  noted, the cells a touch target apart), and an edit is made in place
  (`edit::splice_text`), what follows held in a `SecretText`, with no
  word completed part way through; a word shown as bullets shows the
  caret at its end. The codex32 correction's re-entry boxes take a caret
  (`Correction::caret_to_box`): Left, Right, Home and End move it among
  the boxes to type again, a character goes in the box at the caret in
  place of what was there and the caret moves on, Backspace clears the
  box before it and Delete the box at it; a group of four boxes is
  pressed whole, its rows a touch target tall (`CORRECT_ROW_GAP` is
  8 dp), and the box nearest the press takes the caret. Tests: the
  paperkey line fixed in the middle is checked with no Enter; the words
  grid by the arrows across cells and a press in a cell; the re-entry's
  caret back a box and a box typed over; Add a key's string by a press.
- **C5, the last two fields.** A list typed on its own keyboard
  (`screens/parts.rs` `keyed_cells`; on a small panel `keyed_head` and
  `keyed_taken`): the caret moves within the keys of the word being
  typed, and past its edge a word taken earlier is opened again in its
  cell (`WordEntry::open_word`), its keys the word being typed, the words
  after it held aside and put back as they were once it is taken again
  (`WordEntry::close_word`, `commit`); a word opened and left unchanged
  is taken again as it was, one changed stays until a word is pressed
  among its candidates. An edit inside the keys is taken only where some
  word still starts so (`WordEntry::edit_keys`); nothing is completed
  part way through. Check my copy's list, drawn as bullets, keeps the
  caret at a word's end and its word opened as bullets. A press on a
  cell opens its word (cells numbered `word_entry::STRIDE` apart in the
  press's index; the rows a touch target apart). Make codex32 shares'
  string typed back (`codex32/screen.rs` `typed_back`) is a field
  (`Here::Codex32Back`): its edges where each character stands in the
  groups of four, a press, the caret drawn there, an edit made through
  its keys and the copy compared once it is whole. Tests: a word taken
  earlier opened by Left and by a press, a key taken off and another
  word pressed, the words after it kept; a box typed back wrong fixed
  where the caret is put, the copy then the same.

## 6. Questions for the owner

Answered by the owner, 2026-10-30, and built in C4:

1. **Seed words in a grid** (Entry's words form, Restore, Check my copy,
   a list's own keyboard): Left and Right move the caret within a word
   and past its edge on to the neighbouring cell; a press chooses a cell
   and places the caret in it. Built for the grids of words typed in
   Latin letters (Add a key, Check my copy); not yet for a list typed on
   its own keyboard, whose words are taken from its candidates.
2. **Codex32's per-character boxes:** the caret moves between boxes by
   the arrows, Home, End and a press; typing replaces at the caret as
   the boxes' rules allow. Built for the correction's re-entry; Make
   codex32 shares' string typed back is not built yet.
3. **A GPG key's paperkey line:** the caret moves within the line; the
   line goes on to the next the moment it is right (whole, its CRC
   holding), typed straight through or fixed in the middle, with no
   Enter; a line not yet right stays.
4. **Presses under 48 dp:** Add a key's string field and the small
   panel's Vanity prefix field grow to 48 dp at the source, and a press
   places the caret in both.
