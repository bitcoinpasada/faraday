//! The search bar (`vault/search.rs`) drawn over an open vault's items
//! and the codes, and the items column under it on a wide window: drawing
//! only, from what the vault keeps found.

use super::{item_heading, va};
use crate::compact::screen::row;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Ui, W};
use crate::vault::{Focus, OpenVault, SearchAct, SearchIn, VaultAction as V};
use faraday_draw::tokens::{
    BODY, CAPTION, GAP, GAP_SMALL, LABEL, PAD, RADIUS_SMALL, SEARCH_BAR_H, SEARCH_GLYPH,
    SEARCH_ICON, SEARCH_INSET, TOUCH, VAULT_ITEM_GAP, VAULT_ITEM_H, VAULT_ITEM_LINE,
    VAULT_ITEM_PAD, VAULT_ITEM_TOP,
};
use faraday_draw::widgets::Icon;
use faraday_keys::vault::RecordView;

/// What the bar says while it is empty.
fn hint(s: SearchIn, category: usize) -> &'static str {
    match (s, category) {
        (SearchIn::Codes, _) => EN.codes_search,
        (SearchIn::Items, 1) => EN.vault_search_wallets,
        (SearchIn::Items, 4) => EN.vault_search_gpg_keys,
        (SearchIn::Items, _) => EN.vault_search_entries,
    }
}

/// Material's search bar at (x, y), `w` wide: a full-round field with
/// the magnifying glass leading, what is typed or the placeholder, and a
/// trailing × while it holds text. Returns the height it takes with the
/// gap under it.
pub(crate) fn bar(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, s: SearchIn) -> f32 {
    let q = app.vaults.query(s);
    let focused = app.vaults.focus == Some(Focus::Search(s));
    let h = SEARCH_BAR_H;
    let field = va(V::Search(SearchAct::Focus(s)));
    ui.fill(x, y, w, h, h / 2.0, Role::SurfaceContainerHigh);
    if focused {
        ui.stroke(x, y, w, h, h / 2.0, Role::Primary);
    }
    let icon_y = y + (h - SEARCH_ICON) / 2.0;
    ui.icon(
        x + SEARCH_INSET,
        icon_y,
        SEARCH_ICON,
        Icon::SEARCH,
        SEARCH_GLYPH,
        Role::OnSurfaceVariant,
    );
    ui.hit(x, y, w, h, field);
    let tx = x + 2.0 * SEARCH_INSET + SEARCH_ICON;
    let clear = if q.is_empty() { 0.0 } else { TOUCH };
    let room = (x + w - tx - SEARCH_INSET - clear).max(0.0);
    if q.is_empty() {
        let shown = ui.fit(BODY, W::R, hint(s, app.vaults.category), room);
        ui.text_mid(tx, y, h, BODY, W::R, Role::OnSurfaceVariant, &shown);
    }
    ui.typed(
        tx,
        y,
        h,
        BODY,
        W::R,
        Role::OnSurface,
        q,
        room,
        focused,
        field,
    );
    if !q.is_empty() {
        let cx = x + w - GAP_SMALL - TOUCH;
        let cy = y + (h - TOUCH) / 2.0;
        ui.text_mid(
            cx + (TOUCH - ui.measure(BODY, W::S, "×")) / 2.0,
            cy,
            TOUCH,
            BODY,
            W::S,
            Role::OnSurfaceVariant,
            "×",
        );
        let clear = va(V::Search(SearchAct::Clear(s)));
        ui.hit_tip((cx, cy, TOUCH, TOUCH), clear, EN.vault_search_clear);
    }
    h + GAP
}

/// The records open vault `v` lists in the kind shown, as its search
/// finds them. A lookup.
pub(crate) fn shown<'a>(app: &'a View, v: &OpenVault) -> Vec<RecordView<'a>> {
    let records = app.vaults.records(v.id);
    app.vaults
        .shown(v.id, app.vaults.category)
        .iter()
        .filter_map(|&i| records.get(i))
        .collect()
}

/// The line in place of the items when none is listed: none of the kind,
/// or none the search's query holds.
pub(crate) fn none_line(app: &View) -> &'static str {
    let searched = crate::vault::search::searchable(app.vaults.category)
        && !app.vaults.query(SearchIn::Items).is_empty();
    if searched {
        EN.vault_search_none
    } else {
        EN.vault_none_here
    }
}

