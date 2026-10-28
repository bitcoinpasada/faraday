//! New key's dice in the input panel (`docs/LAYOUT.md` §16.3): the
//! progress line, with the roll just made, and the pad, in one view,
//! pinned at the window's foot across the page area while the Entries
//! card takes rolls one press at a time, the page scrolling clear above
//! it, as Make codex32 shares' panel stands. Where it would take more
//! than `INPUT_PANEL_MAX` of the window's height, and on the small panel,
//! both stay in the card. Where it stands is arithmetic on the window;
//! drawing reads the flow's state.

use faraday_draw::tokens as t;
use faraday_draw::tokens::{BODY_M, TITLE_M};
use faraday_entropy::{RANKS, SUITS};

use super::{KeyGen, Source, kstep};
use crate::Action;
use crate::drawing::View;
use crate::screens::dice_pad;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};

/// The panel's height: the progress line and the pad inside its padding.
const PANEL_H: f32 = t::PAD + t::ENTRY_PROGRESS_H + t::DICE_PAD_KEY_H + t::PAD;

/// The panel's height where it stands at the foot of the page area,
/// else `None`: the Entries card open on dice pressed one at a time, on
/// a window that is not the small panel, where the panel takes at most
/// `INPUT_PANEL_MAX` of the window's height. Arithmetic.
pub(crate) fn place(app: &View, k: &KeyGen) -> Option<f32> {
    let rolling = k.open == Some(kstep::ENTER) && k.active() == Some(Source::Dice) && !k.typing;
    let fits = crate::codex32::input_panel::foot_fits(app.window(), PANEL_H, app.size().1);
    (rolling && fits).then_some(PANEL_H)
}

/// Draws the panel over the foot of the page area at `x0`, `cw` wide,
/// in a window `h` tall, `fh` of it the panel's; its parts in the card
/// column at `cx`, `w` wide.
pub(crate) fn draw(
    k: &KeyGen,
    ui: &mut Ui,
    (x0, cw, h, fh): (f32, f32, f32, f32),
    (cx, w): (f32, f32),
) {
    let top = h - fh;
    crate::codex32::input_panel::cover(ui, (x0, top, cw, fh));
    // At the card body's start on the wide layout; at the one-column
    // page's margins elsewhere, as its cards stand.
    let (x, w) = if ui.compact {
        let m = crate::compact::M;
        (x0 + m, (cw - 2.0 * m).min(t::COLUMN_MAX))
    } else {
        let i = crate::flow::BODY_INDENT;
        (cx + i, (w - 2.0 * i).min(t::COLUMN_MAX))
    };
    let y = top + t::PAD;
    let y = y + progress(k, ui, x, y, w);
    pad(k, ui, (x, y, w));
}

/// The pad of the dice's faces, a face the direct procedure would not
/// take now disabled. Returns its height with the gap under it.
pub(crate) fn pad(k: &KeyGen, ui: &mut Ui, at: (f32, f32, f32)) -> f32 {
    let style = |f| {
        if k.procedure().direct() && !k.procedure().accepts(k.dice.len(), f) {
            Style::Disabled
        } else {
            Style::Secondary
        }
    };
    dice_pad(ui, at, &style, Action::KRoll)
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
