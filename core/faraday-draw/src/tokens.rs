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

use crate::geom::{SizeClass, WindowClass};

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
/// How far in a multi-choice row's label starts: past its checkbox.
pub const LIST_INDENT: f32 = 40.0;
/// What a multi-choice row's label leaves free at its right: the
/// checkbox's place and the row's edge.
pub const LIST_RESERVE: f32 = 50.0;
/// A stepper's row: its buttons and the gap under them.
pub const STEPPER_ROW: f32 = 50.0;
/// Primary menu row height: an icon, a label and a chevron.
pub const MENU_ROW: f32 = 72.0;
/// Menu row height on a `small` panel, where five rows and the app bar
/// have to share 358 dp.
pub const MENU_ROW_SMALL: f32 = 52.0;

/// Menu row height in dp for the size class: 52 on `small`, 72
/// elsewhere.
pub const fn menu_row(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => MENU_ROW_SMALL,
        _ => MENU_ROW,
    }
}

/// Height in dp of a row that carries a value under its label.
///
/// One height for every such row, so a list of mixed kinds reads as one
/// list.
pub const fn stacked_row(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => ROW,
        _ => MENU_ROW,
    }
}

/// A row with its label above its value and a chevron at the right: the
/// two-line row's height.
pub const fn value_row(class: SizeClass) -> f32 {
    stacked_row(class)
}

// ---------------------------------------------------------------------
// Entry
// ---------------------------------------------------------------------

/// Height of a one-row text field above a keyboard: 52 dp, and on
/// `small` a letter key's own height, 40 dp.
///
/// On the panel the field is as tall as a letter key, which leaves room
/// for two rows of candidates over three rows of keys.
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

/// Letter-key height in dp for the size class: 40 on `small`, 56
/// elsewhere.
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

/// A codex32 keyboard's row in dp, every row the same: Material 3's
/// 48 dp touch target ([`TOUCH`]), on a page's keyboard as on the docked
/// one (`docs/CODEX32.md` §8.4).
pub const CODEX32_KEY_ROW: f32 = TOUCH;

/// Columns every other pad divides its width into: the dice pad's three
/// faces and the backspace beside them.
pub const PAD_COLUMNS: f32 = 4.0;

/// Width in key units of an edge key that takes a key and a half:
/// shift, backspace and ✓ at the ends of a letter row.
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

/// Rows of candidates between the field and the keys: two on `small`
/// and `mobile`, one on `wide`; a list whose words are one character
/// has two rows of ten on every class ([`candidates_per_row`]).
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

/// Side in dp of a QR code for the size class: 198 on `small`, 260 on
/// `mobile`, 240 on `wide`. A held SeedQR in a vault is drawn at this
/// side, or the field's width where that is less; `mobile` scales the
/// PNG a QR code is saved as (`docs/DESIGN.md` §3).
pub const fn qr_side(class: SizeClass) -> f32 {
    match class {
        SizeClass::Small => 198.0,
        SizeClass::Mobile => 260.0,
        SizeClass::Wide => 240.0,
    }
}

/// Largest short side in pixels of the camera preview the scanner keeps
/// and draws (`docs/DESIGN.md` §4.9, Scanner). The largest viewfinder is
/// [`VIEWFINDER_MAX`] dp on `wide` at 160 dpi, 360 px, and the reference
/// panel's is 286 px, so a frame reduced to this covers every square
/// without carrying pixels no display can show.
pub const PREVIEW_MAX: usize = 400;

/// Height in dp of a progress bar and of the viewfinder's state line.
pub const BAR_HEIGHT: f32 = 6.0;

/// A corner bracket's arm over a camera's picture, as a share of the
/// picture's short side (`docs/DESIGN.md` §4.9, Scanner).
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

