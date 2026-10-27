//! Explore's two pages over the path's account (F76): the Addresses
//! list (`docs/DESIGN.md` §5's Addresses screen) and Find an address
//! (an Entry over the address keyboard, its result under it).

use faraday_draw::tokens as T;

use super::Eye;
use super::addresses::{DEPTHS, PAGE, Search};
use super::screen::{Cols, eye_button, frame, head, script_name, value_row};
use super::shown::address_path;
use crate::Action;
use crate::drawing::View;
use crate::explore::{Page, XAction};
use crate::screens::button_rows;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W, thousands};
use faraday_draw::tokens::{BODY_L, BODY_M, BODY_S, FIELD_H, FIELD_PAD, LABEL_L};

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
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, EN.explore_account);
    y += T::PAD + T::GAP_SMALL;
    y + ui.wrap_as((x, y, width), BODY_M.mono(), Role::OnSurface, &line) + T::PAD
}

/// One address row: its name ("Receive 12") above, the address below,
/// always whole (§4.5), on one line where it fits and else wrapped; a
/// found address (`mark` is where it matched) has the matched part in the
/// accent and heavier. On the Addresses page, `eye` is the row's eye on
/// its private key, at the row's right end, with the key while shown,
/// under the address. Returns the height used.
fn row(
    ui: &mut Ui,
    (x, y, full): (f32, f32, f32),
    at: (bool, u32),
    (a, mark): (&str, Option<&std::ops::Range<usize>>),
    eye: Option<Option<&str>>,
) -> f32 {
    let width = if eye.is_some() { full - T::TOUCH } else { full };
    let name = if at.0 {
        EN.explore_change_n
    } else {
        EN.explore_receive_n
    };
    ui.text_as(
        x,
        y,
        BODY_S,
        Role::OnSurfaceVariant,
        &fill(name, &[&at.1.to_string()]),
    );
    let y2 = y + T::PAD + T::GAP_SMALL;
    let none = 0..0;
    let used = ui.wrap_marked(
        (x, y2, width),
        (BODY_M.mono()).size,
        W::from((BODY_M.mono()).face),
        (Role::OnSurface.paint(), Role::Primary.paint()),
        a,
        mark.unwrap_or(&none).clone(),
    );
    let Some(wif) = eye else {
        return y2 - y + used + T::GAP;
    };
    eye_button(ui, x + width, y, Eye::Row(at.0, at.1), wif.is_some());
    let mut h = (y2 - y + used + T::GAP).max(T::TOUCH);
    if let Some(w) = wif {
        ui.text_as(x, y + h, BODY_S, Role::OnSurfaceVariant, EN.explore_wif);
        h += T::PAD + T::GAP_SMALL;
        let cols = Cols {
            x,
            px: x,
            pw: width,
            width,
        };
        h += value_row(ui, cols, y + h, "", w);
    }
    h
}

/// A field drawn as Explore's path field is: the text and the caret, a
/// red edge and its `error` line under it. Returns the height used.
fn field(
    ui: &mut Ui,
    (x, y, width): (f32, f32, f32),
    text: &str,
    error: Option<&str>,
    chip: bool,
) -> f32 {
    let used = ui.field_box((x, y, width), true, error);
    // Use a sample goes in the "Demo only" card while the field is empty
    // (`docs/DEMO.md` §14).
    if chip {
        crate::samples::put::offer(ui, Action::UseSample);
    }
    let shown = format!("{text}{}", ui.caret_char());
    let shown = ui.fit_as(BODY_L.mono(), &shown, width - FIELD_PAD * 2.0);
    ui.text_mid_as(
        (x + FIELD_PAD, y, FIELD_H),
        BODY_L.mono(),
        Role::OnSurface,
        &shown,
    );
    used + T::GAP
}

/// The Addresses page: Receive | Change, the first index as a field
/// typed in digits with a page back and on beside it, then a page of
/// rows.
pub(super) fn addresses(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(e) = app.explore.state() else {
        return;
    };
    let (x, width, start, y) = frame(app, ui, x0, cw);
    let mut y = top(app, ui, x, y, width, EN.explore_addresses);
    let pick = Style::chosen;
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
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, EN.explore_from_index);
    y += T::PAD + T::GAP_SMALL;
    let fw = if ui.compact { width } else { width / 2.0 };
    y += field(ui, (x, y, fw), &e.from.to_string(), None, false);
    let page = PAGE.to_string();
    let steps = [
        (
            fill(EN.explore_previous_n, &[&page]),
            Style::Ghost.when(e.from != 0),
            xa(XAction::Step(-1)),
        ),
        (
            fill(EN.explore_next_n, &[&page]),
            Style::Ghost,
            xa(XAction::Step(1)),
        ),
    ];
    y += button_rows(ui, x, y, width, &steps);
    let account = app.explore_account().map(|of| of.1);
    let shown = app.explore_shown();
    for (i, a) in app.explore_list() {
        let wif = account
            .as_ref()
            .and_then(|acc| shown.and_then(|s| s.wif(&address_path(acc, e.change, i))));
        y += row(ui, (x, y, width), (e.change, i), (&a, None), Some(wif));
    }
    crate::compact::screen::finish(app, ui, x0, cw, h, y - start + T::PAD);
}

/// Find an address: the field and its Paste, how deep to look, Search
/// (Stop while it looks), then what it found.
pub(super) fn find(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(e) = app.explore.state() else {
        return;
    };
    let (x, width, start, y) = frame(app, ui, x0, cw);
    let mut y = top(app, ui, x, y, width, EN.explore_find);
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, EN.explore_address);
    y += T::PAD + T::GAP_SMALL;
    let chip = app.explore_sample_shown();
    y += field(ui, (x, y, width), &e.query, e.note, chip);
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
    ui.text_as(
        x,
        y,
        BODY_S,
        Role::OnSurfaceVariant,
        EN.explore_look_through,
    );
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
        (EN.explore_search.to_string(), Style::PrimaryOff)
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
        cy += ui.wrap_as((x, cy, width), LABEL_L, Role::OnSurface, &line) + T::GAP;
        let done = s.next as f32 / s.depth.max(1) as f32;
        ui.progress((x, cy, width), Some(done), None);
        cy += T::PROGRESS_H + T::GAP;
    } else if s.matched == 0 {
        let line = fill(EN.explore_not_in_the_first, &[&depth]);
        return ui.wrap_as((x, cy, width), LABEL_L, Role::Warning, &line) + T::GAP;
    }
    if s.matched > 0 {
        let line = fill(EN.explore_found_n, &[&thousands(u64::from(s.matched))]);
        cy += ui.wrap_as((x, cy, width), LABEL_L, Role::Ok, &line) + T::GAP;
    }
    for f in &s.found {
        cy += row(
            ui,
            (x, cy, width),
            (f.change, f.index),
            (&f.address, Some(&f.mark)),
            None,
        );
    }
    let more = s.matched as usize - s.found.len();
    if more > 0 {
        let line = fill(EN.explore_more_n, &[&thousands(more as u64)]);
        cy += ui.wrap_as((x, cy, width), BODY_S, Role::OnSurfaceVariant, &line) + T::GAP;
    }
    cy - y
}
