//! The plan's Access layers card drawn (`docs/BACKUP-FLOW.md` §20.3,
//! DESIGN §4.17): each layer with its × and its routes, each route with
//! its × and its places as any k of n (a − k + stepper and the places
//! ticked), whether it is its layer's paper route, the passphrases it
//! remembers and, off paper, each vault's password; what each of its
//! places holds when set by hand (§20.13: a − n + stepper for the seeds,
//! and choices for their form, the description and a vault); **+ route**,
//! **+ layer**, **Work it out**, and each conflict with what would resolve
//! it. Labels, values and actions only. What a press does is `layers.rs`.

use osk_ui::tokens;
use osk_ui::widgets::Icon;

use crate::access::{Described, KindVault, SeedForm};
use crate::layers::{KindsView, LayersPress as P, LayersView};
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::{Action, Faraday};

/// A row of the card: a stepper's or a password's, a button's height.
const ROW: f32 = tokens::BOARD_HEAD + tokens::GAP_SMALL;

fn la(p: P) -> Action {
    Action::Layers(p)
}

fn u(n: usize) -> u8 {
    u8::try_from(n).unwrap_or(u8::MAX)
}

/// A head row `h` tall at (x, y), `w` wide: `label` in `size`, semibold,
/// in `color`, and its × at the right when it has one.
#[allow(clippy::too_many_arguments)]
fn head(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    size: f32,
    color: osk_ui::Color,
    label: &str,
    off: Option<Action>,
) {
    let mark = tokens::BOARD_MARK;
    let side = mark + 2.0 * tokens::BOARD_PAD;
    let room = if off.is_some() { w - side } else { w };
    let shown = ui.fit(size, W::S, label, room);
    ui.text_mid(x, y, h, size, W::S, color, &shown);
    if let Some(a) = off {
        crate::board_screen::cross(
            ui,
            x + w - tokens::BOARD_PAD - mark / 2.0,
            y + h / 2.0,
            MUTED,
        );
        ui.hit(x + w - side, y, side, h, a);
    }
}

/// A layer's head row at (x, y), `w` wide, while names are typed: its
/// name a field pressed with `press` (typed while `typing`, an accent rule
/// under it), and its × when it has one.
fn name_head(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    name: &str,
    press: Action,
    typing: bool,
    off: Option<Action>,
) {
    let h = tokens::BOARD_HEAD;
    let mark = tokens::BOARD_MARK;
    let side = mark + 2.0 * tokens::BOARD_PAD;
    let room = if off.is_some() { w - side } else { w };
    if typing {
        ui.typed(x, y, h, tokens::LABEL, W::S, TEXT, name, room, true, press);
        ui.rule(x, y + h - tokens::GAP_SMALL, room, ACCENT);
    } else {
        let shown = ui.fit(tokens::LABEL, W::S, name, room);
        ui.text_mid(x, y, h, tokens::LABEL, W::S, TEXT, &shown);
    }
    ui.hit(x, y, room, h, press);
    if let Some(a) = off {
        crate::board_screen::cross(
            ui,
            x + w - tokens::BOARD_PAD - mark / 2.0,
            y + h / 2.0,
            MUTED,
        );
        ui.hit(x + w - side, y, side, h, a);
    }
}

/// A choice at (x, y), `w` wide: its label on a line of its own, then a
/// button per option, the one chosen marked. Returns its height.
fn choice(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    options: &[(&str, bool, Action)],
    on: bool,
) -> f32 {
    let h = tokens::BOARD_STICK_HEAD;
    let shown = ui.fit(tokens::CAPTION, W::R, label, w);
    ui.text_mid(x, y, h, tokens::CAPTION, W::R, MUTED, &shown);
    let items: Vec<(&str, Style, Action)> = options
        .iter()
        .map(|&(name, chosen, a)| {
            let style = match (on, chosen) {
                (false, _) => Style::Disabled,
                (true, true) => Style::Secondary,
                (true, false) => Style::Ghost,
            };
            (name, style, a)
        })
        .collect();
    h + faraday_ui::parts::wrap_buttons(ui, x, y + h, w, tokens::BOARD_CHIP, &items)
}

/// A route's kinds at (x, y), `w` wide (§20.13 item 1): the seeds at
/// each place on a stepper, their form off paper, the description for a
/// wallet of more than one seed, and off paper a vault. Returns their
/// height.
#[allow(clippy::too_many_arguments)]
fn kinds(ui: &mut Ui, x: f32, y: f32, w: f32, l8: u8, r8: u8, k: &KindsView, on: bool) -> f32 {
    let gap = tokens::GAP;
    let mut cy = y;
    crate::screens::stepper(
        ui,
        x,
        cy,
        w,
        EN.layers_seeds_each,
        k.seeds,
        la(P::KindsFewerSeeds(l8, r8)),
        la(P::KindsMoreSeeds(l8, r8)),
        on,
    );
    cy += ROW + gap;
    if let Some(form) = k.form {
        let options = [
            (
                EN.layers_words_on_paper,
                form == SeedForm::Paper,
                la(P::KindsForm(l8, r8, SeedForm::Paper)),
            ),
            (
                EN.layers_file_on_stick,
                form == SeedForm::Stick,
                la(P::KindsForm(l8, r8, SeedForm::Stick)),
            ),
        ];
        cy += choice(ui, x, cy, w, EN.layers_seeds_as, &options, on);
    }
    if let Some(d) = k.description {
        let options = [
            (Described::None, EN.layers_none),
            (Described::Part, EN.layers_part),
            (Described::Whole, EN.layers_whole),
        ]
        .map(|(x, name)| (name, d == x, la(P::KindsDescription(l8, r8, x))));
        cy += choice(ui, x, cy, w, EN.layers_description, &options, on);
    }
    if let Some(v) = k.vault {
        let options = [
            (KindVault::None, EN.layers_none),
            (KindVault::Remembered, EN.layers_remembered),
            (KindVault::Written, EN.layers_written),
        ]
        .map(|(x, name)| (name, v == x, la(P::KindsVault(l8, r8, x))));
        cy += choice(ui, x, cy, w, EN.layers_vault, &options, on);
    }
    cy + gap - y
}

