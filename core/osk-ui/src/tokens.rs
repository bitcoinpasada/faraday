//! Design tokens (`docs/DESIGN.md` §3): every dimension, size, ratio and
//! timing the design system names, in one place.
//!
//! Nothing here is a screen decision. A screen, an organism, a component
//! and a gallery page all read these names, so changing a number changes
//! it everywhere and a review can be settled by editing one file. Lengths
//! are in dp; only the type ramp is scaled by the size class
//! ([`type_scale_pct`]).
//!
//! Per-class values are `const fn name(class) -> …` rather than three
//! constants, so a caller asks for the value it needs and the table of
//! classes stays next to the value it varies.
//!
//! `tools/lint-tokens.sh` fails the build when `organisms.rs`,
//! `gallery.rs` or a file under `components/` writes a layout number of
//! its own instead of naming one here.

use crate::geom::SizeClass;

// ---------------------------------------------------------------------
// Type ramp
// ---------------------------------------------------------------------

/// Title size: app bar titles and headings.
pub const TITLE: f32 = 20.0;
/// Body text size.
pub const BODY: f32 = 16.0;
/// Label size: row labels and secondary lines.
pub const LABEL: f32 = 14.0;
/// Caption size: notes, hints and badges.
pub const CAPTION: f32 = 12.0;
/// Monospace size for chunked strings, fingerprints and words.
pub const MONO: f32 = 16.0;
/// Monospace size of a words panel on a phone or a desktop window
/// (`docs/DESIGN.md` §4.10: "twelve at 40 dp rows and mono 20 on
/// `mobile` and `wide`"). A word written down is copied character by
/// character, so it is read a step larger than a string that is only
/// identified.
pub const MONO_LARGE: f32 = 20.0;

/// Monospace size in dp of a words panel that also carries the wordlist
/// numbers. Four more characters a row, in two columns, is more than
/// [`MONO_LARGE`] fits across a phone; this is the largest size at which
/// the widest row — an eight-letter word and a four-digit place — still
/// fits the panel on every class that draws two columns.
pub const MONO_INDEX: f32 = 15.0;
/// The size a dense row of chips drops to before it wraps.
pub const CHIP_TIGHT: f32 = 13.0;

/// Largest monospace size in dp a comparison string is drawn at on
/// `class` (`docs/DESIGN.md` §5 Transcribe: "one panel or string centred
/// above the actions, at the largest type that fits: a comparison string
/// steps up the mono sizes the tokens name until the next step would not
/// fit the block's width and height in groups of four, and never past
/// the class's ceiling").
///
/// The ramp is [`MONO`] and every second dp above it — 16, 18, 20 … 32 —
/// so the step is the same one the planner already walks downwards.
/// Thirty-two is the top on `small` and `wide`: a group of four at 32 dp
/// is 77 dp wide, and four groups plus their spaces are 320 dp, the
/// widest a line can be inside the 480 dp column cap of §3. A larger
/// step would put two groups on a line and turn a string a person is
/// comparing into a column.
///
/// On `mobile` the block is tall enough that nothing binds and every
/// long string would reach that top, so the ceiling is the one the eye
/// sets rather than the box: 24 dp. The string is read at arm's length
/// beside body text at [`BODY`] and a words panel at [`MONO_LARGE`], and
/// a rung above 24 reads as a headline.
pub const fn comparison_max(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small | SizeClass::Wide => 32.0,
        SizeClass::Mobile => 24.0,
    }
}

/// A line box as a multiple of the type size (`docs/DESIGN.md` §3: "a
/// line box is about 1.43 × the size"). Used wherever a fixed height in
/// dp has to hold a line of text before a sized face exists to measure it.
pub const LINE_BOX: f32 = 1.43;

/// The height in dp of one line box at `size_dp`.
pub const fn line_box(size_dp: f32) -> f32 {
    size_dp * LINE_BOX
}

/// The height in dp of one line of `size_dp` text at the class's type
/// scale.
///
/// A screen that reserves a line before it has anything to put in it, or
/// that measures a block against the zone it sits in (`docs/DESIGN.md`
/// §2.6), has to count what the class will actually draw, not the
/// unscaled ramp.
pub const fn scaled_line(size_dp: f32, class: SizeClass) -> f32 {
    line_box(size_dp) * type_scale_pct(class) as f32 / 100.0
}

/// The height in dp of one caption line at the class's type scale.
///
/// A screen that reserves a line before it has anything to put in it —
/// the inline error of §4.3 — has to reserve what the class will
/// actually draw, or the message is cut where the ramp is largest.
pub const fn caption_line(class: SizeClass) -> f32 {
    scaled_line(CAPTION, class)
}

/// The type ramp's scale for a size class, in percent
/// (`docs/DESIGN.md` §3).
///
/// One ramp, three densities. `Small` is the size the panel was designed
/// at; a phone reads at the same distance from a screen twice as tall,
/// and a desktop window is read from twice as far away as a phone at the
/// same dp size. Only type scales: touch targets, padding and the 8 dp
/// grid are the same everywhere. [`crate::geom::Scale::for_class`]
/// applies it.
pub const fn type_scale_pct(class: SizeClass) -> u16 {
    match class {
        SizeClass::Small => 100,
        SizeClass::Mobile => 110,
        SizeClass::Wide => 130,
    }
}

/// Whether `class` sets its text in Noto Sans SemiCondensed rather than
/// Noto Sans. A menu row on the 268 dp panel leaves its label 144 dp
/// between the icon and the chevron, and a label such as "Create Codex32
/// shares" is wider than that in the ordinary width; the narrow cut is
/// the same design with the same vertical metrics, so only widths
/// change.
pub const fn narrow_text(class: SizeClass) -> bool {
    matches!(class, SizeClass::Small)
}

// ---------------------------------------------------------------------
// Spacing
// ---------------------------------------------------------------------

