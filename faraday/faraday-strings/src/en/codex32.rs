//! Faraday's English words for the codex32 area.

macro_rules! words_codex32 {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- codex32 blank templates (faraday-paper/src/pdf.rs) -----
    /// Error in `sheet`: a codex32 template of another length.
    pdf_c32_length: "the codex32 template is for 48, 74 or 127 characters",
    /// Title of the share card. `{}`: the length in characters.
    pdf_c32_card: "Codex32 share · {} characters",
    /// Line on the share card.
    pdf_c32_hand: "Write one character in each box. Write 0, 7, S and Z with a slash.",
    /// Label under box 4 of the share card.
    pdf_c32_threshold: "Threshold",
    /// Label under boxes 5 to 8 of the share card.
    pdf_c32_identifier: "Identifier",
    /// Label under box 9 of the share card.
    pdf_c32_index: "Share index",
    /// Label over the alphabet at the foot of the share card.
    pdf_c32_alphabet: "The 32 characters",
    /// Error when a Krux sheet's side is not one of the four.
    pdf_krux_size: "the Krux code template is for 25, 29, 33 or 37 squares",
    /// Title of the Krux code sheet. `{}`, `{}`: squares across and down.
    pdf_krux_title: "Krux code · {} × {} squares",
    /// Line on the Krux code sheet about the passphrase.
    pdf_krux_passphrase: "The passphrase is never written on this sheet.",
    /// Line on the Krux code sheet. `{}`: blocks across, last row letter.
    pdf_krux_hand: "Fill each square the code shows black, block by block: columns 1 to {}, rows A to {}.",
    /// Label on the Krux code sheet.
    pdf_krux_version: "Version",
    /// Label on the Krux code sheet.
    pdf_krux_rounds: "Rounds",
    /// Label on the Krux code sheet.
    pdf_krux_id: "Id",
    /// Caption of the Krux code grid.
    pdf_krux_grid: "The black squares are printed already; the format squares beside them are drawn by hand.",
    /// Caption of the Codex32QR grid on the share card.
    pdf_c32_grid: "Codex32QR · the black squares are printed already",
    /// Title of the Codex32QR sheet.
    pdf_c32qr_title: "Codex32QR · 48 characters",
    /// Line on the Codex32QR sheet.
    pdf_c32qr_hand: "Fill each square the device shows black, block by block: columns 1 to 6, rows A to F.",
    /// Title of the page of four Codex32QR grids.
    pdf_c32qr_grids_title: "Codex32QR · four grids",
    /// Title of the dice worksheet, the booklet's (two pages). `{}`: the
    /// page, the pages.
    pdf_c32_dice: "Dice de-biasing worksheet · page {} of {}",
    /// The printed dice worksheet's credit, at the foot of each page
    /// (owner, 2026-10-24; never on a screen).
    pdf_c32_dice_credit: "Dice worksheet after the codex32 booklet, © 2022 Blockstream, MIT.",
    /// Title of the checksum worksheet. `{}`: the length in characters.
    pdf_c32_checksum: "Checksum worksheet · {} characters",
    /// A worksheet title with its page. `{}`: the title, the page, the pages.
    pdf_c32_page: "{} · page {} of {}",
    /// Key of the checksum worksheet: the bold boxes.
    pdf_c32_key_given: "Given",
    /// Key of the checksum worksheet: the pink boxes.
    pdf_c32_key_checksum: "Checksum",
    /// Key of the checksum worksheet: the grey rows.
    pdf_c32_key_lookup: "Row of the checksum table",
    /// Title of a translation worksheet. `{}`: k, the length in characters.
    pdf_c32_translation: "Translation worksheet · k = {} · {} characters",
    /// Label of an input string on the translation worksheet. `{}`: its index.
    pdf_c32_share: "Share {}",
    /// Label over the translation symbol's box.
    pdf_c32_symbol: "Symbol",
    /// Label of the sum row on the translation worksheet.
    pdf_c32_new_share: "New share",
    /// Title of a derivation table. `{}`: k.
    pdf_c32_derivation: "Derivation table · k = {}",
    /// Title of the page of the principal tables.
    pdf_c32_tables: "Tables",
    /// Name of the Addition table.
    pdf_c32_addition: "Addition",
    /// Name of the Translation table.
    pdf_c32_translation_table: "Translation",
    /// Name of the Recovery table.
    pdf_c32_recovery: "Recovery",
    /// Name of the Fusion table.
    pdf_c32_fusion: "Fusion",
    /// Title of a page of the checksum table. `{}`: the page, the pages.
    pdf_c32_checksum_table: "Checksum table · {} of {}",
    // ----- Codex32 error correction (docs/CODEX32.md §8.4), faraday-core -----
    /// The tick on a codex32 form, off whenever the form opens.
    codex32_correct_suggest: "Suggest corrections",
    /// Title over a proposal's pairs.
    codex32_correct_possible: "Possible correction",
    /// One box a proposal changes: `{}` the box, counted from 1 at `m`, what it was, what it would be.
    codex32_correct_pair: "Box {} · {} → {}",
    /// The page of pairs on show: `{}` the page, `{}` how many.
    codex32_correct_page: "Page {} of {}",
    /// Button: the proposal's previous page.
    codex32_correct_previous: "Previous",
    /// Button: the proposal's next page.
    codex32_correct_next: "Next",
    /// Button: the paper is right, the typing was wrong.
    codex32_correct_mistyped: "I mistyped",
    /// Button: the paper itself is wrong.
    codex32_correct_damaged: "The paper is damaged",
    /// Button: the boxes marked `?` cannot be read on the paper.
    codex32_correct_unreadable: "Unreadable on the paper",
    /// Button: back to the string as typed, to edit it.
    codex32_correct_check_boxes: "Check the boxes",
    /// Button: on past a caution or the repairs.
    codex32_correct_continue: "Continue",
    /// The caution before a proposal that fills a long run of unreadable boxes.
    codex32_correct_assumes: "This assumes every other box is right",
    /// Title over the pairs as repairs to write on a damaged paper.
    codex32_correct_repairs: "Repairs to write on the paper",
    /// Title over the re-entry of the changed boxes.
    codex32_correct_type_again: "Type the changed boxes from the paper",
    /// The re-entry's count: `{}` boxes typed, `{}` to type.
    codex32_correct_typed_of: "{} of {} boxes typed",
    /// The re-entry differed from the proposal: nothing taken.
    codex32_correct_differs: "The boxes typed differ from the correction",
    /// The caution before a proposal for unreadable boxes is taken as it is.
    codex32_correct_from_checksum: "These characters come from the checksum, not from the paper",
    /// Button: take the proposal as it is.
    codex32_correct_take: "Take the correction",
    /// No correction within BIP 93's bound.
    codex32_correct_not_found: "No correction found within 4 changes or 8 unreadable boxes",
    /// Too many unreadable boxes: `{}` the most, `{}` the longest run filled.
    codex32_correct_too_many_unreadable: "More unreadable boxes than can be filled: at most {}, or one run of up to {}",
    /// Boxes 4 to 8 differ from the strings already in, or are unreadable.
    codex32_correct_header_fixed: "Boxes 4 to 8 must read as in the strings already in",
    /// The text is not a codex32 string even with its unreadable boxes: `{}` why.
    codex32_correct_malformed: "Not a codex32 string: {}",
    /// The one correction is not a codex32 string: `{}` why.
    codex32_correct_not_a_string: "The correction is not a codex32 string: {}",
    /// Beside the fingerprint of a key whose strings were typed again from the paper.
    codex32_correct_entry_corrected: "Share entry corrected",
    /// Beside the fingerprint of a key whose strings were repaired or filled from the checksum.
    codex32_correct_repairs_unchecked: "Share repairs unchecked",
    /// Beside a string repaired or filled from the checksum.
    codex32_correct_repaired_not_checked: "Repaired, not checked",
    // ----- The dice roller on Faraday Demo (docs/CODEX32.md §5.6), faraday-core -----
    /// Label over the roller's die: what its values are.
    roller_by_this_computer: "Rolled by this computer",
    /// Button: roll the die as many times as one click rolls.
    roller_roll: "Roll",
    /// Button: stop a click's rolls where they are.
    roller_stop: "Stop",
    /// Choice: one roll per click.
    roller_one: "1",
    /// Choice: five rolls per click.
    roller_five: "5",
    /// Choice: ten rolls per click.
    roller_ten: "10",
    /// Choice: rolls until the entry is full.
    roller_to_the_end: "To the end",
    /// The typed number's field when it is empty.
    roller_number: "Number",
    // ----- The five-dice roller (docs/CODEX32.md §5.5), faraday-core/src/fresh/roller/ -----
    /// Heading of the codex32 dice step's roller, beside the entry for real dice.
    roller_or_let_faraday_roll: "Or let Faraday roll",
    /// Result's entropy row after the roller made every value.
    roller_entropy_simulated: "Entropy · simulated dice",
    /// Result's entropy row after the roller and real dice made the values.
    roller_entropy_both: "Entropy · simulated dice and your dice",
    // ----- Make codex32 shares (docs/CODEX32.md §6, §7), faraday-core/src/codex32/ -----
    /// New key's Start card: the label over the ways to add a key held.
    keygen_add_a_key_you_have: "Add a key you have",
    /// New key's Start card, under Make a new key: BIP-39 words.
    keygen_bip39_words: "BIP-39 words",
    /// New key's Start card, under Make a new key: Make codex32 shares.
    keygen_codex32: "Codex32",
    /// The Start card closed on a way under Make a new key. `{}`: the way.
    keygen_start_new_key: "Make a new key · {}",
    /// Make codex32 shares' card in New key's step list.
    codex32_codex32: "Codex32",
    /// Card title (§6.1).
    codex32_scheme: "Scheme",
    /// Card title (§6.2), and a row of Strings made.
    codex32_identifier: "Identifier",
    /// Card title (§6.3), and a row of Strings made.
    codex32_entropy: "Entropy",
    /// Card title (§6.4) once every character is in.
    codex32_enter: "Characters",
    /// Card title (§6.5).
    codex32_worksheets: "Worksheets",
    /// Card title (§6.6), a stepper's label, and the closed card's name.
    codex32_strings: "Strings",
    /// Card title (§6.7).
    codex32_strings_made: "Strings made",
    /// The entry's title. `{}`: the share ("Share A"), the character, the characters in all.
    codex32_share_character: "{} · character {} of {}",
    /// The entry's title where the bar has no room for the whole one
    /// (the Pi's panel). `{}`: the share, the character, the characters
    /// in all.
    codex32_share_character_short: "{} · {} of {}",
    /// The Strings card's title. `{}`: which string, how many.
    codex32_share_n_of: "Share {} of {}",
    /// A share named by its index. `{}`: the index, upper case.
    codex32_share_x: "Share {}",
    /// The string whose index is s.
    codex32_the_secret: "Secret",
    /// The Scheme card closed. `{}`: the length, the split.
    codex32_scheme_summary: "{} characters · {}",
    /// A split. `{}`: the threshold, the strings.
    codex32_k_of_n: "{} of {}",
    /// A card closed once its work is done.
    codex32_done: "Done",
    /// The Strings card closed. `{}`: how many strings.
    codex32_strings_n: "{} written",
    /// The Scheme card's first Choice, and a row of Strings made.
    codex32_length: "Length",
    /// Length: 48 characters.
    codex32_chars_48: "48 characters",
    /// Length: 74 characters.
    codex32_chars_74: "74 characters",
    /// The Scheme card's second Choice, and a row of Strings made.
    codex32_split: "Split",
    /// Split: 2 of 3.
    codex32_two_of_three: "2 of 3",
    /// Split: 3 of 5.
    codex32_three_of_five: "3 of 5",
    /// Split: the steppers.
    codex32_other: "Other",
    /// Split: no threshold.
    codex32_split_alone: "None, the secret alone",
    /// Other's first stepper.
    codex32_threshold: "Threshold",
    /// Under the split where k is 4 or more. `{}`: k.
    codex32_no_translation_sheet: "The booklet publishes no translation sheet for k = {}; derived shares are this device's arithmetic",
    /// The identifier's Suggest button.
    codex32_suggest: "Suggest",
    /// The identifier's Continue before four characters are in.
    codex32_four_characters: "Four characters",
    /// Where an identifier is already used: the codex32 strings in Add a key.
    codex32_used_in_add_a_key: "Used by the strings typed in Add a key",
    /// Where an identifier is already used: the set of a key loaded from
    /// codex32 in this session. `{}`: the key's fingerprint.
    codex32_used_by_key_set: "Used by the set of key {}",
    /// Where an identifier is already used: a codex32 record of an open
    /// vault. `{}`: the vault's name.
    codex32_used_in_vault: "Used in vault {}",
    /// Where an identifier is already used: a codex32 form of the plan in
    /// hand.
    codex32_used_in_plan: "Used in this backup plan",
    /// The Entropy card's Choice: dice.
    codex32_roll_dice: "Roll dice",
    /// The Entropy card's Choice: characters typed.
    codex32_type_the_characters: "Type the characters",
    /// Above the entry of characters typed (§5.2, owner 2026-10-23).
    codex32_not_chosen: "Characters from dice or a worksheet, not chosen",
    /// A refusal from the codec. `{}`: its reason.
    codex32_refused: "Refused: {}",
    /// The small panel's tile: the dice sheet.
    codex32_tile_dice: "Dice",
    /// The small panel's tile and the strip's first chip: the checksum worksheet.
    codex32_tile_checksum: "Checksum",
    /// The small panel's tile: the share's string.
    codex32_tile_string: "String",
    /// The dice de-biasing sheet's head.
    codex32_dice_sheet: "Dice de-biasing",
    /// A die's track. `{}`: its number.
    codex32_die_n: "Die {}",
    /// A die's pair. `{}`: first, second, lower or higher, the bit.
    codex32_die_then: "{} then {} · {} · {}",
    /// A second value lower than the first.
    codex32_lower: "lower",
    /// A second value higher than the first.
    codex32_higher: "higher",
    /// An equal pair, struck. `{}`: the die, its first and second value.
    codex32_die_equal: "Die {} · {} then {} · equal · roll both again",
    /// A die's pair in a cell's working. `{}`: the die, first, second, the bit.
    codex32_die_row: "Die {} · {} then {} · {}",
    /// The tree's path reaching its leaf. `{}`: the bits, their value, the character.
    codex32_path_leaf: "{} = {} → {}",
    /// The checksum worksheet's head. `{}`: the share.
    codex32_checksum_of: "Checksum of {}",
    /// The review's checksum worksheet head. `{}`: the share, where its data came from.
    codex32_checksum_of_marked: "Checksum of {} · {}",
    /// The data came from dice.
    codex32_from_your_rolls: "from your rolls",
    /// The data was typed.
    codex32_as_typed: "as typed",
    /// A worksheet row: the string.
    codex32_row_string: "String",
    /// A worksheet row: the residue of MS.
    codex32_row_ms: "MS",
    /// A worksheet row: a sum. `{}`: its number.
    codex32_row_sum: "Sum {}",
    /// A worksheet row: a lookup from the checksum table. `{}`: its number.
    codex32_row_lookup: "Table {}",
    /// A translation worksheet row: an input times its symbol. `{}`: the input.
    codex32_row_times: "{} × symbol",
    /// A translation worksheet's head. `{}`: what it makes.
    codex32_translation_to: "Translation to {}",
    /// Label over a sheet's translation symbols.
    codex32_translation_symbols: "Translation symbols",
    /// Label over a recovery sheet's symbols.
    codex32_recovery_symbols: "Recovery symbols",
    /// Label over a symbol's pair symbols.
    codex32_pair_symbols: "Pair symbols",
    /// An input's symbol. `{}`: the input, the symbol's name.
    codex32_symbol_of: "{} {}",
    /// The review's Previous, on the small panel.
    codex32_previous: "Previous",
    /// The review's Next, on the small panel.
    codex32_next: "Next",
    /// A review tab: a share's dice. `{}`: the share.
    codex32_tab_dice: "Dice · {}",
    /// A review tab: a share's checksum. `{}`: the share.
    codex32_tab_checksum: "Checksum · {}",
    /// A review tab: a derived share. `{}`: its index.
    codex32_tab_share: "Share {}",
    /// A rolled character in the dice review. `{}`: the box, the rows, the bits, the character.
    codex32_box_rolled: "Box {} · {} · {} → {}",
    /// A typed character in the dice review. `{}`: the box, the character.
    codex32_box_typed: "Box {} · typed · {}",
    /// A die's two values while the sheets are masked.
    codex32_die_masked: "• then •",
    /// A character in the dice review while the sheets are masked. `{}`: the box, the mask.
    codex32_box_masked: "Box {} · {}",
    /// The Strings card's toggle: the string.
    codex32_string: "String",
    /// The Strings card's toggle: the Codex32QR code.
    codex32_codex32qr: "Codex32QR",
    /// A string typed back with a box that differs. `{}`: the box.
    codex32_box_differs: "Box {} differs",
    /// A string typed back the same.
    codex32_typed_back_same: "Typed back the same",
    /// Continue before the string is typed back.
    codex32_type_it_back: "Type the string back, or skip",
    /// The Strings card's Skip.
    codex32_skip: "Skip",
    /// After a first press on Skip (§6.6).
    codex32_skip_caution: "Without typing it back, a box copied wrongly is found only when the strings are needed. Press Skip again to go on.",
    /// A row of Strings made.
    codex32_fingerprint: "Fingerprint",
    /// Strings made's entropy: dice.
    codex32_your_dice: "your dice",
    /// Strings made's entropy: characters typed.
    codex32_your_characters: "your characters",
    /// Strings made's entropy: both.
    codex32_your_dice_and_characters: "your dice and your characters",
    /// The entry's pad for real rolls, beside the roller (CODEX32 §5.5).
    codex32_your_dice_head: "Your dice",
    /// The Entropy card's row of dice, by their faces (CODEX32 §5.1).
    codex32_dice: "Dice",
    /// A die on that row. `{}`: its faces.
    codex32_die_size: "d{}",
    /// The Entropy card's row of each die's colour (CODEX32 §5.1).
    codex32_die_colours: "Colours",
    /// A die's colour, a press steps to the next. `{}`: the die, the colour.
    codex32_die_colour: "Die {} · {}",
    /// A die's colour.
    codex32_colour_red: "Red",
    /// A die's colour.
    codex32_colour_orange: "Orange",
    /// A die's colour.
    codex32_colour_yellow: "Yellow",
    /// A die's colour.
    codex32_colour_green: "Green",
    /// A die's colour.
    codex32_colour_blue: "Blue",
    /// A die's colour.
    codex32_colour_purple: "Purple",
    /// A die's colour.
    codex32_colour_white: "White",
    /// A die's colour.
    codex32_colour_black: "Black",
    /// The dice sheet's place for a die's marker before its first roll
    /// (the booklet's words), on screen and printed.
    codex32_free_space: "Free space",
    /// Head over the dice sheet's five tracks (the booklet's words).
    codex32_die_tracks: "Die tracks",
    /// Head over a track's pads 1 to 6 (the booklet's words).
    codex32_die_pads: "Die pads",
    /// Head over a track's pads 7 to 20, for dice of more faces (the
    /// booklet's words).
    codex32_die_pads_large: "Die pads (d7+)",
    /// Head over each die's first roll, its marker (CODEX32 §6.4.2).
    codex32_first_roll: "First roll",
    /// Head over each die's second roll, the die.
    codex32_second_roll: "Second roll",
    /// What a die's pair gives. `{}`: lower or higher, the bit.
    codex32_pair_bit: "{} · {}",
    /// The note when the dice are changed in the middle of a character
    /// (CODEX32 §5.1).
    codex32_dice_changed: "Dice changed: this character's rolls are cleared",
    /// Over the pad of ten-faced dice: a die numbered 0 to 9 enters its 0
    /// as 10 (CODEX32 §5.1).
    codex32_d10_zero: "0 is 10",
    /// A tens digit typed on the keyboard, waiting for its units digit,
    /// Space or Enter (dice of ten faces or more). `{}`: the digit.
    codex32_face_pending: "Face {}…",
    /// Strings made's button (§6.8).
    codex32_check_on_another: "Check on another device",
    /// Check on another device's row: the subsets to try.
    codex32_sets_to_try: "Sets to try",
    /// The cell sheet's Done.
    codex32_done_button: "Done",
    /// A cell sheet's head: a box. `{}`: the box.
    codex32_cell_box: "Box {}",
    /// A cell sheet's head: the residue of MS. `{}`: the box.
    codex32_cell_ms: "Box {} · residue of MS",
    /// A cell sheet's head: a sum. `{}`: the box, the row.
    codex32_cell_sum: "Box {} · sum row {}",
    /// A cell sheet's head: a lookup. `{}`: the box, the row.
    codex32_cell_lookup: "Box {} · table row {}",
    /// A cell named in a working. `{}`: its row, its box, its value.
    codex32_named_cell: "{}, box {}: {}",
    /// The residue of MS row's working.
    codex32_ms_residue: "The residue of MS: the same on every codex32 sheet",
    /// A cell's value in its working. `{}`: the value.
    codex32_this_is: "This cell: {}",
    /// How a sum is worked.
    codex32_addition_wheel: "Addition wheel, or the addition table",
    /// A checksum box's rule.
    codex32_worked_from_below: "Worked from the bottom row up: the cell below and the table cell added",
    /// The final row reads what it must. `{}`: the letter.
    codex32_should_read_does: "Should read {}: it does",
    /// The final row does not. `{}`: the letter.
    codex32_should_read_not: "Should read {}: it does not",
    /// A lookup's leading pair. `{}`: the row, the pair.
    codex32_sum_starts: "Sum row {} starts {}",
    /// A lookup's table row. `{}`: the pair, the character, of how many.
    codex32_table_row: "Checksum table, row {}, character {} of {} (booklet pages 15 and 16)",
    /// A header box: the threshold. `{}`: it.
    codex32_header_threshold: "Threshold · {}",
    /// A header box: the identifier. `{}`: it.
    codex32_header_identifier: "Identifier · {}",
    /// A header box: the share index. `{}`: it.
    codex32_header_index: "Share index · {}",
    /// A data cell typed.
    codex32_typed: "Typed",
    /// A translated cell. `{}`: the input, the box, its character, the symbol, the product.
    codex32_translated_is: "Share {}, box {}: {} × {} = {}",
    /// The translation wheel's setting. `{}`: the symbol.
    codex32_wheel_set_to: "Translation wheel set to {}",
    /// A translation sum. `{}`: the cells added, the sum.
    codex32_sum_is: "{} = {}",
    /// A symbol's value. `{}`: its name, its value.
    codex32_symbol_value: "{} · value {}",
    /// What a symbol is the fusion of. `{}`: the kind, the symbols.
    codex32_fused_from: "{}: {}",
    // ----- Codex32 strings typed, checked and scanned (docs/CODEX32.md §8.1, §8.3, §8.5, §10.1), faraday-core -----
    /// The same codex32 string typed or scanned again: nothing changes.
    codex32_form_already_in: "Already in",
    /// A string at a share index already in, which differs from the one held. `{}`: the index, upper case.
    codex32_form_clash: "Share {} is in already, and differs",
    /// Label over the string held, beside the one typed.
    codex32_form_held: "Held",
    /// Label over the string just typed or scanned.
    codex32_form_new: "This one",
    /// The boxes the two strings differ in. `{}`: the boxes, counted from 1 at `m`.
    codex32_form_differ: "Boxes {} differ",
    /// Keep the string held, drop the one typed.
    codex32_form_keep: "Keep the one held",
    /// Put the string typed in place of the one held.
    codex32_form_replace: "Replace it",
    /// A share whose checksum fails. `{}`: its index, upper case.
    codex32_form_share_fails: "Share {} · checksum fails",
    /// The secret string, its checksum failing.
    codex32_form_secret_fails: "Secret · checksum fails",
    /// The codex32 form's button to Check a string (§8.3).
    codex32_form_check_a_string: "Check a string",
    /// Check a string: its button.
    codex32_form_check_it: "Check it",
    /// Check a string: leaves for the form that adds strings.
    codex32_form_back_to_adding: "Back to adding strings",
    /// Check a string: the checksum holds.
    codex32_form_holds: "Checksum holds",
    /// Check a string: the checksum fails.
    codex32_form_fails: "Checksum fails",
    /// Check a string: the string is a share of the set typed in.
    codex32_form_of_the_set: "Of the set typed in",
    /// Check a string: the string is not of the set typed in.
    codex32_form_not_of_the_set: "Not of the set typed in",
    /// Check a string: what the string is. `{}`: the index, upper case; the identifier; the threshold.
    codex32_form_checked_share: "Share {} · id {} · {} needed",
    /// Check a string: the secret. `{}`: the identifier.
    codex32_form_checked_secret: "The secret · id {}",
    // Fewer presses (F103).
    /// Button: closes the board's Add list, a sheet on a small panel.
    board_list_close: "Close",
    // ----- Codex32 in backups (docs/CODEX32.md §9), faraday-core -----
    /// A codex32 string on paper, as the plan names it. `{}`: the seed, the share index.
    c32_plan_string: "Seed {}: codex32 {}",
    /// A codex32 string's Codex32QR drawn on paper. `{}`: the seed, the share index.
    c32_plan_qr: "Seed {}: Codex32QR {}",
    /// A codex32 string as a text file. `{}`: the seed, the share index.
    c32_plan_file: "Seed {}: codex32 {} file",
    /// A codex32 string's Codex32QR as a picture. `{}`: the seed, the share index.
    c32_plan_qr_file: "Seed {}: Codex32QR {} picture",
    /// A codex32 string, as the chart names it. `{}`: the key's number, the share index.
    c32_glance_string: "Key {} codex32 {}",
    /// The chart's sheet on a codex32 string: its checklist item.
    c32_glance_check: "Check my copy",
    /// That row's second line.
    c32_glance_strings: "Codex32 strings",
    /// The chart's sheet on a codex32 string: the set's actions.
    c32_glance_set: "Add, replace or rotate the set",
    /// A codex32 string typed back that holds. `{}`: its share index, the set's identifier.
    c32_holds: "{} · checksum holds, a string of {}",
    /// A codex32 string typed back whose checksum fails.
    c32_checksum_fails: "Checksum fails",
    /// A codex32 string typed back whose header is not the set's. `{}`: the box, the identifier.
    c32_not_of_set: "Box {} is not this set's: not a string of {}",
    /// Rotate the set, where enough old strings may be in other hands. `{}`: how many, the identifier.
    c32_rotate_not_enough: "{} strings of {} may be in other hands: rotating is not enough; move the coins to a new key",
    /// Rotate the set, every old string held. `{}`: the strings.
    c32_rotate_destroy_all: "Destroy {}. Then no string of the old set is left.",
    /// Rotate the set, some lost or exposed. `{}`: the held strings, the others, the threshold, the strings.
    c32_rotate_destroy: "Destroy {}. Then at most {} is left, and fewer than {} strings of a set of {} are worthless.",
    /// A share to add, where the set has every index.
    c32_set_full: "This set has every index",
    /// More strings to type for a share. `{}`: how many.
    c32_set_type_more: "Type {} strings of the set first",
    /// The strings typed do not rebuild this key.
    c32_set_not_this_seed: "These strings do not rebuild this key",
    /// The new set's identifier, not four characters.
    c32_set_identifier: "Type four characters for the new identifier",
    /// The new set's identifier, the old one's.
    c32_set_identifier_old: "That is the old set's identifier: type a new one",
    /// The checklist's codex32 item. `{}`: the seed.
    c32_item_title: "Codex32 strings of seed {}",
    /// A set of the secret alone.
    c32_secret_alone: "The secret alone",
    /// A split. `{}`: the threshold, the strings.
    c32_k_of_n: "{} of {}",
    /// A form's facts. `{}`: the split, the length.
    c32_facts: "{} · {} characters",
    /// A form's facts with its identifier. `{}`: the identifier, the rest.
    c32_facts_id: "Identifier {} · {}",
    /// A key of words split as codex32: what the strings hold.
    c32_holds_the_seed: "Holds · the seed, not the words. Not for checking by hand.",
    /// A string and where it is kept. `{}`: the index, the places.
    c32_string_at: "{} · {}",
    /// A string typed back the same, or one that rebuilds the key.
    c32_checked: "Checked",
    /// A string skipped past with nothing typed back.
    c32_skipped: "Skipped",
    /// A string not yet typed back.
    c32_not_checked: "Not checked",
    /// Strings of the set whose index is not known. `{}`: how many.
    c32_more_unknown: "And {} more, their index not known",
    /// The strings typed back rebuild this key.
    c32_rebuilds: "These strings rebuild this key",
    /// The strings typed back rebuild another key.
    c32_rebuilds_not: "These strings rebuild another key: look at each again",
    /// The set's actions' heading.
    c32_the_set: "The set",
    /// Add a share.
    c32_add_a_share: "Add a share",
    /// Rotate the set.
    c32_rotate: "Rotate the set",
    /// Replace a lost share. `{}`: its index.
    c32_replace: "Replace {}",
    /// A string open. `{}`: its index, its places.
    c32_share_at: "Share {} · {}",
    /// A string of a set made before, typed from its paper.
    c32_type_from_paper: "Type it from the paper",
    /// Back to the item's strings.
    c32_all_strings: "All strings",
    /// Replace a lost share, its title. `{}`: the index, the identifier.
    c32_replacing: "Replace {} of {}",
    /// Add a share, its title. `{}`: the index, the identifier.
    c32_adding: "Add {} to {}",
    /// Strings typed in so far. `{}`: how many, how many needed, the identifier.
    c32_typed_in: "{} of {} strings of {} typed",
    /// Make the share. `{}`: its index.
    c32_make_share: "Make share {}",
    /// Rotate the set, its title. `{}`: the identifier.
    c32_rotate_title: "Rotate {}",
    /// Rotate the set: each old string marked.
    c32_rotate_mark: "Mark each string held, lost or exposed",
    /// A string held.
    c32_held: "Held",
    /// A string lost.
    c32_lost: "Lost",
    /// A string exposed.
    c32_exposed: "Exposed",
    /// A held string confirmed destroyed. `{}`: its index.
    c32_destroyed: "{} destroyed",
    /// The new set, its title. `{}`: the old identifier.
    c32_new_set_of: "A new set in place of {}",
    /// The new set's identifier.
    c32_new_identifier: "New identifier",
    /// Make the new set.
    c32_make_new_set: "Make the new set",
    /// Leave a set action.
    c32_leave: "Leave",
    /// A codex32 seed's group head on the board. `{}`: the seed, the identifier, the split.
    c32_board_head_id: "{} · codex32 {} · {}",
    /// A string's chip. `{}`: the seed, the index.
    c32_board_chip: "{} · {}",
    /// Check: a string kept nowhere. `{}`: the index, the seed.
    c32_warn_nowhere: "Share {} of {} is kept nowhere",
    /// Check: a place that keeps enough strings. `{}`: the place, how many, the set.
    c32_warn_place_seed: "{} keeps {} shares of {}: the seed",
    /// Check: a place whose loss leaves too few strings. `{}`: the place, the threshold, the set.
    c32_warn_lose_place: "Losing {} leaves fewer than {} shares of {}",
    /// Check: a vault with a seed and a string of it. `{}`: the vault, the seed.
    c32_warn_vault_both: "Vault {} holds {} and a share of it",
    /// Check: a set's seed whole in a vault. `{}`: the vault, the seed.
    c32_warn_vault_whole: "Vault {} holds {} whole: one place with its passphrase is enough",
    /// Check: a string as a file in the clear. `{}`: the set.
    c32_warn_file: "A share of {} is a file that is not encrypted; enough such files are the seed",
    /// Templates item: the codex32 sheets, after the words'. `{}`: how many, the length.
    c32_templates_clause: "{} from Blank templates, codex32, {} characters",
    /// Templates item: the codex32 sheets alone. `{}`: how many, the length.
    c32_templates_starter: "Print {} from Faraday Starter: Blank templates, codex32, {} characters",
    /// Templates item: codex32 sheets with a Codex32QR, after the words'. `{}`: how many, the length, the Starter's Copy choice ("Codex32QR", "Both").
    c32_templates_clause_copy: "{} from Blank templates, codex32, {} characters, {}",
    /// Templates item: codex32 sheets with a Codex32QR, alone. `{}`: how many, the length, the Starter's Copy choice.
    c32_templates_starter_copy: "Print {} from Faraday Starter: Blank templates, codex32, {} characters, {}",
    // ----- A string's Codex32QR drawn by hand (docs/CODEX32.md §9.5, §10.2), faraday-core -----
    /// The sheet a string's Codex32QR is drawn on.
    c32_qr_sheet: "Sheet · Faraday Starter, Blank templates, codex32, 48 characters, Codex32QR",
    // ----- A QR copied by hand, SeedQR or Codex32QR (docs/BACKUP-FLOW.md §17.24), faraday-core -----
    /// The copy grid's choice: marked block by block.
    copy_grid_blocks: "Blocks",
    /// The copy grid's choice: marked row by row.
    copy_grid_rows: "Rows",
    /// The block marked. `{}`: its name, "A1" on a Codex32QR, "5 · 10" (first row · first column) on a SeedQR.
    copy_grid_block: "Block {}",
    /// Button: the block (or row) before.
    copy_grid_prev: "Previous",
    /// Button: the next block (or row).
    copy_grid_next: "Next",
    /// Key under the copy grid: the squares the blank sheet prints.
    copy_grid_printed: "Grey · printed on the sheet",
    /// Key under the copy grid: the squares to draw.
    copy_grid_draw: "Black · draw",
    /// Button: the camera on the Codex32QR drawn.
    c32_scan_my_copy: "Scan my copy",
    /// The camera read something that is not a codex32 string of the set's length.
    c32_scanned_not_string: "Not a codex32 string",
    /// Templates item's line with only codex32 sheets. `{}`: the length.
    c32_templates_sub: "Codex32, {} characters",
    // ----- Codex32 strings written, gathered and counted (docs/CODEX32.md §9.1, §9.5), faraday-core -----
    /// A vault saved a codex32 share record. `{}`: the index, the key's fingerprint.
    c32_vault_saved: "Share {} of {} saved",
    /// A string the plan keeps in a vault or a file that this session does not hold. `{}`: the index, the identifier.
    c32_vault_missing: "Share {} of {} is not here: type it in its Codex32 strings item first",
    /// A save leaves a codex32 share the plan keeps in another vault: nothing else holds the string. `{}`: the index, the identifier.
    c32_vault_left: "Share {} of {} stays in this vault: nothing else holds it",
    // ----- A codex32 string typed in the round after a lock (docs/CODEX32.md §9.5), faraday-core -----
    /// Make card of the round: a string a vault takes that is not here. `{}`: the index, the identifier.
    c32_round_not_here: "Share {} of {} is not here",
    /// Button under it: type that string on Add a key's codex32 form. `{}`: the index.
    c32_round_type: "Type share {} from its copy",
    /// Add a key's heading while it takes that string. `{}`: the index, the identifier.
    c32_round_heading: "Type share {} of {}",
    /// A string typed of another set. `{}`: what it is (`codex32_form_checked_share`), the identifier wanted.
    c32_round_other_set: "{} is not of set {}",
    /// A string typed of the set at another index. `{}`: what it is, the index and identifier wanted.
    c32_round_other_share: "{} is not share {} of {}",
    /// A string typed of the set's header whose seed is another. `{}`: the index, the identifier, the key's fingerprint.
    c32_round_other_seed: "Share {} of {} is of another seed than {}",
    /// A string of the set at the index wanted that cannot be proved the key's. `{}`: the index, the identifier.
    c32_round_unproved: "Share {} of {} cannot be checked against the key with the strings here",
    /// The key the string is of is not loaded. `{}`: its fingerprint.
    c32_round_no_key: "Key {} is not here: add it from its copy first",
    /// After `wallet_not_a_valid`: the strings typed and an open vault's
    /// strings of the same identifier make no key the vault's records
    /// name; nothing loads.
    c32_vault_other_seed: "the open vaults' strings of this set are of another seed",
    /// A vault's share record, its row. `{}`: the index, the key's fingerprint.
    c32_vault_row: "Share {} of {}",
    /// The share record's line. `{}`: the identifier, the threshold.
    c32_vault_row_line: "codex32 {} · threshold {}",
    /// Keys loaded at unlock from strings gathered across open vaults. `{}`: the fingerprints.
    c32_gathered: "Loaded {} from codex32 shares",
    /// A vault's line on the backup checklist: the shares of a set it is to keep. `{}`: how many, the threshold, the seed.
    c32_shares_of: "{} of {} shares of {}",
    /// A place's line: shares of a set it keeps, too few. `{}`: how many, the threshold, the seed.
    c32_place_nothing: "{} of {} shares of {} · alone: nothing",
    /// A place's line: enough shares of a set, the seed. `{}`: how many, the threshold, the seed.
    c32_place_seed: "{} of {} shares of {}: the seed",
    /// A place's line: enough shares with a vault's passphrase. `{}`: how many, the threshold, the seed.
    c32_place_vault: "{} of {} shares of {}: the seed, with a vault's passphrase",
    /// A codex32 string's picture file's title. `{}`: the index, the identifier.
    c32_picture_title: "Codex32 share {} of {}",
    /// The picture's line under its title.
    c32_picture_line: "Secret: a share of a seed",
    // ----- Codex32: Make the rest, Add a share, Check a string's worksheet, Practice (docs/CODEX32.md §8.1 to §8.4, §12), faraday-core -----
    /// The codex32 form's button: the set's other strings from the k in.
    c32r_make_the_rest: "Make the rest",
    /// The codex32 form's button: one more string of the set.
    c32r_add_a_share: "Add a share",
    /// The codex32 form: strings of the set typed in that open vaults
    /// keep, which count towards k (§9.5). `{}`: how many.
    c32r_in_vaults: "{} more of this set in open vaults",
    /// The plan's codex32 item for a set that exists (§9.4). `{}`: the
    /// identifier.
    c32r_how_many: "How many strings of {} are there?",
    /// Make codex32 shares' Entropy card: shares made by hand (§5.3).
    c32r_type_shares_by_hand: "Type shares made by hand",
    /// New key's Start card closed on strings typed in going on into
    /// Make the rest or Add a share. `{}`: "Add a key you have", the way.
    c32r_start_typed: "{} · {}",
    /// Add a share's first card, and its index row.
    c32r_index: "Index",
    /// Add a share's first card closed. `{}`: the index.
    c32r_index_value: "Index · {}",
    /// Under Add a share's index.
    c32r_give_index: "Give it an index no other string of the set has",
    /// Add a share's index is one typed in, or the secret's.
    c32r_index_taken: "A string typed in has this index",
    /// A set typed in: the strings it is made from.
    c32r_from: "From",
    /// Add a share's split: the set's threshold alone. `{}`: k.
    c32r_k_needed: "{} needed",
    /// Strings made's entropy for a set typed in.
    c32r_your_shares: "your shares",
    /// Practice: the line on every card after the entry. `{}`: the sample's
    /// identifier.
    c32r_demo_sample: "Practice puts sample {} in place of these strings",
    /// Practice: Strings made's entropy. `{}`: the sample's identifier.
    c32r_entropy_sample: "sample {}",
    /// Practice: a checksum worksheet's marking for the sample.
    c32r_the_sample: "the sample",
    /// Practice with no sample to put in place of the strings entered.
    c32r_demo_no_sample: "Practice has no sample set to show in place of these strings",
    /// Check a string: the final row's first cell that is not SECRETSHARE32's. `{}`: the box.
    c32r_final_differs: "Final row differs at box {}",
    /// Check a string: every row of its worksheet.
    c32r_show_worksheet: "Show the worksheet",
    /// Check a string: back to the final row alone.
    c32r_final_row_only: "Final row only",
    /// Wallets: beside a key loaded from corrected strings. `{}`: the
    /// fingerprint, the line.
    c32r_key_mark: "{} · {}",
        }
    };
}
