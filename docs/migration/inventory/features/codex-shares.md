# Codex32 shares

Making codex32 (BIP 93) shares by hand on the booklet's worksheet, and
correcting a typed codex32 string. Make codex32 shares is for a person who
wants a seed split into shares, or kept as one secret string, that can be
checked with paper and pen. It lives inside New key (`KeyGen`,
`kstep::CODEX32`, the field `k.codex32`) and is drawn in New key's column.
A set typed into Add a key's codex32 form can be completed here (Make the
rest, Add a share). Suggest corrections is a tick held by the forms that
take a codex32 string (Add a key's codex32 form, Check a string).

## Screens and sheets

Make codex32 shares (`Codex32Make`, `faraday/faraday-core/src/codex32/mod.rs:~466`; `MAKE_OFFERED` = true, `~45`). Seven cards `Card::{Scheme, Identifier, Entropy, Enter, Sheets, Strings, Made}` (`~78`, `~96`), drawn by `cards()` (`faraday/faraday-core/src/codex32/screen.rs:~126`) and `body()` (`~325`):
- Scheme (`faraday/faraday-core/src/codex32/screen.rs:~439`): length 48 or 74 characters; split 2-of-3, 3-of-5, Other (threshold and strings steppers), or Alone (the secret alone).
- Identifier (`~523`): 4 characters on the codex32 keyboard, Suggest, Continue; a system randomness row (`~527-530`).
- Entropy (`~607`): way Dice, Typed or Shares; a dice size chooser.
- Enter (`~636`): characters in, string boxes, the worksheet strip, translation sheets, pad or keyboard. On compact windows a context panel tiles Dice, Checksum and String (`panel_tiles`, `~702`; `enter`, `~660`).
- Sheets (`~1294`): tabs for the dice rows per share, the checksum sheet, translation sheets.
- Strings (`~1461`): the finished string or its Codex32 QR, typed back, Skip or Continue.
- Made (`~1530`): fact rows, Check on another device, Lock in / Done.
- Dice chooser on the Entropy card: face counts d4 to d20 (`C32::Faces`), per-die colour chips (`C32::Colour`), a pad for dice that are not six-faced (`face_pad`).
- The booklet's "Dice de-biasing" sheet (`dice_sheet`, `faraday/faraday-core/src/codex32/dice_screen.rs:~241-268`): head row, five die rows as words, a 32-leaf tree, five die tracks; toggled by `C32::DiceSheet`.
- Entry panel (`faraday/faraday-core/src/codex32/input_panel.rs`): dice pad ("Your dice"), the codex32 keyboard for typed characters, Continue once every character is in; placed beside the sheets, pinned at the foot, or in the page (`PanelAt`, `~46-54`).
- Worksheet pan (`faraday/faraday-core/src/codex32/pan.rs`, `faraday/faraday-core/src/codex32/worksheet_screen.rs`): the checksum worksheet drawn pannable with a cell size by window class (`Geom`, `~161`; `draw` `~496`; `entry_sheet` `~719`; `review_sheet` `~758`). `SheetPan` glides to the front cell, `SheetFit` fits the whole sheet (`faraday/faraday-core/src/codex32/pan.rs:~125`, `~139`); `Follow` picks Work, Calm or Still per press (`~269`).
- Cell working: `cell_sheet` (`faraday/faraday-core/src/codex32/screen.rs:~1648`, a dialog on wide windows; its own page `cell_page` `~1704` on compact) shows the working of one cell (`working`, `~1770`).
- Check on another device (`C32::Check` toggles `check_open`, `faraday/faraday-core/src/codex32/mod.rs:~1631`; `sets_to_try` `~2337`; `check_record`, `faraday/faraday-core/src/codex32/screen.rs:~1606`).
- Final-row check sheet for Check a string (`check_sheet`, `faraday/faraday-core/src/codex32/worksheet_screen.rs:~800`).
- Column variant on a wide, non-compact window (`faraday/faraday-core/src/codex32/pane_screen.rs:~34-40`).
- "Demo only" card with Previous, Next and Play for the Practice walk-through.

Suggest corrections (`Correction`, `faraday/faraday-core/src/codex32_correct/mod.rs:~272`): `tick_row` (`faraday/faraday-core/src/codex32_correct/screen.rs:~30`), `panel` (`~92`), `proposal` (`~147`), `retype` (`~212`), `boxes` (`~370`), `string_boxes` (`~481`), `fingerprint_line` (`~51`).

## Actions

Make (`C32`, `faraday/faraday-core/src/codex32/mod.rs:~308-368`; `act_press` `~1506-1590`):
- `Card(i)` opens a card only when every earlier card is done (`toggle`, `~1646-1659`); `Next` (`~1748`).
- `Length(0|1)`, `Split(i)`, `K(d)`, `N(d)`.
- `Key(OskPress)`, routed by the open card (`key`, `~1919`).
- `Suggest` (`~1554`): an identifier from fresh randomness.
- `Way(i)` (only before work exists, `~1563`), `Faces(i)` (`~1985`), `Colour(d)`, `Face(f)`, `DiceSheet`.
- `Tab(i)`, `TabStep(d)`, `View(v)`, `Panel(p)` (capped at 3), `Cell(cell)`, `CellClose`, `CellStep(d)`, `Eye`, `Check`, `Qr`, `Skip`, `Sample`, `Lock`.
- Typing a string back: `Action::TypeHere(Here::Codex32Back)` (`faraday/faraday-core/src/codex32/screen.rs:~1423`).
- From New key: `keygen_codex32` (`faraday/faraday-core/src/keygen/mod.rs:~2369`), `keygen_codex32_by_hand` (`~2462`), `keygen_codex32_lock` (`~2647`). `C32::Lock` is handled at `faraday/faraday-core/src/keygen/mod.rs:2377` and runs `keygen_codex32_lock` (`2647`).
- Typed strings from Add a key: `keygen_codex32_given` (`faraday/faraday-core/src/keygen/mod.rs:~1487`); Make the rest / Add a share (`make_given`, `faraday/faraday-core/src/codex32/mod.rs:~1798`).
- Practice: `C32::Sample` / `use_sample` (`faraday/faraday-core/src/codex32/mod.rs:~1867`), `put_sample` (`~1833`).

Suggest corrections (`Press`, `faraday/faraday-core/src/codex32_correct/mod.rs:~111-135`): `Tick`, `Continue`, `Previous`, `Next` (pages of four pairs), `Mistyped`, `Damaged`, `Unreadable`, `Take`, `CheckBoxes` (back to the entry as typed), `Group(u8)`. Keys (`Correction::key`, `~584`): bech32 characters fill the caret box in Retype; Backspace, Delete, arrows, Home, End; Escape is CheckBoxes; Enter is Continue, Next or Take by step (`~653-655`).

## Effects

- Costly work takes `c: &Compute`: `WorkSet::new/given`, `work.push`, strings and fingerprint, `faraday_codec::qr::codex32qr(c, s)` (`faraday/faraday-core/src/codex32/mod.rs:~1499`, `~1809`, `~1838`, `~1896`, `~2120`, `~2160`, `~2222`).
- Randomness: `C32::Suggest` takes a 32-byte draw made by the caller, `self.fresh(b"codex32 identifier")` (`faraday/faraday-core/src/keygen/mod.rs:~2385`).
- The worksheets are read as a paper form (`Reveal::PaperForm`, `faraday/faraday-core/src/codex32/mod.rs:~1045`, `~1328`, `~1710-1731`, `~2160`).
- Reads vault records and keys, without writing, to collect identifier clashes (`~2364-2420`).
- Lock in loads the key: `add_codex32` (`faraday/faraday-core/src/keygen/mod.rs:~2672`); `create_codex32_made(set)` (`~2709`).
- Suggest corrections runs the decoder with `Compute`: syndromes, Berlekamp-Massey, a box search or a solve of at most 15 unknowns (`correct`, `faraday/faraday-core/src/codex32_correct/mod.rs:~475-490`).
- No storage, clipboard, camera, printing or worker.

## State and lock

- `Codex32Make` (`faraday/faraday-core/src/codex32/mod.rs:~466-579`): `open`, `done[7]`, `length`, `split`, `other_k`, `other_n`, `fixed`, `shares_only`, `id[4]`, `taken`, `way`, `work: Option<WorkSet>` (secret, a faraday-keys type), `seq` and `rows` (rolls in progress, `Zeroizing`), `dice: BookletChar`, `made: Vec<Vec<Made>>` (`Made.value` zeroed on drop, `~409-413`), `typed`, `strings: Option<Held<Vec<Text>>>` (wipes), `fingerprint`, `qr: Option<QrMatrix>`, `back: Back` (with `typed: SecretText`), `locked`, `hidden`, `given`, `inputs: Vec<Codex32>`, `demo` (the published sample, not secret), and pan, fit and scroll fields.
- `keygen.dice` (the dice choice) is kept for the session (`faraday/faraday-core/src/keygen/mod.rs:~2423`).
- App lock: the flow belongs to New key's state; `KeyGenArea::lock` calls `forget()`, which sets the state to None and drops the whole `Codex32Make` (`faraday/faraday-core/src/keygen/area.rs:728-741`, `175-177`). The lock then restarts the process (`faraday/faraday-core/src/vault/seal.rs:933-946`). Leaving New key drops it as for `keygen-flow`.
- Flow Lock in (`set_locked`, `faraday/faraday-core/src/codex32/mod.rs:2314-2319`) sets `locked`, marks Made done and clears `note`; work and strings stay, shown on Made. While locked only `Card`, `Tab` and `TabStep` pass; `Check`, `Cell`, `CellClose`, `CellStep` and `Eye` go to `view_press`; all else is ignored (`faraday/faraday-core/src/codex32/mod.rs:1507-1515`).
- Masking and leaving a step: `mask()` (`~1671`) sets `hidden` and closes any cell; `hide_qr` (`~2297`) and `hide_qr_of_left_step` (`~2305`) drop or hide the QR; `undo_entry` (`~2179`) clears strings, fingerprint and QR; `put_sample` clears `made` and `seq` and replaces `work`.
- Suggest corrections (`faraday/faraday-core/src/codex32_correct/mod.rs:~272-276`): `on`, `stage` (`Refused(Refusal, usize)`, `Shown(Box<Shown>)`, `Closed`), `noted`. Fresh with the tick off every time the form opens (`faraday/faraday-core/src/codex32_correct/mod.rs:285-290`). `Shown` (`~230-243`) holds `proposal`, `text: Codex32Ascii` (secret, wiped on drop), `step`, `page`, `seen`, `mark`, `retyped` (secret, zeroized on clear and drop), `differs`. `Taken` (`~152-155`) holds the taken `Codex32Ascii` (secret, wipes on drop).
- `take` (`faraday/faraday-core/src/codex32_correct/mod.rs:~662-670`) replaces the stage with `Closed`, so `Shown` and `Retyped` drop; the string moves into `Taken`.
- The correction lives in the Add a key entry state (`faraday/faraday-core/src/state.rs:~76`); the entry's lock calls `reset()` (`faraday/faraday-core/src/state.rs:2766-2781`).
- `KeyMarks` (`faraday/faraday-core/src/codex32_correct/mod.rs:~697`): per fingerprint, for the session, "Share entry corrected" or "Share repairs unchecked"; never weakened (`~703`). Not secret.
- `Codex32Make.inputs` (`faraday/faraday-core/src/codex32/mod.rs:544`) is a `Vec<Codex32>`. `Codex32` derives `ZeroizeOnDrop` (`core/faraday-bip/src/codex32/mod.rs:370-374`), so each string typed in wipes as it drops.

## Reach

- Opened from New key (`faraday/faraday-core/src/keygen/mod.rs:~2367-2377` forwards presses and runs Lock in), from Add a key's codex32 form (a given set), and Check a string (`check_sheet`).
- Shares way hands over to Add a key's codex32 form (`faraday/faraday-core/src/codex32/mod.rs:~1761-1763`, `~778`).
- Uses `keygen::{KeyGen, kstep::CODEX32, screen::codex32_work_w}`, `screens::hub_layout::{guide_room, guide_place_at}`, `screens::{next_button, next_button_when, stepper, section_label, buttons_and_next, randomness::row}`, `fresh::roller::five_screen::{pairs, pairs_across, rolls_line, Pairs}` (the five-dice roller, see `randomness`), `codex32_correct::Mark`, `plan::Answers` (`a.codex32`), `vault::VaultArea` (records), `secret_text::SecretText`, `motion`, `compact::{BAR_H, M, bar}`, `edit::Here`, `wallet::fp_text`.
- Convert's codex32 target uses `codex32::taken_ids` for identifier clashes (see `convert`).
- Suggest corrections is used by Add a key (`add-key`) and Check a string.

## Rules and refusals

Scheme
- Length and split change only before the work, not when `fixed`, not for a typed-in set (`scheme_open`, `faraday/faraday-core/src/codex32/mod.rs:~1602`, `~1519`, `~1526`).
- Threshold `k` clamped to `2..=MAX_THRESHOLD`, with `n >= k` and `n <= MAX_SHARES` (`faraday/faraday-core/src/codex32/mod.rs:1531-1550`).
- `shares_only` drops the "secret alone" split (`~729-742`, decision F240).
- Make the rest: the strings typed must number at least threshold + 1 (`~1537-1543`).
- `k >= 4` shows a note that there is no translation sheet (`faraday/faraday-core/src/codex32/screen.rs:~493-500`).

Identifier
- Exactly 4 characters before Continue (`faraday/faraday-core/src/codex32/mod.rs:~1753-1755`); typing stops at 4 (`~1932-1937`).
- An identifier already carried on this device is refused, and the refusal names which set holds it: the typed Add a key form, loaded keys, the plan, open vaults' share and key records (`id_refused`, `~859-868`; `taken`, `~2364-2420`).
- Suggest only when not locked and with no work (`~1596-1598`); hidden once work exists (`faraday/faraday-core/src/codex32/screen.rs:~561`). The Done key only when `id_ready` (`~588-598`).

Shares and entropy
- Add a share: an index is refused when it is one of the typed strings or the secret's (`add_at_taken`, `faraday/faraday-core/src/codex32/mod.rs:~758-765`).
- Entropy's Continue begins the work; a refused set stays on the card with a note (`begin`, `~1893-1915`). The Shares way only when there is no work.
- Typed strings from Add a key need k >= 2 and k strings typed (`faraday/faraday-core/src/keygen/mod.rs:~1493-1497`); unparseable strings are dropped without a note (`~1496-1501`); in Practice a set whose fingerprint is not public is refused `NotAllowed` (`~1502-1515`).
- Practice uses the shell's sample (BIP 93 vector 2) in place of typed strings (`codex32_here`, `faraday/faraday-core/src/keygen/mod.rs:~1454-1478`); the sample is offered only where `sample_offered` (`faraday/faraday-core/src/codex32/mod.rs:~662`); none gives `c32r_demo_no_sample` (`~1857-1860`).

Dice
- `faces_ok` (`faraday/faraday-core/src/codex32/mod.rs:~706`, `~1976`); changing faces mid-character clears the rolls with a note (`~1985-1993`).
- A roll only on Enter with Way::Dice and a character still to enter (`~2043-2046`, `~1581`).
- Values 1..=faces; on ten-faced dice 0 is 10; a tens digit with `d*10 <= n` waits (`~2007-2019`).
- Struck rows (equal pairs) show "roll both again" (`faraday/faraday-core/src/codex32/dice_screen.rs:~494-516`); a die's bit is lower on 0, else higher (`~426-431`).

Cards and view
- A card opens only after every earlier card is done (`faraday/faraday-core/src/codex32/mod.rs:~1654-1659`); going back before Enter masks the sheets (`~1662-1663`).
- Eye only once sheets exist (`~821`); masking closes any cell; a cell is refused while masked (`~1617`, `~1397`). Panel index capped at 3; a Tab must be seeable.

Strings and Made
- Typed back: symbols only, up to the string's room (`~2241`); the finished typing is compared box by box and marked `checked` if equal (`~2247-2258`).
- Continue needs the typed copy equal, or skipped; the last string leads to Made (`~2265-2276`).
- Skip: the first press asks, the second skips, only on Strings (`~2281-2292`).
- The Codex32 QR only for a 48-character string and only when not masked (`faraday/faraday-core/src/codex32/screen.rs:~1469-1481`). The masked check record shows at most 12 dots (`~1616-1617`).
- Undo is refused for a given set and where a share has no characters (`faraday/faraday-core/src/codex32/mod.rs:~2171`, `~2193`).
- Check sets to try: none when k = 0; else `ceil(n/k)` (`~2339-2344`).
- Lock in needs `ready_to_lock()`: not locked, Made open, work complete (`faraday/faraday-core/src/codex32/mod.rs:~2327-2331`); outside secret-making only given strings are taken (`faraday/faraday-core/src/keygen/mod.rs:~2667-2669`). No key is accepted before the self-test passes (`keygen_codex32_lock`, `~2649`).
- Errors of the key crate are shown as notes through `faraday_strings::bip::codex32_error` (`faraday/faraday-core/src/codex32/mod.rs:~1819-1823`, `~1908-1911`, `~2121-2124`).

Entry panel and drawing
- The entry panel shows only on `Card::Enter` with work and at least one character (`faraday/faraday-core/src/codex32/input_panel.rs:~108-111`). On a small panel it stays in the page; beside the sheets when the work width holds both; pinned at the foot when the height is at most `INPUT_PANEL_MAX * h` (`~115-130`, `~475-478`). Continue replaces the pad when complete (`~133-136`); the typed way shows the codex32 keyboard; the rolls panel only when incomplete and on Dice; a d10 shows "0 is 10" (`~421`). A press under the foot panel reaches nothing under it (`~480-484`).
- Prefix boxes 1-3 are `ms1` and never open a working; `Cell::Given` from box 4 (`faraday/faraday-core/src/codex32/screen.rs:~1064`, `faraday/faraday-core/src/codex32/worksheet_screen.rs:~354-358`).
- The final row's differing cell is drawn in Error; an unknown final shows the target faintly (`faraday/faraday-core/src/codex32/worksheet_screen.rs:~460-465`). `check_sheet` shows only the final row unless `whole` and names the first differing box (`~800-826`). A short entry sheet scrolls the page instead of fitting (`~744-750`).
- String boxes per row by width: 24, 16, 12, 8 or 4 (`faraday/faraday-core/src/codex32/screen.rs:~869-894`); on compact `window()` keeps two boxes either side of the front box (`~1157-1173`).
- The `lint-secrets` rule keeps the worksheet type borrowed only through the `paper!` macro (`faraday/faraday-core/src/codex32/worksheet_screen.rs:~50-51`, `~133-157`).

Suggest corrections (`faraday/faraday-core/src/codex32_correct/mod.rs`)
- Off by default; while off nothing is decoded and the form refuses as before (`~287-288`, `~487-489`). Turning it off closes the screen and keeps the entry as typed (`~466-473`).
- `offer` (`~490-511`): a string that already holds with no unreadable box is `Offered::Holds`; else a proposal or a refusal.
- Refusals (`~328-346`): NotFound; TooManyUnreadable (bound `CORRECTION_BOUND`, a run of 13 or 15 by `MAX_STRING_LEN`); HeaderFixed (the header cannot be corrected); Malformed; NotAString. A caution when the proposal assumes the rest (more than 8 unreadable boxes, `~494-495`).
- Mistyped, Damaged and Unreadable only after every page was seen (`~543-552`); Unreadable only with no substitutions and edits (`~402-405`, `~552`). A Take from the checksum marks the string `Repaired` (`~555`). Previous not below 0; Next in range (`~538-542`).
- Retype: `?` cannot be typed (`~304-306`, `~716-717`); a character must be a bech32 `charval` and the caret within the changed boxes (`~595-600`); taken only if it matches the proposal exactly, else cleared and `differs` set (`~606-617`).
- Marks never go down (`~682`, `~460-462`, `~703`).
- The whole string is drawn only within `WHOLE_ROWS = 3`, else only the groups with a box to retype (`faraday/faraday-core/src/codex32_correct/screen.rs:~386-392`, `~474`). Constants: `PAIRS_PER_PAGE = 4` (`faraday/faraday-core/src/codex32_correct/mod.rs:52`), `RUN = 13`, `RUN_LONG = 15`.
- `preview` (`faraday/faraday-core/src/codex32_correct/screen.rs:~556`) is test-kit only.

## Domain calls

- `core/faraday-bip`: `codex32::{Length, Scheme::new, Codex32, CHARSET, charval, symbol_char, INDEX_ORDER, MAX_THRESHOLD, MAX_SHARES, SYMBOL_GLYPHS}`; for corrections `codex32::{correct, Proposal, Edit, Refusal, Codex32Ascii, CORRECTION_BOUND, MAX_EDITS, MAX_STRING_LEN, UNREADABLE}`.
- `core/faraday-entropy`: `booklet::{BookletChar, BOOKLET_DICE, BOOKLET_FACES, TREE, bit, faces_ok}`.
- `faraday/faraday-keys`: `WorkSet`, `Making`, `Reveal::PaperForm`, `Held`, `StringCheck`, `vault::{codex32_share_header, codex32_key_set_identifier}`, `codex32_seed`, `Keys::add_codex32`.
- `core/faraday-codec`: `qr::codex32qr`.
- `core/faraday-crypto`: `work::Compute`.
- `faraday/faraday-strings`: `bip::codex32_error`.
- `faraday/faraday-ui`: `keyboard`, `tabs`, `sheet::dialog_in`, `parts::{qr_code, wrap_buttons}`.

## Fixtures

None. Practice's sample is BIP 93 test vector 2, handed in by the shell (`faraday/faraday-core/src/codex32/mod.rs:~647-648`, `~1866`). The `ms1`/`MS1` prefix is a literal (`faraday/faraday-core/src/codex32/screen.rs:~41`, `faraday/faraday-core/src/codex32/worksheet_screen.rs:~42`); die colours are fixed RGB values (`faraday/faraday-core/src/codex32/dice_screen.rs:~33-43`).

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| eb8082390 | The five text faces lacked 22 code points the codex32 booklet uses (ℵ, Greek letters, Δ, μ, €, ⊕, †, ‡); those symbols could not be drawn; ♦ and ♥ came from a fallback face. | Every symbol in `SYMBOL_GLYPHS` resolves to a real outline in every text face. |
| 7285b52a3 | The page moved while one's own dice were rolled; a face, digit or Backspace inside a character moved the tracks; Previous could not step back through one's own faces in Practice; the Identifier card had two primary actions. | Rolls and digits inside a character move nothing; Previous steps back through the person's own faces in Practice; one primary action on the Identifier card. |
| 988c3cf21 | In the Demo only card, Previous, Next and Play moved with the step's text; Next's place changed at each step. | The buttons hold their place at every step of every character, in the card at 1280x800 and in the strip at 480x640. |
| 51b51666c | At Larger text the entry drew text over other text: the input panel did not grow with its lines, and the rolls' names ran past its foot. | At Larger no text is drawn over other text; the panel is as tall as its lines. |
| d3498f6d9 | Make codex32 shares could take an identifier another set on the device already carried (a loaded key's set, the plan in hand, or an open vault's share records). | A codex32 set never takes an identifier another set on this device carries; the refusal names which set holds it. |
| e51ba530b | The checksum squares were unreadable on some fills after a theme change; badges, step numbers and pills had low contrast. | Colours readable on every fill; a "checksum" role built from the booklet's hue. |

## Doubts

- Rows left out as not about this feature: e20186b1f (a plan record's codex32 string), 54332b693 (a codex32 secret read at a flow's scanner), 363c80dbf (a vault's share-set identity), 591ae4a10 (codex32 offered as a backup of a key of words).
- `Strings` of a backup's codex32 set are not counted in that area's `holds()`, so a typed but unlocked codex32 string would not raise the lock prompt on its own (Back up, not this flow).
- Drawing writes flow state: `note_dice_body_h` (`faraday/faraday-core/src/codex32/dice_screen.rs:~253`, `~266`) and `note_sheet_room` (`faraday/faraday-core/src/codex32/screen.rs:~749`, `~787`); whether this is interior mutability or a real write is unclear.
- `column_scale` returns `Option` but never `None` (`faraday/faraday-core/src/codex32/dice_screen.rs:~284`); `die_line` could read "higher . 0" for an equal pair (`~420-431`); `kw` can go negative at tiny widths (`~184`).
- The masked check record caps the dots at 12, so the dot count bounds the string length (`faraday/faraday-core/src/codex32/screen.rs:~1617`); intent unclear.
- `offer` returns `Holds` before assigning `stage`, so a stale proposal could stay on screen (`faraday/faraday-core/src/codex32_correct/mod.rs:~490-512`); unclear if reachable.
- Where `KeyMarks` is reset was not read.
- 7285b52a3 and 51b51666c name files (`codex32/tracker.rs`, `tools/costs-lint/src/walk.rs`, `walk_screen.rs`, `tracker_screen.rs`) that no longer exist at this commit.
