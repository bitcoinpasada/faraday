//! The failed start-up self-test: the whole screen, no sidebar and no sheet. It names the check
//! that failed and offers Exit: a build whose published vectors do not reproduce has nowhere to
//! go but out.

use faraday_atoms::{Icon, Look, Pressed, Shown, Ui};
use faraday_tokens::dims::{BUTTON_H, GAP_SMALL};
use faraday_tokens::palette::Role;
use faraday_tokens::type_roles::{BODY_S, TITLE_M, sheet_head};

/// The column's side margins and its widest, in units.
const SIDE: f32 = 64.0;
const COLUMN_MAX: f32 = 560.0;
/// How far above the middle the column starts, and its least top.
const ABOVE_MIDDLE: f32 = 120.0;
const TOP_MIN: f32 = 24.0;
/// The head's row, its icon, the icon's glyph; where the check's row starts under it.
const HEAD_ROW: f32 = 30.0;
const HEAD_GLYPH: f32 = 18.0;
const HEAD_TEXT_AT: f32 = 42.0;
const HEAD_TO_CHECK: f32 = 56.0;
/// What stands between the check's name and Exit, and Exit's width.
const BEFORE_EXIT: f32 = 32.0;
const EXIT_W: f32 = 140.0;

/// The failed self-test's words and the press of Exit.
pub struct SelftestFailed<'s, R> {
    pub title: Shown<'s>,
    /// The label over the check's name.
    pub check_row: Shown<'s>,
    /// Which check failed.
    pub check: Shown<'s>,
    pub exit_label: Shown<'s>,
    pub exit: R,
}

/// Draws the failed self-test over a `w` by `h` window.
pub fn draw_selftest_failed<R: Pressed>(
    m: &SelftestFailed<'_, R>,
    ui: &mut Ui<'_, R>,
    (w, h): (f32, f32),
) {
    ui.clear(Role::Surface);
    let cw = (w - SIDE).min(COLUMN_MAX);
    let x = (w - cw) / 2.0;
    let mut y = (h / 2.0 - ABOVE_MIDDLE).max(TOP_MIN);
    ui.icon(x, y, HEAD_ROW, Icon::Error, HEAD_GLYPH, Role::Error);
    ui.text_mid_as(
        (x + HEAD_TEXT_AT, y, HEAD_ROW),
        sheet_head(false),
        Role::OnSurface,
        m.title.as_str(),
    );
    y += HEAD_TO_CHECK;
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, m.check_row.as_str());
    y += ui.line_as(BODY_S) + GAP_SMALL;
    y += ui.wrap_as((x, y, cw), TITLE_M, Role::Error, m.check.as_str()) + BEFORE_EXIT;
    ui.button(
        x,
        y,
        Some(EXIT_W),
        BUTTON_H,
        m.exit_label.as_str(),
        Look::Primary,
        m.exit.clone(),
    );
}
