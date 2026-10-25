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
/// What a flow's heading block grows by when its chip does not fit
/// beside the heading and sits on its own line under it.
pub const CHIP_LINE: f32 = 36.0;
/// The gap a dense list uses, where a full grid step would cost a row.
pub const GAP_SMALL: f32 = 4.0;
/// How far a marked run of monospace text is drawn again to the right to read heavier (the face has one weight).
pub const MARK_BOLD: f32 = 0.5;
/// Screen padding: the margin between the content and the screen edge.
pub const PAD: f32 = 16.0;
/// Corner radius of cards, rows and tiles: Material's medium.
pub const RADIUS: f32 = SHAPE_M;
/// Corner radius of the small rectangles: chips, candidate cells, keys:
/// Material's small.
pub const RADIUS_SMALL: f32 = SHAPE_S;

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
pub const FOCUS_STROKE: f32 = FOCUS_RING;

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

/// Corner radius of an entry field and its word cells: Material's
/// extra-small.
pub const FIELD_RADIUS: f32 = SHAPE_XS;

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

/// A filter chip (Material's): its height. Settings' theme chips
/// (`docs/THEMES.md` §12).
pub const FILTER_CHIP_H: f32 = 32.0;

/// A filter chip: the room before its leading icon.
pub const FILTER_CHIP_LEAD: f32 = 8.0;

/// A filter chip: the room after its label.
pub const FILTER_CHIP_TRAIL: f32 = 16.0;

/// A filter chip: its leading icon, a theme chip's swatch.
pub const FILTER_CHIP_ICON: f32 = 18.0;

/// Filter chips: the gap between two, side by side and row to row.
pub const FILTER_CHIP_GAP: f32 = 8.0;

/// A theme chip's swatch: the radius of the primary dot inside it.
pub const THEME_SWATCH_DOT: f32 = 4.0;

/// A theme chip's swatch: the tick drawn in it on the chosen theme.
pub const THEME_SWATCH_TICK: f32 = 14.0;

/// A segmented button's height (Material's): Settings' Mode and
/// Contrast.
pub const SEGMENTED_H: f32 = 40.0;

/// Settings' Appearance rows on the wide layout: the width of the
/// labels' column, the controls to its right.
pub const APPEARANCE_LABEL_W: f32 = 128.0;

/// Settings' Appearance rows: the gap between one row and the next.
pub const APPEARANCE_ROW_GAP: f32 = 16.0;

/// Settings' Appearance rows on a small panel: the gap from a row's
/// label to its control under it.
pub const APPEARANCE_LABEL_GAP: f32 = 8.0;

/// A switch (Material's): its track's width.
pub const SWITCH_W: f32 = 52.0;

/// A switch: its track's height.
pub const SWITCH_H: f32 = 32.0;

/// A switch: its track's edge while off.
pub const SWITCH_EDGE: f32 = 2.0;

/// A switch: its handle's diameter while off.
pub const SWITCH_HANDLE_OFF: f32 = 16.0;

/// A switch: its handle's diameter while on.
pub const SWITCH_HANDLE_ON: f32 = 24.0;

/// A switch: its handle's diameter while pressed.
pub const SWITCH_HANDLE_PRESSED: f32 = 28.0;

/// A switch: the state layer round its handle, its diameter.
pub const SWITCH_STATE: f32 = 40.0;

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

/// A row of Settings' choices, its height.
pub const SETTINGS_CHOICE_H: f32 = 38.0;

/// The space above a value under a row of choices.
pub const SETTINGS_VALUE_GAP: f32 = 12.0;

/// A value under a row of choices, its text size.
pub const SETTINGS_VALUE_TEXT: f32 = 13.0;

/// A value's line under a row of choices.
pub const SETTINGS_VALUE_LINE: f32 = 16.0;

/// The screens the desktop app can show its screens at
/// (`docs/PREVIEW-SIZE.md`), width and height in dp, in Settings'
/// order: the Pi panel, a narrow window, a half-screen window, a 7-inch
/// panel, a 10-inch panel, the desktop window, a full monitor.
pub const PREVIEW_SIZES: [(u16, u16); 7] = [
    (268, 358),
    (400, 800),
    (760, 1000),
    (960, 580),
    (1370, 860),
    (1280, 800),
    (1920, 1080),
];

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

/// Material's small badge on a destination's icon: a dot this wide.
pub const BADGE_SMALL: f32 = 6.0;

/// Material's large badge on a destination's icon: its height, and its
/// least width.
pub const BADGE_LARGE: f32 = 16.0;

