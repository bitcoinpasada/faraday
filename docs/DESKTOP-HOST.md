# Faraday Demo as a simulator: the host

**Kind:** Spec
**Status:** Built (F118, `faraday/faraday-core/src/host/`): the host holds the app, the strip at the window's foot with the sticks, Samples, Size and every practice aid of the screen on show, the preview frame centred above it, the pointer carried in; the host's colours the app's theme turned inside out, the screen's aids in a section of their own colour, the strip's row centred and scrolling sideways where it is wider than the window, its controls 48 dp tall whatever the window and the preview (§2, §3); Samples' sheet is the host's (§6); at the window's own size the shell copies the app's pixels into the window once (§9); the menus fade in and out (§5); the menus, Samples' sheet and the strip's row scroll under a finger and coast after it, a tap pressing once (§5); a stick the app asks for has its pill marked, pulsing (§3); the app's frame carries a focus border (§4); the die's values are in the entry at the click (§3.2); the app draws no stick pill, Load samples, Preview size or "Demo only" card.
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
| A control's outline; the scroll indicator | the host scheme's outline |
| A preview frame's focus border (§4) | the host scheme's primary while the pointer or the keys reach the app, its outline-variant while they do not |
| This window's focus border (§4), standing on the app's own edge | the app's primary, its outline-variant |
| A stick pill the app asks for (§3) | a 2 dp outline in the app's inverse primary over a layer of it |
| A control whose menu is open | filled in the app's inverse primary, its label in the inverse surface |
| The space round a preview's frame | the host scheme's surface |
| The screen's aids' section (§3) | the host scheme's tertiary container, its text and outlines on-tertiary-container |
| The step's main press there (codex32's Next, Roll) | filled in tertiary, its label on-tertiary |
| A menu; Samples' sheet | the host scheme's surface container, an outline-variant edge, on-surface text, on-surface-variant for headings, sizes and kinds |
| The sheet's main press (Load ticked); a tick | primary, on-primary |
| A stick's dot | OK while it is in, the outline while out |
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
   out, so a pulled one can be put back. A stick the app asks for (the
   Pull sheet asking for every stick in to be pulled, the backup flow
   asking for its next stick; `Pill::asked`) has its pill marked, and
   **Sticks · N** where that stick is folded into it: a 2 dp primary
   outline (`HOST_MARK_EDGE`) over a primary layer that pulses three
   times (`HOST_PULSES`), each pulse Material's extra long 4
   (`HOST_PULSE_MS`, 1 s) rising from the steady highlight
   (`HOST_MARK_LAYER`, Material's dragged state layer) to
   `HOST_PULSE_LAYER` and back, then holds the steady highlight; under
   Reduce motion and with fades off (`docs/MOTION.md` §3.5) the steady
   highlight alone. A newly asked stick pulses again; the mark ends when
   the stick is moved as asked or the sheet closes.
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
window), the section's head (the mode's name: "Practice", or
"Testnet" where Paste words is the one aid) and the die go, the
sticks' pill reads **Sticks** and More reads the mode's name. The controls never shrink below
48 dp: where even then the row is wider than the window less its 16 dp
margins (a phone held upright, the Pi panel as a window), it starts at
the margin and scrolls sideways, under the wheel or a finger dragged
along it (a drag past the slop presses nothing, and coasts on when the
finger lifts while it moves, §5), with a scroll
indicator along the strip's foot, so its end is always reached.

Off Practice (Testnet, Signet, Regtest) the strip holds the sticks and Size
only: Practice is the one condition for every aid (`docs/DEMO.md` §14).

### 3.1 Every practice element and where it goes

Every row of `docs/DEMO.md` §14.1 leaves the app area for the strip.