/// The spacing grid: every gap is a multiple of this.
pub const GRID: f32 = 8.0;
/// The ordinary gap between two things that belong together.
pub const GAP: f32 = GRID;
/// The gap a dense list uses, where a full grid step would cost a row.
pub const GAP_SMALL: f32 = 4.0;
/// Screen padding: the margin between the content and the screen edge.
pub const PAD: f32 = 16.0;
/// Corner radius of cards, rows, tiles and buttons.
pub const RADIUS: f32 = 12.0;
/// Corner radius of the small rectangles: chips, candidate cells, keys.
pub const RADIUS_SMALL: f32 = 6.0;
/// Width of a scrollbar track, in dp.
pub const SCROLLBAR: f32 = 3.0;
/// Shortest scrollbar thumb, in dp.
pub const SCROLLBAR_MIN_THUMB: f32 = 24.0;

// ---------------------------------------------------------------------
// Chrome
// ---------------------------------------------------------------------

/// App bar height.
pub const APP_BAR: f32 = 56.0;
/// Home's status line: the same height as an app bar, so the content
/// under it starts at the same place on every screen.
pub const STATUS_LINE: f32 = 56.0;
/// Height of the bottom-anchored primary action.
pub const CTA: f32 = 52.0;
/// Space under the bottom action, between it and the screen edge,
/// before the shell's own bottom inset is added.
pub const ACTION_BOTTOM: f32 = PAD;

/// Space under the bottom action: the screen padding plus `inset_bottom`,
/// the unusable strip the shell reported at the bottom edge, so the
/// action stands clear of a gesture bar where there is one and sits at
/// the screen's own margin where there is not.
pub fn action_bottom(inset_bottom: f32) -> f32 {
    ACTION_BOTTOM + inset_bottom
}
/// Minimum touch target.
pub const TOUCH: f32 = 48.0;
/// Icon size inside a menu row or a tile.
pub const ICON: f32 = 28.0;
/// Icon size inside a list row or an app bar.
pub const ICON_SMALL: f32 = 20.0;
/// Icon size of a status screen's one large mark.
pub const ICON_LARGE: f32 = 48.0;

// ---------------------------------------------------------------------
// Rows
// ---------------------------------------------------------------------

/// Standard list row height.
pub const ROW: f32 = 56.0;
/// Primary menu row height: an icon, a label and a chevron.
pub const MENU_ROW: f32 = 72.0;
/// Menu row height on a `small` panel, where five rows and the app bar
/// have to share 358 dp.
pub const MENU_ROW_SMALL: f32 = 52.0;

/// Menu row height in dp for the size class (`docs/DESIGN.md` §3: row 52
/// on `small` and 72 elsewhere).
pub const fn menu_row(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => MENU_ROW_SMALL,
        _ => MENU_ROW,
    }
}

/// Gap in dp between two rows of a menu or a choice list.
pub const fn menu_row_gap(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => GAP_SMALL,
        _ => GAP,
    }
}

/// A choice row (`docs/DESIGN.md` §4.2): the same height as a menu row,
/// because a choice is a menu whose rows answer a question.
pub const fn choice_row(class: SizeClass) -> f32 {
    menu_row(class)
}

/// Height in dp of a row that carries a value under its label
/// (`docs/DESIGN.md` §4.1: "label above, value below, everywhere a row
/// carries a value").
///
/// One height for every such row — menu row with a value, value row,
/// path row, script-type row, key row, reference row, secret row — so a
/// list of mixed kinds reads as one list.
pub const fn stacked_row(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => ROW,
        _ => MENU_ROW,
    }
}

/// Height in dp of a record row drawn on one line: the label and its
/// value side by side at the label token, with the dense step above and
/// below (`docs/DESIGN.md` §4.3).
///
/// The row an entry screen falls back to when the space above its field
/// holds one line and not two, with the keyboard already at
/// [`KEY_MIN_HEIGHT`].
pub const fn one_line_row(class: SizeClass) -> f32 {
    scaled_line(LABEL, class) + GAP_SMALL
}

/// The gap in dp between the two lines of a two-line row: the label
/// above and the value under it (`docs/DESIGN.md` §4.1). Tighter than a
/// grid step, because the two lines are one fact and read as a pair.
pub const STACKED_LINE_GAP: f32 = 2.0;

/// A value row (`docs/DESIGN.md` §4.2 "Value row"): the label above, the
/// value below, a chevron at the right. The two-line row's height.
pub const fn value_row(class: SizeClass) -> f32 {
    stacked_row(class)
}

/// Height in dp of one preset row of the path editor (§4.6): a script
/// type over the path that produces it.
///
/// §2.6 gives the editor's block the space between the app bar and the
/// field, and the block fits it on `wide` rather than scrolling. That
/// space is 375 dp once the keyboard is at its own least height. The
/// block that fits it is 368 dp: a 26 dp group label, the dense gap, four
/// rows of 60 with three 4 dp gaps (252), the dense gap, the second 26 dp
/// label, the dense gap and the 52 dp Receive | Change pair. The 60 comes
/// from the row's own two lines under [`Widget::tight`], since this
/// height is below them; at [`MENU_ROW`] and the grid gap the same block
/// is 440 dp and the pair falls under the field. `mobile` has 1135 dp
/// for it and keeps the full row.
///
/// [`Widget::tight`]: crate::widgets::Widget::tight
pub const fn preset_row(class: SizeClass) -> f32 {
    match class {
        SizeClass::Mobile => stacked_row(SizeClass::Mobile),
        _ => stacked_row(SizeClass::Small),
    }
}

/// Gap in dp between two preset rows, dense on every class but `mobile`
/// for the reason [`preset_row`] gives.
pub const fn preset_row_gap(class: SizeClass) -> f32 {
    match class {
        SizeClass::Mobile => menu_row_gap(SizeClass::Mobile),
        _ => menu_row_gap(SizeClass::Small),
    }
}

