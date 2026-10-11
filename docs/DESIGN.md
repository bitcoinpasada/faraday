# Design system

**Status:** v0.7 · 2026-09-09 · approved by the owner at v0.2; v0.3 folds
in the owner's review of the gallery (passes 1 and 2); v0.4 adds the
ergonomic rules from the review of pass 3; v0.5 replaces the zone rule
with screen intents and one placement law (§2.6, §5); v0.6 folds in the
whole-app re-review of the rebuilt screens (the QR side and part count,
the `wide` column, the words-so-far panel, the path editor, and the
places where a rule and the build disagreed); v0.7 puts the camera's
own frames inside the scanner's square and adds the camera-rotation
setting (§4.9). It replaced UX.md §5 and
§6 and the composition specs on 2026-09-09; UX.md keeps the jobs,
the navigation and the flows. Every dimension in it is a name in
`core/osk-ui/src/tokens.rs`, and `tools/lint-tokens.sh` keeps it that
way.

## 1. Why this exists

Six fix passes and one batch of composed screens improved individual
screens without making the product uniform. Screens were built one at a
time from findings, so the same kind of content is drawn several ways.
The fix is to decide once, per kind of content, how it is shown and how
it behaves, and then build every screen from those decisions.

This document is that decision list: the principles, the inventory of
content kinds with one rule each, and the reusable screens built from
them. A screen spec after this is short: which reusable screen, which
content, what is left off.

## 2. Principles

1. **Labels, not prose.** The interface is titles, labels, values and
   actions. Sentences appear only as a one-line reason on a dimmed
   control ("no camera"), or in Learn.
   Nothing on a working screen tells a story or restates what the action
   button already says. Idioms are not labelled: an orange-outlined box
   is a secret and needs no "hold to show" under it. Learn and About are
   not working screens and may hold paragraphs. Text longer than a label,
   wherever it appears, is plain statement of fact in an encyclopedia
   voice, never metaphor or flourish in place of direct statement.
2. **One kind of content, one rendering.** Every content kind in §4 has
   exactly one way of being shown. A screen never invents a variant.
3. **One screen per kind of data.** Seed words, a PIN, a QR code, an
   address, a secret string, a derivation path each have one screen that
   every flow reuses. The flow changes the title and the action, not the
   layout.
4. **One item per row on touch screens.** Options are a list, one per
   row. The one exception is a pair of short options (Receive | Change),
   which sit side by side as two equal buttons.
5. **Fixed geometry for anything the user sees repeatedly.** The PIN
   pad, the keyboard, the secret panel and the record table are the same
   size in the same place every time. Content never resizes a control.
6. **Every screen has one intent, and one placement law places it.**
   A screen navigates, chooses, enters, helps understand, transcribes,
   shows to a camera or reads (§5). The law: actions and the controls the
   thumb works (buttons, keyboards, pads, toggles, pairs) are
   bottom-anchored; lists and documents start at the top and scroll;
   everything else is one block, vertically centred in the space between
   the app bar and the actions. Nothing else is top-anchored, and no
   block is placed by how tall it happens to be. A block that does not
   fit pages or scrolls. The same law holds on `wide` inside the width
   caps, with the primary action bottom-right at its natural width.
   Every screen is designed for the 268 × 358 dp panel first, where the
   law and "it fits" coincide.
7. **Direct manipulation, one confirm idiom.** A choice is checked, then
   confirmed with Continue. Typed or rolled input has a Continue. A hold
   confirms an action the app cannot undo on its own: signing, forgetting
   a key, wiping. No other confirm dialog, and the word "irreversible"
   appears nowhere: the app is offline, and nothing it does is final
   until something outside it acts.
8. **Secrets are a place, not a state.** Short secrets (words, one-line
   values) live in a panel that masks in place with the same geometry
   either way. Long secrets and long strings live on their own screen,
   reached by a row. Nobody is shown a hundred bullets or a six-line
   string inside another screen.
9. **One unit.** Amounts are shown in sats or in BTC, by setting, never
   both.
10. **No pressure.** Nothing counts down at the user except the eye's
    own timer. Status shows state, not urgency.
11. **Common idioms, not invented ones.** Where a phone or a desktop has
    a well-understood pattern, the screen uses it: a keyboard has a
    focused text field above it; a settings list is rows; a choice is a
    row with a check; a review is a table. Spare space is never filled
    with decoration or with a second rendering of the same value.

## 3. Sizes

