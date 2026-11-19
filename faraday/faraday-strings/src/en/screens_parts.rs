//! Faraday's English words for the screens parts area.

macro_rules! words_screens_parts {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- ticked (faraday-core/src/screens/parts.rs) -----
    /// Text in `ticked`.
    ticked_none: "None",
    // ----- kind_rows (faraday-core/src/screens/parts.rs) -----
    /// Button in `kind_rows`.
    kind_rows_more_kinds: "More kinds",
    // ----- first_addresses (faraday-core/src/screens/parts.rs) -----
    /// Section label in `first_addresses`.
    first_addresses_heading: "The wallet's first addresses",
    /// Label in `first_addresses`, over the addresses, so that "It matches"
    /// answers it. `{}`: the plan's wallet software.
    first_addresses_compare: "Same in {}?",
    /// In `first_addresses_compare`, where the plan names no software.
    first_addresses_your_software: "your wallet software",
    /// Text in `first_addresses`.
    first_addresses_receive_0_0: "Receive 0/0",
    /// Text in `first_addresses`.
    first_addresses_receive_0_1: "Receive 0/1",
    /// Text in `first_addresses`.
    first_addresses_change: "Change 1/0",
    // ----- made_place (faraday-core/src/screens/parts.rs) -----
    /// Text in `made_place`. `{}`: `r.label`.
    made_place_written: "Written to {}",
    // ----- made_state (faraday-core/src/screens/parts.rs) -----
    /// Text in `made_state`.
    made_state_remove: "Remove",
    /// Text in `made_state`.
    made_state_make_again: "Make again",
    // ----- locked_tag (faraday-core/src/screens/parts.rs) -----
    /// Text in `locked_tag`.
    locked_tag_text: "Locked in",
    // ----- descriptor_row (faraday-core/src/screens/parts.rs) -----
    /// Line in `descriptor_row`.
    descriptor_row_descriptor: "Descriptor",
    // ----- kind_line (faraday-core/src/screens/parts.rs) -----
    /// Text in `kind_line`. `{}`: `bytes as f32 / 1e6`.
    kind_line_mb: "{} MB",
    /// A size in gigabytes, in `size_text`. `{}`: the number.
    kind_line_gb: "{} GB",
    /// Text in `kind_line`. `{}`: `bytes as f32 / 1e3`.
    kind_line_kb: "{} KB",
    // ----- kind_name (faraday-core/src/screens/parts.rs) -----
    /// Text in `kind_name`.
    kind_name_wallet_descriptor: "Wallet descriptor",
    /// Text in `kind_name`.
    kind_name_finished: "Finished transaction",
    /// Text in `kind_name`.
    kind_name_print_sheet: "Sheet for printing",
    /// Text in `kind_name`.
    kind_name_signed_message: "Signed message",
    /// Text in `kind_name`.
    kind_name_account_xpub: "Account xpub",
    /// Text in `kind_name`.
    kind_name_seed_words: "Seed words",
    /// Text in `kind_name`.
    kind_name_split_share: "Share of a split backup",
    /// Text in `kind_name`.
    kind_name_vault_locked: "Vault, locked",
    /// Text in `kind_name`.
    kind_name_vault_entries: "Entries for a vault",
    /// Text in `kind_name`.
    kind_name_keepass_database: "A KeePass file of an older version",
    /// Text in `kind_name`.
    kind_name_keepass_file: "A KeePass file",
    /// Text in `kind_name`.
    kind_name_pdf_to_print: "PDF to print",
    /// Text in `kind_name`.
    kind_name_efi_image: "EFI image",
    /// Text in `kind_name`.
    kind_name_seed_part: "Part of a seed: SLIP-39 or codex32",
    /// Text in `kind_name`.
    kind_name_text: "Text",
    /// Text in `kind_name`.
    kind_name_gpg: "GPG",
    /// A QR vault in `kind_name`.
    kind_name_qr_vault: "QR vault",
    /// A file's kind in Files: a SQLite database, which may be a Bitcoin Core wallet.
    kind_name_sqlite: "SQLite database",
    /// Text in `kind_name`: another wallet's file that holds a private
    /// key or a seed beside its public keys.
    kind_name_secret_wallet: "Wallet file with a private key",
    /// Text in `kind_name`: an account key file with a seed or a private
    /// key beside the key.
    kind_name_secret_key: "Key file with a secret",
    /// Text in `kind_name`.
    kind_name_file: "File",
    // ----- word_keys (faraday-core/src/screens/parts.rs) -----
    /// Button in `word_keys`.
    word_keys_shift: "Shift",
    // ----- tz_row (faraday-core/src/screens/parts.rs) -----
    /// Line in `tz_row`.
    tz_row_time_zone: "Time zone",
    // ----- nothing_to_work_on (faraday-core/src/screens/parts.rs) -----
    /// Button in `nothing_to_work_on`: opens Files.
    parts_open_files: "Open Files",
    /// Button in `nothing_to_work_on`, with the stick attached. `{}`: the stick's name.
    parts_bring_in_from: "Bring in from {}",
        }
    };
}
