# Faraday Demo as a simulator: the host

**Kind:** Spec
**Status:** Built (F118, `faraday/faraday-core/src/host/`): the host holds the app, the strip at the window's foot with the sticks, Samples, Size and every practice aid of the screen on show, the preview frame centred above it, the pointer carried in; the host's colours the app's theme turned inside out, the screen's aids in a section of their own colour, the strip's row centred and scrolling sideways where it is wider than the window, its controls 48 dp tall whatever the window and the preview (§2, §3); Samples' sheet is the host's (§6); at the window's own size the shell copies the app's pixels into the window once (§9); the app draws no stick pill, Load samples, Preview size or "Demo only" card.
**Open:** the website demo's shell holding the app in the host (in the website's own repository); planned.

The owner asked (2026-10-25) that Faraday Demo read as a simulator of
the device, so it is always clear what is the app and what is Faraday
Demo's or Practice's. The app area shows exactly what the device shows
(F117); everything that exists only because this is a desktop window or
a demo stands outside it, in a strip drawn in the host's own look.

## 1. The window

| Size shown | The app area | The strip |
|---|---|---|
| This window (the default) | the window less the strip, laid out as a device of that size would be | `HOST_STRIP_H` (64 dp) along the window's foot |
| A preview (Pi panel, 7-inch panel, …; `docs/PREVIEW-SIZE.md` §1) | the screen's size in dp, centred in the host's space above the strip with a pixel-wide edge, at its own pixels where it fits inside `MARGIN_WIDE`, scaled down where the window is smaller | the same, at the window's foot, never inside the frame |

The app is told the app area as its display and knows nothing else:
the class, short and low rules, the navigation and the on-screen
keyboard follow as on that screen. The strip is the window's: its
height and its controls' size are set by the window alone, so a Pi
panel previewed in a 1920 × 1080 window has the same strip as the
window itself. It grows only while the page shows the computer's dice
roller, to the roller's row with a gap above and below (§3.2).

## 2. The host's look

The strip, its menus, Samples' sheet and the space round a preview's
frame are the host's, not the app's: they never take the app's surface
or its components. Their colours are the app's theme turned inside out,
so that the host stands in stark contrast to the app whatever the theme,
and change with it (theme, Light or Dark, contrast level; THEMES, F113):
a light app has a dark host, a dark app a light one. The strip is
Material's inverse surface with inverse on-surface text; everything that
stands on or round it takes its role from the scheme of the same theme
in the opposite mode at the same contrast level (the host scheme), each
pair one Material makes to read:

| Part | Role |
|---|---|
| The strip | the app's inverse surface |
| Its labels; a chevron | the app's inverse on-surface |
| A control's outline; the preview frame's edge; the scroll indicator | the host scheme's outline |
| A control whose menu is open | filled in the app's inverse primary, its label in the inverse surface |
| The space round a preview's frame | the host scheme's surface |
| The screen's aids' section (§3) | the host scheme's tertiary container, its text and outlines on-tertiary-container |
| The step's main press there (codex32's Next, Roll) | filled in tertiary, its label on-tertiary |
| A menu; Samples' sheet | the host scheme's surface container, an outline-variant edge, on-surface text, on-surface-variant for headings, sizes and kinds |
| The sheet's main press (Load ticked); a tick | primary, on-primary |
| A stick's dot | OK while it is in, the outline while out; a warning edge while a flow asks for it |
| Not available now | its ink at Material's disabled opacities |
| Hovered, pressed | Material's state layers in the control's ink |

Controls are Material's 48 dp (`HOST_CONTROL_H`, the touch target, the
press their whole box) with labels in title medium (`HOST_LABEL`,
16 sp), the stick pills fully rounded and the rest at the medium shape;
the strip is `HOST_STRIP_H`, the controls with 8 dp above and below.
What the host writes reads on its fill in every theme and scheme, as
the app's text does (4.5:1, 7:1 at High contrast).

## 3. The strip, left to right

The strip holds two kinds of control. Those always there come first:

1. **The stick pills**: the boot stick first, then every stick with its
   state ("Stick 2 · out"); a press puts it in or pulls it. Where they
   do not all fit beside the rest, the boot stick and as many as fit,
   then **Sticks · N** opening a menu of every stick that exists, in or
   out, so a pulled one can be put back.
2. **Samples**, in Practice with samples set: opens Load samples' sheet,
   the host's (§6).
3. **Size · <size shown>**: opens the Size menu (§5), which lists the
   sizes with their dp, the one shown checked.
Then, only on a screen or flow that offers them, its practice aids in a
section of their own colour (§2), so it is plain they came with the
screen at hand:

4. **Practice**, then the screen's practice aids (§3.1); those with no
   room go into **More**; then, in the room left, the lines that say
   what Practice puts in place of the person's own ("Rolls d20",
   "Practice puts sample … in place of these strings"), cut to one line.

