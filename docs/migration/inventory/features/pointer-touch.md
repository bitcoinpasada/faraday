# Pointer, touch and focus

How a press, a drag, a scroll, a hover and keyboard focus become actions.
In `faraday-core`, `Pointer` keeps the hit list of the last drawn frame and
decides which control a press acts on, starts drags and scrolls, shows
tooltips, moves keyboard focus, and, after every input, drops what a screen
left behind. On the device, the stick shell turns mouse, touchpad and
touchscreen events into `Event::Touch` and pointer motion, draws the arrow and
maps touch packets to panel pixels. It serves every screen and every person;
it has no screen of its own.

## Screens and sheets

- None of its own. It draws a tooltip on a long touch press, the arrow (cursor) on the device while something can point, and a keyboard focus ring on the focused control.

## Actions

- Press and lift on the same control acts (`Faraday::touch`, `faraday/faraday-core/src/pointer.rs:~433`; act on Up only if the same control is under press and lift, `~580-585`).
- Hold actions never act on release, only on the tick (`is_hold`, `faraday/faraday-core/src/action.rs:1152`; `faraday/faraday-core/src/pointer.rs:~583`).
- Double press on a text field within 450 ms selects all (`faraday/faraday-core/src/pointer.rs:591`); a drag over a field selects what it went over; a press that only places a caret brings the on-screen keyboard (`osk_after`).
- A press on a screen's way out is Back (`as_back_at`, `faraday/faraday-core/src/pointer.rs:~606-607`, F77).
- Drag and scroll: a drag starts past `motion::SLOP * f` (`~534-536`); a touch on flying content only stops it (`swallow`, `~475-484`); board chips follow the finger and do not start a scroll; sliders act again on each control; no scrollbar grab in compact layouts; a grab within `ui::BAR_GRAB` at a region's right edge (`~619-633`).
- Tooltips: a long touch press shows the tooltip and drops the press (`~347-358`); Escape dismisses it (`~339`).
- Keyboard focus: Tab and Shift+Tab move in drawing order, skipping holds, `BootNoticeClose` and `InputNoticeClose` (`~146-166`); Enter or Space presses the focused control (`~167`).
- Device pointer (`faraday/shells/stick/src/pointer.rs`, `faraday/shells/stick/src/cursor.rs`): `Motion::{Move, Button, Finger(Down/Up/Cancel), Pan, PanEnd, Wheel, Back, Forward}` become `Pointed::{Touch, Reclick, Scroll, ScrollEnd, Wheel, Back, Forward, Hover}`. Mouse Back and Forward thumb buttons act on press only.
- `--pointer-speed` scales the cursor.

## Effects

- Scripted presses (`press`, `press_as`, `faraday/faraday-core/src/pointer.rs:~675, 681`) forget the entry probe, set the Inbox's typed-at, `note_secrets`, `note_here`, allow inline jobs, `later_all`, `refresh_text_files`, `backup_settle`. `slow::guard` runs on every press (`~716-722`, debug only).
- `after_input` (`faraday/faraday-core/src/pointer.rs:~768-870`) drops, when leaving a screen unless the next screen keeps it (`kept`, `~800-806`): the vault back-to, Add a key's visit return, the keygen card (`keygen_drop`), `bip85.clear`, `vanity.clear`, `lightning.clear`, `silent.clear`, `wordlist.clear`, `decode.drop_typed`, `decode.drop_nonce_input`, the unlock passphrase (`vaults.clear_unlock`, `~823-829`); `secret_cancel()` on a press that leaves a detour (`~796`). A stick visit's return is kept only while Files is up (`~730-758`).
- Shell (`faraday/shells/stick/src/main.rs`):
  - `touch` (`~1361`) sends `Event::Touch`;
  - `move_arrow` (`~1096`) and `input_ended` hide the arrow when the last pointing device goes;
  - `Arrow` (`~2599-2621`) is the handle input threads use to move the arrow;
  - `collapse_moves` (`~2535-2580`): only the newest touch move is kept, pointer moves are summed, hover and touch moves are dropped when consecutive;
  - `handle_input` (`~2715-2774`): a finger hides the arrow and sends `HoverEnd`.

## State and lock

- `Pointer` (`faraday/faraday-core/src/pointer.rs:~24-54`): the hit lists of the last frame (`hits`, `reach`, `cut`), press, last tap, drag, hover, focus, tips. It holds no secret and is `at_lock::Public` (`~53`): a lock does not wipe it (the process ends at a lock). `forget_drawn` (`~106`) replaces the buffers so a secret-holding screen's hit list goes.
- `focused()` only while the last frame offered it (`~136-138`); a press takes focus off its control (`~200-203`).
- Shell: `Cursor { x, y, width, height, speed, scale, visible, button, tap: Tap, pan_carry, wheel_carry }` for the run; `Pointer` per device; `Parser` per touch device. No secrets. The failed-lock rebuild (`start_again`) does not reset the cursor.

## Reach