/// The large badge's room each side of its label.
pub const BADGE_PAD: f32 = 4.0;

/// The large badge's label: Material's label small.
pub const BADGE_LABEL: f32 = 11.0;

/// The page's side margin on a compact window.
pub const MARGIN_COMPACT: f32 = 16.0;

/// The page's side margin on every wider window.
pub const MARGIN_WIDE: f32 = 24.0;

/// The widest a one-column page grows beside the navigation.
pub const COLUMN_MAX: f32 = 720.0;

/// The width of the area a reading page is laid out in, centred in the
/// room beside the navigation once that room is wider than the area
/// (`docs/LAYOUT.md` §14): a 900 dp column and its side insets.
pub const READING_PAGE_W: f32 = 1000.0;
/// Set the time's reading area: its 520 dp column between two 56 dp insets.
pub const READING_TIME_W: f32 = 520.0 + 2.0 * 56.0;
/// Add randomness' reading area: its `COLUMN_MAX` column between two
/// 48 dp insets.
pub const READING_RANDOM_W: f32 = COLUMN_MAX + 4.0 * MARGIN_WIDE;
/// Explore's inset from its area's edges on a wide window.
pub const EXPLORE_INSET: f32 = 56.0;
/// Explore's reading area: its `COLUMN_MAX` column between two
/// `EXPLORE_INSET`s (`docs/LAYOUT.md` §17).
pub const READING_EXPLORE_W: f32 = COLUMN_MAX + 2.0 * EXPLORE_INSET;
/// The narrowest a derivation path's field is drawn on a wide window
/// (`docs/LAYOUT.md` §8): room for `m/86h/1h/0h/1/2147483647` and the
/// caret at the body size; a longer path widens it, up to the column.
pub const PATH_FIELD_W: f32 = 320.0;
/// A transaction table's first column, the row's name (Input, Output).
pub const TABLE_LABEL_W: f32 = 64.0;
/// A transaction table's last column, the amount, and the gap before it.
pub const TABLE_AMOUNT_W: f32 = 110.0;
/// The area a page that has no wide layout yet keeps on a wide window:
/// the room the 1600 dp frame gave it, centred beside the navigation,
/// until its batch gives it the work width (`docs/LAYOUT.md` §17, §19).
pub const PAGE_AREA_UNTIL_W: f32 = 1360.0;

/// The space between two panes (`docs/LAYOUT.md` §5): Material's spacer.
pub const SPACER: f32 = 24.0;

/// A list pane's width in list and detail (`docs/LAYOUT.md` §5):
/// Material's default pane width.
pub const LIST_PANE: f32 = 360.0;

/// Where a list pane and its detail begin under a wide page's title: the
/// title's 28 dp top and its 56 dp row.
pub const PANE_TOP: f32 = 84.0;

/// Where a list pane and its detail begin under the top app bar, on the
/// rail's windows: Material's list padding.
pub const PANE_TOP_BAR: f32 = 8.0;

/// A button's height: Material's common buttons (`docs/LAYOUT.md` §8).
pub const BUTTON_H: f32 = 40.0;

/// A one-line list item's height (Material 3 Lists).
pub const LIST_1: f32 = 56.0;

/// A two-line list item's height (Material 3 Lists).
pub const LIST_2: f32 = 72.0;

/// A list item's room at its start and end (Material 3 Lists).
pub const LIST_PAD: f32 = 16.0;

/// A list item's headline: Material's body large.
pub const LIST_HEADLINE: f32 = 16.0;

/// A list item's supporting text: Material's body medium.
pub const LIST_SUPPORTING: f32 = 14.0;

/// A list item's trailing text: Material's label small.
pub const LIST_TRAILING: f32 = 11.0;

/// The headline's top in a two-line list item.
pub const LIST_HEADLINE_Y: f32 = 14.0;

/// The supporting text's top in a two-line list item.
pub const LIST_SUPPORTING_Y: f32 = 38.0;

/// A supporting pane's width (`docs/LAYOUT.md` §6).
pub const SUPPORT_PANE: f32 = 400.0;

/// The narrowest a page is laid out beside the "Demo only" card at the
/// window's right (`docs/DEMO.md` §14); narrower, the card is a strip at
/// the window's foot.
pub const DEMO_PAGE_MIN: f32 = 420.0;

/// Where the "Demo only" card's top stands on a wide window: just under
/// the top-right strip's pills (owner, 2026-10-24); under the eye too
/// where the codex32 sheets show one there.
pub const DEMO_CARD_TOP: f32 = TOUCH + GAP_SMALL;

