# The caret in every text field

**Kind:** Reference
**Status:** Built (`faraday/faraday-core/src/edit.rs`, `Ui::typed`, `Ui::typed_lines`, `Ui::draw_typed`)

In every text field of Faraday (the device app, on the stick, the Pi and
the desktop) the caret goes where the person puts it: Left and Right move
it one character, Home and End to the field's ends; Backspace takes the
character before it and Delete the one after; a character typed goes in
at it; a press or a tap in the field puts it between the two characters
nearest the pointer. This is Material 3's text field (m3.material.io,
Text fields: "Cursor", the insertion point placed by a tap or the keys),
first built for some of the fields in `docs/history/NEW-WALLET.md`
§13.1 and §13.2 and now in every one: the names, finds, paths and
passphrases, the fields of several lines, the grids of seed words, the
codex32 boxes and a GPG key's paperkey lines. The Starter has no text
field.

## 1. How it works now

`faraday/faraday-core/src/edit.rs` holds the caret and the selection
(`TypingArea`, `faraday_ui::Edit`) of the field typing goes to, which the
glue names by the action that focuses it (`Faraday::typing_target`) and
whose text it reads (`Faraday::typing_text`). A field drawn with
`Ui::typed` puts the caret at `Edit`'s place and notes where it drew each
character's edge (`faraday_ui::FieldAt`); a press on the field is turned
into a character index at the event from the last frame's edges
(`Faraday::field_press`), never while drawing. An edit inside the text is
made through the field's own keys (`Faraday::splice`): what follows the
caret comes off with Backspace, the edit is typed, and what came off is
typed back, so each field keeps what it does with a key (the characters
it takes, the errors a key clears, its length).

A field has the whole of this only when it is (a) a `typing_target`,
(b) read by `typing_text`, (c) drawn with `Ui::typed`, (d) named by
`pointer::field_action`, so a press places the caret, and (e) reached by
`edit_key` before its own key handler in `Faraday::key`.

## 2. Survey (2026-10-30)

Moved to `docs/history/TEXT-CURSOR-LOG.md`.


## 3. Design

- **Material 3.** Text fields (m3.material.io, Text fields): the caret is
  the insertion point; a tap or a press places it, the arrow keys move
  it, and the field's other behaviour (label, outline, supporting text)
  is unchanged. Selection stays as `docs/history/NEW-WALLET.md` §13.2
  built it.
- **Masked fields.** A masked field draws one dot per character of what
  it holds (`chars().count()`, never bytes), so the edges a press reads
  and the caret drawn between dots fall where the characters are; no
  character is drawn or measured, and the eye stays the only way to see
  them.
- **Secrets.** No new operation on a secret is needed: an edit inside a
  field pops and pushes through the field's own keys (§1), which wipe
  what they drop, and the characters typed back are held meanwhile in a
  `SecretText` (`faraday/faraday-intake/src/secret_buf.rs`), wiped when
  dropped. `typing_text` reads a field into a `SecretText` as well. No
  key material is in these fields: what becomes a key is parsed from them
  into `faraday-keys` as before.
- **Drawing reads.** The caret's index from a press is worked out at the
  event from the edges the last frame noted. `Ui::typed` measures each
  prefix of what it shows, as it does for the fields that have it now;
  the fields that move to it are one line and short (a name, a find, a
  path, an index, a prefix, a passphrase).
- **Fixed text.** Vanity's prefix starts with the address kind's fixed
  characters (`bc1q`, `1`, `3`); the caret moves only within what was
  typed after them, and a press on the fixed part puts it at their end.
- **Several lines.** A field that wraps notes each line's edges with the
  line's top and bottom (`FieldAt` gains a band); a press finds its line
  first, then the edge. Up and Down move to the edge nearest the same
  column on the line above or below; Home and End go to the field's
  ends. Enter keeps what the field does with it. A field of several
  lines that sizes its box first is broken into lines once
  (`Ui::wrapped_as`), then drawn with its caret (`Ui::draw_typed`).
- **Fields that take typing unpressed.** A field that takes typing as
  soon as it is shown (a find, a path, an index, a prefix, a message, a
  string, a grid of words) is pressed with `Action::TypeHere`
  (`edit::Here`), which places the caret; on a message not being typed
  it starts typing, and on Add a key's string, words or the aezeed's
  words while their passphrase takes typing it moves typing there.
- **Touch targets.** Every field a press places the caret in is a touch
  target, 48 dp, tall (Material: 48 dp): Add a key's string field and
  the small panel's Vanity prefix field are `tokens::TOUCH` tall, the
  grids' rows are a touch target apart, and the codex32 re-entry's boxes
  are pressed by their group of four.
- **Grids of words.** The words typed in Latin letters (Add a key, Check
  my copy) are one text, a cell a word: Left and Right move the caret
  within a word and on past its edge into the next cell, a press on a
  cell puts it there, and an edit is made in place (`edit::splice_text`),
  what follows held in a `SecretText`, no word completed part way
  through. A word shown as bullets shows the caret at its end. A list
  typed on its own keyboard holds words taken and the keys of the one
  being typed: the caret moves within those keys, and past their edge a
  word taken earlier is opened again in its cell (`WordEntry::open_word`)
  with the words after it held aside and put back once it is taken again
  (`WordEntry::close_word`); a word left unchanged is taken again as it
  was, one changed stays until a word is pressed among its candidates; an
  edit inside the keys is taken only where some word still starts so
  (`WordEntry::edit_keys`). A press on a cell opens its word.
- **Codex32's boxes.** The correction's re-entry takes a caret among the
  boxes to type again (`Correction::caret_to_box`): a character goes in
  the box at the caret, in place of what was there, and the caret moves
  on; Backspace clears the box before it, Delete the one at it. Make
  codex32 shares' string typed back is a field after its `ms1`, its
  edges where each character stands in the groups of four.
- **A GPG key's paperkey line.** The caret moves in the line, a press
  places it, Up and Down a line where it wraps; an edit inside it holds
  the line from ending part way (`RestoreLive::held`), and the moment it
  is whole and its CRC holds it is taken and the next asked for, typed
  straight through or fixed in the middle, with no Enter; a line not yet
  right stays.

## 4. Batches

Moved to `docs/history/TEXT-CURSOR-LOG.md`.


## 5. Tests

User-facing only (CLAUDE.md): type, move left twice, type: the character
is inserted there; a press between characters: the next key lands there;
Backspace and Delete at the caret; the same in a masked field, where the
field still shows only dots, as many as the characters; a grid's words
across their cells; a word typed on a list's own keyboard opened and
changed with the words after it kept; the codex32 re-entry's boxes and
the string typed back; the paperkey line fixed in the middle. In
`faraday/faraday-core/tests/all/caret.rs`, `languages.rs`,
`codex32_make.rs`, `gpg_restore.rs` and `codex32_correct.rs`.

## 6. Questions for the owner

Moved to `docs/history/TEXT-CURSOR-LOG.md`.