/// Height in dp of a pager row.
///
/// The row is pressed with a pen in the other hand while a word page is
/// copied onto steel, so it meets the class's touch floor
/// ([`touch_floor`]) like every other target. The Words screen on
/// `small` still fits it: 56 app bar + a six-row panel (6 × 24 dp rows
/// and 2 × 8 dp of padding = 160) + 8 gap + 44 pager + 8 gap + 52 action
/// + 16 under it = 344 of 358 dp.
pub const fn pager_row(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => touch_floor(class),
        _ => TOUCH,
    }
}

/// Least height in dp of a chip or a candidate cell: a finger on a panel
/// or a phone, a pointer on a desktop window (`docs/DESIGN.md` §3).
pub const fn touch_floor(class: SizeClass) -> f32 {
    match class {
        SizeClass::Wide => 32.0,
        _ => 44.0,
    }
}

// ---------------------------------------------------------------------
// Entry
// ---------------------------------------------------------------------

/// Height of a one-row text field above a keyboard (`docs/DESIGN.md`
/// §4.3: "52 dp, single line — 40 dp on `small`, a letter key's own
/// height").
///
/// On the panel the field is as tall as a letter key, which is what
/// leaves the entry group room for two rows of candidates over three
/// rows of keys (`docs/PLANNING.md` §16.118 rule 4).
pub const fn field(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => key_height(SizeClass::Small),
        _ => FIELD,
    }
}

/// The field's height off the panel.
pub const FIELD: f32 = 52.0;

/// Letter-key height in dp for the size class (`docs/DESIGN.md` §4.3:
/// "Keys 40 dp on `small`, 56 dp on `mobile`").
pub const fn key_height(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => 40.0,
        _ => 56.0,
    }
}

/// Smallest acceptable key height in dp.
pub const KEY_MIN_HEIGHT: f32 = 36.0;
/// Key height in dp for a keyboard laid out as a grid of five rows — the
/// 五十音 — on a 240 × 320 dp panel, which has the height for five rows
/// only at a shorter key than a three-row keyboard's.
pub const KEY_HEIGHT_GRID: f32 = 28.0;
/// Smallest that grid's key may be squeezed to.
pub const KEY_MIN_HEIGHT_GRID: f32 = 24.0;
/// Largest letter-key height in dp. A taller key is not easier to hit
/// and costs the screen a row of content.
pub const KEY_MAX_HEIGHT: f32 = 56.0;
/// Share of the height a screen offers that one letter-key row takes.
pub const KEY_HEIGHT_FRACTION: f32 = 0.09;
/// Gap between key cells in dp. Zero: keys tile the keyboard area with
/// no dead zones and each paints a face inset from its cell.
pub const KEY_GAP: f32 = 0.0;
/// Inset in dp from a key's cell to its visible face, left and right.
pub const KEY_FACE_INSET: f32 = 3.0;
/// Inset in dp from a key's cell to its visible face, top and bottom.
pub const KEY_FACE_INSET_Y: f32 = 2.0;
/// Inset in dp under the bottom row's faces, so the visible keys stand
/// clear of the screen edge while their hit rectangles reach it.
pub const KEY_BOTTOM_INSET: f32 = 16.0;
/// Height in dp the keyboard region takes on top of its rows, so the
/// bottom row keeps a full-height face and still reaches the edge,
/// before the shell's own bottom inset is added.
pub const KEY_BOTTOM_RESERVE: f32 = KEY_BOTTOM_INSET - KEY_FACE_INSET_Y;

/// The bottom row's face inset: the plain inset plus `inset_bottom`, the
/// strip the shell reported at the bottom edge, so the keys people see
/// stand clear of a gesture bar while their hit rectangles still reach
/// the edge.
pub fn key_bottom_inset(inset_bottom: f32) -> f32 {
    KEY_BOTTOM_INSET + inset_bottom
}

/// The keyboard region's reserve ([`KEY_BOTTOM_RESERVE`] plus the
/// shell's bottom inset).
pub fn key_bottom_reserve(inset_bottom: f32) -> f32 {
    key_bottom_inset(inset_bottom) - KEY_FACE_INSET_Y
}

/// Largest width in dp a PIN, dice or coin pad takes. Wider than this
/// the cells letterbox, so the pad keeps this width and centres
/// (`docs/DESIGN.md` §4.3: "280 dp wide at most").
pub const PAD_MAX_WIDTH: f32 = 280.0;
/// Largest key height in dp for a pad, so a tall window does not turn
/// the pad into a column of slabs.
pub const PAD_MAX_KEY_HEIGHT: f32 = 72.0;
/// Widest a PIN cell may be against its height. Squarer than this reads
/// as a keypad; wider reads as a letterbox.
pub const PAD_KEY_ASPECT: f32 = 1.4;

/// Key height in dp of the PIN pad on a `small` panel: ten digits and
/// two edge keys, larger than a letter key because the pad is the whole
/// lower half of the screen.
pub const PIN_KEY_HEIGHT_SMALL: f32 = 44.0;
/// Key height in dp of the dice pad on a `small` panel: six large faces
/// (`docs/DESIGN.md` §4.3).
pub const DICE_KEY_HEIGHT_SMALL: f32 = 72.0;
/// Key height in dp of the coin and binary pads on a `small` panel: two
/// large sides, which are the screen's whole lower half.
pub const COIN_KEY_HEIGHT_SMALL: f32 = 112.0;
/// Key height in dp of the PIN and dice pads on the taller classes.
pub const PAD_KEY_HEIGHT: f32 = 56.0;
/// Key height in dp of the coin and binary pads on the taller classes.
pub const COIN_KEY_HEIGHT: f32 = 96.0;

/// Columns the PIN pad divides its width into.
pub const PIN_PAD_COLUMNS: f32 = 3.0;
/// Columns every other pad divides its width into: the dice pad's three
/// faces and the backspace beside them.
pub const PAD_COLUMNS: f32 = 4.0;