/// The same for a page with a side panel of its own beside its step
/// column (Create a vault, Back up, Spend, the backup flow): Material's
/// expanded width, so the column and the panel keep their room.
pub const DEMO_PAGE_PANEL_MIN: f32 = 840.0;

/// Where a supporting pane's top stands on a wide window: below the
/// top-right strip's two 48 dp rows (the network pill and the ?, then
/// the eye), never level with them (owner, 2026-10-24).
pub const SUPPORT_PANE_TOP: f32 = 2.0 * TOUCH;

/// The input panel's width where it stands beside the sheets
/// (`docs/LAYOUT.md` §16.3): the codex32 entry's rolls, dice and pad.
pub const CODEX32_ENTRY_W: f32 = 424.0;
/// A key of the input panel's pad: its width; its height is `TOUCH`
/// (`docs/LAYOUT.md` §8, §16.3).
pub const PAD_KEY_W: f32 = 64.0;
/// The most keys in one row of the input panel's pad.
pub const PAD_KEYS_ROW_MAX: usize = 10;
/// The most of the window's height the input panel takes at the foot;
/// a taller one stands in the page (`docs/LAYOUT.md` §16.3).
pub const INPUT_PANEL_MAX: f32 = 0.4;
/// The rolls laid across the input panel at the window's foot: the
/// column of the row names, "First roll" and "Second roll".
pub const INPUT_ROLLS_LABEL_W: f32 = 80.0;
/// A die's column of the rolls across the input panel.
pub const INPUT_ROLLS_COL: f32 = 68.0;
/// The line a 12 dp label takes in the rolls across the input panel.
pub const INPUT_ROLLS_LINE: f32 = 16.0;
/// The struck pairs the rolls across the input panel keep room for,
/// the latest first.
pub const INPUT_ROLLS_STRUCK: usize = 2;
/// The room the dice's six sizes take as one segmented row in the input
/// panel; narrower, they wrap onto two rows.
pub const INPUT_SIZES_W: f32 = 512.0;

/// The narrowest a Home tile is; the grid takes as many columns as fit
/// (`docs/LAYOUT.md` §4).
pub const TILE_MIN: f32 = 200.0;

/// The widest Home's lead tile is (`docs/LAYOUT.md` §4).
pub const LEAD_MAX: f32 = 560.0;

/// A Home tile's height on the wide Home.
pub const TILE_H: f32 = 84.0;

/// A Home tile's height in the compact Home's grid (icon beside name).
pub const TILE_H_GRID: f32 = 60.0;

/// The wide Home tile's icon box.
pub const HOME_TILE_ICON: f32 = 40.0;
/// The wide Home tile's icon inset from the tile's left edge.
pub const HOME_TILE_INSET: f32 = 18.0;
/// The left edge of the wide Home tile's text.
pub const HOME_TILE_TEXT_X: f32 = 74.0;

/// A grid tile's inset on the compact Home.
pub const GRID_TILE_INSET: f32 = 12.0;
/// The left edge of a grid tile's text on the compact Home.
pub const GRID_TILE_TEXT_X: f32 = 54.0;

/// A Home tile's height on the small panel's menu.
pub const TILE_H_COMPACT: f32 = 76.0;

/// The widest a Starter page's content grows, centred in the space beside
/// the rail (`docs/STARTER.md` §3.1d).
pub const CONTENT_MAX: f32 = 1040.0;

/// The narrowest a Starter Home card is: a wider window gains columns of
/// at least this, never wider cards (`docs/STARTER.md` §3.1d).
pub const CARD_MIN_W: f32 = 300.0;

/// A status chip's height (Home's status strip, a flow's chip).
pub const STATUS_CHIP_H: f32 = 32.0;

/// A status chip's inner padding at each end.
pub const STATUS_CHIP_PAD: f32 = 12.0;

/// The space between an icon and the text after it, on a chip and an
/// expanded rail's destination.
pub const NAV_ICON_GAP: f32 = 12.0;

// ----- Add randomness (docs/OWN-RANDOMNESS.md §2.2) -----

/// A button on Add randomness, on Settings' Randomness card and New
/// key's link to it: Material's 40 dp container, pressed in a `TOUCH`
/// target.
pub const OWN_BUTTON: f32 = 40.0;

/// Add randomness's way back: from the page's top to its row.
pub const OWN_TOP: f32 = 28.0;

