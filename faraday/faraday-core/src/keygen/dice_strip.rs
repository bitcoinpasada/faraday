//! The word being made, in New key's Entries card for Dice words: its
//! rolls in slots, each with the bits it gives (the faces 1 to 4 two bits
//! apiece, the sixth roll one), the 11 bits as the word's number, and the
//! word. A 5 or a 6 in a word's first five rolls shows struck under
//! "Rolled again" and is not a roll (`KeyGen::rerolled`). The last word
//! takes the rolls its kept bits need; its checksum bits come in the
//! checksum's colour. Drawing reads the flow's state.

use faraday_bip::bip39::Language;
use faraday_draw::tokens as t;
use faraday_draw::tokens::{BODY_M, BODY_S, LABEL_L, LABEL_S, TITLE_L};
use faraday_draw::widgets::Icon;

use super::{KeyGen, Way};
use crate::Action;
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::Shape;
use crate::ui::Ui;
use crate::ui::pal::*;

/// Whether the strip stands: New key's dice under direct selection.
pub(crate) fn stands(k: &KeyGen) -> bool {
    k.way() == Some(Way::DiceWords)
}

/// What a roll of `face` at place `p` in its word gives: its bits.
pub(super) fn bits_of(p: usize, face: u8, last: bool) -> &'static str {
    if !last && p >= faraday_entropy::DICE_WORD_FACES {
        if face >= 4 { "1" } else { "0" }
    } else {
        ["00", "01", "10", "11"][usize::from(face.saturating_sub(1) & 3)]
    }
}

/// The ink on a tint: the two alternate by roll.
fn tint(odd: bool) -> (Role, Role) {
    if odd {
        (Role::SurfaceContainerHighest, Role::OnSurface)
    } else {
        (Role::SecondaryContainer, Role::OnSecondaryContainer)
    }
}

/// One of the number's 11 boxes.
#[derive(Clone, Copy)]
enum Bit {
    Empty,
    Rolled(char, bool),
    Dropped(char),
    Checksum(char),
}

