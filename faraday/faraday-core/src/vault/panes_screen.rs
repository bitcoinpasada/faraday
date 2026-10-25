//! Vaults as list and detail (`docs/LAYOUT.md` §5, §17): from expanded
//! windows up, the vaults in the list pane and the one chosen beside it,
//! its card or, once chosen, its Unlock. A vault open is chosen into its
//! contents, which are a list and detail of their own and take the page.
//! Also how a vault's row reads, which every list of vaults shares.

use crate::drawing::View;
use crate::panes::screen::{Item, ListPane, split};
use crate::screens::{guide_text, title};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::vault::{self, VaultAction as V};
use crate::{Action, Screen};
use faraday_draw::tokens::{BUTTON_H, GAP, LIST_PAD, MARGIN_WIDE, SHAPE_M, TOUCH};
use faraday_draw::widgets::Icon;

/// How a vault's row reads: its title, its state and the state's colour,
/// its icon's colour and its edge.
pub(crate) struct Look {
    /// The vault's name, or what it is while locked.
    pub title: String,
    /// Open, locked, written or not, changes waiting.
    pub state: String,
    /// The state's colour.
    pub tone: Role,
    /// The lock icon's colour.
    pub icon: Role,
    /// The row's edge.
    pub edge: Role,
}

/// How vault file `f` reads in a list of vaults. Reads the state.
pub(crate) fn look(app: &View, f: &vault::VaultFile) -> Look {
    let (primary, good, warn) = (Role::Primary, Role::Ok, Role::Warning);
    let (muted, rule) = (Role::OnSurfaceVariant, Role::OutlineVariant);
    let open = f.open.and_then(|o| app.vaults.open.get(o));
    let in_inbox = app
        .inbox
        .iter()
        .filter(|i| i.kind == crate::FileKind::Vault)
        .any(|i| faraday_vault::read_header(&i.bytes).is_ok_and(|h| h.salt == f.header.salt));
    let fresh = f.in_outbox && !in_inbox;
    // A locked vault seen open since power-on goes by its name.
    let remembered = app.vault_summary(f).map(|s| s.name.as_str());
    let (title, state, tone, icon, edge) = match open {
        Some(v) if v.changes > 0 => (
            v.label(app.vaults.store()),
            fill(
                EN.vault_open_unsaved,
                &[
                    &v.changes.to_string(),
                    (if v.changes == 1 {
                        EN.vault_change
                    } else {
                        EN.vault_changes
                    }),
                ],
            ),
            warn,
            primary,
            rule,
        ),
        Some(v) => (
            v.label(app.vaults.store()),
            EN.vault_open_button.to_string(),
            good,
            primary,
            rule,
        ),
        None if !app.may_load_keys() => (
            remembered
                .unwrap_or(if fresh {
                    EN.vault_new_vault
                } else {
                    EN.vault_locked_vault
                })
                .to_string(),
            fill(EN.vault_remove_to_unlock, &[app.medium.noun()]),
            warn,
            muted,
            rule,
        ),
        None if fresh => (
            remembered.unwrap_or(EN.vault_new_vault).to_string(),
            EN.vault_locked_not_written.to_string(),
            good,
            Role::OnOkContainer,
            Role::OkContainer,
        ),
        None => (
            remembered.unwrap_or(EN.vault_locked_vault).to_string(),
            EN.vault_locked.to_string(),
            muted,
            muted,
            rule,
        ),
    };
    Look {
        title,
        state,
        tone,
        icon,
        edge,
    }
}

/// The vault the detail shows: Unlock's, else the first.
fn chosen(app: &View) -> usize {
    if app.screen == Screen::Unlock {
        app.vaults.pick
    } else {
        0
    }
}

