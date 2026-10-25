# Faraday Desktop as a simulator: the host

**Kind:** Spec
**Status:** Built (F118, `faraday/faraday-core/src/host/`): the host holds the app, the strip at the window's foot with the sticks, Samples, Size and every demo aid of the screen on show, the preview frame centred above it, the pointer carried in; the app draws no stick pill, Load samples, Preview size or "Demo only" card.
**Open:** Samples' sheet as a host sheet (§6), the website demo's shell holding the app in the host (in the website's own repository), the copy of the app's pixels into the window's frame on large windows (§9); planned.

The owner asked (2026-10-25) that Faraday Desktop read as a simulator of
the device, so it is always clear what is the app and what is Faraday
Desktop or Demo. The app area shows exactly what the device shows
(F117); everything that exists only because this is a desktop window or
a demo stands outside it, in a strip drawn in the host's own look.

## 1. The window

| Size shown | The app area | The strip |
|---|---|---|
| This window (the default) | the window less the strip, laid out as a device of that size would be | `HOST_STRIP_H` (56 dp) along the window's foot |
| A preview (Pi panel, 7-inch panel, …; `docs/PREVIEW-SIZE.md` §1) | the screen's size in dp, centred in the dark space above the strip with a pixel-wide edge, at its own pixels where it fits inside `MARGIN_WIDE`, scaled down where the window is smaller | the same, at the window's foot, never inside the frame |

The app is told the app area as its display and knows nothing else:
the class, short and low rules, the navigation and the on-screen
keyboard follow as on that screen. The strip grows only while the page
shows the computer's dice roller, to the roller's row with a small gap
above and below (§3.2).

## 2. The host's look

The strip and its menus are the host's, not the app's: the dark surface
round a preview's frame (`SURROUND`, a shade darker for the strip, a
hairline rule along its top and between its groups), light ink in
small type (12 sp, Material's label small), controls outlined and 32 dp
tall (`HOST_CONTROL_H`) in the middle of a 48 dp press, Material's state
layers as light washes. They never take the person's theme, the app's
surface or its components. The step's main press (codex32's Next) is
filled in the light ink. A stick's pill carries a dot, green while it is
in, grey while out, and an amber edge while a flow asks for it; "Demo"
heads the demo aids in the same amber as the mode's caution pill.

## 3. The strip, left to right

1. **The stick pills**: the boot stick first, then every stick with its
   state ("Stick 2 · out"); a press puts it in or pulls it. Where they
   do not all fit beside the rest, the boot stick and as many as fit,
   then **Sticks · N** opening a menu of every stick that exists, in or
   out, so a pulled one can be put back.
2. **Samples**, in Demo with samples set: opens Load samples' sheet.
3. **Size · <size shown>**: opens the Size menu (§5), which lists the
   sizes with their dp, the one shown checked.
4. **Demo**, then the screen's demo aids (§3.1), only those the screen
   or flow on screen offers; those with no room go into **More**.

