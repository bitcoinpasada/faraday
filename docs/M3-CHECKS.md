# Material 3 checks: touch targets and state layers

**Kind:** Reference
**Status:** Built (F152)

Two of Material 3's rules that Faraday follows (F91, F92, F108) are
today held by review alone: a press is taken on at least 48 × 48 dp, and
every control shows its hover, focus and pressed states as a state
layer. This spec makes the first a test over the snapshot tours, with a
count that only falls for what is under the floor today, and the second
a property of the API: a pressable region cannot be made without the
layer that shows its state.

Checked against Material 3's Foundations pages on accessibility (target
size: 48 × 48 dp; as Android's accessibility documentation states it)
and on interaction states (state layers for hover, focus, pressed and
dragged; F113's opacities). m3.material.io's pages are script-drawn; the
values used are the ones F108 and F113 already record.

## 1. How a press is registered today

Every pressable region of a frame, in Faraday and in the Starter, goes
through one function: `Ui::hit` in `faraday/faraday-ui/src/lib.rs`. No
other code pushes to `Ui::hits`. `Ui::hit`:

1. drops the region when its action is dimmed (`Ui::dimmed`);
2. cuts it to the clip (or `Ui::reveal`'s clip) and notes a control cut
   at a side (`Ui::cut`);
3. draws the default state layer (on-surface, `SHAPE_S` corners, over the
   box given) and the focus ring, unless the caller said it draws its
   own (`hit_quiet`, `hit_drawn`; §4.3) or the
   region covers a quarter of the screen or more;
4. pushes the box drawn to `Ui::hits` and calls `Ui::reach_for`
   (`faraday/faraday-ui/src/controls.rs`), which grows it, centred, to
   `TOUCH` square in pixels (`core/faraday-draw/src/tokens.rs`) inside the
   clip and, where it would overlap an earlier target, cuts both back to
   the middle of the gap between their controls (F108). The result is a
   `Reach` (target, drawn, action) in `Ui::reach`.

After the frame the app keeps both lists in its pointer
(`faraday/faraday-core/src/pointer.rs`, `Pointer::set_hits`), letting go
of a target whose control is no longer among the hits as drawn. A press
lands on the topmost control drawn under it, else on the target it falls
in (`Pointer::hit_at`). Tab moves through `reach` in drawing order.

So what a person can press is exactly `reach` after the frame: each
control's target, at least its drawn box. A target under the floor is a
control a finger misses or takes for its neighbour, which is what the
test of §2 reports.

Five screens trim `Ui::hits` themselves after drawing, to a scrolled
pane's clip (`board/screen.rs`, `codex32/input_panel.rs` twice,
`codex32/pane_screen.rs`, `codex32/worksheet_screen.rs`, under
`faraday/faraday-core/src/`), and two take a mark and truncate
(`flow.rs`, `menu.rs`). A hit trimmed this way loses its target in
`set_hits` and is pressed on its box alone; it is a control cut by a
clip, which §2.2 does not count. faraday-ui's clip method does it now (§4.3).

## 2. Touch targets: the test

### 2.1 What it reads

No new state on the app. Two small additions, both read-only:

- `Reach` gains `cut: bool`: the control's box as asked was cut by its
  clip (scrolled part-way out of its region, or under a reveal). Set in
  `Ui::hit`, where the uncut box is already computed for `Ui::cut`.
- `Faraday::drawn_targets()`: each press the last frame offers, in
  drawing order, as `(Action, target, drawn, cut)` in pixels: the
  pointer's `reach`, and for a hit with no target left (trimmed, §1) its
  box, marked cut. A lookup, beside `Faraday::hit_box` in
  `faraday/faraday-core/src/lib.rs`.

The floor in pixels is `TOUCH` times the frame's pixels per unit
(`pixels_per_unit_of`, `faraday/faraday-core/src/drawing.rs`), rounded as
`Ui::px` rounds; a target is under it when its width or height is more
than one pixel short (rounding at a fractional scale).

### 2.2 What counts

A target is counted when it is under the floor and:

- its control is not cut by a clip (§2.1): a row half scrolled out is
  pressed by scrolling it in, as a person does;
- it is not an exception listed in §3 at that size class;
- it is not `hit_around`'s press beside a sheet, which has no target
  (already true: `Ui::hit` keeps it out of `reach`).

A dimmed control takes no press and is not in the lists.

### 2.3 Where it runs

A unit test module of the snapshot shell, `targets` in
faraday/shells/snapshot/src/targets.rs (`faraday-snapshot` is a binary;
nextest runs a binary's unit tests). The tours run as `just faraday-snap`
runs them (`faraday/shells/snapshot/src/tour.rs`), with two switches on
the tour driver, `Tour`, which is the tool's state, not the app's:

- `pictures: false`: `Tour::save` writes no PNG and no mark file;
- `targets: Option<Vec<Small>>`: at each frame `Tour::save` takes (one
  tour step), the counted targets of §2.2 are collected.

The test kit is written into a temporary directory from
`faraday_core::testkit::files()`, as `faraday-testkit` writes it, so the
test needs nothing under `out/`.

One test function a size, each running every named tour and the default
tour at that size at Standard text:

| Test | Size | Why |
|---|---|---|
| `targets::small_240x320` | 240 × 320 at 143 dpi | the smallest the design supports |
| `targets::small_480x640` | 480 × 640 at 286 dpi | the Pi's 2.8-inch panel |
| `targets::stick_1024x600` | 1024 × 600 at 170 dpi | the stick's 7-inch panel |
| `targets::desktop_1280x800` | 1280 × 800 at 160 dpi | the desktop window, one dp a pixel |

Larger text only grows controls (`docs/TEXT-SIZE.md` §5) and is left to
the review run below. A measuring run (§5) took 5 to 11 s for the default
tour at one size on the build machine, PNGs included; A1 times the whole
set and, if one size's test passes a minute, splits it by tour.

`just faraday-snap` and `just faraday-snapshots` also write
`targets.txt` beside the pictures, one line per counted target, as they
write `cut.txt` (`docs/TEXT-SIZE.md` §7), for review at the other sizes.

### 2.4 What it reports

Each counted target on its own line:

```
desktop_1280x800 default 41-backups Backups (no sheet) Nav(Backups): target 208 × 24 px, floor 48; drawn 208 × 40 at (16, 312)
```

the test, the tour, the step (the picture's number and name, as the
pictures are named), the screen and the open sheet, the action, the
target and the floor, and the control as drawn. This is what a person
would notice, named so the frame can be rendered and looked at; it
states no design value beyond the floor.

### 2.5 The ratchet

Today's count is far from zero (§5), so the test holds a count that only
falls, as `lint-tokens` does: tools/touch-targets.counts, one line per
size, screen and action kind (the action's variant, its fields left
off), with how many distinct actions of that kind are under the floor
there:

```
desktop_1280x800 Backups Nav 7
```

The test fails when a line's count rises or a key is new (listing the
targets of §2.4 that make it), and when a count falls (lower the line, so
the ratchet only tightens). A rise is the owner's decision, recorded in
DECISIONS (`structure.ratchet-raised`, §7). Writing the file again is
`FARADAY_TARGETS_WRITE=1` on the test, for a change that lowers counts.

## 3. Exceptions

A target under the floor that cannot be larger on that layout is listed
here with the decision that allows it; the test reads the same table
(a constant beside the test, by action kind and size class). The owner
allowed these three (§8, Q2; F152).

| Case | Action kinds | Size classes | What the measuring run found | Allowed by |
|---|---|---|---|---|
| The on-screen keyboard and word entry's keys: ten keys across a 240 to 268 dp panel | `Osk`, `EntryKey`, `EntryKeyBack`, `EntryShift` | small: the 240 × 320 and 480 × 640 panels | 22 to 25 × 26 px against a 43 px floor at 240 × 320; 45 to 52 × 52 px against 86 at 480 × 640 | F152 |
| The codex32 worksheet's cells, a grid that must be read whole | `Codex32` (worksheet cells) | every class | 28 to 30 × 30 px against 48 at 1280 × 800 | F152 |
| The backup board's chips, 28 and 20 dp, sharing the gap with their neighbours | `Board` (chips) | every class | 38 × 32 to 40 px against 48 at 1280 × 800 | F108 (its touch targets line) |

Everything else under the floor is a defect, on every layout, the
desktop's included (§8, Q3): it is counted in the ratchet and
fixed by spacing or sizing the controls: the desktop sidebar's and the
lists' rows drawn 40 dp or less and stacked with no gap (`Nav`,
`Resume`, `OpenWallet`, `ExploreKey`, `Vault`, `Step`), the small
panel's step rows (`KStep`, `CStep`, `RStep`), word entry's candidates
and the wide keys at 1280 × 800 (`EntryCandidate`, `EntryKey`).