/// Vaults and a vault's Unlock from expanded windows up: the list pane
/// and, beside it, the chosen vault's card or its Unlock.
pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let p = split(ui, x0, cw);
    let (lx, lw) = p.list;
    if !ui.compact {
        title(ui, lx, 28.0, EN.vault_vaults);
    }
    let files = app.vault_files();
    let at = chosen(app);
    let mut pane = ListPane::begin(app, ui, (lx, p.top, lw, h - p.top));
    for (i, f) in files.iter().enumerate() {
        let l = look(app, f);
        let item = Item {
            headline: &l.title,
            supporting: &l.state,
            trailing: None,
            action: Action::Vault(V::Open(i)),
        };
        pane.item(ui, &item, i == at);
    }
    // Made from another flow, the way back is the bar's or the detail's,
    // and no second vault is started in its place.
    if app.vaults.back_to.is_none() {
        pane.advance(GAP);
        let y = pane.at().2;
        let create = EN.vault_create_a_vault;
        let style = Style::Secondary;
        ui.button(
            lx + LIST_PAD,
            y,
            None,
            BUTTON_H,
            create,
            style,
            va(V::Create),
        );
        pane.advance(BUTTON_H + GAP);
    }
    pane.end(ui);
    let (dx, _) = p.detail;
    let w = p.reading();
    if app.screen == Screen::Unlock {
        // Unlock's own page, beside the list that chose its vault.
        let inset = if ui.compact {
            crate::compact::M
        } else {
            2.0 * MARGIN_WIDE
        };
        return crate::vault::screen::unlock(app, ui, dx - inset, w + 2.0 * inset, h);
    }
    let clip = ui.rect(dx, p.top, p.detail.1, h - p.top);
    ui.c.push_clip(clip);
    let top = p.top - app.list_offset;
    let mut y = top;
    match files.get(at) {
        Some(f) => y += card(app, ui, (dx, y, w), at, f) + MARGIN_WIDE,
        None => {
            let muted = Role::OnSurfaceVariant;
            y += ui.wrap(dx, y, w, 15.0, W::R, muted, EN.vault_no_vault_in_files) + GAP;
            y += crate::samples::put::button(app, ui, dx, y) + MARGIN_WIDE;
        }
    }
    let about = fill(
        EN.vault_a_vault_holds,
        &[app.medium.a(), app.medium.for_the()],
    );
    y += guide_text(app, ui, dx, y, w, &about);
    ui.c.pop_clip();
    app.scroll.content_h().set(y - top);
    ui.report_scroll(clip, y - top - (h - p.top));
}

/// Vault `i`'s card in the detail: its lock, title and file, what it
/// held or where it is, its state, and Unlock, or View contents and Lock.
/// Returns its height.
fn card(
    app: &View,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    i: usize,
    f: &vault::VaultFile,
) -> f32 {
    let l = look(app, f);
    let pad = 2.0 * GAP + GAP / 2.0;
    let (ix, iw) = (x + pad, w - 2.0 * pad);
    let muted = Role::OnSurfaceVariant;
    let where_ = if f.in_outbox {
        app.medium.for_box()
    } else {
        app.medium.from_box()
    };
    let line = super::screen::list_line(app, f, where_);
    let head = TOUCH + GAP;
    let line_h =
        crate::compact::screen::measured(ui, |ui| ui.wrap(ix, y, iw, 13.0, W::R, muted, &line));
    let ch = pad + head + line_h + GAP + TOUCH / 2.0 + GAP + BUTTON_H + pad;
    ui.fill(x, y, w, ch, SHAPE_M, Role::SurfaceContainerLow);
    ui.stroke(x, y, w, ch, SHAPE_M, l.edge);
    let mut cy = y + pad;
    ui.fill(ix, cy, TOUCH, TOUCH, GAP + GAP / 4.0, l.icon.with_alpha(30));
    ui.icon(ix, cy, TOUCH, Icon::Lock, 20.0, l.icon);
    let tx = ix + TOUCH + 2.0 * GAP;
    let tw = iw - TOUCH - 2.0 * GAP;
    let t = ui.fit(16.0, W::S, &l.title, tw);
    ui.text(tx, cy, 16.0, W::S, Role::OnSurface, &t);
    let name = ui.fit(12.0, W::M, &f.name, tw);
    ui.text(tx, cy + TOUCH / 2.0 + GAP / 2.0, 12.0, W::M, muted, &name);
    cy += head;
    cy += ui.wrap(ix, cy, iw, 13.0, W::R, muted, &line) + GAP;
    ui.text(ix, cy, 13.0, W::S, l.tone, &l.state);
    cy += TOUCH / 2.0 + GAP;
    // An open vault: what is in it, and Lock, which seals it (and every
    // other) into the Outbox. A locked one: Unlock.
    let buttons: &[(&str, Style, Action)] = if f.open.is_some() {
        &[
            (EN.vault_view_contents, Style::Secondary, va(V::Open(i))),
            (EN.vault_lock, Style::Primary, Action::LockAsk),
        ]
    } else {
        &[(EN.vault_unlock, Style::Primary, va(V::Open(i)))]
    };
    crate::screens::wrap_buttons(ui, ix, cy, iw, BUTTON_H, buttons);
    ch
}

fn va(a: V) -> Action {
    Action::Vault(a)
}
