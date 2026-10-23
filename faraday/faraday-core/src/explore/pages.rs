//! Explore's two pages over the path's account (F76): the Addresses
//! list (`docs/DESIGN.md` §5's Addresses screen) and Find an address
//! (an Entry over the address keyboard, its result under it).

use faraday_draw::tokens as T;

use super::addresses::{DEPTHS, PAGE, Search};
use super::screen::{frame, head, script_name};
use crate::Action;
use crate::drawing::View;
use crate::explore::{Page, XAction};
use crate::screens::button_rows;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W, thousands};

/// A press on these pages.
fn xa(a: XAction) -> Action {
    Action::X(a)
}

/// The head of either page: the way back to the path and the title on
/// the desktop (a small panel's bar has both), then the account the
/// list is of. Returns where the page goes on.
fn top(app: &View, ui: &mut Ui, x: f32, y: f32, width: f32, name: &str) -> f32 {
    let mut y = y;
    if !ui.compact {
        let back = (EN.explore_explore, xa(XAction::Page(Page::Path)));
        y = head(ui, x, y, back, name);
    }
    let Some((_, account, script, ..)) = app.explore_account() else {
        return y;
    };
    let path = account.to_string().replace('\'', "h");
    let line = fill(EN.explore_account_path, &[&path, script_name(script)]);
    ui.text(x, y, T::CAPTION, W::R, MUTED, EN.explore_account);
    y += T::PAD + T::GAP_SMALL;
    y + ui.wrap(x, y, width, T::LABEL, W::M, TEXT, &line) + T::PAD
}

/// One address row: its name ("Receive 12") above, the address below,
/// always whole (§4.5), on one line where it fits and else wrapped; a
/// found address (`mark` is where it matched) has the matched part in the
/// accent and heavier. Returns the height used.
fn row(
    ui: &mut Ui,
    (x, y, width): (f32, f32, f32),
    at: (bool, u32),
    (a, mark): (&str, Option<&std::ops::Range<usize>>),
) -> f32 {
    let name = if at.0 {
        EN.explore_change_n
    } else {
        EN.explore_receive_n
    };
    ui.text(
        x,
        y,
        T::CAPTION,
        W::R,
        MUTED,
        &fill(name, &[&at.1.to_string()]),
    );
    let y2 = y + T::PAD + T::GAP_SMALL;
    let none = 0..0;
    let used = ui.wrap_marked(
        (x, y2, width),
        T::LABEL,
        W::M,
        (TEXT, ACCENT),
        a,
        mark.unwrap_or(&none).clone(),
    );
    y2 - y + used + T::GAP
}

/// A field drawn as Explore's path field is: the text and the caret, a
/// red edge when `bad`. Returns the height used.
fn field(ui: &mut Ui, x: f32, y: f32, width: f32, text: &str, bad: bool, chip: bool) -> f32 {
    ui.fill(x, y, width, T::FIELD, T::GAP, BG);
    ui.stroke(
        x,
        y,
        width,
        T::FIELD,
        T::GAP,
        if bad { ERR } else { ACCENT },
    );
    if chip {
        crate::samples::put::chip(ui, (x, y, width, T::FIELD), Action::UseSample);
        return T::FIELD + T::GAP;
    }
    let shown = format!("{text}{}", ui.caret_char());
    let shown = ui.fit(T::BODY, W::M, &shown, width - T::PAD * 2.0);
    ui.text_mid(x + T::PAD, y, T::FIELD, T::BODY, W::M, TEXT, &shown);
    T::FIELD + T::GAP
}

/// The Addresses page: Receive | Change, the first index as a field
/// typed in digits with a page back and on beside it, then a page of
/// rows.
pub(super) fn addresses(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(e) = app.explore.as_ref() else {
        return;
    };
    let (x, width, start, y) = frame(app, ui, x0, cw);
    let mut y = top(app, ui, x, y, width, EN.explore_addresses);
    let pick = |on: bool| if on { Style::Primary } else { Style::Secondary };
    let chains = [
        (
            EN.explore_receive.to_string(),
            pick(!e.change),
            xa(XAction::Chain(false)),
        ),
        (
            EN.explore_change.to_string(),
            pick(e.change),
            xa(XAction::Chain(true)),
        ),
    ];
    y += button_rows(ui, x, y, width, &chains);
    ui.text(x, y, T::CAPTION, W::R, MUTED, EN.explore_from_index);
    y += T::PAD + T::GAP_SMALL;
    let fw = if ui.compact { width } else { width / 2.0 };
    y += field(ui, x, y, fw, &e.from.to_string(), false, false);
    let page = PAGE.to_string();
    let steps = [
        (
            fill(EN.explore_previous_n, &[&page]),
            if e.from == 0 {
                Style::Disabled
            } else {
                Style::Ghost
            },
            xa(XAction::Step(-1)),
        ),
        (
            fill(EN.explore_next_n, &[&page]),
            Style::Ghost,
            xa(XAction::Step(1)),
        ),
    ];
    y += button_rows(ui, x, y, width, &steps);
    for (i, a) in app.explore_list() {
        y += row(ui, (x, y, width), (e.change, i), (&a, None));
    }
    crate::compact::screen::finish(app, ui, x0, cw, h, y - start + T::PAD);
}