F108's split stays: it keeps a press from reaching the wrong control. A
split target under the floor still counts; the cure is room between the
controls, not a smaller target.

## 4. State layers by construction

### 4.1 Today

Moved to `docs/history/M3-CHECKS-LOG.md`.

### 4.2 The constructor

A new module, faraday/faraday-ui/src/press.rs, is the only way to make a
region pressable. Each function registers the press (hits, target, Tab
order, tooltip anchor as today) and draws its state layer and focus ring
in one call; neither can be left out, because neither is public any
more.

```rust
/// A control's shape: where its state layer and focus ring go, design units.
pub struct Shape { pub x: f32, pub y: f32, pub w: f32, pub h: f32, pub r: f32 }

/// How the focus ring follows the shape: whole, or one segment of a
/// segmented button, square where it meets another.
pub enum Ring { Whole, Segment { first: bool, last: bool } }

/// A press with no state layer, and the case that allows it. Each
/// variant names its case; a new one is the owner's decision.
pub enum Plain {
    /// Beside an open sheet: its way out (`hit_around`).
    Beside,
    /// A menu's or sheet's own body: a press there closes nothing.
    Body,
    /// The page itself, a quarter of the window or more.
    Page,
}

/// A press with no pressed layer (hover and focus kept), and the case
/// that allows it.
pub enum Unpressed {
    /// A held secret: what it shows reads as drawn under the finger.
    HeldSecret,
    /// An on-screen keyboard's key: a watcher reads nothing from a
    /// press (F152, Faraday's difference from Material).
    Keyboard,
}

impl<A: Copy + PartialEq> Ui<'_, A> {
    /// Layer in `ink` over `shape`, ring round it, target `shape` grown to TOUCH.
    pub fn press(&mut self, shape: Shape, ink: Paint, action: A);
    /// As `press`, the target the larger box `target` (an icon button, a nav pill).
    pub fn press_to(&mut self, shape: Shape, target: (f32, f32, f32, f32), ink: Paint, action: A);
    /// As `press`, the ring one segment's.
    pub fn press_segment(&mut self, shape: Shape, ring: Ring, ink: Paint, action: A);
    /// Hover and focus layers and the ring, no pressed layer, for one of
    /// Unpressed's cases.
    pub fn press_unpressed(&mut self, shape: Shape, ink: Paint, why: Unpressed, action: A);
    /// No layer, for one of Plain's cases.
    pub fn press_plain(&mut self, at: (f32, f32, f32, f32), why: Plain, action: A);
    /// Today's default, unchanged: on-surface over the box, SHAPE_S corners.
    pub fn press_box(&mut self, x: f32, y: f32, w: f32, h: f32, action: A);
}
```