/// Add randomness: the size of the status line and the batch's line.
pub const OWN_LINE_TEXT: f32 = 13.0;

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
/// The gap between the two panels on `wide`.
pub const CODEX32_PANEL_GAP: f32 = 24.0;
/// The five-dice roller's column beside the codex32 dice pad (CODEX32
/// §5.5): five dice and their gaps, and room for the reach chips.
pub const CODEX32_ROLLER_W: f32 = 320.0;
/// The narrowest the codex32 entry's own column is drawn beside the
/// "Demo only" box (CODEX32 §5.5); narrower, the box goes under it.
pub const CODEX32_MAIN_MIN: f32 = 420.0;
/// How far New key's column stands in from its page area's left edge,
/// and as far again is kept on its right.
pub const KEYGEN_COLUMN_IN: f32 = 40.0;
/// The widest New key's column of cards grows: reading content. Wide
/// content in it (the codex32 sheets) takes the work width
/// (`docs/LAYOUT.md` §14).
pub const KEYGEN_COLUMN_MAX: f32 = 900.0;
/// A codex32 card's segmented button (Material 3's 40 dp), in a `TOUCH`
/// row.
pub const CODEX32_CHOICE_H: f32 = 40.0;
// The dice de-biasing sheet's die tracks, drawn as the booklet draws
// them (SSS32.ps, `drawDiceRow`): in units of the diamond pad's
// half-diagonal (the booklet's `sz2`, 38.2 pt), each measure the
// booklet's own over 38.2.
/// The most a track unit grows to, in dp; less where the card is narrow.
pub const CODEX32_TRACK_UNIT: f32 = 20.0;
/// A track's width in units: the free space to the last hexagon's edge.
pub const CODEX32_TRACK_W: f32 = 18.8;
/// The highest point of a track above its low pads' centre line, in
/// units (a raised diamond's top); the lowest is one unit below it.
pub const CODEX32_TRACK_TOP: f32 = 2.05;
/// The gap the booklet leaves between pads, in units (2 pt).
pub const CODEX32_TRACK_GAP: f32 = 0.052;
/// The free space's centre across and up, and its radius, in units.
pub const CODEX32_FREE_X: f32 = 0.94;
/// The free space's centre, up from the low pads' centre line.
pub const CODEX32_FREE_Y: f32 = 1.15;
/// The free space's radius.
pub const CODEX32_FREE_R: f32 = 0.79;
/// A d7+ pad: the hexagon's side, in units (the booklet's 0.8).
pub const CODEX32_HEX_SIDE: f32 = 0.8;
/// A hexagon's width, its side times the root of three.
pub const CODEX32_HEX_W: f32 = 1.386;
/// Where hexagon 0 would stand: across, and down from the centre line.
pub const CODEX32_HEX_X: f32 = 2.5;
/// Hexagon 0's lower corner, from the low pads' centre line.
pub const CODEX32_HEX_Y: f32 = -0.473;
/// The step from one hexagon to the next.
pub const CODEX32_HEX_PITCH: f32 = 0.745;
/// How far an even hexagon is raised: a side and a half and a gap.
pub const CODEX32_HEX_RAISE: f32 = 1.252;
/// Where the booklet sets "Die pads" and "Die pads (d7+)" over a track,
/// in units across (160 pt and 440 pt).
pub const CODEX32_PADS_HEAD_X: f32 = 4.19;
/// Where "Die pads (d7+)" stands.
pub const CODEX32_PADS_LARGE_HEAD_X: f32 = 11.52;
/// A die's marker on its pad: a ring this many units across.
pub const CODEX32_MARKER: f32 = 1.1;
/// A die standing on its pad: a square this many units across.
pub const CODEX32_TRACK_DIE: f32 = 0.95;
/// The ring of a marker.
pub const CODEX32_MARKER_STROKE: f32 = 2.0;
/// The column before a track: the die's name and its bit.
pub const CODEX32_TRACK_LABEL_W: f32 = 52.0;
/// The space between two tracks.
pub const CODEX32_TRACK_SPACE: f32 = 6.0;
/// The narrowest the tree is drawn beside each die's two rolls; under
/// them where there is less room.
pub const CODEX32_TREE_MIN_W: f32 = 300.0;
/// The widest the dice sheet's tree is drawn (the booklet's, 420 pt).
pub const CODEX32_TREE_MAX_W: f32 = 420.0;
/// One level of the tree over the space between two leaves (the
/// booklet: 60 pt over 13.1 pt).
pub const CODEX32_TREE_LEVEL: f32 = 4.57;
/// A node of the tree.
pub const CODEX32_TREE_NODE: f32 = 2.0;
/// The tree's path, in the accent.
pub const CODEX32_PATH_STROKE: f32 = 2.0;
/// Columns shown either side of the front on `small`'s checksum panel.
pub const CODEX32_BAND_SIDE: usize = 2;
/// A worksheet tab's height (`docs/CODEX32.md` §6.5).
pub const CODEX32_TAB: f32 = 48.0;
/// A cell of the checksum worksheet, drawn as the booklet draws it:
/// squares side by side on shared rules (`docs/CODEX32.md` §7.1, §7.5),
/// by the window's class.
pub const fn codex32_cell(class: WindowClass) -> f32 {
    match class {
        WindowClass::Compact => 20.0,
        WindowClass::Medium => 24.0,
        _ => 28.0,
    }
}

