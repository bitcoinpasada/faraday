//! The pad molecules: the dice, coin and card pads that take a person's own randomness
//! (`docs/TEXT-SIZE.md` §3; 09 §3.2). A pad keeps its size whatever the text size, as a
//! keyboard's keys do.

use faraday_atoms::button::Look;
use faraday_atoms::{Pressed, Shown, Ui};
use faraday_tokens::dims::{BODY_S, BUTTON_H, DICE_PAD_KEY_H, GAP};
use faraday_tokens::palette::Role;

/// One face of a pad: a key, its look and the press it sends. A face is a key, so its press is
/// the press itself (V11); its label is fixed wording or a symbol.
pub struct Face<A> {
    /// What the key shows.
    pub label: Shown<'static>,
    /// Its look: chosen, secondary, or disabled where it takes no press.
    pub look: Look,
    /// What a press sends.
    pub press: A,
}

/// A pad.
pub enum PadModel<A> {
    /// The dice's faces, in a row of them or two rows of half. With `touch` given, a single
    /// row only where each face's target, the key and the gap beside it, reaches that side.
    Dice {
        faces: Vec<Face<A>>,
        touch: Option<f32>,
    },
    /// Heads and Tails side by side.
    Coin { heads: Face<A>, tails: Face<A> },
    /// The thirteen ranks, then the four suits, and the deck's count beside the suits (under
    /// them on a small panel).
    Cards {
        ranks: Vec<Face<A>>,
        suits: Vec<Face<A>>,
        deck: Shown<'static>,
    },
}

/// The gap between two keys of the dice pad.
const DICE_GAP: f32 = 8.0;
/// A dice key's widest.
const DICE_KEY_MAX: f32 = 84.0;
/// A coin key's height and its widest.
const COIN_H: f32 = 56.0;
const COIN_MAX: f32 = 160.0;
/// The height a one-row pad takes, the gap under it included.
const ROW_H: f32 = 68.0;
/// The card pad's rank gap, widest key and row pitch.
const RANK_GAP: f32 = 6.0;
const RANK_MAX: f32 = 52.0;
const RANK_ROW: f32 = 52.0;
/// The card pad's suit gap and widest key.
const SUIT_GAP: f32 = 8.0;
const SUIT_MAX: f32 = 64.0;

/// A dice key's width when `row` keys and their gaps share `w`.
fn dice_key_w(w: f32, row: usize) -> f32 {
    let n = row as f32;
    ((w - (n - 1.0) * DICE_GAP) / n).min(DICE_KEY_MAX)
}

/// How a set of equal keys lays out in a width `w` with `gap` between them, each at most
/// `max_w` wide: as columns and a width. A single row where every key, chosen and with its
/// check, has the room; else as many rows of equal length as that takes.
fn grid<A: Pressed>(
    ui: &Ui<'_, A>,
    w: f32,
    gap: f32,
    max_w: f32,
    faces: &[Face<A>],
) -> (usize, f32) {
    let n = faces.len().max(1);
    let need = faces
        .iter()
        .map(|f| ui.button_width(f.label.as_str(), Look::Selected))
        .fold(0.0, f32::max);
    let one = ((w - gap * (n - 1) as f32) / n as f32).min(max_w);
    if one >= need {
        return (n, one);
    }
    let fit = (((w + gap) / (need + gap)) as usize).max(1);
    let rows = n.div_ceil(fit);
    let cols = n.div_ceil(rows);
    let bw = ((w - gap * (cols - 1) as f32) / cols as f32).min(max_w);
    (cols, bw)
}

/// A key at (x, y), `w` by `h`.
fn key<A: Pressed>(ui: &mut Ui<'_, A>, (x, y, w, h): (f32, f32, f32, f32), f: &Face<A>) {
    ui.showing(&f.label, |ui| {
        ui.button(x, y, Some(w), h, f.label.as_str(), f.look, f.press.clone());
    });
}

/// Draws the pad from (x, y), `w` wide. Returns its height.
pub fn draw<A: Pressed>(m: &PadModel<A>, ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32)) -> f32 {
    ui.fixed_text(|ui| match m {
        PadModel::Dice { faces, touch } => dice(ui, (x, y, w), faces, *touch),
        PadModel::Coin { heads, tails } => {
            let bw = ((w - GAP) / 2.0).min(COIN_MAX);
            key(ui, (x, y, bw, COIN_H), heads);
            key(ui, (x + bw + GAP, y, bw, COIN_H), tails);
            ROW_H
        }
        PadModel::Cards { ranks, suits, deck } => cards(ui, (x, y, w), (ranks, suits), deck),
    })
}

