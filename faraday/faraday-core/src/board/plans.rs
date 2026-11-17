//! The read-only board's buttons: the ready plans first, then the
//! software and Customize backup (`docs/BACKUP-FLOW.md` §17.21 item 3,
//! §17.25).

use faraday_strings::EN;
use faraday_ui::Style;

use super::{BoardPress, BoardView};
use crate::Action;

/// The buttons in the list's place while the board is read-only: a button
/// for each ready plan, the one the board is now drawn as selected, then
/// **Add watch-only software** (or **Take off watch-only software**) and
/// **Customize backup**.
pub(super) fn read_only_buttons(v: &BoardView) -> Vec<(&'static str, Style, Action)> {
    let software = if v.software.is_some() {
        EN.board_take_off_watch
    } else {
        EN.board_add_watch_only
    };
    let mut out: Vec<(&'static str, Style, Action)> = v
        .ready
        .iter()
        .map(|&(r, on)| {
            let style = if on {
                Style::Selected
            } else {
                Style::Secondary
            };
            (r.label(), style, Action::Board(BoardPress::Plan(r)))
        })
        .collect();
    out.push((
        software,
        Style::Secondary,
        Action::Board(BoardPress::Software),
    ));
    out.push((
        EN.board_customize,
        Style::Secondary,
        Action::Board(BoardPress::Customize),
    ));
    out
}
