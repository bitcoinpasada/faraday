//! The Load samples sheet (F81, `docs/MODES.md` §3 and §10 M6): the sample
//! wallets, then the sample keys, each a tick box; Select all or Unselect
//! all over them; Not now and Load ticked at the foot. A sample already
//! loaded shows ticked and dimmed. Drawing reads the rows
//! ([`super::SheetState`]) and works nothing out.

use super::{PASSPHRASE, Pick, Row, label};
use crate::Action;
use crate::compact::{buttons, scroll_sheet};
use crate::drawing::View;
use crate::modes::screen::{head, place};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use faraday_draw::tokens::{CAPTION, GAP, LABEL, PAD, TOUCH};
use faraday_draw::widgets::Icon;

/// The sheet, over the screen it was opened from.
pub(crate) fn sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let place = place(ui, w);
    scroll_sheet(ui, place.0, h, place.1, place.2, &mut |ui, x, y, iw| {
        body(app, ui, x, y, iw)
    });
}

/// The rows of `multi_list_valued` and the value under each.
type Listed = (Vec<(String, bool, bool, Action)>, Vec<String>);

/// A list's rows for `multi_list_valued`: the sample's name, ticked once
/// loaded too, and pressable until it is; and, for each, what it is.
fn list(rows: &[Row], name: impl Fn(usize) -> String, pick: fn(u8) -> Pick) -> Listed {
    rows.iter()
        .enumerate()
        .filter_map(|(i, r)| {
            let at = u8::try_from(i).ok()?;
            let action = Action::SampleTick(pick(at));
            let row = (name(r.sample), r.ticked || r.loaded, !r.loaded, action);
            Some((row, r.kind.clone()))
        })
        .unzip()
}

/// A small heading over a list. Returns its height.
fn heading(ui: &mut Ui, x: f32, y: f32, text: &str) -> f32 {
    ui.text(x, y, CAPTION, W::S, Role::OnSurfaceVariant, text);
    CAPTION + GAP
}

/// The title, the line, Select all, the two lists and the buttons. Returns
/// its height.
fn body(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let (Some(set), state) = (app.samples, app.sample_area.sheet()) else {
        return 0.0;
    };
    let mut cy = y;
    cy += head(ui, (x, cy, w), Icon::Download, EN.samples_title);
    cy += ui.code_line(
        (x, cy, w),
        LABEL,
        Role::OnSurfaceVariant,
        EN.samples_password,
        PASSPHRASE,
    ) + PAD;
    // Load ticked with a stick in asks for it out (the Pull sheet); the
    // line is at the top, where the scrolled list does not hide it.
    if app.selftest_passed() && !app.stick.is_empty() {
        let line = fill(
            EN.pull_medium_to_reason,
            &[app.medium.noun(), EN.pull_what_load_samples],
        );
        cy += ui.wrap(x, cy, w, LABEL, W::S, Role::Warning, &line) + PAD;
    }
    // Unselect all while any is ticked; Select all once none is.
    let any = state.any_ticked();
    let link = if any {
        EN.samples_unselect_all
    } else {
        EN.samples_select_all
    };
    let lw = ui.measure(LABEL, W::S, link);
    ui.text_mid(x, cy, TOUCH, LABEL, W::S, Role::Primary, link);
    ui.hit(x - GAP, cy, lw + 2.0 * GAP, TOUCH, Action::SamplesAll);
    cy += TOUCH;
    cy += heading(ui, x, cy, EN.samples_wallets);
    let (wallets, kinds) = list(
        &state.wallets,
        |i| label(set.wallets[i].name, "", 0).unwrap_or_default(),
        Pick::Wallet,
    );
    cy += ui.multi_list_valued(x, cy, w, &wallets, &kinds) + PAD;
    cy += heading(ui, x, cy, EN.samples_keys);
    let (keys, kinds) = list(&state.keys, |i| set.keys[i].label(), Pick::Key);
    cy += ui.multi_list_valued(x, cy, w, &keys, &kinds) + PAD;
    // Pressable while a stick is in: the press opens the Pull sheet.
    let load = Style::Primary.when(any && app.selftest_passed());
    cy += buttons(
        ui,
        x,
        cy,
        w,
        &[
            (EN.samples_not_now, Style::Secondary, Action::Cancel),
            (EN.samples_load, load, Action::SamplesLoad),
        ],
    );
    cy - y
}