/// A character's size in a checksum worksheet cell, by the window's
/// class.
pub const fn codex32_cell_char(class: WindowClass) -> f32 {
    match class {
        WindowClass::Compact => 12.0,
        WindowClass::Medium => 14.0,
        _ => 16.0,
    }
}

/// The least cell at which a checksum worksheet is drawn whole, without
/// a view (`docs/LAYOUT.md` §16.1): the size compact windows draw.
pub const CODEX32_CELL_FIT_MIN: f32 = 20.0;
/// The largest cell a checksum worksheet drawn whole grows to
/// (`docs/LAYOUT.md` §16.1).
pub const CODEX32_CELL_FIT_MAX: f32 = 32.0;
/// A step between two fitted cell sizes: a whole dp.
pub const CODEX32_CELL_FIT_STEP: f32 = 1.0;
/// A checksum worksheet cell's character as a part of the cell: four
/// sevenths, 16 at 28 dp.
pub const CODEX32_CELL_CHAR_PART: f32 = 4.0 / 7.0;

/// The least side of a string's box beside the worksheets
/// (`docs/LAYOUT.md` §16.2).
pub const CODEX32_STRING_BOX_MIN: f32 = 24.0;
/// The largest side of a string's box (`docs/LAYOUT.md` §16.2).
pub const CODEX32_STRING_BOX_MAX: f32 = 40.0;
/// A string's boxes a row where the room holds them, in groups of four,
/// longest first (`docs/LAYOUT.md` §16.2).
pub const CODEX32_STRING_ROWS: [usize; 5] = [24, 16, 12, 8, 4];
/// A string box's character as a part of the box.
pub const CODEX32_STRING_CHAR_PART: f32 = 0.6;
/// A string box's number as a part of the box, never under `TEXT_MIN`.
pub const CODEX32_STRING_NUMBER_PART: f32 = 0.4;

/// The tallest the checksum worksheet's view is before the sheet
/// scrolls inside it, by the window's class.
pub const fn codex32_sheet_view(class: WindowClass) -> f32 {
    match class {
        WindowClass::Compact => 200.0,
        WindowClass::Medium => 320.0,
        _ => 400.0,
    }
}

/// The checksum worksheet's gap between two groups of four columns, and
/// between two pairs of rows: the booklet's double rule.
pub const CODEX32_SHEET_GAP: f32 = 4.0;
/// The checksum worksheet's bold rule round a given cell (the header, a
/// character brought down): centred on the cell's thin rule.
pub const CODEX32_SHEET_BOLD: f32 = 2.0;
/// A box number in a bolded cell's top right corner, and a row letter in
/// an `=` row's leading cells' bottom left corner, as small as the
/// booklet sets them.
pub const CODEX32_CELL_NOTE: f32 = 7.0;
/// How far a box number's line rises above its cell's top, so its
/// digits sit in the corner clear of the cell's character.
pub const CODEX32_CELL_NOTE_RISE: f32 = 2.0;
/// How far a box number or row letter sits in from its cell's edge.
pub const CODEX32_CELL_NOTE_INSET: f32 = 2.0;
/// The room between a row's `+` or `=` and its first cell.
pub const CODEX32_MARK_GAP: f32 = 4.0;
/// The number above each of a string's boxes: smaller than the boxes'
/// characters, so every box carries one without crowding them.
pub const CODEX32_STRING_NUMBER: f32 = 9.0;

// ----- The five-dice roller (docs/CODEX32.md §5.5) -----

/// The five-dice roller: the side of each of its five dice.
pub const ROLLER_FIVE_DIE: f32 = 44.0;

/// The five-dice roller: the gap between two dice.
pub const ROLLER_FIVE_GAP: f32 = 8.0;