/// Width in key units of an edge key that takes a key and a half:
/// shift, backspace and ✓ at the ends of a letter row (`docs/DESIGN.md`
/// §4.3).
pub const KEY_UNITS_EDGE: f32 = 1.5;
/// Width in key units of ✓ where it closes a row of its own: the card
/// pad's row of suits.
pub const KEY_UNITS_DONE: f32 = 3.0;
/// Width in key units of the passphrase keyboard's space bar.
pub const KEY_UNITS_SPACE: f32 = 6.0;

/// Keycap size in dp on the PIN and dice pads, whose keys are few and
/// large.
pub const KEYCAP_PAD: f32 = 24.0;
/// Keycap size in dp on the hex, path and coin keyboards, and on a
/// letter keyboard on `mobile` and `wide`.
pub const KEYCAP_LETTER: f32 = 18.0;
/// Keycap size in dp where the keys are dense: a letter keyboard on
/// `small`, and the kana and jamo grids, whose characters are about an
/// em wide in ten columns.
pub const KEYCAP_DENSE: f32 = 16.0;

/// Columns of the kana grid: the ten consonant groups of the 五十音
/// (`docs/DESIGN.md` §4.3).
pub const KANA_COLUMNS: usize = 10;

/// Cells in one row of candidates on a list whose words are one
/// character — the two Chinese lists, looked up by a Mandarin reading.
///
/// A character needs no more width than a key, and the keyboard under
/// the strip is ten keys wide, so a cell at the keyboard's own key pitch
/// is a target the design already accepts: 24 px, 4.3 mm, on the 240 px
/// panel and 6.5 mm on a phone. Two rows of ten hold the largest tone
/// group there is (`yi4`, 20 characters, `docs/PLANNING.md` §16.44), so
/// nobody pages.
pub const CANDIDATE_CELLS_HANZI: usize = 10;

/// Rows of candidates for such a list, on every class: two, which is
/// [`CANDIDATE_CELLS_HANZI`] short of a whole tone group.
pub const CANDIDATE_ROWS_HANZI: usize = 2;

/// Height in dp of one candidate cell.
///
/// A one-character cell is shorter on the short panels: two rows of
/// [`touch_floor`] do not share a 358 dp panel with the app bar, the
/// field, the error line and a four-row pinyin or 注音 keyboard, and
/// §4.3 gives the keys their height first. The cell is still as tall as
/// a key there.
pub const fn candidate_cell(class: SizeClass, one_char: bool) -> f32 {
    match (one_char, class) {
        (true, SizeClass::Small) => CANDIDATE_CELL_HANZI_SMALL,
        (false, SizeClass::Small) => CANDIDATE_CELL_SMALL,
        _ => touch_floor(class),
    }
}

/// Height in dp of a spelled list's candidate cell on `small`: the
/// floor of the key range, so that two rows of candidates and three
/// rows of keys share the 358 dp panel (`docs/PLANNING.md` §16.118).
pub const CANDIDATE_CELL_SMALL: f32 = KEY_MIN_HEIGHT;

/// Height in dp of a one-character candidate cell on `small`.
pub const CANDIDATE_CELL_HANZI_SMALL: f32 = 32.0;

/// Gap in dp between two candidate cells.
pub const CANDIDATE_GAP: f32 = 4.0;
/// Padding in dp at each end of a candidate's word.
pub const CANDIDATE_PAD: f32 = 6.0;

/// Candidate label size in dp: a word to pick, not a string to compare,
/// so the proportional face fits more letters per cell.
pub const fn candidate_label(class: SizeClass) -> f32 {
    match class {
        SizeClass::Mobile => CANDIDATE_LABEL_MOBILE,
        _ => BODY,
    }
}

/// The phone's candidate size, the most a strip of four cells takes
/// before a word of [`CANDIDATE_FIT_CHARS`] letters fills its cell.
pub const CANDIDATE_LABEL_MOBILE: f32 = 18.0;

/// The letters a candidate cell is sized for: the longest words of every
/// Latin list but Italian, whose nine-letter words take a smaller size in
/// their own cells.
pub const CANDIDATE_FIT_CHARS: usize = 8;

/// The step in dp a candidate's size falls by until its word fits.
pub const CANDIDATE_SIZE_STEP: f32 = 0.5;

/// Rows of candidates between the field and the keys
/// (`docs/DESIGN.md` §4.3: "Two rows of 3 at 36 dp (`small`), two rows
/// of 4 (`mobile`), one row of natural-width chips (`wide`); a list
/// whose words are one character shows ten a row, two rows, on every
/// class").
///
/// `one_char` is that list: a Chinese one, whose words are single
/// characters and whose strip is [`CANDIDATE_CELLS_HANZI`] wide.
pub const fn candidate_rows(class: SizeClass, one_char: bool) -> usize {
    if one_char {
        return CANDIDATE_ROWS_HANZI;
    }
    match class {
        SizeClass::Wide => 1,
        _ => 2,
    }
}

/// Candidates in one row.
pub const fn candidates_per_row(class: SizeClass, one_char: bool) -> usize {
    if one_char {
        return CANDIDATE_CELLS_HANZI;
    }
    match class {
        SizeClass::Small => 3,
        _ => 4,
    }
}

/// Candidates the strip shows in all.
pub const fn candidates(class: SizeClass, one_char: bool) -> usize {
    candidate_rows(class, one_char) * candidates_per_row(class, one_char)
}

// ---------------------------------------------------------------------
// Panels
// ---------------------------------------------------------------------

/// Padding in dp inside a secret panel, between its border and its rows.
pub const SECRET_PAD: f32 = GAP;

/// Words on one page of a secret panel (`docs/DESIGN.md` §4.10).
///
/// Six is what the 268 × 358 dp panel holds once the app bar, the pager
/// under the panel and the action row have their height. A phone is tall
/// enough for a whole secret in two columns — twelve words as two columns
/// of six, twenty-four as two columns of twelve — so nothing a person has
/// to write down is split across a pager there. A 640 dp desktop window
/// is not: twelve rows of [`WORD_ROW_LARGE`] are taller than the space
/// between its app bar and its action, so it pages twelve at a time,
/// which is the count §4.10 names for it.
pub const fn words_per_page(class: SizeClass) -> usize {
    match class {
        SizeClass::Small => 6,
        SizeClass::Mobile => 24,
        SizeClass::Wide => 12,
    }
}

