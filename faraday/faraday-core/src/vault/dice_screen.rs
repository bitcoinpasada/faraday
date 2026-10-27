//! Generate with dice on Create a vault (`docs/FLOWS.md`, Create a vault):
//! drawing only. What a press does is `vault/dice.rs`.

use super::screen::text_box;
use crate::Action;
use crate::drawing::View;
use crate::screens::never_under;
use crate::strings::EN;
use crate::strings::fill;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::vault::{self, Focus, TextBox, VaultAction as V};
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H, FIELD_H, LABEL_L, LABEL_S};

fn va(a: V) -> Action {
    Action::Vault(a)
}

/// Rolling dice for a passphrase: the rolls typed, the words they make,
/// and the strength so far. Returns its height.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn dice_panel(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    rolls: &TextBox,
    list: faraday_bip::diceware::List,
) -> f32 {
    use faraday_bip::diceware::List;
    let mut cy = y;
    ui.fill(x, cy, w, 4.0, 2.0, Role::Primary.with_alpha(60));
    cy += 14.0;
    ui.text_as(x, cy, LABEL_L, Role::OnSurface, EN.dice_list_title);
    cy += 22.0;
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
        ui.hit(bx, cy, lw, 44.0, Action::Vault(V::DiceList(k as u8)));
    }
    cy += if compact { 3.0 * 52.0 + 4.0 } else { 56.0 };
    cy += ui.wrap_as(
        (x, cy, w),
        LABEL_L,
        Role::OnSurface,
        &fill(EN.vault_dice_per_word, &[&list.dice_per_word().to_string()]),
    ) + 8.0;
    let focused = app.vaults.focus == Some(Focus::Dice);
    text_box(
        ui,
        x,
        cy,
        w,
        rolls,
        false,
        focused,
        Action::Vault(V::Dice(app.vaults.dice.as_ref().map_or(0, |d| d.0))),
    );
    cy += FIELD_H + 10.0 + never_under(app, ui, x, cy, w);
    // The computer's roller, on Faraday Demo only, is in the "Practice
    // only" card (`docs/CODEX32.md` §5.6, `samples/card.rs`).
    let words = vault::dice_words(&rolls.text, list);
    let per = list.dice_per_word();
    let left = rolls.text.len() % per;
    // The words as the rolls make them, each a pill that opens it in its
    // list, as New key shows its words.
    if words.is_empty() {
        ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, EN.vault_no_word_yet);
        cy += 26.0;
    } else {
        ui.text_as(
            x,
            cy,
            BODY_S,
            Role::OnSurfaceVariant,
            crate::wordlist::PRESS_A_WORD,
        );
        cy += 22.0;
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
    cy += ui.wrap_as(
        (x, cy, w),
        BODY_S,
        if bits >= vault::STRONG_BITS {
            Role::Ok
        } else {
            Role::OnSurfaceVariant
        },
        &status,
    ) + 8.0;
    // How long is long enough, and where the lists come from.
    cy += ui.wrap_as(
        (x, cy, w),
        BODY_S,
        Role::OnSurfaceVariant,
        &vault::dice_aim(list),
    ) + 12.0;
    let style = if words.is_empty() {
        Style::Disabled
    } else {
        Style::Primary
    };
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
