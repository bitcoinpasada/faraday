# Material 3 checks: touch targets and state layers

**Kind:** Spec
**Status:** Specified (F152); A1, A2, A3 and B1 built (the touch-target test, its registry rule and the Starter's frames; the press constructor in faraday-ui)
**Open:** of §6, the press constructor's use in faraday-core and the Starter and the hit functions made private (B2 and B3)

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
   own (`hit_quiet`, `hit_drawn`, `hit_ringed`, `hit_held`; §4) or the
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
clip, which §2.2 does not count. B1 moves these into faraday-ui (§4.3).

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

`Ui::hit` draws the default layer unless the caller opts out. The
opt-outs (`hit_quiet`, `hit_drawn`, `hit_ringed`, `hit_held`) trust the
caller to draw its own with `Ui::state_layer` and `Ui::focus_ring`, which
are public; nothing checks that it does. A survey of the 56 opt-out call
sites (faraday-ui, faraday-core and faraday-starter; none in `core/`):

| Group | Sites | Where |
|---|---|---|
| Draw press, focus and hover layers and the ring | 37 | faraday-ui's components: `button_drawn` (`button`, `button_in`), `field_action`, `fab`, `hit_shaped` (`word_pill`), `segmented`, `outlined_icon_button`, `theme_chip`, `switch`, `hold_button`, the menu's item, `list_item`, `stick_pill`, `status_chip`, the top app bar's back and actions, the navigation bar's and rail's items, `tabs`; the Starter's link, mode, tick rows and Home rows; in faraday-core the board's menu, chips and vault head, the compact rows, the backup row, Learn's pane button, the wallet card, Files' chip and row, the app's menu |
| Partial | 6 | the desktop host's button, menu row, Select all link and row (`host/screen.rs`: press and hover, no focus layer); `step_circle` (`faraday/faraday-ui/src/parts.rs`: pressed only, and only unticked); the slider's stops (`faraday/faraday-core/src/ui.rs`: the thumb grows held, no hover or focus layer) |
| None, no case named | 10 | the backup board's six zone boxes (paper, place, stick, vault, soft stick, list; `board/screen.rs`); the codex32 sheet cell and worksheet cell; the codex32 backup chip; the randomness screen's pointer area |
| None, by design | 3 | the desktop host's menu and sheet bodies ("shows no state of its own"); the eye's held secret (`hit_held`, no pressed layer over the secret it shows) |

Besides these, the default layer doubles where a screen washes its own
look and then calls `Ui::hit`: Modes' rows (`modes/screen.rs`), the
backup flow's wallet rows (`backup_flow/wallet_screen.rs`), and the bar's
Cancel (`compact/mod.rs`, a text button with its own layer and ring under
a plain hit: two rings); Stick visit's scrollbar thumb takes a layer it
does not need (`screens/visit.rs`). 121 plain `Ui::hit` calls take the
default.

The on-screen keyboard's geometry says no press highlight shows
(`core/faraday-draw/src/widgets/keyboard.rs`, its anti-observation
defaults), while its keys are drawn with `Ui::button_in`
(`faraday/faraday-ui/src/keyboard.rs`), which draws the pressed layer.
The owner settled it for the geometry's rule (§8, Q1): the keys draw no
pressed layer, keeping hover and focus, Faraday's difference from
Material (F152); B1 makes the keys do so and the geometry's comment say
so.

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

Built so far (B1): `Ui::press*` and the mark, rewind, clip, take, drop
and cover methods, with `Ui::hits` and `Ui::reach` private; the old
`hit*`, `state_layer` and `focus_ring` stay public until B3. A control
whose layer is not its box (a switch's handle, a menu item's inset ring)
lays it in `Ui::press_custom`, crate-private to faraday-ui, which still
takes the press with it. The keyboard's keys are `Ui::key_in`.

The compiler then holds the rule: no code outside faraday-ui can make a
press without its layer, nor draw a layer apart from its press.

### 4.4 Migration

| Sites | Change | Kind |
|---|---|---|
| 21 in faraday-ui | the components call `press*` in place of their own `state_layer` + `focus_ring` + `hit_drawn` or `hit_ringed` | mechanical |
| 16 in faraday-core and the Starter that draw their layer properly | the same: their `state_layer` and `hit_ringed` become one `press_to` | mechanical |
| 6 partial | `press_to`; the host's own washes go, the slider's thumb takes `press` on the handle (Material's slider: the layer on the handle) | a visible change: the focus layer appears |
| 10 with none | `press`/`press_to` (the board's zones, the codex32 cells, the codex32 backup chip); the randomness screen's pointer area `press`, or `press_plain(Page)` where it covers a quarter of the window | a visible change |
| 3 by design | `press_plain(Body)` twice, `press_unpressed(HeldSecret)` once | mechanical |
| the on-screen keyboard's keys (`faraday/faraday-ui/src/keyboard.rs`, through `Ui::button_in` today) | a key's own form of the button that calls `press_unpressed(Keyboard)` | a visible change: no pressed layer on a key |
| 4 doubled | the screen's own wash goes, one `press` | a visible change: one layer, one ring |
| 121 plain `hit` | `press_box`, the same drawing | mechanical, by search and replace |
| 8 direct uses of `ui.hits` and `ui.reach` | the mark, rewind and clip methods | mechanical |

## 5. Today's count

Measured once on the build machine with a probe that is not kept: the
counting of §2.1 and §2.2 in the snapshot shell, over eight tour runs at
Standard text. Each cell is distinct actions under the floor (of which
distinct screen and action-kind pairs), controls cut by a clip left out.

| Run | Distinct actions | Screen and kind pairs |
|---|---|---|
| default, 240 × 320 at 143 | 194 | 41 |
| default, 480 × 640 at 286 | 234 | 52 |
| default, 1024 × 600 at 170 | 280 | 49 |
| default, 1280 × 800 at 160 | 245 | 113 |
| codex32, 480 × 640 at 286 | 227 | 4 |
| codex32, 1280 × 800 at 160 | 805 | 5 |
| board, 480 × 640 at 286 | 21 | 4 |
| board, 1280 × 800 at 160 | 78 | 5 |

Over the eight runs, 157 distinct screen and action-kind pairs, of 58
action kinds. The largest groups: the keyboard's and word entry's keys on
the small panels; the codex32 worksheet's cells; the board's chips; on
the desktop the sidebar's rows (`Nav` drawn 40 dp, targets 24 to 40 px),
Resume, OpenWallet and Explore's rows, the vault's and the steps' rows
(39 to 46 px). A1's first run over every tour gives the file of §2.5; it
will be larger than this, since these runs reach fewer screens.

## 6. Batches

Each batch is its own commit with `just` passing at its end.

| Batch | What | Size | Model |
|---|---|---|---|
| A1 | `Reach::cut`, `Faraday::drawn_targets`, the tour driver's two switches, faraday/shells/snapshot/src/targets.rs with the four sizes and the exceptions of §3 as the owner settles them, tools/touch-targets.counts written from the first run, `targets.txt` beside `faraday-snap`'s pictures; timed per size | about 300 lines and the counts file | Sonnet |
| A2 | The registry (§7), docs/CHECKS.md regenerated, F152's Held by line, tools/README.md's line for the counts file, DESIGN §4.13's line naming the test, `docs/DEVELOPING.md` if `targets.txt` is an output it lists | small | Sonnet |
| A3 | The Starter: its snapshot example's frames checked the same way, in faraday-starter's tests, with its own lines in the counts file | about 150 lines | Sonnet |
| B1 | faraday/faraday-ui/src/press.rs; faraday-ui's 21 components moved onto it; the keyboard's keys on `press_unpressed(Keyboard)` and the geometry's anti-observation comment saying hover and focus show, the press does not (§4.1); the mark, rewind, clip and take methods and their 8 call sites; the fields private | about 400 lines changed | Sonnet |
| B2 | faraday-core's and the Starter's 35 opt-out sites and the 4 doubled ones (§4.4); renders of the board, codex32, the host, Modes and the backup flow looked at (`just faraday-snap` at 480x640 and 1280x800 for each) | about 60 sites | Sonnet, the visible changes reviewed by Opus |
| B3 | The 121 plain `hit` calls to `press_box`; the `hit*`, `state_layer` and `focus_ring` functions `pub(crate)`| mechanical | Sonnet |

Order: A1 and A2 first (the test is cheap and stops new defects); B1 to
B3 in order; A3 any time after A1. B1 changes no pixel but the keyboard's pressed key; B2 does,
and carries a `Material: Interaction states` trailer. The fixes that
lower the ratchet (the desktop rows, the step rows, word entry) are
design work for later items, each lowering its lines.

## 7. The registry when built

In tools/impact.toml, with A2:

```toml
[[check]]
id = "touch-targets"
run = "cargo nextest run --locked -p faraday-snapshot -E 'test(/^targets::/)'"
stage = "just test"
catches = "a press target under 48 dp on a frame a snapshot tour reaches, at its four sizes, not in M3-CHECKS §3; a count in tools/touch-targets.counts risen or not lowered"
misses = "frames no tour or example reaches; sizes other than the four (targets.txt, by review); the held app's frames inside the Starter"

[[rule]]
id = "design.touch-targets"
title = "every press target is at least 48 dp, or listed with its case"
source = ["docs/decisions/F152.md", "docs/M3-CHECKS.md §3"]
when = ["**/screen.rs", "faraday/faraday-core/src/screens/**", "faraday/faraday-ui/**", "faraday/shells/snapshot/src/tours/**"]
kind = "held"
held_by = ["touch-targets"]
```

and with B3:

```toml
[[rule]]
id = "design.press-layers"
title = "a press is made only through faraday-ui's press constructor, which draws its state layers"
source = ["docs/decisions/F152.md", "docs/M3-CHECKS.md §4"]
when = ["faraday/faraday-ui/src/**", "**/screen.rs", "faraday/faraday-core/src/**", "faraday/faraday-starter/src/**"]
kind = "held"
held_by = ["clippy"]
misses = "a Plain case chosen where a layer was owed; a layer in the wrong ink"
```

`structure.ratchet-raised` adds tools/touch-targets.counts to its `when`.
`design.material3`'s `misses` drops "48 dp touch targets over the
snapshot tours (planned check)" and keeps "whether the page named is the
right one". F152's line reads `**Held by:** design.touch-targets,
design.press-layers`, and F108's source list in the two rules names it.
`python3 -I -B tools/impact.py docs` writes docs/CHECKS.md again;
`impact.py lint` and `lint-docs` rule 6 pass.

## 8. Questions for the owner

Answered (owner, 2026-10-28):

1. **Q1.** The on-screen keyboard's keys draw no pressed layer, hover and
   focus kept: anti-observation wins over Material's pressed state, and
   F152 records it as Faraday's difference (`Unpressed::Keyboard`, §4.2;
   built in B1).
2. **Q2.** All three exceptions of §3 are allowed: the keyboard's and
   word entry's keys on the 240 × 320 and 480 × 640 panels, the codex32
   worksheet's cells, the backup board's chips (F108).
3. **Q3.** 48 dp holds on every layout, the desktop's included; the
   desktop's rows under it are defects in the ratchet.
