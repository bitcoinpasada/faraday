//! Faraday's English words for the screens entry area.

macro_rules! words_screens_entry {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- entry (faraday-core/src/screens/entry.rs) -----
    /// Text in `entry`.
    entry_spend: "Spend",
    /// Text in `entry`.
    entry_wallets: "Wallets",
    /// Text in `entry`. `{}`: `fp_text(faraday_bip::keys::Fingerprint(fp))`.
    entry_add_key: "Add key {}",
    /// Text in `entry`. `{}`: `name`.
    entry_add_key_title: "Add a key for {}",
    /// Text in `entry`. `{}`: `app.entry.form.name()`.
    entry_add_a_key_n: "Add a key · {}",
    /// Text in `entry`.
    entry_add_a_key: "Add a key",
    /// Text in `entry`.
    entry_other_languages: "Other languages",
    /// Text in `entry`.
    entry_other_forms: "Other forms",
    /// Button in `entry`.
    entry_scan_a_seedqr: "Scan a SeedQR",
    /// Text in `entry`.
    entry_no_word_starts: "No word starts with that",
    /// Text in `entry`, after the candidates and three spaces.
    entry_tab_completes: "· Tab completes a single match",
    // ----- word_count_row (faraday-core/src/screens/entry.rs) -----
    /// Line in `word_count_row`.
    word_count_row_words: "Words",
    // ----- keyed_words (faraday-core/src/screens/entry.rs) -----
    /// Text in `keyed_words`.
    keyed_words_type_next: "Type the next word",
    /// Text in `keyed_words`.
    keyed_words_tone_prompt: "Now its tone",
    /// Text in `keyed_words`.
    keyed_words_no_word: "No word starts with that",
    /// Line in `keyed_words`.
    keyed_words_press_hint: "Press a word to take it",
    // ----- keyed_status (faraday-core/src/screens/entry.rs) -----
    /// Text in `keyed_status`. `{}`: `n`.
    keyed_status_word_count: "{} words",
    /// Text in `keyed_status`. `{}`: `n`, `fp_text(fp)`.
    keyed_status_words: "{} words · checksum correct · {}",
    // ----- entry_status (faraday-core/src/screens/entry.rs) -----
    /// Text in `entry_status`. `{}`: `n`, `fp_text(fp)`.
    entry_status_words_checksum: "{} words · checksum correct · {}",
    /// Text in `entry_status`. `{}`: `n`.
    entry_status_words: "{} words",
    // ----- entry_finish (faraday-core/src/screens/entry.rs) -----
    /// Line in `entry_finish`.
    entry_finish_bip39_passphrase: "BIP-39 passphrase",
    /// Button in `entry_finish`.
    entry_finish_add_key: "Add key",
    /// Button in `entry_finish`.
    entry_finish_clear: "Clear",
    /// Button in `entry_finish`.
    entry_finish_new_key: "Make a new key",
    /// Line in `entry_finish`.
    entry_finish_keys_hint: "Type on the keyboard · Enter adds · Esc goes back",
    // ----- entry_stick (faraday-core/src/screens/entry.rs) -----
    /// Text in `entry_stick`. `{}`: `app.medium.a()`.
    entry_stick_from_plug: "From {} · plug it in",
    /// Button in `entry_stick`.
    entry_stick_copy_files: "Copy files in",
    // ----- entry_form (faraday-core/src/screens/entry.rs) -----
    /// Text in `entry_form`.
    entry_form_one_share_words: "Words of one share",
    /// Text in `entry_form`.
    entry_form_nothing: "Nothing in yet",
    /// Text in `entry_form`.
    entry_form_count_one: "1 in",
    /// Text in `entry_form`. `{}`: `n`.
    entry_form_count_many: "{} in",
    /// Line in `entry_form`.
    entry_form_passphrase: "Passphrase",
    /// Button in `entry_form`.
    entry_form_add_share: "Add this one",
    /// Button in `entry_form`.
    entry_form_scan_a_qr: "Scan a QR code",
    /// Button in `entry_form`.
    entry_form_make_key: "Make the key",
    /// Line in `entry_form`.
    entry_form_enter_adds: "Enter adds what is typed · Enter on an empty line makes the key",
        }
    };
}