/// Stroke width in dp of the accent edge on what the keyboard is on: a
/// ⋮ it has moved to, and the row of a ⋮ menu the arrows are on
/// (`docs/DESIGN.md` §4.15, §4.17).
pub const FOCUS_STROKE: f32 = 2.0;

/// Hold-button duration in milliseconds (`docs/DESIGN.md` §4.13).
pub const HOLD_MS: u64 = 1500;
/// Milliseconds a passing state stays on a line before the line goes
/// back to what it said: "Nothing to paste" on the scanner's state line
/// (`docs/DESIGN.md` §4.9, Scanner).
pub const NOTE_FLASH_MS: u64 = 2000;
/// Milliseconds the scanner waits for the camera's first frame before
/// it says the camera sends no picture (`docs/QR.md` §8 item 6).
pub const NO_PICTURE_MS: u64 = 6000;
/// Milliseconds a QR is seen large enough and not read before the
/// scanner says how to hold it (`docs/QR.md` §8 item 8).
pub const POOR_PICTURE_MS: u64 = 4000;

/// Where the contents of a right-hand side panel (Spend, Back up, Create a
/// vault, a board's list) start: below the header's row of pills and the ?,
/// which stand over the panel's top (`docs/MODES.md` §3).
pub const SIDE_PANEL_TOP: f32 = GRID * 6.0;

/// The mode pill's height (`docs/MODES.md` §3).
pub const MODE_PILL_HEIGHT: f32 = 28.0;

/// The mode pill's inset from its edge to its label, each side.
pub const MODE_PILL_PAD: f32 = 12.0;

/// The mode sheet's widest, as the Load sheet's.
pub const MODE_SHEET_WIDTH: f32 = 640.0;

/// A row of the mode sheet, and of Settings' Mode row.
pub const MODE_ROW: f32 = 48.0;

/// The chip inside an empty entry field (Use a sample): its height.
pub const FIELD_ACTION_HEIGHT: f32 = 26.0;

/// The chip inside an empty entry field: the space between the field's
/// edge and the chip, and between the chip's edge and its label.
pub const FIELD_ACTION_INSET: f32 = 10.0;

/// The chip inside an empty entry field: the size of its label.
pub const FIELD_ACTION_TEXT: f32 = 13.0;

/// A word cell of Add a key on a wide display: its height.
pub const ENTRY_CELL_HEIGHT: f32 = 38.0;

/// Add a key's word grid on a wide display: the distance from one row to
/// the next.
pub const ENTRY_ROW: f32 = 46.0;

/// The field where one share or part is typed on a wide display: its
/// height.
pub const ENTRY_FIELD_HEIGHT: f32 = 44.0;

/// Corner radius of an entry field and its word cells.
pub const FIELD_RADIUS: f32 = 8.0;

/// The word being typed on a small panel: how far its row stands above the
/// line of its label's text, so a chip in it centres on the text.
pub const KEYED_HEAD_LIFT: f32 = 4.0;

/// A cosigner's key box in the seeds piece: its height.
pub const SEED_FIELD_HEIGHT: f32 = 40.0;

/// A cosigner's key box in the seeds piece: how far it stands in from the
/// left of the row, behind the number of its place.
pub const SEED_FIELD_INSET: f32 = 22.0;

/// The scanner's Sample tile: its height.
pub const SCAN_TILE_HEIGHT: f32 = 44.0;

/// The scanner's Sample tile: the size of its label.
pub const SCAN_TILE_TEXT: f32 = 14.0;

/// The scanner's Sample tile: the room it takes beyond its label, the
/// padding each side.
pub const SCAN_TILE_PAD: f32 = 32.0;

/// The scanner's Sample tile on a wide display: the most room its label
/// may take; a longer one is the short label.
pub const SCAN_TILE_MAX_SPAN: f32 = 210.0;

/// A step's Load sample PSBT button beside its Scan: its height, and the
/// row it takes with the gap under it.
pub const SAMPLE_BUTTON_HEIGHT: f32 = 40.0;