The row is centred in the window.

Where the window is too narrow for all of it, the strip gives way in
this order (Material's overflow: the actions that fit, the rest in a
menu): the sticks fold into **Sticks · N** after the boot stick and as
many as fit; **Size** loses its size's name, which its menu shows
checked; the aids that do not fit go into **More**, the step's main
press (Next, Roll) keeping its place; the sticks stand as **Sticks · N**
alone; Samples joins More; narrowest (the Pi panel's 268 dp as a
window), the "Practice" head and the die go, the sticks' pill reads
**Sticks** and More reads **Practice**. The controls never shrink below
48 dp: where even then the row is wider than the window less its 16 dp
margins (a phone held upright, the Pi panel as a window), it starts at
the margin and scrolls sideways, under the wheel or a finger dragged
along it (a drag past the slop presses nothing), with a scroll
indicator along the strip's foot, so its end is always reached.

Off Practice (Testnet, Signet, Regtest) the strip holds the sticks and Size
only: Practice is the one condition for every aid (`docs/DEMO.md` §14).

### 3.1 Every practice element and where it goes

Every row of `docs/DEMO.md` §14.1 leaves the app area for the strip.

| Screen | Element | In the strip |
|---|---|---|
| Add a key; the Spend tab's Type the words | **Use a sample** | button |
| Add a key | **Use faraday** (BIP-39 passphrase) | button |
| New key, a typed key | **Use a sample** | button |
| New key, the Key card | **Use faraday** (passphrase, repeat); "Lock in adds sample … in place of these words" | button; line |
| New key, the dice entry | the computer's die: the die, **Roll** (Stop while it rolls), Rolls per click | the roller (§3.2) |
| Make codex32 shares, the entry | **Use a sample**; the walk's **Previous** (back through the computer's steps and the person's own faces alike, `docs/CODEX32.md` §5.5), **Next**, **Play** (Pause); **Play options** (how far Play runs, how fast); "Rolls dN" | buttons, Next filled; a menu; a line |
| Make codex32 shares, the cards after the entry | "Practice puts sample NAME in place of these strings" | line |
| Vaults, no vault in Files | **Use a sample** | button |
| Create a vault | **Use faraday** for each passphrase and repeat; the Generate with dice roller; "Practice puts faraday in place of these words" | buttons; the roller (§3.2); line |
| Unlock; the boot import's passphrase; a passphrase prompt; the Spend tab's vault passphrase; the backup flow's unlock | **Use faraday** | button |
| Vault contents, an entry's password | **Use faraday** | button |
| Make a new wallet, a slot's passphrase | **Use faraday** | button |
| A seed read's passphrase sheet; the potential wallet's sheet | **Use faraday** | button |
| The scanner, every purpose that takes a sample | **Load sample seed**, **share**, **key**, **wallet**, **PSBT**, **SeedQR**, **code**, **address** | buttons, while the sheet is open |
| The Spend tab: What do you have?, the first step's shortcut, Write the payment | **Load sample PSBT** | button |
| Restore, seeds first, a cosigner's box | **Use a sample** | button |
| Lightning node key, the LND cipher seed | **Use a sample**; **Use faraday** | buttons |
| Explore, Find an address | **Use a sample** | button |
| Back up, Check my copy | **Fill words**; **Use faraday** | buttons |

And from `docs/DEMO.md` §14.2: **Load samples** leaves the top bar for
the strip's **Samples**; the mode pill (Practice · Testnet) and its sheet
stay in the app, as the device has them; the dimmed actions, the line
"Never type a seed that holds real bitcoin here." and the toast after
Lock in stay where they are, being the app's own.

How the strip learns them: a page hands each button and line to the
frame where it would have drawn it (`Ui::demo_offer`, `Ui::demo_line`),
as for the card; while held, the frame notes them after drawing
(`SamplesArea::note_aids`) and the strip reads the note. Make codex32
shares' walk and the rollers are read from the flow's state
(`codex32/walk_screen.rs` `host_aids`, `host_play_options`;
`Faraday::demo_hosted_now`).

### 3.2 The roller in the strip

The computer's one die (New key's dice entry, Create a vault's
Generate with dice; `docs/CODEX32.md` §5.6) stands first among the
aids in their section: the die itself, drawn by the roller's own code
(`fresh/roller/screen.rs`, `die_alone`) in the host scheme (§2),
tumbling while a click rolls; then **Roll** (**Stop**
while a click rolls; filled as the step's main press, dimmed once the
entries are full) and **Rolls per click · 1**, which opens a menu of
1, 5, 10, To the end and Number (Number takes the count typed on the
keyboard, which the control then shows). The strip grows to the die's
height with its bounce (`ROW_H`, 62 dp, and an 8 dp gap above and
below) only while the die shows; the app is laid out above it.

