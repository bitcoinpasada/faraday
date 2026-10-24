# Design system

**Kind:** Reference
**Status:** Built

Faraday and the Starter follow Material 3 (F92, F91) in their
components, type, spacing, touch targets, states, colour roles, shape,
elevation, motion and layout, with Faraday's own look; where this
document differs from Material 3, a decision says so.

Every dimension in it is a name in
`core/faraday-draw/src/tokens.rs`, and `tools/lint-tokens.sh` keeps it that
way.

## 1. Why this exists

Faraday's screens were built one at a time, so the same kind of content
could be drawn several ways. This document fixes, per kind of content,
how it is drawn, how it behaves and what draws it, and names every
screen and how it is laid out: the principles (§2), the two layouts and
the theme (§3), the content kinds (§4) and the screens (§5).

## 2. Principles

1. **Labels, not prose.** A working screen is titles, labels, values and
   actions. Longer text on one is a single line: the reason under a
   dimmed control ("Load a key first"), an error or warning line, a
   toast, a sheet's line naming what an act loses. Paragraphs are on
   Learn and About, which are not working screens, in Guided mode's
   walk-through at the top of an open step (§4.12), and behind the Spend
   tab's **More about this**, closed until pressed. Text longer than a
   label, wherever it appears, is plain statement of fact in an
   encyclopedia voice, never metaphor or flourish in place of direct
   statement.
2. **One kind of content, one rendering.** Each content kind in §4 is
   drawn the way its row says, by what the row names.
3. **One screen per kind of data.** Moved to `docs/history/DESIGN-LOG.md`.
4. **One item per row on touch screens.** Moved to `docs/history/DESIGN-LOG.md`.
5. **Fixed geometry for the keyboards.** An on-screen keyboard is the
   same size in the same place every time it is up: the docked keyboard
   at the foot of the panel or of the page beside the sidebar, word
   entry's keys under the word grid (§4.3).
6. **One placement law.** Moved to `docs/history/DESIGN-LOG.md`.
7. **Direct manipulation, few confirms.** A choice is made by pressing
   or ticking it. A single choice that decides a step moves the flow on
   by itself, and pressing the option already chosen moves on too; no
   button only repeats the choice just made (F103). Continue stays where
   a step holds more than one value (steppers, ticks, typing) or the
   press is the act (the words quiz checks a word and Continue answers);
   Back opens the step before with its choice kept (F104). Signing a
   transaction is a button, **Sign with** (§4.4). A hold (§4.13) confirms signing
   with a GPG or Secure Boot key from a vault, deleting from a
   vault, marking a key revoked, the stick a backup writes to, and wiping
   a stick; **Hold to show** reveals a held secret (§4.10). A question
   before an act is a sheet: Lock's, and a flow's Cancel where ending it
   loses something (§4.13).
8. **Secrets are hidden where they are read.** Words, seeds, private
   keys, passphrases and shares are masked in place: a held secret's box
   shows its value only while held, a **Show** button shows one until
   **Hide**, a passphrase field shows dots (§4.3, §4.10). Faraday has no
   screen for one secret or one string alone (F68).
9. **One unit.** Amounts are in BTC with eight decimals; there is no
   unit setting (§4.7). A fee is in sats with its rate.
10. **No pressure.** Two things count down: the idle warning before the
    lock, and a TOTP code's bar for the time left in its period (§4.14).
    Status shows state, not urgency.
11. **Common idioms, not invented ones.** Where a phone or a desktop has
    a well-understood pattern, the screen uses it: a keyboard types into
    the field with the caret; a list of options is rows; several choices
    are ticked boxes; a fact is a label and its value. Spare space is
    never filled with decoration or with a second rendering of the same
    value.

## 3. Sizes

Faraday lays every screen out one of two ways (`Faraday::is_compact`). `docs/LAYOUT.md` (F92, planned) replaces these two with Material 3's five window size classes, a navigation bar, rail or expanded rail by class, and list-and-detail panes.