/// A step's Load sample PSBT button: the row it takes.
pub const SAMPLE_BUTTON_ROW: f32 = 52.0;

/// The scanner's foot: the gap between its buttons.
pub const SCAN_TILE_GAP: f32 = 12.0;

/// The scanner on a small panel: the room a full-width button above the
/// foot takes.
pub const SCAN_TILE_ROW: f32 = 52.0;

/// The scanner sheet's side margin, inside the sheet.
pub const SCAN_SIDE: f32 = 32.0;

/// Add a key's word grid on a wide display: the gap between cells.
pub const ENTRY_CELL_GAP: f32 = 10.0;

/// Add a key's word cell on a wide display: the room its number takes at
/// the left, before the word.
pub const ENTRY_CELL_NUMBER: f32 = 24.0;

/// A passphrase field that takes no typing (Demo's `faraday`): how far
/// its code box stands in from the field's left edge.
pub const PASS_CODE_INSET: f32 = 12.0;

/// A passphrase field that takes no typing: the size of the passphrase in
/// its code box.
pub const PASS_CODE_TEXT: f32 = 14.0;

/// A one-line text field a little taller than [`TEXT_FIELD`]: a
/// passphrase field on a small panel, and Check my copy's.
pub const TEXT_FIELD_TALL: f32 = 42.0;

/// A one-line text field of a sheet and of the Lightning screen's cipher
/// seed.
pub const TEXT_FIELD_SHEET: f32 = 44.0;

/// A theme tile on Settings: its height, room for a name on two lines
/// under its sample card.
pub const THEME_TILE_HEIGHT: f32 = 72.0;

/// A theme tile: the size of its name.
pub const THEME_TILE_TEXT: f32 = 12.0;

/// A theme tile: the step from one line of its name to the next.
pub const THEME_TILE_LINE: f32 = 15.0;

/// A theme tile: how far its sample card and its name stand in from its
/// edges.
pub const THEME_TILE_INSET: f32 = 10.0;

/// A theme tile: the height of its sample card.
pub const THEME_TILE_SAMPLE: f32 = 22.0;

/// A theme tile: the narrowest it is drawn, whatever its name.
pub const THEME_TILE_MIN: f32 = 96.0;

/// Theme tiles: the gap between two side by side.
pub const THEME_TILE_GAP: f32 = 8.0;

/// Theme tiles: the gap between two rows.
pub const THEME_TILE_ROW_GAP: f32 = 10.0;

/// A theme group on Settings: its label's row, from the label's top to
/// the group's first tiles.
pub const THEME_GROUP_HEAD: f32 = 22.0;

/// Theme groups: the space above each after the first.
pub const THEME_GROUP_GAP: f32 = 14.0;

/// A Guided text's Learn link: the button's height.
pub const GUIDE_LINK_H: f32 = 32.0;

/// A Guided text's Learn link: the room between the text and the button.
pub const GUIDE_LINK_ABOVE: f32 = 6.0;

/// A Guided text's Learn link: the room added to its label's width.
pub const GUIDE_LINK_PAD: f32 = 24.0;

/// A Guided text's Learn link: the size of its label.
pub const GUIDE_LINK_TEXT: f32 = 13.0;

/// A Guided text: how far in from its bar the words stand.
pub const GUIDE_INDENT: f32 = 14.0;

/// A card with its Guided text open: how far down from the card's top its
/// body starts.
pub const GUIDE_CARD_HEAD: f32 = 56.0;

/// A Guided text: the room under it.
pub const GUIDE_AFTER: f32 = 18.0;

// Window size classes (`docs/LAYOUT.md` §2, Material 3's widths and
// heights, in dp).

/// A window this wide or wider is medium, not compact.
pub const WINDOW_MEDIUM: f32 = 600.0;

/// A window this wide or wider is expanded.
pub const WINDOW_EXPANDED: f32 = 840.0;