/// The Access layers card's body at (x, y), `w` wide; `on`, its rows take
/// presses. Returns its height.
pub(crate) fn card(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32, on: bool) -> f32 {
    let Some(v) = app.layers_view() else {
        return 0.0;
    };
    let v: &LayersView = &v;
    let gap = tokens::GAP;
    let inset = tokens::BOARD_PAD;
    let style = if on {
        Style::Secondary
    } else {
        Style::Disabled
    };
    let mut cy = y;
    let many = v.layers.len() > 1;
    for (l, layer) in v.layers.iter().enumerate() {
        let l8 = u(l);
        let off = (many && on).then_some(la(P::RemoveLayer(l8)));
        if v.names_typed && on {
            // A press on the name types it, as a place's is (§19.18 item
            // 3): just pressed, the whole of it selected and no caret.
            name_head(
                ui,
                (x, cy, w),
                &layer.name,
                Action::BLayerName(l8),
                v.naming == Some(l),
                off,
            );
        } else {
            head(
                ui,
                x,
                cy,
                w,
                tokens::BOARD_HEAD,
                tokens::LABEL,
                TEXT,
                &layer.name,
                off,
            );
        }
        cy += tokens::BOARD_HEAD;
        let (rx, rw) = (x + inset, w - inset);
        let routes = layer.routes.len() > 1;
        for (r, route) in layer.routes.iter().enumerate() {
            let r8 = u(r);
            head(
                ui,
                rx,
                cy,
                rw,
                tokens::BOARD_STICK_HEAD,
                tokens::CAPTION,
                MUTED,
                &route.title,
                (routes && on).then_some(la(P::RemoveRoute(l8, r8))),
            );
            cy += tokens::BOARD_STICK_HEAD;
            crate::screens::stepper(
                ui,
                rx,
                cy,
                rw,
                &route.any.0,
                route.any.1,
                la(P::Fewer(l8, r8)),
                la(P::More(l8, r8)),
                on,
            );
            cy += ROW + gap;
            let mut rows: Vec<(String, bool, bool, Action)> = route
                .places
                .iter()
                .enumerate()
                .map(|(p, (name, held))| (name.clone(), *held, on, la(P::Place(l8, r8, u(p)))))
                .collect();
            rows.push((
                EN.layers_paper.to_string(),
                route.paper,
                on,
                la(P::Paper(l8, r8)),
            ));
            for (label, held, i) in &route.passphrases {
                rows.push((label.clone(), *held, on, la(P::Passphrase(l8, r8, u(*i)))));
            }
            rows.push((
                EN.layers_kinds_set.to_string(),
                route.kinds.is_some(),
                on,
                la(P::KindsSet(l8, r8)),
            ));
            cy += ui.multi_list(rx, cy, rw, &rows) + gap;
            if let Some(k) = &route.kinds {
                cy += kinds(ui, rx, cy, rw, l8, r8, k, on);
            }
            // A vault opens on a route off paper only (§20.1).
            if !route.paper {
                for (k, (label, state)) in route.passwords.iter().enumerate() {
                    let bw = (ui.measure(tokens::LABEL, W::S, EN.layers_written)
                        + 2.0 * tokens::BOARD_HEAD)
                        .min(rw / 2.0);
                    let shown = ui.fit(tokens::LABEL, W::R, label, rw - bw - gap);
                    ui.text_mid(rx, cy, ROW, tokens::LABEL, W::R, TEXT, &shown);
                    ui.button(
                        rx + rw - bw,
                        cy,
                        Some(bw),
                        ROW,
                        state,
                        style,
                        la(P::Password(l8, r8, u(k))),
                    );
                    cy += ROW + gap;
                }
            }
        }
        if on {
            cy += faraday_ui::parts::wrap_buttons(
                ui,
                rx,
                cy,
                rw,
                tokens::BOARD_CHIP,
                &[(EN.layers_add_route, Style::Ghost, la(P::AddRoute(l8)))],
            ) + gap;
        }
        cy += gap;
    }
    if !on {
        return cy - y;
    }
    let mut buttons = vec![(EN.layers_add_layer, Style::Ghost, la(P::AddLayer))];
    if v.work {
        buttons.push((EN.layers_work_out, Style::Secondary, la(P::WorkItOut)));
    }
    cy += faraday_ui::parts::wrap_buttons(ui, x, cy, w, ROW, &buttons) + gap;
    // The conflicts (§20.4): a line each, and what would resolve it.
    for c in &v.conflicts {
        cy += gap;
        let side = tokens::BOARD_MARK;
        ui.icon(x, cy + tokens::GAP_SMALL, side, Icon::Warning, side, WARN);
        cy += ui.wrap(
            x + side + gap,
            cy,
            w - side - gap,
            tokens::LABEL,
            W::R,
            WARN,
            &c.line,
        ) + gap;
        if !c.fixes.is_empty() {
            let fixes: Vec<(&str, Style, Action)> = c
                .fixes
                .iter()
                .map(|(label, press)| (label.as_str(), Style::Secondary, la(*press)))
                .collect();
            cy += faraday_ui::parts::wrap_buttons(
                ui,
                x + side + gap,
                cy,
                w - side - gap,
                tokens::BOARD_CHIP,
                &fixes,
            ) + gap;
        }
    }
    cy - y
}