| Screen | Element | In the strip |
|---|---|---|
| Add a key; the Spend tab's Type the words | **Use a sample** | button |
| Add a key | **Use faraday** (BIP-39 passphrase) | button |
| Add a key's words, a Restore slot's, New key's typed key | **Paste words**: the clipboard's words go into the words whole (F75), in Practice and on the test networks; Ctrl+V does the same there | button |
| New key, a typed key | **Use a sample** | button |
| New key, the Key card | **Use faraday** (passphrase, repeat); "Lock in adds sample … in place of these words" | button; line |
| New key, the dice entry | the computer's die: the die, **Roll**, Rolls per click | the roller (§3.2) |
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
(`fresh/roller/screen.rs`, `die_alone`) in the host scheme (§2); then
**Roll** (filled as the step's main press, dimmed once the entries are
full; **Stop** only while a click waits for this computer's randomness)
and **Rolls per click · 1**, which opens a menu of
1, 5, 10, To the end and Number (Number takes the count typed on the
keyboard, which the control then shows). A click's values, one or
many, are in the entry in the frame that shows the press (owner,
2026-10-25); the die then tumbles for Material's short 4
(`ROLLER_ONE_TUMBLE_MS`, 200 ms) and shows the last value, not at all
under Reduce motion. The strip grows to the die's
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

A focus border says where the pointer and the keys go: `HOST_FOCUS_EDGE`
(2 dp) round the app's frame, in This window on the app's own edge
above the strip (drawn over the app's frame as the host shows it), for
a preview just outside the frame. It is primary while the pointer is
over the app's frame, or a key has reached the app since the pointer
last left it, and outline-variant once the pointer goes to the strip,
a menu, the host's space round a preview or out of the window, and
while the window has lost the keyboard to another (`Event::Focus`, which
the desktop shell sends); the window opens with the keys the app's. It
changes colour over Material's short 4 (`HOST_FOCUS_FADE_MS`) on the
standard curve, at once under Reduce motion and with fades off. Its
colours are §2's: the app's own roles in This window, where it stands
on the app, the host scheme's round a preview.

## 5. The menus

Each stands up from its control on the strip: Material's menu, 280 dp
(`HOST_MENU_W`), rows 48 dp (`HOST_MENU_ROW`) with title-medium labels,
fading in over Material's medium 1 on the standard decelerate and out
over its short 3 on the standard accelerate (owner, 2026-10-25), at
once under Reduce motion and with fades off (`docs/MOTION.md` §3.5).
A row takes a press while the menu fades in, none while it fades out. A press outside closes it and does nothing
else, a press on the menu between its rows does nothing; a choice acts and closes it. A menu taller than the room
above the strip stands as tall as that room, less the 16 dp margin, its
rows scrolling inside it under the wheel and a finger. Sticks: every stick with its dot.
Size: the sizes with their dp, the one shown checked. Play options: "Play
runs" and "Speed" as headings over their choices, the chosen checked.
More: the aids with no room (and Samples, narrowest). Rolls per click:
1, 5, 10, To the end, Number, the chosen checked.

**Samples' sheet** opens and closes as the menus do, fading, above the
strip's top, in the host's look (§2): the title, "In Practice, the sample passphrase is
faraday", where a stick is in the line that it must be pulled, Select
all (Unselect all while any is ticked), the sample wallets and keys as
48 dp rows with a tick box, a loaded one ticked and dimmed, taking no
press, and what each is at the row's end; Not now and Load ticked (the
filled press, dimmed until one is ticked) pinned at its foot, the lists
scrolling between under the wheel and a finger where the room above
the strip is short. A tick and Select all keep it open; Not now, Load ticked, Back
and a press beside it close it, a press beside it doing nothing else.
It stands in the host's own space where there is room: beside a
preview's frame where the host's space there holds a menu's width
(`HOST_MENU_W`), at most `HOST_SHEET_W` (360 dp), on Samples' side
first; else under the frame's foot where it fits there; else, as in
This window, which leaves no host space above the strip, up from
Samples over the app's foot. Load ticked runs the app's own load: with
a stick in, the app's Pull sheet asks for it out, as on the device.

**Under a finger** (owner, 2026-10-25: a touch laptop's finger could not
scroll Samples). A finger, or the mouse, pressed on a menu's rows,
Samples' sheet's lists or the strip's row where they scroll, and moved
past the slop (8 dp, `motion::SLOP`, the app's), scrolls them and
presses nothing; lifted while it moves, they coast and slow; pulled
past an end, they stretch and spring back; a touch while they coast
only stops them; under Reduce motion they follow the finger and stop at
its lift. It is the app's own scrolling (`Motion`, `docs/MOTION.md`
§3.3), one for the menu or sheet open and one for the strip
(`host/scroll.rs`), and it asks for frames only while it moves (§9). A
press that does not move is a tap. In the app's frame a finger is the
app's, as on the device's panel: its pages scroll and coast, and the
backup board's chips are dragged into places by a finger as by the
mouse. The window (`faraday-window` `input.rs`, the Starter's too)
gives a touchscreen's first finger to the app as its press, on Wayland,
X11 and Windows alike, and drops the pointer moves and the left press
and release a system makes of a touch (Windows' mouse messages of a
touch, X11's pointer moved to the first finger) while a finger is down
and for 500 ms after it lifts (`FINGER_ECHO`), so a tap presses once
and a tap on the top strip's room does not move the window. Every host
control and menu row is 48 dp tall (§2).

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
- **The host's menus fade** (owner, 2026-10-25): Material's fade
  pattern, on its short and medium durations and standard easing, in
  place of the menu's own unrolling, which read as heavy over the app.
- **A stick the app asks for pulses a few times, then holds** (owner,
  2026-10-25, a pulse; the count is this pass's): Material asks for
  emphasis drawn once, and a pulse that ran for as long as a sheet waits
  would keep the window drawing.
- **The focus border stands on the app's edge in This window**: the
  window leaves no room round the app there, and the app laid out 4 dp
  smaller would no longer be the device at the window's size.
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
words, gone on Home; Paste words in the strip in Practice and on
Testnet (`paste_a_seed.rs`); Make codex32 shares' walk in the strip at three
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
nothing. A menu fading in stands part of the way over the app a few
frames in, and whole at once under Reduce motion. The stick the app
asks for: its pill marked while the Pull sheet waits and not once the
stick is pulled, steady and asking for no frame under Reduce motion;
with motion, pulsing a while, then holding its mark with the window at
rest; in the Pi preview the pulse's frames place the app's frame not
once; Sticks · N marked where the stick asked for is folded into it.
The focus border: primary at the start, outline-variant with the
pointer on the strip or out of the window, primary again over the app
or after a key, outline-variant while the window has lost the keys;
the preview's border in the host scheme; with motion, part of the way
a few frames after the pointer leaves, then at rest. A Roll from the
strip, one value or five, in the entry with no tick between the press
and the frame, the die tumbling a moment after. The idle window asks
for no frame after a hover onto the strip and back (`an_idle_demo_asks_for_no_frame_at_any_size`).
Under a finger: a drag that starts on a sample's tick box scrolls
Samples' sheet with the row and ticks nothing, a tap after ticking it;
in a window too short for the Size menu a drag brings its last row into
view, choosing nothing, and a tap there chooses it; a flick on the
sheet coasts on after the lift, a frame each tick, settles, and the
window then asks for no frame; a touch while it coasts stops it and
ticks nothing; under Reduce motion it stops at the lift; a flick along
the narrowest strip coasts, opens nothing, and the window rests. In
`faraday-window`: a finger presses once and a second finger makes
nothing; the press a system makes of a touch is dropped until 500 ms
after the lift.

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

At rest the host asks for no frame. A tick asks for one only while the
host's own motion runs (a menu or Samples' sheet fading in or out, the
focus border changing colour, a stick pill's pulses) and for the frame
after that shows it at rest (`Host::drawn_moving`); the app asks
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

The host's motion is light. A menu or Samples' sheet fading copies the
pixels it stands on once a frame and blends them with what it drew
(`Canvas::copy_region`, `Canvas::fade_region`), for its own rectangle
alone. The focus border is four thin fills a frame. A stick pill's
pulse redraws the strip alone: at the window's own size the host draws
nothing but the strip anyway, and where it holds the whole window (a
preview) a frame that only the pulse asked for, with nothing else
changed since the whole frame was drawn and no menu open, draws the
strip again over the frame it holds, clipped to the strip, and neither
clears the window nor places the app's frame (`Host::paint`). After
every event the host reads which sticks the app asks for
(`Faraday::stick_pills`), a lookup.
