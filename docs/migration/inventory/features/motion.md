# Scrolling, motion and fades

How pages move: finger pans and flicks, the wheel, rubber-band stretch at the ends, the overlay scrollbar, rows moved instead of redrawn, toasts, cross-fades between pages and glides between rectangles. It applies to nearly every page, pane and sheet, on every product, and it follows the Motion and Speed choices in Settings.

## Screens and sheets

- No screen of its own.
- Scrolled regions: nearly every page, pane and sheet (`screen_place`, `faraday/faraday-core/src/motion.rs:~1343-1426`). Compact-only regions: CheckMessage, Vaults, Unlock, VaultContents, Tools. Contained flow checks for Invoice, Decode, CheckMessage, NonceCheck, Tools (`faraday/faraday-core/src/motion.rs:1347-1408`).
- Overlay scrollbar: shown for 800 ms after the last move, fades over 300 ms (`faraday/faraday-core/src/motion.rs:~241-253`).
- Toast (snackbar): text and expiry, rises and fades over `TOAST_SHOW_MS` (`faraday/faraday-core/src/out.rs`).

## Actions

- Pan follows the finger. A flick coasts (`COAST_TAU_MS` 325, `faraday/faraday-core/src/motion.rs:~22`), only at or above 0.02 px/ms. Past an end the content stretches and springs back.
- Drag starts past `SLOP` 8 (`faraday/faraday-core/src/motion.rs:31`). On pages that move only up and down, it starts only when the move is more vertical than horizontal.
- Wheel: one notch is 56 units (`WHEEL_NOTCH`, `faraday/faraday-core/src/motion.rs:45`), from the shells' 48 px notch (`SHELL_NOTCH_PX`, `faraday/faraday-core/src/motion.rs:51`).
- Reduce motion (`rigid`): content follows fingers; no coast, no stretch; wheel and keys jump.
- Codex32 worksheet: arrow keys pan or glide 56 units (`faraday/faraday-core/src/motion.rs:~941-960`); Shift+wheel pans across; a worksheet at its end hands the move to the page. With the word-list sheet open, any key goes to `wordlist.key`.
- Page changes: `page_through` (`faraday/faraday-core/src/fade.rs:~220`), the pair and guide glides.

## Effects

- Shell commands only: `Moved` frames and `Command` values (`core/faraday-shell-api`).
- Host calls from the fade: `set_edge_drawn_over`, `take_changed`, `paints_next`.
- `Out` (`faraday/faraday-core/src/out.rs`): the command queue, the `storage_out` queue (which can carry file bytes), redraw flags, the toast.

## State and lock

- `Motion` per region (`faraday/faraday-core/src/motion.rs:~91-116`); `Scroll` on `Faraday` (`:~489-539`). `Scroll` is `at_lock::Public` and holds no secret.
- Offsets clamp to whole device pixels and to the extent last drawn (`:~1630-1639, ~718-733`).
- Settings sections open at their top; a list pane starts at its top; Backups opens on its first wallet (`:~903-920`). Sheets start at their top each time they open (`:~769-774`).
- `Fade` (`faraday/faraday-core/src/fade.rs`) keeps pixel copies of the frames it blends (`from`, `after`, `surface`, `frost`). `Fade::end` clears all but `frost`; `display_changed` clears `frost`. Marked `Public`.
- A frame that showed a secret (`secret_drawn`, `step_secret`, `secret_shown`) is not kept for a fade: the next change is drawn at once, or a live fade redraws both cards from state (`faraday/faraday-core/src/fade.rs:~600-615, ~748, ~874-890`).
- A lock does not clear `Fade` or `Scroll` (confirmed at `faraday/faraday-core/src/fade.rs:164, 193-205, 395`). Every lock ends in a fresh process (`faraday/faraday-core/src/vault/seal.rs:933-946`), so neither outlives it.
- `Out` is declared to hold nothing a lock wipes.

## Reach

- Read by every scrolled page and sheet; by the screen frame (`screen-frame`), the pane container's page turn (`pane-container`), the guide's glide (`guide`) and the codex32 worksheet (`codex-shares`).
- Uses `glide::through` and `glide::eased`, `address_check::pane::scanning` and the `contained_*` checks (`faraday/faraday-core/src/motion.rs:200, 1329, 1350-1402`).
- Codex32 worksheet follow reads `disclosed.moving` (`faraday/faraday-core/src/motion.rs:~1189-1216`).
- Settings' Motion and Speed choices (`settings-time`).

## Rules and refusals

- Motion Off (`reduce_motion`) gives no fade (`fades()`, `faraday/faraday-core/src/fade.rs:~1068`).
- A frame that showed a secret is never reused for a fade (see State).
- Frost keep is skipped on a spoiled pass (`faraday/faraday-core/src/drawing.rs:~919-922`). `spoiled` is a layout flag, not a secrecy flag.
- Codex32 worksheet follow keeps the work in view after entry, stops on a person's own move, and does nothing while a card opens or closes (`faraday/faraday-core/src/motion.rs:~1107-1232`).
- Moved frames (`faraday/faraday-core/src/moved.rs`): a scroll step moves rows already drawn and paints only the rows it exposes. A frame is reused only for the same screen, slot, theme, scale, text size and size (`:~75`). A composition that does not compose gives a whole frame (`:~168`). Speed feature only.
- Glide (`faraday/faraday-core/src/glide.rs`): pure arithmetic. `through` fades the old page out while `e < STEP_FADE_OUT`; rectangles are interpolated. No state.

## Domain calls

- `core/faraday-draw`: tokens and easing.
- `core/faraday-shell-api`: `Moved`, `Command`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 8dc38cf10 | A scrolled page's edge fade striped: a translucent fill was rounded once per pass, so a fade over a fill of another tone was uneven. | A fade over a fill of another tone falls steadily; a compact Settings page fades into its card. |
| 74fc0bda0 | A page that begins at the top of the frame with no bar over it drew a top edge fade where there is nothing to fade under. | The top edge fade is drawn only when there is a bar or a sheet, or when the page is scrolled from the top. |
| ca52458e9 | A page that follows its work stopped gliding: the stop of other regions' motion ran at each follow. | The glide to a section is the page's own; what moved the screen left behind stops at once. |
| f98d45234 | On a Pi's HDMI screen the first start drew fades the mode could not draw smoothly. | A first start on a screen the HDMI mode cannot draw smoothly starts with Reduce motion on; a choice in Settings still wins; the dev image no longer forces faraday.window, so the full monitor mode is drawn (F222). |

## Doubts

- Which is authoritative after a lock, the `Scroll` offsets or the screens' own offsets, is not known.
- `Out.storage_out` can carry file bytes, yet `Out` is declared to hold nothing a lock wipes. That holds only if the queue is drained before a lock; when the shell drains it was not checked.
- `own_motion` in `faraday/faraday-core/src/fade.rs:~629` is always false (a dead toggle).
- `faraday/faraday-core/src/moved.rs` has `cfg(test)` counters in non-test code.
- `faraday/faraday-core/src/fade/bands.rs` was read by no one.
- Rows left out as not about motion: dfdf7341c (the secret sheet's "Stays in the Outbox" row), b96e16ed6 (an Outbox file replacing another of the same name), 4a4df10e1 (a completed spend's part-signed PSBT left in the Outbox). They came under the command queue's heading.
