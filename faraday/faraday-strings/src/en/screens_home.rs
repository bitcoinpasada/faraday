//! Faraday's English words for the screens home area.

macro_rules! words_screens_home {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- home_lead (faraday-core/src/screens/home.rs) -----
    /// Text in `home_lead`.
    home_lead_make_a_new: "Make a new wallet",
    /// Text in `home_lead`.
    home_lead_single_key: "Single key or multisig",
    // ----- home_job (faraday-core/src/screens/home.rs) -----
    /// Text in `home_job`.
    home_job_carry_on_spend: "Carry on the spend",
    /// Text in `home_job`. `{}`: `w.name`, `s.signers.len()`, `app.spend_needed()`.
    home_job_signatures: "{} · Signatures {} of {}",
    /// Text in `home_job`. `{}`: `s.spend.source`.
    home_job_continue_signing: "Continue signing {}",
    /// Text in `home_job`. `{}`: `s.signers.len()`, `app.spend_needed()`.
    home_job_of_signatures: "{} of {} signatures",
    /// Text in `home_job`. `{}`: `app.inbox[i].name`.
    home_job_sign: "Sign {}",
    /// Text in `home_job`.
    home_job_from_files: "From Files",
    /// Text in `home_job`. `{}`: `imp.label`.
    home_job_read_source: "Read from {}",
    /// Text in `home_job`. `{}`: `imp.label`.
    home_job_import_source: "Import from {}",
    /// Text in `home_job`. `{}`: `r.label`.
    home_job_written_target: "Written to {}",
    /// Text in `home_job`. `{}`: `f.name`.
    home_job_in_files: "{} · in Files",
    /// Text in `home_job`. `{}`: `st.label`.
    home_job_files_on: "Files on {}",
    /// Text in `home_job`. `{}`: `name`.
    home_job_unlock: "Unlock {}",
    /// Text in `home_job`. `{}`: `app.medium.noun()`.
    home_job_pull_to_unlock: "Pull the {} to unlock",
    /// Text in `home_job`. `{}`: `name`, `app.medium.a()`.
    home_job_write_target: "Write {} to {}",
    // ----- the wide Home's lead and wallet cards (faraday-core/src/screens/home.rs) -----
    /// The lead's button: carry on a spend.
    home_btn_carry_on: "Carry on",
    /// The lead's button: finish a backup.
    home_btn_finish: "Finish",
    /// The lead's button: sign a PSBT in Files.
    home_btn_sign: "Sign",
    /// The lead's button: unlock a vault.
    home_btn_unlock: "Unlock",
    /// The lead's button: open Files.
    home_btn_open: "Open",
    /// The lead's button: write a changed vault.
    home_btn_write: "Write",
    /// The lead's button: make a wallet from a key no wallet uses.
    home_btn_make_a_wallet: "Make a wallet",
    /// The lead's line under a backup's name.
    home_card_backup_not_finished: "Backup not finished",
    /// The tail of a vault's chip on Home, after its name.
    home_going_open: "open",
    /// The row on Home that opens Wallets.
    home_all_wallets: "All wallets",
    /// The row on Home that opens Explore.
    home_all_keys: "All keys",
    /// The line under a key no wallet uses.
    home_card_no_wallet_uses_it: "No wallet uses it",
    /// A key's line on Home when it was loaded with a passphrase; the same words without it are another key.
    home_key_passphrase: "passphrase",
    /// A wallet card's line while its spend is under way. `{}`: signatures, needed.
    home_card_signatures: "Signatures {} of {}",
    /// A wallet card's line while Files holds a PSBT for it.
    home_card_a_psbt_to_sign: "A PSBT to sign",
    /// A wallet card's line while a key made here waits for its backup.
    home_card_not_backed_up: "Not backed up",
    // ----- home_ways (faraday-core/src/screens/home.rs) -----
    /// Text in `home_ways`.
    home_ways_make_a_new: "Make a new wallet",
    /// Text in `home_ways`.
    home_ways_restore_wallet: "Restore an existing wallet",
    /// Text in `home_ways`.
    home_ways_spend_backup: "Spend from a backup, step by step",
    /// Home's top row button, with an icon: `home_ways_make_a_new`.
    home_ways_new_short: "New wallet",
    /// Home's top row button: `home_ways_restore_wallet`.
    home_ways_restore_short: "Restore",
    /// Home's top row button: `home_ways_spend_backup`.
    home_ways_spend_short: "Spend from backup",
    /// Text in `home_ways`.
    home_ways_a_stick_attached: "A stick attached",
    /// Text in `home_ways`.
    home_ways_backups: "Backups",
    /// Text in `home_ways`. `{}`: `without`.
    home_ways_not_backed: "{} not backed up",
    // ----- home (faraday-core/src/screens/home.rs) -----
    /// The heading over Home's box of what is going on.
    home_box_heading: "This session",
    /// Hint in Home's search bar.
    home_search_hint: "Search",
    /// Title in `home`.
    home_home: "Home",
    /// Button in `home`.
    home_scan: "Scan",
        }
    };
}
