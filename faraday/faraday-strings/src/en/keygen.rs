//! Faraday's English words for the keygen area.

macro_rules! words_keygen {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- keygen (faraday-core/src/keygen/mod.rs) -----
    /// Text in `source_name`.
    keygen_seed_xor: "Seed XOR",
    /// Text in `procedure_name`.
    keygen_words_chosen: "Words chosen by the dice",
    /// Error in `deal`.
    keygen_no_secret_yet: "No secret yet",
    /// Text in `deal`. `{}`: `e`.
    keygen_the_shares_could: "The shares could not be made: {}",
    /// Text in `caution_lines`.
    keygen_caution_long_run: "A long run of one result",
    /// Text in `caution_lines`.
    keygen_far_from_even: "Far from even: one result came up much more than the others",
    /// Text in `caution_lines`.
    keygen_a_long_run_counting: "A long run counting up or down",
    /// Text in `caution_lines`: cards in the order a new deck comes in.
    keygen_cards_in_deck_order: "Cards in the order of a new deck",
    /// Text in `roll`.
    keygen_reroll_rule: "A 5 or a 6 is rolled again in a word's first five rolls",
    /// Note in `die_flip`.
    keygen_a_die_has_six: "A die has six faces",
    /// Note in `type_char`.
    keygen_no_more_are: "No more are needed",
    /// Text in `card`.
    keygen_card_already_drawn: "That card is already out of this deck",
    /// Note in `take_frame`.
    keygen_no_camera_picture: "No picture from the camera yet",
    /// Note in `take_frame`.
    keygen_too_little_variation: "Too little variation in that picture",
    /// Note in `take_frame`: the same frame came again.
    keygen_picture_already_taken: "That picture was already taken",
    /// Note in `finish_entry`. `{}`: `have`, `need`.
    keygen_entry_count: "{} of {}",
    /// Toast in `keygen_open`. `{}`: `self.medium.noun()`.
    keygen_remove_the_first: "Remove the {} first",
    /// Text in `keygen_act`.
    keygen_choose_two_sources: "Choose two sources or more",
    /// Note in `source_next`: a mix of sources holds dice or this device.
    keygen_mix_needs_dice_or_device: "A mix takes dice or this device among its sources",
    /// Text in `keygen_act`.
    keygen_choose_a_source: "Choose a source",
    /// Line under the second of two passphrases entered, and the note when the action is pressed with them differing.
    pass_pair_differ: "The two passphrases differ",
    /// Text in `keygen_lock`. `{}`: `self.new_keys + 1`.
    keygen_new_key_n: "New key {}",
    /// Text in `keygen_done`. `{}`: `crate::wallet::fp_text(faraday_bip::keys:…`, `if k.slip39 { FEN.keygen_shares_inline } els…`.
    keygen_key_added_back: "Key {} added; back up its {} before you rely on it",
    /// Text in `keygen_done`.
    keygen_shares_inline: "shares",
    /// Text in `keygen_done`.
    keygen_words_inline: "words",
    // ----- keygen (faraday-core/src/keygen/screen.rs) -----
    /// Text in `TITLES`.
    keygen_check: "Check",
    /// Text in `TITLES`.
    keygen_words: "Words",
    /// Text in `draw`.
    keygen_make_a_new_wallet: "Make a new wallet",
    /// Text in `draw`.
    keygen_add_a_key: "Add a key",
    /// New key's Start card.
    keygen_start: "Start",
    /// New key's Start card.
    keygen_make_a_new_key: "Make a new key",
    /// New key's Start card.
    keygen_type_the_words: "Type the words",
    /// New key's Start card.
    keygen_scan_a_seedqr: "Scan a SeedQR",
    /// New key's Start card.
    keygen_make_slip_39_shares: "Make SLIP-39 shares",
    /// New key's Start card.
    keygen_type_slip_39_shares: "Type SLIP-39 shares",
    /// New key's Start card.
    keygen_type_a_codex32: "Type codex32 strings",
    /// New key's Start card.
    keygen_type_seed_xor: "Type Seed XOR parts",
    /// Text in `draw`.
    keygen_tools: "Tools",
    /// Text in `draw`.
    keygen_back: "Back",
    /// Text in `draw`.
    keygen_new_key: "New key",
    /// Text in `entries_title`.
    keygen_flips: "Flips",
    /// Text in `entries_title`.
    keygen_cards: "Cards",
    /// Text in `entries_title`.
    keygen_pictures: "Pictures",
    /// Text in `entries_title`.
    keygen_generator: "Generator",
    /// Text in `entries_title`.
    keygen_entries: "Entries",
    /// Text in `entries_title`. `{}`: `what`, `(k.mix_at + 1).min(n)`, `n`.
    keygen_source_count: "{} · source {} of {}",
    /// Text in `asks`. `{}`: `k.dice_needed()`.
    keygen_rolls_of_a_six: "{} rolls of a six-sided die",
    /// Text in `asks`. `{}`: `CoinFlips::needed(st)`.
    keygen_flips_or_rolls: "{} flips, or rolls of a die read as flips",
    /// Text in `asks`. `{}`: `CardDraws::needed(st)`.
    keygen_cards_source: "{} cards from a shuffled deck",
    /// Text in `asks`. `{}`: `st.hex_digits()`.
    keygen_digits_made: "{} digits made elsewhere",
    /// Text in `asks`. `{}`: `CameraNoise::needed(st)`.
    keygen_pictures_n: "{} pictures",
    /// Text in `asks`.
    keygen_two_or_more: "Two or more of these, combined",
    /// This device's randomness, in `asks` and `device_rows`. `{}`: `EN.create_device_os`, the source the shell named (`/dev/random`, `getrandom(2)`, …).
    keygen_device_source: "{} · {}",
    /// This device's randomness once the person has added their own,
    /// which it mixes in (OWN-RANDOMNESS §2.4). `{}`: the line above.
    keygen_device_with_own: "{} · with your own",
    /// Text in `summary`. `{}`: `k.slip_m`, `k.slip_n`, `k.words`.
    keygen_slip_39_of_shares: "SLIP-39 · {} of {} shares · {} words each",
    /// Text in `summary`. `{}`: `source_name(Source::Coins)`.
    keygen_coins_from_die: "{} · read from a die",
    /// Text in `summary`.
    keygen_not_chosen: "Not chosen",
    /// Text in `summary`.
    keygen_complete: "Complete",
    /// Text in `summary`.
    keygen_not_locked: "Not locked in",
    /// Text in `summary`. `{}`: `fp`.
    keygen_locked_summary: "{} · locked in",
    /// Text in `summary`. `{}`: `fp`.
    keygen_passphrase_set: "{} · passphrase set · locked in",
    /// Text in `summary`. `{}`: `fp`.
    keygen_locked_no_passphrase: "{} · passphrase not set · locked in",
    /// Text in `summary`. `{}`: `k.mixed.len()`.
    keygen_sources_combined: "{} sources combined",
    /// Text in `summary`.
    keygen_no_cautions: "No cautions",
    /// Text in `summary`.
    keygen_1_caution: "1 caution",
    /// Text in `summary`. `{}`: `n`.
    keygen_cautions: "{} cautions",
    /// Text in `summary`. `{}`: `n`.
    keygen_shares_n: "{} shares",
    /// Text in `summary`.
    keygen_not_yet: "Not yet",
    /// Label in `read_only`.
    keygen_length: "Length",
    /// Label in `read_only`.
    keygen_randomness: "Randomness",
    /// Line in `verdict`.
    keygen_fair_no_cautions: "Fair: no cautions",
    /// Label in `key_card`.
    keygen_passphrase_again: "Passphrase again",
    /// Text in `key_card`.
    keygen_lock_button: "Lock in",
    /// Line in `key_card`. `{}`: the sample's name (`sample_words`), its
    /// fingerprint.
    keygen_lock_in_adds: "Lock in adds sample {} ({}) in place of these words",
    /// Line in `handed_line`. `{}`: the sample's name (`sample_words`), its
    /// fingerprint.
    keygen_added_sample: "Added sample {} ({}) in place of the key made",
    /// Line in `demo_lines_now` and `handed_line`. `{}`: the sample's name
    /// (`sample_guide`, ...).
    keygen_words_are_sample: "These words are sample {}",
    /// Text in `key_card` and `handed_line`. `{}`: the word, how many times
    /// it is repeated.
    sample_words: "{} ×{}",
    /// Line in `key_card`.
    keygen_enter_locks: "Enter locks in",
    /// Label in `key_card`.
    keygen_passphrase: "Passphrase",
    /// Text in `key_card`.
    keygen_set: "Set",
    /// Text in `key_card`.
    keygen_not_set: "Not set",
    /// Label in `key_card`.
    keygen_first_address: "First address",
    /// Label in `key_card`.
    keygen_key: "Key",
    /// Text in `SLIP39_FACTS`.
    keygen_fewer_shares: "Fewer shares than the threshold say nothing about the seed: with them, every seed stays as likely as every other. No entropy is lost to a holder of too few shares.",
    /// Text in `SLIP39_FACTS`.
    keygen_each_share_states: "Each share states in the clear its backup's identifier, the shares needed, how many were dealt and its own number: one share tells its holder how many others to look for.",
    /// Text in `SLIP39_FACTS`.
    keygen_the_threshold: "The threshold of shares rebuilds the whole seed on one device, and that device holds all of it while it signs.",
    /// Text in `SLIP39_FACTS`.
    keygen_the_split_s: "The split's own randomness comes from the device that deals it, here this device's system generator. A dealer with weak randomness gives the seed away in its shares.",
    /// Text in `SLIP39_FACTS`.
    keygen_a_share_cannot: "A share cannot be checked on its own against the seed; a wrong or mixed-up share shows only when the threshold is put together, by a 4-byte digest.",
    /// Text in `SLIP39_FACTS`.
    keygen_shares_restore: "Shares restore only in software that reads SLIP-39, such as Trezor's; a wallet that reads only BIP-39 words cannot.",
    /// Text in `SLIP39_FACTS`.
    keygen_slip39_share_length: "A share is 20 words for a 128-bit seed, the strength of 12 BIP-39 words, or 33 words for 256 bits.",
    /// Text in `length`.
    keygen_shares_needed: "Shares needed",
    /// Text in `length`.
    keygen_shares_dealt: "Shares dealt",
    /// Text in `length`. `{}`: `k.slip_m`, `k.slip_n`.
    keygen_any_of_the_shares: "Any {} of the {} shares restore the key",
    /// Line in `length`.
    keygen_what_shares: "What shares reveal",
    /// Text in `length_counts`. `{}`: `n`.
    keygen_words_n: "{} words",
    /// The Length card's button that opens the menu of 15, 18 and 21
    /// words, and that menu's title.
    keygen_other_lengths: "Other",
    /// The Generator card's line once randomness of the person's own is
    /// in the pool This device draws from.
    keygen_randomness_added: "Randomness added to the pool",
    /// The Generator card's text button: on to the words with none of
    /// the person's own randomness added.
    keygen_skip_randomness: "Skip",
    /// Text in `way_name`. `{}`: `source_name(Source::Dice)`.
    keygen_flip_mode: "{} · Flip mode",
    /// Text in `way_line`. `{}`: `CoinFlips::needed(st)`.
    keygen_rolls_read_as: "{} rolls, read as flips",
    /// Text in `group_head`.
    keygen_your_own_entropy: "Your own entropy, words verifiable by hand",
    /// Text in `group_head`.
    keygen_your_entropy: "Your entropy, computer generates words",
    /// Text in `group_head`.
    keygen_device_group: "Made by this device",
    /// Chip in `group_head`.
    keygen_recommended: "Recommended",
    /// Text in `entries`.
    keygen_rolls_inline: "rolls",
    /// Text in `entries`.
    keygen_flips_inline: "flips",
    /// Text in `entries`.
    keygen_cards_inline: "cards",
    /// Text in `entries`.
    keygen_digits_inline: "digits",
    /// Text in `entries`.
    keygen_pictures_inline: "pictures",
    /// Text in `entries`. `{}`: `have`, `need`, `unit`.
    keygen_entry_count_unit: "{} of {} {}",
    /// Text in `entries`. `{}`: `last`.
    keygen_last: "Last: {}",
    /// Text in `entries`. `{}`: `k.cards.deck()`, `k.cards.in_deck()`.
    keygen_deck_status: "Deck {} · {} drawn from it",
    /// Text in `entries`. `{}`: `st.distinct`.
    keygen_luma_values: "{} luma values in the last picture",
    /// Text in `entries`.
    keygen_camera_active: "Camera on",
    /// Text in `entries`.
    keygen_waiting_camera: "Waiting for the camera",
    /// Line in `entries`.
    keygen_no_need_to_write: "No need to write the words down yet: Back up shows them again.",
    /// Section label in `entries`.
    keygen_randomness_check: "Randomness check",
    /// Text in `entries`.
    keygen_next_source: "Next source",
    /// Text in `entries`.
    keygen_make_the_words: "Make the words",
    /// Text in `entries`.
    keygen_undo: "Undo",
    /// Text in `entries`.
    keygen_clear: "Clear",
    /// Text in `entries`.
    keygen_dice_keys_hint: "Type 1 to 6 · Backspace takes one back · Enter continues",
    /// Text in `entries`.
    keygen_type_h_or_t: "Type H or T · Backspace takes one back · Enter continues",
    /// Text in `entries`.
    keygen_type_the_rank: "Type the rank (A 2–9 T J Q K), then the suit (S H D C)",
    /// Text in `entries`.
    keygen_hex_keys_hint: "Type 0–9 and a–f · Backspace takes one back · Enter continues",
    /// Text in `entries`.
    keygen_space_takes: "Space takes a picture · Enter continues",
    /// Text in `entries`.
    keygen_enter_continues: "Enter continues",
    /// Text in `entry_style`.
    keygen_coin: "Coin",
    /// Text in `entry_style`.
    keygen_die: "Die",
    /// Text in `entry_style`.
    keygen_typed: "Typed",
    /// Text in `entry_style`.
    keygen_buttons: "Buttons",
    /// Text in `entry_style`.
    keygen_dice_flip_rule: "Each roll is one flip: 1, 2, 3 are 0 (tails); 4, 5, 6 are 1 (heads). The key is the one these flips make",
    /// Text in `entry_style`.
    keygen_heads_is_1_tails: "Heads is 1, tails is 0",
    /// Text in `typed_box`.
    keygen_h_or_1_for_heads: "H or 1 for heads, T or 0 for tails",
    /// Text in `typed_box`.
    keygen_dice_faces_caption: "Faces 1 to 6, as rolled",
    /// Text in `reveal`. `{}`: `part`.
    keygen_of_11_bits: "{} of 11 bits",
    /// Row in `reveal`.
    keygen_from_the_checksum: "From the checksum, once the rest is in",
    /// Text in `reveal`. `{}`: `checksum`, `last + 1`.
    keygen_the_last_bits: "The last {} bits of word {} are the checksum",
    /// Section label in `rolled_words`.
    keygen_words_so_far: "Words so far",
    /// Line in `rolled_words`.
    keygen_dice_word_rule: "Rolls 1 to 5 of a word: 1 = 00, 2 = 01, 3 = 10, 4 = 11, 5 or 6 rolled again · Roll 6: 1 to 3 = 0, 4 to 6 = 1 · The 11 bits are the word's number in the list, from 0",
    /// Line in `rolled_words`.
    keygen_checksum: "checksum",
    /// Text in `rolled_words`. `{}`: `i`, `i + 1`.
    keygen_word: "= {} · word {}",
    /// Text in `rolled_words`. `{}`: `i`, `i + 1`.
    keygen_word_number: "= {} · word {} of the list",
    /// Note in `rolled_words`. `{}`: `got.len()`, `per`.
    keygen_of_rolls: "{} of {} rolls",
    /// Row in `rolled_words`.
    keygen_the_checksum: "The checksum, from all the other bits, once every roll is in",
    /// Text in `device_rows`.
    keygen_result: "Result",
    /// Text in `check`.
    keygen_continue: "Continue",
    /// Text in `again_label`.
    keygen_start_again: "Start again",
    /// Label in `check_rows`.
    keygen_rolls: "Rolls",
    /// Label in `check_rows`.
    keygen_faces: "Faces",
    /// Label in `check_rows`.
    keygen_suits: "Suits",
    /// Label in `check_rows`.
    keygen_digits: "Digits",
    /// Text in `check_rows`. `{}`: `hex`.
    keygen_sha_256: "SHA-256 {}…",
    /// Label in `share_words`.
    keygen_share: "Share",
    /// Text in `share_words`. `{}`: `k.share_at + 1`, `k.shares.len()`, `k.slip_m`.
    keygen_of_needed: "{} of {} · {} needed",
    /// Button in `share_words`.
    keygen_previous_share: "Previous share",
    /// Button in `share_words`.
    keygen_next_share: "Next share",
    /// Button in `share_words`.
    keygen_i_wrote_them_all: "I wrote them all down",
    /// Line in `words`.
    keygen_not_made_yet: "Not made yet",
    /// Label in `words`.
    keygen_source: "Source",
    /// Text in `words`.
    keygen_hide_words: "Hide words",
    /// Text in `words`.
    keygen_show_words: "Show words",
    /// Text in `words`.
    keygen_i_wrote_them: "I wrote them down",
    /// Line in `quiz`.
    keygen_show_the_words_first: "Show the words first",
    /// Label in `quiz`.
    keygen_fingerprint: "Fingerprint",
    /// Text in `quiz`. `{}`: `k.shares.len()`.
    keygen_all_shares_right: "All {} shares right",
    /// Text in `quiz`. `{}`: `n`.
    keygen_all_words_right: "All {} words right",
    /// Text in `quiz`.
    keygen_quiz_skipped: "Skipped: the copy is not checked",
    /// Label in `quiz`.
    keygen_quiz: "Quiz",
    /// Text in `quiz`.
    keygen_done: "Done",
    /// Text in `quiz`.
    keygen_use_this_key: "Use this key",
    /// Text in `quiz`.
    keygen_add_key: "Add key",
    /// Text in `quiz`. `{}`: `k.quiz_share + 1`, `k.shares.len()`, `done`, `n`.
    keygen_quiz_progress: "Share {} of {} · {} of {} right",
    /// Text in `quiz`. `{}`: `done`, `n`.
    keygen_of_right: "{} of {} right",
    /// Text in `quiz`. `{}`: `q.position() + 1`.
    keygen_which_is_word: "Which is word {}?",
    /// Line in `quiz`.
    keygen_not_that_one: "Not that one. Check your copy against the words",
    /// Button in `quiz`.
    keygen_ask_it_again: "Ask it again",
    /// Button in `quiz`.
    keygen_show_the_words: "Show the words",
    /// Line in `quiz`.
    keygen_quiz_skip_warning: "Without the quiz, a word copied wrongly is found only when the backup is needed",
    /// Text in `quiz`.
    keygen_skip_and_add: "Skip: the key waits until its shares are typed back",
    /// Button in `quiz`.
    keygen_skip_the_quiz: "Skip the quiz",
    /// Text in `quiz`.
    keygen_quiz_keys_add: "Type 1 to 4 to pick · Enter adds the key once it passes",
    /// Text in `quiz`.
    keygen_quiz_keys_done: "Type 1 to 4 to pick · Enter is Done once it passes",
        }
    };
}