Where the window is too narrow for all of it, the strip gives way in
this order (Material's overflow: the actions that fit, the rest in a
menu): the sticks fold into **Sticks · N** after the boot stick and as
many as fit; the aids that do not fit go into **More**; the sticks
stand as **Sticks · N** alone; **Size** loses its size's name and
Samples joins More; narrowest (the Pi panel's 268 dp as a window), the
rules and the "Demo" head go, the sticks' pill reads **Sticks** and More
reads **Demo**. Every control stays inside the window.
   After them, in the room left, the lines that say what Demo puts in
   place of the person's own ("Rolls d20", "Demo puts sample … in place
   of these strings"), cut to one line.

Off Demo (Testnet, Signet, Regtest) the strip holds the sticks and Size
only: Demo is the one condition for every aid (`docs/DEMO.md` §14).

### 3.1 Every demo element and where it goes

Every row of `docs/DEMO.md` §14.1 leaves the app area for the strip.

| Screen | Element | In the strip |
|---|---|---|
| Add a key; the Spend tab's Type the words | **Use a sample** | button |
| Add a key | **Use faraday** (BIP-39 passphrase) | button |
| New key, a typed key | **Use a sample** | button |
| New key, the Key card | **Use faraday** (passphrase, repeat); "Lock in adds sample … in place of these words" | button; line |
| New key, the dice entry | the computer's die: the die, **Roll** (Stop while it rolls), Rolls per click | the roller (§3.2) |
| Make codex32 shares, the entry | **Use a sample**; the walk's **Previous**, **Next**, **Play** (Pause); **Play options** (how far Play runs, how fast); "Rolls dN" | buttons, Next filled; a menu; a line |
| Make codex32 shares, the cards after the entry | "Demo puts sample NAME in place of these strings" | line |
| Vaults, no vault in Files | **Use a sample** | button |
| Create a vault | **Use faraday** for each passphrase and repeat; the Generate with dice roller; "Demo puts faraday in place of these words" | buttons; the roller (§3.2); line |
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
the strip's **Samples**; the mode pill (Demo · Testnet) and its sheet
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
aids: the die itself, drawn by the roller's own code
(`fresh/roller/screen.rs`, `die_alone`) in the dark scheme of the
person's theme, tumbling while a click rolls; then **Roll** (**Stop**
while a click rolls; filled as the step's main press, dimmed once the
entries are full) and **Rolls per click · 1**, which opens a menu of
1, 5, 10, To the end and Number (Number takes the count typed on the
keyboard, which the control then shows). The strip grows to the die's
height with its bounce (`ROW_H`, 62 dp, and a small gap above and
below) only while the die shows; the app is laid out above it.

## 4. Inside the app area

The mode pill and the ? stay: the device has them. The top bar's stick
pills, the Pi panel's pill strip, Load samples, Settings' Preview size,
every "Demo only" card and column and every demo aid drawn in a page
are gone (F117): the app area is the device's screen. Demo stays a mode
of the app (the pill reads Demo · Testnet); only its aids move. Faraday
Desktop's own differences that F117 keeps (sticks as folders, Quit,
Transfer, Make PDF, paste, the seed line, the wordmark) stay in the app.

## 5. The menus

Each opens up from its control on the strip: Material's menu, 280 dp
(`HOST_MENU_W`), rows 48 dp (`HOST_MENU_ROW`), in on the emphasized
decelerate over medium 1, out on the emphasized accelerate over short 3,
at once under Reduce motion. A press outside closes it and does nothing
else; a choice acts and closes it. Sticks: every stick with its dot.
Size: the sizes with their dp, the one shown checked. Play options: "Play
runs" and "Speed" as headings over their choices, the chosen checked.
More: the aids with no room (and Samples, narrowest). Rolls per click:
1, 5, 10, To the end, Number, the chosen checked.

## 6. Design calls taken

- **Material 3's menu** for Size, Sticks, Play options and More: an app
  bar's or toolbar's overflow is a menu; a single choice closes it.
- **Samples' sheet stays the app's sheet** in the first cut: it opens
  over the app area. As a host sheet it would stand outside the device's
  screen; asked of the owner (report).
- **Bare host with `--panel`**: the window is a device's screen, so no
  strip and no preview; the app draws its own pills as a device does.

## 7. What the app no longer draws

The "Demo only" card and its pane, column and foot strip (the page's
and the sheet's, and Make codex32 shares' own with its More and Less);
the top bar's stick pills and their Sticks menu sheet on Faraday
Desktop; Load samples beside the mode pill; Settings' Preview size; the
desktop shell's own framing of a preview. The dice sheet's column at
the right (`docs/LAYOUT.md` §16.1) stays: it is the entry's own, not the
demo's.

## 8. Tests

`faraday/faraday-core/tests/all/desktop_host.rs`: in Demo the strip
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
control inside it and Samples reached from the Demo menu; held bare, no
strip.

## 9. Speed

The app's own frames cost what they did (`docs/SPEED.md` §7, the late
flow at 1366 × 768 and 2560 × 1600: every screen within the run-to-run
spread). The host adds, per frame, the strip and a copy of the app's
pixels into the window's frame: about 0.1 to 0.2 ms at 1366 × 768 and
about 2 ms at 2560 × 1600 at scale 2, where the frame is 16 MB. At the
window's own size the window is not cleared first, the app's frame
covering all above the strip. Planned: the shell copying the app's
frame and the strip's into the window itself, so the app's pixels are
copied once, as before the host.