/// Columns of numbered words in a secret panel.
pub const fn word_columns(class: SizeClass) -> usize {
    match class {
        SizeClass::Small => 1,
        _ => 2,
    }
}

/// Height in dp of one numbered word row on a 268 × 358 dp panel, which
/// is also the height of the one-line value panel.
pub const WORD_ROW: f32 = 24.0;

/// Height in dp of one numbered word row on a phone or a desktop window
/// (`docs/DESIGN.md` §4.10: "twelve at 40 dp rows").
pub const WORD_ROW_LARGE: f32 = 40.0;

/// Height in dp of one numbered word row for the size class. Fixed
/// rather than measured, so the panel's rectangle is the same masked and
/// revealed.
pub const fn word_row(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => WORD_ROW,
        _ => WORD_ROW_LARGE,
    }
}

/// Monospace size in dp of a word in a secret panel, for the size class
/// (`docs/DESIGN.md` §4.10).
pub const fn word_mono(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => MONO,
        _ => MONO_LARGE,
    }
}

/// Monospace size in dp of a word in a panel that also carries the
/// wordlist number beside it: [`MONO_INDEX`] where the class draws two
/// columns, and [`MONO`] on a 268 dp panel, which draws one and has the
/// width for it.
pub const fn word_mono_with_index(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => MONO,
        _ => MONO_INDEX,
    }
}

/// Monospace size in dp of a words panel carrying the steel rows of
/// `docs/PLANNING.md` §8.2 item 6: the wordlist number, the four
/// punched letters and the word. Fourteen characters more a row than a
/// word alone, which on a 268 dp panel is the largest size that still
/// fits inside the panel's own padding.
pub const MONO_STEEL: f32 = 14.0;

/// Monospace size in dp of a steel row for the size class.
pub const fn word_mono_steel(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => MONO_STEEL,
        _ => MONO_INDEX,
    }
}

/// Gap in dp between the two columns of a words panel. Monospace text has
/// no padding of its own, so an eight-letter word in the first column
/// would otherwise touch the number that starts the second.
pub const WORD_COLUMN_GAP: f32 = 24.0;

/// The same gap for a panel that also carries the wordlist numbers: the
/// number itself already separates the columns, and the four characters
/// it takes have to come from somewhere.
pub const WORD_COLUMN_GAP_INDEXED: f32 = GAP;

/// Gap in dp between the columns of a words panel, with and without the
/// wordlist numbers beside the words.
pub const fn word_column_gap(indexed: bool) -> f32 {
    if indexed {
        WORD_COLUMN_GAP_INDEXED
    } else {
        WORD_COLUMN_GAP
    }
}

// ---------------------------------------------------------------------
// Codes
// ---------------------------------------------------------------------

/// Side in dp of a QR code for the size class (`docs/DESIGN.md` §3:
/// "Code side: 198 on `small`, 260 on `mobile`, 240 on `wide`").
///
/// One side per class rather than "as large as the width allows", so a
/// code is the same square wherever a flow shows one and a reader knows
/// what to aim a camera at. §4.9: "The screen finds the room; on `small`
/// the toggle row and the progress row give way ... before the square
/// does." The QR screen's budget on `small`, which is the tightest of
/// them: 56 app bar + 198 square + 4 gap + 20 label + 4 gap + 44 toggle
/// ([`qr_toggle_row`]) + 4 gap + 6 progress ([`qr_progress_row`]) + 16
/// under it = 352 of 358 dp.
pub const fn qr_side(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => 198.0,
        SizeClass::Mobile => 260.0,
        SizeClass::Wide => 240.0,
    }
}

/// Modules of light margin around a code, as the QR standard requires.
pub const QR_QUIET_ZONE: usize = osk_codec::qr::QUIET_ZONE;

/// Modules of a transcription grid between two thick guide lines
/// (`docs/PLANNING.md` §8.2 item 5a). Five is what the community's
/// SeedQR transcription templates use, and what a person counts without
/// losing their place.
pub const QR_GRID_BAND: usize = 5;

/// Side in dp of one module of a transcription grid, at which a square
/// can be filled in by hand with a pen. The grid takes the largest whole
/// number of pixels per module that fits its square and never goes below
/// this, which is what makes a 29-module symbol page by quadrant on
/// `small`.
pub const QR_GRID_MODULE: f32 = 8.0;

/// Width in dp of the thin line between two modules of a transcription
/// grid.
pub const QR_GRID_LINE: f32 = 1.0;

/// Width in dp of the thick guide line every [`QR_GRID_BAND`] modules.
pub const QR_GRID_GUIDE: f32 = 2.0;

/// Type size in dp of a transcription grid's row and column labels,
/// which sit in a gutter one module wide along the top and left edges.
pub const QR_GRID_LABEL: f32 = CAPTION;

/// The same label where a cell is too narrow for [`QR_GRID_LABEL`]: a
/// 29-module symbol drawn whole has two-digit columns in 11 dp cells,
/// and a label that spills into its neighbour is worse than a small one.
pub const QR_GRID_LABEL_SMALL: f32 = 8.0;

/// Space in dp a grid label leaves inside its cell, so that two
/// two-digit column numbers side by side read as two numbers and not
/// as "1011".
pub const QR_GRID_LABEL_GAP: f32 = 2.0;

/// Side in dp of a transcription grid for the size class: as much of the
/// panel as the class has, capped on `wide` so the grid does not become
/// the window.
pub const fn qr_grid_side(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => 232.0,
        SizeClass::Mobile => 340.0,
        SizeClass::Wide => 380.0,
    }
}

