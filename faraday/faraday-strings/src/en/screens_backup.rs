//! Faraday's English words for the screens backup area.

macro_rules! words_screens_backup {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- question_title (faraday-core/src/screens/backup.rs) -----
    /// Text in `question_title`.
    question_title_software: "Software",
    // ----- item_title (faraday-core/src/screens/backup.rs) -----
    /// Text in `item_title`. `{}`: `k`, `if k == 1 { EN.item_title_template_inline …`.
    item_title_print_blank: "Print {} blank {}",
    /// Text in `item_title`.
    item_title_template_inline: "template",
    /// Text in `item_title`.
    item_title_templates: "templates",
    /// Text in `item_title`. `{}`: `b.words`.
    item_title_template_n: "Template for {} words",
    /// Text in `item_title`. `{}`: `shape.seeds.get(i).map_or("", |s| s.n…`.
    item_title_copy_seed: "Copy seed {} by hand",
    /// Text in `item_title`. `{}`: the seed's name.
    item_title_passphrase: "Write the passphrase of seed {} on paper",
    /// Text in `item_title`.
    item_title_checked: "Checked",
    /// Text in `item_title`.
    item_title_not_checked: "Not checked",
    /// Text in `item_title`: a seed's Krux code. `{}`: the seed's name.
    item_title_kef: "Krux code of {}",
    /// Text in `item_title`. `{}`: `v + 1`.
    item_title_vault_number: "Vault {}",
    /// Text in `item_title`. `{}`: `v + 1`, `names` joined by ", ".
    item_title_vault: "Vault {} · {}",
    /// Text in `item_title`. `{}`: `name`.
    item_title_in_vault: "In {}",
    /// Text in `item_title`.
    item_title_not_saved: "Not saved",
    /// Text in `item_title`.
    item_title_the_seeds: "The seeds as files",
    /// Text in `item_title`.
    item_title_not_made: "Not made",
    /// Text in `item_title`.
    item_title_the_shares: "The shares",
    /// Text in `item_title`. `{}`: `keys_n`, `keys_n - a.omit.min(shape.m.saturatin…`.
    item_title_shares_keys: "{} shares, {} keys each",
    /// Text in `item_title`.
    item_title_the_wallet: "The wallet sheet",
    /// Text in `item_title`.
    item_title_print_pdf: "A PDF to print",
    /// Text in `item_title`.
    item_title_the_public: "The public files",
    /// Text in `item_title`.
    item_title_show_descriptor: "Show the descriptor to the software",
    /// Text in `item_title`.
    item_title_shown: "Shown",
    /// Text in `item_title`.
    item_title_not_shown: "Not shown",
    /// Text in `item_title`.
    item_title_one_envelope: "One envelope per place",
    /// Text in `item_title`.
    item_title_envelope: "envelope",
    /// Text in `item_title`.
    item_title_envelopes: "envelopes",
    // ----- question_summary (faraday-core/src/screens/backup.rs) -----
    /// Text in `question_summary`. `{}`: `c.lost.text()`.
    question_summary_any: "Any one place lost: {}",
    // ----- backup (faraday-core/src/screens/backup.rs) -----
    /// Text in `backup_screen`. `{}`: `t`.
    backup_after: "After: {}",
    /// Text in `backup_screen`.
    backup_back_up_a_wallet: "Back up a wallet",
    /// Text in `backup_screen`. `{}`: `wallet.name`.
    backup_back_up: "Back up {}",
    /// Text in `backup_screen`.
    backup_vault_contents: "Vault contents",
    /// Text in `backup_screen`.
    backup_wallets: "Wallets",
    /// Text in `backup_screen`.
    backup_then_wallets: "Then: Wallets",
    /// Text in `backup_screen`.
    backup_change_the_plan: "Change the plan",
    /// Line in `backup_body`.
    backup_seed_words: "Words on the seed",
    /// Line in `backup_body`.
    backup_no_typed_key: "No key of this wallet was loaded from words in this session",
    /// Text in `backup_body`.
    backup_hide_the_seed: "Hide the seed",
    /// Text in `backup_body`.
    backup_show_the_seed: "Show the seed",
    /// Text in `backup_body`, a passphrase item.
    backup_hide_the_passphrase: "Hide the passphrase",
    /// Text in `backup_body`, a passphrase item.
    backup_show_the_passphrase: "Show the passphrase",
    /// Text in `backup_body`, a passphrase item. `{}`: the key's line.
    backup_passphrase_item: "Passphrase of {}",
    /// Line in `backup_body`: the key keeps no passphrase to show.
    backup_no_passphrase_kept: "This key was loaded without its passphrase",
    /// Text in `backup_body`. `{}`: `key_line(key.master.fingerprint(), &k…`, `list.len()`.
    backup_words: "{} · {} words",
    /// Line in `backup_body`.
    backup_seedqr_english_only: "SeedQR is for English words only: copy these words by hand",
    /// Text in `backup_body`.
    backup_standard_seedqr: "Standard SeedQR",
    /// Text in `backup_body`.
    backup_compact_seedqr: "Compact SeedQR",
    /// Text in `backup_body`.
    backup_shown: "Shown",
    /// Text in `backup_body`.
    backup_not_shown: "Not shown",
    /// Text in `backup_body`.
    backup_make_button: "Make it",
    /// Row in `backup_body`.
    backup_backup_sheet: "Backup sheet",
    /// Row in `backup_body`.
    backup_a_pdf_to_print: "A PDF to print",
    /// Text in `backup_body`.
    backup_make_them: "Make them",
    /// Row in `backup_body`.
    backup_the_shares: "The shares",
    /// Row in `backup_body`.
    backup_sheets_to_print: "Sheets to print, text files and pictures",
    /// Text in `backup_body`.
    backup_nothing: "Nothing",
    /// Text in `backup_body`.
    backup_back_to_wallet: "Back to the wallet",
    /// Text in `backup_body`.
    backup_done: "Done",
    // ----- plan (faraday-core/src/screens/backup.rs) -----
    /// Line in `plan_body`.
    plan_a_key_made_here: "A key made here signs only once this backup is done.",
    /// Line on the board and the plan's map for a key loaded here that is kept
    /// as shares outside the plan. `{}`: the key's fingerprint.
    plan_seed_kept_as_shares: "Seed {} was loaded from shares or a master seed: this plan does not count it",
    /// Text in `plan_body`.
    plan_what_is_a_share: "What is a share?",
    /// Section label in `plan_body`.
    plan_form: "Form",
    /// Text in `plan_body`.
    plan_qr_picture: "QR picture",
    /// Text in `plan_body`.
    plan_text: "Text",
    /// Text in `plan_body`. `{}`: `v + 1`, `app.medium.noun()`.
    plan_vault_s_needs: "Vault {}'s {} needs a place",
    /// Button in `plan_body`.
    plan_make_the_checklist: "Make the checklist",
    /// Button in `plan_body`.
    plan_continue: "Continue",
    // ----- place_names (faraday-core/src/screens/backup.rs) -----
    // ----- map_rows (faraday-core/src/screens/backup.rs) -----
    /// Text in `map_rows`. `{}`: `app.medium.cap()`.
    map_rows_of_files: "{} of files",
    /// Text in `map_rows`.
    map_rows_watch_only: "Watch-only software",
    /// Text in `map_rows`.
    map_rows_on_its_own: "On its own device",
    /// Line in `map_rows`.
    map_rows_nothing: "Nothing",
    // ----- wallet_done (faraday-core/src/screens/backup.rs) -----
    /// Title in `wallet_done` after Make a new wallet. `{}`: the wallet's name.
    wallet_done_made: "Wallet made · {}",
    /// Title in `wallet_done` after Back up. `{}`: the wallet's name.
    wallet_done_backup: "Backup done · {}",
    /// Title in `wallet_done` after Restore. `{}`: the wallet's name.
    wallet_done_restored: "Wallet restored · {}",
    /// Value of the Backup line in `wallet_done` before any backup.
    wallet_done_not_made: "not made",
    /// Label of the source line in `wallet_done` after Restore.
    wallet_done_restored_from: "Restored from",
    /// Value of the source line in `wallet_done`: the wallet came from its description.
    wallet_done_from_description: "wallet description",
    /// Value of the source line in `wallet_done`: the wallet was made from the seeds.
    wallet_done_from_seeds: "seeds",
    /// Filled button in `wallet_done` while the wallet has no backup.
    wallet_done_back_up_now: "Back up now",
    /// Text in `wallet_done`.
    backup_done_open_wallet: "Open the wallet",
    /// Text in `wallet_done`.
    backup_done_change: "Change the plan",
    // ----- shares_card (faraday-core/src/screens/backup.rs) -----
    /// Text in `shares_card`.
    shares_card_keys_left: "Keys left off each share",
    /// Text in `shares_card`. `{}`: `k + 1`.
    shares_card_share: "Share {}",
    /// Text in `shares_card`. `{}`: `m`.
    shares_card_any_shares: "Any {} shares rebuild the wallet",
    /// Text in `shares_card`. `{}`: `m`.
    shares_card_some_shares: "Some {} shares do not rebuild the wallet",
    /// Text in `shares_card`. `{}`: `a.smallest_rebuild`.
    shares_card_the_fewest: "The fewest shares that rebuild it: {}",
    /// Text in `shares_card`. `{}`: `a.copies_per_key`.
    shares_card_each_key: "Each key is on {} shares",
    /// Text in `shares_card`.
    shares_card_no_single: "No single share holds every key",
    /// Text in `shares_card`.
    shares_card_each_share: "Each share holds every key",
    // ----- copy_check (faraday-core/src/screens/backup.rs) -----
    /// Label of the typed words in `copy_check`.
    copy_check_your_copys_words: "Your copy's words",
    /// Text in `copy_check`. `{}`: the word being typed, the seed's word count.
    copy_check_word_count: "Word {} of {}",
    /// Text in `copy_check`.
    copy_check_your_copy: "Your copy matches the seed",
    /// Text in `dating`: asked when the plan keeps the form checked at more than one place; a button follows for each.
    copy_check_which_copy: "Which copy was this?",
    /// Button in `dating`, beside a match the clock not checked left with no date.
    copy_check_set_the_time: "Set the time",
    /// Text in `copy_check`. `{}`: `k`.
    copy_check_word_differs: "Word {} differs from the seed",
    /// Button in `copy_check`.
    copy_check_clear: "Clear",
    /// Label of the typed passphrase in `copy_check`.
    copy_check_your_copys_passphrase: "Your copy's passphrase",
    /// Text in `copy_check`, once the words match: what to write on the sheet.
    copy_check_write_these: "Write these on the sheet",
    /// Text in `copy_check`, once the words match a key with a passphrase.
    copy_check_tick_passphrase: "Tick: passphrase kept elsewhere",
    /// Label in `copy_check`.
    copy_check_fingerprint: "Fingerprint",
    /// Label in `copy_check`.
    copy_check_first_address: "First address",
    /// Text in `copy_check`.
    copy_check_passphrase_matches: "Your copy matches the passphrase",
    /// Text in `copy_check`.
    copy_check_passphrase_differs: "Your copy differs from the passphrase",
    /// Button on a SeedQR copy's item: the copy dropped for the words.
    copy_check_keep_the_words: "Keep the words only",
    /// Text on a SeedQR copy's item. `{}`: the count.
    copy_check_scans_without: "{} scans without a match",
    /// Text on a copy's item. `{}`: the seed's fingerprint.
    copy_check_drawn_in_part: "Seed {}'s SeedQR, drawn in part, stays on the map until destroyed",
    // ----- item_action (faraday-core/src/screens/backup.rs) -----
    /// Text in `item_action`.
    item_action_continue: "Continue",
    /// Text in `item_action`.
    item_action_check_my: "Check my copy",
    /// Text in `item_action`: a passphrase typed back, compared.
    item_action_compare: "Compare",
    /// Text in `item_action`.
    item_action_lock_new_vault: "Lock it and make a new vault",
    /// Text in `item_action`. `{}`: `open.name`.
    item_action_save_into: "Save into {}",
    /// Text in `item_action`.
    item_action_make: "Make it",
    /// Text in `item_action`.
    item_action_make_them: "Make them",
    /// Text in `item_action`.
    item_action_show_as: "Show as QR",
    // ----- vault_item (faraday-core/src/screens/backup.rs) -----
    /// Text in `vault_item`. `{}`: `o.name`.
    vault_item_holds_another: "{} holds another vault's seed",
    /// Text in `vault_item`. `{}`: `o.name`.
    vault_item_into: "Into {}",
    /// Text in `vault_item`.
    vault_item_into_vault: "Into the vault",
    /// Text in `vault_item`. `{}`: `name`.
    vault_item_seed_passphrase: "Seed {} with its passphrase",
    /// Text in `vault_item`. `{}`: `name`.
    vault_item_seed: "Seed {}",
    /// Text in `vault_item`.
    vault_item_wallet_description: "Descriptor",
    // ----- seed_files (faraday-core/src/screens/backup.rs) -----
    /// Text in `seed_files`.
    seed_files_not_made: "Not made",
    // ----- paper_section (faraday-core/src/screens/backup.rs) -----
    /// Section label in `paper_section`.
    paper_section_other: "Other paper forms",
    /// Text in `paper_section`.
    paper_section_seed_xor: "Seed XOR, 2 parts",
    /// Text in `paper_section`.
    paper_section_seed: "Seed XOR, 3 parts",
    /// Text in `paper_section`. `{}`: `parts.len()`.
    paper_section_parts: "{} parts · every one is needed",
    /// Button in `paper_section`.
    paper_section_put_them: "Put them away",
    /// Line in `paper_section`. `{}`: the number of parts, the whole key's fingerprint.
    paper_section_part_is: "Each part alone is valid words of another, empty wallet. Write 'Seed XOR part 1 of {}' and the key's fingerprint {} on each.",
    /// Line in `paper_section`.
    paper_section_not_on_plan: "Not on the plan: its checks do not count these parts",
    /// Line in `paper_section`. `{}`: the places the parts are on, by name.
    paper_section_on_plan: "On the plan: {}",
    /// Toast on Seed XOR in `paper_section`: the key's plan already has a set.
    paper_section_has_set: "This key's plan already has a Seed XOR set. Replace it on the plan.",
    /// Line in `paper_section`. `{}`: the word's place, the part's number.
    paper_section_word_differs: "Word {} differs from part {}",
    /// Button in `paper_section`, per part.
    paper_section_check_part: "Check my copy of part {}",
    // ----- seed_grid (faraday-core/src/screens/backup.rs) -----
    /// Text in `seed_grid`. `{}`: `pin`.
    seed_grid_row: "row {}",
    // ----- backup_panel (faraday-core/src/screens/backup.rs) -----
    /// Line in `backup_panel`.
    backup_panel_map: "Map",
    // ----- kept_rows (faraday-core/src/screens/backup.rs) -----
    /// Line in `kept_rows`.
    kept_rows_keys: "Keys",
    // ----- public_files (faraday-core/src/screens/backup.rs) -----
    /// Text in `public_files`.
    public_files_show_as: "Show as QR",
    /// Text in `public_files`.
    public_files_descriptor: "Descriptor",
    /// Text in `public_files`.
    public_files_any_wallet: "Any wallet software",
    /// Text in `public_files`.
    public_files_wallet: "Wallet file",
    /// Text in `public_files`.
    public_files_sparrow: "Sparrow, Specter",
    /// Text in `public_files`.
    public_files_multisig: "Multisig config",
    /// Text in `public_files`.
    public_files_coldcard: "Coldcard, Keystone, Passport",
    /// Text in `public_files`.
    public_files_backup: "Backup sheet",
    /// Text in `public_files`.
    public_files_a_pdf: "A PDF to print",
    /// Text in `public_files`.
    public_files_bsms_descriptor: "BSMS descriptor record",
    /// Text in `public_files`.
    public_files_bip_129: "BIP 129: Coldcard, Sparrow, Nunchuk",
    /// Text in `public_files`.
    public_files_bitcoin: "Core 29 or later",
    /// Text in `public_files`. `{}`: `fp`.
    public_files_key: "Key {}",
    /// Text in `public_files`.
    public_files_cosigners: "For the cosigners",
    /// Text in `public_files`. `{}`: `fp`.
    public_files_key_bsms: "Key {}, BSMS",
    /// Text in `public_files`.
    public_files_signed: "Signed xpub record, BIP 129",
    /// Text in `public_files`: the Caravan config's row.
    public_files_caravan: "Caravan config",
    /// Text in `public_files`: who reads the Caravan config.
    public_files_caravan_unchained: "Caravan, Unchained",
    /// Text in `public_files`: the Wasabi file's row.
    public_files_wasabi: "Wasabi file",
    /// Text in `public_files`: who reads the Wasabi file.
    public_files_wasabi_wallet: "Wasabi Wallet",
    /// Text in `public_files`: the row of Specter DIY's `addwallet` command.
    public_files_specter_diy: "Specter DIY wallet",
    /// Text in `public_files`: who reads Specter DIY's `addwallet` command.
    public_files_specter_diy_reads: "Specter DIY, addwallet",
    /// Text in `public_files`: a silent payments wallet's BIP-392 descriptor row.
    public_files_sp_descriptor: "sp() descriptor",
    /// Text in `public_files`: who reads the `sp()` descriptor.
    public_files_sp_reads: "Sparrow, Keystone · holds the scan key",
    /// Text in `public_files`: the row of Bitcoin Core's import file
    /// for Core before 29.
    public_files_core_before_29: "Core before 29",
    /// Text in `public_files`: who reads either of Bitcoin Core's import
    /// files.
    public_files_bitcoin_core: "Bitcoin Core, watch-only",
    /// Text in `public_files`: the Electrum wallet file's row.
    public_files_electrum: "Electrum wallet file",
    /// Text in `public_files`: who reads the Electrum wallet file.
    public_files_electrum_watch_only: "Electrum, watch-only",
    // ----- public_rows_for (faraday-core/src/screens/backup.rs) -----
    /// Text in `public_rows_for`.
    public_rows_for_make: "Make the file",
    /// Text in `public_rows_for`: the ghost button the files few people
    /// need are under (F64).
    public_rows_for_more_options: "More options",
    // ----- backup flow cards (faraday-core/src/backup_flow/screen.rs) -----
    /// The Lock card's line for the key just made or restored. `{}`: the
    /// key's name; the vaults it is in.
    backup_flow_key_in_vaults: "{} · in {}",
    /// Done's button for a Secure Boot set: it opens the item that holds
    /// the enrolment files. `{}`: the vault.
    backup_flow_unlock_for_enrolment: "Unlock {} for the enrolment files",
    // ----- the board's Seed XOR and SLIP-39 parts (faraday-core/src/board/) -----
    /// A key's Seed XOR group head on the board. `{}`: the seed, how many parts.
    board_xor_head: "{} · Seed XOR · {} parts",
    /// A key's SLIP-39 group head on the board. `{}`: the seed, how many groups are needed, how many groups.
    board_slip39_head: "{} · SLIP-39 · {} of {} groups",
    /// A Seed XOR part's chip. `{}`: the seed, the part's number.
    board_xor_chip: "{} · part {}",
    /// A SLIP-39 share's chip. `{}`: the seed, the group, the share.
    board_slip39_chip: "{} · group {} share {}",
    /// A part's chip under its group. `{}`: the part's number.
    board_xor_part: "Part {}",
    /// A share's chip under its group. `{}`: the group, the share.
    board_slip39_part: "Group {} · share {}",
    /// A part's line at the chip's right. `{}`: how many places keep it, and
    /// that the part was not typed back here.
    board_part_count: "{} · {}",
        }
    };
}
