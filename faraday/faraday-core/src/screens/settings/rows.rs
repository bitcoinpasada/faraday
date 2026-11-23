//! The words drawn on every card of Settings, for the search
//! (`tools/search.rs`): each card's title, labels, options' names and values'
//! words, taken from the same strings the card draws, worked out once. A
//! label or option added to a card is added to its list here.

use super::{Part, part_title, parts};
use crate::modes::Mode as Network;
use crate::stick::settings_choice::{MotionSpeed, MotionStyle};
use crate::strings::EN;
use faraday_ui::{Contrast, Mode, TextSize, ThemeName};
use std::sync::OnceLock;

/// The strings a card draws, the card's title first.
fn drawn(p: Part) -> Vec<&'static str> {
    let mut words = vec![part_title(p)];
    match p {
        Part::Mode => {
            let all = [
                Network::Demo,
                Network::Mainnet,
                Network::Testnet,
                Network::Signet,
                Network::Regtest,
            ];
            words.extend(all.map(Network::name));
        }
        Part::Session => words.extend([
            EN.settings_keys,
            EN.settings_lock,
            EN.settings_remove_every,
            EN.settings_quit,
            EN.settings_power_off_button,
            EN.app_panic_key_hint,
            EN.settings_lock_after,
            EN.settings_power_off,
            EN.settings_min,
            EN.settings_never,
        ]),
        Part::Time => words.extend([
            EN.settings_now,
            EN.tz_row_time_zone,
            EN.settings_set_the_time,
        ]),
        Part::Randomness => words.push(EN.own_add_randomness),
        Part::Appearance => {
            words.push(EN.ui_theme_row);
            words.extend(ThemeName::ALL.map(ThemeName::name));
            words.push(EN.ui_mode_row);
            words.extend(Mode::ALL.map(Mode::name));
            words.push(EN.ui_contrast_row);
            words.extend(Contrast::ALL.map(Contrast::name));
            words.push(EN.settings_text_size);
            words.extend(TextSize::ALL.map(TextSize::name));
            words.extend([EN.settings_motion, EN.settings_speed]);
            words.extend(MotionStyle::ALL.map(MotionStyle::name));
            words.extend(MotionSpeed::ALL.map(MotionSpeed::name));
        }
        Part::Display => words.extend([
            EN.settings_rotation,
            EN.settings_landscape,
            EN.settings_portrait,
            EN.settings_scale,
            EN.settings_auto,
        ]),
        Part::Remembered => words.extend([
            EN.settings_amounts_signed,
            EN.settings_transactions,
            EN.settings_in_open_vault,
            EN.settings_until_power_off,
            EN.settings_signatures_made,
            EN.settings_signatures,
            EN.settings_nonces_off,
        ]),
        Part::About => words.extend([
            EN.settings_faraday,
            EN.settings_keys_are_typed,
            EN.settings_own_risk,
            EN.settings_vaults_files,
            EN.settings_are_folders,
            EN.settings_medium_reader_note,
            EN.settings_selftest,
            EN.settings_not_run,
        ]),
    }
    words
}

/// Every card's words, one text per section in [`parts`]' order, worked out
/// on first use.
fn cards() -> &'static [String] {
    static CARDS: OnceLock<Vec<String>> = OnceLock::new();
    CARDS.get_or_init(|| parts().into_iter().map(|p| drawn(p).join(" ")).collect())
}

/// The sections whose card holds every word of `find`, as indexes into
/// [`parts`].
pub(crate) fn matching(find: &str) -> Vec<u8> {
    cards()
        .iter()
        .enumerate()
        .filter(|(_, text)| crate::catalog::words_in(text, find))
        .filter_map(|(i, _)| u8::try_from(i).ok())
        .collect()
}
