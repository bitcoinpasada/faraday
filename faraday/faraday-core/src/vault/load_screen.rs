//! The Load sheet an unlock shows (`vault_load`, F62): a card per wallet
//! the vault holds with the seeds of it there, the Keys card, Unselect all
//! or Select all over them, Not now and Load ticked at its foot.

use crate::drawing::View;
use crate::strings::EN;
use faraday_draw::tokens::{BODY_M, BODY_S, FIELD_H, LABEL_L, TITLE_L, TITLE_M};
use faraday_draw::widgets::Icon;

use crate::Action;
use crate::compact::{buttons, scroll_sheet, sheet_head};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::vault::VaultAction as V;
use crate::vault::load::{LoadCard, LoadSeed, LoadView};

fn va(a: V) -> Action {
    Action::Vault(a)
}

/// Inside a card, from its edge.
const PAD: f32 = 12.0;
/// From a checkbox's left to the text beside it.
const BESIDE: f32 = 28.0;

/// The sheet, over the screen the unlock left.
pub(crate) fn draw(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(view) = app.vault_load_view() else {
        return;
    };
    let may = app.may_load_keys();
    let place = if ui.compact {
        ((8.0, w - 16.0), 8.0, 16.0)
    } else {
        let sw = (w - 48.0).min(640.0);
        (((w - sw) / 2.0, sw), 40.0, 32.0)
    };
    scroll_sheet(ui, place.0, h, place.1, place.2, &mut |ui, x, y, iw| {
        body(ui, x, y, iw, &view, may)
    });
}

/// The title, the count, the cards and the buttons. Returns its height.
fn body(ui: &mut Ui, x: f32, y: f32, w: f32, view: &LoadView, may: bool) -> f32 {
    let mut cy = y;
    cy += if ui.compact {
        sheet_head(ui, x, cy, w, Icon::Download, Role::Primary, &view.title)
    } else {
        ui.icon(x, cy, 30.0, Icon::Download, 18.0, Role::Primary);
        let t = ui.fit_as(TITLE_L.medium(), &view.title, w - 42.0);
        ui.text_mid_as((x + 42.0, cy, 30.0), TITLE_L.medium(), Role::OnSurface, &t);
        46.0
    };
    cy += ui.wrap_as((x, cy, w), LABEL_L, Role::OnSurface, &view.count) + 16.0;
    // Unselect all while any is ticked; Select all once none is.
    let any = view.any_ticked();
    if view.tickable() {
        let t = if any {
            EN.visit_unselect_all
        } else {
            EN.visit_select_all
        };
        let tw = ui.measure_as(LABEL_L, t);
        ui.text_mid_as((x, cy - 6.0, 30.0), LABEL_L, Role::Primary, t);
        ui.press_box(x - 6.0, cy - 6.0, tw + 12.0, 30.0, va(V::LoadTickAll));
        cy += 30.0;
    }
    for c in &view.cards {
        cy += wallet_card(ui, x, cy, w, c) + 10.0;
    }
    if !view.keys.is_empty() {
        cy += keys_card(ui, x, cy, w, &view.keys) + 10.0;
    }
    cy += buttons(
        ui,
        x,
        cy,
        w,
        &[
            (EN.load_sheet_not_now, Style::Secondary, va(V::LoadNotNow)),
            (
                EN.load_sheet_load_ticked,
                Style::Primary.when(any && may),
                va(V::LoadTicked),
            ),
        ],
    );
    cy - y
}

/// "Loaded" at the right of a line at `y`, greyed. Returns the width it
/// took, with its gap.
fn loaded_tag(ui: &mut Ui, right: f32, y: f32) -> f32 {
    let lw = ui.measure_as(BODY_S, EN.load_sheet_loaded);
    ui.text_as(
        right - lw,
        y,
        BODY_S,
        Role::OnSurfaceVariant,
        EN.load_sheet_loaded,
    );
    lw + 10.0
}