/// The dice's faces, `row` to a row where the touch side allows six, else half.
fn dice<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    faces: &[Face<A>],
    touch: Option<f32>,
) -> f32 {
    let n = faces.len().max(1);
    let row = match touch {
        Some(t) if dice_key_w(w, n) + DICE_GAP < t => n.div_ceil(2),
        _ => n,
    };
    let bw = dice_key_w(w, row);
    let step = DICE_PAD_KEY_H + DICE_GAP;
    for (i, f) in faces.iter().enumerate() {
        let (col, line) = ((i % row) as f32, (i / row) as f32);
        key(
            ui,
            (
                x + col * (bw + DICE_GAP),
                y + line * step,
                bw,
                DICE_PAD_KEY_H,
            ),
            f,
        );
    }
    ROW_H + (n.div_ceil(row) - 1) as f32 * step
}

/// The ranks, the suits and the deck's count.
fn cards<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    (ranks, suits): (&[Face<A>], &[Face<A>]),
    deck: &Shown<'static>,
) -> f32 {
    // On a small panel a chosen rank keeps its check, so the ranks wrap into as many rows as
    // that room takes.
    let (cols, bw) = if ui.compact {
        grid(ui, w, RANK_GAP, RANK_MAX, ranks)
    } else {
        (13, ((w - 12.0 * RANK_GAP) / 13.0).min(RANK_MAX))
    };
    for (r, f) in ranks.iter().enumerate() {
        let (col, line) = ((r % cols) as f32, (r / cols) as f32);
        key(
            ui,
            (x + col * (bw + RANK_GAP), y + line * RANK_ROW, bw, BUTTON_H),
            f,
        );
    }
    let mut cy = y + RANK_ROW * ranks.len().div_ceil(cols) as f32;
    // Four to the row's width at most; on a small panel the deck goes under them.
    let sw = ((w - 3.0 * SUIT_GAP) / 4.0).min(SUIT_MAX);
    for (s, f) in suits.iter().enumerate() {
        key(ui, (x + s as f32 * (sw + SUIT_GAP), cy, sw, BUTTON_H), f);
    }
    ui.showing(deck, |ui| {
        if ui.compact {
            cy += 52.0;
            ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, deck.as_str());
            cy += ui.advance_as(BODY_S, 28.0);
        } else {
            ui.text_mid_as(
                (x + 4.0 * 72.0 + 8.0, cy, 44.0),
                BODY_S,
                Role::OnSurfaceVariant,
                deck.as_str(),
            );
            cy += 56.0;
        }
    });
    cy - y
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas};
    use faraday_atoms::{MotionStore, Theme};

    fn dice_model(touch: Option<f32>) -> PadModel<P> {
        let labels = ["1", "2", "3", "4", "5", "6"];
        PadModel::Dice {
            faces: labels
                .iter()
                .enumerate()
                .map(|(i, l)| Face {
                    label: Shown::plain(*l),
                    look: Look::Secondary,
                    press: P(i as u8 + 1),
                })
                .collect(),
            touch,
        }
    }

    fn drawn(m: &PadModel<P>, w: f32) -> (f32, Vec<(u8, i32, i32)>) {
        let mut c = canvas(160);
        let mut store = MotionStore::new();
        let pointer = at_rest(None);
        let mut ui: Ui<'_, P> = Ui::new(&mut c, Theme::default(), 1.0, &mut store, &pointer, true);
        let h = draw(m, &mut ui, (0.0, 0.0, w));
        let hits = ui
            .finish()
            .hits
            .iter()
            .map(|h| (h.press.0, h.rect.x, h.rect.y))
            .collect();
        (h, hits)
    }

    /// Invariant: The dice pad sends the face pressed, and where a face's touch target would fall short in one row the six wrap into two rows of three.
    /// Regression: On a narrow column the six faces stay in one row of keys too small to press, or a face sends another face's press.
    /// Owner: kit
    #[test]
    fn the_dice_pad_wraps_into_two_rows_where_one_row_is_too_narrow_to_press() {
        let (one_h, one) = drawn(&dice_model(None), 400.0);
        assert_eq!(
            one.iter().map(|h| h.0).collect::<Vec<_>>(),
            [1, 2, 3, 4, 5, 6]
        );
        assert!(one.iter().all(|h| h.2 == one[0].2), "{one:?}");
        let (two_h, two) = drawn(&dice_model(Some(400.0)), 400.0);
        assert!(two_h > one_h);
        assert_eq!(
            two[3].1, two[0].1,
            "the fourth face starts the second row: {two:?}"
        );
        assert!(two[3].2 > two[0].2);
    }
}