/// Largest side in dp of the scanner's square viewfinder on `wide`,
/// where the pane would otherwise give it most of the window
/// (`docs/DESIGN.md` §4.9: "as wide as the pane allows").
pub const VIEWFINDER_MAX: f32 = 360.0;

/// Largest short side in pixels of the camera preview the scanner keeps
/// and draws (`docs/DESIGN.md` §4.9). The largest viewfinder is
/// [`VIEWFINDER_MAX`] dp on `wide` at 160 dpi, 360 px, and the reference
/// panel's is 286 px, so a frame reduced to this covers every square
/// without carrying pixels no display can show.
pub const PREVIEW_MAX: usize = 400;

/// Least module pitch in millimetres a phone camera reads first time. A
/// code below it is offered as animated parts instead
/// (`docs/DESIGN.md` §4.9).
pub const QR_MIN_PITCH_MM: f32 = 0.5;

/// Least side in millimetres a keyboard key's hit target may have. The
/// touch floor of a chip or a row is stated in dp, but a keyboard fits
/// ten keys across whatever the panel, so the key that a finger has to
/// hit is measured against the finger instead: 4 mm is the smallest
/// target a keyboard can be typed on without mistakes piling up
/// (ISO 9241-411's dense-target minimum).
pub const KEY_MIN_TAP_MM: f32 = 4.0;

/// Height in dp of a progress bar and of the viewfinder's state line.
pub const BAR_HEIGHT: f32 = 6.0;

/// Height in dp of the QR screen's "Animated" toggle row
/// (`docs/DESIGN.md` §4.9: "on `small` the toggle row and the progress
/// row give way (they shrink to one line each ...) before the square
/// does").
///
/// On `small` the row is one line at the touch floor and the reason sits
/// beside the label, because the class's own square, its label and a
/// progress row leave the row 44 dp and no more (see [`qr_side`]).
/// Elsewhere it is the ordinary choice row, two lines when it carries a
/// reason (§4.11).
pub const fn qr_toggle_row(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => touch_floor(class),
        _ => choice_row(class),
    }
}

/// Height in dp of the QR screen's progress row, reserved whether or not
/// the QR is animated (`docs/DESIGN.md` §4.9: "a progress bar row, always
/// present, blank when static").
///
/// On `small` it is the bar alone and the count folds into the title,
/// which is what §3 allows a class to do with progress that does not fit
/// ("may fold progress into the title where a progress row does not
/// fit"). Elsewhere it is §4.14's count above its bar.
pub const fn qr_progress_row(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => BAR_HEIGHT,
        _ => line_box(LABEL) + GAP_SMALL + BAR_HEIGHT,
    }
}

// ---------------------------------------------------------------------
// Chips and badges
// ---------------------------------------------------------------------

/// Padding in dp at each end of a chip's label.
pub const CHIP_PAD: f32 = 12.0;
/// Height in dp of the compact chip: a label with a rectangle round it,
/// not a touch target of its own.
pub const CHIP_COMPACT: f32 = 24.0;
/// Padding in dp at each end of a compact chip's label.
pub const CHIP_COMPACT_PAD: f32 = 6.0;
/// Gap in dp between two chips in a row.
pub const CHIP_GAP: f32 = GAP_SMALL;
/// Padding in dp at each end of a badge's label.
pub const BADGE_PAD: f32 = 6.0;
/// Padding in dp above and below a badge's label.
pub const BADGE_PAD_Y: f32 = 3.0;
/// Corner radius in dp of a badge.
pub const BADGE_RADIUS: f32 = 4.0;

/// Characters in one group of a chunked or elided string
/// (`docs/DESIGN.md` §4.5: "chunked in fours", "in groups of four").
pub const CHUNK_GROUP: usize = 4;
/// Strings longer than this are chunked in groups of four
/// (`docs/DESIGN.md` §4.5); a fingerprint is never chunked.
pub const CHUNK_THRESHOLD: usize = 16;
/// Smallest type size in dp a chunked string is drawn at. Below this a
/// group of four is guessed rather than read, so the grid gives way to a
/// pager.
pub const CHUNK_MIN_SIZE: f32 = 8.0;

/// Characters kept at the head of an elided reference string
/// (`docs/DESIGN.md` §4.5: "first 8 · last 8 characters").
pub const ELIDE_HEAD: usize = 8;
/// Characters kept at the tail of an elided reference string.
pub const ELIDE_TAIL: usize = 8;
/// The character that stands for the middle of an elided string.
pub const ELLIPSIS: &str = "\u{2026}";

/// The character that groups the digits of an amount
/// (`docs/DESIGN.md` §4.7). A no-break space rather than a thin space:
/// the faces carry U+0020–U+00FF and a short list of symbols, so U+2009
/// would draw as a missing glyph. This one is there, reads as a space,
/// and keeps a grouped number on one line. Adding U+2009 to
/// `tools/fontbake` and re-running it would let this be the thin space
/// the design names.
pub const GROUP_SEPARATOR: char = '\u{00A0}';

// ---------------------------------------------------------------------
// Record
// ---------------------------------------------------------------------

/// Share of the body width a record's label column takes.
pub const RECORD_LABEL_SHARE: f32 = 0.36;
/// Least width in dp of a record's label column: two or three words.
pub const RECORD_LABEL_MIN: f32 = 96.0;
/// Greatest width in dp of a record's label column, so a wide pane does
/// not put the values in the middle of the screen.
pub const RECORD_LABEL_MAX: f32 = 176.0;

// ---------------------------------------------------------------------
// Hub and the wide template
// ---------------------------------------------------------------------

/// Columns in the hub grid: two on a `Small` or `Mobile` panel that is
/// taller than it is wide, four on one that is wider than it is tall, so
/// that eight tiles stand in two rows there rather than four, and three
/// in a `Wide` content pane.
pub const fn hub_columns(class: SizeClass, landscape: bool) -> usize {
    match class {
        SizeClass::Wide => 3,
        _ if landscape => 4,
        _ => 2,
    }
}

