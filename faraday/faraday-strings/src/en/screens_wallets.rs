//! Faraday's English words for the screens wallets area.

macro_rules! words_screens_wallets {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- missing_keys_line (faraday-core/src/screens/wallets.rs) -----
    /// Text in `missing_keys_line`. `{}`: `files[i].name`.
    missing_keys_line_locked: "{} is locked · it may hold them",
    /// Button in `missing_keys_line`.
    missing_keys_line_unlock: "Unlock",
    // ----- loose_card (faraday-core/src/screens/wallets.rs) -----
    /// Text in `loose_card`. `{}`: `fp_text(*only)`.
    loose_card_key_is_loaded: "Key {} is loaded. No wallet uses it.",
    /// Text in `loose_card`. `{}`: `loose.len()`.
    loose_card_keys_are: "{} keys are loaded. No wallet uses them.",
    /// Text in `loose_card`.
    loose_card_add_another: "Add another key",
    /// Text in `loose_card`.
    loose_card_restore: "Restore an existing wallet",
    // ----- wallets (faraday-core/src/screens/wallets.rs) -----
    /// Text in `wallets`.
    wallets_signing: "Signing",
    /// Text in `wallets`.
    wallets_continue: "Continue",
    /// Text in `wallets`.
    wallets_in_files: "in Files",
    /// Text in `wallets`.
    wallets_review: "Review",
    /// Text in `wallets`: opens the PSBT in Files as a spend.
    wallets_sign: "Sign",
    /// Button in `wallets`: Check an address, opened from the wallet.
    wallets_check_an_address: "Check an address",
    /// Button in `wallets`: Explore on a key of the wallet that is loaded.
    wallets_explore_its_keys: "Explore its keys",
    /// Button in the signed message's result: Check a signed message on it.
    message_check_it: "Check it",
    /// Section label in `wallets`.
    wallets_wallets: "Wallets",
    /// Text in `wallets`. `{}`: `shape`.
    wallets_watch_only: "{} · watch only",
    /// Section label in `wallets`.
    wallets_keys_without: "Keys without a wallet",
    /// Button in `wallets`.
    wallets_restore_wallet: "Restore an existing wallet",
    /// Text in `wallets`. `{}`: `wl.name`.
    wallets_scan_key_title: "Scan a key for {}",
    /// Text in `wallets`.
    wallets_add_a_key: "Add a key",
    /// Text in `wallets`.
    wallets_no_wallet_loaded: "No wallet loaded",
    /// Line in `wallets`.
    wallets_enter_keeps: "Enter keeps it · Esc leaves it",
    /// Button in `wallets`.
    wallets_rename: "Rename",
    /// Heading in `wallets`: the group of a wallet's own actions.
    wallets_what_you_can_do: "What you can do",
    /// Heading in a row's details: what the row says of the item.
    row_details_facts: "Facts",
    /// Text in `wallets`.
    wallets_show_wallet: "Show wallet QR",
    /// Text in `wallets`.
    wallets_back_up: "Back up",
    /// Text in `wallets`.
    wallets_sign_a_message: "Sign a message",
    /// Text in `wallets`.
    wallets_remove_from_session: "Remove from session",
    /// Text in `wallets`. `{}`: `here`, `m`.
    wallets_of_signatures: "{} of {} signatures possible here",
    /// Text in `wallets`.
    wallets_can_spend: "Can spend",
    /// A wallet's row in the Wallets list and the wallet rows, in place of
    /// Can spend, for a silent payments wallet whose record the loaded key
    /// with its fingerprint contradicts.
    wallets_does_not_match: "Does not match its key",
    /// Chip and list mark in `wallets` and `wallet_card` for a wallet a key not of its own spends alone (F132).
    wallets_another_can_spend: "Another key can spend",
    /// Text in `wallet_rows` for such a wallet. `{}`: `here`, `m`.
    wallet_rows_another_can_spend: "{} of {} here · another key can spend",
    /// Text in `wallets`.
    wallets_scan_a_key: "Scan a key",
    /// Line in `wallets`.
    wallets_at_a_glance: "At a glance",
    /// Line in `wallets`.
    wallets_first_receive: "First receive address · 0/0",
    // ----- silent_wallet_card (faraday-core/src/screens/wallets.rs) -----
    /// Line in `silent_wallet_card`.
    silent_wallet_card_key: "Key",
    /// Text in `silent_wallet_card`.
    silent_wallet_card_here: "Here",
    /// Text in `silent_wallet_card`.
    silent_wallet_card_not_here: "Not here",
    /// Chip on a silent payments wallet's card, wide and small panel,
    /// where the loaded key with its fingerprint gives another scan key or
    /// spend key.
    silent_wallet_card_differs: "Does not match",
    /// Text in `silent_wallet_card`.
    silent_wallet_card_load: "Load from vault",
    /// Text in `silent_wallet_card`.
    silent_wallet_card_add: "Add its key",
    /// Line in `silent_wallet_card`.
    silent_wallet_card_address_label: "Address",
    /// Line in `silent_wallet_card`.
    silent_wallet_card_labels: "Labels handed out",
    /// Text in `silent_wallet_card`.
    silent_wallet_card_address: "Address, labels and payments",
    /// Text in `silent_wallet_card`.
    silent_wallet_card_show: "Show wallet QR",
    /// Text in `silent_wallet_card`.
    silent_wallet_card_remove: "Remove from session",
        }
    };
}
