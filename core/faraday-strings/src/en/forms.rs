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
    /// Text in `name`.
    forms_extended_private_key: "Extended private key (xprv)",
    /// Text in `name`.
    forms_master_seed_in_hex: "Master seed in hex",
    /// The empty field of an extended private key.
    forms_xprv_hint: "xprv… or tprv…",
    /// The empty field of a master seed in hex.
    forms_hex_seed_hint: "32 to 128 hex digits",
    /// Text in `form_key`.
    forms_xprv_key: "xprv key",
    /// Text in `form_key`.
    forms_hex_seed_key: "Hex seed key",
    /// The button that adds the key a line of text is.
    forms_add_key: "Add key",
    /// An extended private key's text is not base58 or its checksum fails.
    forms_xprv_base58: "Not an extended private key: it is not base58 with a good checksum",
    /// `{}`: the length in bytes.
    forms_xprv_length: "Not an extended private key: it decodes to {} bytes, not 78",
    /// The four version bytes are not xprv's or tprv's.
    forms_xprv_version: "Not an extended private key: its version is not xprv or tprv",
    /// A key at depth 0 with a parent or a child number.
    forms_xprv_zero_depth: "Not an extended private key: at depth 0 it has a parent or a child number",
    /// The pad byte before the key is not zero.
    forms_xprv_pad: "Not an extended private key: the byte before the key is not zero",
    /// The key is zero or not below the curve's order.
    forms_xprv_key_range: "Not an extended private key: the key is out of range",
    /// `{}`: the spelling, such as zprv.
    forms_xprv_slip132: "That is a {} key, a SLIP-132 spelling: Faraday reads xprv and tprv only",
    /// `{}`: the version written, the version this session reads.
    forms_xprv_wrong_network: "The key is {}: this session reads {}",
    /// A valid key below depth 0. `{}`: depth, parent fingerprint, child number.
    forms_xprv_not_master: "Not a master key: depth {}, parent {}, child {}",
    /// The origin before an extended private key is not a fingerprint.
    forms_xprv_origin_unreadable: "The origin before the key is not [ and eight hex digits and ]",
    /// The origin before an extended private key has a path.
    forms_xprv_origin_path: "The origin has a path: only the key's own fingerprint is taken",
    /// The origin names another key. `{}`: the fingerprint named, the key's own.
    forms_xprv_origin_differs: "The origin names {}, but the key is {}",
    /// A hex seed with a character that is not a hex digit or a space.
    forms_hex_not_hex: "Not hex: only 0-9 and a-f or A-F",
    /// `{}`: the digit count.
    forms_hex_odd: "{} hex digits: a seed has an even count",
    /// `{}`: the length in bytes.
    forms_hex_length: "{} bytes: a master seed is 16 to 64 bytes",
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
    /// Error where the session takes codex32 strings: two differ at one index.
    forms_string_clash: "Two different strings are given for the same index",
    /// Error in `form_add_part`.
    forms_share_duplicate: "That share is in already",
    /// A SLIP-39 share past the most a set can have (16 groups of 16).
    forms_shares_full: "256 shares are in: a SLIP-39 set has no more",
    /// A passphrase field's line for a letter it refuses: a BIP-39 or
    /// aezeed passphrase here is printable ASCII (docs/STANDARDS.md). It
    /// names no letter: the field is masked.
    passphrase_ascii_only: "A passphrase here is printable ASCII only",
    /// A passphrase field at the most its standard holds.
    passphrase_too_long: "That passphrase is as long as one can be",
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
