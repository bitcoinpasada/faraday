//! The search in the top app bar of the wide layout, and on Home the three
//! first actions as text-only pills beside it (`docs/WORKSPACE.md` §4,
//! `docs/HOME.md` §1): the field takes what the page's name, the pills and
//! the bar's actions leave, centred over the page's centre line where the
//! bar has room for the group, packed left to right where it has not.

use super::catalog::Bar;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::{Style, Ui};
use crate::{Action, Screen};
use faraday_draw::tokens::{
    APP_BAR_H, GAP_SMALL, LEAD_MAX, MARGIN_COMPACT, MENU_GAP, SEARCH_BAR_H, TITLE_L,
};

/// The search the wide layout's bar holds on `screen`, and whether Home's
/// pills stand beside it. A page with no search today has none.
pub(crate) fn on(screen: Screen) -> Option<(Bar, bool)> {
    match screen {
        Screen::Home => Some((Bar::Home, true)),
        Screen::Catalog => Some((Bar::Tools, false)),
        _ => None,
    }
}

/// Home's three first actions: the label and what a press does.
fn ways() -> [(&'static str, Action); 3] {
    [
        (EN.home_ways_new_short, Action::CreateWallet),
        (EN.home_ways_restore_short, Action::RestoreWallet),
        (EN.home_ways_spend_short, Action::Nav(Screen::SpendBackup)),
    ]
}

/// Where the group (field, then pills) starts and how wide the field is:
/// `free` is the bar's room between the name and the actions, `centre` the
/// page's centre line, `pills` the pills' width with their gaps, `want` the
/// field's width where nothing is short. The group is centred on `centre`
/// where it fits there, else it starts at the room's start; the field
/// shrinks to what the room leaves.
fn place((from, to): (f32, f32), centre: f32, pills: f32, want: f32) -> (f32, f32) {
    let room = (to - from).max(0.0);
    let sw = want.min((room - pills).max(0.0));
    let group = sw + pills;
    let start = (centre - group / 2.0).clamp(from, (to - group).max(from));
    (start, sw)
}

/// Draws the bar's search, and Home's pills, for the page on show, `ends`
/// being where the bar's actions begin and `title` the page's name. A
/// lookup. Nothing where the page has no search.
pub(crate) fn draw(app: &View, ui: &mut Ui, (room, title, ends): (f32, &str, f32)) {
    let Some((bar, with_ways)) = on(app.screen) else {
        return;
    };
    let beside = if app.guide_pane_shown() && !app.unit_on_show() {
        super::support_pane_room()
    } else {
        0.0
    };
    let (ox, cw) = super::page_area(app, app.screen, room - beside);
    let name_end = MARGIN_COMPACT + ui.measure_as(TITLE_L, title) + ui.bar_pill_w + MENU_GAP;
    let to = ends - GAP_SMALL;
    let ways: Vec<_> = if with_ways { ways().into() } else { Vec::new() };
    let widths: Vec<f32> = ways
        .iter()
        .map(|(label, _)| MENU_GAP + ui.button_width(label, Style::Tonal))
        .collect();
    let pills: f32 = widths.iter().sum();
    let (start, sw) = place((name_end, to), ox + cw / 2.0, pills, LEAD_MAX);
    let y = (APP_BAR_H - SEARCH_BAR_H) / 2.0;
    super::catalog::home_search_bar(app, ui, (start, y, sw), None, bar);
    let mut x = start + sw;
    let by = (APP_BAR_H - SEARCH_BAR_H) / 2.0;
    for ((label, action), w) in ways.into_iter().zip(widths) {
        ui.button(
            x + MENU_GAP,
            by,
            Some(w - MENU_GAP),
            SEARCH_BAR_H,
            label,
            Style::Tonal,
            action,
        );
        x += w;
    }
}
