//! Faraday's English words for the forms area.

macro_rules! words_forms {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- forms (faraday-core/src/forms.rs) -----
    /// Text in `name`.
    forms_bip_39_words: "BIP-39 words",
    /// Text in `name`.
    forms_slip_39_shares: "SLIP-39 shares",
    /// Text in `name`.
    forms_codex32: "codex32",
    /// Text in `name`.
    forms_krux_code: "Krux code",
    /// Text in `name`.
    forms_seed_xor_parts: "Seed XOR parts",
    /// Text in `lines`. `{}`: `s.group_index() + 1`, `s.group_count()`, `s.member_index() + 1`, `s.member_threshold()`, `s.identifier()`.
    forms_group_of_member: "Group {} of {}, member {} · {} of the group needed · id {}",
    /// Text in `lines`. `{}`: `id`.
    forms_the_secret_id: "The secret · id {}",
    /// Text in `lines`. `{}`: `char::from(c.share_index())`, `id`, `c.threshold()`.
    forms_share_id_needed: "Share {} · id {} · {} needed",
    /// Text in `lines`. `{}`: `i`, `words`.
    forms_part_words: "Part {} · {} words",
    /// Text in `form_add_part`. `{}`: `e`.
    forms_not_slip39_share: "Not a SLIP-39 share: {}",
    /// Text in `form_add_part`. `{}`: `e`.
    forms_not_a_codex32: "Not a codex32 string: {}",
    /// Error in `form_add_part`.
    forms_other_seed_string: "That string is of another seed: its id differs",
    /// Error in `form_add_part`.
    forms_share_duplicate: "That share is in already",
    /// A SLIP-39 share past the most a set can have (16 groups of 16).
    forms_shares_full: "256 shares are in: a SLIP-39 set has no more",
    /// A passphrase field's line for a letter it refuses: a BIP-39 or
    /// aezeed passphrase here is printable ASCII (docs/STANDARDS.md). It
    /// names no letter: the field is masked.
    passphrase_ascii_only: "A passphrase here is printable ASCII only",
    /// Text in `form_add_part`. `{}`: `e`.
    forms_not_bip_39_words: "Not BIP-39 words: {}",
    /// Error in `form_add_part`.
    forms_every_part_has: "Every part has the same number of words",
    /// Text in `form_recover`.
    forms_slip_39_key: "SLIP-39 key",
    /// Text in `form_recover`.
    forms_codex32_key: "codex32 key",
    /// Text in `form_recover`.
    forms_seed_xor_key: "Seed XOR key",
    /// Text in `form_recover`. `{}`: `e`.
    forms_the_shares_do: "The shares do not make a key: {}",
    /// Text in `form_recover`. `{}`: `e`.
    forms_the_strings_do: "The strings do not make a key: {}",
    /// Error in `form_recover`.
    forms_seed_xor_needs: "Seed XOR needs two parts or more",
    /// Toast in `form_recover`. `{}`: `fp_text(fp)`.
    forms_key_added: "Key {} added",
    /// Text in `language_name`.
    forms_english: "English",
    /// Text in `language_name`.
    forms_spanish: "Spanish",
    /// Text in `language_name`.
    forms_french: "French",
    /// Text in `language_name`.
    forms_italian: "Italian",
    /// Text in `language_name`.
    forms_czech: "Czech",
    /// Text in `language_name`.
    forms_portuguese: "Portuguese",
    /// Text in `language_name`.
    forms_japanese: "Japanese",
    /// Text in `language_name`.
    forms_korean: "Korean",
    /// Text in `language_name`.
    forms_chinese_simplified: "Chinese (Simplified)",
    /// Text in `language_name`.
    forms_chinese_traditional: "Chinese (Traditional)",
    /// Text in `typed_mnemonic`. `{}`: `k + 1`, `language_name(lang)`.
    forms_word_not_in_list: "Word {} is not on the {} list",
        }
    };
}
