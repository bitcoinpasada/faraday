//! New key's dice in the Entries card: the progress line, with the roll
//! just made, and the pad of faces, both in the card (`docs/LAYOUT.md`
//! §16.3). Drawing reads the flow's state.

use faraday_draw::tokens::{BODY_M, TITLE_M};
use faraday_entropy::{RANKS, SUITS};

use super::{KeyGen, Source};
use crate::Action;
use crate::screens::{dice_pad, dice_pad_fit};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};

/// The pad of the dice's faces, every one pressable: direct selection
/// shows a 5 or a 6 in a word's first five rolls struck and rolled again.
/// Returns its height with the gap under it.
pub(crate) fn pad(ui: &mut Ui, at: (f32, f32, f32), full: bool) -> f32 {
    dice_pad(ui, at, &|_| face_style(full), Action::KRoll)
}

/// The pad as [`pad`] draws it, in two rows of three where one row of
/// six would leave a face's target under `touch`.
pub(crate) fn pad_fit(ui: &mut Ui, at: (f32, f32, f32), touch: f32, full: bool) -> f32 {
    dice_pad_fit(ui, at, touch, &|_| face_style(full), Action::KRoll)
}

/// A face's style: greyed once every roll is in.
fn face_style(full: bool) -> Style {
    if full {
        Style::Disabled
    } else {
        Style::Secondary
    }
}

/// How far along, as a count and a bar, with the entry just made at the
/// count's end. Returns its height.
pub(crate) fn progress(k: &KeyGen, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let muted = Role::OnSurfaceVariant;
    let (have, need) = k.progress();
    let active = k.active();
    let mut cy = y;
    let unit = match active {
        Some(Source::Dice) => EN.keygen_rolls_inline,
        Some(Source::Coins) => EN.keygen_flips_inline,
        Some(Source::Cards) => EN.keygen_cards_inline,
        Some(Source::Hex) => EN.keygen_digits_inline,
        _ => EN.keygen_pictures_inline,
    };
    ui.text_as(
        x,
        cy,
        TITLE_M,
        Role::OnSurface,
        &fill(
            EN.keygen_entry_count_unit,
            &[&have.to_string(), &need.to_string(), unit],
        ),
    );
    if let Some(last) = last_entry(k) {
        ui.text_right_as(
            (x + w, cy - 6.0, 30.0),
            BODY_M.mono(),
            muted,
            &fill(EN.keygen_last, &[&last.to_string()]),
        );
    }
    cy += 28.0;
    ui.fill(x, cy, w, 6.0, 3.0, Role::SurfaceContainerHighest);
    let f = if need == 0 {
        0.0
    } else {
        (have as f32 / need as f32).min(1.0)
    };
    if f > 0.0 {
        ui.fill(
            x,
            cy,
            w * f,
            6.0,
            3.0,
            if have >= need {
                Role::Ok
            } else {
                Role::Primary
            },
        );
    }
    cy += 22.0;
    cy - y
}

/// The entry just made, shown once so a mistyped one can be taken back.
fn last_entry(k: &KeyGen) -> Option<String> {
    match k.active()? {
        Source::Dice if k.by_die => k
            .coins
            .last()
            .map(|h| if h { EN.create_heads } else { EN.create_tails }.to_string()),
        Source::Dice => k.dice.last().map(|r| r.to_string()),
        Source::Coins => k
            .coins
            .last()
            .map(|h| if h { EN.create_heads } else { EN.create_tails }.to_string()),
        Source::Cards => k.cards.last().map(|i| {
            let (r, s) = faraday_entropy::card_parts(i);
            format!("{}{}", RANKS[usize::from(r)], SUITS[usize::from(s)])
        }),
        Source::Hex => k.hex.last().map(|c| c.to_string()),
        _ => None,
    }
}