## 4. Inside the app area

The mode pill and the ? stay: the device has them. The top bar's stick
pills, the Pi panel's pill strip, Load samples, Settings' Preview size,
every "Demo only" card and column and every practice aid drawn in a page
are gone (F117): the app area is the device's screen. Practice stays a mode
of the app (the pill reads Practice · Testnet); only its aids move. Faraday
Demo's own differences that F117 keeps (sticks as folders, Quit,
Transfer, Make PDF, paste, the seed line, the wordmark) stay in the app.

## 5. The menus

Each opens up from its control on the strip: Material's menu, 280 dp
(`HOST_MENU_W`), rows 48 dp (`HOST_MENU_ROW`) with title-medium labels, in on the emphasized
decelerate over medium 1, out on the emphasized accelerate over short 3,
at once under Reduce motion. A press outside closes it and does nothing
else; a choice acts and closes it. Sticks: every stick with its dot.
Size: the sizes with their dp, the one shown checked. Play options: "Play
runs" and "Speed" as headings over their choices, the chosen checked.
More: the aids with no room (and Samples, narrowest). Rolls per click:
1, 5, 10, To the end, Number, the chosen checked.

**Samples' sheet** opens and closes as the menus do, from the strip's
top, in the host's look (§2): the title, "In Practice, the sample passphrase is
faraday", where a stick is in the line that it must be pulled, Select
all (Unselect all while any is ticked), the sample wallets and keys as
48 dp rows with a tick box, a loaded one ticked and dimmed, taking no
press, and what each is at the row's end; Not now and Load ticked (the
filled press, dimmed until one is ticked) pinned at its foot, the lists
scrolling between under the wheel where the room above the strip is
short. A tick and Select all keep it open; Not now, Load ticked, Back
and a press beside it close it, a press beside it doing nothing else.
It stands in the host's own space where there is room: beside a
preview's frame where the host's space there holds a menu's width
(`HOST_MENU_W`), at most `HOST_SHEET_W` (360 dp), on Samples' side
first; else under the frame's foot where it fits there; else, as in
This window, which leaves no host space above the strip, up from
Samples over the app's foot. Load ticked runs the app's own load: with
a stick in, the app's Pull sheet asks for it out, as on the device.

## 6. Design calls taken

- **Material 3's menu** for Size, Sticks, Play options and More: an app
  bar's or toolbar's overflow is a menu; a single choice closes it.
- **Samples' sheet is the host's** (owner, 2026-10-25): it is not the
  device's, so it stands outside the device's screen where the window
  leaves room (§5). Its shape is Material's side sheet (a standard
  sheet's 360 dp at most) rising from the strip, with Material's
  checkbox rows and its menu's open and close motion.
- **Bare host with `--panel`**: the window is a device's screen, so no
  strip and no preview; the app draws its own pills as a device does.
- **The host's colours are the app's theme turned inside out**
  (owner, 2026-10-26): Material's inverse roles for the strip, and the
  opposite mode's scheme of the same theme and contrast level for what
  stands on and round it (§2), so the host is told from the app at a
  glance in every scheme and changes with it.
- **The aids' section is a tertiary container** (owner, 2026-10-26:
  a container role of its own colour): Material's tertiary is the
  accent for what balances primary and secondary and draws attention,
  apart from the strip's neutral inverse surface.
- **48 dp controls with title-medium labels at every window size**
  (owner, 2026-10-26, from Material 3): the strip grows to hold them and
  scrolls sideways rather than shrink them; the row is centred.
- **Size gives up its size's name before an aid leaves the strip, and
  the step's main press is the last aid to go into More**: Material's
  overflow keeps the actions and shortens labels first; the size shown
  stays in the Size menu, checked.

## 7. What the app no longer draws

The "Demo only" card and its pane, column and foot strip (the page's
and the sheet's, and Make codex32 shares' own with its More and Less);
the top bar's stick pills and their Sticks menu sheet on Faraday
Demo; Load samples beside the mode pill; Settings' Preview size; the
desktop shell's own framing of a preview. The dice sheet's column at
the right (`docs/LAYOUT.md` §16.1) stays: it is the entry's own, not the
demo's.

## 8. Tests