/// Find an address: the field and its Paste, how deep to look, Search
/// (Stop while it looks), then what it found.
pub(super) fn find(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(e) = app.explore.as_ref() else {
        return;
    };
    let (x, width, start, y) = frame(app, ui, x0, cw);
    let mut y = top(app, ui, x, y, width, EN.explore_find);
    ui.text(x, y, T::CAPTION, W::R, MUTED, EN.explore_address);
    y += T::PAD + T::GAP_SMALL;
    let chip = app.explore_sample_shown();
    y += field(ui, x, y, width, &e.query, e.note.is_some(), chip);
    if let Some(n) = e.note {
        y += ui.wrap(x, y, width, T::CAPTION, W::R, ERR, n) + T::GAP;
    }
    let running = e.search.as_ref().is_some_and(Search::running);
    let mut acts = Vec::new();
    if app.clipboard {
        acts.push((
            EN.scan_paste.to_string(),
            Style::Secondary,
            xa(XAction::Paste),
        ));
    }
    if !acts.is_empty() {
        y += button_rows(ui, x, y, width, &acts);
    }
    ui.text(x, y, T::CAPTION, W::R, MUTED, EN.explore_look_through);
    y += T::PAD + T::GAP_SMALL;
    let depths: Vec<(String, Style, Action)> = DEPTHS
        .iter()
        .enumerate()
        .map(|(i, &d)| {
            (
                fill(EN.explore_first_n, &[&thousands(u64::from(d))]),
                if d == e.depth {
                    Style::Primary
                } else {
                    Style::Secondary
                },
                xa(XAction::Depth(i as u8)),
            )
        })
        .collect();
    y += button_rows(ui, x, y, width, &depths);
    let go = if running {
        (EN.explore_stop.to_string(), Style::Secondary)
    } else if e.query.is_empty() {
        (EN.explore_search.to_string(), Style::Disabled)
    } else {
        (EN.explore_search.to_string(), Style::Primary)
    };
    y += button_rows(ui, x, y, width, &[(go.0, go.1, xa(XAction::Search))]);
    let of = app.explore_account();
    if let Some(s) = e.search.as_ref().filter(|s| Some(&s.of) == of.as_ref()) {
        y += result(ui, x, y, width, s);
    }
    crate::compact::screen::finish(app, ui, x0, cw, h, y - start + T::PAD);
}

/// What a search found: how far it has looked while it looks; each
/// match, receive or change and its index, and how many more there are
/// than are listed; or that the address is not among those looked at.
/// Returns the height used.
fn result(ui: &mut Ui, x: f32, y: f32, width: f32, s: &Search) -> f32 {
    let mut cy = y;
    let depth = thousands(u64::from(s.depth));
    if s.running() {
        let line = fill(
            EN.explore_searched,
            &[&thousands(u64::from(s.next)), &depth],
        );
        cy += ui.wrap(x, cy, width, T::LABEL, W::S, TEXT, &line) + T::GAP;
    } else if s.matched == 0 {
        let line = fill(EN.explore_not_in_the_first, &[&depth]);
        return ui.wrap(x, cy, width, T::LABEL, W::S, WARN, &line) + T::GAP;
    }
    if s.matched > 0 {
        let line = fill(EN.explore_found_n, &[&thousands(u64::from(s.matched))]);
        cy += ui.wrap(x, cy, width, T::LABEL, W::S, OK, &line) + T::GAP;
    }
    for f in &s.found {
        cy += row(
            ui,
            (x, cy, width),
            (f.change, f.index),
            (&f.address, Some(&f.mark)),
        );
    }
    let more = s.matched as usize - s.found.len();
    if more > 0 {
        let line = fill(EN.explore_more_n, &[&thousands(more as u64)]);
        cy += ui.wrap(x, cy, width, T::CAPTION, W::R, MUTED, &line) + T::GAP;
    }
    cy - y
}
