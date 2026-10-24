//! `Screen::Randomness`: Add randomness (`docs/OWN-RANDOMNESS.md` §2.2).
//! The status line, the source chosen (Camera, Dice, Coins, Cards) and
//! its pad, New key's own, with the batch not yet added and its Add.

use super::parts::{buttons_and_next, camera_row, card_pad, coin_pad, dice_pad};
use crate::Action;
use crate::drawing::View;
use crate::fresh::OwnSource;
use crate::fresh::page::{Act, OwnPage};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use faraday_draw::tokens::{
    BACK_LINK_ROW, CAPTION, COLUMN_MAX, GAP, GAP_SMALL, LABEL, MARGIN_COMPACT, MARGIN_WIDE,
    OWN_BUTTON, OWN_LINE_TEXT, OWN_TOP, PAD,
};
use faraday_ui::parts::title;

/// The sources a person picks from, in order: Camera only where the
/// shell has a camera.
const SOURCES: [(OwnSource, &str); 4] = [
    (OwnSource::Camera, EN.own_source_camera),
    (OwnSource::Dice, EN.own_source_dice),
    (OwnSource::Coins, EN.own_source_coins),
    (OwnSource::Cards, EN.own_source_cards),
];

fn own(a: Act) -> Action {
    Action::Own(a)
}

/// Add randomness, in the column at `x0`, `cw` wide.
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(p) = app.own_page() else {
        return;
    };
    let (x, w) = if ui.compact {
        (x0 + MARGIN_COMPACT, cw - 2.0 * MARGIN_COMPACT)
    } else {
        let w = (cw - 4.0 * MARGIN_WIDE).min(COLUMN_MAX);
        (x0 + 2.0 * MARGIN_WIDE, w)
    };
    let top = if ui.compact { PAD } else { OWN_TOP } - app.list_offset;
    let mut y = top;
    // A small panel has the way back and the name in its bar.
    if !ui.compact {
        let name = app.trail_name().unwrap_or(EN.compact_settings);
        crate::vault::screen::back_link(app, ui, x, y, (name, Action::Back));
        y += BACK_LINK_ROW;
        y += title(ui, x, y, EN.own_add_randomness) + PAD;
    }
    y += ui.wrap(
        x,
        y,
        w,
        OWN_LINE_TEXT,
        W::R,
        Role::OnSurfaceVariant,
        &app.own_status(),
    ) + PAD;
    let sources: Vec<(&str, Action)> = SOURCES
        .iter()
        .filter(|(s, _)| *s != OwnSource::Camera || !app.cameras().is_empty() || p.wants_camera())
        .map(|&(s, label)| (label, own(Act::Source(s))))
        .collect();
    let on = sources
        .iter()
        .position(|&(_, a)| a == own(Act::Source(p.source())));
    y += if ui.compact {
        source_grid(ui, (x, y, w), &sources, on)
    } else {
        ui.choices((x, y, w, OWN_BUTTON), LABEL, &sources, on)
    } + PAD;
    y += pad(p, ui, x, y, w);
    y += batch(p, ui, x, y, w);
    let content = y + PAD - top;
    crate::compact::screen::finish(app, ui, x0, cw, h, content);
}

/// The sources on a small panel: two to a row, each half the panel, so
/// every name is whole. Returns its height.
fn source_grid(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    sources: &[(&str, Action)],
    on: Option<usize>,
) -> f32 {
    let bw = (w - GAP) / 2.0;
    let mut rows = 0.0;
    for (i, &(label, action)) in sources.iter().enumerate() {
        let (col, row) = ((i % 2) as f32, (i / 2) as f32);
        let style = Style::chosen(on == Some(i));
        let at = y + row * (OWN_BUTTON + GAP);
        ui.button(
            x + col * (bw + GAP),
            at,
            Some(bw),
            OWN_BUTTON,
            label,
            style,
            action,
        );
        rows = row + 1.0;
    }
    rows * (OWN_BUTTON + GAP) - GAP
}

/// The chosen source's pad, New key's own. Returns its height.
fn pad(p: &OwnPage, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    match p.source() {
        OwnSource::Camera => camera_row(
            ui,
            (x, y, w),
            p.frame_live(),
            p.last_frame(),
            own(Act::Frame),
        ),
        OwnSource::Dice => dice_pad(ui, (x, y, w), &|_| Style::Secondary, |f| own(Act::Roll(f))),
        OwnSource::Coins => coin_pad(ui, x, y, own(Act::Flip(true)), own(Act::Flip(false))),
        OwnSource::Cards => card_pad(
            ui,
            (x, y, w),
            p.rank(),
            p.cards(),
            |r| own(Act::Rank(r)),
            |s| own(Act::Suit(s)),
        ),
        OwnSource::PointerKeys => 0.0,
    }
}

/// What a batch holds before it is added, what the last press could not
/// do, Undo, Clear and Add, and the keys on a wide layout. A picture is
/// added as it is taken, so the camera has none of these but the keys.
/// Returns its height.
fn batch(p: &OwnPage, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let unit = match p.source() {
        OwnSource::Dice => Some((EN.own_dice_roll, EN.own_dice_rolls)),
        OwnSource::Coins => Some((EN.own_coin_flip, EN.own_coin_flips)),
        OwnSource::Cards => Some((EN.own_card, EN.own_cards)),
        OwnSource::Camera | OwnSource::PointerKeys => None,
    };
    if let Some((one, many)) = unit {
        let n = p.batch_len();
        let noun = if n == 1 { one } else { many };
        let line = fill(EN.own_batch, &[&n.to_string(), noun]);
        cy += ui.wrap(x, cy, w, OWN_LINE_TEXT, W::R, Role::OnSurface, &line) + GAP;
        // As New key names it: 99 rolls are enough for 256 bits alone.
        if p.source() == OwnSource::Dice
            && n >= faraday_entropy::DiceRolls::needed(faraday_entropy::Strength::Bits256)
        {
            cy += ui.wrap(x, cy, w, OWN_LINE_TEXT, W::R, Role::Ok, EN.own_enough_256) + GAP;
        }
    }
    if let Some(n) = p.note() {
        cy += ui.wrap(x, cy, w, OWN_LINE_TEXT, W::R, Role::Error, n) + GAP;
    }
    if unit.is_some() {
        cy += GAP_SMALL;
        let edits = [
            (EN.keygen_undo, Style::Ghost, own(Act::Undo)),
            (EN.keygen_clear, Style::Ghost, own(Act::Clear)),
        ];
        let next = (p.batch_len() > 0).then_some((EN.own_add, own(Act::Add)));
        cy += buttons_and_next(ui, x, cy, w, &edits, next) + GAP_SMALL;
    }
    if !ui.compact {
        let hint = match p.source() {
            OwnSource::Dice => EN.own_dice_keys,
            OwnSource::Coins => EN.own_coin_keys,
            OwnSource::Cards => EN.keygen_type_the_rank,
            OwnSource::Camera | OwnSource::PointerKeys => EN.own_camera_keys,
        };
        cy += ui.wrap(x, cy, w, CAPTION, W::R, Role::OnSurfaceVariant, hint);
    }
    cy - y
}