/// A window this wide or wider is large.
pub const WINDOW_LARGE: f32 = 1200.0;

/// A window this wide or wider is extra-large.
pub const WINDOW_EXTRA_LARGE: f32 = 1600.0;

/// A window less tall than this is short (the Pi panel).
pub const WINDOW_SHORT: f32 = 480.0;

/// A window less tall than this is low (a 7-inch panel).
pub const WINDOW_LOW: f32 = 600.0;

/// The window assumed before a shell has said what its display is: the
/// desktop window's default, 1280 × 800 dp.
pub const WINDOW_DEFAULT_W: f32 = 1280.0;

/// The default window's height (`WINDOW_DEFAULT_W`).
pub const WINDOW_DEFAULT_H: f32 = 800.0;

/// Held inside the Starter, a window narrower than this takes the
/// one-column layout: the wide layout's 1280 dp page (`docs/STARTER.md`
/// §3.1d).
pub const WINDOW_HELD_WIDE: f32 = 1280.0;

// Navigation by window class (`docs/LAYOUT.md` §3, §9), Material 3's
// rail, bar and top app bar in Faraday's look.

/// The navigation rail's width, at the window's left (medium, expanded
/// and low windows).
pub const RAIL_W: f32 = 80.0;

/// The expanded rail's width (large and extra-large): the sidebar's.
pub const RAIL_EXPANDED_W: f32 = 240.0;

/// The navigation bar's height, across a compact window's foot.
pub const NAV_BAR_H: f32 = 80.0;

/// The top app bar's height, across the page.
pub const APP_BAR_H: f32 = 64.0;

/// The pill behind the current destination's icon on the rail and the
/// bar: its width.
pub const NAV_PILL_W: f32 = 56.0;

/// The pill's height (`NAV_PILL_W`).
pub const NAV_PILL_H: f32 = 32.0;

/// A rail destination's height where the rail has room: its pill, its
/// label under it and the space round them.
pub const NAV_ITEM_H: f32 = 56.0;

/// A destination's icon, drawn in its pill.
pub const NAV_ICON: f32 = 16.0;

/// The space between a destination's pill and its label.
pub const NAV_LABEL_GAP: f32 = 4.0;

/// The space above the rail's first destination and under its last.
pub const NAV_RAIL_TOP: f32 = 12.0;

/// The page's side margin on a compact window.
pub const MARGIN_COMPACT: f32 = 16.0;

/// The page's side margin on every wider window.
pub const MARGIN_WIDE: f32 = 24.0;

/// The widest a one-column page grows beside the navigation.
pub const COLUMN_MAX: f32 = 720.0;

/// A status chip's height (Home's status strip, a flow's chip).
pub const STATUS_CHIP_H: f32 = 32.0;

/// A status chip's inner padding at each end.
pub const STATUS_CHIP_PAD: f32 = 12.0;

/// The space between an icon and the text after it, on a chip and an
/// expanded rail's destination.
pub const NAV_ICON_GAP: f32 = 12.0;

// ----- Codex32 error correction (docs/CODEX32.md §8.4) -----

/// Add a key's other forms: a line of the list of shares or strings in,
/// from its tick to its text (and a line under it, a string's mark).
pub const PART_INDENT: f32 = 24.0;

/// The same on a small panel.
pub const PART_INDENT_COMPACT: f32 = 20.0;

/// Codex32 correction: a box of the string as the re-entry shows it.
pub const CORRECT_BOX: f32 = 24.0;

/// Codex32 correction: the gap between two boxes of a group of four.
pub const CORRECT_BOX_GAP: f32 = 2.0;

/// Codex32 correction: a group of four boxes, from the first box's left
/// edge to the fourth's right.
pub const CORRECT_GROUP: f32 = 4.0 * CORRECT_BOX + 3.0 * CORRECT_BOX_GAP;