The layer is drawn where the call is made, so a component calls it after
its container and before its content, as `hit_shaped` is called now
(Material: the layer sits between container and content). `ink` is the
content's colour (`Style::label` for a button). Hover is Faraday's
(F108, F113): every function draws it where the pointer rests.

`press` of a shape a quarter of the window or more fails a
`debug_assert!` naming `press_plain(.., Plain::Page, ..)`, so the quarter
rule of today is said at the call, not applied silently.

### 4.3 What becomes private

`hit`, `hit_quiet`, `hit_drawn`, `hit_ringed`, `hit_shaped`, `hit_held`,
`hit_around`, `state_layer`, `focus_ring` become `pub(crate)` in
faraday-ui (the components there are built on them), and the fields
`Ui::hits` and `Ui::reach` private, with:

- `Ui::presses_mark()` and `Ui::presses_rewind(mark)`, for `flow.rs`'s and
  `menu.rs`'s truncation and `glance/sheet.rs`'s;
- `Ui::clip_presses_since(mark, clip)`, for the five screens that trim
  their hits to a pane (§1), which also trims their targets so they keep
  a target where it is in the pane;
- `Ui::take_presses()`, for the frame's hand-over to the pointer
  (`faraday/faraday-core/src/lib.rs`).
- `Ui::drop_presses_inside(area)` and `Ui::cover_presses(area)`, for a
  pane drawn over a page (`screens/learn_pane.rs`) and a surface drawn
  over its foot (`codex32/input_panel.rs`), which take the page's presses
  under them.

`Ui::press*` and the mark, rewind, clip, take, drop and cover methods
are built, with `Ui::hits` and `Ui::reach` private; `hit`, `hit_quiet`,
`hit_drawn`, `hit_around`, `state_layer` and `focus_ring` are
`pub(crate)`. A control whose layer is not its box (a switch's handle, a
menu item's inset ring) lays it in `Ui::press_custom`, crate-private to
faraday-ui, which still takes the press with it. The keyboard's keys are
`Ui::key_in`. A plain press is `Ui::press_box`; a press beside an open
sheet is `Ui::press_plain(.., Plain::Beside, ..)`.

The compiler then holds the rule: no code outside faraday-ui can make a
press without its layer, nor draw a layer apart from its press.

### 4.4 Migration

Moved to `docs/history/M3-CHECKS-LOG.md`.

## 5. Today's count

Moved to `docs/history/M3-CHECKS-LOG.md`.

## 6. Batches

Moved to `docs/history/M3-CHECKS-LOG.md`.

## 7. The registry when built

Moved to `docs/history/M3-CHECKS-LOG.md`.

## 8. Questions for the owner

Moved to `docs/history/M3-CHECKS-LOG.md`.

