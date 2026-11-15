//! Faraday's English words for the vanity area.

macro_rules! words_vanity {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- vanity (faraday-core/src/vanity/mod.rs) -----
    /// Text in `script_name`.
    vanity_legacy: "Legacy",
    /// Text in `script_name`.
    vanity_nested_segwit: "Nested SegWit",
    /// Text in `script_name`.
    vanity_native_segwit: "Native SegWit",
    /// Text in `script_name`.
    vanity_taproot: "Taproot",
    /// Text in `script_name`.
    vanity_silent_payment_code: "Silent payment code",
    /// Text in `script_name`. `{}`: `name`.
    vanity_m_or_n: "{} · m… or n…",
    /// Text in `dial_name`.
    vanity_account_number: "Account number",
    /// Text in `dial_name`.
    vanity_the_same_words: "The same words and passphrase, a different account: fast. The wallet is the key's own, at the account found.",
    /// Text in `dial_name`.
    vanity_passphrase: "Passphrase",
    /// Text in `dial_name`.
    vanity_a_few_characters: "A few characters added to the key's BIP-39 passphrase: slow, one PBKDF2 a try. The words with that passphrase are another wallet, which needs the passphrase kept with them.",
    /// Toast in `vanity_open`.
    vanity_load_a_key_first: "Load a key first",
    /// Error in `vanity_act`.
    vanity_type_at_least: "Type at least one character after the fixed part",
    /// Text in `vanity_act`.
    vanity_this_key_was: "This key was not loaded from BIP-39 words: only its account turns",
    /// Error in `vanity_tick`.
    vanity_every_counter: "Every counter was tried: no address begins that way",
    /// Text in `vanity_use`. `{}`: `label`.
    vanity_vanity_passphrase: "{} · vanity passphrase",
    /// Text in `vanity_use`.
    vanity_vanity_search: "Vanity search",
    /// Toast in `vanity_use`. `{}`: `name`.
    vanity_is_open: "{} is open",
    /// Error in `vanity_key_in`. `{}`: `c`.
    vanity_cannot_stand: "{} cannot stand there in this kind of address",
    // ----- vanity (faraday-core/src/vanity/screen.rs) -----
    /// Text in `TITLES`.
    vanity_key: "Key",
    /// Text in `TITLES`.
    vanity_what_turns: "What turns",
    /// Text in `TITLES`.
    vanity_kind_of_address: "Kind of address",
    /// Text in `TITLES`.
    vanity_characters: "Characters",
    /// Text in `TITLES`.
    vanity_search: "Search",
    /// Text in `draw`.
    vanity_wallets: "Wallets",
    /// Text in `draw`.
    vanity_vanity_address: "Vanity address",
    /// Text in `summary`.
    vanity_not_chosen: "Not chosen",
    /// Text in `summary`. `{}`: `thousands(v.grind.tested)`.
    vanity_tried_count: "{} tried",
    /// Text in `card`. `{}`: `thousands(expected)`.
    vanity_about_tries: "About {} tries",
    /// Button in `card`.
    vanity_continue: "Continue",
    /// Line in `card`.
    vanity_found: "Found",
    /// Text in `card`. `{}`: `f.account`, `thousands(v.grind.tested)`.
    vanity_found_account: "Account {} of this key, after {} tries",
    /// Line in `card`.
    vanity_added_to_passphrase: "Added to the passphrase:",
    /// Text in `card`.
    vanity_hide: "Hide",
    /// Text in `card`.
    vanity_show: "Show",
    /// Line in `card`.
    vanity_these_characters: "These characters are part of the passphrase: write them down with it. The words with any other passphrase open another wallet.",
    /// Button in `card`.
    vanity_open_this_wallet: "Open this wallet",
    /// Button in `card`.
    vanity_open_in_silent_payments: "Open in Silent payments",
    /// Button in `card`.
    vanity_search_again: "Search again",
    /// Text in `card`. `{}`: `thousands(r as u64)`.
    vanity_a_second: "{} a second",
    /// Text in `card`, after a space. `{}`: `seconds_text(s)`.
    vanity_about_expected: "· about {} expected",
    /// Text in `card`.
    vanity_measuring: "Measuring",
    /// Text in `card`. `{}`: `v.grind.prefix.as_str()`.
    vanity_searching_prefix: "Searching for {}",
    /// Text in `card`. `{}`: `thousands(v.grind.tested)`, `rate`, `left`.
    vanity_tried: "{} tried · {}{}",
    /// Button in `card`.
    vanity_stop: "Stop",
    /// Text in `card`. `{}`: `v.grind.prefix.as_str()`.
    vanity_search_prefix: "Search for {}",
    /// Text in `seconds_text`. `{}`: `s / 60`.
    vanity_min: "{} min",
    /// Text in `seconds_text`. `{}`: `s / 86_400`.
    vanity_days: "{} days",
        }
    };
}