- Every screen: the press model decides what every control does.
- Drops the state of Vaults, Add a key, keygen, BIP-85, Vanity, Lightning, Silent payments, the word list and Decode when their screens are left.
- Held-back pointers and touch devices aim through it (`devices-inputs`); the shell's input threads apply believed mouse moves directly to the arrow.
- `offers_live` ignores dimmed controls that only answer with a toast (`faraday/faraday-core/src/pointer.rs:~889-891`, F292); `offered_actions` is testkit-only.

## Rules and refusals

Core (`faraday/faraday-core/src/pointer.rs`):
- A press acts only if the same control is under press and lift (`~580-585`).
- Hold actions act only on the tick, never on release (`faraday/faraday-core/src/action.rs:1152`).
- Double press within 450 ms on a text field selects all (`faraday/faraday-core/src/pointer.rs:591`).
- A long touch press shows a tooltip and drops the press.
- No scrollbar grab in compact layouts.
- Focus skips holds and the two notice close buttons.

Device pointer (`faraday/shells/stick/src/pointer.rs`):
- No acceleration (doc). Thumb buttons act on press only (value 1); a release and key repeats send nothing.
- A left button press cancels a touch and ends a pan. Two fingers scroll only when the button is not held; with the button held every finger moves the cursor.
- Multitouch: the contact that moved furthest counts; a contact's first position moves nothing; 10 slots (`SLOTS`), others ignored; the single-touch fallback only when no multitouch is reported. `HI_RES_PER_NOTCH` 120. A tap is cancelled once per touch and never for a tap-click pad.
- `hi_res_wheel` (`faraday/shells/stick/src/evdev.rs:~269`): only the fine axis counts when present. `left_button` (`~276`): no button means the touchpad clicks by tap.
- Pointer travel and wheel counts saturate.

Cursor (`faraday/shells/stick/src/cursor.rs`):
- `TAP_MS` 180: a touch within it of a lift is a drag or a second tap. `TAP_TRAVEL_PX` 6, scaled by `scale`: a tap that moved further is not a tap. A lift is a tap only if no button is held; a press under a held tap clears the tap with no new Down; a due tap click is not released while the button is held; two fingers while pressed drag like one.
- Wheel: 48 px per 120 units; natural scrolling for pan. Moves send Hover unless a contact is down.
- The cursor is clamped inside the frame, also on resize. `forget_press` drops a press and tap unless a finger still touches.
- Arrow scale (`faraday/shells/stick/src/cursor.rs:485-487`): `(lines + 600) / 1200`, at least 1: 1 under 1800 short-side pixels, 2 from 1800, then whole steps.
- Fractional motion is carried and rounded, clamped to i16 (`whole`). `draw_rows` skips pixels outside the frame or band.

Touchscreen (`faraday/shells/stick/src/touch.rs:214-300`):
- Packets become `Event::Touch` Down, Move or Up in panel pixels, with calibration (`--touch-swap`, `--touch-invert-x`, `--touch-invert-y`), window mapping (`faraday.window=`), rotation (`orient::Shared`) and a grid from `--touch-grid` or the HID range (`hid::ranges(...).for_node(multitouch)`).
- Only slot 0; another slot's multitouch events are ignored; single-touch ABS_X and ABS_Y are ignored once multitouch is seen; a release with no open contact is ignored; no touch before a position is reported.
- A press outside the window is dropped; a dragged finger is held at the edge. A repeat of the last position is not a Move. A grid axis of 0 is read as 1; the HID span is at least 1.

HID ranges (`faraday/shells/stick/src/hid.rs:140-150`):
- Reads the HID report descriptor for touch screen and pen X and Y logical ranges; short items only (long items skipped; a truncated item ends the parse).
- Logical Maximum is unsigned unless Logical Minimum is negative.
- A range is kept only if min < max for both X and Y (`faraday/shells/stick/src/hid.rs:146`).
- Only Inputs inside a Touch Screen or Pen application; the first X and first Y kept; a pen is chosen for a non-multitouch node when present; multitouch uses fingers, else the pen.

## Domain calls

- `faraday-ui` (`ui::BAR_GRAB`), `faraday-core` `motion::SLOP`; `faraday_shell_api::{Event, TouchPhase}` in the shell.
- No crypto, keys, vault or PSBT calls.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 1dc64f08c | Scrolling the Files stick list tinted the top app bar and banded the card. | The list scrolls on its own offset and fades into the card. |
| 069caaf4d | A held mouse was refused its arrow and targets when a trusted or believed keyboard was present, so the mouse could not approve itself. | The test is whether anything trusted can point; the arrow shows from the sheet's first frame. |
| a6c59705c | The stick's mouse Back and Forward thumb buttons did nothing. | Back and Forward act on press only; a release and key repeats send nothing. |
| 425eb6b48 | Pointer travel and wheel counts could overflow. | Counts saturate. |

## Doubts

- a6c59705c and 425eb6b48 came with the stick shell's rows and are carried here.
- 069caaf4d's file list was not fully read.
- Most line numbers in `faraday/faraday-core/src/pointer.rs` and the shell's `faraday/shells/stick/src/main.rs` come from one report and were not re-read; only those written plainly were checked.
- `after_input` drops several areas' state on leaving a screen; which areas are kept by `kept` was not listed screen by screen.
