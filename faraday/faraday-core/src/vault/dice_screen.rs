//! Generate with dice on Create a vault (`docs/FLOWS.md`, Create a vault):
//! drawing only. What a press does is `vault/dice.rs`.

use super::screen::text_box;
use crate::Action;
use crate::drawing::View;
use crate::screens::{dice_pad, never_under};
use crate::strings::EN;
use crate::strings::fill;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::vault::{self, Focus, TextBox, VaultAction as V};
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H, FIELD_H, LABEL_L, LABEL_S};
use faraday_draw::widgets::Icon;

fn va(a: V) -> Action {
    Action::Vault(a)
}

/// The input panel's height where it stands at the window's foot while
/// dice are rolled for a passphrase (`docs/LAYOUT.md` §16.3): the rolls,
/// the strength line in up to two lines and the pad, inside its padding,
/// on a window that is not the small panel and where it takes at most
/// `INPUT_PANEL_MAX` of the height, while the on-screen keyboard is not
/// up; else `None`, and all three stay in the card. Arithmetic.
pub(super) fn foot(app: &View, ui: &Ui) -> Option<f32> {
    use faraday_draw::tokens as t;
    let open = app
        .vaults
        .create
        .as_ref()
        .is_some_and(|c| c.open == Some(vault::vstep::PHRASES));
    let line = ui.line_as(BODY_S);
    let fh = t::PAD + ui.field_h() + t::GAP + 2.0 * line + t::GAP + t::DICE_PAD_KEY_H + t::PAD;
    let fits = crate::codex32::input_panel::foot_fits(app.window(), fh, app.size().1);
    // With the on-screen keyboard up, its keys are the pad, at the foot.
    let keyboard = app.osk_height() > 0.0;
    (open && app.vaults.dice.is_some() && fits && !keyboard).then_some(fh)
}

/// The input panel at the foot of the flow's page, `fh` tall, where
/// [`foot`] stands it: the rolls, the strength line and the pad, a key
/// per face and Backspace, in the column `col` lays the cards in.
pub(super) fn foot_panel(
    app: &View,
    ui: &mut Ui,
    (col, h): (&crate::flow::Column, f32),
    fh: Option<f32>,
) {
    use faraday_draw::tokens as t;
    let (Some(fh), Some((_, rolls, list))) = (fh, app.vaults.dice.as_ref()) else {
        return;
    };
    let top = h - fh;
    crate::codex32::input_panel::cover(ui, (col.area_x, top, col.area_w, fh));
    // At the card body's start on the wide layout; at the one-column
    // page's margins elsewhere, as its cards stand.
    let (x, w) = if ui.compact {
        let m = crate::compact::M;
        (col.area_x + m, (col.area_w - 2.0 * m).min(t::COLUMN_MAX))
    } else {
        let i = crate::flow::BODY_INDENT;
        (col.x + i, (col.w - 2.0 * i).min(t::COLUMN_MAX))
    };
    let mut cy = top + t::PAD;
    rolls_box(app, ui, (x, cy, w), rolls);
    cy += ui.field_h() + t::GAP;
    let (status, tone) = strength(app, &rolls.text, *list);
    ui.wrap_as((x, cy, w), BODY_S, tone, &status);
    cy += 2.0 * ui.line_as(BODY_S) + t::GAP;
    let key = t::DICE_PAD_KEY_H;
    let pad_w = w - key - t::GAP;
    dice_pad(ui, (x, cy, pad_w), &|_| Style::Secondary, |f| {
        va(V::DiceKey(f))
    });
    let bx = x + w - key;
    ui.button(
        bx,
        cy,
        Some(key),
        key,
        "",
        Style::Secondary,
        va(V::DiceKey(0)),
    );
    let ic = t::ICON_SMALL;
    let at = (bx + (key - ic) / 2.0, cy + (key - ic) / 2.0);
    ui.icon(
        at.0,
        at.1,
        ic,
        Icon::Delete,
        t::CODEX32_TAB_GLYPH,
        Role::OnSurface,
    );
}

/// The rolls typed, in their field, at (x, y), `w` wide.
fn rolls_box(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32), rolls: &TextBox) {
    let focused = app.vaults.focus == Some(Focus::Dice);
    let press = Action::Vault(V::Dice(app.vaults.dice.as_ref().map_or(0, |d| d.0)));
    text_box(ui, x, y, w, rolls, false, focused, press);
}

/// Rolling dice for passphrase `i`, where its dice are open: the rolls
/// typed, the words they make, and the strength so far; the rolls and
/// the strength stand in the input panel at the foot where [`foot`]
/// stands it. Returns its height.
pub(super) fn dice_panel(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32), i: usize) -> f32 {
    match app.vaults.dice.as_ref() {
        Some((di, rolls, list)) if *di == i => {
            let at_foot = foot(app, ui).is_some();
            dice_card(app, ui, (x, y, w), (rolls, *list), at_foot)
        }
        _ => 0.0,
    }
}