/// Codex32 correction: the gap between two groups of four.
pub const CORRECT_GROUP_GAP: f32 = GRID;

/// Codex32 correction: a box's corner radius.
pub const CORRECT_BOX_RADIUS: f32 = 4.0;

/// Codex32 correction: the size of a box's character.
pub const CORRECT_BOX_TEXT: f32 = 16.0;

/// Codex32 correction: the row under a box that carries its number.
pub const CORRECT_NUMBER_ROW: f32 = 16.0;

/// Codex32 correction: the size of a box's number.
pub const CORRECT_NUMBER_TEXT: f32 = 11.0;

/// Codex32 correction: the gap between two rows of boxes.
pub const CORRECT_ROW_GAP: f32 = GAP_SMALL;

/// Codex32 correction: a pair's row, "Box 17 · q → p".
pub const CORRECT_PAIR_ROW: f32 = 28.0;

/// Codex32 correction: the size of a pair's text.
pub const CORRECT_PAIR_TEXT: f32 = 16.0;

/// Codex32 correction: the size of a line (a refusal, a caution, the
/// count typed, the line beside a fingerprint).
pub const CORRECT_LINE_TEXT: f32 = 14.0;

/// Codex32 correction: the space between the panel's parts.
pub const CORRECT_GAP: f32 = GRID;

/// Codex32 correction: the box a line's icon is centred in.
pub const CORRECT_ICON: f32 = 18.0;

/// Codex32 correction: a line's icon.
pub const CORRECT_ICON_SIZE: f32 = 11.0;

/// Codex32 correction: the tick's row, **Suggest corrections**: a
/// whole touch target.
pub const CORRECT_TICK_ROW: f32 = TOUCH;

/// Codex32 correction: the tick box's side, as `Ui::checkbox` draws it.
pub const CORRECT_TICK: f32 = 18.0;

/// Codex32 correction: from the tick box to its label.
pub const CORRECT_TICK_LABEL: f32 = 28.0;
// ----- The dice roller on Faraday Desktop (docs/CODEX32.md §5.6) -----

/// The roller's die: the side of its square.
pub const ROLLER_DIE: f32 = 52.0;

/// The roller's die: the radius of its corners.
pub const ROLLER_DIE_RADIUS: f32 = 10.0;

/// The roller's die: the radius of one pip.
pub const ROLLER_PIP: f32 = 4.5;

/// The roller's die: how far the outer pips stand from its centre.
pub const ROLLER_PIP_OFFSET: f32 = 14.0;

/// The roller's die: how high it jumps in a tumble, at the top of the
/// bounce.
pub const ROLLER_BOUNCE: f32 = 10.0;

/// The roller's die: how far it tilts sideways in a tumble, at most.
pub const ROLLER_SWAY: f32 = 6.0;

/// The roller: how long one die tumbles before its value lands, ms.
pub const ROLLER_TUMBLE_MS: u64 = 700;

/// The roller: the pause after a value lands before the next tumble
/// starts, ms, so each value can be followed.
pub const ROLLER_PAUSE_MS: u64 = 350;

/// The roller: how long a tumbling die shows one face before the next,
/// ms.
pub const ROLLER_FACE_MS: u64 = 90;

/// The roller: the longest number of rolls a typed number may name.
pub const ROLLER_COUNT_MAX: u32 = 9_999;

/// The roller: the most digits a typed number has.
pub const ROLLER_COUNT_DIGITS: usize = 4;

/// The roller: the height of its choice chips and its button.
pub const ROLLER_CHIP: f32 = 32.0;

/// The roller: the gap between its parts.
pub const ROLLER_GAP: f32 = 10.0;

/// The roller: the size of its small labels.
pub const ROLLER_LABEL: f32 = 12.0;

/// The roller: the width of the typed number's field.
pub const ROLLER_COUNT_FIELD: f32 = 64.0;
// ---------------------------------------------------------------------
// Codex32 worksheets (`docs/CODEX32.md` §6.4.7, §7.5, §13.4)
// ---------------------------------------------------------------------