/// Each die's two rolls side by side (CODEX32 §5.5): a marker's or a
/// die's side.
pub const ROLLER_PAIR_DIE: f32 = 32.0;
/// A column of the pairs: the first roll's, the second roll's.
pub const ROLLER_PAIR_COL: f32 = 76.0;
/// The die's name before its pair.
pub const ROLLER_PAIR_LABEL_W: f32 = 48.0;
/// A die's colour on its chip: the swatch's radius.
pub const CODEX32_SWATCH: f32 = 7.0;
/// A die's colour chip: Material's assist chip, 32 dp in a 48 dp row.
pub const CODEX32_CHIP_H: f32 = 32.0;

/// The five-dice roller: the most faces that land in one tick under
/// Reduce motion, so that a click rolled to the end is never one long
/// frame.
pub const ROLLER_FIVE_BATCH: usize = 400;

/// The codex32 step-through's Play (CODEX32 §5.5): the time between two
/// steps at Slow, a step a second.
pub const CODEX32_WALK_SLOW_MS: u64 = 1000;
/// At Normal, the default: slow enough to read a step's line.
pub const CODEX32_WALK_NORMAL_MS: u64 = 400;
/// At Fast.
pub const CODEX32_WALK_FAST_MS: u64 = 120;
/// The most of a window's height the codex32 "Demo only" card takes when
/// it is opened from its strip at the foot of a narrow window.
pub const CODEX32_STRIP_OPEN_MAX: f32 = 0.85;
/// The furthest along Material's standard curve a following move of the
/// codex32 worksheet's view enters it when it takes over from a move
/// under way, so the move left is never cut to nothing (`docs/MOTION.md`
/// §3.5).
pub const SHEET_FOLLOW_ENTRY_MAX: f32 = 0.9;

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

/// A Codex32QR to copy by hand (`docs/CODEX32.md` §9.5): the room beside
/// and above the whole grid for its blocks' letters and numbers.
pub const C32_GRID_LABEL: f32 = 16.0;
/// The whole grid's largest side; a narrower column draws it smaller.
pub const C32_GRID_MAX: f32 = 320.0;
/// The block shown large: its largest side, the block and a square of
/// its neighbours on each side.
pub const C32_ZOOM_MAX: f32 = 320.0;
/// The smallest square of the block shown large: below it the large view
/// goes under the whole grid rather than beside it.
pub const C32_ZOOM_MIN_CELL: f32 = 24.0;

/// How far the entry's page is scrolled at every press on a small panel, past
/// its header, so the panel and the pad are in view together (the tabs a
/// scroll up away).
pub const CODEX32_ENTER_SCROLL: f32 = 140.0;
/// Material's icon button: its container, round, drawn in a `TOUCH`
/// target with an `ICON_GLYPH` icon (the codex32 sheets' eye).
pub const ICON_BUTTON: f32 = 40.0;
/// A stepper's label (`faraday_ui`'s `Ui::stepper`), body small.
pub const STEPPER_LABEL_TEXT: f32 = 13.0;
/// A stepper's count between its − and +, title medium.
pub const STEPPER_VALUE_TEXT: f32 = 18.0;
/// The room for a stepper's count between its two icon buttons.
pub const STEPPER_COUNT_W: f32 = 44.0;
/// How far in from a stepper's label its − stands on the wide layout,
/// when the label is shorter: the steppers of one card line up.
pub const STEPPER_LABEL_COL: f32 = 180.0;
/// The height of the window's top band, where a press on empty room moves
/// the window: Material's top app bar.
pub const TOP_BAND: f32 = 56.0;

/// The wide layout's ?: its left edge in from the window's right, where
/// the top-right row of the mode pill and the ? begins; the ? is 28 dp
/// across and stands `MARGIN_WIDE` from the window's edge
/// (`docs/LAYOUT.md` §15).
pub const TOP_ROW_QUESTION: f32 = MARGIN_WIDE + 28.0;
/// The middle of the wide layout's top-right row: the ? stands 12 dp
/// from the top and is 28 dp tall, and what else stands in the row (Files'
/// stick chip) is centred on it (`docs/LAYOUT.md` §17).
pub const TOP_ROW_MID: f32 = 12.0 + 14.0;
/// The wide page's eye: its container's right edge in from the window's,
/// under the ? at the top right, level with it at `MARGIN_WIDE`
/// (`docs/LAYOUT.md` §15, `docs/CODEX32.md` §6.4.6).
pub const EYE_WIDE_INSET: f32 = MARGIN_WIDE;
/// The check glyph in a seen worksheet tab.
pub const CODEX32_TAB_GLYPH: f32 = 12.0;
/// The icon's room at the head of the cell sheet.
pub const CODEX32_SHEET_ICON: f32 = 28.0;
/// The check icon in a seen worksheet tab.
pub const CODEX32_TAB_ICON: f32 = 20.0;
/// A tab's active indicator, Material's 3 dp.
pub const CODEX32_INDICATOR: f32 = 3.0;

