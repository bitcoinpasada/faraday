//! The words drawn on every card of Settings, for the search (`main`'s `settings/rows.rs`): each
//! card's title, labels, options' names and values' words, taken from the same strings the card
//! draws. A label or option added to a card is added to its list here.

use faraday_strings::Strings;

/// Settings' cards, in the order the page draws them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Part {
    Mode,
    Appearance,
    Session,
    Time,
    Display,
    Remembered,
    About,
}

pub(crate) const PARTS: [Part; 7] = [
    Part::Mode,
    Part::Appearance,
    Part::Session,
    Part::Time,
    Part::Display,
    Part::Remembered,
    Part::About,
];

/// A card's name, as the card shows it.
pub(crate) fn title(p: Part, s: &Strings) -> &'static str {
    match p {
        Part::Mode => s.settings_mode,
        Part::Appearance => s.settings_appearance,
        Part::Session => s.settings_lock_and_power,
        Part::Time => s.settings_time,
        Part::Display => s.settings_display,
        Part::Remembered => s.settings_what_remembered,
        Part::About => s.settings_about,
    }
}

/// The strings a card draws, the card's title first.
fn drawn(p: Part, s: &Strings) -> Vec<&'static str> {
    let mut words = vec![title(p, s)];
    match p {
        Part::Mode => words.extend([
            s.mode_demo,
            s.wallet_mainnet,
            s.wallet_testnet,
            s.wallet_signet,
            s.wallet_regtest,
        ]),
        Part::Appearance => words.extend([
            s.ui_theme_row,
            s.ui_cobalt,
            s.ui_bitcoin_orange,
            s.ui_lamplight,
            s.ui_mars,
            s.ui_forest,
            s.ui_frost,
            s.ui_dusk,
            s.ui_lavender,
            s.ui_rose,
            s.ui_monochrome,
            s.ui_mode_row,
            s.ui_light,
            s.ui_dark,
            s.ui_contrast_row,
            s.ui_contrast_standard,
            s.ui_contrast_medium,
            s.ui_contrast_high,
            s.settings_text_size,
            s.settings_text_standard,
            s.settings_text_large,
            s.settings_text_larger,
            s.settings_motion,
            s.settings_speed,
            s.settings_motion_smooth,
            s.settings_motion_crossfade,
            s.settings_motion_off,
            s.settings_speed_quick,
            s.settings_speed_medium,
            s.settings_speed_relaxed,
        ]),
        Part::Session => words.extend([
            s.settings_keys,
            s.settings_lock,
            s.settings_remove_every,
            s.settings_quit,
            s.settings_power_off_button,
            s.app_panic_key_hint,
            s.settings_lock_after,
            s.settings_power_off,
            s.settings_min,
            s.settings_never,
        ]),
        Part::Time => words.extend([s.settings_now, s.tz_row_time_zone, s.settings_set_the_time]),
        Part::Display => words.extend([
            s.settings_rotation,
            s.settings_landscape,
            s.settings_portrait,
            s.settings_scale,
            s.settings_auto,
        ]),
        Part::Remembered => words.extend([
            s.settings_amounts_signed,
            s.settings_transactions,
            s.settings_in_open_vault,
            s.settings_until_power_off,
            s.settings_signatures_made,
            s.settings_signatures,
            s.settings_nonces_off,
        ]),
        Part::About => words.extend([
            s.settings_faraday,
            s.settings_keys_are_typed,
            s.settings_own_risk,
            s.settings_selftest,
            s.settings_not_run,
        ]),
    }
    words
}

/// Letters, digits and spaces, lower-cased: "bip85" and "bip-85" search alike.
fn fold(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Whether every word of `find` is among `text`'s, case and punctuation not counting.
fn words_in(text: &str, find: &str) -> bool {
    let hay = fold(text);
    find.split_whitespace()
        .all(|word| hay.contains(&fold(word)))
}

/// The cards whose words hold every word of `find`.
pub(crate) fn matching(find: &str, s: &Strings) -> Vec<Part> {
    PARTS
        .into_iter()
        .filter(|&p| words_in(&drawn(p, s).join(" "), find))
        .collect()
}

#[cfg(test)]
mod tests {
    use faraday_strings::EN;

    use super::*;

    /// Invariant: A card matches a search only when every word of it is among the card's drawn words, case and punctuation not counting.
    /// Regression: A search matches a card holding only one of its words, or a word typed in capitals or with a dash finds nothing.
    /// Owner: app
    #[test]
    fn a_card_matches_only_when_every_word_of_the_search_is_on_it() {
        assert_eq!(matching("", &EN), PARTS.to_vec());
        assert_eq!(matching("SCALE", &EN), vec![Part::Display]);
        assert_eq!(matching("scale rotation", &EN), vec![Part::Display]);
        assert_eq!(matching("scale mars", &EN), vec![]);
        assert_eq!(matching("self-test", &EN), vec![Part::About]);
        assert_eq!(matching("zzz", &EN), vec![]);
    }
}
