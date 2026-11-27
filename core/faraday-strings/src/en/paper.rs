//! Faraday's English words for the paper area.

macro_rules! words_paper {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- paper -----
    /// Toast in `paper_xor`.
    paper_no_words_to_split: "This key has no words to split",
    /// Toast in `paper_codex32`. `{}`: `e`.
    paper_codex32: "codex32: {}",
    // ----- pdf (faraday-paper/src/pdf.rs) -----
    /// Error in `fields`.
    pdf_not_a_faraday_sheet: "not a Faraday sheet",
    /// Error in `fields`: a field read once appears twice. `{}`: the field.
    pdf_sheet_names_twice: "this sheet names {} twice",
    /// Text in `sheet`. `{}`: `other`.
    pdf_a_sheet_of_kind: "a sheet of kind {} is not one this build prints",
    /// Text in `wallet_sheet`. `{}`: `name`.
    pdf_wallet_backup: "{} · wallet backup",
    /// Line in `wallet_sheet`.
    pdf_xpubs_only_note: "Xpubs only. This sheet spends nothing; with the seeds it rebuilds the wallet.",
    /// Line in `wallet_sheet`.
    pdf_descriptor: "Descriptor",
    /// Line in `wallet_sheet`.
    pdf_xpubs: "Xpubs",
    /// Line in `wallet_sheet`.
    pdf_first_addresses: "First addresses",
    /// Heading in `wallet_sheet`.
    pdf_spend_paths: "Spend paths",
    /// Line in `wallet_sheet`, a wait in blocks. `{}`: days, blocks, the keys.
    pdf_unmoved_blocks: "Coins left unmoved for {} days ({} blocks) can be spent by {} alone. Move every coin to a new address of this wallet before then.",
    /// Line in `wallet_sheet`, a wait in 512-second units. `{}`: days, the keys.
    pdf_unmoved_days: "Coins left unmoved for {} days can be spent by {} alone. Move every coin to a new address of this wallet before then.",
    /// Line in `wallet_sheet`, over the table. `{}`: the margin in days.
    pdf_move_before: "Move before = received on + {} days",
    /// Line in `wallet_sheet`, over the table on the Starter's blank sheet.
    pdf_move_before_blank: "Move before = received on + the margin in days",
    /// Heading in `wallet_sheet`: a table column.
    pdf_received_on: "Received on",
    /// Heading in `wallet_sheet`: a table column.
    pdf_amount: "Amount",
    /// Heading in `wallet_sheet`: a table column.
    pdf_move_before_col: "Move before",
    /// Line in `share_sheet`.
    pdf_has_recovery_path: "This wallet has a recovery path: see its wallet sheet",
    /// Line in `blank_sheet`, where its keys and wait are filled in by hand.
    pdf_path_keys_wait: "Keys · Wait",
    /// Line in `blank_sheet`, the sentence with its numbers left blank.
    pdf_unmoved_blank: "Coins left unmoved for ______ days (______ blocks) can be spent by ______ alone. Move every coin to a new address of this wallet before then.",
    /// Text in `wallet_sheet`.
    pdf_written_on_label: "Written on",
    /// Text in `wallet_sheet`.
    pdf_kept_at_label: "Kept at",
    /// Text in `wallet_sheet`.
    pdf_goes_with_seed: "Goes with seed of key",
    /// Line in `wallet_sheet`.
    pdf_made_by_faraday_check: "Made by Faraday. Check the first address against your wallet software before relying on this sheet.",
    /// Text in `envelopes_sheet`. `{}`: `name`.
    pdf_envelopes: "{} · envelopes",
    /// Text in `share_sheet`.
    pdf_wallet: "Wallet",
    /// Text in `share_sheet`. `{}`: `name`, `part`.
    pdf_split_backup_sheet: "{} · split backup, sheet {}",
    /// Text in `share_sheet`. `{}`: `quorum`.
    pdf_part_of_the_wallet: "Part of the wallet's descriptor: xpubs only, it spends nothing. Sheets enough to hold every xpub ({} wallet) rebuild it.",
    /// Line in `share_sheet`.
    pdf_made_by_faraday: "Made by Faraday. Scan the code into Faraday, or type the lines, one sheet at a time.",
    /// Error in `blank_sheet`.
    pdf_template_word_count: "the template is for 12 or 24 words",
    /// Text in `blank_sheet`. `{}`: `words`.
    pdf_seed_sheet_words: "Seed sheet · {} words",
    /// Line in `blank_sheet`.
    pdf_written_by_hand: "Written by hand. Never photographed or scanned by any other device once filled in.",
    /// Text in `blank_sheet`. `{}`: `get(f, "network").unwrap_or("mainnet")`.
    pdf_network: "Network: {}",
    /// Text in `blank_sheet`.
    pdf_fingerprint: "Fingerprint",
    /// Text in `blank_sheet`.
    pdf_first_address: "First address",
    /// Line in `blank_sheet`, under the title.
    pdf_bip39_english: "BIP-39 words, English. Any BIP-39 wallet restores them.",
    /// Line in `blank_sheet`: two boxes to tick by hand.
    pdf_passphrase_tick: "Passphrase: none [ ] kept elsewhere [ ]",
    /// Line in `blank_sheet`: for a sheet that holds one Seed XOR part.
    pdf_seed_xor_part: "Seed XOR part ___ of ___ (leave empty for whole words)",
    /// Line in `blank_sheet`, where the template was made with no name and shape.
    pdf_wallet_kind_path: "Wallet: ______ (kind, path)",
    /// Line in `blank_sheet`.
    pdf_there_is_no_line: "There is no line for a passphrase. Written beside the words, it would stop being a second factor.",
    /// The blank template's footer, on every page.
    pdf_print_actual_size: "Print at 100% (actual size).",
    /// Line in `blank_sheet`.
    pdf_places: "Places",
    /// Line in `blank_sheet`.
    pdf_place: "Place",
    /// Line in `blank_sheet`.
    pdf_holds: "holds",
    // ----- build (faraday-paper/src/build.rs) -----
    /// Text in `version`.
    build_0_1_0_or_earlier: "0.1.0 or earlier",
    /// Text in `describe`. `{}`: `number`, `commit`.
    build_test_release_short: "{} test release ({})",
    /// Text in `describe`. `{}`: `number`, `commit`, `fp`.
    build_test_release: "{} test release ({}, changes {})",
    /// Text in `describe`. `{}`: `number`, `commit`.
    build_dev: "{} dev ({})",
    /// Text in `describe`. `{}`: `number`, `commit`, `fp`.
    build_dev_changes: "{} dev ({}, changes {})",
        }
    };
}
