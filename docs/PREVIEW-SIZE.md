# Preview size: Faraday Desktop at another screen's size

**Kind:** Reference
**Status:** Built (F110)

Faraday Desktop shows its screens at any of the layout's screen sizes
(`docs/LAYOUT.md` §2) at the press of a choice, so a person can see and
try Faraday as it looks on the Pi's panel, a 7-inch panel, a
half-screen window or a full monitor. The owner asked for it
(2026-10-23).

## 1. The control

Settings gains a **Preview size** card after Display scale, in Faraday
Desktop only. Its choices, as a row of chips that wraps:

| Choice | Size (dp) | Class (LAYOUT §2) |
|---|---|---|
| This window (the default) | the window's own | by the window |
| Pi panel | 268 × 358 | compact, short |
| Narrow window | 400 × 800 | compact |
| Half screen | 760 × 1000 | medium |
| 7-inch panel | 960 × 580 | expanded, low |
| 10-inch panel | 1370 × 860 | large |
| Desktop window | 1280 × 800 | large |
| Full monitor | 1920 × 1080 | extra-large |

Under the chips, the size the screens are laid out at, in dp (after
the person's Display scale). The sizes are `PREVIEW_SIZES` in
`core/faraday-draw/src/tokens.rs`.

Settings, not the top app bar: Material 3 puts a preference that
changes how the whole app is shown in Settings, and keeps the top app
bar for the current screen's own actions. The top app bar is also not
over every page (LAYOUT §3), and a control there would be one more
thing on the Pi panel's 358 dp. Settings is reached at every size, so
the way back to This window is always there.

## 2. What it does

The chosen size is given to the app as its display, in place of the
window's (`faraday/shells/desktop/src/main.rs`, `Shell::display`): a
screen that many dp in size at the window's own density. Everything
the layout reads from a display follows from that, exactly as on that
screen: the class, the short and low rules, the navigation (LAYOUT
§3), one column or two, and on the Pi size the on-screen keyboard
rising by itself where a field takes typing. No screen knows it is
previewed.

The frame is drawn centred in the window with a pixel-wide edge, at
the window's own pixels where it fits inside a 24-unit margin, and
scaled down to fit where the window is smaller (each window pixel the
average of the frame's pixels it covers;
`faraday/shells/window/src/preview.rs`). Resizing the window moves and
scales the frame and does not lay the screens out again.

The pointer is carried into the frame: a press, a hover, a scroll or
the wheel inside it reaches the frame's pixel under it; a press begun
outside does nothing; a press begun inside follows the pointer to the
frame's edge. The keyboard works as in the window.

## 3. Where it is offered

Only Faraday Desktop offers it. The desktop shell sends
`Event::PreviewOffered` (`core/faraday-shell-api`) after the display
and after each change; the core shows the card only once it has had
one, and asks for a size with `Command::PreviewSize`. The stick, the
Pi and the Starter send no offer and show no card; started with
`--panel`, whose window already stands for a device's screen, the
desktop offers none either.

The size is the shell's: a lock, which starts a fresh app in the same
window, keeps it; a new launch opens at This window. It is in no
settings file and no kept state.

## 4. Tests

`faraday/shells/desktop/src/main.rs`: Settings offers the sizes;
choosing the Pi panel asks the shell for it, lays Home out with the
Pi's menu, one column and short; a press inside the frame, centred in a
1280 × 800 window, opens the Vaults tile under it.
`faraday/shells/window/src/preview.rs`: the Pi panel sits centred at
its own size in a 1280 × 800 window and a full monitor shrinks to fit;
a press inside lands on the frame's pixel under it, one outside and its
release do nothing.
