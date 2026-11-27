//! Faraday's English words for the screens restore area.

macro_rules! words_screens_restore {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- restore (faraday-core/src/screens/restore.rs) -----
    /// Text in `restore_screen`. `{}`: `s.m`, `s.n`.
    restore_threshold: "{} of {}",
    /// Text in `restore_screen`.
    restore_none: "None",
    /// Text in `restore_screen`. `{}`: `here`, `all`.
    restore_of_filled: "{} of {} filled",
    /// Text in `restore_screen`.
    restore_home: "Home",
    /// Text in `restore_screen`.
    restore_tools: "Tools",
    /// Text in `restore_screen`.
    restore_decode: "Decode a transaction",
    /// Text in `restore_screen`.
    restore_back: "Back",
    /// Text in `restore_screen`.
    restore_title: "Restore an existing wallet",
    /// Text in `restore_body`.
    restore_have_description: "I have the descriptor",
    /// Text in `restore_body`.
    restore_scan_button: "Scan it",
    /// Text in `restore_body`.
    restore_from_files: "From Files",
    /// Text in `restore_body`.
    restore_continue: "Continue",
    /// Text in `restore_body`.
    restore_it_matches: "It matches",
    /// Restore's Check card for a wallet made from the seeds alone: the button.
    restore_it_matches_record: "It matches my record",
    /// Restore's Check card for a wallet made from the seeds alone: the other way on.
    restore_nothing_to_compare: "I have nothing to compare",
    /// Restore's Check card, a line on a wallet made from the seeds alone: its
    /// kind, account and path, which the seeds do not say.
    restore_made_from_seed: "Made from the seed alone: {}, account {}, {}",
    /// Restore's Check card, the same line for a custom path.
    restore_made_from_seed_path: "Made from the seed alone: {}, path {}",
    /// Line in `restore_body`.
    restore_make_the_wallet: "Make the wallet first",
    /// Restore's seeds card, a line on what a wrong passphrase does.
    restore_wrong_passphrase: "A wrong passphrase shows on the key's row",
    /// Text in `restore_body`.
    restore_back_up_button: "Back up this wallet",
    /// Text in `restore_body`.
    restore_wallets: "Wallets",
    /// Text in `restore_body`: New wallet from this one, on Done.
    restore_another_wallet: "Another wallet from these seeds",
    // ----- restore_description (faraday-core/src/screens/restore.rs) -----
    /// Text in `restore_description`. `{}`: `n`.
    restore_description_source: "From {}",
    /// Text in `restore_description`.
    restore_description_i_have: "I have the seed",
    /// Text in `restore_description`.
    restore_description_i: "I do not have it",
    /// Line in `restore_description`.
    restore_description_shares: "Shares of a split backup",
    /// Text in `restore_description`. `{}`: `m.have.len()`, `m.n`, `m.have` joined by " ".
    restore_description_progress: "{} of {} keys in hand · {}",
    /// Text in `restore_description`, where the shares of two wallets of one checksum each complete. `{}`: the first wallet's name, the second's.
    restore_two_wallets: "The shares rebuild both {} and {}, so neither is rebuilt.",
    /// Text in `restore_description`, added to `restore_two_wallets` where a share file in Files ends it.
    restore_two_wallets_tick: "Tick a share file that only one of them holds.",
    /// Text in `restore_description`, added to `restore_two_wallets` where opening one vault alone ends it.
    restore_two_wallets_vault: "Open only the vault of one of them.",
    /// Text in `restore_description`, as `restore_two_wallets_vault` after the tick sentence.
    restore_two_wallets_vault_or: "Or open only the vault of one of them.",
    /// Button in `restore_description`.
    restore_description_rebuild: "Rebuild the wallet",
    // ----- restore_seeds (faraday-core/src/screens/restore.rs) -----
    /// Text in `restore_seeds`. `{}`: `k + 1`.
    restore_seeds_key_number: "Key {} of the wallet",
    /// Button in `restore_seeds`.
    restore_seeds_change: "Change",
    /// Button in `restore_seeds`.
    restore_seeds_not_here: "Not here",
    /// Text in `restore_seeds`.
    restore_seeds_here: "Here",
    /// Text in `restore_seeds`. `{}`: `label`.
    restore_seeds_here_n: "Here · {}",
    /// Row in `restore_seeds`.
    restore_seeds_empty: "Empty",
    /// Text in `restore_seeds`.
    restore_seeds_its_xpub: "Its xpub",
    /// Text in `restore_seeds`.
    restore_seeds_make: "Make the wallet",
    // ----- restore_open_slot (faraday-core/src/screens/restore.rs) -----
    /// Text in `restore_open_slot`.
    restore_open_slot_scan: "Scan a SeedQR",
    // ----- restore_sources (faraday-core/src/screens/restore.rs) -----
    /// Text in `restore_sources_compact`.
    restore_sources_copy_files: "Copy files in",
    /// Text in `restore_sources_compact`.
    restore_sources_key_loaded: "Not while a key is loaded; Lock, then Restore starts over",
    /// Text in `restore_sources_compact`.
    restore_sources_vault_open: "Not while a vault is open; Lock, then Restore starts over",
    /// Text in `restore_sources_compact`.
    restore_sources_secret_typed: "Not while a secret is typed; Lock, then Restore starts over",
    /// Button in `restore_sources_compact`.
    restore_sources_lock: "Lock",
    /// Text in `restore_sources_compact`. `{}`: `app.medium.noun()`.
    restore_sources_insert: "Insert the {}",
    /// Line in `restore_sources_compact`.
    restore_sources_vault: "Vault",
    /// Text in `restore_sources_compact`. `{}`: `f.name`.
    restore_sources_unlock: "Unlock {}",
    /// Text in `restore_sources_compact`.
    restore_sources_wallet: "Wallet",
    /// Text in `restore_sources_compact`. `{}`: `app.medium.noun()`.
    restore_sources_copy: "Copy the vault in from its {}",
        }
    };
}
