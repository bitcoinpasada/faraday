//! Faraday's English words for the compact area.

macro_rules! words_compact {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- compact (faraday-core/src/compact/mod.rs) -----
    /// Text in `screen_name`.
    compact_spend: "Spend from a backup",
    /// Text in `screen_name`.
    compact_add_a_key: "Add a key",
    /// Text in `screen_name`.
    compact_back_up: "Back up",
    /// Text in `screen_name`.
    compact_sign_a_message: "Sign a message",
    /// Text in `screen_name`.
    compact_make_a_new: "Make a new wallet",
    /// Text in `screen_name`.
    compact_restore_wallet: "Restore an existing wallet",
    /// Text in `screen_name`.
    compact_set_the_time: "Set the time",
    /// Text in `screen_name`.
    compact_totp_codes: "TOTP codes",
    /// Text in `screen_name`.
    compact_create_a_vault: "Make a new vault",
    /// Text in `screen_name`.
    compact_new_key: "Make a new key",
    /// Text in `screen_name`.
    compact_silent_payments: "Silent payments",
    /// Text in `screen_name`.
    compact_explore_a_key: "Explore a key",
    /// Text in `screen_name`.
    compact_lightning: "Lightning",
    /// Text in `screen_name`.
    compact_vanity_address: "Vanity address",
    /// Title in `compact_screen_name`.
    compact_invoice: "Invoice",
    /// Text in `page`.
    compact_at_a_glance: "At a glance",
    /// Text in `page`.
    compact_vault_contents: "Vault contents",
    /// Text in `page`.
    compact_home: "Home",
    /// Text in `home`. `{}`: `v`, `if v == 1 { EN.compact_vault } else {…`.
    compact_vaults_open: "{} {} open",
    /// Text in `home`.
    compact_vault: "vault",
    /// Text in `home`.
    compact_vaults_inline: "vaults",
    /// Button in `home`.
    compact_lock: "Lock",
    /// Button in `home`.
    compact_scan: "Scan",
    /// Text in `home`.
    compact_key_inline: "key",
    /// Text in `home`.
    compact_keys: "keys",
    /// Text in `tiles`.
    compact_wallets: "Wallets",
    /// Text in `tiles`.
    compact_vaults: "Vaults",
    /// Text in `tiles`. `{}`: `n`.
    compact_locked: "{} locked",
    /// Text in `tiles`. `{}`: `o`.
    compact_open: "{} open",
    /// Text in `tiles`.
    compact_files: "Files",
    /// Text in `tiles`.
    compact_guide: "Guide",
    /// Text in `tiles`.
    compact_backups: "Backups",
    /// Text in `tiles`. `{}`: `without`.
    compact_not_backed: "{} not backed up",
    /// Check's name in the bar and on the Pi's Home menu.
    compact_check: "Check",
    /// Text in `tiles`.
    compact_tools: "Tools",
    /// Text in `tiles`.
    compact_transfer: "Transfer",
    /// Text in `tiles`.
    compact_settings: "Settings",
    /// Text in `keyboard`.
    compact_done: "Done",
    /// Text in `home_files_line`: the count, one file.
    compact_file_count_one: "file",
    /// Text in `home_files_line`: the count, other than one.
    compact_file_count_many: "files",
    /// Text in `home_files_line`. `{}`: `self.medium.noun()`.
    compact_no_medium: "no {}",
    // ----- compact (faraday-core/src/compact/screen.rs) -----
    /// Line in `wallet_card`.
    compact_no_wallet_loaded: "No wallet loaded",
    /// Text in `wallet_card`. `{}`: `s.spend.source`.
    compact_continue_signing: "Continue signing {}",
    /// Text in `wallet_card`. `{}`: `app.inbox[i].name`.
    compact_review: "Review {}",
    /// Button in `wallet_card`.
    compact_rename: "Rename",
    /// Text in `wallet_card`. `{}`: `here`, `m`.
    compact_of_signatures: "{} of {} signatures here",
    /// Chip in `wallet_card`.
    compact_can_spend: "Can spend",
    /// Button in `wallet_card`.
    compact_scan_a_key: "Scan a key",
    /// Text in `wallet_card`. `{}`: `nodes.len()`, `if nodes.len() == 1 { EN.compact_plac…`.
    compact_backup: "{} backup {}",
    /// Text in `wallet_card`.
    compact_place: "place",
    /// Text in `wallet_card`.
    compact_places: "places",
    /// Text in `wallet_card`. `{}`: `vault`.
    compact_plan_in_locked: "plan in {}, locked",
    /// Text in `wallet_card`.
    compact_no_backup_plan: "no backup plan",
    /// Heading in `wallet_card`.
    compact_first_receive: "First receive address · 0/0",
    /// Heading in `silent_card`.
    compact_key: "Key",
    /// Text in `silent_card`.
    compact_here: "Here",
    /// Text in `silent_card`.
    compact_not_here: "Not here",
    /// Text in `silent_card`.
    compact_load_from_vault: "Load from vault",
    /// Text in `silent_card`.
    compact_add_its_key: "Add its key",
    /// Heading in `silent_card`.
    compact_address: "Address",
    /// Line in `silent_card`.
    compact_labels_handed: "Labels handed out",
    /// Text in `silent_card`.
    compact_address_labels: "Address, labels and payments",
    /// Text in `silent_card`.
    compact_show_wallet: "Show wallet QR",
    /// Text in `silent_card`.
    compact_remove_from_session: "Remove from session",
    /// Text in `about`.
    compact_about_page: "About this page",
    /// Text in `entry`. `{}`: `wl.name`.
    compact_entry_for_wallet: "For {}",
    /// Text in `entry`. `{}`: `fp_text(faraday_bip::keys::Fingerprint(fp))`.
    compact_for_key: "For key {}",
    /// Text in `entry`.
    compact_other_languages: "Other languages",
    /// Text in `entry`. `{}`: `crate::forms::language_name(lang)`.
    compact_other_lists: "{} · other lists",
    /// Text in `entry`.
    compact_other_forms: "Other forms",
    /// Button in `entry`.
    compact_scan_a_seedqr: "Scan a SeedQR",
    /// Text in `entry`. `{}`: `taken.len() + 1`.
    compact_word: "Word {}",
    /// Heading in `entry`.
    compact_words: "Words",
    /// Label in `entry`.
    compact_bip_39_passphrase: "BIP-39 passphrase",
    /// Text in `entry`.
    compact_add_key: "Add a key",
    /// Text in `entry`.
    compact_make_a_new_key: "Make a new key",
    /// Text in `entry`.
    compact_clear: "Clear",
    /// Text in `entry_form`.
    compact_words_of_one: "Words of one share",
    /// Text in `entry_form`.
    compact_nothing_in_yet: "Nothing in yet",
    /// Text in `entry_form`.
    compact_count_one: "1 in",
    /// Text in `entry_form`. `{}`: `n`.
    compact_count_many: "{} in",
    /// Label in `entry_form`.
    compact_passphrase: "Passphrase",
    /// Text in `entry_form`.
    compact_add_this_one: "Add this one",
    /// Text in `entry_form`.
    compact_scan_a_qr_code: "Scan a QR code",
    /// Text in `entry_form`.
    compact_make_the_key: "Make the key",
        }
    };
}