/// A worksheet box's side in dp, by the window's class: 16 compact, 18
/// medium, 20 expanded and up. Under the touch floor everywhere.
pub const fn codex32_box(class: WindowClass) -> f32 {
    match class {
        WindowClass::Compact => 16.0,
        WindowClass::Medium => 18.0,
        _ => 20.0,
    }
}

/// A character's size in a worksheet box, by the window's class: room
/// round it.
pub const fn codex32_char(class: WindowClass) -> f32 {
    match class {
        WindowClass::Compact => 11.0,
        WindowClass::Medium => 12.0,
        _ => 13.0,
    }
}

/// The gap between two boxes side by side in a group of four.
pub const CODEX32_BOX_GAP: f32 = 2.0;
/// The gap between two groups of four boxes.
pub const CODEX32_GROUP_GAP: f32 = 8.0;
/// The air between two rows of the staircase.
pub const CODEX32_ROW_GAP: f32 = 4.0;
/// A box number's size, above its box.
pub const CODEX32_NUMBER: f32 = 11.0;
/// The room the box numbers take above a row.
pub const CODEX32_NUMBER_ROW: f32 = 16.0;
/// A box's corners.
pub const CODEX32_RADIUS: f32 = 4.0;
/// The heavier outline of a given box (the person's data, the header)
/// and the bolded checksum squares.
pub const CODEX32_GIVEN_STROKE: f32 = 1.5;
/// The accent outline of the cell being filled.
pub const CODEX32_FRONT_STROKE: f32 = 2.0;
/// The column of row names left of a worksheet.
pub const CODEX32_LABEL_W: f32 = 76.0;
/// The dice sheet's panel beside the checksum worksheet on `wide`.
pub const CODEX32_DICE_W: f32 = 320.0;
/// The gap between the two panels on `wide`.
pub const CODEX32_PANEL_GAP: f32 = 24.0;
/// A pad on a die's track: the free space and the values 1 to 6.
pub const CODEX32_PAD: f32 = 24.0;
/// One level of the dice sheet's tree.
pub const CODEX32_TREE_LEVEL: f32 = 16.0;
/// A node of the tree.
pub const CODEX32_TREE_NODE: f32 = 2.0;
/// The tree's path, in the accent.
pub const CODEX32_PATH_STROKE: f32 = 2.0;
/// Columns shown either side of the front on `small`'s checksum panel.
pub const CODEX32_BAND_SIDE: usize = 2;
/// A worksheet tab's height (`docs/CODEX32.md` §6.5).
pub const CODEX32_TAB: f32 = 48.0;
/// The most rows of a worksheet the entry shows round its front column.
pub const CODEX32_BAND_ROWS: usize = 16;

// ----- The five-dice roller (docs/CODEX32.md §5.5) -----

/// The five-dice roller: the side of each of its five dice.
pub const ROLLER_FIVE_DIE: f32 = 44.0;

/// The five-dice roller: the gap between two dice.
pub const ROLLER_FIVE_GAP: f32 = 8.0;

/// The five-dice roller: the most faces that land in one tick under
/// Reduce motion, so that a click rolled to the end is never one long
/// frame.
pub const ROLLER_FIVE_BATCH: usize = 400;

/// One layer of a Material 3 elevation shadow, in dp: how far it falls
/// below what floats, its blur radius and how far it grows past it.
#[derive(Clone, Copy)]
pub struct ElevationLayer {
    /// Dp below what floats.
    pub y: f32,
    /// Blur radius in dp.
    pub blur: f32,
    /// Dp grown past it on every side.
    pub spread: f32,
}

const fn layer(y: f32, blur: f32, spread: f32) -> ElevationLayer {
    ElevationLayer { y, blur, spread }
}