| Class | dp frame | Renders | What changes |
|---|---|---|---|
| `small` | 268 × 358 | 480×640 @ 286 dpi (the 2.8" panel), 240×320 @ 143 dpi | the design target; everything fits; the text faces in their SemiCondensed cut |
| `mobile` | 411 × 891 | 1080×2340 @ 420 dpi | type ×1.1; the content group sits above the bottom action; codes capped; keyboards grow |
| `wide` | 960 × 640 with a 240 sidebar | the desktop window | type ×1.3; the sidebar on every screen; the same placement law inside the width caps; the column, the code, the pad and the viewfinder centred in the pane; the primary action at the pane's bottom-right |

Tokens (dp): app bar 56, bottom action 52 with 16 below, screen padding
16, gap 8, row 52 on `small` and 72 elsewhere, touch floor 44 — a
candidate cell alone drops to 36 on `small`, so that two rows of them
fit over the keys (§16.118). On
`mobile` a 24 dp safe-area inset is added under everything at the bottom
edge (the action row, a pad, the keyboard's bottom faces), because a
phone draws under its gesture bar; the shell may report the real inset
later and the token gives way to it. Type:
title 20, body 16, label 14, caption 12, mono 16; a line box is about
1.43 × the size. Mono 20 is the words panel on `mobile` and `wide`,
where a word is copied character by character; mono 15 is that panel
with the wordlist numbers beside the words, which is four more
characters a row; mono 14 is a steel row, which is the widest row the
panel draws; 13 is the size a dense row of chips drops to before it
wraps. A comparison string steps the mono ramp — every second dp — from
32 down to 8: 32 is the largest at which four groups of four fit the 480
dp column, and below 8 a group of four is guessed rather than read. On
`mobile` the ramp stops at 24, because there the box binds on nothing
and the eye sets the ceiling instead.
Width caps on `wide`: a form or list column
480, a button 320, a code 240. Code side: 198 on `small`, 260 on
`mobile`, 240 on `wide`. On `wide` the capped column, and every list in
it, is centred in the pane under the centred title; a scrollbar is drawn
against the column it scrolls, not against the window edge.

Theme: background `#000000`, surface `#1C1C1E`, raised surface
`#3A3A3C`, accent `#FF9F0A`, text `#FCFCFC`, muted `#98989E`, divider
`#2C2C2E`, success `#30D158`, caution `#FFD60A`, danger `#FF453A`, info
`#409CFF`. A pressed surface is drawn from its own resting fill: a
neutral one mixed `PRESSED_TINT` towards the accent, an accent one
mixed `PRESSED_MIX` towards white. A control drawn on the background
itself — a Text button, the app bar's icon buttons and its eye while a
reveal runs — has no fill of its own to tint, so it takes the raised
surface mixed `PRESSED_TINT` towards the accent (§16.132). The look outlasts the lift by
`PRESS_LINGER_MS` in two steps — full strength for the first
`PRESS_LINGER_FULL_MS`, both mixes halved for the rest (§16.124).
Faces: Noto Sans and Noto Sans SemiBold for text — their SemiCondensed
cuts on `small`, the same letters with the same line height, so a row's
label fits between its icon and its chevron (§16.130) — Iosevka for data, and
a closed icon set (`widgets::Icon`, taken from the SeedSigner
icon font and Font Awesome 6 Free Solid); no screen pulls a symbol out
of a text face. Each glyph is stored once as an outline and filled at
the exact pixel size a display asks for; no size is rounded to another
size's bitmap.
Anything read character by character is set in the mono face: fingerprints wherever they appear, including a title and a row label — and so a label or a title that names a key, such as a wallet called by its key ("73c5da0a · SegWit") or a signature ("#0 · 73c5da0a"), and an input's row, which states its amount, script and key —
hex, paths, addresses, xpubs, every comparison string, the seed words
and the words panel, the candidate strip and the entry field on a
wordlist, the word list tool's rows and the word page's title and
pager, and every keycap on every keyboard and pad; prose, labels, buttons and titles that are words stay in the
text faces, and a kana, jamo, bopomofo or hanzi keycap or candidate is
in the CJK face (§16.126).
One meaning per glyph: a glyph is either a fact marker —
the key and the eye of §4.4 — or the identity of one row, never both.
Within one menu no two rows share a glyph, and a label that appears in
two menus carries the same glyph in both. A result's severity marks
(success, warning, error, info) are not row bullets, and a row that
states a value carries no glyph.

Size classes swap variants, sizes and placement. They never reorder,
hide or add content. A class may shorten a value ("9.0 · normal" for
"9.0 · normal for 50 rolls") and may fold progress into the title where
a progress row does not fit; it never drops a row and never changes a
title's form. The one written exception: the Sign review on `small`
leaves off the key context row to keep the table short, and the Hold
screen names the signing key before anything is signed.

## 4. Content inventory

Each kind: its one rendering, how it behaves, the component that
implements it. Names in `code` are components in `osk-ui`.

### 4.1 Navigation

| Kind | Rendering | Behaviour |
|---|---|---|
| Hub | Home is the launcher (`launcher`): the status line, then six equal tiles in two columns of three — Wallets · Keys · Scan · Tools · Learn · Settings — and nothing else. Home never changes shape: what is loaded is on Wallets and on Keys, which carry their own empty states, and Scan is the one way in for anything read. `wide`: the sidebar lists the same six in the same order and the pane shows the same grid. | The grid fits the panel without scrolling. No tile carries a badge. A tile opens its area; Scan opens the scanner, which routes by what it reads. |
| Status line | 56 dp on Home: tier badge, network badge (off mainnet), a session badge ("Session open") while the device holds a MuSig2 secret nonce, then at the right the lock icon button (tap locks now). Nothing else; no countdown, no gear. After one Back on Home the badges give way to the caption "Back again to exit and clear memory" for two seconds, on the caption line every field reserves (§4.3), so the list does not move. | Settings is a tile of the band and a row of the sidebar, like Tools and Learn. |
| App bar | 56 dp: back chevron, centred title, one optional trailing action (`app_bar`). | The chevron is the only way back and the only cancel. The trailing slot holds the eye on a secret screen; otherwise it holds the info button on a screen that has a Learn page, which opens that page; otherwise it is empty. One control there, never two. |
| Menu row | Full-width row: optional icon, label, chevron (`menu_row`); the label starts one gap (8) after the icon. With a value: the label above and the value below, both left-aligned, chevron right (label above, value below, everywhere a row carries a value). A label that does not fit at its size is drawn at the label size; one that still does not fit breaks at a space onto a second line at the label size, and the row grows where its height does not hold the two lines. A single word wider than the row is cut at the row's edge (§16.133). | Tap opens. |
| Sidebar (`wide`) | One row per area, the current one in the accent (`sidebar`); Learn is the row after Tools, opening the Learn sheet rather than a screen. Below the rows, the session strip: one line, **Bring in · Open · Work · Write out**, the current stage in the accent, the rest dim. | Dimmed and inert during a wizard, the scanner, the lock screen and the hold screens. The session strip opens Files; it never carries a count down. |
| Pager | `‹ label ›` in the bottom action slot (`pager`). | For runs of same-kind pages only (word pages, outputs). Never for two different views of one thing: those are a row or a button that opens a screen, and Back returns. |
| Title | Short; carries progress where progress exists ("Word 7 of 12"). Where a Choice has a progress row and the row does not fit the class, the title carries the progress instead ("Word 10 · 1 of 12"). | No step indicators anywhere. |

### 4.2 Choice

| Kind | Rendering | Behaviour |
|---|---|---|
| Choice | A list of full-width rows (`choice_list`), one option per row; the chosen row carries an accent check; Continue at the bottom. The question is the title ("How many words?"). The rows are one group centred in the space above Continue; a list taller than the space scrolls from the top. | Tap checks (the list keeps its scroll position). Continue is dimmed until a row is checked; a default is pre-checked where one exists. Used for: source, word count, language, how a transaction arrives, script type, unit, network, timers. |
| Multi choice | A list of full-width rows (Faraday's `Ui::multi_list`; no `osk-ui` component yet), one option per row, a checkbox at its start; any number ticked; Continue under it. The question is the card's or the page's title. A row that may not be ticked is dimmed and inert. Several lists may share a page, each under its own label. | Tap ticks or unticks the row. Nothing has to be ticked. Used for: the backup plan's questions (where the seeds go, which places keep the vault's stick, where the wallet description goes, the software and the form, a passphrase's places). |
| Setting | The same rows with the check on the current value; no Continue. | Tap applies at once and the screen stays. Used inside Settings only. |
| Pair | Two equal buttons side by side, the chosen one on the accent (`pair`). | Exactly two short options: Receive \| Change, One code \| Animated where a toggle does not fit. |
| Toggle | A row with a switch (`toggle_row`). | A boolean setting; dimmed with a reason when forced. |
| Value row | Label above, value below, chevron (`value_row`). | A mode shown as its value; tap opens the Choice screen for it. |

Rows of more than two chips are not used on any class.

### 4.3 Entry

| Kind | Rendering | Behaviour |
|---|---|---|
| Text field | 52 dp, single line — 40 dp on `small`, a letter key's own height — the value being typed (`text_field`). Directly above its candidates or keyboard, the same width as the pad or keyboard under it. | The tail stays in view for long values. Nothing is typed without a field. |
| Keyboard | Bottom-anchored, full-bleed, one group with the field (`entry_group`). Keys 40 dp on `small`, 56 dp on `mobile`; on `wide` the keyboard is capped at 720 dp and centred. | Kinds: BIP-39, kana, jamo, pinyin, 注音, passphrase, path, hex, decimal, bech32/base58, codex32 (the bech32 alphabet alone: no `1`, no `b`, no shift to base58). Dead keys dimmed. ✓ dimmed until the input is acceptable. A key under a finger is drawn pressed — its own fill tinted towards the accent, ✓ mixed towards white — and stays so for `PRESS_LINGER_MS` after the finger lifts, halved for the second step of that window, so the key is seen once the finger is off it (§16.120, §16.124); a dead key never is. A wordlist keyboard — BIP-39, kana, jamo, pinyin, 注音 — has no ✓ at all: a word is taken by a tap on its candidate and by nothing else (§16.118), and the backspace holds the bottom-right cell. On the BIP-39 rows it is the 1.5-unit key ending row three, the seven letters filling the rest. |
| Kana keyboard | The 五十音 as a grid: ten columns of the consonant groups あかさたなはまやらわ, five rows of the vowels あいうえお, every key one unit wide. Keys 28 dp on `small`, where five rows are the height a three-row keyboard has. The cells the grid leaves empty carry ゛, ゜ and 小; the backspace takes the bottom-right cell, ん the one above it, and the fifth cell is empty, because four keys no longer fill five. | Types the Japanese list. ゛and ゜ add the voicing marks the list is written with; 小 turns the last kana into its small form and back. The field composes what has been typed: か and ゛ read as が. |
| Jamo keyboard | The two-set layout on the QWERTY positions: ten, nine, and seven keys with shift at the left and the backspace at the right of the last row, the same 1.5-unit edge keys as the BIP-39 row 3. | Types the Korean list. Shift gives the five doubled consonants and the two shifted vowels and releases after one key; a shifted key is dead or live by the character it types. The field runs the keys through the two-set automaton, so ㄱㅏ reads as 가 and ㄱㅏㄱ as 각. |
| Pinyin keyboard | The BIP-39 letter rows unchanged, and a fourth row of the five tone keys 1 2 3 4 5 with the 1.5-unit backspace ending it. Keys 28 dp on `small`, as the kana grid's: four rows over the two-row one-character strip are the height problem five rows of kana are (§16.121). | Types the Simplified Chinese list, which is looked up by a Mandarin reading rather than spelled: the syllable on the letters (`v` for `ü`), then its tone. A letter is live while some candidate's syllable continues with it; a tone key is live in the tones that syllable has. The tone completes the reading, and the strip fills only then: a syllable on its own leaves far too many characters to choose between, and an incomplete reading is not an error, so nothing is offered and nothing is said. The field shows what was typed, `zhong1`. |
| 注音 keyboard | The 大千 layout, four rows on the QWERTY positions: eleven keys in the first two rows, ten in the others, the tone marks ˊ ˇ ˋ ˙ in the first row where that layout puts them and ㄦ at its end, ˉ ending the second row where 大千 has the space bar this keyboard has no room for, and the 1.5-unit backspace ending the last row. The key width follows the row. Keys 28 dp on `small`, as the kana grid's (§16.121). | Types the Traditional Chinese list. Every tone is a mark key, ˉ for the first, so a syllable ends the same way here as on the pinyin keyboard: the mark completes the reading and the strip fills only then. The field shows the bopomofo and the mark, ㄓㄨㄥˉ. |
| Candidates | Two rows of 3 at 36 dp (`small`), two rows of 4 (`mobile`), one row of natural-width chips (`wide`) between the field and the keys (`candidate_strip`); on the two cell classes the cells are a fixed width whatever the count, the lone candidate in the first cell and the rest blank, so a cell never changes width from one word to the next (§16.120). A list whose words are one character — the two Chinese ones — shows ten a row and two rows on every class, full-bleed at the keyboard's own key pitch, so a whole tone group (20 at most) is on the screen at once. | A tap on a candidate is the only thing that takes a word: a word typed out in full is the one candidate left and waits in the strip until it is tapped, on every class (§16.118). Tap accepts on `mobile` and `wide`. On `small` the first tap selects a candidate and the second accepts it: the cell is 24 px there and a fingertip covers it, so the tap shows what it hit before it is taken. A selected cell is drawn as a chosen cell is, on the accent colour, and shows its own press by mixing that accent towards white; a tap on another cell moves the selection and accepts nothing; a key or a backspace clears it. An unselected cell under a finger takes the accent tint as a chip does, and a cell of either kind keeps its pressed look for `PRESS_LINGER_MS` after the lift, halved for the second step of that window, as every tappable thing does (§16.120, §16.124). The strip's words are drawn at the largest size, up to the class's own (16 dp on `small`, 18 on `mobile`), at which a word of eight letters fills its cell; a longer word — Italian has nine-letter ones — takes the size at which it fits, in its own cell alone, and is never clipped. A lone candidate is already selected, so it takes one tap, and a reading that leaves one character is selected rather than committed. Where more candidates remain than the strip holds, the last cell carries the chevron instead of a word and turns the page; every key puts the strip back on the first page. The lone candidate is outlined where a tap accepts. A physical keyboard's Enter takes the selected candidate or the lone one and does nothing with several. Words are shown the way their readers write them: が as one kana, 가격 as syllables, ábaco with its accent. |
| PIN pad | 3 × 4 pad, fixed size, centred, 280 dp wide at most; the dots field directly above it at the pad's width; the group bottom-anchored on `mobile` (`pin_pad`). | Same geometry on the lock screen, the stored key's pad and both set-PIN steps. Word numbers types on the same grid with the ✓ cell left empty — no key and no target — because a word is taken by a tap on its candidate; the digits and ⌫ stay where the PIN pad has them, and every other pad keeps its ✓ (§16.132). The lock screen and the stored key's pad are one pad to the person: the same title, the same caption, and on a device that keeps a key the same PINs do the same things on either (§16.63). The pad is in digit order unless the "Shuffle PIN pad" setting is on; it is off by default, because nothing on the pad screen can explain a shuffle. A shuffle changes digits, not layout. |
| Dice, coin pads | Fixed pads like the PIN pad. Directly above the pad, inside the pad group at the pad's width: the entries so far in groups of five, masked as a secret (the newest entry shows for half a second, then masks; hold or the eye shows them all), then the progress line "23 of 50 · 59 bits" and its bar. No colour on the groups. The eye moves nowhere between the pad and what it typed. | Continue when the target is reached. |
| Inline error | One caption line under the field, always reserved at the caption's line box for the class (`inline_error`). | Appears in the danger tone; nothing moves. |

An Entry screen never carries the key row: typing words or a passphrase
is not about a key yet. On `mobile` and `wide` the space above the entry
group holds, for word entry, a masked words panel of the words accepted
so far, centred in that space, so a typo can be checked without leaving
the screen. The panel is drawn only once a word has been accepted (an
empty panel says nothing), at the fixed geometry of the word count, and
while it is drawn the app bar carries the eye, as on every screen with a
words panel. On `small` the screen shows the word being typed and
nothing else.

An address being typed (Verify) is the one value whose tail is not
enough: the space above the entry group holds the typed address whole,
as a comparison string at the largest size that fits, on every class,
and the field under it keeps the tail in view as usual.

The same space holds a fact row above the entry group where what is
typed derives one value the person will recognise: the fingerprint of a
passphrase key or a child seed. It is one record row, label and value,
centred in the space and reserved the way the typed address is, so the
value changes on every keystroke and nothing under it moves. Where what
is typed derives no key — an empty passphrase, an index out of range —
the value is the placeholder for no value, so nothing on the screen
reads as a key to add.

The block above the field comes before the keys. Where the reserved
block — a fact row, a mode row, or the typed address — does not fit
above the field at the class's key height, the keyboard drops to the
36 dp floor of the key range; where it still does not fit, the row is
drawn on one line, the label and then the value, both at the label size.
Nothing is drawn over the field on any class. On `small` both halves of
the rule are in use: a typed address needs about 30 dp the 40 dp keys do
not leave, and a mode row over the five-row passphrase keyboard has the
height for one line and not two.

### 4.4 Identity and keys

| Kind | Rendering | Behaviour |
|---|---|---|
| Fingerprint | Eight hex characters in mono, never split. Shown as a value — a row's value, a fact row, a record cell, the key context, a chip — it is the fingerprint glyph and then the characters: the glyph at the value's size, one small gap before the first character, in the value's colour, and before each fingerprint of a value that lists several. Not on a row whose bullet is already the fingerprint glyph, not inside a longer string (an origin, a descriptor, an account key) and not in an app bar title (§16.131). | The key's name everywhere. |
| Key row | The fingerprint glyph, the fingerprint above, what the key is made of below — "12 words", "24 words and a passphrase", "BIP-85 child", "SLIP-39 shares", "Codex32" — chevron (`key_row`). Never the network, which is the device's and not the key's. | Keys, and every key chooser. |
| Wallet glyph | A wallet at the start of a row, where the row's icon sits. | This device holds a private key of the wallet, so it can sign for it: a 2-of-3 with one of this device's keys in it carries the wallet. Never on a key row, where the fingerprint glyph sits. |
| Eye glyph | An eye at the start of a row, in the same place. | Public key only: look, derive and compare addresses, never spend. A wallet with none of this device's keys carries the eye. |
| Fingerprint glyph | A fingerprint at the start of a row, or before the characters of a fingerprint shown as a value. | The eight hex characters beside it are a fingerprint. Every key row carries it. |
| Key context | A two-line row like every other row with a value: "Key" above, the fingerprint(s) below in mono, chevron (`key_context`). No chip, no caret. On a surface among menu rows; flat inside a Record's table, like the reference row beside it. In Explore with typed words and no loaded key the value is "Typed words". | Only on the Sign review and in Explore, where the screen acts on a key the user can switch. Never on Entry, never with "Sign with". Tap opens the key chooser. |
| Sign with | Caption and a row of fingerprint chips on the Hold screen. | Tap toggles a key in a multisig. |
| Then with | A second "Sign with" row under the first, captioned "Then with": the other members of a FROST wallet that will sign at a later location, each named by its own fingerprint as the member row names it. Only at the first location of a FROST spend, where the signer set is chosen; a later one signs with the set the file it read names. | Tap toggles a member. Exactly the number still to sign may be chosen, and the hold is dead until they are. |
| Member row | A row of a FROST wallet's Keys review: the fingerprint of that member's public share (hash160 of it), with the wallet glyph where a loaded key computes that public share and the eye where none does. | A 24-word key that is a member of a group has two fingerprints — its BIP-32 master's, which Keys shows, and its public share's, which this row shows — and the paper beside the words carries the second. The device matches by public share and never by either label (§16.104 rule 3). |

The wallet and the eye appear wherever a row could be either: the rows
of Wallets, a wallet's and a transaction's Keys review, and the member
rows of a FROST record. They say whether this device can sign, so they
appear nowhere else: no menu row uses the wallet or the eye as its
bullet, and neither ever stands on a key row, where the fingerprint
glyph says what the characters beside it are. A wallet's "Key" or
"Keys" row carries the fingerprint glyph, and the scanner's way in from
a loaded key carries it too. The MINE badge keeps its one meaning — an output or
an address that pays one of this device's wallets — and is not used on a
key row. Wording follows the glyph: a wallet's subtitle says "SegWit",
never "read-only" or "watch-only".

The eye also means "reveal" on the app bar of a screen with a secret
panel (§4.10); there it is a button, here a marker, and the two never
share a screen. Both meanings are "look".

### 4.5 Long strings

| Kind | Rendering | Behaviour |
|---|---|---|
| Reference row | A row: label above, the elided value below as `xpub 6CUG … Au3f DVmz` (first 8 and last 8, in groups of four), chevron (`reference_row`). The value drops a size where the row is too narrow for it at the mono token. The whole row is the target. | Every long string wherever it appears inside another screen: xpubs, master keys, seed hex, signatures, txids, addresses in a transaction review. Tap opens the Compare screen. |
| Comparison string | Whole, mono, chunked in fours with alternating colour, largest size that fits (`comparison_string`). | Only on the Compare screen and the Address detail screen. Never inside another screen. |
| Descriptor | Structure: function, origin in the accent, the key as a reference value, suffix and checksum whole (`descriptor`). | On the Compare screen for a descriptor. Inside another screen a descriptor is a summary row, not an elision: the script type in §4.6's vocabulary, the origin fingerprint and the checksum, "SegWit · 73c5da0a · #qf45pmyh", label above, chevron, opening the Compare screen. Faraday has no Compare screen: its row (Create's Check) opens the wallet's QR sheet, which carries the descriptor's whole text under the code. |

PLANNING §13's "always the whole string" rule now means: whole on its own
screen, one tap away.

### 4.6 Derivation paths and script types

| Kind | Rendering | Behaviour |
|---|---|---|
| Path | A mono string: `m/84h/0h/0h/0/0`. As a row: label "Path" above, the string below, chevron when it opens the editor (`path_row`). | No per-level crumbs. |
| Path editor | The Entry screen with the path keyboard. Above the field, in the space the law centres: the presets as two labelled groups, each with its own check, because they are two independent choices. "Purpose": four choice rows named in §4.6's vocabulary (Legacy, Nested, SegWit, Taproot) with the path as the value under the name; "Chain": the Receive \| Change pair. A row's check follows the typed path, so editing the text unchecks what no longer matches. On `small` the groups scroll from the top and the checked row is scrolled into view when the editor opens. | ✓ dead while the text does not parse. Never "BIP-84" on the screen; a dimmed reason names script types ("only for Nested and SegWit"). |
| Script type | A value row: "Script type" above, "SegWit" below, chevron; chosen on a Choice screen. | One vocabulary: SegWit, Taproot, Nested, Legacy. Never "BIP-84" or "p2wpkh" on a working screen. |

### 4.7 Amounts

| Kind | Rendering | Behaviour |
|---|---|---|
| Amount | In the unit the setting names: "60 000 sats" or "0.00060000 BTC", grouped with a no-break space. | The setting's rows read "sats" and "BTC". No echo in the other unit. |
| Fee, Rate | Two record rows: "Fee · 1 000 sats" and "Rate · 7.1 sat/vB". | The rate row is absent when unknown. |

### 4.8 Badges

| Kind | Rendering |
|---|---|
| Network | A caution pill "REGTEST" / "TESTNET" / "SIGNET"; absent on mainnet. On Home and on every Sign review screen. |
| Tier | "Tier C · desktop" on Home's status line and in About; on the 268 dp status line, where the full form does not fit beside the network badge and two buttons, "C · desktop", and the letter alone, "C", while the session badge stands there too. Opens the tiers screen. The gallery draws the same forms. |
| Mine | On a transaction output that pays one of the loaded keys: "MINE" (success) when the change was verified, "MINE · NOT VERIFIED" (danger) when not. Outputs to others carry no badge; there is no "recipient" badge and no "kind" row. The review's change entry carries the same success badge under the amount when every change output was derived from a loaded key, so the state is read without walking the outputs; where one was not, the entry is the state value "not verified" in the danger tone instead of an amount. |
| State value | Backup "not verified" in the caution tone as a row value; never a badge. |

### 4.9 Codes

| Kind | Rendering | Behaviour |
|---|---|---|
| QR out | A square at the class's side (198 / 260 / 240 dp), centred, its label under it (`qr_block`). The side is the token on every screen that shows a QR: the Address screen, the QR screen, the Secret screen for a SeedQR. The screen finds the room; on `small` the toggle row and the progress row give way before the square does: the toggle is one line at the touch floor with its reason beside the label, the progress row is the bar alone with the count folded into the title ("Transaction · 1 of 3", the §3 allowance), and a static QR has no progress row there. The word is "QR", never "code", including in the scanner's state and the unknown-QR result. The label under the square names what it is ("Descriptor", "Receive 2", "signed transaction"), never the screen's own title again. | The module pitch floor (`QR_MIN_PITCH_MM`, 0.5 mm) is a hard floor at the class's side. A payload whose one QR would fall below it is split into as many BC-UR parts as keep every part above the floor: the part count follows from the side and the floor, not from a fixed fragment size. An "Animated" toggle row under the square, dimmed with the reason "too dense" when forced; a progress bar row, always present, blank when static. The wallet export animates on the panel like any other payload. |
| Scanner | A square viewfinder with corner brackets, as wide as the pane allows, centred in the space above the actions, as a camera app's preview is. The square shows the camera's latest frame, in colour where the camera gives colour and in grey where it does not, scaled to cover the square and cropped to it, centred; the brackets, the state line and the parts bar are drawn over it, the line and the bar on a translucent band so they read over a light frame. Before the first frame, and where the shell has no camera, the square is the empty panel. The state ("Looking for a QR", "3 of 8") is drawn inside the square, at its bottom; a progress bar inside it for parts (`viewfinder`). | No torch control. Under the square, one band of equal tiles — the §4.1 idiom Home draws over its actions, a mark over a word — holding the ways in, in this order: "Read a file", "Paste", "Type". One band on every class, whatever it holds: three ways in cost the height of one tile rather than of three rows, which is what leaves a 268 dp panel a square worth aiming by, and the square takes what the band leaves. A scanner with two ways in (the seed scanner has no Paste; a flow with no keyboard has no Type) holds two equal tiles. "Read a file" asks the shell for a file; a shell that can list its files answers with the list and the core shows it as a Menu (Files); a shell with a picker of its own answers with the file. "Paste" takes the clipboard through the same router a scan uses, and a clipboard with nothing on it puts "Nothing to paste" on the state line for two seconds. "Type" opens the keyboard the flow behind the scanner has for the value. A way this shell cannot serve is a dimmed tile and nothing else: §4.11's dimming already says unavailable, and a tile has no second line to say it again. A payload that classifies as words in the clear, a seed code, an extended private key or a private key is refused on the way in from the clipboard, and refused under every calculator whatever it arrived on: a secret is never pasted (§4.10). The scanner is the way in to every flow that takes a value, so a shell with no camera shows "No camera" in the square and the band stays live. The expected type is in the title ("Scan a transaction"). A Settings row, "Camera rotation" (0°, 90°, 180°, 270°, default 0°), turns every frame clockwise before the preview and the decode, because a device someone built mounts its camera whichever way the case allows. |

### 4.10 Secrets

| Kind | Rendering | Behaviour |
|---|---|---|
| Secret panel | An orange-outlined panel (`secret_panel`) sized to its content at the class's type, never to the screen: the words panel (numbered rows: six per page at 24 dp rows on `small`; twelve at 40 dp rows and mono 20 on `mobile` and `wide`) and the value panel (one line box). A words panel takes two columns where the width allows them at the class's monospace size and one column where it does not, and it is left off only where even one column is taller than the space it sits in — which is what the 240 dp panels hit, and why word entry there shows the word being typed and nothing else. The Words screen pages a one-column list as it pages any other; the Entry and Pad panels keep one row per word of the secret in that column, so nothing moves as the entry goes on. A row wider than its column drops a monospace step rather than crossing the panel's edge. The geometry is fixed per class and per word count, identical masked and revealed. No caption inside it, in either state. Masked: bullets per item, same rows. | Hold anywhere on it to show; release masks. The eye in the app bar shows it for 30 s; while it does, the eye is drawn as a ring that empties clockwise, which is the only countdown. |
| Long secret | A reference row (bullets as the value while masked). | Opens the Secret screen: one panel as tall as the string at the largest mono size that fits the width and is not past the class's ceiling (§5 Transcribe: the top of the ramp on `small` and `wide`, 24 dp on `mobile`), centred in the space under the app bar. Masked, the panel holds one centred eye-slash icon and nothing else. |
| Mask policy | Whatever the entropy becomes is a secret: words, numbers, dice entries, checksum bits, seed, keys. They mask together. | Fingerprint, checksum validity and addresses are not secrets. A secret is never pasted and never copied: no screen with a secret panel carries a "Copy" row, and a payload the policy calls a secret is refused on the way in from the clipboard. |
| Copy | A "Copy" row action (§4.13), dimmed with "no clipboard" where the shell has answered that it has none. | On the Compare screen, on Wallet export, and on the Sign, Message and Encrypted backup results: addresses, extended public keys, descriptors, policies, transaction ids, signatures, signed transactions and encrypted backups, and nothing else. The screen states "Copied" or "No clipboard" on its caption line for two seconds. The Address screen carries no such row, because §4.5 keeps the whole address on the screen and a 268 dp panel has no room for both; the address is copied from the Compare screen its reference row opens. |

### 4.11 Records and results

| Kind | Rendering | Behaviour |
|---|---|---|
| Record | A table: label column left, value column right, on every class (`record`). A short value sits beside its label and may wrap to a second line. A long string is a reference row drawn flat inside the table: the label in the label column, the elided value on the line under it, a chevron at the right, no surface. One fact per row. The table, with its badge and warnings, is one block centred above the action. | Sign review, stats, tiers, About. Nothing inline; every row looks like a table row. |
| Result | Icon and a coloured title at the top of the body, then a record under it (`result`). | "Signed", "Yours", "Not found", "Passed", "Self-test failed". The colour is on the title. |
| Warning | A card: level icon, label, value (`warning_card`). "Change · not verified", "Fee · 12 % of the amount". Two lines, as a row with a value is: the label above, the value below it across the card's whole width, the icon beside them. Nothing is cut and nothing is squeezed into half the panel. | Ranked blocked, danger, caution, info. A danger needs its own acknowledgement before the hold. A block offers no acknowledgement: the review ends there and the hold never opens. |
| Dimmed reason | Two or three words under a dead row's label, on every class: a dead row with a reason is a two-line row, like a row with a value. | Only for a condition of this device or this state that the user could change: "no camera", "needs Tier B", "too dense". A row that is dead because the feature is not built yet is dimmed with its label and nothing else; dim already means unavailable. |

### 4.12 Text

| Kind | Rendering | Where |
|---|---|---|
| Title | Title token in the app bar. | Every screen. |
| Label | Label token, muted. Above its value in rows; left of it in a record. | Record labels, row labels, "Key", "Sign with". |
| Value | Body or mono token. | Everything a person reads or compares. |
| Reason | Caption, muted, one line. | Dimmed controls only. |
| Explainer | Body, a short paragraph, on a Learn page. | Never on a working screen and never behind a control on one. |

The app bar's info button is the only bridge from a working screen to an
explanation: it opens the Learn page the screen is about, and a screen
carries no prose, no caption under a row and no link of its own.

Prose that exists today and goes: the PIN hints, the passphrase warning
sentence, the checksum explanation, the quiz instructions, "Write these
on paper or steel", the wipe consequences, the nonce sentence, the
secret panel captions.

### 4.13 Actions

| Kind | Rendering | Behaviour |
|---|---|---|
| Primary | One accent button at the bottom (`primary`). Full width on `small` and `mobile`; on `wide` natural width, at least 160 and at most 320 dp, at the pane's bottom-right. | Confirms a choice or typed input; moves on. A primary dead for a condition the person can change carries §4.11's reason as a caption above it, and keeps its own place and size. |
| Secondary | Beside the primary. On `small` and `mobile` the two share the width equally; on `wide` both are natural width, secondary to the left of primary. Both buttons use one label font per class; a label that does not fit is shortened ("Numbers"), never shrunk. | The safe alternative ("Skip quiz" is never the accent). |
| Hold | An outlined button that fills while held (`hold`), placed as a primary. One secondary may sit beside it, and then the pair shares the row as a secondary and a primary do. | Sign, forget a key, wipe. The label is the verb: "Hold to sign". No sentence above it.; the hold that signs a message. The secondary is the step that declines the hold, and only a hold the app offered carries one: the Keep offer's "Not now". |
| Row action | A menu row. | Ways out of a result: "Show as QR", "Save to file". |

### 4.14 Progress and state

| Kind | Rendering |
|---|---|
| Progress | A bar with a count "2 of 5" above it. Entropy entry, scanning parts, animated codes. |
| Countdown | Only the eye's ring (§4.10). The status line has none. |
| Default | A step card whose value has a default is closed at entry, showing its value and a **Change** text button at the card's right in place of the chevron that opens it. The flow opens on the first card with no default; opening a closed card does not reset the cards after it. On a small panel the same: the paged flow starts on the first open card, and that page carries at its head one row per earlier card closed on its default, "{title} · {value}" with **Change**, which opens it. Tapping the page's title (the card's name alone, "Kind", with "Step 1 of 8" beside it in the muted caption type) closes the card and shows the list of steps, from which any card opens; no step indicator is drawn (2026-10-10; the step count moved off the title and beside it as its own muted text, 2026-10-10). Create a wallet and New key are the exception: each opens on its first card — Kind, Length — whatever opened it, even with a default already ticked; a card still closed on its default shows its value and **Change** the same way once Continue has passed it (`docs/NEW-WALLET.md` §2.1, 2026-10-10). |
| Empty state | A row that starts the flow: Keys with nothing loaded is "Load a key" and "Create a key"; Wallets with nothing registered is "Add a wallet" and "Scan a wallet"; the builder's Keys list with no key gathered is "Add a key". Never a sentence about emptiness. The bottom action — "Add a key", "Add a wallet" — is on the screen in every state. |
| Terminal state | Result layout with no action and no back chevron: "Session ended", "Self-test failed · vector 3". |
| Leaving | Back where nothing is behind — Home, the lock screen, the stored key's pad — shows "Back again to exit and clear memory" on that screen's caption line for two seconds; a second Back inside the window clears memory, keeps a key kept on the device, and asks the shell to exit, ending on the terminal "Session ended" titled "Exit" with "Keys · cleared from memory". "Wipe and exit" is the only exit that forgets the device's copy. On desktop, Escape is Back. |

### 4.15 Keys

A shell with a keyboard — a laptop booted from the stick, the desktop
shell — drives every screen with it, by the idiom a desktop already has
(§2.11). Nothing on any screen says so: a person who reaches for Tab or
an arrow finds it works, and a person who does not is never told about
it.

| Kind | Rendering | Behaviour |
|---|---|---|
| Focus | One item at a time, outlined by a 2 dp ring just inside its own rectangle at its own corner radius (`focus_ring`): the accent on any item, and the accent's own text colour on an item whose fill is the accent (a primary button, a checked chip, a toggle that is on), since an accent ring on an accent fill cannot be seen; the item's content is otherwise unchanged. The ring exists only after a key has moved focus, and a touch or a pointer press removes it, as a desktop hides its focus ring until the keyboard is used. Nothing has focus when a screen opens. | Everything a tap could act on can hold focus: menu and value rows, choice rows, tiles, toggles, pair buttons, reference rows, the pager's arrows, the eye, the actions, a hold. A keyboard's keys and a pad's digits never hold focus: they are typed. A dimmed control holds focus and does nothing on Enter, as it does nothing on a tap. |
| Tab, Shift+Tab | — | Focus moves to the next, or previous, focusable item in reading order (top to bottom, left to right, as the layout placed them), wrapping at the ends. An item inside a scrolling list is scrolled into view when it takes focus. |
| Up, Down | — | The same as Tab and Shift+Tab while something has focus. While nothing has focus, they scroll the screen's first scrolling region by one line, as they do today. |
| Left, Right | — | Focus moves within the same row — a pair, a band of tiles, the two actions, the pager's arrows, the sidebar's column on `wide` is a list and takes Up and Down — and does nothing when the focused item has no neighbour in that row. On a Words pager they turn the page, as the chevrons do. |
| Enter, Space | — | Act on the focused item as a tap does: open a row, check a choice, flip a toggle, press an action, turn a page. On a screen with an on-screen keyboard or pad and nothing focused, Enter is the keyboard's ✓, as it is today. A hold is held: the key going down starts the fill, the key coming up before it completes cancels it, and the hold completes under the key exactly as it does under a finger. A shell that cannot report a key coming up cannot hold, and the hold button then does nothing on Enter. |
| Escape | — | Back, as today. |
| Typing | — | Characters, Backspace and Enter go to the screen's on-screen keyboard or pad whatever has focus (§4.5), so a person types a word and then presses Tab to reach Continue. |

The focus ring is the only thing this adds to the screen, and it is
drawn by the widget the way its pressed state is. The rules are the
same on every screen because they are written once, over the layout's
hit targets, and no screen has keys of its own.

### 4.16 Structure chart

How a thing is made of others and where each of those is: Faraday's
wallet at a glance (`docs/NEW-WALLET.md` §6), the wallet over its keys
over its backup; and the same chart from an open vault (§7.2), the
backup over the keys over the wallet.

| Kind | Rendering | Behaviour |
|---|---|---|
| Node | The summary row's box: `RADIUS`, the page's fill, an inner edge, `CHART_NODE_PAD` inside. A title in the label type, an optional tag at its right in the caption type (secret, sealed, public, in their tones), then caption lines that wrap; a line marked lost or exposed reads "Key 1 words · lost" in `ERR`, struck through by a rule of its colour; a node that offers a step carries a `CHART_BUTTON` button at its foot. | The whole node is the target: it opens the node's sheet. A caption line that stands for one thing (a thing a backup node holds, a key's way in) is a target of its own over it: its sheet, or its way in. |
| Sheet | What a node or a line opens (`docs/NEW-WALLET.md` §9): a sheet `CHART_SHEET_WIDTH` wide, or the whole small panel as a page. Its head (icon and title), its facts as label-over-value rows, lines of what follows in the label type in their tones, a field being typed (its label in caption over a `CHART_SHEET_ROW` box with the caret; a passphrase as dots), then its actions as menu rows under optional caption headings, and its own primary button (Confirm, Keep, Check) beside Close (Back on a small panel) at its foot. A change asked for shows the plan's three check lines as facts, "before → after", a line made worse in `WARN`. | Each row leads into the flow that does it; opening the sheet does nothing else. A change is made only by its sheet's Confirm (`docs/NEW-WALLET.md` §9.7). |
| Row | Nodes side by side at equal widths, between `CHART_NODE_MIN` and `CHART_NODE_MAX`, `CHART_NODE_GAP` apart, centred. A row wider than its column wraps onto further lines at the widths a full line gives; the nodes of one line take its tallest node's height. | — |
| Line | A `CHART_STROKE` run (`CHART_STROKE_BOLD` for a key whose sheet's Where it is shows its lines, every other line then the inner-edge colour) from the middle of a node's bottom edge to its target's top edge, orthogonal: down into the band under the line of nodes, across, down. Where a line of nodes stands between, it runs down the gap between two nodes, or beside them, so it never crosses a node. Each band is `CHART_BAND` above and below its runs, each source's run across `CHART_LANE` from the next; a target's lines come in at points spread along its top. Each source has its colour (`KEY_LINES`, the theme's own colours in turn) and its dash pattern (`CHART_DASHES`, solid first), so the lines are told apart without colour too; the parent's lines are the border colour, solid. | Lines are not targets. |
| Column | On a small panel the rows stack, one node a line, full width, under the rows' headings, and no lines are drawn: a node names what the lines would have joined it to. | — |
| Direction | The rows read top down either way: the parent first (the wallet, its keys, its backup, as the wallet's card draws it), or the parts first (the backup, the keys, the wallet at the foot, as an open vault draws it). Lines always leave an upper node's bottom edge and enter a lower node's top edge. The plan's three checks go above the chart read either way, as a strip: three boxes side by side where each is at least `CHART_NODE_MIN` wide, else one under another, each its line in caption and its value at `TITLE` size in its colour (`OK`, the accent, `WARN`, `ERR`); the backup's map panel and the end of the backup draw the same strip (`docs/NEW-WALLET.md` §14.6). The column reverses its rows and headings the same way, the strip staying on top. | — |

## 5. Screen intents and the reusable screens

Every screen does one of seven things. The intent decides the standard
layout, so a person knows what a screen is for before reading it, and
the title's form says it too.

| Intent | What the user is here to do | Standard layout | Title | Screens |
|---|---|---|---|---|
| Navigate | go somewhere | a grid or list from the top; lists scroll | a noun ("Keys") | Hub, Menu, Addresses |
| Choose | decide one thing | the options as one group centred above Continue | a question ("How many words?") | Choice |
| Enter | type or roll | the entry group (field, candidates, keys or pad) at the bottom; its context (words so far, progress) centred above it | the step ("Word 7 of 12", "Set a PIN") | Entry, Pad |
| Understand | review facts, then act | one block (badge, table, warnings) centred above the action | the subject ("Review", "Output 1 of 2") | Record, Result, Hold |
| Transcribe | copy or compare carefully | one panel or string centred above the actions, at the largest type that fits: a comparison string steps up the mono sizes the tokens name until the next step would not fit the block's width and height in groups of four, and never past the class's ceiling — the top of the ramp on `small` and `wide`, 24 dp on `mobile`, where the string is read at arm's length beside body text and a words panel and a larger rung reads as a headline. One ladder and one ceiling for every long string, secret or public; the eye in the app bar for secrets | the thing ("Words", "Master key") | Words, Secret, Compare |
| Show | hold up to a camera | the code and its label centred above the controls | the thing ("Receive 2", "Transaction") | QR, Address |
| Read | read text | paragraphs from the top; scrolls | the subject | Document, Scanner is Show with the camera's own preview |

The Scanner is a Show screen turned around: the viewfinder is the
centred block, the rows the controls.

Every flow is built from these sixteen screens. A screen spec names one
of them.

| Screen | Content | Used by |
|---|---|---|
| Hub | status line, the wallet list, the band of three tiles, Add and Scan | Home |
| Menu | rows: menu, value, key, reference, secret, toggle; optional key context | a silent payments wallet's page (Address · Labels · Check a payment · Export · Keys · Forget, and no Sign, Addresses, Check an address or Recovery sheet row, because its outputs are ones a sender computed and it cannot send), "Labels" (one reference row per label handed out — "Label 1" above, its own `sp1q…` address below, dimmed with "not loaded" where the wallet's key is not — and "Add a label" as the bottom row, dimmed with "20 labels" once it has them all), "Check a payment" (the row that reads the transaction, and the row that reads the previous transaction an input still needs, with the transaction it is waiting for under it), Keys (one row per loaded key, in the order they were loaded: the fingerprint glyph, the fingerprint, and what the key is made of under it — "12 words", "24 words and a passphrase", "BIP-85 child" — and no network; "Add a key" as the bottom action), Wallets (one row per registered policy, with §4.4's key or eye glyph, the kind and the network as its label — "single-sig · mainnet", "multisig · regtest", "taproot multisig · regtest", "recovery · regtest", "miniscript · regtest", "MuSig2 · mainnet", "FROST · mainnet" — and what the wallet is called as its value: its name, or its shape — "73c5da0a · SegWit", "2 of 3 · SegWit", "2 of 3 · Taproot", "Recovery · SegWit", "Miniscript · SegWit", "FROST · 2 of 3", "MuSig2 · 2 keys · Taproot" — which is also the wallet page's title; the descriptor checksum is on the wallet's page and its export, never here; "Add a wallet" as the bottom action), a key's page (titled with its fingerprint: the "Key" row stating what it is made of — "12 words", "24 words and a passphrase", "BIP-85 child", "SLIP-39 shares", "SLIP-39 shares and a passphrase", "Codex32" — then "Account key", which exports one of the key's public accounts to a coordinator, then "BIP-85", which derives one of BIP-85's applications from the key, then "Open with passphrase", which opens this key's passphrase entry with no picker, on a key with words, then "Add a wallet", which starts Add a wallet's wizard with this key in hand, then "Vanity address", which grinds one of the key's own dials until its first address begins with the characters asked for, then Backup · Keep on this device on Tier B · Forget; no row of a wallet already in use), a wallet's page (the same shape for every kind, titled with what the wallet is called: the Name row where the wallet has a name, the actions, "Set a name" where it has none, then Keys, then "Kept on this device" as a toggle on a Tier B device that keeps keys — yes or no, whether this wallet is in the blob — then Forget — a wallet over one key this device holds, titled "73c5da0a · SegWit": Sign a transaction · Sign a message · Addresses · Check an address · Export · Recovery sheet · Keys · Forget; a policy this device holds a key of, titled "2 of 3 · SegWit": Sign a transaction · Addresses · Check an address · Export · Recovery sheet · Keys · Forget; a wallet it holds no key of, with the eye: Addresses · Check an address · Export · Keys · Forget; a FROST wallet, titled "FROST · 2 of 3 · Taproot": Sign a transaction, where this device holds a key of the group · Addresses · Check an address · Export · Keys · Forget), a wallet's Keys review (the Keys screen filtered to the wallet's members: one row per member with the wallet glyph where this device holds it and the eye where it does not, the dimmed reason "not loaded" on a member it has no key for, which opens Add a key and comes back here once one is loaded; a key opened there with a passphrase or as a BIP-85 child is compared with the fingerprint the row named and is not kept when it is another key, while a key loaded from words is loaded whatever it is), the transaction's Keys review (the same screen over the keys a transaction names rather than a wallet's members, opened from the Sign review's key context row, with the same glyphs, the same "not loaded" reason and the same return path; the transaction is inspected again when a key comes back, so the review it returns through continues with that key), Add a key (Load a key · Create a key · Create SLIP-39 shares · Create Codex32 shares: the ways a key is made, and nothing done with a key already loaded, which is a key's own page. Opened from a wallet's or a transaction's not-loaded row, where one key in particular is being asked for, it also carries Open with passphrase · Open BIP-85 child, which ask which loaded key to start from; making words, making shares and making codex32 strings are three rows and three flows, never one screen asking which), Add a wallet (New wallet · Scan a wallet, where New wallet is the one wizard that builds all six kinds), a wallet's Export (the Format value row — Descriptor · Policy, with the two account-key forms for a wallet over one key this device holds, "BSMS record" for a multisig wallet BIP 129 covers and "Bitcoin Core import" for every wallet with a descriptor — the string as a reference row, "Show QR" and Copy, with "Save to file" on the import file), the key picker "Which key?" (the loaded keys as key rows; a tap is the answer; asked only where a wallet or a transaction is waiting for one key in particular), Check an address (Scan a QR · Type an address), Tools (Key explorer, Word list, Dice passphrase, Verify a message, the calculators, Decode a transaction, Compare transactions, which reads two PSBTs and needs no key, Notes and Lightning node key), Notes (the note in hand, the notes kept on the device, each by its first line, and the recovery sheets kept here whose wallet is not in use, each by the wallet's name with "Recovery sheet" as its value, then "New note" and "Read an encrypted backup", which opens the scanner Load a key's row of that name opens, a plain note still reading through its "Read a file"; a device with none of them says "No notes"), Compare transactions while only the first has arrived (the first transaction's id as a value row and "Read the second transaction" under it, both opening the scanner), Backup menu (a key made of words: Words · SeedQR · CompactSeedQR · Encrypted backup · Draw a SeedQR · Numbers for steel · Split with Seed XOR · Codex32 · Verify backup; a key with no words: Encrypted backup, which seals its master seed, and Codex32, and, where the seed is one SLIP-39 takes, 16 or 32 bytes, SLIP-39 shares. Codex32 is on every key, because every key has a seed), Settings (with "Backup memory", whose value is the MiB every encrypted export is made at, and with "Duress PIN" and "Forget stored key" while a key is kept), About (with the secure hardware named), the key explorer's own menus, Files (one value row per file the shell listed, the date as the caption above and the name as the bold line below, since the name is what a person chooses by, newest first; tapping one is the answer), Faraday's Backups (`docs/SIMPLIFY.md` §5.1: every wallet known, once each, a card each with its name and shape and **Back up** where it is loaded, then one row per place of its backup map — where, what it holds, its tag secret, sealed or public, and what this device saw of it, in the state's tone — or "No backup plan"; a list from the top that scrolls, and on the small panel one wallet a page with Previous · Next), Faraday's wallet card (`docs/NEW-WALLET.md` §6: the name, shape and source, the backup line, the signatures possible here, then **At a glance**, §4.16's structure chart of the wallet, its keys and its backup with the plan's three checks as a strip above it, each node and each line of a backup node opening its sheet, then the first receive address and the card's actions along its foot; the body between the head and the actions scrolls; on the small panel an **At a glance** row opens the chart as its own page), Faraday's Vault contents (the kinds it holds, then **Add…**, the chosen kind's items, the chosen item; opened just unlocked on Wallets where it holds one. A wallet's item is `docs/NEW-WALLET.md` §7.2's chart read parts first — the plan's check lines, the backup this vault keeps the plan of, the keys, the wallet — then its descriptor and actions; a wallet with no plan in this vault shows its wallet and keys with "No backup plan in this vault". While a wallet's item is shown the kinds column is `CHART_PANE_KINDS` wide and the items `CHART_PANE_ITEMS`, giving the chart the rest. Each node opens its sheet (§4.16), whose actions load the wallet and its seeds in the vault first; on the small panel the item's page carries the column the same way up) |
| Choice | rows with a check, Continue | "How?" (which dial a vanity grind turns: Passphrase counter · Account index, each row's second line what one candidate costs — "a key derived each try", "an account derived each try" — with "slow here" at the trailing edge on a Tier A device, and the passphrase row dimmed with "has a passphrase" on a key this device cannot append to or "no words" on one with none), "From?" (where a Lightning node key comes from: An LND cipher seed · A loaded key, the second dimmed with "no key with words" where no loaded key has a BIP-39 seed), "Which procedure?" (how a run of dice becomes a key, after the language step of Create a key and of the SLIP-39 and codex32 plans: Hashed (Coldcard, SeedSigner), checked · Six as zero, hashed (Keystone) · Words chosen by the dice (BitBox), dimmed with "no words" where the key has none, each row's second line the rolls it takes at the chosen length), "Which form?" (over a silent payment address: Address · Payment URI, which is BIP-321's `bitcoin:?sp=…`), "Which form?" (how an export leaves the device, each row's second line the programs that open it: Encrypted backup, checked, "OpenSigner · decrypt.py" · KDBX 4, "a KeePass app", which has no QR · Plain text, "any text editor · not encrypted", offered for a note and a recovery sheet and for nothing that holds a key; no QR row, since the Result shows an encrypted backup as one code where it fits one; the step before the passphrase for anything encrypted), "Backup memory" (64 MiB · 256 MiB · 1 GiB, each row's second line the trade-off in a few words — "opens anywhere", "recommended" on the row this device recommends, "slowest to open"), "Which account?" (a key's six public accounts, the path under each name for the network the device is on: Legacy `m/44h/0h/0h` · Nested SegWit `m/49h/0h/0h` · SegWit `m/84h/0h/0h` · Taproot `m/86h/0h/0h` · Nested multisig `m/48h/0h/0h/1h` · SegWit multisig `m/48h/0h/0h/2h`), "Which application?" (the BIP-85 applications a key derives: Words, which opens the child-key flow, · WIF · Extended private key · Hex · Password (base64) · Password (base85)), "How many bytes?" (16 · 32 · 64, for the hex application), "Rescan from?" (where Bitcoin Core starts looking for the wallet: "The start · finds old coins, slow" · "Now · a new wallet"), source (Load's rows, most used first: Type the words · Scan a SeedQR · Read an encrypted backup · SLIP-39 shares · Seed XOR parts · Codex32 · Word numbers · Hex entropy, with Secure element dimmed; Codex32 has no count and no language step after it, and Word numbers and Hex entropy take the count and the language like typed words), word count (12, 15, 18, 21, 24 for a key's words; 20 and 33 under the same question, "How many words?", for a SLIP-39 share, which has no language step after it), language, script type, arrival, unit, network, timers, the word list's notation, the diceware list and its length; and Add a wallet's own steps — "Passphrase?" (No, checked, · Yes), the first step where a key's page started the wizard with that key in hand, on a key whose words this device holds, and on no other path: No goes on over that key, and Yes opens that key's passphrase entry, titled "New key with passphrase", whose ✓ puts the new key on Keys beside it and goes on over the new key; Back from it, and from the kind step after it, is the key's page, and the discard question is asked there only once keys beyond the one in hand are gathered — "What kind of wallet?" (Single-sig · Multisig · Taproot multisig · MuSig2 · FROST · Recovery · Silent payments, whose Keys step takes one key, so a tap on another row moves the check, and which has no script step; with a key in hand, Single-sig and Silent payments go on past the Keys step over that key and every other kind opens its Keys step with that key checked), "Keys · 2" (the loaded keys as key rows, several of which may be checked, a key the kind cannot use dimmed with its reason — a kind built on one key dims none of them and a tap on another row moves the check, a kind full of several keys dims the rows it cannot take with "full" — the cosigners scanned in under them and "Scan a key" last where the kind takes one, and, for a FROST group short of its count, the dimmed row that states what it needs), "Which script type?" and "How many must sign?", with FROST's "How many keys?" and "How many must sign?" before its keys, and a recovery wallet's "Now · 1" and "Later · 1" — the same key list, a key already on the other path dimmed with "on the other path" — its "How many must sign now?" and "How many must sign later?" where a path has more than one key, "After how long?" (30, 90, 180 and 365 days, each with the blocks it is under its name, a wait at or below the path before this one dimmed with "not after 30 days", and "Type a number of days" last) and "Another path later?" (No, checked, · Yes), which follows every recovery path's wait until the wallet has three of them; and "Keep this wallet on the device?" (No, checked, · Yes), which follows "Add this wallet" on a Tier B device that keeps keys when the wallet is one this device built over a passphrase key it holds; and a SLIP-39 backup's plan — "How many groups?" (1 to 16, 1 first), "How many groups must be present?" (2 to the count, and only where there is more than one group), then per group "How many shares?" and "How many must be present?", the group in front of the question where the backup has more than one ("Group 2 of 3 · How many shares?"), a group of one share having no threshold to choose — and "Random shares from?", which is Create a key's own source rows under ids of its own, asked by Backup and skipped by Create, where the source is the one the key was just made from; and a codex32 backup's own steps — "How long a seed?" (128, 160, 192, 224 and 256 bits, in place of the word count, since a codex32 key has no words), "Split into shares?" (No, one string, checked, · Yes), "How many shares?" (2 to 31) and "How many must be present?" (2 to 9, and never more than the shares), then "Random shares from?" again, asked by Backup and skipped by Create |
| Entry | field, one fact or mode row above it, candidates or presets, keyboard, error line | a vanity prefix ("Address prefix", on the address keyboard at the layer the script type decides, the fixed prefix its addresses all begin with already in the field and not deletable, every key the encoding cannot carry dimmed, the prefix whole above the field, and ✓ dead until one character past the fixed prefix is asked for), an LND cipher seed's passphrase (masked, after its twenty-four words; an empty field is LND's own default, so ✓ is live from the start), words (a SLIP-39 share's words on the same screen over the SLIP-39 list, titled "Share 1 · Word 7 of 20"), passphrase (opened from a wallet's not-loaded member row, its error line states which key the passphrase gave and which the wallet asked for, and the key is not kept), path, address (on the address keyboard: the bech32 layer, with `1` and `b` for the prefix, and a base58 layer behind shift, keys that cannot continue an address on the set network dimmed), a codex32 string (on the codex32 keyboard, `ms1` in the field and not deletable, the characters in groups of four with the string whole above the group, keys that cannot continue a string dimmed, ✓ live only once the whole string parses and the line under the field stating the codec's reason where it does not), the same field again as "Type it back · Share 2 of 5" over a string this device has just written, where a string that parses and is not the one shown is "Not the same string" and stays in the field, a BSMS session token (on the hex keyboard, with "None · 00" as the one preset row above the field, ✓ live only on `00` or a nonce of sixteen or thirty-two hex digits), a BSMS description (on the passphrase keyboard, the device's fingerprint already in the field, ✓ dead and the error line stating the limit past eighty characters), hex, a word by its number ("Word 7 of 12" on the digit pad, the digits in the field, the word the number names in the strip, the digits that cannot lead to a number on the list dimmed and no ✓), a key's entropy in hex ("Hex entropy" on the hex keyboard, the digits masked one by one, ✓ dead until the count's 32 to 64 digits are in), a wallet's name, a note (on the passphrase keyboard, up to 4 096 characters, ✓ dead while the field is empty), the word list's search |
| Pad | dots or count field directly above the pad, progress and masked entries for dice and coins, the words a run has completed on a masked panel above them where it builds words, pad | PIN (lock and stored key, set, repeat), dice, coins, the dice passphrase's rolls, "Days" (a recovery path's wait typed on the digits pad, the blocks it is under the field), "Length" (a BIP-85 password's length, the application's bounds under the field) and "Index" (which child of a BIP-85 application is derived, the largest index under the field). On a device that keeps a key the stored key's pad is the first screen, and the screen whenever no key is loaded; it has no back (§16.64) |
| Words | the words panel, pager when it pages (the title is "Words", the pager's label the page's name, "Words 1 to 6"), "Show numbers" + primary as an equal pair ("Numbers" only on `small`, where the full label does not fit half the width), eye | Backup › words, Create › words, Load › check your words, quiz › show the words, Explore › words, one share of a SLIP-39 backup this device is writing (the title is where it stands in the plan, "Group 1 · Share 2 of 3", or "Share 2 of 3" where the backup has one group), and each computed key of a FROST group Add a wallet has just dealt (the title is that member's own fingerprint, "a4dc80c1", which is what the wallet's Keys review shows) |
| Secret | the whole body as one panel, the facts it needs beside it, eye, one action where the screen ends a flow, with a second beside it where the flow can be gone on with without doing what the action asks | a vanity grind's counter ("Passphrase counter", what the find appends to the key's passphrase, masked because it is part of one), a Lightning node private key, with "Entropy" beside Done where it came from a cipher seed, which swaps the panel to the sixteen bytes and puts "Node private key" in its place, seed hex, master private key, SeedQR, CompactSeedQR, a rolled passphrase, a silent payments wallet's scan descriptor, `sp([73c5da0a/352h/0h/0h]spscan1q…)`, which the Export menu's row opens and which carries no code and no copy, a value BIP-85 derives (the title is the application and the index, "WIF · 0"; Done is the only action, because a secret is never copied), one codex32 string this device has just written (the title is "Codex32", or where it stands in the set, "Share 2 of 5"; the string is a comparison string in groups of four at the largest mono size that fits, 127 characters for a 512-bit seed; Continue types it back and Skip goes on without, behind the quiz's own caution) |
| Compare | label, one comparison string (or a descriptor as structure), Done | every reference row; the txid |
| Addresses | script type value row, Receive \| Change pair, then one reference row per address ("Receive 2" above, the elided address below). The script type is a value row where it is a choice and a §4.12 fact where it follows from what is listed, as it does for a policy | a single-sig wallet's addresses, a policy's addresses, the key explorer › addresses |
| Address | the QR at the class's side, the label under it and the whole address (comparison string) under that; the title is the address's name ("Receive 2") and the label under the QR is its script type ("SegWit"); indices are the derivation index, from 0. Where the address has more than one form, a §4.2 value row above the copy row names the form and opens the Choice for it | tap on an Addresses row; Verify result's address; a silent payments wallet's address and each of its labels (the title is "Address" or "Label 1", the label under the code "Silent payment", and the Format row switches between the address and its BIP-321 URI) |
| QR | QR, label, Animated toggle row, progress row, the "Save as PNG" row on a code that is not a secret (on `small` only where the screen has no Animated toggle, which with the code and the progress row takes the whole frame), and an action row where a flow carries on past the code | a key's export, a wallet's export, transaction, SeedQR (as a Secret); a message signature as one text code; "Group record", the last step of Add a wallet's FROST flow, whose label is the wallet's fingerprint and whose actions are "Save to file" and Continue, the wallet being in use once it is left; an encrypted backup's and a sealed file's code (ciphertext). "Save as PNG" saves the one static code, black modules on white with the four-module quiet zone, eight pixels to a module, and is dimmed with "too dense" where the content is too long for one code; once the shell answers, a line under the label says "Saved as opensigner-qr.png" or "Not saved". The Address screen carries the same row under the address, except on `small`, where the whole address takes that room (§4.5). SeedQR, CompactSeedQR and the grid are secrets and have none |
| Record | optional key context, optional network badge, the table with reference rows for strings, optional warning cards, a pager where the record is one of a run, one action | "Grinding" (a vanity grind as it runs: "Tried", the candidates so far, "Rate", the candidates a second this device is measured at, and "Expected", how long the prefix takes at that rate, with Stop as the action — §4.14's bar is not drawn, because a grind counts towards no total and the expected figure is a mean rather than an end), Sign overview, output, inputs (one row per input, never a free block under the table, and the Signatures row where the transaction carries any), Signatures (one row per signature the transaction carries, whoever made it: the input and the key as the label, "valid", "invalid" or "not checked · <reason>" as the value, with "· deterministic · Low R" or "· not deterministic" after it for a key this device holds; the row opens the signature whole on Compare; reached from the inputs review, from Decode's and from the Sign result), stats, tiers, one word of a BIP-39 list, a wallet's keys — for a FROST wallet read from a record, one row per member with the glyph, "Key 1 of 3" as the label and that member's own fingerprint as the value; and Add a wallet's review of a FROST group before it is dealt, which states the kind, the network and the chosen keys, there being no descriptor until the deal has run. A taproot wallet whose internal key is BIP 341's NUMS point carries "Key path · unspendable". A wallet with a timelocked path carries "Kind · Recovery", "Now · 1 of 1" and one row per recovery path, "After 365 days · 1 of 1 · 52 560 blocks", whether it was built here or arrived as a descriptor. A wallet read from a BIP 129 descriptor record carries, beside its quorum, "Paths · /0/*,/1/*", "First address" and "This device · Key A" |
| Result | icon and coloured title, a record, one or two actions | "Address found" (a vanity grind's find: the address as a reference row, then the account it is at for the account dial or the candidates it took for the passphrase dial, "Use it" — which opens the passphrase key exactly as "Open with passphrase" does, or adds the single-sig wallet at that account — and, on the passphrase dial, "Show it" beside it; "No address found" where the counter ran out first, and leaving either discards the find), "Cipher seed read" / "Node key derived" (a Lightning node key: the node public key as a reference row, then "Cipher seed version", "Birthday" — the day and the count of days the seed holds, "2018-03-22 · 3365 days" — or the key it came from, then "Derived as · LND" or "Derived as · ldk-node", with "Show secret" beside Done; a cipher seed that does not open is the same screen with "Wrong passphrase", "The words do not check out" or "Not a version 0 cipher seed" and no rows), "Check a payment" ("Paid", with one row per output that pays this wallet — "Output 0" above, the amount and the label or "Address" beside it — or "Not paid here", or what stopped the check: no input a key can be read from, a transaction that is not signed yet, a wallet whose key is not loaded), Verify result, Sign result, "Compare transactions" ("Same transaction" or "Different", with one row per field that differs — `Input 0 · Sequence`, `Output 1 · Amount`, `Input 0 · Signing` — the two values where they fit beside the label and "differs" where they do not), "Nonce shared" (a MuSig2 pass that ended in round 1), quiz result, checksum, "Share accepted" (a SLIP-39 share that belongs with the set: "Groups · 1 of 2 needed", then one row per group seen, "Group 2 · 1 of 3 shares", and Continue, which asks the next share or goes on once the set is enough) and "Share refused" (the codec's own reason as the result line), the same two after a codex32 share ("Shares · 1 of 3 needed"), "Shares made" (a SLIP-39 backup this device has just written: "Groups · 2 of 3", one row per group "Group 1 · 2 of 3 shares", then "Random shares · <source>" with the Trust row for This device), "Strings made" (a codex32 backup this device has just written: "Split · 3 of 5" or "Split · none", "Random shares · <source>" with the same Trust row where the seed was split, and, for a key with words, "Holds · the seed, not the words"), "Encrypted backup" (a key's: Key, Inside, Bytes, Memory, then "Format" "OSKB 3" or "KDBX 4", "Cipher" "XChaCha20-Poly1305" or "ChaCha20", "Key derivation" "Argon2id · 3 passes · 1 lane" as the file's header states it, "Read with" the form's programs, then the File row once saved and Copy; "Save to file" or Done, and "Show as QR" except on KDBX) and "Sealed" (a note's or a sheet's: Bytes, Memory and the same four rows; "Save to file" or Done, and "Show as QR" on an encrypted backup that fits one code; a plain text file states its Bytes alone), scan results, wipe done |
| Hold | a record of what will happen, "Sign with" chips where several keys could sign, the hold, and one plain action beside it where a step declines the hold | Sign confirm, Forget, Wipe, Keep ("Not now" beside the hold) |
| Scanner | square viewfinder with the state inside, and one band of tiles under it — "Read a file", "Paste", "Type" | Scan, and every flow that takes a value |
| Document | scrolling text with headings; may end in one Menu action row, carry one bottom action, or both | tiers, About, Learn (a page whose subject has a flow ends in its "Try it" row; a page opened from a working screen's info button draws no such row and opens at the section that screen is about), Start here (Continue at the bottom), a message to sign or verify (the message whole, the address and the format as rows above it), a note (the text whole, how many characters it is as the one row above it, "Keep on this device" as a toggle under that on a Tier B device that keeps keys — dead with "8 kept" where the record is full — Export and Forget under it; it is drawn as text and not as a secret panel, because a note is what a person reads), a wallet's recovery sheet (the descriptor as a reference row, the wallet's name, the row that writes the note, the same "Keep on this device" toggle, "Read an encrypted backup", which opens the same scanner as Notes' row and lands a sealed sheet on the document below, and the note under them; Export), a recovery sheet that arrived encrypted (the same, with "Add this wallet" where its descriptor is one this device reads) |

Sixteen screens, placed by the law in §2.6:

| Screen | Centred block | Bottom (touch) | Starts at the top |
|---|---|---|---|
| Hub | — | the tile band, Add and Scan | the wallet rows |
| Menu | — | — | the rows |
| Choice | the option rows | Continue | (the rows, when taller than the space) |
| Entry | the "Words so far" panel (`mobile`, `wide`), or the one fact or mode row above the field | field, candidates, keyboard | — |
| Pad | the words a run has completed (`mobile`, `wide`) | entries and progress, field, pad, Continue | — |
| Words | the words panel and its pager | Numbers · Continue | — |
| Secret | the panel and its facts | the action, where there is one | — |
| Compare | the label and the string | Copy, where the string is public; Done | — |
| Addresses | — | — | the value row, the pair, the rows |
| Address | QR, label, the string | the Format and Copy rows where it has them, Save as PNG above `small` | — |
| QR | QR and label | Animated toggle, progress, Save as PNG | — |
| Record | badge, table, warnings | the pager and the action | — |
| Result | title, table | the actions | — |
| Hold | the table | Sign with, the hold with its secondary where it has one | — |
| Scanner | the viewfinder | the band of ways in: Read a file, Paste, Type | — |
| Document | — | the one action, where there is one | the text, and its one row after it |

On `wide` every screen except Hub is drawn in the sidebar frame inside
the width caps, centred in the pane, with the same placement and the
primary action at the pane's bottom-right. The Pad group is
bottom-anchored on every class; the PIN pad is one rectangle on the lock
screen and both set-PIN steps, and a dice or coin pad is its own, with
its entries and progress above it. A centred block taller than its
space scrolls from the top.

A Menu with few rows on `mobile` keeps its start-the-flow buttons at the
bottom, where the thumb is, and leaves the space between empty. A row
whose value is a neutral state ("verified", "no") keeps the value in the
body tone; only the label is muted, so the row does not read as dead.

## 6. What changes from today

- No segmented rows except the two-option pair.
- Choices are checked then confirmed; settings apply on tap.
- Rows with values are label above, value below.
- "Using" becomes "Key"; the row leaves Entry screens.
- Long strings are reference rows everywhere except Compare and Address.
- Addresses is a list; Address is one address with its QR; the
  Address \| Code pager goes.
- "Code" becomes "QR"; "Verdict" becomes "Result"; "Satoshi" becomes
  "sats"; "Chain" becomes the Receive \| Change pair; "Kind ·
  Recipient" goes in favour of a "MINE" badge.
- Secret panels lose their captions; the eye carries the countdown;
  the status line loses its countdown.
- Dice and coin entries are masked and grouped in fives.
- The scanner is square, with its state inside, and has no torch row.
- QR sides and `wide` widths are capped by tokens; `wide` actions are
  natural width at the bottom-right; `mobile` content groups sit above
  the action.
- Six words per page on `small`; "Show numbers" and Continue share the
  width equally on touch classes.
- Amounts get a unit setting; Sign screens are records on every class.
- One Pad screen; one Words screen; prose removed from working screens.

## 7. Open questions for the owner

Settled on 2026-09-09 from the re-review: the 0.5 mm pitch is a hard
floor and the part count follows from it (§4.9); the Sign review on
`small` leaves off the key row (§3); the `wide` column is centred in the
pane (§3); the PIN pad is not shuffled by default (§4.3); a row that is
dead only because the feature is not built carries no reason (§4.11);
the gallery
follows the app where they disagreed, except the Words pager label and
the tier badge, which follow this document.

Still open:

1. Sign output's address as a reference row (§4.5) adds a tap to job F1.
   Keep the rule, or let the Record screen show one comparison string
   when it is the only long value on it?
2. Direct selection with a bottom-anchored list on `mobile` is the other
   way to fix thumb reach on Choice screens, without the extra Continue
   tap. v0.3 chose check + Continue for consistency with settings lists.
3. `wide`: whether short secrets may show inline in more places, since
   the pane has the room.

## 8. How this gets built

1. Gallery pass 3 applies v0.3 to the components and screens; the owner
   reviews the renders.
2. One more gallery pass if needed.
3. Screens rebuilt area by area from the sixteen, each with a three-line
   spec: which screen, which content, what is left off. (Done, five
   passes.)
4. A whole-app re-review of the rebuilt screens against this document
   at the four renders, then the fix passes from its findings, which is
   where v0.6 came from.
5. UX.md §5 and §6, the composition specs and PLANNING §13's screen
   rules are folded into this file. (Done, 2026-09-09.)