/// The strength line for `rolls` from `list`, and its tone.
fn strength(app: &View, rolls: &str, list: faraday_bip::diceware::List) -> (String, Role) {
    let words = vault::dice_words(rolls, list);
    let per = list.dice_per_word();
    let left = rolls.len() % per;
    let bits = list.bits(words.len());
    let (_, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
    let own = words.len() as f32 * list.bits_per_word();
    let status = fill(
        EN.vault_dice_strength,
        &[
            &words.len().to_string(),
            (if words.len() == 1 {
                EN.vault_word
            } else {
                EN.vault_words_inline
            }),
            &format!("{own:.1}"),
            &format!("{:.1}", own + vault::cost_bits(mem, passes)),
            &(if left > 0 {
                format!(
                    " {}",
                    fill(
                        EN.vault_dice_for_the_next_word,
                        &[&left.to_string(), &per.to_string()]
                    )
                )
            } else {
                String::new()
            })
            .to_string(),
        ],
    );
    let tone = if bits >= vault::STRONG_BITS {
        Role::Ok
    } else {
        Role::OnSurfaceVariant
    };
    (status, tone)
}

/// [`dice_panel`]'s card: the rolls and the strength line left out where
/// the input panel at the foot holds them (`at_foot`).
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn dice_card(
    app: &View,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    (rolls, list): (&TextBox, faraday_bip::diceware::List),
    at_foot: bool,
) -> f32 {
    use faraday_bip::diceware::List;
    let mut cy = y;
    ui.fill(x, cy, w, 4.0, 2.0, Role::Primary);
    cy += 14.0;
    ui.text_as(x, cy, LABEL_L, Role::OnSurface, EN.dice_list_title);
    cy += ui.advance_as(LABEL_L, 22.0);
    let lists = [
        (List::Large, EN.dice_list_large, EN.dice_list_large_detail),
        (
            List::Short1,
            EN.dice_list_short1,
            EN.dice_list_short1_detail,
        ),
        (
            List::Short2,
            EN.dice_list_short2,
            EN.dice_list_short2_detail,
        ),
    ];
    // Side by side; on a small panel one under another.
    let compact = ui.compact;
    let lw = if compact { w } else { (w - 2.0 * 10.0) / 3.0 };
    for (k, (l, name, detail)) in lists.iter().enumerate() {
        let on = *l == list;
        let (bx, cy) = if compact {
            (x, cy + k as f32 * 52.0)
        } else {
            (x + k as f32 * (lw + 10.0), cy)
        };
        let detail = ui.fit_as(LABEL_S.regular(), detail, lw - 20.0);
        use crate::ui::pal::{on_selected, selected_or};
        let fill_ = selected_or(on, Role::SurfaceContainerHighest);
        ui.fill(bx, cy, lw, 44.0, 8.0, fill_);
        ui.stroke(bx, cy, lw, 44.0, 8.0, selected_or(on, Role::OutlineVariant));
        let ink = on_selected(on, Role::OnSurface);
        ui.text_as(bx + 10.0, cy + 6.0, LABEL_L, ink, name);
        let muted = on_selected(on, Role::OnSurfaceVariant);
        ui.text_as(bx + 10.0, cy + 24.0, LABEL_S.regular(), muted, &detail);
        ui.press_box(bx, cy, lw, 44.0, Action::Vault(V::DiceList(k as u8)));
    }
    cy += if compact { 3.0 * 52.0 + 4.0 } else { 56.0 };
    cy += ui.wrap_as(
        (x, cy, w),
        LABEL_L,
        Role::OnSurface,
        &fill(EN.vault_dice_per_word, &[&list.dice_per_word().to_string()]),
    ) + 8.0;
    if !at_foot {
        rolls_box(app, ui, (x, cy, w), rolls);
        cy += FIELD_H + 10.0 + never_under(app, ui, x, cy, w);
    }
    // The computer's roller, on Faraday Demo only, is in the strip
    // (`docs/CODEX32.md` §5.6, `host`).
    let words = vault::dice_words(&rolls.text, list);
    // The words as the rolls make them, each a pill that opens it in its
    // list, as New key shows its words.
    if words.is_empty() {
        ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, EN.vault_no_word_yet);
        cy += ui.advance_as(BODY_S, 26.0);
    } else {
        ui.text_as(
            x,
            cy,
            BODY_S,
            Role::OnSurfaceVariant,
            crate::wordlist::PRESS_A_WORD,
        );
        cy += ui.advance_as(BODY_S, 22.0);
        let place = crate::wordlist::WordList::Eff(list).place();
        let mut bx = x;
        for (word, i) in words
            .iter()
            .zip(vault::dice_word_indices(&rolls.text, list))
        {
            let bw = ui.measure_as(BODY_M.mono(), word) + 24.0;
            if bx > x && bx + bw > x + w {
                bx = x;
                cy += 32.0;
            }
            bx += ui.word_pill(
                bx,
                cy,
                word,
                Action::WordList(crate::wordlist::WordListAction::Open(place, Some(i))),
            ) + 6.0;
        }
        cy += 38.0;
    }
    if !at_foot {
        let (status, tone) = strength(app, &rolls.text, list);
        cy += ui.wrap_as((x, cy, w), BODY_S, tone, &status) + 8.0;
    }
    // How long is long enough, and where the lists come from.
    cy += ui.wrap_as(
        (x, cy, w),
        BODY_S,
        Role::OnSurfaceVariant,
        &vault::dice_aim(list),
    ) + 12.0;
    let style = Style::Primary.when(!words.is_empty());
    let uw = ui.button(
        x,
        cy,
        None,
        BUTTON_H,
        EN.vault_use_these_words,
        style,
        va(V::DiceUse),
    );
    ui.button(
        x + uw + 8.0,
        cy,
        None,
        BUTTON_H,
        EN.vault_cancel,
        Style::Ghost,
        va(V::DiceClose),
    );
    cy += 50.0;
    cy - y
}