/// A seed's line under a wallet, or a key's on the Keys card, from `tx`
/// to `right`: its fingerprint and label, Loaded, and the other wallets
/// it is in. Returns its height.
fn seed_line(ui: &mut Ui, tx: f32, y: f32, right: f32, s: &LoadSeed) -> f32 {
    let mut cy = y;
    let tag = if s.loaded {
        loaded_tag(ui, right, cy)
    } else {
        0.0
    };
    let line = ui.fit_as(BODY_S.mono(), &s.line, right - tx - tag);
    ui.text_as(
        tx,
        cy,
        BODY_S.mono(),
        if s.loaded {
            Role::OnSurfaceVariant
        } else {
            Role::OnSurface
        },
        &line,
    );
    cy += ui.line_as(BODY_S.mono()) + faraday_draw::tokens::GAP_SMALL;
    if let Some(also) = &s.also_line {
        cy += ui.wrap_as((tx, cy, right - tx), BODY_S, Role::OnSurfaceVariant, also) + 2.0;
    }
    cy - y
}

/// A wallet's card: its checkbox (none once everything on it is loaded),
/// name, kind, its seeds here, and what is kept elsewhere. Returns its
/// height.
fn wallet_card(ui: &mut Ui, x: f32, y: f32, w: f32, c: &LoadCard) -> f32 {
    let (ix, right) = (x + PAD, x + w - PAD);
    let tx = ix + BESIDE;
    let mut cy = y + PAD;
    if !c.done {
        ui.checkbox(ix, cy + 1.0, c.ticked, true);
    }
    let tag = if c.loaded {
        loaded_tag(ui, right, cy + 1.0)
    } else {
        0.0
    };
    let name = ui.fit_as(TITLE_M, &c.name, right - tx - tag);
    ui.text_as(
        tx,
        cy,
        TITLE_M,
        if c.done {
            Role::OnSurfaceVariant
        } else {
            Role::OnSurface
        },
        &name,
    );
    cy += ui.line_as(TITLE_M);
    if !c.kind.is_empty() {
        let kind = ui.fit_as(BODY_S, &c.kind, right - tx);
        ui.text_as(tx, cy, BODY_S, Role::OnSurfaceVariant, &kind);
        cy += ui.advance_as(BODY_S, 22.0);
    }
    for s in &c.seeds {
        cy += seed_line(ui, tx, cy, right, s);
    }
    if let Some(e) = &c.elsewhere {
        cy += ui.wrap_as((tx, cy, right - tx), BODY_S, Role::OnSurfaceVariant, e) + 2.0;
    }
    cy += PAD - 4.0;
    ui.stroke(
        x,
        y,
        w,
        cy - y,
        faraday_draw::tokens::SHAPE_M,
        Role::OutlineVariant,
    );
    if !c.done {
        ui.press_box(x, y, w, cy - y, va(V::LoadTick(c.record)));
    }
    cy - y
}

/// The Keys card: the keys in no wallet of the vault, each with its
/// checkbox. Returns its height.
fn keys_card(ui: &mut Ui, x: f32, y: f32, w: f32, keys: &[LoadSeed]) -> f32 {
    let (ix, right) = (x + PAD, x + w - PAD);
    let mut cy = y + PAD;
    ui.text_as(ix, cy, TITLE_M, Role::OnSurface, EN.load_sheet_keys_card);
    cy += ui.advance_as(TITLE_M, 26.0);
    for s in keys {
        let top = cy - 4.0;
        if !s.loaded {
            ui.checkbox(ix, cy, s.ticked, true);
        }
        cy += seed_line(ui, ix + BESIDE, cy + 1.0, right, s) + 4.0;
        if !s.loaded {
            ui.press_box(x, top, w, cy - top, va(V::LoadTick(s.record)));
        }
    }
    cy += PAD - 8.0;
    ui.stroke(
        x,
        y,
        w,
        cy - y,
        faraday_draw::tokens::SHAPE_M,
        Role::OutlineVariant,
    );
    cy - y
}

