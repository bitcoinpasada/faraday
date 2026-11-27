# The on-screen keyboard

Faraday's own on-screen keyboard, in two layers: the key geometry of 16
layouts in the drawing base (`core/faraday-draw/src/widgets/keyboard.rs`:
which keys, where, which are live) and the keyboard panel in the component
layer (`faraday/faraday-ui/src/keyboard.rs`: a panel of key buttons, Shift
and Symbols state, the Done key's style). Typing screens raise it when a
field that takes typing is pressed (seed words, PINs, passphrases, hex,
dice, coins, cards, paths, addresses, codex32, and the kana, jamo, pinyin
and zhuyin word lists). It is for a person with a touch screen and no
hardware keyboard.

## Screens and sheets

- The panel `keyboard` / `keys_drawn` (`faraday/faraday-ui/src/keyboard.rs:~137-253`): SurfaceContainer panel with an OutlineVariant top rule; keys drawn as `key_in` buttons; the Done key styled per `DoneKey` (Primary filled, Secondary outlined, Off disabled; `faraday/faraday-ui/src/keyboard.rs:~67-76`); Shift drawn as an up-chevron icon, backspace and space icons; width capped at `KEYS_MAX_W` and centred; text fixed at Standard size.
- Layouts (`KeyboardKind`, 16 kinds: Bip39, Pin, Hex, Passphrase, Dice, Coin, Binary, Cards, Path, Address, Codex32, Codex32Unreadable, Kana, Jamo, Pinyin, Zhuyin; `core/faraday-draw/src/widgets/keyboard.rs:20-80`):
  - Bip39 (word list): three letter rows, row 3 seven letters and backspace, no ✓ (`core/faraday-draw/src/widgets/keyboard.rs:~578-592`).
  - Pin: digits, bottom row backspace, 0, ✓ (`core/faraday-draw/src/widgets/keyboard.rs:~593-619`).
  - Hex: three rows of six keys; ✓ and backspace on row 3 at 1 unit (`core/faraday-draw/src/widgets/keyboard.rs:~620-632`).
  - Dice pad: 4 columns (three faces plus backspace), backspace spans both rows in column 4, no ✓ (`core/faraday-draw/src/widgets/keyboard.rs:~633-660`).
  - Coin: H and T keys (2 units each), backspace, ✓ in one row (`core/faraday-draw/src/widgets/keyboard.rs:~661-669`); Binary the same with 0 and 1 (`core/faraday-draw/src/widgets/keyboard.rs:~670-678`).
  - Cards: 7 rank keys, 6 rank keys plus backspace, four suits plus ✓ (3 units), in three rows (`core/faraday-draw/src/widgets/keyboard.rs:~679-702`).
  - Path: rows of 5, 5, then `/`, `h`, backspace and ✓ at 1.5 units (`core/faraday-draw/src/widgets/keyboard.rs:~703-723`).
  - Address: bech32 rows, Shift swaps to base58 lower and upper; a Symbols key appended to row 0 under Shift; row 3 Shift, letters, Backspace, ✓ on every layer (`core/faraday-draw/src/widgets/keyboard.rs:~724-757`, rows `core/faraday-draw/src/widgets/keyboard.rs:~117-151`).
  - Codex32: no Shift; row 3 has `?` only for Codex32Unreadable, then Backspace and ✓ (`core/faraday-draw/src/widgets/keyboard.rs:~758-784`).
  - Kana: 10 columns laid by column; empty cells draw nothing and are not hit (`core/faraday-draw/src/widgets/keyboard.rs:~785-808`, grid `core/faraday-draw/src/widgets/keyboard.rs:~179-240`).
  - Pinyin: three letter rows and a fourth of tones 1-5 with backspace (`core/faraday-draw/src/widgets/keyboard.rs:~830-853`).
  - Zhuyin: four rows of 11, 11, 10, 10 keys, backspace ends row 4 (`core/faraday-draw/src/widgets/keyboard.rs:~854-868`, rows `core/faraday-draw/src/widgets/keyboard.rs:~284-289`).
  - Passphrase: four rows from `PASS_ROWS` or `SYMBOL_ROWS`; Shift upper-cases only the letter layer; bottom row Symbols (2 units), space (6 units), ✓ (2 units) (`core/faraday-draw/src/widgets/keyboard.rs:~869-904`).

## Actions

- A key press gives an `OskPress`: Char, Back, Done, Shift, Symbols, Show, Hide (`faraday/faraday-ui/src/keyboard.rs:~17-33`).
- `Keys::press` turns it into `Typed` Char, Backspace or Enter: typing a letter clears Shift; Shift and Symbols toggle; Back is Backspace and Done is Enter (`faraday/faraday-ui/src/keyboard.rs:~81-101`, `faraday/faraday-ui/src/keyboard.rs:~36-44`).
- `Keys::reset` returns to Shift off, letters layer (`faraday/faraday-ui/src/keyboard.rs:~104-106`).
- In the geometry layer a key does `KeyInput` Char, Backspace, Done, Shift or Symbols (`core/faraday-draw/src/widgets/keyboard.rs:~84-95`).

## Effects

None outside the app's state. Typed characters go to the caller's field.

## State and lock

- `Keys` (`faraday/faraday-ui/src/keyboard.rs:56-64`) holds `shift`, `symbols` and the Done style only, no typed text. Callers in faraday-core build it fresh (`OskKeys::default()` at `faraday/faraday-core/src/codex32/input_panel.rs:~399`, `faraday/faraday-core/src/codex32_backup/screen.rs:~159`, `faraday/faraday-core/src/codex32/screen.rs:~508`) or from their own Shift and Symbols state (`faraday/faraday-core/src/codex32/screen.rs:~595`).
- `Modifiers` (`shift`, `symbols`) is a value type passed to `keys()` (`core/faraday-draw/src/widgets/keyboard.rs:~99-104`).
- `KeyMask` (u64) is owned by the caller (`core/faraday-draw/src/widgets/keyboard.rs:~298-314`).
- `keys()` is a pure function; nothing is kept. No secrets held by the keyboard; what is typed (a passphrase, seed words) is the caller's.

## Reach

- `faraday_ui::keyboard::{keyboard, height, Keys, DoneKey, OskPress}`, paths under `faraday/faraday-core/src/`:
  - `codex32/screen.rs:416,518-519,591-599,812-818`; `codex32/input_panel.rs:217,393-399`; `codex32/pan.rs:287`; `codex32_backup/screen.rs:158-162`.
  - `compact/mod.rs:1665,1874` (`OskPress` at 543,740,1190,1409-1424,1833-1840).
  - `keygen/mod.rs:2378,2776`; `wallet_backup.rs:1236,1270`; `action.rs:775`.
- `faraday_draw` `keyboard::{keys, rows, KeyInput, Modifiers}`: `screens/parts.rs:1478-1530` (`word_keys`); `rows` and `KeyboardKind::Bip39` at `screens/entry.rs:315,339`; `KeyMask`, `key_bit`, `KeyboardKind` in `word_entry.rs`.
- `key_bottom_reserve` and `keycaps` are used by no app module (the font baker reads `keycaps`, per its doc); the PIN scramble is used by none.
- The shell's `keyboard` module (`shells/stick/src/keyboard.rs`) is the physical keyboard, not this component.
- Raised by typing fields (`ui-text`) on Add a key, passphrase fields, the codex32 input panel and worksheet, codex32 backups, dice and coin entry, paths, addresses and the Asian word lists.
- Its rows push the focused field above the keyboard through the focus box noted by `ui-text` and `ui-regions`.
- Keys drawn by `key_in` (`ui-controls`), hover and focus only.
No code in the app passes a PIN scramble seed: both callers of `keyboard::keys` (faraday/faraday-ui/src/keyboard.rs:187, faraday/faraday-core/src/screens/parts.rs:1491) pass `None`, and no caller builds `KeyboardKind::Pin`, so `scrambled_digits` runs only in the crate's tests (core/faraday-draw/src/widgets/keyboard.rs:504, 593-598, 1034-1042).

## Rules and refusals

Panel (`faraday/faraday-ui/src/keyboard.rs`):
- Height is rows x row height + 8 (`height`, `faraday/faraday-ui/src/keyboard.rs:~110-112`).
- Row height: the kind's `min_row`, else `KEY_ROW_COMPACT` (32, `faraday/faraday-ui/src/keyboard.rs:48`) on compact, else `OSK_KEY_ROW` (56 = `TOUCH + GAP`) (`row`, `faraday/faraday-ui/src/keyboard.rs:~118-127`).
- Keys width capped at `KEYS_MAX_W` (720, `faraday/faraday-ui/src/keyboard.rs:52`) and centred on a wide layout; compact uses the full width (`faraday/faraday-ui/src/keyboard.rs:~171-172`).
- The layout is built at a fixed `Scale::new(160)` and `SizeClass::Small`, not the Ui's scale (`faraday/faraday-ui/src/keyboard.rs:~179-182`).
- The panel asks `keys()` for every key (`ALL_KEYS`) and passes no PIN scramble seed (`None`) (`faraday/faraday-ui/src/keyboard.rs:~187`).
- Done key style by `DoneKey` (`faraday/faraday-ui/src/keyboard.rs:~194-199`).
- A letter clears Shift (`faraday/faraday-ui/src/keyboard.rs:~83-86`).
- Keys draw hover and focus only, no pressed layer (`key_in`, `Unpressed::Keyboard`, `faraday/faraday-ui/src/lib.rs:~2197-2221`).
- Shift is drawn as an up-chevron icon, in the accent's text colour while Shift is on (history, 10a89d54d).

Geometry (`core/faraday-draw/src/widgets/keyboard.rs`):
- Every row but the last is `key_h = (area.h - reserve) / rows`, at least 1; the last row takes the remainder to `area.bottom()` (`core/faraday-draw/src/widgets/keyboard.rs:~530`, `core/faraday-draw/src/widgets/keyboard.rs:~535-542`); reserve is 0 for equal-row kinds, else `key_bottom_reserve` (`core/faraday-draw/src/widgets/keyboard.rs:~535-539`).
- Equal-row kinds are Codex32, Codex32Unreadable and Dice (`equal_rows`, `core/faraday-draw/src/widgets/keyboard.rs:~444-449`); `min_row` is `HEX_KEY_ROW` for Hex, `CODEX32_KEY_ROW` for equal-row kinds, else 0 (`core/faraday-draw/src/widgets/keyboard.rs:~456-464`); `rows()` per kind (`core/faraday-draw/src/widgets/keyboard.rs:~467-482`).
- Horizontal: weight units per key, `usable = area.w - gap*(n-1)`, the last key ends at `area.right()` (`lay_row`, `core/faraday-draw/src/widgets/keyboard.rs:~553-575`, `core/faraday-draw/src/widgets/keyboard.rs:~557-574`).
- Key faces are inset `KEY_FACE_INSET` 3 dp across and `KEY_FACE_INSET_Y` 2 dp up and down; keys tile with `KEY_GAP` 0 (`core/faraday-draw/src/tokens.rs:~337-341`); `KEY_BOTTOM_INSET` 16 dp under the bottom row plus the shell's inset (`core/faraday-draw/src/tokens.rs:~344-351`); `key_bottom_reserve(inset) = KEY_BOTTOM_INSET + inset - KEY_FACE_INSET_Y` (`core/faraday-draw/src/tokens.rs:~356-357`).
- Key mask: bit n is the n-th key of a kind; bit 63 `DONE_DISABLED` disables ✓; bit 62 `DONE_ABSENT` leaves the ✓ cell empty (`core/faraday-draw/src/widgets/keyboard.rs:~298-314`). `live()`: a Char key follows its bit; Done follows `DONE_DISABLED`; other keys are always live (`core/faraday-draw/src/widgets/keyboard.rs:~912-918`).
- `key_bit()`: BIP-39 and address letters a-z; digits from 26; upper case from 36 (Address); pinyin tones at 26+i; Kana, Jamo and Zhuyin by layout index (`core/faraday-draw/src/widgets/keyboard.rs:~319-392`); `letter_bit()` is `key_bit(Bip39, c)` (`core/faraday-draw/src/widgets/keyboard.rs:~396-398`).
- PIN: digits are scrambled per seed by `scrambled_digits` while mask bits keep natural digit numbering; `DONE_ABSENT` pops the ✓ cell and leaves the columns (`core/faraday-draw/src/widgets/keyboard.rs:~593-619`).
- `scrambled_digits(seed)`: xorshift32 from `seed | 1`, then Fisher-Yates over 0-9; deterministic from the caller's seed (`core/faraday-draw/src/widgets/keyboard.rs:504-515`).
- Path: ✓ is dead while `DONE_DISABLED` (`core/faraday-draw/src/widgets/keyboard.rs:~703-723`).
- Drawing uses the same rects as hit-testing (`KeyCap`, `core/faraday-draw/src/widgets/keyboard.rs:~108-115`).
The panel and the word keyboard lay their keys out at a fixed 160 dpi scale (one design unit per pixel) inside `fixed_text`, so the keys keep their size whatever the text size; the panel passes size class Small, the word keyboard Wide, and key rectangles do not depend on the class (faraday/faraday-ui/src/keyboard.rs:145-149, 181-184, faraday/faraday-core/src/screens/parts.rs:1466-1485).

## Domain calls

- `core/faraday-draw/src/widgets/keyboard.rs`: `keys` (`core/faraday-draw/src/widgets/keyboard.rs:~521-907`), `key_bit`, `letter_bit`, `keycaps`, `equal_rows`, `min_row`, `rows`, `scrambled_digits`, `live`, `KeyboardKind`, `KeyInput`, `Modifiers`, `KeyCap`, `KeyMask`, `ALL_KEYS`.
- `faraday_draw::layout::LayoutCtx::new`, `faraday_draw::geom::Scale::new`, `SizeClass::Small` (`core/faraday-draw/src/layout.rs`, `core/faraday-draw/src/geom.rs`).
- `faraday_draw::tokens::{OSK_KEY_ROW, HEX_KEY_ROW, CODEX32_KEY_ROW, KEY_GAP, KEY_FACE_INSET, KEY_FACE_INSET_Y, KEY_BOTTOM_INSET, key_bottom_reserve, key_label, PAD_COLUMNS, KEY_UNITS_EDGE, KEY_UNITS_DONE, KEY_UNITS_SPACE, KANA_COLUMNS, RADIUS_SMALL}` (`core/faraday-draw/src/tokens.rs`).

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 10a89d54d | The on-screen keyboard's Shift key showed "?" because Faraday's fonts lack "⇧". The same commit fixed the Companion's Back in Windows sheet naming the wrong app. | Shift is drawn as an up-chevron icon, in the accent's text colour while Shift is on; the Starter is named by its name (F16). |
| 7ac5cd6a0 | On the desktop the on-screen keyboard's rows (32 dp) left its keys under the 48 dp touch floor. | Off a compact panel a row is `OSK_KEY_ROW` (touch target plus a gap, 56 dp) and `height` takes `compact`; a compact panel, the stick's included, keeps its 32 dp row (`faraday/faraday-ui/src/keyboard.rs:118-127`). |

## Doubts

- The kana grid comment does not match the grid (`core/faraday-draw/src/widgets/keyboard.rs:~171-178`).
- Where the "no popup, no press layer" rule of the module doc is enforced (`core/faraday-draw/src/widgets/keyboard.rs:~8-10`); partly answered by `key_in` drawing hover and focus only.
- `KEY_BOTTOM_RESERVE` is named in a doc comment but only the function `key_bottom_reserve` exists.