// ---------------------------------------------------------------------
// Material 3: shape, state layers, focus, targets, icons, motion
// (`docs/DESIGN.md` §3, `docs/decisions/F108.md`)
// ---------------------------------------------------------------------

/// Material's corner scale: none.
pub const SHAPE_NONE: f32 = 0.0;
/// Material's corner scale: extra-small, a menu, a field, the toast.
pub const SHAPE_XS: f32 = 4.0;
/// Material's corner scale: small, a chip.
pub const SHAPE_S: f32 = 8.0;
/// Material's corner scale: medium, a card.
pub const SHAPE_M: f32 = 12.0;
/// Material's corner scale: large, the floating action.
pub const SHAPE_L: f32 = 16.0;
/// Material's corner scale: extra-large, a dialog and a sheet.
pub const SHAPE_XL: f32 = 28.0;
/// Material's corner scale: full, a button, a segmented button, a tag:
/// round ends on a shape `h` tall.
pub fn shape_full(h: f32) -> f32 {
    h / 2.0
}
/// A checkbox's corners: Material's checkbox, which is under the scale.
pub const SHAPE_CHECKBOX: f32 = 2.0;

/// A state layer's opacity while the pointer is over a control: the
/// control's content colour laid over it, so it reads on any fill.
pub const STATE_HOVER: f32 = 0.08;
/// A state layer's opacity while the keyboard is on a control
/// (Material 3's states guidance, `docs/THEMES.md` §8).
pub const STATE_FOCUS: f32 = 0.10;
/// A state layer's opacity while a control is pressed.
pub const STATE_PRESS: f32 = 0.10;
/// A state layer's opacity while a thing is dragged.
pub const STATE_DRAG: f32 = 0.16;
/// The content of a control that is not available now: on-surface at
/// this opacity. It keeps its kind and shape (F108).
pub const DISABLED_CONTENT: f32 = 0.38;
/// The container of a control that is not available now, and an
/// outlined one's edge: on-surface at this opacity.
pub const DISABLED_CONTAINER: f32 = 0.12;
/// In a high contrast theme, a theme colour filled at under this
/// opacity (of 255) is a tint under what it holds, and is drawn as its
/// edge instead; at this or more it is a mark, drawn at full strength.
pub const HIGH_CONTRAST_TINT_MAX: u8 = 96;