/// **Seeds with no passphrase stored** (F62): the seeds the open vaults
/// keep without a passphrase, each with its vault; a seed picked, its
/// passphrase field, what the last try gave, and Load.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn pass_seeds(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(p) = app.vaults.pass_seeds.as_ref() else {
        return;
    };
    let seeds = app.vault_passphrase_seeds();
    let may = app.may_load_keys();
    let place = if ui.compact {
        ((8.0, w - 16.0), 8.0, 16.0)
    } else {
        let sw = (w - 48.0).min(560.0);
        (((w - sw) / 2.0, sw), 40.0, 32.0)
    };
    scroll_sheet(ui, place.0, h, place.1, place.2, &mut |ui, x, y, iw| {
        let mut cy = y;
        cy += if ui.compact {
            sheet_head(
                ui,
                x,
                cy,
                iw,
                Icon::Lock,
                Role::Primary,
                EN.pass_seeds_title,
            )
        } else {
            ui.icon(x, cy, 30.0, Icon::Lock, 18.0, Role::Primary);
            let t = ui.fit_as(TITLE_L.medium(), EN.pass_seeds_title, iw - 42.0);
            ui.text_mid_as((x + 42.0, cy, 30.0), TITLE_L.medium(), Role::OnSurface, &t);
            46.0
        };
        let row = |ui: &mut Ui, cy: f32, s: &crate::vault::load::PassSeed| -> f32 {
            let vault = ui.fit_as(BODY_S, &s.vault_name, iw);
            ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, &vault);
            let line = ui.fit_as(BODY_M.mono(), &s.line, iw);
            ui.text_as(x, cy + 20.0, BODY_M.mono(), Role::OnSurface, &line);
            44.0
        };
        let picked = p
            .picked
            .and_then(|(v, r)| seeds.iter().find(|s| s.vault == v && s.record == r));
        match picked {
            None => {
                for s in seeds.iter() {
                    let top = cy;
                    cy += row(ui, cy, s);
                    ui.press_box(
                        x - 4.0,
                        top - 4.0,
                        iw + 8.0,
                        cy - top,
                        va(V::PassphrasePick(s.vault, s.record)),
                    );
                    ui.rule(x, cy, iw, Role::OutlineVariant);
                    cy += 10.0;
                }
                cy += buttons(
                    ui,
                    x,
                    cy,
                    iw,
                    &[(EN.pass_seeds_cancel, Style::Secondary, Action::Cancel)],
                );
            }
            Some(s) => {
                cy += row(ui, cy, s) + 8.0;
                ui.text_as(
                    x,
                    cy,
                    BODY_S,
                    Role::OnSurfaceVariant,
                    EN.pass_seeds_passphrase,
                );
                cy += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
                let empty = app
                    .vaults
                    .prompt
                    .as_ref()
                    .is_none_or(|q| q.text.text.is_empty());
                if let Some(q) = app.vaults.prompt.as_ref() {
                    crate::demo::pass_box(
                        app,
                        ui,
                        (x, cy, iw),
                        &q.text,
                        (
                            app.vaults.typed_shown,
                            app.vaults.focus == Some(crate::vault::Focus::Prompt),
                        ),
                        (va(V::FocusPrompt), va(V::ShowTyped)),
                        crate::demo::PassField::Prompt,
                    );
                    cy += FIELD_H + 10.0 + crate::screens::never_under(app, ui, x, cy, iw);
                    if let Some(e) = &q.error {
                        cy += ui.wrap_as((x, cy, iw), BODY_S, Role::Error, e) + 10.0;
                    }
                }
                let back = if seeds.len() > 1 {
                    (
                        EN.pass_seeds_other_seed,
                        Style::Secondary,
                        va(V::PassphraseList),
                    )
                } else {
                    (EN.pass_seeds_cancel, Style::Secondary, Action::Cancel)
                };
                cy += buttons(
                    ui,
                    x,
                    cy,
                    iw,
                    &[
                        back,
                        (
                            EN.pass_seeds_load,
                            Style::Primary.when(may && !empty),
                            va(V::PromptGo),
                        ),
                    ],
                );
            }
        }
        cy - y
    });
}
