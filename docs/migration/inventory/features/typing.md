# Typing and the caret

The typing caret and field selection shared by every text field, the routing that decides which field the keyboard types into on each screen and sheet, and the drawing of invisible characters as marks. It serves every field of every product: seed words, passphrases, addresses, paths, messages, searches.

## Screens and sheets

- No screen of its own. The caret and selection are drawn inside each field.
- Invisible characters (`faraday/faraday-core/src/visible.rs`): `visible` (`:~75`) and `first_line` (`:~96`) draw text with marks: `HIDDEN_MARK` ◌ (`:~6`) and `BREAK_MARK` ¶ (`:~8`).
- `choice_row` (`faraday/faraday-core/src/visible.rs:~159-173`) builds a flow's Your-choices row; a value of none is drawn "Not chosen yet".

## Actions

- Typing, Backspace, Shift selection, double-press select-all, drag select (12 px horizontal, or 3 px across a character edge).
- Up and Down only on multi-line fields: Message, Invoice, CheckAddress, Query, DecodeText, `GPG_LINE`, TransferText (Demo), compact Entry, a multi-line vault field.
- Caret blink.

## Effects

None outward.

## State and lock

- `TypingArea` (`faraday/faraday-core/src/edit.rs`) is `Public`: it holds no text; it has the `pass_eye` flag.
- `Shown` cache (`faraday/faraday-core/src/visible.rs:~105-155`): `RefCell<Option<Kept>>` holding the source `Text`, the visible string and up to 4 wrap tables. Its one holder is `MessageArea.shown` (`faraday/faraday-core/src/state.rs:1108`), given only a signed message's text (`faraday/faraday-core/src/screens/message.rs:351`, `471`; `faraday/faraday-core/src/screens/check_message.rs:336`), which is public.
- Typed text lives in each field's own area, not here. `typing_text` builds `SecretText` copies, which wipe when dropped.
- Every lock ends in a fresh process (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- `moved_plan` and `plan_moved` (the frame-only redraw plan) also live in `faraday/faraday-core/src/edit.rs`.

## Reach

- Used by every field: Add a key (`add-key`), passphrases (`secrets`), messages (`messages`), Check an address and Explore (`address-check-explore`), Decode (`decode`), vault fields (`vault-contents`), Set the time (`settings-time`), the guide's Find (`guide`), Home's search (`menu-pages`).
- The docked keyboard (`small-panel`).

## Rules and refusals

- `typing_target` and `typing_field` decide which field the keyboard types into per screen and sheet: Explore only Path and Find; Bip85 only on the INDEX step; Silent only on the ADDRESS step; Vanity while typing the prefix; Lightning the Aezeed and Core Lightning passphrases; Create the pass and xpub slots; KeyGen when not locked. The Krux form is read, never typed (`Screen::Entry => form != Form::Krux`).
- Control characters are refused (`faraday/faraday-core/src/edit.rs:~959`).
- Latin-letter word fields take only ASCII letters and space (`words_splice`, `faraday/faraday-core/src/edit.rs:~1091-1093`).
- A capped field that takes less than was typed restores the original text.
- A re-entrancy guard holds while editing.
- `can_show` (`faraday/faraday-core/src/edit.rs:~1231-1275`): SetTime never; Unlock only while the selected file is not open; Transfer only online in Demo; Decode and CheckMessage need state or a landing; menu pages always.
- Invisible characters (`faraday/faraday-core/src/visible.rs`):
  - Control characters other than `\n` are drawn ◌ (`:~53-56`).
  - `\r`, U+000B, U+000C, U+0085, U+2028, U+2029 are drawn ¶ (`:~11-16`).
  - The `INVISIBLE` table is Unicode 16.0 Default_Ignorable plus Cf (`:~21-47`), a static table.
- Since the history fixes: Sign a message shows the whole message with invisible characters drawn as marks (38a890f28, under `messages`).

## Domain calls

- `core/faraday-text`: `Text`, `SecretText`.
- `faraday/faraday-forms`: `Form`, typed candidates (`faraday/faraday-core/src/forms.rs:~449`).

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 1761268bd | A word grid showed a caret in every empty cell after the one being typed into. | The caret shows only in the cell being typed into: a cell with no word yet shows one only when it is the current cell (`faraday/faraday-core/src/screens/parts.rs:1220-1221`). |

## Doubts

- The `INVISIBLE` table is fixed to Unicode 16.0 and will drift from later Unicode versions.