/// Tallest a hub tile is drawn, as a share of its own width
/// (`docs/DESIGN.md` §4.1: "eight equal, square tiles ... A tile is
/// never taller than it is wide"). A square where the height allows one,
/// and wider than it is tall on a panel that cannot give eight squares.
pub const TILE_ASPECT: f32 = 1.0;
/// Shortest a hub tile is drawn, so a full grid stays tappable.
pub const TILE_MIN_HEIGHT: f32 = 56.0;

/// Icon size in dp inside a hub tile: on a phone the grid is the whole
/// screen, so the mark in it is one step up the ramp.
pub const fn tile_icon(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => ICON,
        _ => ICON_LARGE,
    }
}

/// Label size in dp inside a hub tile: the dense label on a 268 dp
/// panel, the body size elsewhere.
pub const fn tile_label(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => LABEL,
        _ => BODY,
    }
}

/// The step down a tile's label takes where its word is wider than the
/// tile: the scanner's band holds "Read a file" beside "Paste" and
/// "Type", and a third of a 268 dp panel is narrower than the first of
/// them at [`tile_label`].
pub const fn tile_label_tight(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => CAPTION,
        _ => LABEL,
    }
}

/// Height in dp a tile needs for its mark, the grid step under it and
/// its word, which is the height of Home's band of three
/// (`docs/DESIGN.md` §4.1). Never shorter than [`TILE_MIN_HEIGHT`], so
/// the band stays tappable.
pub const fn tile_band_height(class: SizeClass) -> f32 {
    // The mark, the grid step under it, the word at the class's own type
    // scale, and one step of air so no descender meets the tile's edge.
    let needed = tile_icon(class) + GAP + scaled_line(tile_label(class), class) + GAP;
    if needed > TILE_MIN_HEIGHT {
        needed
    } else {
        TILE_MIN_HEIGHT
    }
}

/// Width in dp of the navigation sidebar on `Wide`.
pub const SIDEBAR_WIDTH: f32 = 240.0;
/// Largest width in dp of a wizard or secret frame on `Wide`: buttons
/// and keyboards never span a desktop window.
pub const WIZARD_MAX_WIDTH: f32 = 640.0;
/// Largest width in dp of a document's content on `Wide`.
pub const DOCUMENT_MAX_WIDTH: f32 = 800.0;
/// Largest width in dp of an on-screen keyboard on `Wide`.
pub const KEYBOARD_MAX_WIDTH: f32 = 720.0;
/// Largest width in dp of a form or list column on `Wide`
/// (`docs/DESIGN.md` §3). A row wider than this puts its value half a
/// window away from its label.
pub const COLUMN_MAX_WIDTH: f32 = 480.0;
/// Largest width in dp of a button on `Wide`.
pub const BUTTON_MAX_WIDTH: f32 = 320.0;
/// Least width in dp of a button on `Wide`, so a one-word action is
/// still a target and two actions beside each other read as a pair.
pub const BUTTON_MIN_WIDTH: f32 = 160.0;

/// Flex weight of a screen body against a weighted footer. Shrinking is
/// proportional to weight, so this is large enough that the body absorbs
/// a short screen's whole deficit before the footer gives up anything.
pub const BODY_SHRINK_WEIGHT: f32 = 100.0;
/// Flex weight of an entry screen's keyboard against the group above it.
pub const KEYBOARD_SHRINK_WEIGHT: f32 = 100.0;
/// Space in dp between an entry screen's docked group and its keys.
pub const ENTRY_DOCK_GAP: f32 = GAP_SMALL;

/// Least height in dp of the block above an entry group that holds the
/// typed value whole — §4.3's address on Verify, "the one value whose
/// tail is not enough".
///
/// The block is the space the law centres, so it takes whatever is going;
/// this is the floor it claims, and the 268 × 358 dp panel is the binding
/// case: 56 app bar + 4 gap + 24 block + 4 gap + 52 field + 4 gap + 17
/// error line + 194 keys (the passphrase keyboard's five rows at
/// [`KEY_MIN_HEIGHT`] and the bottom reserve) = 355 of 358 dp. What is
/// left over goes to the block, which is 31 dp there — two lines of
/// groups of four at the small end of the comparison ramp, which is what
/// puts a 42-character address on the panel whole.
pub const TYPED_VALUE_BLOCK: f32 = 24.0;

// ---------------------------------------------------------------------
// Structure chart
// ---------------------------------------------------------------------

/// Padding in dp inside a structure chart's node (`docs/DESIGN.md`
/// §4.16): a grid step and a half, so that five nodes stand side by side
/// on a desktop card.
pub const CHART_NODE_PAD: f32 = GRID + GAP_SMALL;
/// The narrowest a node is drawn before its row wraps onto another line.
pub const CHART_NODE_MIN: f32 = 112.0;
/// The widest a node is drawn: a row of few nodes is centred, not
/// stretched across the card.
pub const CHART_NODE_MAX: f32 = 240.0;
/// The gap in dp between two nodes side by side, which a line passing
/// down between them runs through.
pub const CHART_NODE_GAP: f32 = GRID + GAP_SMALL;
/// The space in dp above and below the lines' runs across, between two
/// lines of nodes.
pub const CHART_BAND: f32 = GRID + GAP_SMALL;
/// The distance in dp between two lines running side by side, across a
/// band or down a gap.
pub const CHART_LANE: f32 = GAP_SMALL;
/// A line's stroke in dp.
pub const CHART_STROKE: f32 = 1.0;
/// A key's lines while its sheet's **Where it is** shows them, the rest
/// dimmed.
pub const CHART_STROKE_BOLD: f32 = 2.0;
/// Width in dp of the sheet a chart's node opens, on a wide display.
pub const CHART_SHEET_WIDTH: f32 = 560.0;
/// Height in dp of an action row on that sheet: one line, its value at
/// the right.
pub const CHART_SHEET_ROW: f32 = 44.0;
/// The open vault's kinds column, in dp, while a wallet's chart is
/// shown: narrower, so that the chart beside it has room for five nodes
/// on a line at the desktop window's size.
pub const CHART_PANE_KINDS: f32 = 128.0;
/// The open vault's list of items, in dp, while a wallet's chart is
/// shown.
pub const CHART_PANE_ITEMS: f32 = 176.0;
/// Height in dp of a button inside a node.
pub const CHART_BUTTON: f32 = 32.0;
/// Each key's dash pattern, in dp: on, off, on, off. The first is solid.
/// With the line colours, which cycle with them, they tell the keys'
/// lines apart where colour alone does not.
pub const CHART_DASHES: [[f32; 4]; 5] = [
    [1.0, 0.0, 1.0, 0.0],
    [8.0, 4.0, 8.0, 4.0],
    [2.0, 3.0, 2.0, 3.0],
    [10.0, 3.0, 2.0, 3.0],
    [14.0, 4.0, 5.0, 4.0],
];