/// Draws the strip at (x, y) in a column `w` wide. Returns its height
/// with the gap under it, and where the word's pill stands while it
/// glides into Words so far.
pub(crate) fn strip(
    app: &View,
    k: &KeyGen,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
) -> (f32, Option<(f32, f32)>) {
    let muted = Role::OnSurfaceVariant;
    let rolls = k.dice.rolls();
    let per = faraday_entropy::DICE_WORD_ROLLS;
    let total = k.dice_needed();
    let last_n = k.words - 1;
    let (last_rolls, dropped) = k.last_rolls();
    let kept = k.last_bits();
    // The word shown: the last once every roll is in, one just made while
    // it glides into Words so far, else the one in progress.
    let arriving = app.keygen_arrival();
    if rolls.len() >= total && arriving.is_none() {
        // Every word is made: the strip clears.
        return (0.0, None);
    }
    let n = if rolls.len() >= total {
        last_n
    } else if let Some((s, _)) = arriving {
        usize::from(s)
    } else {
        rolls.len() / per
    };
    let last = n == last_n;
    let need = if last { last_rolls } else { per };
    let from = n * per;
    let got = rolls.get(from..).unwrap_or(&[]);
    let got = &got[..got.len().min(need)];
    let done = got.len() == need;
    let live = k.live_words();
    let index = if last {
        k.checksum_word().filter(|_| done)
    } else {
        live.get(n).copied().filter(|_| done)
    };
    let sums = k.words / 3;
    let check = index
        .filter(|_| last)
        .map(|i| format!("{:0w$b}", i & ((1 << sums) - 1), w = sums));

    // The number's boxes.
    let mut boxes = [Bit::Empty; 11];
    let mut rolled = Vec::new();
    for (p, &f) in got.iter().enumerate() {
        for c in bits_of(p, f, last).chars() {
            rolled.push((c, p % 2 == 1));
        }
    }
    for (i, b) in boxes.iter_mut().enumerate() {
        let from_check = last && i >= kept;
        *b = match (rolled.get(i), &check) {
            (Some(&(c, odd)), _) if !from_check => Bit::Rolled(c, odd),
            (Some(&(c, _)), None) if i == kept && dropped == 1 => Bit::Dropped(c),
            (_, Some(s)) if from_check => s.chars().nth(i - kept).map_or(Bit::Empty, Bit::Checksum),
            _ => Bit::Empty,
        };
    }

    // Sizes: the slots and bits narrow to fit a small panel.
    let side = ui.touch.max(t::ICON_BUTTON);
    let inner = w - 2.0 * t::PAD;
    let slot = t::DICE_STRIP_SLOT.min((inner - (need as f32 - 1.0) * t::GAP) / need as f32);
    let slot_gap = if slot < t::DICE_STRIP_SLOT {
        t::GAP_SMALL
    } else {
        t::GAP
    };
    let slot = t::DICE_STRIP_SLOT.min((inner - (need as f32 - 1.0) * slot_gap) / need as f32);
    let bit_gap = 2.0;
    let group = t::GAP_SMALL;
    let bw = t::DICE_STRIP_BIT_W.min((inner - 5.0 * group - 10.0 * bit_gap) / 11.0);
    let bh = t::DICE_STRIP_BIT_H * bw / t::DICE_STRIP_BIT_W;
    let caption = ui.line_as(LABEL_S) + t::GAP_SMALL;
    let eq = index.map(|i| fill(EN.keygen_equals_number, &[&i.to_string()]));
    let bits_w = 11.0 * bw + 10.0 * bit_gap + 5.0 * group;
    let eq_below = eq
        .as_ref()
        .is_some_and(|e| bits_w + t::GAP + ui.measure_as(BODY_M.mono(), e) > inner);
    let line = check.is_some().then(|| ui.line_as(BODY_S) + t::GAP_SMALL);
    let height = t::GAP
        + side
        + slot
        + caption
        + bh
        + if eq_below { bh } else { 0.0 }
        + line.unwrap_or(0.0)
        + t::GAP
        + t::TAG_H
        + t::PAD;
    ui.fill(x, y, w, height, t::SHAPE_M, Role::SurfaceContainer);

    // The head: the label, the Guide's button, the word's place, the eye.
    let (x0, mut cy) = (x + t::PAD, y + t::GAP);
    let label = ui.text_mid_as((x0, cy, side), LABEL_L, muted, EN.keygen_word_being_made);
    guide_button(ui, x0 + label + t::GAP_SMALL, cy, side);
    let masked = k.strip_masked();
    let eye_x = x + w - t::GAP - side;
    eye_button(ui, eye_x, cy, side, masked);
    ui.text_right_as(
        (eye_x - t::GAP, cy, side),
        LABEL_L.regular(),
        muted,
        &fill(EN.keygen_word_n, &[&(n + 1).to_string()]),
    );
    cy += side;
    let rest = height - (cy - y);
    if masked {
        let r = t::SHAPE_S;
        ui.fill(x0, cy, inner, rest - t::GAP, r, Role::SurfaceContainerHigh);
        ui.text_mid_as((x0, cy, rest - t::GAP), TITLE_L, muted, "• • • •");
        return (height, arriving.map(|_| (x0, cy)));
    }

    // The slots.
    let again = app.keygen_rerolled().filter(|_| !done && got.len() < need);
    for i in 0..need {
        let sx = x0 + i as f32 * (slot + slot_gap);
        match got.get(i) {
            Some(&f) => {
                let (fill_role, ink) = tint(i % 2 == 1);
                ui.fill(sx, cy, slot, slot, t::SHAPE_M, fill_role);
                let bits = bits_of(i, f, last);
                let drop_last = last && i == need - 1 && dropped == 1;
                face_and_bits(ui, (sx, cy, slot), f, bits, ink, drop_last);
            }
            None => {
                ui.stroke(sx, cy, slot, slot, t::SHAPE_M, Role::OutlineVariant);
                if let (Some((f, drop)), true) = (again, i == got.len())
                    && drop < 1.0
                {
                    let dy = drop * t::DICE_STRIP_DROP;
                    ui.stroke(sx, cy + dy, slot, slot, t::SHAPE_M, Role::Outline);
                    face_and_bits(ui, (sx, cy + dy, slot), f, "", muted, true);
                    let cap = ui.measure_as(LABEL_S, EN.keygen_rolled_again);
                    let capy = cy + dy + slot + t::GAP_SMALL;
                    ui.text_as(
                        sx + (slot - cap) / 2.0,
                        capy,
                        LABEL_S,
                        muted,
                        EN.keygen_rolled_again,
                    );
                }
            }
        }
    }
    cy += slot + caption;

    // The number's 11 bits, then `= n`.
    let mut bx = x0;
    for (i, b) in boxes.iter().enumerate() {
        let r = t::SHAPE_XS;
        let ch = match *b {
            Bit::Empty => {
                ui.stroke(bx, cy, bw, bh, r, Role::OutlineVariant);
                None
            }
            Bit::Rolled(c, odd) => {
                let (f, ink) = tint(odd);
                ui.fill(bx, cy, bw, bh, r, f);
                Some((c, ink, false))
            }
            Bit::Dropped(c) => Some((c, muted, true)),
            Bit::Checksum(c) => {
                ui.fill(bx, cy, bw, bh, r, Role::ChecksumContainer);
                Some((c, Role::OnChecksumContainer, false))
            }
        };
        if let Some((c, ink, strike)) = ch {
            let s = c.to_string();
            let tw = ui.measure_as(BODY_M.mono(), &s);
            ui.text_mid_as((bx + (bw - tw) / 2.0, cy, bh), BODY_M.mono(), ink, &s);
            if strike {
                ui.fill(bx + 2.0, cy + bh / 2.0, bw - 4.0, 2.0, 0.0, ink);
            }
        }
        bx += bw + bit_gap;
        if (i % 2 == 1 && i < 9) || (!last && i == 9) {
            bx += group;
        }
    }
    if let Some(e) = &eq {
        let (ex, ey) = if eq_below {
            (x0, cy + bh)
        } else {
            (x0 + bits_w + t::GAP, cy)
        };
        ui.text_mid_as((ex, ey, bh), BODY_M.mono(), Role::OnSurface, e);
    }
    cy += bh + if eq_below { bh } else { 0.0 };

    // The last word's checksum, named.
    if let Some(h) = line {
        let sw = ui.line_as(BODY_S);
        ui.fill(
            x0,
            cy + t::GAP_SMALL,
            sw,
            sw,
            t::SHAPE_XS,
            Role::ChecksumContainer,
        );
        let text = fill(
            EN.keygen_the_last_bits,
            &[&sums.to_string(), &(n + 1).to_string()],
        );
        let text = ui.fit_as(BODY_S, &text, inner - sw - t::GAP);
        ui.text_as(x0 + sw + t::GAP, cy + t::GAP_SMALL, BODY_S, muted, &text);
        cy += h;
    }

    // The word, and its place in the list.
    cy += t::GAP;
    let mut from = None;
    if let Some(i) = index {
        let word = Language::English.word(i);
        // The word gliding into Words so far is drawn there, from here.
        let pw = if arriving.is_some() {
            from = Some((x0, cy));
            ui.measure_as(BODY_M.mono(), word) + 2.0 * t::TAG_PAD
        } else {
            ui.word_pill(x0, cy, word, Action::KWordOpen(n as u8))
        };
        ui.text_mid_as(
            (x0 + pw + t::GAP, cy, t::TAG_H),
            BODY_S,
            muted,
            &fill(EN.keygen_word_of_list, &[&(u32::from(i) + 1).to_string()]),
        );
    }
    (height, from)
}

