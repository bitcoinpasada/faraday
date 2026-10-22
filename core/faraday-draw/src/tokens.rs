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
/// How far a marked run of monospace text is drawn again to the right to read heavier (the face has one weight).
pub const MARK_BOLD: f32 = 0.5;
/// Screen padding: the margin between the content and the screen edge.
pub const PAD: f32 = 16.0;
/// Corner radius of cards, rows, tiles and buttons.
pub const RADIUS: f32 = 12.0;
/// Corner radius of the small rectangles: chips, candidate cells, keys.
pub const RADIUS_SMALL: f32 = 6.0;

// ---------------------------------------------------------------------
// Chrome
// ---------------------------------------------------------------------

/// Minimum touch target.
pub const TOUCH: f32 = 48.0;
/// Icon size inside a menu row or a tile.
pub const ICON: f32 = 28.0;
/// Icon size inside a list row or an app bar.
pub const ICON_SMALL: f32 = 20.0;
/// The room a page's way back takes above its title, on a wide
/// display: the chevron and where it goes.
pub const BACK_LINK_ROW: f32 = 26.0;

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

/// A value row (`docs/DESIGN.md` §4.2 "Value row"): the label above, the
/// value below, a chevron at the right. The two-line row's height.
pub const fn value_row(class: SizeClass) -> f32 {
    stacked_row(class)
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
/// rows of keys.
pub const fn field(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => key_height(SizeClass::Small),
        _ => FIELD,
    }
}

/// A one-line text field's height.
pub const TEXT_FIELD: f32 = 40.0;

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
/// group there is (`yi4`, 20 characters), so
/// nobody pages.
pub const CANDIDATE_CELLS_HANZI: usize = 10;

/// Rows of candidates for such a list, on every class: two, which is
/// [`CANDIDATE_CELLS_HANZI`] short of a whole tone group.
pub const CANDIDATE_ROWS_HANZI: usize = 2;

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

/// Largest short side in pixels of the camera preview the scanner keeps
/// and draws (`docs/DESIGN.md` §4.9). The largest viewfinder is
/// [`VIEWFINDER_MAX`] dp on `wide` at 160 dpi, 360 px, and the reference
/// panel's is 286 px, so a frame reduced to this covers every square
/// without carrying pixels no display can show.
pub const PREVIEW_MAX: usize = 400;

/// Height in dp of a progress bar and of the viewfinder's state line.
pub const BAR_HEIGHT: f32 = 6.0;

/// A corner bracket's arm over a camera's picture, as a share of the
/// picture's short side (`docs/DESIGN.md` §4.9: "a square viewfinder
/// with corner brackets").
pub const VIEWFINDER_BRACKET_SHARE: f32 = 1.0 / 6.0;
/// Stroke in dp of a corner bracket.
pub const VIEWFINDER_BRACKET_STROKE: f32 = 3.0;
/// How far in dp the brackets stand inside the picture's edge.
pub const VIEWFINDER_BRACKET_INSET: f32 = GAP;
/// Space in dp round the state line and the parts bar on their band
/// inside the picture, and between them.
pub const VIEWFINDER_BAND_PAD: f32 = GAP;
/// Opacity (of 255) of the band the state line sits on over the
/// picture: dark enough that white words read over a white QR, light
/// enough that the picture shows through.
pub const VIEWFINDER_BAND_ALPHA: u8 = 180;
/// Opacity (of 255) of the parts bar's empty track on that band: the
/// bar's own colour, faint.
pub const VIEWFINDER_TRACK_ALPHA: u8 = 70;
/// Size in dp of the state line's words on the band inside the picture.
pub const VIEWFINDER_STATE_SIZE: f32 = CAPTION;

// ---------------------------------------------------------------------
// Chips and badges
// ---------------------------------------------------------------------

// ---------------------------------------------------------------------
// Record
// ---------------------------------------------------------------------

// ---------------------------------------------------------------------
// Hub and the wide template
// ---------------------------------------------------------------------

/// The highest screen density, in pixels per dp, that raises the
/// window size the wide layout needs (`docs/DESIGN.md` §3, F78). Above
/// it, a window's own pixels decide: a window of the same pixel size
/// takes the same layout at any compositor scale from this one up.
pub const WIDE_FLOOR_DENSITY: f32 = 1.5;

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
// Backup board
// ---------------------------------------------------------------------