/// The items column on a wide window, from (x, y), `w` wide, down to
/// `bottom`: an item per record, its title whole, wrapped where it does
/// not fit, and its line under it; the selected one, `sel`, kept in view.
pub(crate) fn item_rows(
    app: &View,
    ui: &mut Ui,
    items: &[RecordView<'_>],
    sel: usize,
    (x, y, w): (f32, f32, f32),
    bottom: f32,
) {
    let (ink, muted) = (Role::OnSurface, Role::OnSurfaceVariant);
    if items.is_empty() {
        ui.text(x, y, LABEL, W::R, muted, none_line(app));
        return;
    }
    let room = w - 2.0 * VAULT_ITEM_PAD;
    let row_h = |ui: &Ui, title: &str, mono: bool| {
        let weight = if mono { W::M } else { W::S };
        let lines = ui.wrap_lines(LABEL, weight, title, room);
        VAULT_ITEM_H + (lines - 1) as f32 * ui.line(LABEL, weight)
    };
    // The first item drawn: the selected one stays on screen.
    let mut first = sel.min(items.len() - 1);
    let mut used = {
        let (t, _, mono) = item_heading(app, items[first]);
        row_h(ui, &t, mono) + VAULT_ITEM_GAP
    };
    while first > 0 {
        let (t, _, mono) = item_heading(app, items[first - 1]);
        let h = row_h(ui, &t, mono) + VAULT_ITEM_GAP;
        if y + used + h > bottom {
            break;
        }
        used += h;
        first -= 1;
    }
    let mut iy = y;
    for (k, r) in items.iter().enumerate().skip(first) {
        if iy >= bottom {
            break;
        }
        let (t, line, mono) = item_heading(app, *r);
        let h = row_h(ui, &t, mono);
        let on = k == sel && app.vaults.form.is_none() && !app.vaults.saving;
        ui.fill(
            x,
            iy,
            w,
            h,
            RADIUS_SMALL,
            selected_or(on, Role::SurfaceContainerHighest),
        );
        ui.stroke(
            x,
            iy,
            w,
            h,
            RADIUS_SMALL,
            selected_or(on, Role::OutlineVariant),
        );
        let (ink, muted) = (on_selected(on, ink), on_selected(on, muted));
        let weight = if mono { W::M } else { W::S };
        let tx = x + VAULT_ITEM_PAD;
        let title_h = ui.wrap(tx, iy + VAULT_ITEM_TOP, room, LABEL, weight, ink, &t);
        let line_y = iy + VAULT_ITEM_LINE + title_h - ui.line(LABEL, weight);
        let line = ui.fit(CAPTION, W::R, &line, room);
        ui.text(tx, line_y, CAPTION, W::R, muted, &line);
        ui.hit(x, iy, w, h, va(V::Item(k)));
        iy += h + VAULT_ITEM_GAP;
    }
}

/// The items on a compact window, from (x, y), `w` wide: a list row each,
/// its title whole. Returns their height.
pub(crate) fn compact_rows(
    app: &View,
    ui: &mut Ui,
    items: &[RecordView<'_>],
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let muted = Role::OnSurfaceVariant;
    let mut iy = y;
    for (k, r) in items.iter().enumerate() {
        let (t, line, _) = item_heading(app, *r);
        let action = Some(va(V::Item(k)));
        iy += row(ui, x, iy, w, None, &t, &line, muted.paint(), action);
    }
    if items.is_empty() {
        ui.text(x, iy, LABEL, W::R, muted, none_line(app));
        iy += TOUCH;
    }
    iy - y
}

/// On a compact window while typing goes to the items' search bar,
/// Material's search view: the bar at the page's top and the items it
/// finds under it, in the page `(x0, cw, h)`, its column at `(x, top)`,
/// `w` wide.
pub(crate) fn compact_view(
    app: &View,
    ui: &mut Ui,
    items: &[RecordView<'_>],
    (x0, cw, h): (f32, f32, f32),
    (x, top, w): (f32, f32, f32),
) {
    let mut y = top + bar(app, ui, x, top, w, SearchIn::Items);
    y += compact_rows(app, ui, items, x, y, w);
    crate::compact::screen::finish(app, ui, x0, cw, h, y - top + PAD);
}
