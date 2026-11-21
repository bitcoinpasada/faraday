//! Faraday's English words for the glance area.

macro_rules! words_glance {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- glance (faraday-core/src/glance/mod.rs) -----
    /// Text in `RECEIVE_WARNING`.
    glance_back_up_before: "Back up before you receive",
    /// Text in `WAITING`.
    glance_waiting_backup: "Waiting for its backup",
    /// Text in `build`.
    glance_no_origin: "no origin",
    /// Text in `build`.
    glance_can_sign_passphrase: "Can sign · passphrase",
    /// Text in `build`.
    glance_can_sign_here: "Can sign here",
    /// Text in `build`.
    glance_in_this_vault_passphrase: "In this vault · passphrase",
    /// Text in `build`.
    glance_in_this_vault: "In this vault",
    /// Text in `build`.
    glance_cosigner_xpub: "Cosigner · xpub only",
    /// Text in `build`.
    glance_load_from_vault: "Load from vault",
    /// Text in `backup_nodes`. `{}`: `v + 1`.
    glance_vault: "Vault {}",
    /// Text in `backup_nodes`. `{}`: `p + 1`.
    glance_place: "Place {}",
    /// Text in `backup_nodes`. `{}`: `j + 1`, `shape.keys`.
    glance_share_count: "Share {} of {}",
    /// Text in `backup_nodes`. `{}`: `vault_name(v)`.
    glance_stick: "{} stick",
    /// Text in `backup_nodes`.
    glance_description: "Descriptor",
    /// Text in `backup_nodes`. `{}`: `number(i)`.
    glance_key_file: "Key {} file",
    /// Text in `backup_nodes`. `{}`: `inside` joined by ", ".
    glance_in_the_vault: "{} in the vault",
    /// Text in `backup_nodes`. `{}`: `vault_name(v)`.
    glance_copy: "{} · copy",
    /// Text in `backup_nodes`. `{}`: `app.medium.cap()`.
    glance_of_files: "{} of files",
    /// Text in `backup_nodes`.
    glance_watch_only_software: "Watch-only software",
    /// Text in `backup_nodes`.
    glance_on_its_own_device: "On its own device",
    /// Text in `backup_nodes`.
    glance_its_own_stick: "Its own stick",
    /// Text in `backup_nodes`. `{}`: `at` joined by ", ".
    glance_stick_places: "Stick at {}",
    /// Text in `backup_nodes`.
    glance_not_here: "Not here",
    /// Text in `checked_text`.
    glance_checked: "Checked · no date",
    /// Text in `checked_text`. `{}`: `d`.
    glance_checked_n: "Checked {}",
    /// Text in `age_text`.
    glance_age_today: "today",
    /// Text in `age_text`.
    glance_age_yesterday: "yesterday",
    /// Text in `age_text`. `{}`: the days.
    glance_age_days: "{} days ago",
    /// Text in `age_text`.
    glance_age_month: "1 month ago",
    /// Text in `age_text`. `{}`: the months.
    glance_age_months: "{} months ago",
    /// Text in `age_text`. `{}`: the years.
    glance_age_years: "{} years ago",
    /// Text in `NodeAge::line`. `{}`: an age or a date.
    glance_last_checked_n: "Last checked {}",
    /// Text in `NodeAge::line`.
    glance_never_checked: "Never checked",
    /// Text in `rows`, at a node's right when a copy there is due.
    glance_to_check: "To check",
    /// Text in `due_line`. `{}`: the interval, `glance_interval_months`, `glance_interval_day` or `glance_interval_days`.
    glance_every_checked: "Every place checked in the last {}",
    /// Text in `Interval::text`.
    glance_interval_months: "12 months",
    /// Text in `Interval::text`, for a margin of 1 day.
    glance_interval_day: "1 day",
    /// Text in `Interval::text`. `{}`: the days.
    glance_interval_days: "{} days",
    /// Text in `due_line`. `{}`: the places that are due.
    glance_to_check_places: "To check: {}",
    /// Text in `due_line`, with the clock not checked.
    glance_set_time_to_see: "Set the time to see what is due",
    /// Button in `due_line`.
    glance_set_the_time: "Set the time",
    /// Text in `view`: a date and its age. `{}`: the date, the age.
    glance_sheet_date_age: "{} · {}",
    /// Text in `view`: a line's date and age with the line due. `{}`: the date and age.
    glance_sheet_due: "{} · To check",
    /// Text in `alone_color`.
    glance_nothing: "Nothing",
    /// Text in `alone_color`.
    glance_sealed: "Sealed",
    /// Text. `{}`: `k.number`, `k.fingerprint`.
    glance_key_fingerprint: "Key {} · {}",
    /// Text. `{}`: `vault`.
    glance_plan_in_locked: "Plan in {}, locked",
    /// Text in `rows`.
    glance_unlock: "Unlock",
    /// Text in `rows`.
    glance_no_backup_plan_short: "No backup plan",
    /// Text in `rows`.
    glance_no_backup_plan: "No backup plan in this vault",
    /// Text in `draw_column_with`.
    glance_backup: "Backup",
    /// Text in `draw_column_with`.
    glance_keys: "Keys",
    /// Text in `draw_column_with`.
    glance_wallet: "Wallet",
    /// Text in `thing_label`. `{}`: `number(i)`.
    glance_key_words: "Key {} words",
    /// Text in `thing_label`. `{}`: `number(i)`.
    glance_key_seedqr: "Key {} SeedQR",
    /// Text in `thing_label`. `{}`: `number(i)`.
    glance_passphrase: "Passphrase {}",
    /// Text in `thing_label`.
    glance_wallet_sheet: "Wallet sheet",
    /// Text in `thing_label`. `{}`: `j + 1`.
    glance_share: "Share {}",
    /// Text in `thing_label`. `{}`: `v + 1`.
    glance_vault_copy: "Vault {} · copy",
    /// Text in `thing_label`. `{}`: `number(i)`.
    glance_key_seed: "Key {} seed",
    /// Text in `thing_label`. `{}`: `number(i)`.
    glance_key_seed_passphrase: "Key {} seed + passphrase",
    /// Text in `thing_label`.
    glance_the_description: "The descriptor",
    /// Text in `thing_label`. `{}`: `number(i)`.
    glance_key: "Key {}",
    // ----- glance_sheet (faraday-core/src/glance/sheet.rs) -----
    /// Row in `view`.
    glance_sheet_open_vault: "Open the vault",
    /// Row in `view`.
    glance_sheet_open_wallets: "Open in Wallets",
    /// Row in `view`.
    glance_sheet_check_addresses: "Check addresses",
    /// Row in `view`.
    glance_sheet_the_first: "The first three",
    /// Row in `view`.
    glance_sheet_public_files: "Public files",
    /// Row in `view`.
    glance_sheet_for_the_software: "For the software",
    /// Text in `view`.
    glance_sheet_reprint_wallet_sheet: "Print the wallet sheet again",
    /// Row in `view`.
    glance_sheet_rename_wallet: "Rename the wallet",
    /// Text in `view`.
    glance_sheet_shape: "Shape",
    /// Text in `view`.
    glance_sheet_checksum: "Checksum",
    /// Text in `view`.
    glance_sheet_where_label: "Where",
    /// Text in `view`.
    glance_sheet_label: "Label",
    /// Text in `view`.
    glance_sheet_last_checked_label: "Last checked",
    /// Text in `view`.
    glance_sheet_its_seed: "Its seed is at",
    /// Row in `view`.
    glance_sheet_show_xpub: "Show xpub QR",
    /// Row in `view`.
    glance_sheet_xpub_file: "Xpub file",
    /// Row in `view`.
    glance_sheet_explore: "Explore this key",
    /// Row in `view`.
    glance_sheet_where: "Where it is",
    /// Row in `view`.
    glance_sheet_seed_xor: "Seed XOR or codex32",
    /// Row in `view`.
    glance_sheet_by_hand: "By hand",
    /// Text in `view`. `{}`: `n.name`.
    glance_sheet_into: "Into {}",
    /// Row in `view`.
    glance_sheet_as_a_file: "As a file",
    /// Text in `view`.
    glance_sheet_make_another_backup: "Make another backup of this seed",
    /// Text in `view`.
    glance_sheet_check_seed_copy: "Check a copy of this seed",
    /// Text in `view`.
    glance_sheet_its_passphrase: "Its passphrase",
    /// Row in `view`.
    glance_sheet_show_key_xpub: "Show its xpub",
    /// Text in `view`.
    glance_sheet_name_holder: "Name its holder",
    /// Text in `view`.
    glance_sheet_stick: "Stick",
    /// Text in `view`.
    glance_sheet_found: "Found alone",
    /// Row in `view`.
    glance_sheet_open_vault_to_keep: "Open a vault to keep this",
    /// Text in `view`.
    glance_sheet_mark_checked: "Mark checked",
    /// Row in `view`.
    glance_sheet_what_goes: "What goes in its envelope",
    /// Row in `view`.
    glance_sheet_the_envelope: "The envelope list",
    /// Row in `view`.
    glance_sheet_envelope_pdf: "PDF, for the stick",
    /// Text in `view`.
    glance_sheet_mark_lost: "Mark lost",
    /// Text in `view`.
    glance_sheet_mark_exposed: "Mark exposed",
    /// Row in `view`.
    glance_sheet_change_in_plan: "Change in the plan",
    /// Text in `view`.
    glance_sheet_rename: "Rename",
    /// Row in `view`.
    glance_sheet_save_seeds: "Save its seeds into it",
    /// Row in `view`.
    glance_sheet_write_the_seed: "Write the seed files again",
    /// Row in `view`.
    glance_sheet_rewrite_public: "Write the public files again",
    /// Row in `view`.
    glance_sheet_reshow_descriptor_qr: "Show the descriptor QR again",
    /// Row in `view`.
    glance_sheet_public: "Public files for this software",
    /// Text in `view`.
    glance_sheet_contents_group: "In it",
    /// Text in `view`.
    glance_sheet_kept_at_label: "Kept at",
    /// Text in `view`.
    glance_sheet_seen_here: "Seen here",
    /// Text in `view`.
    glance_sheet_last_checked: "Last checked here",
    /// Text in `NodeAge::line` and `view`: a stick read from an older copy of its vault, in place of a date.
    glance_older_copy: "Older copy",
    /// Text in `view`.
    glance_sheet_marked: "Marked",
    /// Row in `view`.
    glance_sheet_load_this_key: "Load this key",
    /// Row in `view`.
    glance_sheet_scan_my: "Scan my copy",
    /// Row in `view`.
    glance_sheet_type_my: "Type my copy",
    /// Row in `view`.
    glance_sheet_check_copy: "Check this copy",
    /// Row in `view`.
    glance_sheet_rewrite: "Write it again",
    /// Row in `view`.
    glance_sheet_stick_detail: "For the stick",
    /// A kept old Seed XOR part's line on the chart. `{}`: the seed, the chip's text.
    glance_xor_old_part: "Seed {}: {}",
    /// Row in `view`: a kept old Seed XOR part, taken off the plan.
    glance_sheet_destroyed_it: "I destroyed it",
    /// Text in `view`: under a kept old part's row.
    glance_sheet_destroyed_detail: "Off the plan",
    /// Row in `view`.
    glance_sheet_check_row: "Check it",
    /// Row in `view`.
    glance_sheet_by_derived_key: "By the key it makes",
    /// Row in `view`.
    glance_sheet_reprint: "Print it again",
    /// Row in `view`.
    glance_sheet_as_a_qr: "As a QR picture",
    /// Row in `view`.
    glance_sheet_as_text: "As text",
    /// Row in `view`.
    glance_sheet_print: "Print the shares again",
    /// Row in `view`.
    glance_sheet_pdfs_text: "PDFs, text and pictures",
    /// Row in `view`.
    glance_sheet_keys_left: "Keys left off each share",
    /// Row in `view`.
    glance_sheet_write: "Write it out again",
    /// Row in `view`.
    glance_sheet_changed: "Changed since written",
    /// Row in `view`.
    glance_sheet_show_wallet: "Show wallet QR",
    /// Row in `view`.
    glance_sheet_the_descriptor: "The descriptor",
    /// Text in `view`.
    glance_sheet_lost: "Lost",
    /// Text in `view`.
    glance_sheet_exposed: "Exposed",
    /// A line's mark on its sheet: a SeedQR dropped for the words.
    glance_sheet_drawn_in_part: "Drawn in part",
    /// Text in `view`.
    glance_sheet_someone_can: "Someone can now spend",
    /// Text in `view`.
    glance_sheet_someone_could: "Someone could spend with a vault's passphrase",
    /// Row in `view`.
    glance_sheet_move_money: "Move the money",
    /// Row in `view`.
    glance_sheet_new_wallet: "New wallet",
    /// Text in `view`.
    glance_sheet_the_wallet: "The wallet can no longer be rebuilt from what is left",
    /// Row in `view`.
    glance_sheet_back_up: "Back up again now",
    /// Text in `view`.
    glance_sheet_someone: "Someone can see the balance",
    /// Text in `view`.
    glance_sheet_nothing_lost: "Nothing lost that is not elsewhere",
    /// Text in `view`.
    glance_sheet_nothing: "Nothing it holds spends or shows the balance",
    /// Text in `view`. `{}`: `key.number`, `key.fingerprint`.
    glance_sheet_key: "Key {} · {}",
    /// Text in `view`.
    glance_sheet_holder: "Holder",
    /// Text in `view`.
    glance_sheet_name: "Name",
    /// Text in `view`.
    glance_sheet_keep: "Keep",
    /// Text in `view`.
    glance_sheet_type_passphrase: "Type it",
    /// Text in `view`.
    glance_sheet_passphrase: "Passphrase",
    /// Text in `view`. `{}`: `key.fingerprint`.
    glance_sheet_matches: "Matches: it makes key {}",
    /// Text in `view`.
    glance_sheet_mismatch: "Does not match",
    /// Text in `view`.
    glance_sheet_check_button: "Check",
    /// Text in `view`. `{}`: `key.number`.
    glance_sheet_passphrase_n: "Passphrase {}",
    /// Text in `view`.
    glance_sheet_change: "Change the plan",
    /// Text in `draw`.
    glance_sheet_back: "Back",
    /// Text in `draw`.
    glance_sheet_close: "Close",
        }
    };
}