/// A slot's face over its bits; the face struck when `struck`, the last
/// of the bits struck as dropped when `drop_last`.
fn face_and_bits(
    ui: &mut Ui,
    (x, y, s): (f32, f32, f32),
    face: u8,
    bits: &str,
    ink: Role,
    struck: bool,
) {
    let f = face.to_string();
    let fw = ui.measure_as(TITLE_L, &f);
    let fh = ui.line_as(TITLE_L);
    let bh = ui.line_as(BODY_S);
    let top = y + (s - fh - bh) / 2.0;
    ui.text_as(x + (s - fw) / 2.0, top, TITLE_L, ink, &f);
    if bits.is_empty() {
        // A struck face with no bits: the line through the face.
        if struck {
            ui.fill(
                x + (s - fw) / 2.0 - 2.0,
                top + fh / 2.0,
                fw + 4.0,
                2.0,
                0.0,
                ink,
            );
        }
        return;
    }
    let (keep, drop) = if struck {
        bits.split_at(bits.len() - 1)
    } else {
        (bits, "")
    };
    let role = BODY_S.mono();
    let (kw, dw) = (ui.measure_as(role, keep), ui.measure_as(role, drop));
    let bx = x + (s - kw - dw) / 2.0;
    ui.text_as(bx, top + fh, role, Role::OnSurfaceVariant, keep);
    if !drop.is_empty() {
        ui.text_as(bx + kw, top + fh, role, Role::OnSurfaceVariant, drop);
        ui.fill(
            bx + kw,
            top + fh + bh / 2.0,
            dw,
            2.0,
            0.0,
            Role::OnSurfaceVariant,
        );
    }
}

/// The info button after the label: Material's standard icon button,
/// round, the icon in on-surface-variant, opening the Guide at this
/// step's section.
fn guide_button(ui: &mut Ui, x: f32, y: f32, side: f32) {
    let a = Action::Guide;
    let edge = (side - t::ICON_BUTTON) / 2.0;
    ui.press(
        Shape::new(x, y, side, side, t::shape_full(side)),
        Role::OnSurface.paint(),
        a,
    );
    ui.icon(
        x + edge,
        y + edge,
        t::ICON_BUTTON,
        Icon::Info,
        t::ICON_SMALL,
        Role::OnSurfaceVariant,
    );
    ui.tip_plain((x, y, side, side), a, EN.tip_guide);
}

/// The eye: struck while the strip is masked, as other eyes show what a
/// press does. Its press is `KShow`.
fn eye_button(ui: &mut Ui, x: f32, y: f32, side: f32, masked: bool) {
    let a = Action::KShow;
    let edge = (side - t::ICON_BUTTON) / 2.0;
    let (bx, by) = (x + edge, y + edge);
    if masked || ui.is_pressed(a) {
        ui.fill(
            bx,
            by,
            t::ICON_BUTTON,
            t::ICON_BUTTON,
            t::shape_full(t::ICON_BUTTON),
            selected_or(masked, Role::SurfaceContainerHighest),
        );
    }
    ui.icon(
        bx,
        by,
        t::ICON_BUTTON,
        if masked { Icon::Eye } else { Icon::EyeOff },
        t::ICON_GLYPH,
        on_selected(masked, Role::OnSurfaceVariant),
    );
    crate::compact::eye_hit(ui, (x, y, side, side), a, !masked);
}
