//! Every named dimension (09 §3.2).
// The dimensions the copied crates read, with `main`'s values from `core/faraday-draw/src/tokens.rs`
// (`faraday-paper`'s sheets, `faraday-scanner`'s preview); the rest are named with the kit's copy.

/// Largest short side in pixels of the camera preview the scanner keeps
/// and draws (`docs/DESIGN.md` §4.9, Scanner). The largest viewfinder is
/// [`VIEWFINDER_MAX`] dp on `wide` at 160 dpi, 360 px, and the reference
/// panel's is 286 px, so a frame reduced to this covers every square
/// without carrying pixels no display can show.
pub const PREVIEW_MAX: usize = 400;

// The wallet sheet's Spend paths block, the share sheets' recovery line and
// the Starter's blank paths page (`docs/BACKUP-FLOW.md` §18.4), in PDF points.

/// Printed sheets: the size of a line of text (a path's keys and wait, the
/// table's heads, the line about coins left unmoved).
pub const SHEET_TEXT_SIZE: f32 = 9.0;
/// Printed sheets: the size of a group's heading and the recovery line.
pub const SHEET_LABEL_SIZE: f32 = 10.0;
/// Printed sheets: the size of the foot line.
pub const SHEET_FOOT_SIZE: f32 = 8.0;
/// Printed sheets: the size of the rule over the table of deposits.
pub const SHEET_RULE_SIZE: f32 = 8.5;
/// Printed sheets: the size of a page's title.
pub const SHEET_TITLE_SIZE: f32 = 18.0;
/// Printed sheets: the grey of a rule to write on.
pub const SHEET_RULE_GREY: f32 = 0.55;
/// Spend paths: from the heading to the first path.
pub const PATHS_HEAD_GAP: f32 = 16.0;
/// Spend paths: one path's line, and one line of the table's head.
pub const PATHS_LINE: f32 = 14.0;
/// Spend paths: where a path's wait starts, from the left margin.
pub const PATHS_WAIT_X: f32 = 150.0;
/// Spend paths: the room after the paths.
pub const PATHS_GAP: f32 = 6.0;
/// Spend paths: the room after the line about coins left unmoved.
pub const PATHS_PARA_GAP: f32 = 8.0;
/// Spend paths: the room under the block.
pub const PATHS_END: f32 = 10.0;
/// The blank paths page: from the title to the line naming the columns.
pub const PATHS_TITLE_GAP: f32 = 40.0;
/// The blank paths page: from the line naming the columns to the first rule.
pub const PATHS_FIELDS_GAP: f32 = 20.0;
/// The blank paths page: from a field's top to its rule, and the room
/// after the last field.
pub const PATHS_RULE_DROP: f32 = 12.0;
/// The blank paths page: one path's field to fill in.
pub const PATHS_FIELD_ROW: f32 = 28.0;
/// The table of deposits: its columns (received on, amount, move before).
pub const MOVE_TABLE_COLS: usize = 3;
/// The table of deposits: its rows to fill in.
pub const MOVE_TABLE_ROWS: usize = 8;
/// The table of deposits: a column head's inset from its column's left.
pub const MOVE_TABLE_INSET: f32 = 4.0;
/// The table of deposits: the room between one column's rule and the next.
pub const MOVE_TABLE_RULE_GAP: f32 = 8.0;
/// The table of deposits: one row.
pub const MOVE_TABLE_ROW: f32 = 18.0;
/// Share sheets: the room under the quorum line, and the recovery line's row.
pub const SHARE_QUORUM_GAP: f32 = 18.0;
/// Share sheets: the room before the share's lines.
pub const SHARE_LINES_GAP: f32 = 10.0;
