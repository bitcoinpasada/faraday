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
    APP_BAR_H, GAP_SMALL, HOME_SEARCH_MIN, LEAD_MAX, MARGIN_COMPACT, MENU_GAP, SEARCH_BAR_H,
    TITLE_L, TOUCH,
};
use faraday_draw::widgets::Icon;

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

/// How Home's first actions stand beside the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ways {
    /// Three text pills.
    Pills,
    /// One **Start** button that opens a menu of the three.
    Start,
}

/// Where the group (field, then the actions) starts, how wide the field is
/// and how the actions stand. `room` is the room between `from` and `to`
/// (the bar's, between the name and the actions; the page row's whole
/// width), `centre` the line the group is centred on where it fits there,
/// `pills` the three pills' width with their gaps (none where only the
/// one button may stand), `start` the one button's width with its gap,
/// `want` the field's width where nothing is short. The field shrinks to
/// [`HOME_SEARCH_MIN`] before the pills give way to the one button, and the
/// field takes what the button leaves after that. A page with no actions
/// passes `start` 0. The one rule of the bar and of the page's row.
pub(crate) fn place(
    (from, to): (f32, f32),
    centre: f32,
    (pills, start): (Option<f32>, f32),
    want: f32,
) -> (f32, f32, Ways) {
    let room = (to - from).max(0.0);
    let (ways, extra) = match pills {
        Some(p) if room - p >= HOME_SEARCH_MIN => (Ways::Pills, p),
        _ => (Ways::Start, start),
    };
    let sw = want.min((room - extra).max(0.0));
    let group = sw + extra;
    let at = (centre - group / 2.0).clamp(from, (to - group).max(from));
    (at, sw, ways)
}

/// Draws the bar's search, and Home's pills, for the page on show, `ends`
/// being where the bar's actions begin and `title` the page's name. A
/// lookup. Nothing where the page has no search.
pub(crate) fn draw(app: &View, ui: &mut Ui, (room, title, ends): (f32, &str, f32)) {
    let Some((bar, with_ways)) = on(app.screen) else {
        return;
    };
    // The expanded rail centres on the wide page; the narrow rail's page
    // is one column, centred in the area.
    let (ox, cw) = if app.nav() == crate::nav::Nav::ExpandedRail {
        // The guide's pane stands under the bar, so the search is placed
        // from the bar's whole room and stands still as the guide opens.
        super::page_area(app, app.screen, room)
    } else {
        (0.0, room)
    };
    // The back arrow of a row open in the page stands before the name.
    let arrow = if crate::row_open::opened(app).is_some() {
        TOUCH
    } else {
        0.0
    };
    let name_end =
        MARGIN_COMPACT + arrow + ui.measure_as(TITLE_L, title) + ui.bar_pill_w + MENU_GAP;
    let to = ends - GAP_SMALL;
    let y = (APP_BAR_H - SEARCH_BAR_H) / 2.0;
    group(
        app,
        ui,
        (bar, with_ways, true),
        ((name_end, to), ox + cw / 2.0),
        (y, LEAD_MAX),
    );
}

/// The search and Home's first actions from `from` to `to`, at height `y`:
/// the one place that draws them, in the top bar and in the page's row
/// (`pills_ok` false there: the one **Start** button stands beside the
/// field). `centre` is the line the group centres on, `want` the field's
/// widest. Returns where the group ends.
pub(crate) fn group(
    app: &View,
    ui: &mut Ui,
    (bar, with_ways, pills_ok): (Bar, bool, bool),
    (range, centre): ((f32, f32), f32),
    (y, want): (f32, f32),
) -> f32 {
    let ways: Vec<_> = if with_ways { ways().into() } else { Vec::new() };
    let widths: Vec<f32> = ways
        .iter()
        .map(|(label, _)| MENU_GAP + ui.button_width(label, Style::Tonal))
        .collect();
    let start_w = if with_ways {
        MENU_GAP + ui.button_icon_width(EN.home_ways_start, Style::Tonal)
    } else {
        0.0
    };
    let pills = pills_ok.then(|| widths.iter().sum::<f32>());
    let (start, sw, how) = place(range, centre, (pills, start_w), want);
    super::catalog::home_search_bar(app, ui, (start, y, sw), None, bar);
    let mut x = start + sw;
    if !with_ways {
        return x;
    }
    match how {
        Ways::Start => {
            ui.button_icon_after(
                (x + MENU_GAP, y, Some(start_w - MENU_GAP), SEARCH_BAR_H),
                Icon::ChevronDown,
                EN.home_ways_start,
                Style::Tonal,
                Action::NavMenu(crate::nav::MenuPress::Start),
            );
            x += start_w;
        }
        Ways::Pills => {
            for ((label, action), w) in ways.into_iter().zip(widths) {
                ui.button(
                    x + MENU_GAP,
                    y,
                    Some(w - MENU_GAP),
                    SEARCH_BAR_H,
                    label,
                    Style::Tonal,
                    action,
                );
                x += w;
            }
        }
    }
    x
}