/// The key layer of elevation levels 1 to 5 (Material Web's
/// `elevation/internal/_elevation.scss`).
pub const ELEVATION_KEY: [ElevationLayer; 5] = [
    layer(1.0, 2.0, 0.0),
    layer(1.0, 2.0, 0.0),
    layer(1.0, 3.0, 0.0),
    layer(2.0, 3.0, 0.0),
    layer(4.0, 4.0, 0.0),
];
/// The ambient layer of elevation levels 1 to 5.
pub const ELEVATION_AMBIENT: [ElevationLayer; 5] = [
    layer(1.0, 3.0, 1.0),
    layer(2.0, 6.0, 2.0),
    layer(4.0, 8.0, 3.0),
    layer(6.0, 10.0, 4.0),
    layer(8.0, 12.0, 6.0),
];
/// Opacity (of 255) of the key layer: 30% black.
pub const ELEVATION_KEY_OPACITY: u8 = 77;
/// Opacity (of 255) of the ambient layer: 15% black.
pub const ELEVATION_AMBIENT_OPACITY: u8 = 38;
/// Elevation level of a sheet, a raised card and a board chip.
pub const ELEVATION_SHEET: u8 = 1;
/// Elevation level of a menu.
pub const ELEVATION_MENU: u8 = 2;
/// Elevation level of a floating action button, a toast and a snackbar.
pub const ELEVATION_FAB: u8 = 3;

/// A dialog's elevation level: Material 3 raises a dialog to level 3.
pub const ELEVATION_DIALOG: u8 = 3;
/// Elevation level of a floating action button under the pointer, and
/// of a dragged item.
pub const ELEVATION_RAISED: u8 = 4;
// ---------------------------------------------------------------------
// Codex32 in backups (`docs/CODEX32.md` §9)
// ---------------------------------------------------------------------

/// A filter chip's height on the checklist's codex32 item (Material 3's
/// filter chip, 32 dp), drawn in a `TOUCH` target.
pub const C32_CHIP: f32 = 32.0;

/// The check mark on a chosen filter chip (Material 3's 18 dp).
pub const C32_CHECK: f32 = 18.0;

/// How far the entry's page is scrolled at every press on a small panel, past
/// its header, so the panel and the pad are in view together (the tabs a
/// scroll up away).
pub const CODEX32_ENTER_SCROLL: f32 = 140.0;
/// The eye of the app bar: its side on the small panel's bar and on the
/// wide page's top-right row, the gap to the button beside it, how far
/// its press area reaches round it, its corner, its glyph, and the top of
/// the wide row's.
pub const EYE_SIDE: f32 = 34.0;
/// The eye's side on the wide page.
pub const EYE_WIDE_SIDE: f32 = 28.0;
/// The gap to the eye's neighbour on the bar.
pub const EYE_GAP: f32 = 6.0;
/// The gap to the eye's neighbour on the wide row.
pub const EYE_WIDE_GAP: f32 = 8.0;
/// How far the eye's press area reaches round it.
pub const EYE_HIT: f32 = 6.0;
/// The eye's corner.
pub const EYE_RADIUS: f32 = 8.0;
/// The eye's glyph.
pub const EYE_GLYPH: f32 = 14.0;
/// The top of the wide row's eye.
pub const EYE_WIDE_TOP: f32 = 12.0;
/// The room a wide row's keyboard button takes, which the eye stands left of.
pub const EYE_KEYBOARD_ROOM: f32 = 36.0;
/// The check glyph in a seen worksheet tab.
pub const CODEX32_TAB_GLYPH: f32 = 12.0;
/// The icon's room at the head of the cell sheet.
pub const CODEX32_SHEET_ICON: f32 = 28.0;
/// The check icon in a seen worksheet tab.
pub const CODEX32_TAB_ICON: f32 = 20.0;
/// A tab's active indicator, Material's 3 dp.
pub const CODEX32_INDICATOR: f32 = 3.0;
