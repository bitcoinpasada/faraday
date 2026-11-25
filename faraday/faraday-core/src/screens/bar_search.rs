//! The search in the top app bar of the wide layout, and on Home the three
//! first actions as text-only pills beside it (`docs/WORKSPACE.md` §4,
//! `docs/HOME.md` §1): the field takes what the page's name, the pills and
//! the bar's actions leave, centred over the page's centre line where the
//! bar has room for the group, packed left to right where it has not.

use super::catalog::Bar;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::{Action, Screen};
use faraday_draw::tokens::{
    APP_BAR_H, ELEVATION_SHEET, GAP_SMALL, HOME_SEARCH_MIN, LEAD_MAX, MARGIN_COMPACT, MENU_GAP,
    SEARCH_BAR_H, TITLE_L, TOUCH,
};
use faraday_draw::widgets::Icon;

/// The menu pages, which hold the one search and the three pills in the top
/// bar (`docs/WORKSPACE.md` §4), in the sidebar's order.
const MENU_PAGES: [Screen; 7] = [
    Screen::Home,
    Screen::Catalog,
    Screen::Check,
    Screen::WalletsStart,
    Screen::Files,
    Screen::Vaults,
    Screen::Settings,
];

/// Whether `screen` is a menu page, whose top bar holds the search and the
/// pills, always at one place.
pub(crate) fn on(screen: Screen) -> bool {
    MENU_PAGES.contains(&screen)
}

/// What the menu pages with a search of their own row hold in `Page.search`:
/// the one search where the bar does not hold it, as on the phone, the Pi
/// and a compact rail; none where the bar holds it. A lookup.
pub(crate) fn page_search(app: &View) -> Option<Bar> {
    app.search_in_bar().then_some(Bar::Tools)
}

/// Whether a search is typed over a page that shows the results alone, in
/// the place of its card: a flow's step, or a menu page that is no
/// `menu_page` (Files, Vaults). A lookup.
pub(crate) fn results_alone(app: &View) -> bool {
    !app.tools.home_find().is_empty()
        && (in_flow(app)
            || (app.search_in_bar()
                && !app.is_compact()
                && matches!(app.screen, Screen::Files | Screen::Vaults)))
}

/// Whether the page on show is a flow's step on a window whose top bar
/// holds the search and that does not hold the bar off
/// (`docs/WORKSPACE.md` §4, F331). A lookup.
pub(crate) fn in_flow(app: &crate::Faraday) -> bool {
    app.search_in_bar() && app.nav_bar_hidden() && !app.nav_hidden()
}

/// Whether the three pills stand in the bar, as the last frame drew it: not
/// the one Start button, and not on a window whose bar holds no search. A
/// lookup.
pub(crate) fn pills_stand(app: &View) -> bool {
    app.search_in_bar() && app.tools.pills_up()
}

/// Whether the bar holds the search and the pills for the page on show: a
/// menu page's, or in a flow. A lookup.
pub(crate) fn here(app: &View) -> bool {
    on(app.screen) || in_flow(app)
}

/// Whether the flow of `action`'s way is the one on screen: that pill is
/// off, so New wallet never starts a second one inside the first.
pub(crate) fn way_is_here(screen: Screen, action: Action) -> bool {
    matches!(
        (screen, action),
        (Screen::Create, Action::CreateWallet)
            | (Screen::Restore, Action::RestoreWallet)
            | (Screen::SpendBackup, Action::Nav(Screen::SpendBackup))
    )
}

/// Gives the search or a pill at (x, y, w, h) its height: level 1 on a menu
/// page and none in a flow, never dimmed; at High contrast, which casts no
/// shadow, an outline either way. Only the wide layout's bar: the phone-size
/// window and the Pi's row do not change (F331).
fn raise(app: &View, ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32)) {
    if app.nav() != crate::nav::Nav::ExpandedRail {
        return;
    }
    if !in_flow(app) {
        ui.shadow(x, y, w, h, h / 2.0, ELEVATION_SHEET);
    }
    if ui.theme.is_high_contrast() {
        ui.stroke(x, y, w, h, h / 2.0, Role::Outline);
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
    if !here(app) {
        return;
    }
    let flow = in_flow(app);
    let menu = on(app.screen);
    app.tools.note_pills(false);
    // The expanded rail centres on the wide page; the narrow rail's page
    // is one column, centred in the area. A menu page's is Home's, so the
    // group stands at one place on every one of them.
    let (ox, cw) = if app.nav() == crate::nav::Nav::ExpandedRail {
        // The guide's pane stands under the bar, so the search is placed
        // from the bar's whole room and stands still as the guide opens.
        super::page_area(app, if menu { Screen::Home } else { app.screen }, room)
    } else {
        (0.0, room)
    };
    // The back arrow stands before the name only where it is drawn: in a
    // flow, or with a row open in the page.
    let arrow = if flow || crate::row_open::opened(app).is_some() {
        TOUCH
    } else {
        0.0
    };
    // A menu page reserves the widest page name, and the bar's actions
    // that only one page shows (Files' stick chip), so the same field and
    // pills stand on each.
    let mut name_w = ui.measure_as(TITLE_L, title);
    let mut ends = ends;
    if menu {
        for s in MENU_PAGES {
            name_w = name_w.max(ui.measure_as(TITLE_L, crate::compact::screen_name(s)));
        }
        if app.screen != Screen::Files {
            let chip = crate::compact::screen::measured(ui, |ui| {
                crate::screens::files_stick_chip(app, ui, room)
            });
            ends -= room - chip;
        }
    }
    let name_end = MARGIN_COMPACT + arrow + name_w + ui.bar_pill_w + MENU_GAP;
    let to = ends - GAP_SMALL;
    let y = (APP_BAR_H - SEARCH_BAR_H) / 2.0;
    // In a flow they are left out where not even the field at its least
    // and the one Start button fit beside the heading.
    if flow {
        let start = MENU_GAP + ui.button_icon_width(EN.home_ways_start, Style::Tonal);
        if to - name_end < HOME_SEARCH_MIN + start {
            return;
        }
    }
    group(
        app,
        ui,
        (true, true),
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
    (with_ways, pills_ok): (bool, bool),
    (range, centre): ((f32, f32), f32),
    (y, want): (f32, f32),
) -> f32 {
    let ways: Vec<_> = if with_ways { ways().into() } else { Vec::new() };
    for (_, action) in &ways {
        if way_is_here(app.screen, *action) {
            ui.dimmed.push(*action);
        }
    }
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
    raise(app, ui, (start, y, sw, SEARCH_BAR_H));
    super::catalog::home_search_bar(app, ui, (start, y, sw), None);
    let mut x = start + sw;
    if !with_ways {
        return x;
    }
    match how {
        Ways::Start => {
            raise(app, ui, (x + MENU_GAP, y, start_w - MENU_GAP, SEARCH_BAR_H));
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
            if pills_ok {
                app.tools.note_pills(true);
            }
            for ((label, action), w) in ways.into_iter().zip(widths) {
                raise(app, ui, (x + MENU_GAP, y, w - MENU_GAP, SEARCH_BAR_H));
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
