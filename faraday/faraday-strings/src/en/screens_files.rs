//! Faraday's English words for the screens files area.

macro_rules! words_screens_files {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- inbox_panel (faraday-core/src/screens/files.rs) -----
    /// Line in `inbox_panel`.
    inbox_panel_wallets: "Wallets",
    /// Text in `inbox_panel`. `{}`: `fw.shape`, `fw.seeds_here`, `n`, `if n == 1 { EN.inbox_panel_key } else…`, `if fw.files.len() > 1 { text!(" · {…`.
    inbox_panel_of_here: "{} · {} of {} {} here{}",
    /// Text in `inbox_panel`, after a space. `{}`: `fw.files.len()`.
    inbox_panel_files_agree: "· {} files say it",
    /// Text in `inbox_panel`, after a space.
    inbox_panel_from_split_sheets: "· from split sheets",
    /// Text in `inbox_panel`.
    inbox_panel_key: "key",
    /// Text in `inbox_panel`.
    inbox_panel_keys: "keys",
    /// Text in `inbox_panel`.
    inbox_panel_ready_to_sign: "Ready to sign once loaded",
    /// Text in `inbox_panel`. `{}`: `wallets.len() - ROWS`.
    inbox_panel_more_loaded: "+ {} more, loaded with these",
    /// Text in `inbox_panel`.
    inbox_panel_nothing: "Nothing chosen",
    /// Text in `inbox_panel`.
    inbox_panel_load_the_wallet: "Load the wallet with its seeds",
    /// Text in `inbox_panel`.
    inbox_panel_load_wallet: "Load the wallet",
    /// Text in `inbox_panel`. `{}`: `n`.
    inbox_panel_load_wallets_seeds: "Load {} wallets with their seeds",
    /// Text in `inbox_panel`. `{}`: `n`.
    inbox_panel_load_wallets: "Load {} wallets",
    /// Text in `inbox_panel`. `{}`: `app.medium.noun()`.
    inbox_panel_keys_load: "Keys load once the {} is out",
    /// Line in `inbox_panel`.
    inbox_panel_loose_keys: "Keys with no wallet",
    /// Text in `inbox_panel`.
    inbox_panel_make_wallet: "Make it a wallet",
    /// Text in `inbox_panel`. `{}`: `potential.len() - ROWS`.
    inbox_panel_more: "+ {} more",
    /// Line in `inbox_panel`.
    inbox_panel_loose_xpubs: "Xpubs with no wallet",
    /// Text in `inbox_panel`.
    inbox_panel_no_origin: "no origin",
    /// Text in `inbox_panel`.
    inbox_panel_make_watch_only: "Make it a watch-only wallet",
    /// Line in `inbox_panel`.
    inbox_panel_not_complete: "Not complete or sealed",
    // ----- add_to_vault (faraday-core/src/screens/files.rs) -----
    /// Text in `add_to_vault`.
    add_to_vault_button: "Add to vault",
    // ----- restore_shares_button (faraday-core/src/screens/files.rs) -----
    // ----- files (faraday-core/src/screens/files.rs) -----
    /// Title in `files`.
    files_files: "Files",
    /// Text in `files`.
    files_contents: "Contents",
    /// Button in `files`.
    files_decode: "Decode",
    /// Choice in a text file's ⋮: the file as the message to sign.
    files_sign_this_message: "Sign this message",
    /// Choice in a text file's ⋮: the file read by Check an address.
    files_check_an_address: "Check an address",
    /// Row in the Details sheet of a PSBT Faraday did not open: why.
    details_refused: "Not opened",
    /// Text in `files`. `{}`: `app.medium.a()`.
    files_waiting_medium: "Waiting for {}",
    /// Text in `stick_status` and `pull_sheet`: the kernel shows storage attached that the disk process has not listed. `{}`: `app.medium.noun()`.
    files_attached_unreadable: "A {} is attached and cannot be read",
    /// Text in `files`. `{}`: `app.medium.cap()`.
    files_attached_lock: "{} attached · lock to use it",
    /// Text in `on_no_stick`: the On the stick card while a stick is in and something is held. `{}`: `app.medium.noun()`.
    files_lock_to_use: "Lock to use the {}",
    /// Button in `on_no_stick`.
    files_lock: "Lock",
    /// Line in `files`.
    files_in_memory_emptied: "In memory · emptied at power-off",
    /// Text in `files`.
    files_open: "open",
    /// Text in `out_state`.
    files_ready: "Ready",
    /// Text in `out_state`.
    files_written_after: "Written after Lock",
    /// Text in `out_state`.
    files_unprotected: "Unprotected",
    /// Text in `out_line`. `{}`: `v.changes`, `if v.changes == 1 { EN.vault_change }…`.
    files_vault_open_unsaved: "Vault, open · {} unsaved {} · sealed and locked at Lock",
    /// Text in `out_line`.
    files_vault_open_sealed: "Vault, open · sealed and locked at Lock",
    /// Text in `files`. `{}`: `app.medium.a()`.
    files_empty_files_are: "Empty · files are copied in on {} visit",
    /// Text in `files`.
    files_public: "Public",
    /// Text in `files`.
    files_written_sealed: "Written sealed",
    /// Text in `files`.
    files_unprotected_secrets: "Unprotected secrets",
    /// Text in `files`.
    files_make_pdf: "Make PDF",
    /// Text in `files`.
    files_show_as_qr: "Show as QR",
    /// Text in `files`.
    files_remove: "Remove",
    /// Text in `files`: the ⋮ menu's last choice on a file no stick holds.
    files_delete: "Delete",
    /// Title of the sheet that asks first. `{}`: the file's name.
    outbox_delete_title: "Delete {}?",
    /// Line in that sheet: a vault no stick holds.
    outbox_delete_vault_none: "This vault is on no stick. Everything in it is lost.",
    /// Line in that sheet: a vault a stick holds an older copy of. `{}`: the stick's label.
    outbox_delete_vault_since: "Its changes since {} are lost.",
    /// Line in that sheet: an unprotected secret no stick holds.
    outbox_delete_file_none: "This file is on no stick. Everything in it is lost.",
    /// The hold in that sheet, for a vault.
    outbox_delete_hold_vault: "Hold to delete the vault",
    /// The hold in that sheet, for a file.
    outbox_delete_hold_file: "Hold to delete the file",
    /// The cancel in that sheet.
    outbox_delete_cancel: "Cancel",
    /// Text in `files`. `{}`: `app.medium.for_the()`.
    files_power_off_asks: "Power off asks first while files wait {}",
    // ----- receipt_rows (faraday-core/src/screens/files.rs) -----
    /// Text in `receipt_rows`.
    receipt_rows_verified: "verified",
    // ----- vault_panel (faraday-core/src/screens/files.rs) -----
    /// Text in `vault_panel`. `{}`: `open.name`, `open.label()`.
    vault_panel_open: "{} · {} · open",
    /// Text in `vault_panel`.
    vault_panel_select: "Select all",
    /// Line in `vault_panel`.
    vault_panel_seeded_wallets: "Wallets with their seeds",
    /// Text in `vault_panel`. `{}`: `here`, `wt.total`, `if wt.total == 1 { EN.vault_panel_key…`.
    vault_panel_keys_here: "{} of {} {} in this vault",
    /// Text in `vault_panel`.
    vault_panel_ready_to_sign: "Ready to sign once loaded",
    /// Text in `vault_panel`. `{}`: `more`.
    vault_panel_more_listed: "+ {} more · listed under Each seed and wallet",
    /// Text in `vault_panel`.
    vault_panel_load_wallet: "Load wallet with keys",
    /// Text in `vault_panel`.
    vault_panel_load: "Load 1 wallet with keys",
    /// Text in `vault_panel`. `{}`: `n`.
    vault_panel_load_wallets: "Load {} wallets with keys",
    /// Text in `vault_panel`.
    vault_panel_hide_each: "Hide each seed and wallet",
    /// Text in `vault_panel`. `{}`: `rows.len()`.
    vault_panel_each_seed: "Each seed and wallet · {}",
    /// Text in `vault_panel`.
    vault_panel_loaded: "Loaded",
    /// Text in `vault_panel`.
    vault_panel_everything: "Everything is loaded",
    /// Text in `vault_panel`.
    vault_panel_nothing: "Nothing chosen",
    /// Text in `vault_panel`. `{}`: `count(wl, EN.vault_panel_wallet, EN.v…`.
    vault_panel_load_n: "Load {}",
    /// Text in `vault_panel`. `{}`: `count(s, EN.vault_panel_key, EN.vault…`, `count(wl, EN.vault_panel_wallet, EN.v…`.
    vault_panel_load_keys_wallets: "Load {} and {}",
    /// Text in `vault_panel`.
    vault_panel_key: "key",
    /// Text in `vault_panel`.
    vault_panel_keys: "keys",
    /// Text in `vault_panel`.
    vault_panel_wallet: "wallet",
    /// Text in `vault_panel`.
    vault_panel_wallets: "wallets",
    /// Button in `vault_panel`.
    vault_panel_go_to_wallets: "Go to Wallets",
    // ----- inbox (faraday-core/src/screens/files.rs) -----
    /// Line in `inbox_panel_compact`.
    inbox_wallets: "Wallets",
    /// Text in `inbox_panel_compact`. `{}`: `fw.shape`, `fw.seeds_here`, `n`, `if n == 1 { EN.inbox_key } else { EN.…`, `if fw.files.len() > 1 { text!(" · {…`.
    inbox_of_here: "{} · {} of {} {} here{}",
    /// Text in `inbox_panel_compact`.
    inbox_key: "key",
    /// Text in `inbox_panel_compact`.
    inbox_keys: "keys",
    /// Line in `inbox_panel_compact`.
    inbox_ready_to_sign: "Ready to sign once loaded",
    /// Text in `inbox_panel_compact`. `{}`: `wallets.len() - ROWS`.
    inbox_more_loaded: "+ {} more, loaded with these",
    /// Text in `inbox_panel_compact`.
    inbox_nothing_chosen: "Nothing chosen",
    /// Text in `inbox_panel_compact`.
    inbox_load_wallet_seeds: "Load the wallet with its seeds",
    /// Text in `inbox_panel_compact`.
    inbox_load_the_wallet: "Load the wallet",
    /// Text in `inbox_panel_compact`. `{}`: `n`.
    inbox_load_wallets_seeds: "Load {} wallets with their seeds",
    /// Text in `inbox_panel_compact`. `{}`: `n`.
    inbox_load_wallets: "Load {} wallets",
    /// Text in `inbox_panel_compact`. `{}`: `app.medium.noun()`.
    inbox_keys_load_once: "Keys load once the {} is out",
    /// Line in `inbox_panel_compact`.
    inbox_loose_keys: "Keys with no wallet",
    /// Text in `inbox_panel_compact`.
    inbox_make_it_a_wallet: "Make it a wallet",
    /// Text in `inbox_panel_compact`. `{}`: `potential.len() - ROWS`.
    inbox_more: "+ {} more",
    /// Line in `inbox_panel_compact`.
    inbox_loose_xpubs: "Xpubs with no wallet",
    /// Text in `inbox_panel_compact`.
    inbox_no_origin: "no origin",
    /// Text in `inbox_panel_compact`.
    inbox_make_it_a_watch: "Make it a watch-only wallet",
    /// Line in `inbox_panel_compact`.
    inbox_not_complete: "Not complete or sealed",
    // ----- vault (faraday-core/src/screens/files.rs) -----
    /// Text in `vault_panel_compact`. `{}`: `open.name`, `open.label()`.
    vault_open: "{} · {} · open",
    /// Line in `vault_panel_compact`.
    vault_seeded_wallets: "Wallets with their seeds",
    /// Text in `vault_panel_compact`. `{}`: `here`, `wt.total`, `if wt.total == 1 { EN.vault_key_inline } els…`.
    vault_of_in_this_vault: "{} of {} {} in this vault",
    /// Line in `vault_panel_compact`.
    vault_ready_to_sign: "Ready to sign once loaded",
    /// Text in `vault_panel_compact`. `{}`: `more`.
    vault_more_listed_under: "+ {} more · listed under Each seed and wallet",
    /// Text in `vault_panel_compact`.
    vault_load_wallet_keys: "Load wallet with keys",
    /// Text in `vault_panel_compact`.
    vault_load_1_wallet: "Load 1 wallet with keys",
    /// Text in `vault_panel_compact`. `{}`: `n`.
    vault_load_wallets: "Load {} wallets with keys",
    /// Text in `vault_panel_compact`.
    vault_hide_each_seed: "Hide each seed and wallet",
    /// Text in `vault_panel_compact`. `{}`: `rows.len()`.
    vault_each_seed_wallet: "Each seed and wallet · {}",
    /// Line in `vault_panel_compact`.
    vault_select_all: "Select all",
    /// Text in `vault_panel_compact`.
    vault_loaded: "Loaded",
    /// Text in `vault_panel_compact`.
    vault_everything_loaded: "Everything is loaded",
    /// Text in `vault_panel_compact`.
    vault_nothing_chosen: "Nothing chosen",
    /// Text in `vault_panel_compact`. `{}`: `count(wl, EN.vault_wallet_inline, EN.vault_w…`.
    vault_load_n: "Load {}",
    /// Text in `vault_panel_compact`. `{}`: `count(s, EN.vault_key_inline, EN.vault_keys)`, `count(wl, EN.vault_wallet_inline, EN.vault_w…`.
    vault_load_keys_wallets: "Load {} and {}",
    /// Text in `vault_panel_compact`.
    vault_key_inline: "key",
    /// Text in `vault_panel_compact`.
    vault_keys: "keys",
    /// Text in `vault_panel_compact`.
    vault_wallet_inline: "wallet",
    /// Text in `vault_panel_compact`.
    vault_wallets: "wallets",
    /// Text in `vault_panel_compact`.
    vault_go_to_wallets: "Go to Wallets",
    // ----- inbox_actions (faraday-core/src/screens/files.rs) -----
    /// Text in `inbox_actions`.
    inbox_actions_add_to_vault: "Add to vault",
    /// Text in `inbox_actions`.
    inbox_actions_add_signatures: "Add to signatures",
    /// Text in `inbox_actions`.
    inbox_actions_import: "Import into the vault",
    /// Text in `inbox_actions`.
    inbox_actions_contents: "Contents",
    /// Text in `inbox_actions`.
    inbox_actions_unlock: "Unlock",
    /// Text in `inbox_actions`.
    inbox_actions_sign: "Sign",
    /// Text in `inbox_actions`.
    inbox_actions_load_wallet: "Load the wallet",
    /// Text in `inbox_actions`.
    inbox_actions_decode: "Decode",
    /// Text in `inbox_actions`: a signed PSBT or finished transaction to check rather than sign.
    inbox_actions_nonce_check: "Nonce check",
    /// Text in `inbox_actions`. `{}`: the share files.
    inbox_actions_restore: "Restore the wallet · {} shares",
    /// Note in `inbox_actions`. `{}`: `m.have.len()`, `m.n`.
    inbox_actions_of_keys: "{} of {} keys in hand",
    /// Text in `inbox_actions`.
    inbox_actions_load: "Load this key",
    /// Text in `inbox_actions`.
    inbox_actions_check: "Check the signature",
    /// Files' actions: a SQLite database's button.
    inbox_actions_read_wallet: "Read the wallet",
    /// Text in `inbox_actions`.
    inbox_actions_make: "Make PDF",
        }
    };
}