/// The narrowest a stick box or Paper is drawn on a place's row of the
/// backup board (`docs/DESIGN.md` §4.17) before the sticks wrap onto
/// another line inside the place: a chip with a fingerprint and its
/// marks fits on one line inside it.
pub const BOARD_BOX_MIN: f32 = 160.0;
/// The widest a chip is laid out when it is drawn alone: the chip being
/// dragged.
pub const BOARD_CARD_MAX: f32 = 280.0;
/// The gap in dp between two places on the board, one under another
/// (`docs/BACKUP-FLOW.md` §17.15).
pub const BOARD_PLACE_GAP: f32 = 2.0 * GRID + GAP_SMALL;
/// Width in dp of a place's left stripe, in its edge's colour.
pub const BOARD_PLACE_STRIPE: f32 = 5.0;
/// Stroke in dp of a place's edge, Paper's, and **+ stick**'s and **+
/// place**'s dashed edges.
pub const BOARD_BOX_EDGE: f32 = 1.5;
/// Width in dp of the **+ stick** box on a place's row.
pub const BOARD_ADD_STICK: f32 = 72.0;
/// Height in dp of a place's line of fact, an outlined tag in its header
/// band.
pub const BOARD_TAG: f32 = 20.0;
/// A dash of a dashed edge in dp, and the gap after it.
pub const BOARD_DASH: f32 = GAP_SMALL;
/// How far in dp a chip's faint shadow falls below it.
pub const BOARD_CHIP_DROP: f32 = 1.0;
/// Opacity (of 255) of a chip's faint shadow.
pub const BOARD_CHIP_SHADOW: u8 = 30;
/// The gap in dp round the list sheet, and between the board and what is
/// over it.
pub const BOARD_GAP: f32 = GRID + GAP_SMALL;
/// Height in dp of a place's header band: its glyph, name, line of fact
/// and ×; and of the **+ place** row.
pub const BOARD_HEAD: f32 = 36.0;
/// Padding in dp inside a place card and inside each of its zones.
pub const BOARD_PAD: f32 = GAP;
/// Height in dp of a zone's label row ("Stick 1", "Paper").
pub const BOARD_ZONE_LABEL: f32 = 20.0;
/// Height in dp of a chip, on the cards and in the list.
pub const BOARD_CHIP: f32 = 28.0;
/// The gap in dp between two chips, beside or under each other.
pub const BOARD_CHIP_GAP: f32 = GAP_SMALL;
/// The side of a chip's marks in dp: its glyph, its × target's cross,
/// its link mark and its warning mark.
pub const BOARD_MARK: f32 = 12.0;
/// Stroke in dp of a drawn mark: the × and the link.
pub const BOARD_MARK_STROKE: f32 = 1.5;
/// Stroke in dp of the edge round a zone that takes the chip held, and
/// round the chip held.
pub const BOARD_EDGE: f32 = 2.0;
/// Width in dp of a chip's ⋮ menu, a popup under the ⋮
/// (`docs/BACKUP-FLOW.md` §17.20); the board's sheets on `wide` are twice
/// it.
pub const BOARD_POPUP_WIDTH: f32 = 220.0;
/// Diameter in dp of each of the three dots of a chip's ⋮.
pub const BOARD_MENU_DOT: f32 = 3.0;
/// Opacity (of 255) of the thin rule between a chip and its ⋮, drawn in
/// the ⋮'s colour.
pub const BOARD_MENU_RULE: u8 = 120;
/// Height in dp of a row of a chip's ⋮ menu: a box it can go to, or a
/// place's name over them.
pub const BOARD_MENU_ROW: f32 = 32.0;
/// How far in dp a dragged chip's shadow falls below it.
pub const BOARD_LIFT: f32 = GAP_SMALL;
/// Height in dp of a stick box's head on a place card ("Stick 1" and the
/// × that takes the stick off): the second of the board's three nested
/// boxes, place, stick, vault.
pub const BOARD_STICK_HEAD: f32 = 24.0;
/// Padding in dp inside a stick box, round its vault box and its files.
pub const BOARD_STICK_PAD: f32 = GAP_SMALL;
/// Stroke in dp of a stick box's edge: heavier than a zone's, so the
/// stick reads as a box of its own inside the place card.
pub const BOARD_STICK_EDGE: f32 = 1.5;
/// Height in dp of a vault box's head ("Vault 1", its marks and ×): a
/// chip's height, the head being the vault's chip.
pub const BOARD_VAULT_HEAD: f32 = BOARD_CHIP;
/// Padding in dp inside a vault box, round the records it holds.
pub const BOARD_VAULT_PAD: f32 = GAP_SMALL;
/// Stroke in dp of a vault box's edge.
pub const BOARD_VAULT_STROKE: f32 = 2.0;
/// Opacity (of 255) of the band's text colour laid over a vault's band
/// while it is pressed or hovered.
pub const BOARD_VAULT_HOT: u8 = 40;

// ---------------------------------------------------------------------
// Timings
// ---------------------------------------------------------------------

/// Stroke width in dp of the focus ring a keyboard leaves on the item it
/// has moved focus to (`docs/DESIGN.md` §4.15).
pub const FOCUS_STROKE: f32 = 2.0;

/// Hold-button duration in milliseconds (`docs/DESIGN.md` §4.13).
pub const HOLD_MS: u64 = 1500;
/// Milliseconds a passing state stays on a line before the line goes
/// back to what it said (`docs/DESIGN.md` §4.9: "Nothing to paste" on
/// the scanner's state line "for two seconds").
pub const NOTE_FLASH_MS: u64 = 2000;
/// Milliseconds the scanner waits for the camera's first frame before
/// it says the camera sends no picture (`docs/QR.md` §8 item 6).
pub const NO_PICTURE_MS: u64 = 6000;
/// Milliseconds a QR is seen large enough and not read before the
/// scanner says how to hold it (`docs/QR.md` §8 item 8).
pub const POOR_PICTURE_MS: u64 = 4000;

/// The mode pill's height (`docs/MODES.md` §3).
pub const MODE_PILL_HEIGHT: f32 = 28.0;

/// The mode pill's inset from its edge to its label, each side.
pub const MODE_PILL_PAD: f32 = 12.0;

/// The mode sheet's widest, as the Load sheet's.
pub const MODE_SHEET_WIDTH: f32 = 640.0;

/// A row of the mode sheet, and of Settings' Mode row.
pub const MODE_ROW: f32 = 48.0;
