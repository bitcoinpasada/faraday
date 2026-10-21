//! The Load sheet an unlock shows (`vault_load`, F62): a card per wallet
//! the vault holds with the seeds of it there, the Keys card, Unselect all
//! or Select all over them, Not now and Load ticked at its foot.

use crate::drawing::View;
use crate::strings::EN;
use osk_ui::widgets::Icon;

use crate::Action;
use crate::compact::{buttons, scroll_sheet, sheet_head};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::vault_load::{LoadCard, LoadSeed, LoadView};
use crate::vaults::VaultAction as V;

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
        sheet_head(ui, x, cy, w, Icon::Download, ACCENT, &view.title)
    } else {
        ui.icon(x, cy, 30.0, Icon::Download, 18.0, ACCENT);
        let t = ui.fit(20.0, W::S, &view.title, w - 42.0);
        ui.text_mid(x + 42.0, cy, 30.0, 20.0, W::S, TEXT, &t);
        46.0
    };
    cy += ui.wrap(x, cy, w, 14.0, W::S, TEXT, &view.count) + 16.0;
    // Unselect all while any is ticked; Select all once none is.
    let any = view.any_ticked();
    if view.tickable() {
        let t = if any {
            EN.visit_unselect_all
        } else {
            EN.visit_select_all
        };
        let tw = ui.measure(13.0, W::S, t);
        ui.text_mid(x, cy - 6.0, 30.0, 13.0, W::S, ACCENT, t);
        ui.hit(x - 6.0, cy - 6.0, tw + 12.0, 30.0, va(V::LoadTickAll));
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
                if any && may {
                    Style::Primary
                } else {
                    Style::Disabled
                },
                va(V::LoadTicked),
            ),
        ],
    );
    cy - y
}

/// "Loaded" at the right of a line at `y`, greyed. Returns the width it
/// took, with its gap.
fn loaded_tag(ui: &mut Ui, right: f32, y: f32, size: f32) -> f32 {
    let lw = ui.measure(size, W::R, EN.load_sheet_loaded);
    ui.text(right - lw, y, size, W::R, DIM, EN.load_sheet_loaded);
    lw + 10.0
}

/// A seed's line under a wallet, or a key's on the Keys card, from `tx`
/// to `right`: its fingerprint and label, Loaded, and the other wallets
/// it is in. Returns its height.
fn seed_line(ui: &mut Ui, tx: f32, y: f32, right: f32, s: &LoadSeed) -> f32 {
    let mut cy = y;
    let tag = if s.loaded {
        loaded_tag(ui, right, cy, 12.0)
    } else {
        0.0
    };
    let line = ui.fit(13.0, W::M, &s.line, right - tx - tag);
    ui.text(tx, cy, 13.0, W::M, if s.loaded { DIM } else { TEXT }, &line);
    cy += 20.0;
    if let Some(also) = &s.also_line {
        cy += ui.wrap(tx, cy, right - tx, 12.0, W::R, MUTED, also) + 2.0;
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
        loaded_tag(ui, right, cy + 1.0, 12.0)
    } else {
        0.0
    };
    let name = ui.fit(15.0, W::S, &c.name, right - tx - tag);
    ui.text(tx, cy, 15.0, W::S, if c.done { DIM } else { TEXT }, &name);
    cy += 24.0;
    if !c.kind.is_empty() {
        let kind = ui.fit(12.0, W::R, &c.kind, right - tx);
        ui.text(tx, cy, 12.0, W::R, MUTED, &kind);
        cy += 22.0;
    }
    for s in &c.seeds {
        cy += seed_line(ui, tx, cy, right, s);
    }
    if let Some(e) = &c.elsewhere {
        cy += ui.wrap(tx, cy, right - tx, 12.0, W::R, MUTED, e) + 2.0;
    }
    cy += PAD - 4.0;
    ui.stroke(x, y, w, cy - y, 10.0, BORDER);
    if !c.done {
        ui.hit(x, y, w, cy - y, va(V::LoadTick(c.record)));
    }
    cy - y
}

/// The Keys card: the keys in no wallet of the vault, each with its
/// checkbox. Returns its height.
fn keys_card(ui: &mut Ui, x: f32, y: f32, w: f32, keys: &[LoadSeed]) -> f32 {
    let (ix, right) = (x + PAD, x + w - PAD);
    let mut cy = y + PAD;
    ui.text(ix, cy, 15.0, W::S, TEXT, EN.load_sheet_keys_card);
    cy += 26.0;
    for s in keys {
        let top = cy - 4.0;
        if !s.loaded {
            ui.checkbox(ix, cy, s.ticked, true);
        }
        cy += seed_line(ui, ix + BESIDE, cy + 1.0, right, s) + 4.0;
        if !s.loaded {
            ui.hit(x, top, w, cy - top, va(V::LoadTick(s.record)));
        }
    }
    cy += PAD - 8.0;
    ui.stroke(x, y, w, cy - y, 10.0, BORDER);
    cy - y
}

/// **Seeds with no passphrase stored** (F62): the seeds the open vaults
/// keep without a passphrase, each with its vault; a seed picked, its
/// passphrase field, what the last try gave, and Load.
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
            sheet_head(ui, x, cy, iw, Icon::Lock, ACCENT, EN.pass_seeds_title)
        } else {
            ui.icon(x, cy, 30.0, Icon::Lock, 18.0, ACCENT);
            let t = ui.fit(20.0, W::S, EN.pass_seeds_title, iw - 42.0);
            ui.text_mid(x + 42.0, cy, 30.0, 20.0, W::S, TEXT, &t);
            46.0
        };
        let row = |ui: &mut Ui, cy: f32, s: &crate::vault_load::PassSeed| -> f32 {
            let vault = ui.fit(12.0, W::R, &s.vault_name, iw);
            ui.text(x, cy, 12.0, W::R, MUTED, &vault);
            let line = ui.fit(14.0, W::M, &s.line, iw);
            ui.text(x, cy + 20.0, 14.0, W::M, TEXT, &line);
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
                    ui.hit(
                        x - 4.0,
                        top - 4.0,
                        iw + 8.0,
                        cy - top,
                        va(V::PassphrasePick(s.vault, s.record)),
                    );
                    ui.rule(x, cy, iw, INNER);
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
                ui.text(x, cy, 12.0, W::R, MUTED, EN.pass_seeds_passphrase);
                cy += 20.0;
                let empty = app
                    .vaults
                    .prompt
                    .as_ref()
                    .is_none_or(|q| q.text.text.is_empty());
                if let Some(q) = app.vaults.prompt.as_ref() {
                    crate::vault_screens::secret_box(
                        ui,
                        x,
                        cy,
                        iw,
                        &q.text,
                        app.vaults.typed_shown,
                        app.vaults.focus == Some(crate::vaults::Focus::Prompt),
                        va(V::FocusPrompt),
                        va(V::ShowTyped),
                    );
                    cy += 50.0;
                    if let Some(e) = &q.error {
                        cy += ui.wrap(x, cy, iw, 13.0, W::R, ERR, e) + 10.0;
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
                            if may && !empty {
                                Style::Primary
                            } else {
                                Style::Disabled
                            },
                            va(V::PromptGo),
                        ),
                    ],
                );
            }
        }
        cy - y
    });
}