/// An opacity, 0 to 1, as the 0–255 a colour carries.
pub fn alpha_of(opacity: f32) -> u8 {
    (opacity.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// The focus ring's stroke: round what the keyboard is on.
pub const FOCUS_RING: f32 = 3.0;
/// How far outside its control the focus ring stands.
pub const FOCUS_OFFSET: f32 = 2.0;

/// An icon's glyph where it stands alone: a list row, an icon button, a
/// field, a dialog's head, the floating action.
pub const ICON_GLYPH: f32 = 24.0;
/// An icon's glyph in a dense place: a small panel's row.
pub const ICON_GLYPH_SMALL: f32 = 20.0;
/// An icon's glyph in a button, a chip and a chosen segment.
pub const ICON_GLYPH_BUTTON: f32 = 18.0;
/// The gap between a button's icon and its label.
pub const ICON_GAP: f32 = GAP;

/// The smallest text drawn: Material's label-small. A size asked for
/// under it is drawn at it.
pub const TEXT_MIN: f32 = 11.0;

/// A segmented button's room each side of a segment's label.
pub const SEGMENT_PAD: f32 = 12.0;

/// A dialog's padding inside its edge.
pub const DIALOG_PAD: f32 = 24.0;
/// A dialog's narrowest, on a display that has the room.
pub const DIALOG_MIN: f32 = 280.0;
/// A dialog's widest.
pub const DIALOG_MAX: f32 = 560.0;
/// The gap between a dialog's actions at its end.
pub const DIALOG_ACTION_GAP: f32 = GAP;

/// The toast's height, one line (Material's snackbar).
pub const TOAST_H: f32 = 48.0;
/// The toast's room each side of its line.
pub const TOAST_PAD: f32 = 16.0;
/// How long the toast stays on screen, ms.
pub const TOAST_SHOW_MS: u64 = 4000;

/// A plain tooltip's widest (Material 3's plain tooltip).
pub const TIP_PLAIN_MAX_W: f32 = 200.0;
/// A plain tooltip's least height: one line of body small and its room.
pub const TIP_PLAIN_MIN_H: f32 = 24.0;
/// A plain tooltip's room each side of its text.
pub const TIP_PLAIN_PAD_X: f32 = 8.0;
/// A plain tooltip's room above and below its text.
pub const TIP_PLAIN_PAD_Y: f32 = 4.0;
/// A plain tooltip's text: Material's body small.
pub const TIP_PLAIN_TEXT: f32 = 12.0;
/// A rich tooltip's widest (Material 3's rich tooltip).
pub const TIP_RICH_MAX_W: f32 = 320.0;
/// A rich tooltip's room each side of its text.
pub const TIP_RICH_PAD_X: f32 = 16.0;
/// A rich tooltip's room above and below its text.
pub const TIP_RICH_PAD_Y: f32 = 12.0;
/// A rich tooltip's supporting text: Material's body medium.
pub const TIP_RICH_TEXT: f32 = 14.0;
/// A rich tooltip's elevation level.
pub const TIP_RICH_LEVEL: u8 = 2;
/// The gap between a tooltip and what it is about.
pub const TIP_GAP: f32 = 4.0;
/// The pointer's rest on, or the keyboard's stay on, what a tooltip is
/// about before it shows, ms (Material's tooltip show delay).
pub const TIP_DELAY_MS: u64 = 500;
/// A finger held this long on what a tooltip is about shows it: a long
/// press, ms.
pub const TIP_LONG_PRESS_MS: u64 = 500;

/// The scrim over a sheet's frosted page: Material's 32 %.
pub const SCRIM_ALPHA: u8 = 82;

/// A progress indicator's track and bar.
pub const PROGRESS_H: f32 = 4.0;
/// One pass of the indeterminate indicator's bar along its track, ms.
pub const PROGRESS_CYCLE_MS: u64 = 2000;
/// The indeterminate indicator's bar, as a share of its track.
pub const PROGRESS_SPAN: f32 = 0.3;

/// Material's durations, ms: short 1 to 4.
pub const MOTION_SHORT1: u64 = 50;
/// Short 2.
pub const MOTION_SHORT2: u64 = 100;
/// Short 3: a switch's slide.
pub const MOTION_SHORT3: u64 = 150;
/// Short 4: a fade between screens, the toast coming in.
pub const MOTION_SHORT4: u64 = 200;
/// Medium 1.
pub const MOTION_MEDIUM1: u64 = 250;
/// Medium 2: a step card opening, which moves much of the page.
pub const MOTION_MEDIUM2: u64 = 300;
/// Medium 3: a change of theme, which changes every pixel.
pub const MOTION_MEDIUM3: u64 = 350;
/// Medium 4.
pub const MOTION_MEDIUM4: u64 = 400;
/// Long 1.
pub const MOTION_LONG1: u64 = 450;
/// Long 2.
pub const MOTION_LONG2: u64 = 500;
/// Long 3.
pub const MOTION_LONG3: u64 = 550;
/// Long 4.
pub const MOTION_LONG4: u64 = 600;
/// Extra-long 1.
pub const MOTION_EXTRA_LONG1: u64 = 700;
/// Extra-long 2.
pub const MOTION_EXTRA_LONG2: u64 = 800;
/// Extra-long 3.
pub const MOTION_EXTRA_LONG3: u64 = 900;
/// Extra-long 4.
pub const MOTION_EXTRA_LONG4: u64 = 1000;

/// An easing curve: a cubic Bézier's two control points, x1, y1, x2,
/// y2, from (0, 0) to (1, 1), as CSS writes one.
pub type Easing = [f32; 4];
/// Material's standard easing: what moves on screen and stays there.
pub const EASE_STANDARD: Easing = [0.2, 0.0, 0.0, 1.0];
/// Standard decelerate: what comes onto the screen.
pub const EASE_STANDARD_DECELERATE: Easing = [0.0, 0.0, 0.0, 1.0];
/// Standard accelerate: what leaves the screen.
pub const EASE_STANDARD_ACCELERATE: Easing = [0.3, 0.0, 1.0, 1.0];
/// Emphasized decelerate: a large part of the page coming in, a step
/// card opening.
pub const EASE_EMPHASIZED_DECELERATE: Easing = [0.05, 0.7, 0.1, 1.0];
/// Emphasized accelerate: a large part of the page leaving.
pub const EASE_EMPHASIZED_ACCELERATE: Easing = [0.3, 0.0, 0.8, 0.15];