| Layout | When | What changes |
|---|---|---|
| Small panel | A window under 1200 dp wide, a compact, medium or expanded one (the Pi's 2.8" panel is 268 dp); inside the Starter, its compact class (`docs/STARTER.md` §3.1d: under 600 dp, or narrower for a larger text size); its medium and expanded classes are the wide layout with the navigation rail in place of the sidebar (F91). | No sidebar: Home is a menu with a button for each of the sidebar's entries, and every other page has the bar across its top (§4.1). A step flow is a page per step. A sheet is the panel's width less a margin. The keyboard rises by itself under a field that takes typing. |
| Wide | Every other window. | The sidebar, 240 units, beside the page; the two together at most 1600 units wide, centred on a wider display. A sheet is centred over the page. The keyboard comes up only from its button. |

Every window is laid out at its screen's own density, never scaled to
fit: the desktop app at the compositor's scale, the stick at the
density its kernel command line gives, the Pi panel at its 286 dpi, each
times the person's Display scale (F101). The window's size class
(`Faraday::window()`, `docs/LAYOUT.md` §2) picks the layout: one column
under large (1200 dp), the wide layout from there, until LAYOUT's
navigation is built.

Sizes are design units (dp). The tokens are names in
`core/faraday-draw/src/tokens.rs`: the type ramp (title 20, body 16,
label 14, caption 12), the grid and gaps, the radii, the touch floor,
the field heights, the board's and the chart's dimensions, the scanner's
and the QR code's. The size classes there, `small`, `mobile` and `wide`,
size the keyboards and the QR side; Faraday's screens use `small` on the
small panel and `wide` otherwise, and `mobile` only scales the PNG a QR
code is saved as.

Theme: `faraday_ui::pal` names every colour a screen uses — `SURFACE`
(the page and a sheet, F39), `SIDEBAR`, `LINE`, `INNER`, `BORDER`,
`TEXT`, `MUTED`, `DIM`, `ACCENT`, `ON_ACCENT`, `OK`, `WARN`, `ERR`, and
the board's and the sticks' own — and each `Theme` draws them in its own
values: Dark, Light, Cobalt (the theme a first start is in), Nord,
Catppuccin, Tokyo Night, Gruvbox, Rosé Pine, Bitcoin Orange, Bitcoin
Orange Light and Lamplight, chosen in Settings. A QR code's black and
white and a camera's picture are drawn as they are in every theme.

Faces: Noto Sans and Noto Sans SemiBold for text, Noto Sans Mono for
data, Noto Sans CJK for the characters the wordlists and their keyboards
need and the text faces lack, Terminus for the firmware screens Faraday
sketches (`faraday_ui::sketch`), and a closed icon set (`Icon`, from the
SeedSigner icon font and Font Awesome 6 Free Solid) with Faraday's own
drawings beside it (`Glyph::Stick`, the shield `Ui::mark`, the wordmark
`Ui::wordmark`). Each glyph is stored once as an outline and filled at
the exact pixel size a display asks for.
Anything read character by character is set in the mono face:
fingerprints, hex, paths, addresses, extended keys, txids, a descriptor
row's value, and the words as typed and offered on word entry; prose,
labels, buttons and titles that are words stay in the text faces.

The small panel carries the same flows and the same steps as the wide
layout. It lays them out a page a step and moves the sidebar's entries
to Home.

## 4. Content inventory

Each kind: how it is drawn, how it behaves, and what draws it. A name in
`code` is a function or type in `faraday-ui` (`faraday_ui::…`, `Ui::…`,
the `pal` colours), in `faraday-draw` (`Icon`, `KeyboardKind`, the
tokens), or in `faraday-core`, named by its module where it is not in
`screens/`.

**Elevation.** What floats casts Material 3's shadow of its level
(`Ui::shadow`, `Canvas::elevation`; sizes `ELEVATION_KEY` and
`ELEVATION_AMBIENT`, levels 1 to 5): two layers in black, a key layer at
30% and an ambient layer at 15%, each with its y offset, blur and
spread. The same black in every theme but the high-contrast ones, which
draw none: what floats there keeps its outline, so no text or edge pair
falls below 7:1 and 3:1. Level 1: a modal sheet (with its scrim), a
board chip and a raised card (`ELEVATION_SHEET`). Level 2: a menu, a
popup and the wallet list (`ELEVATION_MENU`). Level 3: the floating
action button, a toast and a notice (`ELEVATION_FAB`). Level 4: the
floating action button under the pointer and a dragged chip
(`ELEVATION_RAISED`).

### 4.1 Navigation

| Kind | Rendering | Behaviour |
|---|---|---|
| Sidebar (wide) | 240 units at the left (`sidebar`): the wordmark; the tabs Home, Vaults, Files, Wallets, Tools, Learn, then the stick visit while a stick is attached and nothing is held, Transfer in the desktop app, and Settings, each with its icon, the current one lit; under its tab a row for each flow in progress with its open step at the right; the loaded keys under Keys and the wallets under Wallets, "+ N more" where they do not fit; the session strip, **Bring in · Open · Work · Write out**, the current stage in the accent; Files and the sticks attached; Lock and the power button. | A tab opens its screen; Learn opens the Learn sheet; a flow's row resumes it; a key opens Explore on it; a wallet opens its card; the session strip opens Files (`docs/FLOWS.md`, Sidebar). |
| Back link (wide) | A chevron and the name of where it lands, small and muted, above the page's title (`flow::Column`'s `back`, and each screen's own head). | It lands on the screen seen before this one, as it was seen, and is named after it (F77). In a flow with steps it opens the step before the open one, its choices as they were, and is named after that step; only the first step's lands on the screen before, and a running step takes none (F104, `StepFlow` in `faraday/faraday-core/src/trail.rs`). The mouse's back button does the same (`Ui::back`), and its forward button opens again the step or screen it left. |
| Bar (small panel) | `compact::BAR_H` (44) across the top of every page but Home and a step flow (`compact::bar`): the chevron, where it goes back to small above the page's name, and at the right end the tools (`compact::tools`): **Cancel** on a flow in progress, the keyboard button where the page types, the mode pill off mainnet, and the ? where the page has a Learn page. A step flow draws its own top and the same tools (`compact::flow_bar_tools`). | The chevron goes back as the back link does. |
| Top-right row (wide) | Beside the title: the mode pill off mainnet (§4.8), the ? where the page has a Learn page, and the keyboard button where typing goes somewhere (`compact::wide_keyboard_button`). | The ? opens the Learn sheet at the page's section; the keyboard button brings the docked keyboard up and puts it away. |
| Title | The page's name, 28 units SemiBold (`faraday_ui::parts::title`); a step flow's heading is 26, with a chip beside it where the flow names what it works on, and **Steps only · Guided** and **Cancel** at the row's right (`flow::column`). | The step count sits beside a step's name as muted text (§4.14), never in the title. |
| Home | On the wide layout: the lead tile while something is under way or waiting, then half-width tiles two to a row (`docs/FLOWS.md`, Home). On the small panel: the menu, a button for each of the sidebar's entries. **Scan** floats at the bottom-right on both (`Ui::fab`). | A tile or a button opens its screen; Scan opens the scanner. |
| ⋮ menu | Three dots at the right end of a file row or a board chip, opening a popup under them on the wide layout and a sheet on the small panel (`faraday/faraday-core/src/menu.rs`; the board's, §4.17). | Arrows move, Enter chooses, Escape closes; a press outside closes it. |
| Pager | **Previous · Next** at the foot of Backups on the small panel, one wallet a page. | For runs of same-kind pages only. |
| Navigation rail (Starter, medium and expanded) | 80 units at the window's left in `SIDEBAR`: the destinations Home, Make, Tools and Learn from the top, each its icon over its caption label, the current one on an accent pill behind the icon (`docs/STARTER.md` §3.1d, F91). Not drawn on a page inside a task. | A press opens the destination as it was last seen; a press on the current one goes to its top. |
| Navigation bar (Starter, compact) | 80 units across the window's foot: the same four destinations, evenly spaced, drawn as the rail draws them. Not drawn on a page inside a task. | As the rail. |
| Top app bar (Starter) | 64 units across the page: the wordmark on Home, a destination's name on the others, a back arrow and the page's name on a page inside a task; at the right end Install or Update while it applies (secondary) and the ⋮ menu (About, Uninstall while installed, Remove the boot entry while offered). It stays while the page scrolls under it. | The back arrow lands on the destination it was opened from, as it was seen (F77). |

### 4.2 Choice

| Kind | Rendering | Behaviour |
|---|---|---|
| Option row | One rounded box 48 units tall per option: its name at the left and its line at the right, the line under the name on the small panel; the chosen one tinted and edged in the accent (`kind_rows`). Two everyday options first, the rest behind **More kinds**, a ghost button. | A press chooses it; on a card that asks only this, it moves on to the next card, and a press on the one chosen does too (F103). Used for: Create's and Restore's Kind, Create a vault's size. |
| Option buttons | A row of buttons that wraps at the width (`faraday_ui::parts::button_rows`, `wrap_buttons`), the chosen one `Style::Primary` and the others secondary or ghost. | A press chooses it, and on a step card that asks only this, moves on as an option row does (F103). Used for: Add a key's list and form, Explore's script, the QR sheet's format, speed and size, Make codex32 shares' Scheme and Entropy, its entry's strip of worksheets and its String · Codex32QR, Sign a message's format, BIP-85's application. |
| Tab row | Option buttons `CODEX32_TAB` high, one per item in its order, the shown one primary and a ✓ before each once shown, wrapping at the width; on a compact window the shown item's name over **Previous** and **Next** (`docs/CODEX32.md` §6.5). | A press shows that item. Used for: Make codex32 shares' Worksheets. |
| Multi choice | A list of full-width rows (`Ui::multi_list`), one option per row, a checkbox at its start; any number ticked; Continue under it. The question is the card's or the page's title. A row that may not be ticked is dimmed and inert. Several lists may share a page, each under its own label. | Tap ticks or unticks the row. Nothing has to be ticked. A row may carry a value under its label, muted (`Ui::multi_list_valued`; the Load samples sheet's type of each sample). Used for: the backup plan's Software card (the software and the form), on Back up and in the wallet flow, and the Load samples sheet. |
| Checkbox | A tick box (`Ui::checkbox`); one ticked and not enabled shows its tick dimmed. | A press ticks or unticks it; a dimmed tick is done and a press does not undo it. |
| Stepper | A label and a count with − and + beside it, at the row's right end on the small panel (`stepper`). | Each press moves the count by one within its bounds. |
| Slider | A track with a stop for each whole number and a round thumb carrying the value, the two ends' numbers under it (`CoreUi::slider`). | A press on a stop sets it; a drag presses each stop it crosses; the arrow keys step the slider last pressed. |

### 4.3 Entry

| Kind | Rendering | Behaviour |
|---|---|---|
| Text field | A rounded box with what is typed in it and the caret where typing goes (`Ui::typed`, `Ui::caret`); what is selected is drawn behind the text (`Ui::selection`). A passphrase field shows dots, and **No passphrase** while it is empty (`pass_field`, `CoreUi::no_passphrase`); a field that may hold a secret has an eye at its right end (`secret_box`). | A press places the caret and a drag selects. The eye shows the text in the clear until it is pressed again. A field that takes no typing while a stick is attached says so in place of its content. |
| Keyboard | The on-screen keyboard docked at the foot of the panel or of the page beside the sidebar (`faraday_ui::keyboard::keyboard`): the path keyboard on Explore, the address keyboard on Explore's Find, the passphrase keyboard everywhere else (`KeyboardKind`). | On the small panel it rises by itself under a field that takes typing; on the wide layout only from the keyboard button. Its keys type into the field typing goes to, as a physical keyboard does (§4.15). |
| Word keyboard | Word entry's own keys, in the page under the word grid (`word_keys`): the BIP-39 letters for a Latin list, and for the others the kana, jamo, pinyin or 注音 keyboard (`KeyboardKind::Kana`, `Jamo`, `Pinyin`, `Zhuyin`). A key that leads to no word on the list is dimmed. | Types the list being entered; the word is taken from the candidates. |
| Kana keyboard | The 五十音 as a grid: ten columns of the consonant groups あかさたなはまやらわ, five rows of the vowels あいうえお, every key one unit wide. The cells the grid leaves empty carry ゛, ゜ and 小; the backspace takes the bottom-right cell, ん the one above it. | Types the Japanese list. ゛and ゜ add the voicing marks the list is written with; 小 turns the last kana into its small form and back. か and ゛ read as が. |
| Jamo keyboard | The two-set layout on the QWERTY positions: ten, nine, and seven keys with shift at the left and the backspace at the right of the last row. | Types the Korean list. Shift gives the five doubled consonants and the two shifted vowels and releases after one key. The keys run through the two-set automaton, so ㄱㅏ reads as 가 and ㄱㅏㄱ as 각. |
| Pinyin keyboard | The BIP-39 letter rows, and a fourth row of the five tone keys 1 2 3 4 5 with the backspace ending it. | Types the Simplified Chinese list, looked up by a Mandarin reading: the syllable on the letters (`v` for `ü`), then its tone. A letter is live while some candidate's syllable continues with it; a tone key is live in the tones that syllable has. The candidates fill only once the tone is typed. |
| 注音 keyboard | The 大千 layout, four rows on the QWERTY positions, the tone marks ˊ ˇ ˋ ˙ in the first row and ˉ ending the second, the backspace ending the last row. | Types the Traditional Chinese list. Every tone is a mark key, ˉ for the first; the mark completes the reading and the candidates fill only then. |
| Word grid | One numbered cell per word of the count, the count's buttons (12 · 15 · 18 · 21 · 24) above it, the word being typed in its cell (`typed_cells`, `keyed_cells`); a status line under it states the words and the checksum. | Each word is checked against the list as it is typed; Tab finishes a word on a Latin list once only one fits. |
| Candidates | The words the keys typed can still become, a pill each in the mono face, wrapping at the width (`keyed_pills`, `Ui::word_pill`). | A press on a pill takes the word. |
| Dice, coin, card keys | A row of buttons in New key's randomness step: 1 to 6 for a die, Heads and Tails for a coin, and the ranks and suits for cards, with the count so far, "{} of {}", and the words a run has made (keygen). A face a procedure does not take next is `Style::Disabled`. | Each press is one entry. |
| Dice roller | Faraday Desktop only (F96, `docs/CODEX32.md` §5.6): beside the dice pad's entries, "Rolled by this computer" over one die (`ROLLER_DIE`, pips drawn, outlined in the accent while it tumbles), **Roll** (**Stop** while a click rolls) beside it, then **Rolls per click**: 1 · 5 · 10 · To the end chips, the chosen one `Style::Primary`, and a number field. Drawn by `fresh/roller/screen.rs`, its sizes and timings `ROLLER_*` tokens. The codex32 dice step's roller (`docs/CODEX32.md` §5.5, `fresh/roller/five_screen.rs`) is the same with five dice (`ROLLER_FIVE_DIE`) labelled Die 1 to Die 5 and **One click rolls**: One pass · One character · One share · To the end. | **Roll** lands the click's values on the entries as presses; the tumble and the pause between values are skipped under Reduce motion (MOTION). Values are marked as this computer's: the result's entropy line says "simulated dice". |
| Code box | A value whose exact characters must be plain, in the mono face on a tinted, rounded box padded by tokens, with no quotes (`Ui::code_box`; `Ui::code_line` puts one after a line of text, below it where it does not fit). Used for: the Demo passphrase. | Not a field: it takes no press. |
| Error line | One line under the field or the grid in `ERR`: the codec's or the list's reason ("No word starts with that"). | Replaces the status line while it stands. |
| Use a sample | In Demo, with samples set (F81, `docs/MODES.md` §6.1), a secondary chip inside the empty field where a key is typed (`Ui::field_action`), at the field's text inset and vertically centred, labelled "Use a sample", or "Sample" where the field is narrower than the chip. On Add a key's word grid the grid keeps all its numbered cells and the chip stands inside cell 1 only, where the word would be; on a small panel's word entry it stands where the word being typed is. Its hit target is the chip, as tall as the touch floor on the small panel and as the field on a wide one. The field's caret and placeholder are not drawn while it shows. | Puts the next sample not loaded in the field as typed; a part of a set fills the next part of the set begun. Gone for the visit once the person presses in the field or types; filling the field also hides it, and Clear or the next part brings it back. Never drawn outside Demo. |

A seed is typed on Add a key (`Screen::Entry`): **Scan a SeedQR** above
the words, then the word grid, the candidates and the word keyboard,
then the passphrase field and **Add key**. A line in `WARN` under it says
never to type a seed that holds real bitcoin (`never_real_seed`).

### 4.4 Identity and keys

| Kind | Rendering | Behaviour |
|---|---|---|
| Fingerprint | Eight hex characters in mono, never split (`fp_text`). | The key's name everywhere: the sidebar's Keys, a wallet's keys, "Sign with 73c5da0a", a board chip. |
| Key in the sidebar | The key's fingerprint and what it is ("passphrase" where it has one) under Keys (`sidebar`). | A press opens Explore on that key. A key has no page of its own: BIP-85, a vanity address and a backup are Tools and wallet flows (F68). |
| Sign with | A primary button in the Spend tab's signing step, "Sign with" and the fingerprints of the loaded keys that sign this transaction, beside the count of inputs (`step_body`); once signed, "Signed with" and the fingerprints in `OK`. | A press signs every input those keys can sign. Where no loaded key signs, the step says so and has no button. |

### 4.5 Long strings

| Kind | Rendering | Behaviour |
|---|---|---|
| Short form | The first eight characters in two groups of four, an ellipsis and the last four, "bc1q 8f3x … k2pz" (`faraday_ui::short`); a string of sixteen characters or fewer is shown whole, in groups of four. | Inside a line that names a string rather than shows it to be read: the Spend review's outputs ("to bc1q 8f3x … k2pz"), its txid and its inputs, the first address on Restore's card and on the Silent payments screen. It opens nothing. |
| Whole string | Whole, mono, in groups of four (`faraday_ui::grouped`), wrapped where it does not fit, never cut. | Where the string is what a person reads or checks: a wallet card's first receive address, a vanity find, a decoded output, a signed message's address, an image's SHA-256. The Key explorer's addresses are whole and wrapped too (F76). |
| Descriptor | A summary row, not the descriptor's text: "Descriptor" above; below, mono and wrapped, the wallet's shape ("2 of 3 · native SegWit multisig"), its keys' fingerprints, and " · #" and the checksum where the wallet did not come from a record; chevron (`descriptor_row`). | On Create, Restore, the key-making flow and a wallet's sheet on the chart. The row opens the wallet's QR sheet, which carries the descriptor's whole text under the code. |

Faraday has no screen that shows one string alone (F68): a string is
whole where it is read.

### 4.6 Derivation paths and script types

| Kind | Rendering | Behaviour |
|---|---|---|
| Path | A mono string, `m/84h/0h/0h/0/0`. As a row: "Path" above, the paths below, wrapped, with **Change** at the right while the wallet can be taken back (`path_row`, Create's Check). | **Change** goes back to the Account row (F98). |
| Account | Create's account, as the Path row is drawn: "Account" above, the number below, with **Change** at the right (`account_row`), at the foot of Quorum or the head of Keys (F98). | **Change** puts the Stepper (§4.2) in its place. |
| Path field | Explore's path: a field typed on the path keyboard (`KeyboardKind::Path`), with **Index -1** and **Index +1** beside it. | The values under it follow the path as it is typed. |
| Script type | Named in one vocabulary: Native SegWit, Nested SegWit, Taproot, Legacy; on Explore one button each (`explore::screen::script_name`), in a wallet's shape after its kind ("Single key · native SegWit"). | Never "BIP-84" or "p2wpkh" on a working screen. |

### 4.7 Amounts

| Kind | Rendering | Behaviour |
|---|---|---|
| Amount | In BTC with eight decimals, "0.00060000 BTC" (`faraday_ui::btc`). | One unit; there is no unit setting. |
| Fee | One value, the fee and its rate: "{fee} sat · {rate} sat/vB". | The Spend review's and Decode's. |

### 4.8 Badges

| Kind | Rendering |
|---|---|
| Network | A caution pill naming the mode off mainnet, `MODE_PILL_HEIGHT` tall (`modes::screen::pill`): on the wide layout's top-right row and in the small panel's bar. Pressing it opens the mode sheet; on mainnet the sheet is Settings' Mode row (F81, `docs/MODES.md`). |
| Load samples | In Demo only, when the app has samples (Faraday Desktop and the website), immediately left of the Demo pill, a gap between, the same height and shape, in the accent tone: it is an action, the mode pill is a state. "Load samples", and "Samples" on the compact bar and Home. Pressing it opens the Load samples sheet: the line "In Demo, the sample passphrase is" with `faraday` in a code box, then a tick box row for each sample wallet and key with its type under its name (a wallet's kind and script, a key's form and size). |
| Chip | A rounded label with a dot, in a tone (`Ui::chip`): beside a step flow's heading, where the flow goes next or what it works on (`flow::Column`'s `chip`). |
| Pill | A rounded label with an edge (`Ui::pill`, `Ui::pill_in`): a stick's pill in its own colour (`faraday_ui::stick_tone`). The HOTP and Unreadable badges, in place of a TOTP code on Codes and on a vault entry, are the same shape. |
| Step badge | A step's number in a circle, ticked once done (`Ui::badge`; a guide step's, `faraday_ui::parts::step_circle`). |
| Pips | A threshold as `n` dots, `filled` of them filled (`Ui::pips`). |

### 4.9 Codes

| Kind | Rendering | Behaviour |
|---|---|---|
| QR sheet | A sheet (`qr_sheet`): the title, an animated code's frame "{} · frame {} of {}", whether scanning it is safe — "Secret · whoever scans this can use it" in `ERR` or "Public · safe to scan" in `OK` — the code on a white card with its quiet zone (`faraday_ui::parts::qr_code`), a descriptor's whole text under it; then the format (UR · BBQr · Parts, Parts for text only), an animated code's speed (2/s · 3/s · 5/s) and how much each frame holds (Small · Medium · Large), **PNG** for one code of public content, and **Done**. On the small panel the code is as large as the panel allows and the format line gives way to it. | The format and size choices take effect at once. PNG saves the code with its label. |
| Scanner | A sheet (`scan_sheet`): the title names what is scanned ("Scan a SeedQR", "Scan a key for {wallet}", "Scan a QR code"); the camera's picture, its corner brackets white while looking and green while a QR that read is in view, the state line and the bar of parts read on a translucent band at its foot (`viewfinder_over`); under it the ways in: **Type the words instead** where the scanner takes a seed, **Paste** (dimmed where the shell has no clipboard), **Sample** in Demo, and **Cancel**, or **Done** on the Spend tab's intake; a camera chooser where there is more than one. | A code read routes by what it is (`docs/QR.md`). A scanner that takes a seed carries the never-type-a-real-seed line in `WARN`. Ctrl+V is Paste. A payload that is a secret is refused on the way in from the clipboard, except a seed pasted into a scanner that loads one, on the desktop (F75). With no camera the picture says so and the ways in stay live. |

### 4.10 Secrets

| Kind | Rendering | Behaviour |
|---|---|---|
| Held secret | A box outlined in `WARN`, bullets in place of the value and "Hold to show" under it (`secret_with`, `seedqr_box` in vault); the value, or the SeedQR, while it is held, with "Release to hide". | Shown only while a finger or the button is held; the value is made only then. |
| Show and Hide | A **Show** button beside a masked value that becomes **Hide** (BIP-85's value, a vanity counter, a Lightning node key). | Shows the value until Hide. |
| Secret out | A sheet before a secret leaves the session (`secret_out_sheet`): "This is a secret" or "This may be a secret", what it is, who can use it, and the ways out — sealed in a vault, or unprotected on the stick, ticked to write. | Cancel keeps it in memory. |
| Worksheet | A codex32 worksheet as the booklet draws it (`docs/CODEX32.md` §7.5, `faraday/faraday-core/src/codex32/screen.rs`): boxes `codex32_box` on a side by the window's class, numbered above in groups of four, the rows' names in a `CODEX32_LABEL_W` column, the staircase as the ladder works down; given boxes (the data, the header) outlined heavier in `TEXT`, the checksum squares in `CODEX32_CHECKSUM_FILL` and bolded, rows read from a table in `CODEX32_LOOKUP_FILL`, the final row under a rule; the box being filled outlined in the accent. The dice de-biasing sheet beside it: five tracks of pads `CODEX32_PAD`, the first value outlined, the second filled, an equal pair struck in `WARN`, and the tree of 32 leaves with the path from die 1 in the accent. | A press on a cell opens its working on a sheet (`CHART_SHEET_WIDTH` wide; the panel's width on a compact window) with **Done**; nothing is computed to open it. |
| Mask policy | Words, seeds, private keys, passphrases and shares are secrets. | Fingerprints, checksums and addresses are not. A secret is never copied to a clipboard. In Demo (F81, `docs/MODES.md` §6.2) a secret a sample filled is drawn unmasked, and a passphrase or password field holds `faraday` in a code box, unmasked, with no eye, once its chip is pressed; an empty one draws the chip "faraday" inside it until the person presses in the field or types, and where it is optional **No passphrase** shows beside it. |

### 4.11 Records and results

| Kind | Rendering | Behaviour |
|---|---|---|
| Fact | A label over its value, the value wrapped (`faraday_ui::sheet::kv`), or a label and value on one row in a card's table. | One fact per row. |
| Card | A surface set apart by its edge alone, `BORDER` (`Ui::card`, F39): a wallet's card, a step's summary, Home's tiles. | — |
| Result | A line with its icon in its tone: done in `OK` ("Signed with 73c5da0a"), refused in `ERR`, waiting in `WARN`. | — |
| Warning | A line with the warning glyph in `WARN` (a backup's checks, a spend's cautions); "No warnings" where there are none. | — |
| Dimmed | A control not available now, drawn `Style::Disabled`; where a step the person can take would make it available, a short line under it says which (Tools' rows: "Load a key first", "Make a vault first"). | A press does nothing. |
| Toast | One line in a rounded box centred at the foot of the page, rising in and fading after three seconds (`notices`): "{what} waits for the stick". The same place holds "Remove the stick to keep working" in `WARN` while a stick stands in the way. | Says what a press did where the screen does not change to show it. |

### 4.12 Text

| Kind | Rendering | Where |
|---|---|---|
| Title | §4.1. | Every page and sheet. |
| Section label | 14 units SemiBold, muted (`faraday_ui::parts::section_label`). | Over a group of rows. |
| Label | Caption or label size, muted, above or left of its value. | Every fact. |
| Value | Body or mono. | Everything a person reads or compares. |
| Walk-through | A short paragraph at the top of a step while it is open, in Guided mode only (`flow::Card`'s `guide`, `guide_text`); Steps only hides it. | Step flows (`docs/WALLETS.md` §6). |
| Explainer | Paragraphs on a Learn page, in the Learn sheet (`learn_sheet`). | Never on a working screen outside Guided mode's walk-through. |

The ? on a page with a Learn page opens the Learn sheet at the section
that page is about.

### 4.13 Actions

| Kind | Rendering | Behaviour |
|---|---|---|
| Button | `Ui::button` in a `Style`: `Primary`, the one action a step leads with, in the accent; `Secondary`, edged; `Ghost`, quiet beside others; `Disabled`, shown but not available now. A row of them shares the width where every label fits its share and stands one under another, full width, where not (`faraday_ui::sheet::buttons`). | A press acts. |
| Hold | An outlined button that fills while held (`hold_button`): "Hold to sign" with a GPG or Secure Boot signing key from a vault, "Hold to delete from vault", "Hold to mark the key revoked". | Acts once held to the end; released before, nothing happens. |
| Floating action | A pill in the accent over the page, its icon and its word, at the bottom-right (`Ui::fab`). | Home's **Scan**. |
| Cancel a flow | **Cancel**, a quiet outlined button at the right end of a flow's title row, beside **Steps only · Guided** (`flow::column`); on the small panel in the bar's tools (`compact::tools`). On every flow that can have a sidebar row but a backup flow whose key is made, which ends only by finishing (`docs/FLOWS.md`, Flows in progress, Cancel; F51). | Ends the flow and shows the screen it was opened from. When ending it loses something the person would notice, a sheet first: its question as the title ("Cancel making this wallet?"), one line naming what is lost, **Keep going** (secondary) and **Cancel the flow** (primary); a press beside the sheet keeps going. With nothing done past the first card, no sheet. |

### 4.14 Progress and state

| Kind | Rendering |
|---|---|
| Progress | A count "2 of 5": the Spend tab's signatures, New key's entries, a scan's parts with its bar (`BAR_HEIGHT`). A TOTP code's row has a bar under it for the time left in its period, the seconds at its end. |
| Default | A step card (in Faraday a flat section of its flow: no fill and no box, divided from the next by a rule in `LINE`, its number, title and value on its row, `docs/DECISIONS.md` F39) whose value has a default is closed at entry, showing its value and a **Change** text button at the card's right in place of the chevron that opens it. The flow opens on the first card with no default; opening a closed card does not reset the cards after it. A card that asks one thing closes on the choice and opens the next (F103); Back opens the card before the open one, its choices as they were, Forward the card Back left, and the first card's Back leaves the flow (F104). On a small panel the same: the paged flow starts on the first open card, and that page carries at its head one row per earlier card closed on its default, "{title} · {value}" with **Change**, which opens it. Tapping the page's title (the card's name alone, "Kind", with "Step 1 of 8" beside it in the muted caption type) closes the card and shows the list of steps, from which any card opens; no step indicator is drawn. Create a wallet and New key are the exception: each opens on its first card — Kind, Length — whatever opened it, even with a default already ticked; a card still closed on its default shows its value and **Change** the same way once Continue has passed it (`docs/history/NEW-WALLET.md` §2.1). |
| Empty state | A line and the buttons that start the flow: Wallets with no wallet loaded, "No wallet is loaded" and its ways in (Make a new wallet, Restore an existing wallet, Spend from a backup, Add a key); TOTP codes with no entry, "No entries with a TOTP secret" and Scan a setup code. |
| Locked | The sheet after the idle lock (`locked_sheet`): "Locked", that every key and vault was wiped, what waits for the stick, and Continue. The idle warning before it counts down (`idle_warn_sheet`). |
| Self-test failed | The whole window: "Self-test failed", the check that failed, and Exit (`selftest_failed`). |

### 4.15 Keys

A shell with a keyboard — a laptop booted from the stick, the desktop
shell — takes keys as a desktop app does. Nothing on a screen says so.

| Kind | Behaviour |
|---|---|
| Typing | Characters, Backspace, Delete and Enter go to the field typing goes to, and to the on-screen keyboard's field where it is up; Left, Right, Home and End move the caret (`edit`). |
| Enter | Takes what is typed where the field has one action: a word on word entry, a share or a part, a name, a passphrase. |
| Tab | Finishes a word on a Latin list once only one fits; moves between a passphrase and its repeat. On the backup board, Tab and Shift+Tab move among the chips' ⋮s, ringed in the accent at `FOCUS_STROKE` (§4.17). |
| Escape | Closes the open sheet or ⋮ menu (a sheet's Cancel), leaves a field, and goes back from Add a key and Set the time. |
| Up, Down | In a ⋮ menu, move between its rows; elsewhere scroll the region under the pointer, or the page's, by one wheel notch. On a slider the arrows step it (§4.2). |
| Ctrl+V | The scanner's Paste, and nothing anywhere else. |

### 4.16 Structure chart

How a thing is made of others and where each of those is: Faraday's
wallet at a glance (`docs/history/NEW-WALLET.md` §6), the wallet over its keys
over its backup; and the same chart from an open vault (§7.2), the
backup over the keys over the wallet.

| Kind | Rendering | Behaviour |
|---|---|---|
| Node | The summary row's box: `RADIUS`, the page's fill, an inner edge, `CHART_NODE_PAD` inside. A title in the label type, an optional tag at its right in the caption type (secret, sealed, public, in their tones), then caption lines that wrap; a line marked lost or exposed reads "Key 1 words · lost" in `ERR`, struck through by a rule of its colour; a node that offers a step carries a `CHART_BUTTON` button at its foot. | The whole node is the target: it opens the node's sheet. A caption line that stands for one thing (a thing a backup node holds, a key's way in) is a target of its own over it: its sheet, or its way in. |
| Sheet | What a node or a line opens (`docs/history/NEW-WALLET.md` §9): a sheet `CHART_SHEET_WIDTH` wide, or the whole small panel as a page. Its head (icon and title), its facts as label-over-value rows, lines of what follows in the label type in their tones, a field being typed (its label in caption over a `CHART_SHEET_ROW` box with the caret; a passphrase as dots), then its actions as menu rows under optional caption headings, and its own primary button (Confirm, Keep, Check) beside Close (Back on a small panel) at its foot. A change asked for shows the plan's three check lines as facts, "before → after", a line made worse in `WARN`. | Each row leads into the flow that does it; opening the sheet does nothing else. A change is made only by its sheet's Confirm (`docs/history/NEW-WALLET.md` §9.7). |
| Row | Nodes side by side at equal widths, between `CHART_NODE_MIN` and `CHART_NODE_MAX`, `CHART_NODE_GAP` apart, centred. A row wider than its column wraps onto further lines at the widths a full line gives; the nodes of one line take its tallest node's height. | — |
| Line | A `CHART_STROKE` run (`CHART_STROKE_BOLD` for a key whose sheet's Where it is shows its lines, every other line then the inner-edge colour) from the middle of a node's bottom edge to its target's top edge, orthogonal: down into the band under the line of nodes, across, down. Where a line of nodes stands between, it runs down the gap between two nodes, or beside them, so it never crosses a node. Each band is `CHART_BAND` above and below its runs, each source's run across `CHART_LANE` from the next; a target's lines come in at points spread along its top. Each source has its colour (`KEY_LINES`, the theme's own colours in turn) and its dash pattern (`CHART_DASHES`, solid first), so the lines are told apart without colour too; the parent's lines are the border colour, solid. | Lines are not targets. |
| Column | On a small panel the rows stack, one node a line, full width, under the rows' headings, and no lines are drawn: a node names what the lines would have joined it to. | — |
| Direction | The rows read top down either way: the parent first (the wallet, its keys, its backup, as the wallet's card draws it), or the parts first (the backup, the keys, the wallet at the foot, as an open vault draws it). Lines always leave an upper node's bottom edge and enter a lower node's top edge. The plan's three checks go above the chart read either way, as a strip: three boxes side by side where each is at least `CHART_NODE_MIN` wide, else one under another, each its line in caption and its value at `TITLE` size in its colour (`OK`, the accent, `WARN`, `ERR`); the backup's map panel and the end of the backup draw the same strip (`docs/history/NEW-WALLET.md` §14.6). The column reverses its rows and headings the same way, the strip staying on top. | — |

### 4.17 Board

Where things are put, by the person: Faraday's backup board
(`docs/BACKUP-FLOW.md` §17), a row for each place and beside them a list
of what can be kept. A thing is a chip; a place is divided into boxes,
and a chip is dropped on a box. The board is the body of a step, on Back
up's plan and on the wallet flow's plan card alike; on Back up it takes
the step's whole width. The narrow column is §19.12 item 3's.

A new plan opens on the board with **The standard way** placed, the
one way a backup starts (`docs/BACKUP-FLOW.md` §17.6, §17.21): each
seed's words on paper with the whole description, a place for each
key, no SeedQR. There is no Start from. The board is read-only until
**Customize backup**; where the list will be (the right-hand column,
or the narrow column's **Add**) stand two secondary buttons, **Add
watch-only software** (**Take off watch-only software** while the
software place is there) and **Customize backup**. A vault box whose
vault is not made yet reads "Vault n · sealed · made in the
checklist", or "Vault n · new" where that does not fit its band.

Three boxes nest, so paper, a stick and a vault are told apart at a
glance (`docs/BACKUP-FLOW.md` §17.14, §17.15): the **place**, the **stick
box** inside it, the **vault box** inside the stick box; Paper stands in
the place beside the sticks. One hue per kind of box, pale fills and
strong edges, the colour carried by edges and header bands and never by
colour alone: each box has its glyph and its label.

| Box | Fill | Edge | Header | Glyph |
|---|---|---|---|---|
| Place | `PLACE_FILL`, a pale green | `PLACE_EDGE`, `BOARD_BOX_EDGE`, and a left stripe `BOARD_PLACE_STRIPE` wide | a `PLACE_BAND` band `BOARD_HEAD` high: the glyph, the name and the × in `PLACE_TEXT`, the balance and how near it brings a spend as outlined tags (`BACKUP-FLOW.md` §17.22) | house |
| Stick | `STICK_FILL`, a cool slate grey | `STICK_EDGE`, `BOARD_STICK_EDGE` | "Stick n" and its × in `STICK_TEXT` | USB stick (SD card on the Pi) |
| Files | `FILES_FILL`, a pale blue | `FILES_EDGE`, `BOARD_BOX_EDGE` | "Files · not sealed" in `FILES_TEXT` | file |
| **+ stick** | `STICK_FILL` | `STICK_EDGE`, dashed (`BOARD_DASH`); solid while a chip it takes is held | "+ stick" in `STICK_TEXT` | |
| Vault | `VAULT_FILL`, a pale accent tint | `VAULT_EDGE`, `BOARD_VAULT_STROKE` | a solid `VAULT_EDGE` band, "Vault n · sealed", its marks and × in `VAULT_ON` | closed lock (`Icon::Lock`) |
| Paper, Scanned in | `PAPER_FILL`, a warm cream | `PAPER_EDGE`, a brown, `BOARD_BOX_EDGE` | "Paper" in `PAPER_TEXT` | file |
| Chip | `SURFACE` (`INNER` while pressed), the shadow of elevation 1 | `CHIP_EDGE`, a neutral a step stronger than `BORDER`, 1 dp | | |
| **+ place** | none | `MUTED`, dashed | "+ place" in `MUTED` | |

Each theme has its own values (`faraday-ui` `pal`, the themes' tables),
by the same rules: a pale, low-chroma fill; an edge at least 3:1 against
the fill round it (a place against the page, a stick and Paper against
the place, a vault and the files box against the stick); all text at least 4.5:1 on its
fill, the vault band's on the band. The light theme's are the approved
mockup's. A chip's edge is not held to 3:1: it stands on its own surface
fill, edge and shadow, and its text is `TEXT` on `SURFACE`.

| Theme | `PLACE_FILL` | `PLACE_BAND` | `PLACE_EDGE` | `PLACE_TEXT` | `STICK_FILL` | `STICK_EDGE` | `STICK_TEXT` | `VAULT_FILL` | `VAULT_EDGE` | `VAULT_ON` | `PAPER_FILL` | `PAPER_EDGE` | `PAPER_TEXT` | `CHIP_EDGE` | `FILES_FILL` | `FILES_EDGE` | `FILES_TEXT` |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Dark | #1E2E30 | #243F38 | #5E9A73 | #86B396 | #2B343E | #7D8B98 | #96A1AC | #2A3E4A | #83D8EE | #0D121A | #343735 | #B89850 | #C8A860 | #697179 | #2E4055 | #538CCD | #8DB0D9 |
| Light | #EDF6EF | #DCEEE1 | #5E9A73 | #1E5B35 | #E3E8ED | #7D8B98 | #36424D | #D3EAF3 | #0B7A9E | #FFFFFF | #FBF5E6 | #977A36 | #6A501A | #AEB9C3 | #E3ECF7 | #4F82BC | #3168A8 |
| Cobalt | #EDF6F0 | #DBEEE1 | #5A946E | #436D52 | #E5E8EA | #74818D | #5B656F | #D1E3EE | #0062A3 | #FFFFFF | #FAF4E6 | #977A36 | #80682E | #AFB2B4 | #E3ECF7 | #5083BD | #3168A8 |
| Nord | #3D4C54 | #3F5857 | #73A785 | #BAD4C3 | #485060 | #9AA5AF | #C2C8CF | #475666 | #88C0D0 | #2E3440 | #515358 | #C1A465 | #DECBA0 | #828793 | #445F7E | #8EB4DE | #CDDDEE |
| Catppuccin | #343E48 | #374C4C | #5E9A73 | #A2C4AE | #404354 | #8995A1 | #AEB7BF | #4A4561 | #CBA6F7 | #1E1E2E | #49464C | #B89850 | #D2B87D | #6F748A | #3D5571 | #7BA6D9 | #B6CDE7 |
| Tokyo Night | #283540 | #2D4445 | #5E9A73 | #92BAA0 | #353A4C | #7D8B98 | #A1ABB5 | #323C59 | #7AA2F7 | #1A1B26 | #3D3D44 | #B89850 | #C9AA63 | #626985 | #364C64 | #689AD3 | #A4C1E1 |
| Gruvbox | #353C35 | #384A3C | #5E9A73 | #9DC1AA | #414142 | #85929E | #AAB4BC | #52472F | #FABD2F | #282828 | #4A443A | #B89850 | #D0B476 | #7C7463 | #374D66 | #6A9BD4 | #A6C2E1 |
| Rosé Pine | #EDF2E5 | #DBEAD8 | #59926D | #426C50 | #E5E4E1 | #727E8A | #59636C | #D8E0DF | #286983 | #FFFAF3 | #FAF0DC | #977A36 | #7F662D | #B2AEB3 | #E3ECF7 | #4C80BB | #3168A8 |
| Bitcoin Orange | #2B2B1F | #2F3C29 | #5E9A73 | #83B193 | #37302D | #7D8B98 | #94A0AB | #483017 | #F7931A | #1C1410 | #403325 | #B89850 | #C8A860 | #786F67 | #2C3D52 | #4E88CB | #89AED8 |
| Bitcoin Orange Light | #EDF4EA | #DBEBDC | #59926D | #426C50 | #E5E5E5 | #73808C | #59636C | #FEE9D0 | #B46B13 | #000000 | #FAF2E0 | #977A36 | #7F662D | #BBB5AE | #E3ECF7 | #4D80BB | #3168A8 |
| Lamplight | #EDF2E4 | #DBEAD7 | #58916C | #416A4F | #E5E4E0 | #727E8A | #59636C | #EEDBCD | #A34E26 | #FFFAF2 | #FAF0DB | #977A36 | #7F662D | #B6AFA6 | #E3ECF7 | #4D80BB | #3168A8 |
| High contrast dark | #0A1A10 | #0F2B1A | #4CD27A | #9AF0B5 | #141414 | #B0B0B0 | #D0D0D0 | #2A2A00 | #FFFF00 | #000000 | #1E1600 | #FFC24D | #FFD98A | #B0B0B0 | #001A33 | #5CA8FF | #A8D0FF |
| High contrast light | #EAF7EE | #D2EFDA | #1B6B3A | #0F4D28 | #ECECEC | #4D4D4D | #262626 | #DCDCFF | #0000CC | #FFFFFF | #FFF3D6 | #7A5200 | #5C3D00 | #595959 | #DCEBFF | #0B4F9E | #0A3D7A |
| Mars | #2B2418 | #2F3A27 | #5E9A73 | #83B193 | #332B27 | #7D8B98 | #94A0AB | #4A2418 | #D9603B | #120D0B | #3F3123 | #B89850 | #C8A860 | #78706A | #2C3D52 | #4E88CB | #89AED8 |

| Kind | Rendering | Behaviour |
|---|---|---|
| Place | One row the main column's width, the places one under another `BOARD_PLACE_GAP` apart (`docs/BACKUP-FLOW.md` §17.15): the place box at `RADIUS`, its header band `BOARD_HEAD` high (the house glyph, the place's name in the label type, semibold, its line of fact as a tag `BOARD_TAG` high, outlined and written in the tone the chart gives it, `glance::alone_color`, on `SURFACE`, and a drawn × at its right, `BOARD_MARK`, `BOARD_MARK_STROKE`); under it, `BOARD_PAD` inside, left to right: each stick box, **+ stick** (`BOARD_ADD_STICK` wide) and Paper. The sticks and Paper share the width equally; when they would be narrower than `BOARD_BOX_MIN` each, Paper keeps `BOARD_BOX_MIN` and the sticks wrap onto further lines inside the place, as many to a line as fit. A row is as tall as its tallest box, every box drawn that tall (a wrapped line of sticks as tall as its tallest). After the last place, **+ place**, a row `BOARD_HEAD` high; the software place last. In the narrow column (`compact`) a place's parts stand one under another at the column's width, **+ stick** a row a chip high, under **Add**. | The name is a field only while a vault is open (a press focuses it with the whole name selected and no caret, an accent rule under it; the first key replaces it, an arrow, Home or End lets the selection go, §19.18); otherwise it is text. The × removes the place with what it keeps; with one place left it is not drawn. **+ place** adds an empty place. A place is itself a drop target for a vault's chip. |
| Zone | Paper and Scanned in: a box at `GAP` radius in the paper colours, `BOARD_PAD` inside: its glyph and label in caption, semibold, on a `BOARD_ZONE_LABEL` row, then its chips laid left to right and wrapping, `BOARD_CHIP_GAP` apart; an empty zone keeps a chip's height. **+ stick** is the dashed box with its label alone. The software place's stick of public files is a stick box headed "Stick 1", with no × of its own and no **+ stick** beside it. | A press on a zone places the chip held. A zone is a drop target only for what it can keep in some form (`docs/BACKUP-FLOW.md` §17.3). |
| Stick box | A box at `GAP` radius in the stick colours, `BOARD_STICK_PAD` inside. Its head, `BOARD_STICK_HEAD` high: the USB stick glyph (`Glyph::Stick`, Faraday's own drawing: the icon face has none; the SD card's and "SD card n" on the Pi) and "Stick n" in caption, semibold, and its × at the right. Under the head the files box (`docs/BACKUP-FLOW.md` §17.17), on every stick, laid as Paper is (its glyph and label on a `BOARD_ZONE_LABEL` row, then its chips; empty, a chip's height), then, `BOARD_STICK_PAD` under it, the vault box when the stick carries one. An empty stick a press on **+ stick** added (§19.16) is the same box with its files box alone. | A press or a drop on the files box, or on the stick box outside the vault box, keeps the thing on the stick as a file. Its × takes the stick off with all it carries; after the lock it is drawn only on a stick that carries a vault and no file. |
| Vault box | A box at `GAP` radius inside the stick box in the vault colours, `BOARD_VAULT_PAD` inside. Its head, `BOARD_VAULT_HEAD` high, is the solid band, and the vault's chip: the closed lock, "Vault n · sealed" in the label type, semibold, its link and warning marks, its × at the right, all in `VAULT_ON`; pressed or hovered, `VAULT_ON` at `BOARD_VAULT_HOT` over it. Under the head the records it holds, chips laid as a zone's, each with the lock mark. | A press or a drop on the vault box seals the thing in the vault. The head selects the vault (a press on another place moves its stick); its × takes the vault's stick off this place. |
| Chip | A pill `BOARD_CHIP` high, as the table above draws it, `BOARD_PAD` at each end: its glyph where it has one (the fingerprint before a seed, the passphrase glyph before a passphrase, the closed lock before a vault; the description and a share have none), its name (a fingerprint in mono at the label size, anything else in the text face), then its forms: on the read-only board (before **Customize backup**) after a middle dot in caption, muted ("9a6a2580 · Words · SeedQR"); once customised, where the zone has more than one form for it, as **Form toggles**. Marks at its right, from the edge in: its ⋮ after a thin rule (once customised; **Chip's ⋮**), the × of a placed chip, the warning mark, the link mark, the lock mark (a placed chip in a vault: the lock glyph in `VAULT_EDGE`), each `BOARD_MARK` with `BOARD_CHIP_GAP` before it. A chip whose forms do not fit beside its name drops them to a second line under it (a caption line, or a row of toggles `BOARD_TAG` high). | One chip per thing per zone; its forms are what it carries there. |
| Chip in the list | The same pill, one per row, its count of places at the row's right in caption, muted ("2 places", "1 place", nothing at none); A vault's chip carries its × at the row's right. A cosigner's seed is outlined, not filled, its name dim, "Not here" for its count. A vault no place keeps a stick of is filled as any chip, its name and glyph dim (§19.16). On `wide` the list is the right-hand column where Back up's Map is when the board is not shown (`side_panel_w`, full height, `SURFACE` with a rule at its left), scrolling on its own with its overlay scrollbar, the list from its top, grouped under caption labels: Seeds, Passphrases, Wallet description (Whole, then **Keys left off each share** as a − n + stepper of buttons a chip high, then each share), Vaults with **+ new vault**, Watch-only software. | Never runs out: placing it places a copy. A cosigner's seed takes no press. A vault's × takes it off the plan; one that holds anything first shows what it holds in `WARN` under it, with **Take off**. |
| Add sheet | In the narrow column only, once the board is customised: **Add**, a secondary button a chip high at the top of the board, opens the list as a sheet (the small panel's full-width sheet, its head the add glyph and "Add", the list's groups, each chip with its ⋮, a way out at its foot). | A press on a chip in the sheet, or on its ⋮, opens its **⋮ menu** (Add to…) over the sheet; a box chosen places it and closes both (`docs/BACKUP-FLOW.md` §17.20 item 4). Nothing is dragged there. |
| Placed chip | The chip in its zone, with its × and its ⋮ once the board is customised. A chip of what an existing vault held before is outlined and dim and has no × and no ⋮. | A press selects it. The × takes it off. |
| Chip's ⋮ | Once the board is customised (`docs/BACKUP-FLOW.md` §17.20), at the right end of every chip that can move (in the list, placed, a vault's band): a rule 1 dp wide inset `GAP_SMALL` top and bottom in the ⋮'s colour at `BOARD_MENU_RULE`, then three dots `BOARD_MENU_DOT` across, spread over `BOARD_MARK`, in a part `BOARD_MARK` plus `BOARD_PAD` each side wide; `MUTED` on a chip (`DIM` while it is dragged or dim), `VAULT_ON` on a vault's band; `INNER` behind while pressed or hovered. The × and the other marks stand before the rule; a placed chip's toggles before them. With the keyboard on it, edged in the accent at `FOCUS_STROKE` (on a vault's band in `VAULT_ON`). Not drawn on the read-only board. | Its own press target: a press opens the chip's ⋮ menu and neither selects the chip nor starts its drag. Tab and Shift+Tab move the keyboard among the ⋮s on screen in the order drawn; Enter or Space opens the menu. |
| ⋮ menu | On `wide` a popup `BOARD_POPUP_WIDTH` wide under the ⋮, its right edge on the ⋮'s (above it where it does not fit below; scrolling where it fits neither), on `SURFACE` with `BORDER` and the shadow of elevation 2, no backdrop; on the small panel the full-width sheet with **Cancel** at its foot, its head the add glyph (Add to…) or the chip's glyph. Inside: its title in caption, semibold ("Add to…", "Move to…", "Copy to…", or the chip's name), each place's name in caption, semibold, `MUTED`, and under it the boxes, a row `BOARD_MENU_ROW` high each, the label type, set in `BOARD_PAD`: "Paper", "Stick 1 · files", "Stick 1 · Vault 1", "New stick"; then the software place's. A placed chip's first level: **Move to…**, **Copy to…**, **Take off**. The row the arrows are on is `INNER`, edged in the accent at `FOCUS_STROKE`. | Lists only the boxes that take the item in some form and do not keep it already. A box chosen places it as a drop there does (the checks, the balance sheet), and closes the menu; Move to… moves it with its forms, Copy to… keeps it in both (not for a share), Take off is its ×. Arrows move, Enter chooses, Escape goes back from Move to… or Copy to… to the first level and else closes it, the keyboard back on the ⋮; a press outside closes it. |
| Form toggles | Once the board is customised (`docs/BACKUP-FLOW.md` §17.3), on a placed chip whose zone has more than one form for it, after its name: a pill `BOARD_TAG` high per form, `BOARD_PAD` at each end, `BOARD_CHIP_GAP` apart, at `SURFACE`; the check glyph `BOARD_MARK` in the accent and its name in caption, `TEXT`, with the accent edge, where the form is kept; the glyph's room left empty, the name `MUTED` and the line edge, where it is not ("◉ 9a6a2580 [✓ Words] [✓ SeedQR] ×"). Where they do not fit beside the name, a row under it. There is no forms popup. | A tap turns the form on or off; the last form on stays on. A press on the chip's name selects it, as before. |
| Customize backup | `docs/BACKUP-FLOW.md` §17.6: while the board is read-only, a secondary button a chip high at the top of the right-hand column in place of the list, and in the narrow column in place of **Add**. Read-only, the board draws no × on places, sticks or chips, no **+ place** or **+ stick**, and chips read their forms as text. | A press opens the board for changes: the list (or **Add**) in its place, the board editable, for this plan from then on. |
| Balance sheet | `docs/BACKUP-FLOW.md` §17.5: the scrolling sheet the board's Add sheet is (its width on `wide`, the panel's less `GAP` each side in the narrow column): its head the eye glyph and "<Place> can see the wallet's balance", one line in the label type naming what there makes the description, then **Take it off** (secondary) and **Keep it** (primary). | Keep it closes it, the drop standing; Take it off undoes the drop. A press beside it is Keep it. |
| Partial descriptions | `docs/BACKUP-FLOW.md` §17.19: under the list's Whole, while the description splits, **Add partial descriptions to this backup** (or **Remove partial descriptions** while shares are kept), a secondary button a chip high, then **What is a share?**. No share chip is in the list and there is no keys-left-off stepper. | A press hands one share to every place (Faraday chooses which, and the keys each leaves off) and opens the **Partials sheet**; while on, a place added or taken off has them handed out again; Remove takes every share off. |
| Partials sheet | The Balance sheet's form: its head the file glyph and "Each place now holds a part of the description.", then, where the whole description is still kept, the line naming those places, then **Take it off every place** (secondary, only where taking it off leaves any M places able to rebuild it) and **Keep it** (primary). | Take it off every place removes the whole description from every place's paper and sticks; Keep it closes. |
| One place per key | The board's **+ place** is not drawn once the plan has one place per key of the wallet (`docs/BACKUP-FLOW.md` §17.19); a single key has one place, or two when its seed has a passphrase (the second for the passphrase alone). With a single place, "Any one place lost" reads "No: a single place" in `MUTED`, not as a warning, and the "every secret at one place" line is not shown for a single key. | None. |
| Extra copies line | On the backup's last card (Back up's done card, the wallet flow's Done), in Guided only: "More copies can be made once the wallet is made: …" in 13 regular `MUTED`, then **Extra copies**, a ghost button that opens that Learn page. | Hidden in Steps only. |
| Selected chip | Edged in the accent, `BOARD_EDGE` round its pill. | A press on a zone that takes it places it there (a placed chip moves); a press on it again, or on anything that is not the board, lets it go. |
| Dragged chip | A copy of the chip follows the pointer, centred under it, with the shadow of elevation 4 and the accent's edge; the chip it came from is dimmed where it stands. | A press is a drag once it moves past the scroll slop, and a tap otherwise. The lift drops it on the zone under it that takes it; anywhere else it goes back. |
| Checks under the board | Under the places, **+ place** and the software place, on `wide` and in the narrow column: the plan's three checks as the Map panel drew them (Any one place lost, One place found: can spend, One place found: sees the balance, each with its value, `glance::check_lines`), then the warning lines, each the warning glyph and its line in `WARN`. The top of the board is kept free. | The checks follow every drop on the next frame. |
| Edged drop target | The zone, the stick box, the vault box, the place or the list column edged in the accent, `BOARD_EDGE`, at its own radius. | While a chip is selected, every zone that takes it is edged; while one is dragged, only the one under the pointer that takes it. A zone that does not take it is never edged. |
| Link mark | Two links of a chain drawn as stroked boxes, in the accent. | On a vault's chip whose sticks are at more than one place: what it holds is drawn at each, and a drop into it at one is in it at all. |
| Warning mark | The warning glyph in `WARN`, and the chip's pill edged in `WARN`. | On a placed chip that one of the plan's checks names. The checks themselves are lines under the board, the warning glyph before each, in `WARN`. |

## 5. The screens

A screen is one of the thirty-three variants of `Screen` in
`faraday/faraday-core/src/lib.rs`, drawn in the file that file's code
map names: an area's `screen.rs`, or `screens/<name>.rs` for a screen
with no area. Each is drawn in one of three ways:

- **Step flow**: the step-card column (`flow::column`): the back link,
  the heading with its chip, **Steps only · Guided** and **Cancel**, then
  one card open at a time, the others closed to a line that says what
  was chosen (§4.14, Default). On the small panel, a page per step with
  the bar's tools (§4.1).
- **Page**: the title and what the screen holds, from the top, scrolling
  where it is taller than the window; on the small panel under the bar.
- **Sheet**: a box over the page, which is frosted and dimmed behind it
  (`Ui::backdrop`, `faraday_ui::parts::sheet_box`); its head is its
  icon and title (`faraday_ui::sheet::sheet_head`), its buttons at its
  foot. On the small panel it is the panel's width less a margin, and a
  long one scrolls (`faraday_ui::sheet::scroll_sheet`). A press beside
  it closes it where closing loses nothing.

What each flow does, step by step, is `docs/FLOWS.md`; the Wallets tab
is `docs/WALLETS.md` and the Spend tab `docs/FAMILY.md`.

| `Screen` | What it holds | Drawn as |
|---|---|---|
| `Home` | The lead tile while something is under way or waiting, then the tiles; on the small panel the menu of the sidebar's entries; **Scan** (`docs/FLOWS.md`, Home). | Page |
| `WalletsStart` | The Wallets tab's first page: the wallets loaded, the keys without a wallet, and every way in by what the person has. | Page |
| `Wallets` | The wallets as a list and the chosen wallet's card: its name and shape, the backup line, the signatures possible here, **At a glance** (§4.16), the first receive address and the card's actions (`docs/WALLETS.md`). | Page |
| `Spend` | Signing a transaction: the review (what it sends, to whom, the fee, change, the txid, warnings), the signers, **Sign with** (§4.4), and the signed PSBT and finished transaction. | Step flow |
| `Family` | The Spend tab: spending from a backup, step by step (`docs/FAMILY.md`). | Step flow |
| `Files` | The Inbox and the Outbox: a row per file with its kind, size and ⋮, and what each file can do (Sign, Decode, Load the wallet, Unlock, Import into the vault). | Page |
| `Visit` | A stick visit: what is on the stick, ticked to bring in, and what goes out to it. | Page |
| `Entry` | Add a key: a seed typed or scanned (§4.3). | Page |
| `Backup` | Backing up a wallet: the plan, the board (§4.17), each copy made and checked. | Step flow |
| `BackupFlow` | The shared backup flow for a key that is not a wallet's: where it will be kept, its sticks and vaults, the lock and the writes (`docs/BACKUP-FLOW.md`). | Step flow |
| `Backups` | Every wallet known and where its backup is, a card each; on the small panel one wallet a page with **Previous · Next**. | Page |
| `Create` | Making a new wallet: Kind, the quorum, whether vaults are part of the backup, the keys, Check, Back up. | Step flow |
| `Restore` | Restoring a wallet from its description and its seeds. | Step flow |
| `KeyGen` | Making a new key: the source of randomness, its entries (§4.3), the length, the passphrase, lock in; or, with Codex32 chosen, Make codex32 shares' cards (`docs/CODEX32.md` §6): Scheme, Identifier, Entropy, the entry with the worksheets filling live (§4.10 Worksheet), Worksheets, Strings and the Result **Strings made**. | Step flow |
| `Message` | Signing a message with a single-key wallet's key. | Step flow |
| `CheckMessage` | Checking a signed message against its address. | Page |
| `Decode` | A transaction taken apart, field by field. | Page |
| `Explore` | A loaded key's public side at a path: the path field, the script, the address, public key and extended key; the account's addresses and Find an address (F76). | Page |
| `Bip85` | BIP-85's applications from a loaded key: a child seed's words, a private key, a password, at an index. | Step flow |
| `Silent` | Silent payments: a loaded key's address, its labels and the payments checked. | Step flow |
| `Vanity` | A vanity address for a loaded key: the script, the prefix, the dial, the grind and its find. | Step flow |
| `Lightning` | A Lightning node's key, from a loaded key or an LND aezeed. | Page |
| `Catalog` | Tools: every flow on one page, by group, each with its line and the standard it follows; the sidebar's Tools. | Page |
| `Tools` | The calculators: hashes, encodings, a descriptor's checksum, an xpub's spellings, units, the Miniscript compiler. | Page |
| `Vaults` | The vaults in Files and those open. | Page |
| `Unlock` | Unlocking one slot of a vault with its passphrase. | Page |
| `VaultContents` | An open vault: its kinds, the chosen kind's items, the chosen item, with the add actions; a wallet's item is its chart read parts first (§4.16). | Page |
| `CreateVault` | Making a vault: where it will be opened, the cost of unlocking, the space per passphrase, its name and passphrases, its size. | Step flow |
| `Codes` | TOTP codes for the entries in open vaults, with the time they are computed for (`docs/TOTP.md` §5). | Page |
| `SetTime` | Setting the time by hand: Date and Time (`docs/TOTP.md` §3). | Page |
| `Settings` | The session and Lock; lock and power-off timers; Mode; Appearance (the themes, §3, as tiles in three groups, Light, Dark and High contrast, each in order of its names, Cobalt's tile marked Default), Motion and the display scale; what is remembered; About. | Page |
| `Transfer` | The desktop app as the device's QR link: files out as codes, in by camera. Only in the desktop app. | Page |
| `Upgrade` | Upgrading another Faraday stick from the one Faraday started from. Never in the desktop app. | Page |

The sheets (`Sheet` in `faraday/faraday-core/src/lib.rs`, drawn in
`faraday/faraday-core/src/screens/sheets.rs` and the area that owns
them) include the QR sheet and the scanner (§4.9), the Learn sheet, the word list, a secret on its
way out (§4.10), the lock and its question, the idle warning and the
locked sheet (§4.14), power, a new input device, what is attached that
can capture the screen or the keys, a seed read by the camera, a chart
node's sheet (§4.16), the board's list, its balance and partials sheets
and its ⋮ menu (§4.17), a file's ⋮ menu (§4.1), Rename and Details, the
mode sheet, Load samples (§4.8), and the question a flow's Cancel asks
(§4.13).

In Faraday (`faraday-ui`) every page's working area is one colour, the
theme's surface (`SURFACE`), and so is a sheet; the sidebar keeps its
own shade (`SIDEBAR`). A step flow's steps are flat sections divided by
a thin rule in `LINE`, with no fill and no box. The right-hand column
(Back up's list or Map, the side panel, `side_panel_w`) is the same
colour, divided from the page by the same rule. A card or tile on the
page is set apart by its edge alone, `BORDER` (`docs/DECISIONS.md` F39).

## 6. What changes from today

Moved to `docs/history/DESIGN-LOG.md`.

## 7. Open questions for the owner

Moved to `docs/history/DESIGN-LOG.md`.

## 8. How this gets built

Moved to `docs/history/DESIGN-LOG.md`.
