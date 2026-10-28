# Preview size: Faraday Demo at another screen's size

**Kind:** Reference
**Status:** Built (F110, amended by F118)

Faraday Demo shows its screens at any of the layout's screen sizes
(`docs/LAYOUT.md` §2) at the press of a choice, so a person can see and
try Faraday as it looks on the Pi's panel, a 7-inch panel, a
half-screen window or a full monitor. The owner asked for it
(2026-10-23).

## 1. The control

**Size · <size shown>** in Faraday Demo's host strip, at the
window's foot outside the app (F118, `docs/DESKTOP-HOST.md` §3, §5),
opens a menu of the sizes, each with its size in dp, the one shown
checked:

| Choice | Size (dp) | Class (LAYOUT §2) |
|---|---|---|
| This window (the default) | the window's own | by the window |
| Pi panel | 268 × 358 | compact, short |
| Narrow window | 400 × 800 | compact |
| Half screen | 760 × 1000 | medium |
| 7-inch panel | 964 × 565 | expanded, low |
| 7-inch panel, portrait | 565 × 964 | compact |
| 10-inch panel | 1365 × 853 | large |
| 10-inch panel, portrait | 853 × 1365 | expanded |
| Desktop window | 1280 × 800 | large |
| Full monitor | 1920 × 1080 | extra-large |

The sizes are `PREVIEW_SIZES` in `core/faraday-draw/src/tokens.rs`. The
7-inch and 10-inch panels are the stick's (`docs/ORIENTATION.md` §10):
1024 × 600 at 170 dpi and 1280 × 800 at 150 dpi, each either way round.
Out of Settings and out of the app (owner, 2026-10-25): the size is
Faraday Demo's, not the device's, so it stands with the host's other
controls; the strip is under every screen, so the way back to This
window is always there.

## 2. What it does

The host (`faraday-core/src/host/`) gives the app a display of the
chosen size in dp at the window's own density, in place of the window
less the strip. Everything the layout reads from a display follows from
that, exactly as on that screen: the class, the short and low rules,
the navigation (LAYOUT §3), one column or two, and on the Pi size the
on-screen keyboard rising by itself where a field takes typing. No
screen knows it is previewed.

The frame is drawn centred in the host's space above the strip with a
pixel-wide edge, at the window's own pixels where it fits inside a
24 dp margin, and scaled down to fit where the window is smaller (each
window pixel the average of the frame's pixels it covers;
`Canvas::place_scaled`). Resizing the window moves and scales the frame
and does not lay the screens out again.

The pointer is carried into the frame: a press, a hover, a scroll or
the wheel inside it reaches the frame's pixel under it; a press begun
outside does nothing; a press begun inside follows the pointer to the
frame's edge. The keyboard works as in the window.

## 3. Where it is offered

Only Faraday Demo's host offers it; the stick, the Pi and the
Starter have no host, and with `--panel`, whose window already stands
for a device's screen, the desktop holds the app bare, with no strip.
The host tells the shell the size chosen (`Command::PreviewSize`), and
the shell gives it back after a lock, which starts a fresh app in the
same window (`Event::PreviewOffered`); a new launch opens at This
window. It is in no settings file and no kept state.

## 4. Tests

`faraday/faraday-core/tests/all/desktop_host.rs`: Size opens the menu
and the Pi panel lays Home out with the Pi's menu, compact and short,
the frame centred above the strip; a press inside it opens the Vaults
tile under it, one beside it nothing; This window lays the app out again
in the window less the strip.
