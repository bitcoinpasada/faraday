# Material 3 checks: how the touch-target test and the press constructor were built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/M3-CHECKS.md` on 2026-10-29. Each keeps its
original number; the source keeps a one-line stub under the same heading.
At the move the plan was built: batches A1 to A3 and B1 to B3.

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
