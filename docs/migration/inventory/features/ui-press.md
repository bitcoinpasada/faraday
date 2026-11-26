# Presses and hits

The frame context of the component layer (`faraday-ui`) and how a drawn
control becomes pressable. Every app (Faraday Offline, Faraday Demo, the
Starter) draws each frame through one `Ui<'a, A>` over a `Canvas`; each
control registers its press region, its state layer (hover, focus, press)
and its focus ring through the press constructors here. It is not a screen:
every screen and component reaches it.

## Screens and sheets

None of its own. It draws on every screen:
- State layers: content colour at `STATE_PRESS`, `STATE_FOCUS` or `STATE_HOVER` opacity over a pressed, focused or hovered control (`faraday/faraday-ui/src/controls.rs:~283-295`).
- Focus ring: `FOCUS_RING` thick, `FOCUS_OFFSET` outside the control, in `Role::Secondary`, when the keyboard is on the action (`faraday/faraday-ui/src/controls.rs:~300-310`); a segment ring for segmented buttons, round at the button's ends and square where segments meet (`faraday/faraday-ui/src/controls.rs:~316-353`).
- Faded controls: regions refused while a job runs are faded to the background at `DISABLED_CONTENT` (`fade_later`/`fade_disabled`, `faraday/faraday-ui/src/lib.rs:~1882-1910`).

## Actions

- Pointer down, move and up on a press region give the region's action (`A`); hover is not reported while pressed (`is_hovered`/`is_pressed`, `faraday/faraday-ui/src/lib.rs:~2033-2043`).
- Tab and Shift+Tab move focus through drawn controls in the order drawn, wrapping (`focus_step`, `faraday/faraday-ui/src/controls.rs:~46-65`).
- A press at a pixel goes to the target found last-drawn-first (`reach_at`, `faraday/faraday-ui/src/controls.rs:~69-75`).
- A press beside a box (sheet dismiss) goes to the box's outside action (`hit_around`, far zones of 10000 units, `faraday/faraday-ui/src/lib.rs:~2007-2019`).

## Effects

None outside the app's state. Writes the frame's hit list, reach list,
faded list and tip anchors, and pixels of the canvas.

## State and lock

- `Ui<'a, A>` (`faraday/faraday-ui/src/lib.rs:~381-652`): one frame's drawing context; `f` pixels per unit, `ox`/`oy` offsets, `theme`, and per-frame fields: hits, pressed, hovered and focused action, edit caret, fields, scrolled regions, pins, sheet and frost, flow card, pane, `dimmed`, `dimmed_answer`, `keep_live`, `reach`, tips, demo items.
- Data types: `Edit<A>` (caret and selection in chars, `faraday/faraday-ui/src/lib.rs:~295-337`); `FieldAt<A>` (per-line char edges for caret placement, `faraday/faraday-ui/src/lib.rs:~339-365`); `Glyph` (`faraday/faraday-ui/src/lib.rs:~367-379`); `W` face choice Regular, SemiBold, Mono (`faraday/faraday-ui/src/lib.rs:~283-293`).
- Press types (`faraday/faraday-ui/src/press.rs`): `Shape` (box plus corner radius, `faraday/faraday-ui/src/press.rs:~18`), `Ring` (Whole or Segment, `faraday/faraday-ui/src/press.rs:~49`), `Plain` (Beside, Body, Page, `faraday/faraday-ui/src/press.rs:~64`), `Unpressed` (HeldSecret, Keyboard, `faraday/faraday-ui/src/press.rs:~76`), `PressMark` (saved counts of hits and reach, `faraday/faraday-ui/src/press.rs:~87`).
- `Reach<A>` (target, drawn, action, cut flag, grown rect, `faraday/faraday-ui/src/controls.rs:~27-40`).
- `Ui::clear_hits` clears hits, reach, faded and tip anchors for a frame, so what is under a sheet has no press or tip (`faraday/faraday-ui/src/controls.rs:~272-277`).
- `Ui.held` (`faraday/faraday-ui/src/lib.rs:572`, set false at `faraday/faraday-ui/src/lib.rs:727`) is never true in the app: its only setter `hit_held` is test-only.
- Holds no secrets. The Ui is rebuilt each frame; a lock or restart keeps nothing of it.

