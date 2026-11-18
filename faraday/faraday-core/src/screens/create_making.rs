//! The making on Make a new wallet's Keys card (F282): a slot's key made
//! from dice where it is chosen, in place of its New key choice. The
//! state is New key's, held in `keygen` with `on_card`; every press is
//! one of its own.

use super::parts::{buttons_and_next_when, pass_field};
use super::pass_pair::{self, pass_pair};
use crate::Action;
use crate::drawing::View;
use crate::keygen::{
    KeyGen, dice_panel,
    screen::{entry_style, no_need_line, rolled_entry, word_grid},
};
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use faraday_draw::tokens::{self, BODY_M, BODY_S, LABEL_L, SEGMENTED_H};
use faraday_text::Text;
use faraday_ui::parts::button_rows;

/// The slot's making: Length, the dice and what is still needed, Make the
/// words; then the words masked with Show words, Make again and Add a
/// passphrase, and the slot's button. Other ways ends the making for
/// New key's own flow. Returns the height used.
pub(super) fn slot_making(
    app: &View,
    ui: &mut Ui,
    k: &KeyGen,
    multi: bool,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let slot = k.slot.unwrap_or(0);
    let mut cy = y;
    // Length: the two counts of BIP-39 words, one chosen.
    let labels = [12u8, 24].map(|n| Text::fill(EN.keygen_words_n, &[&Text::num(u64::from(n))]));
    let options = [
        (&*labels[0], Action::KWords(12)),
        (&*labels[1], Action::KWords(24)),
    ];
    let chosen = k.length_chosen.then_some(usize::from(k.words != 12));
    let off = (tokens::TOUCH - SEGMENTED_H) / 2.0;
    ui.segmented(
        (x, cy + off, SEGMENTED_H),
        LABEL_L.size,
        &options,
        chosen,
        None,
    );
    cy += tokens::TOUCH + tokens::GAP_SMALL;
    let other = (
        EN.create_other_ways,
        Style::Ghost,
        Action::KeyOtherWays(slot),
    );
    if let Some(m) = k.mnemonic.as_ref() {
        cy += word_grid(ui, m, k.shown, x, cy, w);
        let show = if k.shown {
            EN.keygen_hide_words
        } else {
            EN.keygen_show_words
        };
        let mut row = vec![
            (show, Style::Secondary, Action::KShow),
            (EN.create_make_again, Style::Ghost, Action::KAgain),
        ];
        if !k.pass_open {
            row.push((
                EN.create_add_a_passphrase,
                Style::Secondary,
                Action::KPassField(0),
            ));
        }
        cy += button_rows(ui, x, cy, w, &row) + tokens::GAP_SMALL;
        if k.pass_open {
            cy += pass_pair(
                ui,
                (x, cy, w),
                (k.passphrase.as_str(), k.passphrase2.as_str()),
                |ui, n, fy| {
                    let (label, text) = if n == 0 {
                        (EN.create_passphrase, &k.passphrase)
                    } else {
                        (EN.create_passphrase_again, &k.passphrase2)
                    };
                    pass_field(
                        ui,
                        x,
                        fy,
                        w,
                        label,
                        text,
                        (
                            k.focus == Some(n),
                            app.pass_chip(crate::demo::PassField::KeyGen(n)),
                            app.typing.pass_eye.shown,
                        ),
                        Action::KPassField(n),
                    )
                },
            );
        }
        if let Some(n) = &k.note {
            cy += ui.wrap_as((x, cy, w), BODY_M, Role::Error, n) + tokens::GAP_SMALL;
        }
        let agree = !k.pass_open
            || (!pass_pair::differ(k.passphrase.as_str(), k.passphrase2.as_str())
                && !app.typing.pass_eye.waits());
        let label = if multi {
            EN.create_lock_button
        } else {
            EN.create_make_the_wallet_button
        };
        cy += buttons_and_next_when(
            ui,
            x,
            cy,
            w,
            std::slice::from_ref(&other),
            (Some((label, Action::KLock)), agree),
        );
    } else if k.length_chosen {
        cy += dice_panel::progress(k, ui, x, cy, w);
        // The same entry New key's Entries card shows: Typed or Buttons,
        // then the words as the rolls name them.
        let touch = app.touch_side();
        cy = entry_style(k, ui, x, cy, w, touch);
        cy += rolled_entry(app, k, ui, (x, cy, w), Some(Some(touch)));
        cy += no_need_line(k, ui, (x, cy, w));
        if let Some(n) = &k.note {
            cy += ui.wrap_as((x, cy, w), BODY_S, Role::Error, n) + tokens::GAP_SMALL;
        }
        let mut left = vec![
            (EN.keygen_undo, Style::Ghost, Action::KUndo),
            (EN.keygen_clear, Style::Ghost, Action::KClear),
        ];
        left.push(other);
        cy += buttons_and_next_when(
            ui,
            x,
            cy,
            w,
            &left,
            (Some((EN.keygen_make_the_words, Action::KNext)), k.ready()),
        );
    } else {
        // Until a count is picked only the choice and Other ways show.
        cy += buttons_and_next_when(ui, x, cy, w, std::slice::from_ref(&other), (None, false));
    }
    cy + tokens::GAP_SMALL - y
}