// ---------------------------------------------------------------------
// Timings
// ---------------------------------------------------------------------

/// Entries per group in a run of dice rolls or coin flips
/// (`docs/DESIGN.md` §4.3: "in groups of five").
pub const ENTROPY_GROUP: usize = 5;

/// Share of its type size one monospace character advances. The baked
/// mono face is fixed-pitch, so a run of `n` characters is `n` times
/// this times the size, which is how a run of entries works out how many
/// of its groups of [`ENTROPY_GROUP`] fit the pad's width.
pub const MONO_ADVANCE: f32 = 0.6;

/// Groups of [`ENTROPY_GROUP`] entries that fit `width_dp` at the
/// class's monospace size, at least one. A run longer than this shows
/// its most recent groups; a group is never cut in half (§4.3).
pub fn entropy_groups(width_dp: f32, class: SizeClass) -> usize {
    let advance = MONO * type_scale_pct(class) as f32 / 100.0 * MONO_ADVANCE;
    if advance <= 0.0 {
        return 1;
    }
    // Every group but the first carries the space in front of it.
    let chars = (width_dp / advance).max(0.0) as usize;
    ((chars + 1) / (ENTROPY_GROUP + 1)).max(1)
}

/// Width in dp of a switch's track.
pub const SWITCH_WIDTH: f32 = 48.0;
/// Height in dp of a switch's track.
pub const SWITCH_HEIGHT: f32 = 26.0;
/// Inset in dp of a switch's knob inside its track.
pub const SWITCH_KNOB_INSET: f32 = 3.0;

/// Width in dp a hub tile asks for before the grid shares out what the
/// pane has.
pub const TILE_WIDTH: f32 = 88.0;

/// Icon size in dp inside a row shorter than [`MENU_ROW`], where the
/// full [`ICON`] would crowd the two lines beside it.
pub const ICON_DENSE: f32 = 24.0;

/// Padding in dp at each end of the count inside a tile's badge pill.
pub const TILE_BADGE_PAD: f32 = 5.0;

/// Padding in dp at each end of a chip whose label size the caller
/// chose: a dense row of chips gives its words the space the ordinary
/// chip gives them, halved.
pub const CHIP_PAD_TIGHT: f32 = 2.0;

/// The step in dp between two sizes of a chunked string: the comparison
/// ramp is every second dp, walked down until the groups of four fit.
pub const CHUNK_STEP: f32 = 2.0;

/// Millimetres in an inch, for turning a module pitch in pixels into
/// one a camera sees.
pub const MM_PER_INCH: f32 = 25.4;

/// How far towards white the thin lines of the SeedQR transcription grid
/// are mixed: light enough to read the filled modules through, dark
/// enough to draw along.
pub const QR_GRID_LINE_MIX: f32 = 0.55;

/// How far towards white a pressed accent surface is mixed — the primary
/// button, ✓, a selected candidate cell, a selected chip, the danger
/// button from its own red (`docs/PLANNING.md` §16.124 rule 2).
pub const PRESSED_MIX: f32 = 0.35;

/// How far towards the accent a pressed neutral surface is mixed from its
/// own resting fill: a key, a candidate cell, a row, an unselected chip,
/// a tile, a secondary button (`docs/PLANNING.md` §16.124 rule 1).
pub const PRESSED_TINT: f32 = 0.35;

/// Diameter in dp of the eye's ring inside its 48 dp target
/// (`docs/DESIGN.md` §4.10).
pub const RING_SIZE: f32 = 36.0;
/// Stroke width in dp of the eye's ring.
pub const RING_STROKE: f32 = 2.0;

/// Stroke width in dp of the focus ring a keyboard leaves on the item it
/// has moved focus to (`docs/DESIGN.md` §4.15).
pub const FOCUS_STROKE: f32 = 2.0;

/// Hold-button duration in milliseconds (`docs/DESIGN.md` §4.13).
pub const HOLD_MS: u64 = 1500;
/// Milliseconds a newly entered die or coin shows before it masks
/// (`docs/DESIGN.md` §4.3: "the newest entry shows for half a second,
/// then masks").
pub const REVEAL_FLASH_MS: u64 = 500;
/// Milliseconds a key, a cell, a row or a button keeps its pressed look
/// after the finger lifts, so the person sees what they hit once their
/// finger is off it (`docs/PLANNING.md` §16.120, §16.124 rule 3).
pub const PRESS_LINGER_MS: u64 = 200;
/// Milliseconds of that window the look keeps its full strength; for the
/// rest of it both mixes are halved (`docs/PLANNING.md` §16.124 rule 3).
pub const PRESS_LINGER_FULL_MS: u64 = 100;
/// Seconds the app bar's eye keeps a secret on screen
/// (`docs/DESIGN.md` §4.10: "The eye in the app bar shows it for 30 s").
/// While it runs, the eye is drawn as a ring that empties clockwise,
/// which §4.14 makes the only countdown in the interface.
pub const EYE_REVEAL_S: u32 = 30;