`faraday/faraday-core/tests/all/desktop_host.rs`: in Practice the strip
holds the sticks, Samples and Size and the app none of them, the mode
pill staying in the app, Samples opening its sheet; on Testnet the strip
holds the sticks and Size alone, on Add a key too; a stick pulled and
put back from its pill and from the Sticks menu; Size showing the Pi
panel framed above the strip, a press inside reaching the tile under
it, one beside it nothing, This window laying the app out again; a menu
closed by a press beside it that goes no further; Add a key's Use a
sample and Use faraday in the strip alone, Use a sample filling the
words, gone on Home; Make codex32 shares' walk in the strip at three
sizes, Next stepping it from one place, the step's count in the app's
Working area; Play's options choosing a speed; New key's die and
Generate with dice's in the strip, Roll rolling; Rolls per click's menu
choosing five, one Roll then making five; at the narrowest window every
control inside it and Samples reached from the Practice menu, its sheet
inside the window; held bare, no strip. Samples' sheet: the host's, the
app opening none and drawing none of it, standing above the strip, a
tick keeping it open and Load ticked loading the key and closing it,
the loaded key taking no press after; with a stick in, Load ticked
opening the app's Pull sheet; in the Pi preview the sheet clear of the
frame, a press on it keeping it and one on the frame closing it and
reaching nothing. The window as the shell shows the host's parts the
same as its whole frame at 1366 × 768 and 1280 × 800, with the app's
frame handed over at the window's size, placed under a menu and in the
Pi preview; More's rows offered to a script at the clock's very start.
The host's look: its colours follow the app's theme through every
scheme, the strip the app's inverse surface, darker than a light app
and lighter than a dark one, at 3:1 or more against the app's surface,
and so the space round a preview's frame; what the host writes (Home's
strip, the Size menu, Samples' sheet, Add a key's aids, New key's die
and Roll) reads on its fill in every scheme. The strip's controls
centred at 1280 and 1920; 48 dp tall at 1280 × 800, 1920 × 1080,
2560 × 1440, a half screen and a phone held upright, the Pi previewed
keeping the window's strip; at the narrowest the row scrolls under the
wheel and a finger, its end reached and More opened, a drag pressing
nothing.

## 9. Speed

The app's own frames cost what they did (`docs/SPEED.md` §7, the late
flow at 1366 × 768 and 2560 × 1600: every screen within the run-to-run
spread). The host adds, per frame, the strip. At the window's own size it
hands the shell its frame in parts (`Parts`, `App::parts`): its own
canvas and the app's frame where it stands, which the shell copies into
the window once each (`faraday-window` `view::copy`), so the app's
pixels are copied once, as before the host. Under a menu or Samples'
sheet, and for a preview, whose small frame costs less to place than
the window round it to split, the app's frame is placed in the host's
canvas first. The host's share of a frame, the strip drawn and the
window filled (release, two runs each, before and after): at
1366 × 768 0.62–0.66 ms before, 0.52 ms after; at 2560 × 1600 at scale
2 3.28–3.70 ms before, 2.44–2.48 ms after, the one copy into the
window's 16 MB; the Pi preview unchanged (0.78–0.79 ms and
3.21–3.22 ms).

The strip with 48 dp controls, the aids' section and the centred row:
the host's frame with the app's unchanged
(release, best of three runs of 300 frames, Practice on Home and on Add
a key, This window and the Pi preview): at 1366 × 768 0.08, 0.11 and
0.35 ms before, 0.11–0.25, 0.16–0.40 and 0.37–0.85 ms after; at
2560 × 1600 at scale 2 0.24, 0.31 and 1.31 ms before, 0.33–0.84,
0.49–1.01 and 1.41–4.72 ms after, the spread the build machine's other
work at the time.

At rest the host asks for no frame. A tick asks for one only while a
host menu or Samples' sheet is part of the way in or out, and for the
frame after that shows it at rest (`Host::drawn_moving`); the app asks
for its own through `poll_command`, and every other event the host
takes still asks for one. Before (2026-10-26), each tick fell through
to a request for a frame, and on Wayland every redraw sends a tick, so
an idle Faraday Demo drew 22–49 frames a second for ever, 50–56 % of a
core on the owner's laptop; of 200 idle ticks it asked for 200 frames,
the app alone none. Now none of 200, at 1892 × 2085, 2560 × 1600,
3840 × 2160 and 1280 × 800 (the probe of `docs/SPEED.md` §12).

At the window's own size with nothing of the host's over the app, the
host draws the strip alone on a canvas of its own, the window's width
and the strip's height at the window's scale (`Host::strip`,
`Canvas::part`), and hands it to the shell standing at the window's foot
(`Parts::base_y`) with the app's frame above it; the canvas of the
whole window (`Host::canvas`) is made only for a preview, under a menu
or Samples' sheet, or for `App::frame`, and dropped again when none of
them is shown. The host's own canvases at 1892 × 2085: 709 KiB, where
the whole window's was 15,409 KiB; at 2560 × 1600 1,280 KiB, where it
was 16,000 KiB. What else the window holds at that size: the app's
canvas (about 14.4 MiB) and the window's two Wayland buffers (16 MiB
each, softbuffer's pools), so the buffers come to about 47 MiB where
they were about 61. The allocator may keep the room a dropped canvas
held for the next one, so the process's resident size after a menu
has been open can stay where it was.