## Reach

- Used by every control: buttons and choices (`ui-controls`), fields (`ui-text`), menus and sheets (`ui-sheets`), navigation (`ui-navigation`), regions (`ui-regions`, which shift hit and reach rects by the overscroll stretch), tooltips (`hit_tip`).
- Reads the theme (`ui-theme`) for layer colours and the tokens (`tokens`) for opacities and ring sizes.
- F292 (a dimmed action the app still answers) is the decision behind `answer_hit`.

## Rules and refusals

- A press is taken only inside the current clip; a press box cut by the clip is recorded in `cut` (`faraday/faraday-ui/src/lib.rs:~1948-1952`).
- A state layer is drawn only when the drawn rect is under a quarter of the screen (`faraday/faraday-ui/src/lib.rs:~1959-1961`).
- A dimmed action is one in `dimmed`, in `dimmed_answer`, or refused by `keep_live` (`is_dimmed`, `faraday/faraday-ui/src/lib.rs:~1912-1919`); `keep_live` closes everything else while a job runs (`faraday/faraday-ui/src/lib.rs:~1914-1918`, F292).
- A dimmed action takes no press unless the app answers it (`dimmed_answer`, `answer_hit`, `faraday/faraday-ui/src/lib.rs:~1921-1936`); dimmed regions go to the fade list (`faraday/faraday-ui/src/lib.rs:~1882-1910`).
- `press_custom`: a dimmed action draws nothing, takes no hit, is faded later and may be answered; otherwise it draws and then registers the hit (`faraday/faraday-ui/src/press.rs:~195-209`, `faraday/faraday-ui/src/press.rs:~201-204`).
- Touch floor: a drawn control is grown to `Ui.touch` px inside its clip (`reach_for`, `faraday/faraday-ui/src/controls.rs:~239-268`, `faraday/faraday-ui/src/controls.rs:~240-249`); a target never leaves its clip except as it is cut (`faraday/faraday-ui/src/controls.rs:~249-250`).
- Two overlapping targets are cut back to the middle of their gap (`split`, `faraday/faraday-ui/src/controls.rs:~80-115`, `faraday/faraday-ui/src/controls.rs:~252-259`).
- Focus wins over hover: `press_unpressed` washes at `STATE_FOCUS` if focused, else `STATE_HOVER` if hovered, and draws no pressed layer (`faraday/faraday-ui/src/press.rs:~160-170`); it discards its `why` argument, both cases draw the same (`faraday/faraday-ui/src/press.rs:156-159`).
- A segment's layer is widened past its inner ends so it is square where segments meet (`faraday/faraday-ui/src/press.rs:~137-150`).
- `Plain::Beside` uses `hit_around`; `Plain::Body` and `Plain::Page` use `hit_drawn` (no layer) (`faraday/faraday-ui/src/press.rs:~176-182`).
- `clip_presses_since`: hits empty after the clip are dropped; reach entries are cut and marked `cut` (`faraday/faraday-ui/src/press.rs:~231-255`).
- `drop_presses_inside` and `cover_presses` touch only `hits`, not `reach` (`faraday/faraday-ui/src/press.rs:265-292`).
- The held-secret exception (no pressed layer while `held && is_pressed`, `faraday/faraday-ui/src/lib.rs:1964`) never applies in the app: `hit_held` is compiled only under `#[cfg(test)]` (`faraday/faraday-ui/src/lib.rs:1981-1988`), its only caller is `faraday/faraday-ui/src/read_tests.rs:546`; a held secret gets the normal pressed layer.
- Every press is drawn through faraday-ui's press constructor, so its state layer and focus ring are always drawn (history, 9ce43bb3a).
- Presses under a covering surface: `drop_presses_inside` (a pane over a page) and `cover_presses` (a surface at a page's foot) edit the hits only (faraday-ui/src/press.rs:265-292); at the frame's end `Pointer::set_hits` keeps a touch target only while its control is still among the hits as drawn (faraday-core/src/pointer.rs:120-124), so a covered or cut control loses its target and its place in the Tab order.
`Unpressed::HeldSecret` is the press of every eye (show or hide a secret): hover and focus only, no pressed layer; `press_unpressed` does not read which case it is (faraday/faraday-ui/src/press.rs:156-173, faraday/faraday-core/src/compact/mod.rs:523-534, called from compact/mod.rs:509, vault/screen/secret.rs:46, explore/screen.rs:439, keygen/dice_strip.rs:482, vault/screen.rs:245). Only the `Ui.held` path is dead.
`Plain::Page` has no user and no size test: `press_plain` takes it as `Plain::Body` (`hit_drawn`); the quarter-of-the-screen rule is `hit`'s alone, which gives a state layer and ring only to a rect under a quarter of the screen (faraday/faraday-ui/src/press.rs:61-71, 176-182, faraday/faraday-ui/src/lib.rs:1958-1971).

## Domain calls

- `faraday_draw::tokens::{STATE_PRESS, STATE_FOCUS, STATE_HOVER, FOCUS_RING, FOCUS_OFFSET, DISABLED_CONTENT, SHAPE_S, TOUCH, alpha_of}` (`core/faraday-draw/src/tokens.rs`).
- `Canvas` fill, stroke and clip calls (`core/faraday-draw/src/canvas.rs`).
- Press constructors in `faraday/faraday-ui/src/press.rs`: `Ui::press` (`faraday/faraday-ui/src/press.rs:~95`), `press_to` (`faraday/faraday-ui/src/press.rs:~101`), `press_head` (`faraday/faraday-ui/src/press.rs:~110`), `press_segment` (`faraday/faraday-ui/src/press.rs:~130`), `press_unpressed` (`faraday/faraday-ui/src/press.rs:~156`), `press_plain` (`faraday/faraday-ui/src/press.rs:~176`), `press_box` (`faraday/faraday-ui/src/press.rs:~186`), `press_custom` (`faraday/faraday-ui/src/press.rs:~195`), and the bookkeeping `presses_mark` (`faraday/faraday-ui/src/press.rs:~213`), `presses_rewind` (`faraday/faraday-ui/src/press.rs:~222`), `clip_presses_since` (`faraday/faraday-ui/src/press.rs:~231`), `take_presses` (`faraday/faraday-ui/src/press.rs:~259`), `drop_presses_inside` (`faraday/faraday-ui/src/press.rs:~265`), `cover_presses` (`faraday/faraday-ui/src/press.rs:~278`).
- Hit variants in `faraday/faraday-ui/src/lib.rs`: `hit` (`faraday/faraday-ui/src/lib.rs:~1941-1981`), `hit_quiet` (`faraday/faraday-ui/src/lib.rs:~1991-1997`), `hit_drawn` (`faraday/faraday-ui/src/lib.rs:~1999-2005`), `hit_around` (`faraday/faraday-ui/src/lib.rs:~2007-2019`), `reveal`/`unreveal` (clip drawing while the hidden part stays pressable, `faraday/faraday-ui/src/lib.rs:~2021-2031`).

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 9ce43bb3a | Several presses (backup board zones, codex32 sheet boxes and worksheet cells, codex32 backup chips, randomness area, others across core and the Starter) drew part of a press state layer or none: no hover, focus or pressed layer and no focus ring. | Every press is drawn with faraday-ui's press constructor, so its state layer and focus ring are always drawn. |
| 2be654675 | At the older press sites (Home's tiles and lead card, Tools' tiles, Wallets' start cards and rows, the small panel's Home, the bar's eye, Files' stick chip, Select all and ticking rows, a Kind card's rows, the backup board's + stick, + place, stick boxes, place name and x buttons, the Starter's rows and checklist) the press's hover, focus and pressed layers were drawn over the content; the files' ⋮ showed two focus rings. | A press's state layer sits between a control's container and its content (Material: Interaction states); a chip with a press draws it through `Ui::chip_press`; the files' ⋮ draws only the one ring every press draws (`faraday/faraday-ui/src/lib.rs:2463`, `faraday/faraday-core/src/menu.rs:113-125`). |

## Doubts

- Callers were not traced past the component crates (faraday-ui and faraday-draw); the Reach section rests on the component code and a few searches, not on a read of every screen.
- Line numbers in this file are from one pass of the inventory and were not re-checked except those written without `~`.
